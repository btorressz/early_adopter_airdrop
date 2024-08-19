use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, TokenAccount, Token};

declare_id!("MmZW8AQAHZuSAeh69QCgnu5nLxBXnTX3RPCtgBh7qBX");

#[program]
mod early_adopter_airdrop {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, decimals: u8) -> Result<()> {
        let cpi_accounts = token::InitializeMint {
            mint: ctx.accounts.mint.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::initialize_mint(cpi_ctx, decimals, &ctx.accounts.authority.key(), Some(&ctx.accounts.authority.key()))
    }

    pub fn initialize_user(ctx: Context<InitializeUser>, user: Pubkey) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.user = user;
        user_account.loyalty_points = 0;
        user_account.last_activity = Clock::get()?.unix_timestamp;
        user_account.loyalty_tier = 1;
        Ok(())
    }

    pub fn reward_early_adopter(ctx: Context<RewardEarlyAdopter>, amount: u64) -> Result<()> {
        let cpi_accounts = token::MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.recipient.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::mint_to(cpi_ctx, amount)?;
        emit!(EarlyAdopterRewarded {
            user: ctx.accounts.recipient.key(),
            amount,
        });
        Ok(())
    }

    pub fn track_loyalty(ctx: Context<TrackLoyalty>, user: Pubkey, points: u64) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.loyalty_points += points;
        user_account.last_activity = Clock::get()?.unix_timestamp;
        emit!(LoyaltyPointsTracked { user, points });
        Ok(())
    }

    pub fn create_proposal(ctx: Context<CreateProposal>, description: String) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        proposal.description = description;
        proposal.votes_for = 0;
        proposal.votes_against = 0;
        Ok(())
    }

    pub fn vote(ctx: Context<Vote>, in_favor: bool) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        let voter_account = &ctx.accounts.voter_account;

        let voting_power = voter_account.tokens_held;

        if in_favor {
            proposal.votes_for += voting_power;
        } else {
            proposal.votes_against += voting_power;
        }
        emit!(GovernanceVoted {
            proposal: proposal.key(),
            user: ctx.accounts.voter.key(),
            in_favor,
        });
        Ok(())
    }

    pub fn redeem_loyalty(ctx: Context<RedeemLoyalty>, points: u64) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        if user_account.loyalty_points < points {
            return Err(LoyaltyProgramError::InsufficientFunds.into());
        }
        user_account.loyalty_points -= points;
        user_account.last_activity = Clock::get()?.unix_timestamp;

        let amount = points * 100; // Example conversion rate
        let cpi_accounts = token::MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.recipient.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::mint_to(cpi_ctx, amount)?;

        emit!(LoyaltyPointsRedeemed {
            user: ctx.accounts.user_account.user,
            points,
        });
        Ok(())
    }

    pub fn burn_tokens(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
        let cpi_accounts = token::Burn {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::burn(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn time_based_rewards(ctx: Context<TimeBasedRewards>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        let current_time = Clock::get()?.unix_timestamp;
        let time_held = current_time - user_account.last_activity;

        // Example: reward 1 point for every day held
        let points = (time_held / 86400) as u64;
        user_account.loyalty_points += points;
        user_account.last_activity = current_time;

        emit!(LoyaltyPointsTracked {
            user: user_account.user,
            points,
        });

        Ok(())
    }

    pub fn refer_user(ctx: Context<ReferUser>, referrer: Pubkey) -> Result<()> {
        let referrer_account = &mut ctx.accounts.referrer_account;
        referrer_account.loyalty_points += 100; // Example referral reward
        emit!(UserReferred { referrer, referred: ctx.accounts.user.key() });
        Ok(())
    }

    pub fn apply_inactivity_penalty(ctx: Context<ApplyInactivityPenalty>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        let current_time = Clock::get()?.unix_timestamp;
        let inactivity_period = current_time - user_account.last_activity;

        let penalty_points = (inactivity_period / 604800) as u64 * 10;
        if user_account.loyalty_points >= penalty_points {
            user_account.loyalty_points -= penalty_points;
        } else {
            user_account.loyalty_points = 0;
        }
        user_account.last_activity = current_time;

        Ok(())
    }

    pub fn update_profile(ctx: Context<UpdateProfile>, name: String, bio: String) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.name = name;
        user_account.bio = bio;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, mint::decimals = 9, mint::authority = authority)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeUser<'info> {
    #[account(init, payer = authority, space = 8 + 32 + 8 + 8 + 4 + 100 + 200)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RewardEarlyAdopter<'info> {
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub recipient: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TrackLoyalty<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}

#[derive(Accounts)]
pub struct CreateProposal<'info> {
    #[account(init, payer = authority, space = 8 + 256 + 8 + 8)]
    pub proposal: Account<'info, Proposal>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Vote<'info> {
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    #[account(mut)]
    pub voter: Signer<'info>,
    #[account(mut)]
    pub voter_account: Account<'info, UserAccount>,
}

#[derive(Accounts)]
pub struct RedeemLoyalty<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub recipient: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TimeBasedRewards<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}

#[derive(Accounts)]
pub struct ReferUser<'info> {
    #[account(mut)]
    pub referrer_account: Account<'info, UserAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct ApplyInactivityPenalty<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}

#[derive(Accounts)]
pub struct UpdateProfile<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}

#[account]
pub struct UserAccount {
    pub user: Pubkey,
    pub loyalty_points: u64,
    pub last_activity: i64,
    pub loyalty_tier: u8,
    pub tokens_held: u64,
    pub name: String,
    pub bio: String,
}

#[account]
pub struct Proposal {
    pub description: String,
    pub votes_for: u64,
    pub votes_against: u64,
}

#[error_code]
pub enum LoyaltyProgramError {
    #[msg("Insufficient funds.")]
    InsufficientFunds,
    #[msg("Unauthorized action.")]
    Unauthorized,
    #[msg("Account already initialized.")]
    AccountAlreadyInitialized,
    #[msg("Account not initialized.")]
    AccountNotInitialized,
}

#[event]
pub struct EarlyAdopterRewarded {
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct LoyaltyPointsTracked {
    pub user: Pubkey,
    pub points: u64,
}

#[event]
pub struct GovernanceVoted {
    pub proposal: Pubkey,
    pub user: Pubkey,
    pub in_favor: bool,
}

#[event]
pub struct LoyaltyPointsRedeemed {
    pub user: Pubkey,
    pub points: u64,
}

#[event]
pub struct UserReferred {
    pub referrer: Pubkey,
    pub referred: Pubkey,
}
