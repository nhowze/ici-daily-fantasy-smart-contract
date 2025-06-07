use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo};
use anchor_spl::associated_token::AssociatedToken;

declare_id!("4gUqFwJxDDQsgd3qhoGhx42nJo9QHszXy6RdN53AVzjp");

pub const ROYALTY_BPS: u16 = 250;
pub const DISCRIMINATOR_SIZE: usize = 8;

pub fn size_of<T>() -> usize {
    std::mem::size_of::<T>() + DISCRIMINATOR_SIZE
}

//
// CONTEXTS FIRST
//

#[derive(Accounts)]
#[instruction(fixture_id: u64, sport_name: String, player_id: Pubkey, stat_line: u32)]
pub struct InitializeBetPool<'info> {
    #[account(
        init,
        seeds = [
            b"bet_pool",
            fixture_id.to_le_bytes().as_ref(),
            player_id.as_ref(),
            &stat_line.to_le_bytes()
        ],
        bump,
        payer = admin,
        space = size_of::<BetPool>()
    )]
    pub bet_pool: Account<'info, BetPool>,

    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub bettor: Signer<'info>,

    #[account(mut)]
    pub bet_pool: Account<'info, BetPool>,

    #[account(
        init,
        seeds = [b"user_pick", bettor.key().as_ref(), bet_pool.key().as_ref()],
        bump,
        payer = bettor,
        space = size_of::<UserPick>()
    )]
    pub user_pick: Account<'info, UserPick>,

    #[account(
        mut,
        seeds = [b"fee_vault", bet_pool.key().as_ref()],
        bump
    )]
    pub fee_vault: SystemAccount<'info>,

    #[account(
        init,
        seeds = [b"mint", user_pick.key().as_ref()],
        bump,
        payer = bettor,
        mint::decimals = 0,
        mint::authority = nft_mint.key(),
        mint::freeze_authority = nft_mint.key(),
    )]
    pub nft_mint: Account<'info, Mint>,

    #[account(mut)]
    pub user_ata: Account<'info, TokenAccount>,  // <-- SAFE version for 0.31.1

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PublishResult<'info> {
    #[account(mut)]
    pub bet_pool: Account<'info, BetPool>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct SettleClaim<'info> {
    #[account(mut, has_one = owner)]
    pub user_pick: Account<'info, UserPick>,

    #[account(mut)]
    pub bet_pool: Account<'info, BetPool>,

    #[account(mut)]
    pub recipient: SystemAccount<'info>,

    /// CHECK:
    pub owner: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct WithdrawFees<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub bet_pool: Account<'info, BetPool>,

    #[account(
        mut,
        seeds = [b"fee_vault", bet_pool.key().as_ref()],
        bump
    )]
    /// CHECK:
    pub fee_vault: UncheckedAccount<'info>,

    #[account(mut)]
    pub recipient: SystemAccount<'info>,
}

//
// PROGRAM
//

#[program]
pub mod fantasy_sports {
    use super::*;

    pub fn initialize_bet_pool(
        ctx: Context<InitializeBetPool>,
        fixture_id: u64,
        sport_name: String,
        player_id: Pubkey,
        stat_line: u32,
        betting_deadline: i64,
    ) -> Result<()> {
        let bet_pool = &mut ctx.accounts.bet_pool;
        bet_pool.fixture_id = fixture_id;

        let mut name_bytes = [0u8; 32];
        let name_slice = sport_name.as_bytes();
        let copy_len = name_slice.len().min(32);
        name_bytes[..copy_len].copy_from_slice(&name_slice[..copy_len]);
        bet_pool.sport_name = name_bytes;

        bet_pool.player_id = player_id;
        bet_pool.stat_line = stat_line;
        bet_pool.betting_deadline = betting_deadline;
        bet_pool.over_total = 0;
        bet_pool.under_total = 0;
        bet_pool.fee_collected = 0;
        bet_pool.result = Outcome::Pending;
        bet_pool.settled = false;
        bet_pool.authority = ctx.accounts.admin.key();
        bet_pool.version = 1;
        Ok(())
    }

    pub fn place_bet(
        ctx: Context<PlaceBet>,
        amount: u64,
        pick_side: bool,
        _uri: String,
    ) -> Result<()> {
        let bet_pool = &mut ctx.accounts.bet_pool;

        let user_pick_key = ctx.accounts.user_pick.key();

        let user_pick = &mut ctx.accounts.user_pick;

        require!(
            Clock::get()?.unix_timestamp < bet_pool.betting_deadline,
            ErrorCode::BettingClosed
        );

        let platform_fee = amount * 500 / 10000;
        let pool_amount = amount - platform_fee;

        let (expected_fee_vault, _) = Pubkey::find_program_address(
            &[b"fee_vault", bet_pool.key().as_ref()],
            ctx.program_id,
        );
        require_keys_eq!(
            expected_fee_vault,
            ctx.accounts.fee_vault.key(),
            ErrorCode::InvalidFeeVault
        );

        **ctx.accounts.bettor.try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.fee_vault.try_borrow_mut_lamports()? += platform_fee;
        **bet_pool.to_account_info().try_borrow_mut_lamports()? += pool_amount;

        bet_pool.fee_collected += platform_fee;

        if pick_side {
            bet_pool.over_total += pool_amount;
        } else {
            bet_pool.under_total += pool_amount;
        }

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"mint",
            user_pick_key.as_ref(),
            &[ctx.bumps.nft_mint],
        ]];

        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.nft_mint.to_account_info(),
                    to: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.nft_mint.to_account_info(),
                },
                signer_seeds,
            ),
            1,
        )?;

        user_pick.owner = ctx.accounts.bettor.key();
        user_pick.bet_amount = amount;
        user_pick.pick_side = pick_side;
        user_pick.pool = bet_pool.key();
        user_pick.claimed = false;
        user_pick.mint = ctx.accounts.nft_mint.key();
        user_pick.bump = ctx.bumps.user_pick;

        Ok(())
    }

    pub fn publish_result(ctx: Context<PublishResult>, result: Outcome) -> Result<()> {
        let pool = &mut ctx.accounts.bet_pool;
        require!(pool.result == Outcome::Pending, ErrorCode::AlreadySettled);
        pool.result = result;
        pool.settled = true;
        Ok(())
    }

    pub fn settle_claim(ctx: Context<SettleClaim>) -> Result<()> {
        let pick = &mut ctx.accounts.user_pick;
        let pool = &mut ctx.accounts.bet_pool;

        require!(pool.settled, ErrorCode::PoolNotSettled);
        require!(!pick.claimed, ErrorCode::AlreadyClaimed);

        let won = match (pick.pick_side, &pool.result) {
            (true, Outcome::OverWins) => true,
            (false, Outcome::UnderWins) => true,
            _ => false,
        };

        if won {
            let total_winner_pool = if pick.pick_side {
                pool.over_total
            } else {
                pool.under_total
            };
            let total_loser_pool = if pick.pick_side {
                pool.under_total
            } else {
                pool.over_total
            };

            let payout = pick.bet_amount * total_loser_pool / total_winner_pool;

            **pool.to_account_info().try_borrow_mut_lamports()? -= payout;
            **ctx.accounts.recipient.try_borrow_mut_lamports()? += payout;
        }

        pick.claimed = true;
        Ok(())
    }

    pub fn withdraw_fees(ctx: Context<WithdrawFees>) -> Result<()> {
        let pool = &mut ctx.accounts.bet_pool;
        let amount = pool.fee_collected;
        pool.fee_collected = 0;

        **ctx.accounts.fee_vault.try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.recipient.try_borrow_mut_lamports()? += amount;
        Ok(())
    }
}

//
// STATE
//

#[account]
pub struct BetPool {
    pub fixture_id: u64,
    pub sport_name: [u8; 32],
    pub player_id: Pubkey,
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

#[account]
pub struct UserPick {
    pub owner: Pubkey,
    pub bet_amount: u64,
    pub pick_side: bool,
    pub pool: Pubkey,
    pub claimed: bool,
    pub mint: Pubkey,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum Outcome {
    Pending,
    OverWins,
    UnderWins,
    Canceled,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Betting is closed for this pool.")]
    BettingClosed,
    #[msg("Bet has already been settled.")]
    AlreadySettled,
    #[msg("Bet pool has not been settled yet.")]
    PoolNotSettled,
    #[msg("This pick has already been claimed.")]
    AlreadyClaimed,
    #[msg("Invalid fee vault PDA")]
    InvalidFeeVault,
}
