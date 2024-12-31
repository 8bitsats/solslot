use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("AAEbKDHrGn2doRWAXuxEeNStMoxqe3qpCATHZkMuTcNy");

const TREASURY_PDA_SEED: &[u8] = b"treasury";
const USER_VAULT_SEED: &[u8] = b"uvault";
const HOLDER_REGISTRY_SEED: &[u8] = b"holders";
const BET_AMOUNT: u64 = 100000000; // 0.1 SOL
const HOLDER_REWARD_PERCENTAGE: u8 = 10; // 10% of wins go to holder rewards
const MIN_HOLDER_BALANCE: u64 = 1_000_000_000; // 1 SOL minimum to be considered a holder
const PAYOUT_INTERVAL: i64 = 86400; // 24 hours in seconds
const RENT: u64 = 967440;

#[program]
pub mod slots {
    use super::*;

    pub fn init(ctx: Context<CreateVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.spin = 0;
        vault.seed = RENT;
        vault.total_holder_rewards = 0;
        vault.last_payout_time = Clock::get()?.unix_timestamp;
        
        msg!("Initiated pda vault with key {}", vault.to_account_info().key);
        Ok(())
    }

    pub fn init_holder_registry(ctx: Context<CreateHolderRegistry>) -> Result<()> {
        let registry = &mut ctx.accounts.holder_registry;
        registry.holders = Vec::new();
        registry.last_updated = Clock::get()?.unix_timestamp;
        Ok(())
    }

    pub fn create_user_vault(ctx: Context<CreateUserVault>) -> Result<()> {
        let user_vault = &mut ctx.accounts.user_vault;
        user_vault.rewards_claimed = 0;
        
        msg!("Initiated user vault with key {}", ctx.accounts.user_vault.to_account_info().key);
        Ok(())
    }

    pub fn register_as_holder(ctx: Context<RegisterHolder>) -> Result<()> {
        let registry = &mut ctx.accounts.holder_registry;
        let signer = ctx.accounts.signer.key();
        
        // Check if signer has minimum balance
        require!(
            ctx.accounts.signer.lamports() >= MIN_HOLDER_BALANCE,
            ErrorCode::InsufficientHolderBalance
        );
        
        // Add to registry if not already present
        if !registry.holders.contains(&signer) {
            registry.holders.push(signer);
            registry.last_updated = Clock::get()?.unix_timestamp;
        }
        
        Ok(())
    }

    pub fn distribute_holder_rewards(ctx: Context<DistributeRewards>) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;
        
        // Extract values we need before any mutable operations
        let last_payout_time = ctx.accounts.vault.last_payout_time;
        let total_rewards = ctx.accounts.vault.total_holder_rewards;
        let holder_count = ctx.accounts.holder_registry.holders.len();
        let holder_key = ctx.accounts.signer.key();
        
        // Perform validations
        require!(
            current_time >= last_payout_time + PAYOUT_INTERVAL,
            ErrorCode::PayoutTooEarly
        );
        require!(total_rewards > 0, ErrorCode::NoRewardsToDistribute);
        require!(holder_count > 0, ErrorCode::NoHoldersRegistered);
        require!(
            ctx.accounts.holder_registry.holders.contains(&holder_key),
            ErrorCode::NotRegisteredHolder
        );
        
        // Calculate reward
        let reward_per_holder = total_rewards / holder_count as u64;
        
        // Transfer rewards
        let vault_info = &ctx.accounts.vault.to_account_info();
        let holder_vault_info = &ctx.accounts.user_vault.to_account_info();
        
        **vault_info.try_borrow_mut_lamports()? -= reward_per_holder;
        **holder_vault_info.try_borrow_mut_lamports()? += reward_per_holder;
        
        // Update vault state
        ctx.accounts.vault.total_holder_rewards = total_rewards.checked_sub(reward_per_holder).unwrap();
        
        if ctx.accounts.vault.total_holder_rewards == 0 {
            ctx.accounts.vault.last_payout_time = current_time;
        }
        
        Ok(())
    }

    pub fn spin(ctx: Context<Spin>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.spin += 1;

        let mut seed = vault.seed;
        seed ^= seed >> 12;
        seed ^= seed << 25;
        seed ^= seed >> 27;
        seed *= 0x2545F4914F6CDD1D;

        vault.seed = seed;

        let win_decider = seed % 20;
        let mut win = 0;
        let mut win_amount: u64 = 0;

        if win_decider > 17 {
            // mega win
            win = 3;
            win_amount = BET_AMOUNT * 2;
        } else if win_decider > 14 {
            // big win
            win = 2;
            win_amount = BET_AMOUNT;
        } else if win_decider > 8 {
            // small win
            win = 1;
            win_amount = BET_AMOUNT / 2;
        }

        msg!("This is spin #{}, result: {} - {}", vault.spin, win_decider, win);

        // Send bet amount to vault
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.signer.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, BET_AMOUNT)?;

        // If won, calculate holder rewards and user winnings
        if win > 0 {
            let holder_reward = (win_amount as f64 * HOLDER_REWARD_PERCENTAGE as f64 / 100.0) as u64;
            let user_winnings = win_amount - holder_reward;
            
            // Update holder rewards pool
            vault.total_holder_rewards = vault.total_holder_rewards.checked_add(holder_reward).unwrap();
            
            // Transfer user winnings to their vault
            **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= user_winnings;
            **ctx.accounts.user_vault.to_account_info().try_borrow_mut_lamports()? += user_winnings;
        }

        Ok(())
    }

    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        let user_vault_lamports = ctx.accounts.user_vault.to_account_info().lamports();
        let signer_lamports = ctx.accounts.signer.to_account_info().lamports();
        let claimable = user_vault_lamports.checked_sub(RENT).unwrap();

        **ctx.accounts.user_vault.to_account_info().try_borrow_mut_lamports()? = RENT;
        **ctx.accounts.signer.to_account_info().try_borrow_mut_lamports()? = signer_lamports.checked_add(claimable).unwrap();

        Ok(())
    }
}

#[account]
pub struct Vault {
    spin: u16,
    seed: u64,
    total_holder_rewards: u64,
    last_payout_time: i64,
}

#[account]
pub struct UserVault {
    rewards_claimed: u64,
}

#[account]
pub struct HolderRegistry {
    holders: Vec<Pubkey>,
    last_updated: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient balance to register as holder")]
    InsufficientHolderBalance,
    #[msg("No rewards available to distribute")]
    NoRewardsToDistribute,
    #[msg("No holders registered")]
    NoHoldersRegistered,
    #[msg("Not enough time has passed since last payout")]
    PayoutTooEarly,
    #[msg("Signer is not a registered holder")]
    NotRegisteredHolder,
}

#[derive(Accounts)]
pub struct Spin<'info> {
    #[account(mut, seeds = [TREASURY_PDA_SEED], bump)]
    pub vault: Account<'info, Vault>,
    #[account(mut, seeds = [USER_VAULT_SEED, signer.key().as_ref()], bump)]
    pub user_vault: Account<'info, UserVault>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut, seeds = [USER_VAULT_SEED, signer.key().as_ref()], bump)]
    pub user_vault: Account<'info, UserVault>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateVault<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = 8 + 2 + 8 + 8 + 8,
        seeds = [TREASURY_PDA_SEED],
        bump
    )]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateUserVault<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = 8 + 8,
        seeds = [USER_VAULT_SEED, signer.key().as_ref()],
        bump
    )]
    pub user_vault: Account<'info, UserVault>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateHolderRegistry<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = 8 + 32 * 100 + 8, // Space for up to 100 holders
        seeds = [HOLDER_REGISTRY_SEED],
        bump
    )]
    pub holder_registry: Account<'info, HolderRegistry>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterHolder<'info> {
    #[account(mut)]
    pub holder_registry: Account<'info, HolderRegistry>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DistributeRewards<'info> {
    #[account(mut, seeds = [TREASURY_PDA_SEED], bump)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub holder_registry: Account<'info, HolderRegistry>,
    #[account(mut)]
    pub user_vault: Account<'info, UserVault>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
