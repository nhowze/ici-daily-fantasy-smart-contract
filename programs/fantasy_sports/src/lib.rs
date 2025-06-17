#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token};
use anchor_lang::system_program; 
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


declare_id!("5faLUWJzNrtPLoxMBLwhJw9U6BMUXBuqhA3A7Pr2E9qR");

#[program]
pub mod fantasy_sports {
    use super::*;

use anchor_lang::system_program;

pub fn initialize_bet_pool(
    ctx: Context<InitializeBetPool>,
    fixture_id: u64,
    sport_name: [u8; 32],
    player_id: Pubkey,
    stat_name: [u8; 32],
    stat_line: u32,
    betting_deadline: i64,
) -> Result<()> {
    require!(fixture_id > 0, ErrorCode::InvalidFixture);
    require!(stat_line > 0, ErrorCode::InvalidStatLine);
    require!(betting_deadline > Clock::get()?.unix_timestamp, ErrorCode::DeadlinePassed);

    let (expected_bet_pool, bet_pool_bump) = Pubkey::find_program_address(
        &[
            b"bet_pool",
            &fixture_id.to_le_bytes(),
            &sport_name,
            player_id.as_ref(),
            &stat_name,
            &stat_line.to_le_bytes(),
        ],
        ctx.program_id,
    );
    require_keys_eq!(ctx.accounts.bet_pool.key(), expected_bet_pool, ErrorCode::InvalidFixture);

    let (expected_fee_vault, fee_vault_bump) = Pubkey::find_program_address(
        &[b"fee_vault", expected_bet_pool.as_ref()],
        ctx.program_id,
    );
    require_keys_eq!(ctx.accounts.fee_vault.key(), expected_fee_vault, ErrorCode::InvalidFeeVault);

    let rent = Rent::get()?;
    let pool_space = 8 + std::mem::size_of::<BetPool>();
    let pool_lamports = rent.minimum_balance(pool_space);

    let bet_pool_info = ctx.accounts.bet_pool.to_account_info();
    let bet_pool_seeds = &[
        b"bet_pool",
        &fixture_id.to_le_bytes(),
        &sport_name[..],
        player_id.as_ref(),
        &stat_name[..],
        &stat_line.to_le_bytes(),
        &[bet_pool_bump],
    ];

    if bet_pool_info.lamports() == 0 || bet_pool_info.data_is_empty() {
        let create_pool_ix = anchor_lang::solana_program::system_instruction::create_account(
            ctx.accounts.admin.key,
            ctx.accounts.bet_pool.key,
            pool_lamports,
            pool_space as u64,
            ctx.program_id,
        );
        anchor_lang::solana_program::program::invoke_signed(
            &create_pool_ix,
            &[
                ctx.accounts.admin.to_account_info(),
                bet_pool_info.clone(),
            ],
            &[bet_pool_seeds],
        )?;
    }

    let pool = BetPool {
        fixture_id,
        sport_name,
        player_id,
        stat_name,
        stat_line,
        betting_deadline,
        over_total: 0,
        under_total: 0,
        fee_collected: 0,
        result: Outcome::Pending,
        settled: false,
        authority: ctx.accounts.admin.key(),
        version: 1,
    };

let mut data = ctx.accounts.bet_pool.try_borrow_mut_data()?;
let mut cursor = std::io::Cursor::new(data.as_mut());
pool.try_serialize(&mut cursor)?;

    let fee_vault_info = ctx.accounts.fee_vault.to_account_info();
    let fee_vault_lamports = rent.minimum_balance(0);
    let fee_vault_seeds = &[
        b"fee_vault",
        expected_bet_pool.as_ref(),
        &[fee_vault_bump],
    ];

    if fee_vault_info.lamports() == 0 || fee_vault_info.data_is_empty() {
        let create_fee_vault_ix = anchor_lang::solana_program::system_instruction::create_account(
            ctx.accounts.admin.key,
            ctx.accounts.fee_vault.key,
            fee_vault_lamports,
            0,
            ctx.program_id,
        );
        anchor_lang::solana_program::program::invoke_signed(
            &create_fee_vault_ix,
            &[
                ctx.accounts.admin.to_account_info(),
                fee_vault_info.clone(),
            ],
            &[fee_vault_seeds],
        )?;
    }

    Ok(())
}


}

#[derive(Accounts)]
pub struct InitializeBetPool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: We’ll validate this manually
    #[account(mut)]
    pub bet_pool: UncheckedAccount<'info>,

    /// CHECK: Also validated manually
    #[account(mut)]
    pub fee_vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
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

#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct PlaceBet<'info> {
    #[account(
        init,
        payer = bettor,
        seeds = [b"mint", user_pick.key().as_ref()],
        bump,
        mint::decimals = 0,
        mint::authority = mint_authority,
        mint::freeze_authority = mint_authority
    )]
    pub mint: Account<'info, Mint>,

    #[account(seeds = [b"mint", user_pick.key().as_ref()], bump)]
    /// CHECK: PDA mint authority
    pub mint_authority: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: User's token account
    pub user_token_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub fee_vault: SystemAccount<'info>,

    #[account(mut)]
    pub bettor: Signer<'info>,

    #[account(mut)]
    pub bet_pool: Account<'info, BetPool>,

    #[account(
        init,
        payer = bettor,
        space = 8 + 32 + 8 + 1 + 32 + 1 + 32 + 1 + 32,
        seeds = [b"user_pick", bettor.key().as_ref(), bet_pool.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub user_pick: Account<'info, UserPick>,

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
    pub betting_deadline: i64,
    pub over_total: u64,
    pub under_total: u64,
    pub fee_collected: u64,
    pub result: Outcome,
    pub settled: bool,
    pub authority: Pubkey,
    pub version: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum Outcome {
    Pending,
    OverWins,
    UnderWins,
    Canceled,
}