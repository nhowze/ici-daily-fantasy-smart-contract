use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, MintTo};

#[error_code]
pub enum ErrorCode {
    #[msg("This pick has already been claimed.")]
    AlreadyClaimed,
    #[msg("Bet pool has not been settled yet.")]
    PoolNotSettled,
    #[msg("Only the bet pool authority can publish results.")]
    Unauthorized,
}

declare_id!("5faLUWJzNrtPLoxMBLwhJw9U6BMUXBuqhA3A7Pr2E9qR");

#[program]
pub mod fantasy_sports {
    use super::*;

    pub fn initialize_bet_pool(
    ctx: Context<InitializeBetPool>,
    fixture_id: u64,
    sport_name: [u8; 32],
    player_id: Pubkey,
    stat_name: [u8; 32],
    stat_line: u32,
    betting_deadline: i64,
) -> Result<()> {

        let pool = &mut ctx.accounts.bet_pool;
        pool.fixture_id = fixture_id;
        pool.stat_name = stat_name;
        pool.player_id = player_id;
        pool.stat_line = stat_line;
        pool.betting_deadline = betting_deadline;
        pool.over_total = 0;
        pool.under_total = 0;
        pool.fee_collected = 0;
        pool.result = Outcome::Pending;
        pool.settled = false;
        pool.authority = ctx.accounts.admin.key();
        pool.version = 1;
        pool.sport_name = sport_name;
        Ok(())
    }

    pub fn withdraw_fees(ctx: Context<WithdrawFees>) -> Result<()> {
        let pool = &ctx.accounts.bet_pool;
        let fee_amount = pool.fee_collected;

        **ctx.accounts.fee_vault.try_borrow_mut_lamports()? -= fee_amount;
        **ctx.accounts.recipient.try_borrow_mut_lamports()? += fee_amount;
        Ok(())
    }

    pub fn settle_claim(ctx: Context<SettleClaim>) -> Result<()> {
        let pick = &mut ctx.accounts.user_pick;
        let pool = &ctx.accounts.bet_pool;

        require!(!pick.claimed, ErrorCode::AlreadyClaimed);
        require!(pool.settled, ErrorCode::PoolNotSettled);

        let won = match pool.result {
            Outcome::OverWins => pick.pick_side,
            Outcome::UnderWins => !pick.pick_side,
            _ => false,
        };

        if won {
            let payout = pick.bet_amount * 2;
            **ctx.accounts.recipient.try_borrow_mut_lamports()? += payout;
        }

        pick.claimed = true;
        Ok(())
    }

    pub fn place_bet(
        ctx: Context<PlaceBet>,
        bet_amount: u64,
        pick_side: bool,
        sport_name: [u8; 32],
        nonce: u64,
    ) -> Result<()> {

    // 🔍 Debugging mismatched seeds for user_pick
    let expected_user_pick = Pubkey::find_program_address(
        &[
            b"user_pick",
            ctx.accounts.bettor.key().as_ref(),
            ctx.accounts.bet_pool.key().as_ref(),
            &nonce.to_le_bytes(),
        ],
        ctx.program_id,
    ).0;
    msg!("Expected user_pick PDA: {}", expected_user_pick);
    msg!("Received user_pick: {}", ctx.accounts.user_pick.key());


        let bump = ctx.bumps.mint_authority;
        let user_pick_key = ctx.accounts.user_pick.key();
        let seeds = &[b"mint", user_pick_key.as_ref(), &[bump]];
        let signer_seeds = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.mint_authority.to_account_info(),
            },
            signer_seeds,
        );
        token::mint_to(cpi_ctx, 1)?;

        let pick = &mut ctx.accounts.user_pick;
        pick.bet_amount = bet_amount;
        pick.pick_side = pick_side;
        pick.owner = ctx.accounts.bettor.key();
        pick.pool = ctx.accounts.bet_pool.key();
        pick.claimed = false;
        pick.mint = ctx.accounts.mint.key();
        pick.bump = ctx.bumps.user_pick;
        pick.sport_name = sport_name;

        let pool = &mut ctx.accounts.bet_pool;
        let fee = bet_amount / 20;
        pool.fee_collected += fee;
        Ok(())
    }

    pub fn publish_result(ctx: Context<PublishResult>, result: Outcome) -> Result<()> {
        let pool = &mut ctx.accounts.bet_pool;
        require_keys_eq!(ctx.accounts.authority.key(), pool.authority, ErrorCode::Unauthorized);
        pool.result = result;
        pool.settled = true;
        Ok(())
    }
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
#[instruction(fixture_id: u64, sport_name: [u8; 32], player_id: Pubkey, stat_name: [u8; 32], stat_line: u32)]
pub struct InitializeBetPool<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 183,
        seeds = [
            b"bet_pool",
            &fixture_id.to_le_bytes(),
            sport_name.as_ref(),
            player_id.as_ref(),
            stat_name.as_ref(),
            &stat_line.to_le_bytes()
        ],
        bump
    )]
    pub bet_pool: Account<'info, BetPool>,

    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
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
    /// CHECK: This is a PDA mint authority, validated in CPI
    pub mint_authority: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: This is the user's token account to receive the minted NFT. It must be pre-validated or created off-chain.
    pub user_token_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub fee_vault: SystemAccount<'info>,
    #[account(mut)]
    pub bettor: Signer<'info>,
    #[account(mut)]
    pub bet_pool: Account<'info, BetPool>,
    #[account(
        init,
        seeds = [b"user_pick", bettor.key().as_ref(), bet_pool.key().as_ref(), &nonce.to_le_bytes()],
        bump,
        payer = bettor,
        space = 8 + 32 + 8 + 1 + 32 + 1 + 32 + 1 + 32
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
