#![allow(clippy::result_large_err)]
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, MintTo};
use anchor_spl::associated_token::{self, AssociatedToken, Create, get_associated_token_address};
use anchor_spl::token::spl_token::state::Account as SPLTokenAccount;
use anchor_lang::solana_program::program_pack::Pack;
// use mpl_token_metadata::instruction::{create_metadata_accounts_v2, update_metadata_accounts_v2};
// use mpl_token_metadata::state::{Creator, DataV2};
 //use mpl_token_metadata::ID as TOKEN_METADATA_PROGRAM_ID;


 declare_id!("EMijxVq8c7yUTHNA7acNdtJBPU6jqqWcMDxsdowqP4Hb");

#[program]
pub mod fantasy_sports {
    use super::*;

    pub fn place_bet(ctx: Context<PlaceBet>) -> Result<()> {
        let bump = ctx.bumps.mint_authority;
        let user_key = ctx.accounts.user_pick.key();
        let seeds = &[b"mint", user_key.as_ref(), &[bump]];

        associated_token::create(CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            Create {
                payer: ctx.accounts.bettor.to_account_info(),
                associated_token: ctx.accounts.user_ata.to_account_info(),
                authority: ctx.accounts.bettor.to_account_info(),
                mint: ctx.accounts.nft_mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        ))?;

        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.nft_mint.to_account_info(),
                    to: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.mint_authority.to_account_info(),
                },
                &[seeds],
            ),
            1,
        )?;

        // Commented out metadata creation logic
        /*
        let metadata_ix = create_metadata_accounts_v2(
            TOKEN_METADATA_PROGRAM_ID,
            ctx.accounts.metadata.key(),
            ctx.accounts.nft_mint.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.bettor.key(),
            ctx.accounts.mint_authority.key(),
            "Fantasy Pick NFT".to_string(),
            "PICK".to_string(),
            format!("https://example.com/nfts/{}_unsettled.json", ctx.accounts.user_pick.key()),
            Some(vec![Creator {
                address: ctx.accounts.mint_authority.key(),
                verified: true,
                share: 100,
            }]),
            ROYALTY_BPS,
            true,
            true,
        );

        invoke_signed(
            &metadata_ix,
            &[
                ctx.accounts.metadata.to_account_info(),
                ctx.accounts.nft_mint.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.bettor.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ],
            &[seeds],
        )?;
        */

        let pool = &mut ctx.accounts.bet_pool;
        let pick = &ctx.accounts.user_pick;
        let fee = pick.bet_amount / 20;
        pool.fee_collected += fee;

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

        // ✅ Check that recipient owns the NFT
        let expected_ata = get_associated_token_address(
        &ctx.accounts.recipient.key(),
        &ctx.accounts.nft_mint.key(),
    );

        require!(ctx.accounts.nft_ata.key() == expected_ata, ErrorCode::InvalidNFTATA);

        // ✅ Load and verify token balance == 1
        let nft_data = &ctx.accounts.nft_ata.try_borrow_data()?;
        let token_account = SPLTokenAccount::unpack(&nft_data)?;
        require!(token_account.amount == 1, ErrorCode::InvalidNFTBalance);
        require!(token_account.mint == ctx.accounts.nft_mint.key(), ErrorCode::InvalidNFTMint);
        require!(token_account.owner == ctx.accounts.recipient.key(), ErrorCode::InvalidNFTOwner);

        // ✅ Check win condition
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
            **ctx.accounts.recipient.to_account_info().try_borrow_mut_lamports()? += payout;
        }

        // ✅ Mark as claimed and update owner to current holder
        pick.claimed = true;
        pick.owner = ctx.accounts.recipient.key();

        Ok(())
    }

    pub fn withdraw_fees(ctx: Context<WithdrawFees>) -> Result<()> {
        let pool = &mut ctx.accounts.bet_pool;
        let amount = pool.fee_collected;
        pool.fee_collected = 0;

        **ctx.accounts.fee_vault.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.recipient.to_account_info().try_borrow_mut_lamports()? += amount;
        Ok(())
    }

}


















#[derive(Accounts)]
#[instruction(fixture_id: u64, sport_name: String, player_id: Pubkey, stat_name: String, stat_line: u32)]
pub struct InitializeBetPool<'info> {
    #[account(
        init,
        seeds = [
            b"bet_pool",
            fixture_id.to_le_bytes().as_ref(),
            player_id.as_ref(),
            stat_name.as_bytes(),
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
        mint::authority = mint_authority,
        mint::freeze_authority = mint_authority,
    )]
    pub nft_mint: Account<'info, Mint>,

    #[account(
        seeds = [b"mint", user_pick.key().as_ref()],
        bump
    )]
    /// CHECK: PDA used only as signer
    pub mint_authority: UncheckedAccount<'info>,

    /// CHECK: ATA will be created on-chain if not exists
    #[account(mut)]
    pub user_ata: UncheckedAccount<'info>,

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
    pub recipient: SystemAccount<'info>, // user who owns the NFT

    /// CHECK: Owner of user_pick before claim
    pub owner: UncheckedAccount<'info>,

    #[account(mut)]
    pub nft_mint: Account<'info, Mint>,

    /// CHECK: manually verified ATA
    #[account(mut)]
    pub nft_ata: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
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

pub const ROYALTY_BPS: u16 = 250;
pub const DISCRIMINATOR_SIZE: usize = 8;

pub fn size_with_discriminator<T>() -> usize {
    std::mem::size_of::<T>() + DISCRIMINATOR_SIZE
}

pub fn round_stat_line(value: u32) -> u32 {
    let float_val = value as f64 / 10.0;
    let rounded = float_val.round();
    let adjusted = if rounded % 2.0 == 0.0 { rounded - 0.5 } else { rounded + 0.5 };
    (adjusted * 10.0).round() as u32
}

#[account] // #[derive(Debug, AnchorSerialize, AnchorDeserialize)]  replace with later #[account]
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
    #[msg("Invalid fee vault PDA.")]
    InvalidFeeVault,
    #[msg("Invalid NFT ATA: does not match expected associated token account.")]
    InvalidNFTATA,
    #[msg("NFT ATA does not contain the NFT.")]
    InvalidNFTBalance,
    #[msg("NFT token account does not match expected mint.")]
    InvalidNFTMint,
    #[msg("NFT token account owner does not match recipient.")]
    InvalidNFTOwner,
}



















