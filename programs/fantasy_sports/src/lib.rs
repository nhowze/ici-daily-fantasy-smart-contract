#![allow(clippy::result_large_err)]
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};
use anchor_spl::associated_token::{self, AssociatedToken};
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::solana_program::system_instruction;
use std::str::FromStr;

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
    AlreadyPublished, // e.g., 0x65 if you want
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
    // 🔢 1. Save current nonce and increment it
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
    msg!("test");

    let bet_pool = &ctx.accounts.bet_pool;
    let user_pick = &mut ctx.accounts.user_pick;

    require!(user_pick.pool == bet_pool.key(), ErrorCode::Unauthorized);
    require!(bet_pool.result_published, ErrorCode::PoolNotSettled);
    require!(!user_pick.claimed, ErrorCode::AlreadyClaimed);

    let over_wins = bet_pool.final_stat > bet_pool.stat_line;
    let under_wins = bet_pool.final_stat < bet_pool.stat_line;

    let winner_is_over = match (over_wins, under_wins) {
        (true, false) => true,
        (false, true) => false,
        _ => {
            user_pick.claimed = true;
            return Ok(()); // Push/tie
        }
    };

    if user_pick.pick_side != winner_is_over {
        user_pick.claimed = true;
        return Ok(()); // Didn't win
    }

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

    msg!("Paying {} lamports to {}", payout, ctx.accounts.recipient.key());
    msg!("🏦 total_pool: {}", total_pool);
    msg!("🏆 winner_pool: {}", winner_pool);
    msg!("💰 user_bet_amount: {}", user_pick.bet_amount);

    // ✅ Manually transfer lamports between program-owned account and system account
    **ctx.accounts.bet_vault.to_account_info().try_borrow_mut_lamports()? -= payout;
    **ctx.accounts.recipient.to_account_info().try_borrow_mut_lamports()? += payout;

    user_pick.claimed = true;
    Ok(())
}



pub fn withdraw_fees(ctx: Context<WithdrawFees>) -> Result<()> {
    let admin_pk = Pubkey::from_str(ADMIN_PUBKEY).unwrap();
    require!(ctx.accounts.admin.key() == admin_pk, ErrorCode::Unauthorized);

    let bet_pool = &ctx.accounts.bet_pool;

    let (expected_fee_vault, _) = Pubkey::find_program_address(
        &[b"fee_vault", bet_pool.key().as_ref()],
        ctx.program_id,
    );
    require!(
        ctx.accounts.fee_vault.key() == expected_fee_vault,
        ErrorCode::InvalidFeeVault
    );

    let fee_vault_lamports = ctx.accounts.fee_vault.lamports();
    if fee_vault_lamports < 5000 {
        msg!(
            "Skipping withdrawal: too few lamports ({}).",
            fee_vault_lamports
        );
        return Ok(());
    }

    // ✅ Direct lamport transfer
    **ctx.accounts.fee_vault.to_account_info().try_borrow_mut_lamports()? -= fee_vault_lamports;
    **ctx.accounts.recipient.to_account_info().try_borrow_mut_lamports()? += fee_vault_lamports;

    msg!(
        "✅ Withdrew {} lamports from fee_vault {} to recipient {}",
        fee_vault_lamports,
        ctx.accounts.fee_vault.key(),
        ctx.accounts.recipient.key()
    );

    Ok(())
}



    pub fn buy_pick_nft(ctx: Context<BuyPickNFT>, sale_price: u64) -> Result<()> {
        let user_pick = &mut ctx.accounts.user_pick;

        // Prevent buying if pick has already been claimed
        require!(!user_pick.claimed, ErrorCode::AlreadyClaimed);

        // Prevent buying if pool is already settled
        let bet_pool = &ctx.accounts.pool;
        require!(!bet_pool.result_published, ErrorCode::PoolNotSettled);

        // Calculate royalty fee (2.5%) and seller payout
        let royalty_fee = (sale_price * 25) / 1000;
        let seller_amount = sale_price - royalty_fee;

        // Transfer to seller
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

        // Transfer royalty to royalty vault (admin wallet or PDA)
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

        // Transfer the NFT from seller to buyer
        let cpi_accounts = Transfer {
            from: ctx.accounts.seller_token_account.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.seller.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        // Update ownership
        user_pick.owner = ctx.accounts.buyer.key();

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

        Ok(())
    }

pub fn reclaim_unsold_pick(ctx: Context<ReclaimUnsoldPick>) -> Result<()> {
    let user_pick = &ctx.accounts.user_pick;

   let user_pick_key = user_pick.key();
let seeds = &[b"escrow", user_pick_key.as_ref(), &[ctx.bumps.escrow_token_account]];
let signer = &[&seeds[..]];

let cpi_accounts = Transfer {
    from: ctx.accounts.escrow_token_account.to_account_info(),
    to: ctx.accounts.seller_token_account.to_account_info(),
    authority: ctx.accounts.escrow_token_account.to_account_info(), // ⚠️ This should be escrow authority PDA
};

let cpi_ctx = CpiContext::new_with_signer(
    ctx.accounts.token_program.to_account_info(),
    cpi_accounts,
    signer,
);

    token::transfer(cpi_ctx, ctx.accounts.user_pick.bet_amount)?;

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

    /// CHECK: PDA lamport vault
#[account(
    init,
    payer = admin,
    space = 8 + 0,
    seeds = [b"fee_vault", bet_pool.key().as_ref()],
    bump
)]
pub fee_vault: UncheckedAccount<'info>,

/// CHECK: PDA lamport vault for net bets
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
        space = 8 + 8, // 8 discriminator + u64 count
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
    /// CHECK: PDA mint authority signer
    pub mint_authority: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: created in-program if needed
    pub user_token_account: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: PDA for fee collection
    pub fee_vault: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: PDA for net bet storage
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
}

#[account]
pub struct UserNonce {
    pub count: u64,
}

#[derive(Accounts)]
pub struct BuyPickNFT<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut, has_one = mint, has_one = pool)]
    pub user_pick: Account<'info, UserPick>,

    #[account(mut)]
    pub mint: Account<'info, token::Mint>,

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

    /// CHECK: PDA to own NFT
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
        mut,
        seeds = [b"escrow", user_pick.key().as_ref()],
        bump
    )]
    pub escrow_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>, // 👈 this was missing
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
pub struct WithdrawFees<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut, has_one = fee_vault)]
    pub bet_pool: Account<'info, BetPool>,

    #[account(
        mut,
        seeds = [b"fee_vault", bet_pool.key().as_ref()],
        bump,
    )]
    pub fee_vault: UncheckedAccount<'info>,

    #[account(mut)]
    pub recipient: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
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
        bump,
    )]
    pub bet_vault: UncheckedAccount<'info>,

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
