#![allow(clippy::result_large_err)]
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};
use anchor_spl::associated_token::{self, AssociatedToken};
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::solana_program::system_instruction;
use std::str::FromStr;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token::ID as TOKEN_PROGRAM_ID;
declare_id!("6W1NLpkZvfu6y44nmCtQBLUEjGZQoCt6zQ9MouStHrFK");

pub const ADMIN_PUBKEY: &str = "5DcirLSutTThvZu9AJK9yGWXqs4HHumRvrtzZQggb7dW";


#[error_code]
pub enum ErrorCode {
    #[msg("This pick has already been claimed.")]
    AlreadyClaimed,
    #[msg("Bet pool has not been settled yet.")]
    PoolNotSettled,
    #[msg("Only the bet pool authority can publish results.")]
    Unauthorized,
    #[msg("Fee vault PDA does not match.")]
    InvalidFeeVault,
    #[msg("Fixture ID must be non-zero.")]
    InvalidFixture,
    #[msg("Stat line must be non-zero.")]
    InvalidStatLine,
    #[msg("Betting deadline must be in the future.")]
    DeadlinePassed,
    #[msg("Result already published for this bet pool.")]
    AlreadyPublished,
    #[msg("This pick is not listed for sale.")]
    NotListedForSale,
    #[msg("Invalid token balance in escrow account.")]
    InvalidTokenBalance,
    #[msg("Escrow token account has an unexpected delegate.")]
    UnexpectedDelegate,
    #[msg("Escrow token account has an unexpected close authority.")]
    UnexpectedCloseAuthority,
}




#[program]
pub mod fantasy_sports {
    use super::*;

pub fn initialize_bet_pool(
    ctx: Context<InitializeBetPool>,
    fixture_id: u64,
    player_id: Pubkey,
    stat_name: [u8; 32],
    stat_line: u32,
    sport_name: [u8; 32],
    betting_deadline: i64,
) -> Result<()> {
    msg!("🧪 fixture_id: {}", fixture_id);
    msg!("🧪 stat_line: {}", stat_line);
    msg!("🧪 player_id: {}", player_id);

    require!(fixture_id > 0, ErrorCode::InvalidFixture);
    require!(stat_line > 0, ErrorCode::InvalidStatLine);
    require!(
        betting_deadline > Clock::get()?.unix_timestamp,
        ErrorCode::DeadlinePassed
    );

    let bet_pool = &mut ctx.accounts.bet_pool;
    bet_pool.fixture_id = fixture_id;
    bet_pool.player_id = player_id;
    bet_pool.stat_name = stat_name;
    bet_pool.stat_line = stat_line;
    bet_pool.sport_name = sport_name;
    bet_pool.deadline = betting_deadline;
    bet_pool.total_over_amount = 0;
    bet_pool.total_under_amount = 0;
    bet_pool.fee_vault = ctx.accounts.fee_vault.key();
    bet_pool.result_published = false;
    bet_pool.final_stat = 0;
    bet_pool.bump = ctx.bumps.bet_pool;

    msg!("🔒 fixture_id: {:?}", fixture_id.to_le_bytes());
    msg!("🔒 sport_name: {:?}", sport_name);
    msg!("🔒 player_id: {:?}", player_id.to_bytes());
    msg!("🔒 stat_name: {:?}", stat_name);
    msg!("🔒 stat_line: {:?}", stat_line.to_le_bytes());

    // Optional: Debug PDA derivation (useful for verifying correct seeds)
    let seeds = &[
        b"bet_pool",
        &fixture_id.to_le_bytes(),
        &sport_name[..],
        player_id.as_ref(),
        &stat_name,
        &stat_line.to_le_bytes(),
    ];
    let (expected_pda, bump) = Pubkey::find_program_address(seeds, ctx.program_id);
    msg!("🧠 On-chain PDA: {}", expected_pda);
    msg!("🧠 Bump: {}", bump);
    msg!("🧠 Anchor bump: {}", ctx.bumps.bet_pool);

    Ok(())
}


pub fn place_bet(
    ctx: Context<PlaceBet>,
    _fixture_id: u64,
    _player_id: Pubkey,
    _stat_name: [u8; 32],
    _stat_line: u32,
    bet_amount: u64,
    pick_side: bool,
    _sport_name: [u8; 32],
) -> Result<()> {
    // 1. Save current nonce and increment it
    let nonce = ctx.accounts.user_nonce.count;
    ctx.accounts.user_nonce.count += 1;

    let user_pick = &mut ctx.accounts.user_pick;
    let sport_name = ctx.accounts.bet_pool.sport_name;
    let bet_pool = &mut ctx.accounts.bet_pool;

    let fee = (bet_amount * 5) / 100;
    let net_amount = bet_amount - fee;

    user_pick.owner = ctx.accounts.bettor.key();
    user_pick.bet_amount = net_amount;
    user_pick.pick_side = pick_side;
    user_pick.pool = bet_pool.key();
    user_pick.claimed = false;
    user_pick.mint = ctx.accounts.mint.key();
    user_pick.bump = ctx.bumps.user_pick;
    user_pick.sport_name = sport_name;

    if pick_side {
        bet_pool.total_over_amount += net_amount;
    } else {
        bet_pool.total_under_amount += net_amount;
    }

    invoke(
        &system_instruction::transfer(
            &ctx.accounts.bettor.key(),
            &ctx.accounts.fee_vault.key(),
            fee,
        ),
        &[
            ctx.accounts.bettor.to_account_info(),
            ctx.accounts.fee_vault.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    invoke(
        &system_instruction::transfer(
            &ctx.accounts.bettor.key(),
            &ctx.accounts.bet_vault.key(),
            net_amount,
        ),
        &[
            ctx.accounts.bettor.to_account_info(),
            ctx.accounts.bet_vault.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    let mint_authority_bump = ctx.bumps.mint_authority;
    let mint_key = user_pick.key();
    let mint_seeds: &[&[u8]] = &[b"mint", mint_key.as_ref(), &[mint_authority_bump]];
    let signer_seeds: &[&[&[u8]]] = &[mint_seeds];


        let cpi_accounts = token::InitializeMint {
            mint: ctx.accounts.mint.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );

        if ctx.accounts.mint.to_account_info().try_borrow_data()?[0] == 0 {
    let cpi_accounts = token::InitializeMint {
        mint: ctx.accounts.mint.to_account_info(),
        rent: ctx.accounts.rent.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token::initialize_mint(
        cpi_ctx,
        0,
        &ctx.accounts.mint_authority.key(),
        Some(&ctx.accounts.mint_authority.key()),
    )?;
}

    if ctx.accounts.user_token_account.to_account_info().try_borrow_data()?.len() == 0 {
        let ata_ctx = CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            associated_token::Create {
                payer: ctx.accounts.bettor.to_account_info(),
                associated_token: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.bettor.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        );
        associated_token::create(ata_ctx)?;
    }

    let cpi_accounts = MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.mint_authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token::mint_to(cpi_ctx, 1)?;

    Ok(())
}

pub fn admin_update_result(
    ctx: Context<AdminUpdateResult>,
    new_final_stat: u32,
) -> Result<()> {

    // Require only the admin can do this!
    let admin_pk = Pubkey::from_str(ADMIN_PUBKEY).unwrap();
require!(ctx.accounts.authority.key() == admin_pk, ErrorCode::Unauthorized);

    let bet_pool = &mut ctx.accounts.bet_pool;
    bet_pool.result_published = true;
    bet_pool.final_stat = new_final_stat;

    Ok(())
}

pub fn settle_claim(ctx: Context<SettleClaim>) -> Result<()> {
    let bet_pool = &ctx.accounts.bet_pool;
    let user_pick = &mut ctx.accounts.user_pick;

    require!(user_pick.pool == bet_pool.key(), ErrorCode::Unauthorized);
    require!(bet_pool.result_published, ErrorCode::PoolNotSettled);
    require!(!user_pick.claimed, ErrorCode::AlreadyClaimed);


    // Handle one-sided refund (no opposite picks)
    let has_only_over = bet_pool.total_under_amount == 0;
    let has_only_under = bet_pool.total_over_amount == 0;

    if has_only_over || has_only_under {
        let refund = user_pick.bet_amount;
        msg!("💸 Refunding {} lamports to {}", refund, ctx.accounts.recipient.key());

        **ctx.accounts.bet_vault.to_account_info().try_borrow_mut_lamports()? -= refund;
        **ctx.accounts.recipient.to_account_info().try_borrow_mut_lamports()? += refund;

        user_pick.claimed = true;
        return Ok(());
    }

    // Regular outcome logic
    let over_wins = bet_pool.final_stat > bet_pool.stat_line;
    let under_wins = bet_pool.final_stat < bet_pool.stat_line;

    let winner_is_over = match (over_wins, under_wins) {
        (true, false) => true,
        (false, true) => false,
        _ => {
            user_pick.claimed = true;
            return Ok(()); // Push/tie, no payout
        }
    };

    if user_pick.pick_side != winner_is_over {
        user_pick.claimed = true;
        return Ok(()); // Lost
    }

    // Payout calculation
    let total_pool = bet_pool.total_over_amount + bet_pool.total_under_amount;
    let winner_pool = if winner_is_over {
        bet_pool.total_over_amount
    } else {
        bet_pool.total_under_amount
    };

    let user_share = (user_pick.bet_amount as u128)
        .checked_mul(total_pool as u128)
        .unwrap()
        / (winner_pool as u128);
    let payout = user_share as u64;

    msg!("🏆 Paying {} lamports to {}", payout, ctx.accounts.recipient.key());

    **ctx.accounts.bet_vault.to_account_info().try_borrow_mut_lamports()? -= payout;
    **ctx.accounts.recipient.to_account_info().try_borrow_mut_lamports()? += payout;

    user_pick.claimed = true;
    Ok(())
}


pub fn delist_pick(ctx: Context<DelistPick>) -> Result<()> {
let user_pick_key = ctx.accounts.user_pick.key(); // <-- Immutable borrow first

let bump = ctx.bumps.escrow_pda;

let seeds: &[&[u8]] = &[
    b"escrow",
    user_pick_key.as_ref(), // use the key from above
    &[bump],
];
let signer = &[seeds];

let cpi_accounts = Transfer {
    from: ctx.accounts.escrow_token_account.to_account_info(),
    to: ctx.accounts.seller_token_account.to_account_info(),
    authority: ctx.accounts.escrow_pda.to_account_info(),
};

let cpi_ctx = CpiContext::new_with_signer(
    ctx.accounts.token_program.to_account_info(),
    cpi_accounts,
    signer,
);

token::transfer(cpi_ctx, 1)?;

// Now can mutably borrow `user_pick`
let user_pick = &mut ctx.accounts.user_pick;
user_pick.for_sale = false;

Ok(())
}



pub fn buy_pick_nft(ctx: Context<BuyPickNFT>, sale_price: u64) -> Result<()> {
    let user_pick = &mut ctx.accounts.user_pick;

    require!(!user_pick.claimed, ErrorCode::AlreadyClaimed);
    require!(!ctx.accounts.pool.result_published, ErrorCode::PoolNotSettled);

    let royalty_fee = (sale_price * 25) / 1000;
    let seller_amount = sale_price - royalty_fee;

    // Pay seller
    invoke(
        &system_instruction::transfer(
            &ctx.accounts.buyer.key(),
            &ctx.accounts.seller.key(),
            seller_amount,
        ),
        &[
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.seller.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // Pay royalty
    invoke(
        &system_instruction::transfer(
            &ctx.accounts.buyer.key(),
            &ctx.accounts.royalty_vault.key(),
            royalty_fee,
        ),
        &[
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.royalty_vault.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // Derive signer seeds
    let user_pick_key = user_pick.key();
    let seeds: &[&[u8]] = &[
        b"escrow",
        user_pick_key.as_ref(),
        // Note: you must use the actual bump here if you don't use #[account(..., bump)]
        &[Pubkey::find_program_address(&[b"escrow", user_pick_key.as_ref()], ctx.program_id).1],
    ];
    let signer = &[seeds]; // type: &[&[&[u8]]]

    // Define CPI accounts
    let cpi_accounts = anchor_spl::token::Transfer {
        from: ctx.accounts.escrow_token_account.to_account_info(),
        to: ctx.accounts.buyer_token_account.to_account_info(),
        authority: ctx.accounts.escrow_pda.to_account_info(),
    };

    // Create CPI context with signer
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer,
    );

    // Perform NFT transfer from escrow to buyer
    token::transfer(cpi_ctx, 1)?;

    // Update on-chain metadata
    user_pick.owner = ctx.accounts.buyer.key();
    user_pick.for_sale = false;

    Ok(())
}


        pub fn list_pick_nft(ctx: Context<ListPickNFT>) -> Result<()> {
        // Transfer NFT from seller to escrow
        let cpi_accounts = Transfer {
            from: ctx.accounts.seller_token_account.to_account_info(),
            to: ctx.accounts.escrow_token_account.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, 1)?;
        ctx.accounts.user_pick.for_sale = true;
        Ok(())
    }

pub fn reclaim_unsold_pick(ctx: Context<ReclaimUnsoldPick>) -> Result<()> {
    let user_pick = &mut ctx.accounts.user_pick;
    user_pick.for_sale = false;

    let user_pick_key = user_pick.key();
    let bump = ctx.bumps.escrow_pda;
    let seeds: [&[u8]; 3] = [b"escrow", user_pick_key.as_ref(), &[bump]];
    let signer: &[&[&[u8]]] = &[&seeds];

    let cpi_accounts = Transfer {
        from: ctx.accounts.escrow_token_account.to_account_info(),
        to: ctx.accounts.seller_token_account.to_account_info(),
        authority: ctx.accounts.escrow_pda.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer,
    );

    token::transfer(cpi_ctx, 1)?;

    Ok(())
}

}

//program end 
#[derive(Accounts)]
pub struct AdminUpdateResult<'info> {
    #[account(mut)]
    pub bet_pool: Account<'info, BetPool>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(fixture_id: u64, player_id: Pubkey, stat_name: [u8; 32], stat_line: u32, sport_name: [u8; 32])]
pub struct InitializeBetPool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + std::mem::size_of::<BetPool>(),
        seeds = [
            b"bet_pool",
            &fixture_id.to_le_bytes(),
            &sport_name[..],
            player_id.as_ref(),
            &stat_name,
            &stat_line.to_le_bytes(),
        ],
        bump,
    )]
    pub bet_pool: Account<'info, BetPool>,

#[account(
    init,
    payer = admin,
    space = 8 + 0,
    seeds = [b"fee_vault", bet_pool.key().as_ref()],
    bump
)]
pub fee_vault: UncheckedAccount<'info>,

#[account(
    init,
    payer = admin,
    space = 8 + 0,
    seeds = [b"bet_vault", bet_pool.key().as_ref()],
    bump
)]
pub bet_vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}



#[derive(Accounts)]
#[instruction(fixture_id: u64, player_id: Pubkey, stat_name: [u8; 32], stat_line: u32, sport_name: [u8; 32])]
pub struct PlaceBet<'info> {
    #[account(mut, signer)]
    pub bettor: Signer<'info>,

    #[account(mut, has_one = fee_vault)]
    pub bet_pool: Account<'info, BetPool>,

    #[account(
        init_if_needed,
        payer = bettor,
        space = 8 + 8,
        seeds = [b"user_nonce", bettor.key().as_ref(), bet_pool.key().as_ref()],
        bump
    )]
    pub user_nonce: Account<'info, UserNonce>,

    #[account(
        init,
        payer = bettor,
        space = 8 + std::mem::size_of::<UserPick>(),
        seeds = [
            b"user_pick",
            bettor.key().as_ref(),
            bet_pool.key().as_ref(),
            &user_nonce.count.to_le_bytes(),
        ],
        bump
    )]
    pub user_pick: Account<'info, UserPick>,

    #[account(
        init_if_needed,
        payer = bettor,
        seeds = [b"mint", user_pick.key().as_ref()],
        bump,
        mint::decimals = 0,
        mint::authority = mint_authority,
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        seeds = [b"mint", user_pick.key().as_ref()],
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub user_token_account: AccountInfo<'info>,

    #[account(mut)]
    pub fee_vault: UncheckedAccount<'info>,

    #[account(mut)]
    pub bet_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct UserPick {
    pub owner: Pubkey,
    pub bet_amount: u64,
    pub pick_side: bool,
    pub pool: Pubkey,
    pub claimed: bool,
    pub mint: Pubkey,
    pub bump: u8,
    pub sport_name: [u8; 32],
    pub for_sale: bool,
}

#[account]
pub struct UserNonce {
    pub count: u64,
}

#[derive(Accounts)]
pub struct DelistPick<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(mut)]
    pub user_pick: Account<'info, UserPick>,

    #[account(mut)]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub seller_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = escrow_pda,
    )]
    pub escrow_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"escrow", user_pick.key().as_ref()],
        bump
    )]
    pub escrow_pda: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}



#[derive(Accounts)]
pub struct BuyPickNFT<'info> {
    #[account(mut)]
    pub seller: SystemAccount<'info>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut, has_one = mint, has_one = pool)]
    pub user_pick: Account<'info, UserPick>,

    #[account(mut)]
    pub mint: Account<'info, token::Mint>,

    #[account(mut)]
    pub pool: Account<'info, BetPool>,

    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,

    #[account()]
    pub escrow_pda: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = mint,
        associated_token::authority = buyer
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"fee_vault", pool.key().as_ref()], bump)]
    pub royalty_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}


#[derive(Accounts)]
pub struct ListPickNFT<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(mut)]
    pub user_pick: Account<'info, UserPick>,

    #[account(mut)]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub pool: Account<'info, BetPool>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = seller
    )]
    pub seller_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = seller,
        associated_token::mint = mint,
        associated_token::authority = escrow_pda
    )]
    pub escrow_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"escrow", user_pick.key().as_ref()],
        bump
    )]
    pub escrow_pda: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ReclaimUnsoldPick<'info> {
    #[account(
        seeds = [b"escrow", user_pick.key().as_ref()],
        bump
    )]
    pub escrow_pda: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = escrow_pda,
        constraint = escrow_token_account.amount == 1 @ ErrorCode::InvalidTokenBalance,
        constraint = escrow_token_account.delegate == COption::None @ ErrorCode::UnexpectedDelegate,
        constraint = escrow_token_account.close_authority == COption::None @ ErrorCode::UnexpectedCloseAuthority,
    )]
    pub escrow_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub seller_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_pick: Account<'info, UserPick>,

    #[account(mut)]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub pool: Account<'info, BetPool>,

    #[account(mut)]
    pub seller: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}



#[account]
pub struct BetPool {
    pub fixture_id: u64,
    pub sport_name: [u8; 32],
    pub player_id: Pubkey,
    pub stat_name: [u8; 32],
    pub stat_line: u32,
    pub deadline: i64,

    pub total_over_amount: u64,
    pub total_under_amount: u64,

    pub fee_vault: Pubkey,
    pub result_published: bool,
    pub final_stat: u32,

    pub bump: u8,
}



#[derive(Accounts)]
pub struct SettleClaim<'info> {
    #[account(mut)]
    pub user_pick: Account<'info, UserPick>,

    #[account(mut)]
    pub bet_pool: Account<'info, BetPool>,

    #[account(mut)]
    pub recipient: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [b"bet_vault", bet_pool.key().as_ref()],
        bump
    )]
    pub bet_vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}




#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum Outcome {
    Pending,
    OverWins,
    UnderWins,
    Canceled,
}
