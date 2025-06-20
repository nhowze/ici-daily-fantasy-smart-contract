#![allow(clippy::result_large_err)]
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use anchor_lang::solana_program::program::invoke;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::solana_program::system_program;
use anchor_lang::solana_program::program::invoke_signed;
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
}


declare_id!("4ATf8K3EfcdT1RVKUybE95DSF9nZ3rhD9s9mathPPrDy");

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
    require!(betting_deadline > Clock::get()?.unix_timestamp, ErrorCode::DeadlinePassed);

    let bet_pool_account_info = ctx.accounts.bet_pool.to_account_info();



    let pool = BetPool {
        fixture_id,
        sport_name,
        player_id,
        stat_name,
        stat_line,
        deadline: betting_deadline,
        total_over_amount: 0,
        total_under_amount: 0,
        fee_vault: ctx.accounts.fee_vault.key(),
        result_published: false,
        final_stat: 0,
        bump: ctx.bumps.bet_pool,
    };

    msg!("🔒 fixture_id: {:?}", fixture_id.to_le_bytes());
    msg!("🔒 sport_name: {:?}", sport_name);
    msg!("🔒 player_id: {:?}", player_id.to_bytes());
    msg!("🔒 stat_name: {:?}", stat_name);
    msg!("🔒 stat_line: {:?}", stat_line.to_le_bytes());

    let mut data = bet_pool_account_info.try_borrow_mut_data()?;
    let mut cursor = std::io::Cursor::new(data.as_mut());
    pool.try_serialize(&mut cursor)?;

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
        _nonce: u64,
    ) -> Result<()> {
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
            &[ctx.accounts.bettor.to_account_info(), ctx.accounts.fee_vault.to_account_info(), ctx.accounts.system_program.to_account_info()],
        )?;

        invoke(
            &system_instruction::transfer(
                &ctx.accounts.bettor.key(),
                &ctx.accounts.bet_vault.key(),
                net_amount,
            ),
            &[ctx.accounts.bettor.to_account_info(), ctx.accounts.bet_vault.to_account_info(), ctx.accounts.system_program.to_account_info()],
        )?;

        let mint_authority_bump = ctx.bumps.mint_authority;
        let mint_key = user_pick.key();
        let mint_seeds: &[&[u8]] = &[b"mint", mint_key.as_ref(), &[mint_authority_bump]];
        let signer_seeds: &[&[&[u8]]] = &[mint_seeds];
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
}

//program end 


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
#[instruction(fixture_id: u64, player_id: Pubkey, stat_name: [u8; 32], stat_line: u32, bet_amount: u64, pick_side: bool, sport_name: [u8; 32], nonce: u64)]
pub struct PlaceBet<'info> {
    #[account(mut, signer)]
    pub bettor: Signer<'info>,

    #[account(mut, has_one = fee_vault)]
    pub bet_pool: Account<'info, BetPool>,

    #[account(
        init,
        payer = bettor,
        seeds = [b"user_pick", bettor.key().as_ref(), bet_pool.key().as_ref(), &nonce.to_le_bytes()],
        bump,
        space = 8 + std::mem::size_of::<UserPick>(),
    )]
    pub user_pick: Account<'info, UserPick>,

    #[account(
        mut,
        seeds = [b"mint", user_pick.key().as_ref()],
        bump,
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        seeds = [b"mint", user_pick.key().as_ref()],
        bump,
    )]
    /// CHECK: PDA mint authority signer
    pub mint_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: PDA for fee collection
    pub fee_vault: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: PDA for net bet storage
    pub bet_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
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
    #[account(mut)]
    pub bet_pool: Account<'info, BetPool>,
    #[account(mut)]
    pub fee_vault: SystemAccount<'info>,
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
}

#[derive(Accounts)]
pub struct PublishResult<'info> {
    #[account(mut)]
    pub bet_pool: Account<'info, BetPool>,
    pub authority: Signer<'info>,
}


#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum Outcome {
    Pending,
    OverWins,
    UnderWins,
    Canceled,
}
