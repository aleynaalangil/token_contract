#![allow(clippy::result_large_err)]
use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenInterface;

declare_id!("2dae2BYY9kVVDfW4bjprmmrZv6b4PJvudeh4hSJATtS4");

#[error_code]
pub enum CustomError {
    #[msg("You are not the company authority")]
    Unauthorized,
}

#[program]
pub mod token_contract {
    use super::*;

    /// Initialize the Company with name, symbol, total supply, etc.
    pub fn initialize_company(
        ctx: Context<InitializeCompany>,
        name: String,
        symbol: String,
        total_supply: u128,
        token_mint: Pubkey,
        treasury: Pubkey,
    ) -> Result<()> {
        let company = &mut ctx.accounts.company;
        company.authority = ctx.accounts.payer.key();
        company.name = name;
        company.symbol = symbol;
        company.total_supply = total_supply;
        company.token_mint = token_mint;
        company.treasury = treasury;

        msg!("Company initialized successfully");
        Ok(())
    }

    /// Add a new Shareholder by creating a "Shareholder" account (one-PDA-per-shareholder).
    pub fn add_shareholder_by_company(
        ctx: Context<AddShareholderByCompany>,
        shareholder_pk: Pubkey,
        voting_power: u128,
    ) -> Result<()> {
        let shareholder = &mut ctx.accounts.shareholder;
        let company = &mut ctx.accounts.company;

        shareholder.owner = shareholder_pk;
        shareholder.voting_power = voting_power;
        shareholder.company = company.key();

        msg!("Shareholder PDA created for pubkey={}", shareholder_pk);
        Ok(())
    }

    /// Remove a Shareholder by closing the Shareholder account.
    pub fn remove_shareholder_by_company(ctx: Context<RemoveShareholder>) -> Result<()> {
        let company = &ctx.accounts.company;
        require_keys_eq!(company.authority, ctx.accounts.authority.key(), CustomError::Unauthorized);

        msg!("Shareholder {} removed from company {}.",
            ctx.accounts.shareholder.key(), 
            company.key()
        );
        Ok(())
    }

    /// Delegate vote rights by changing the `owner` on the Shareholder account
    /// (and maybe adjusting voting_power).
    // pub fn delegate_vote_rights(
    //     ctx: Context<DelegateVoteRights>,
    //     new_delegated_to: Pubkey,
    //     new_voting_power: u128,
    // ) -> Result<()> {
    //     let shareholder = &mut ctx.accounts.shareholder;
    //     shareholder.owner = new_delegated_to;
    //     shareholder.voting_power = new_voting_power;

    //     msg!("Delegation complete; new owner = {}", new_delegated_to);
    //     Ok(())
    // }
    pub fn delegate_vote_rights(
        ctx: Context<DelegateVoteRights>,
        new_delegated_to: Pubkey,
        shareholder_voting_power: u128,
        company: Pubkey,
    ) -> Result<()> {
        let shareholder = &mut ctx.accounts.shareholder;
    
        // The new delegated wallet is set both as `delegated_to` and also becomes `owner`.
        // So instructions that do `#[account(mut, has_one = owner)]` will expect `new_delegated_to` to sign.
        // shareholder.delegated_to = new_delegated_to;
        shareholder.owner = new_delegated_to;

    
        // Assign the given voting power
        shareholder.voting_power = shareholder_voting_power;
        shareholder.company = company;
    
        msg!("Shareholder delegated successfully");
    
        Ok(())
    }

     /// Create a poll with a list of `options`.
     pub fn initialize_poll(ctx: Context<InitializePoll>, options: Vec<String>) -> Result<()> {
        ctx.accounts.poll.init(options)?;
        Ok(())
    }

    /// Cast a vote. 
    /// - Each user that votes must create a unique `VoteRecord` to prove they haven't voted yet.
    pub fn vote(ctx: Context<Vote>, vote_id: u8, voting_power: u64) -> Result<()> {
        let poll = &mut ctx.accounts.poll;
        poll.vote(vote_id, voting_power)?;

        let record = &mut ctx.accounts.vote_record;
        record.poll = poll.key();
        record.voter = ctx.accounts.voter.key();
        record.voted_option = vote_id;

        Ok(())
    }

    /// Finish the poll. If there's a tie among multiple top options, create a new poll 
    /// containing only those tied options.
    pub fn finish_poll(ctx: Context<FinishPoll>) -> Result<()> {
        let old_poll = &mut ctx.accounts.old_poll;
        require!(!old_poll.finished, PollError::PollAlreadyFinished);

        let (winners, max_votes) = old_poll.calculate_winners();
        msg!(
            "Finishing poll {}, max_votes={}",
            old_poll.key(),
            max_votes
        );
        if winners.len() > 1 {
            // We have a tie => create a new poll
            let tie_poll = &mut ctx.accounts.tie_break_poll;
            tie_poll.init_from_previous(&winners)?;
            msg!(
                "Tie among {} options => new tie-break poll created: {}",
                winners.len(),
                tie_poll.key()
            );
        } else {
            msg!(
                "Single winner: option '{}' with {} votes",
                winners[0].label,
                winners[0].votes
            );
        }

        old_poll.finished = true;
        Ok(())
    }
}

// ---------------------------------------
//        ACCOUNTS & DATA STRUCTS
// ---------------------------------------

#[account]
pub struct Company {
    pub authority: Pubkey,
    pub name: String,
    pub symbol: String,
    pub total_supply: u128,
    pub token_mint: Pubkey,
    pub treasury: Pubkey,
}
impl Company {
    pub const MAX_SIZE: usize = 
        32 +           // authority
        (4 + 50) +     // name (up to 50 chars)
        (4 + 10) +     // symbol (up to 10 chars)
        16 +           // total_supply (u128)
        32 +           // token_mint
        32;            // treasury
}

#[account]
pub struct Shareholder {
    pub owner: Pubkey,
    pub voting_power: u128,
    pub company: Pubkey, // points back to the Company
}
impl Shareholder {
    pub const MAX_SIZE: usize = 32 + 16 + 32;
}

// 1) InitializeCompany
#[derive(Accounts)]
pub struct InitializeCompany<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Company::MAX_SIZE,
        seeds = [b"company", payer.key().as_ref()],
        bump
    )]
    pub company: Account<'info, Company>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

// 2) AddShareholderByCompany
#[derive(Accounts)]
#[instruction(shareholder_pk: Pubkey, voting_power: u128)]
pub struct AddShareholderByCompany<'info> {
    #[account(mut)]
    pub company: Account<'info, Company>,

    #[account(
        init,
        payer = payer,
        space = 8 + Shareholder::MAX_SIZE,
        seeds = [b"shareholder", shareholder_pk.as_ref()],
        bump
    )]
    pub shareholder: Account<'info, Shareholder>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// 3) RemoveShareholder
#[derive(Accounts)]
pub struct RemoveShareholder<'info> {
    #[account(mut, has_one = authority)]
    pub company: Account<'info, Company>,

    #[account(
        mut,
        has_one = company,
        close = authority
    )]
    pub shareholder: Account<'info, Shareholder>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

// 4) DelegateVoteRights
#[derive(Accounts)]
pub struct DelegateVoteRights<'info> {
    #[account(mut)]
    pub company: Account<'info, Company>,  // optional check

    #[account(mut, has_one = company)]
    pub shareholder: Account<'info, Shareholder>,

    #[account(mut)]
    pub payer: Signer<'info>, // if you need to pay for reallocation, etc.
    pub system_program: Program<'info, System>,
}

// ----------------------------------
//   Poll, Options, and VoteRecord
// ----------------------------------
#[account]
pub struct Poll {
    pub options: Vec<PollOption>,
    pub finished: bool,
}

impl Poll {
    pub const MAX_SIZE: usize = 2048; // adjust as needed

    pub fn init(&mut self, options: Vec<String>) -> Result<()> {
        require!(!self.finished, PollError::PollAlreadyFinished);
        let mut idx = 0u8;
        self.options = options
            .into_iter()
            .map(|label| {
                idx += 1;
                PollOption {
                    id: idx,
                    label,
                    votes: 0,
                }
            })
            .collect();
        self.finished = false;
        Ok(())
    }

    pub fn vote(&mut self, vote_id: u8, voting_power: u64) -> Result<()> {
        require!(!self.finished, PollError::PollAlreadyFinished);

        let Some(opt) = self.options.iter_mut().find(|o| o.id == vote_id) 
            else { return Err(PollError::PollOptionNotFound.into()); };

        opt.votes = opt.votes.checked_add(voting_power).ok_or(PollError::Overflow)?;
        Ok(())
    }

    pub fn calculate_winners(&self) -> (Vec<PollOption>, u64) {
        let mut max_votes = 0u64;
        for o in &self.options {
            if o.votes > max_votes {
                max_votes = o.votes;
            }
        }
        let winners = self.options
            .iter()
            .filter(|o| o.votes == max_votes)
            .cloned()
            .collect();
        (winners, max_votes)
    }

    // For tie-break: create a new poll with only the winning (tied) options
    pub fn init_from_previous(&mut self, winners: &Vec<PollOption>) -> Result<()> {
        self.finished = false;
        self.options = winners.iter().map(|o| PollOption {
            id: o.id,
            label: o.label.clone(),
            votes: 0,
        }).collect();
        Ok(())
    }
}

#[derive(Clone, AnchorDeserialize, AnchorSerialize)]
pub struct PollOption {
    pub id: u8,
    pub label: String,
    pub votes: u64,
}

/// Each user must create a VoteRecord with seeds = [b"vote-record", pollPubkey, userPubkey]
/// to ensure they do not vote twice.
#[account]
pub struct VoteRecord {
    pub poll: Pubkey,
    pub voter: Pubkey,
    pub voted_option: u8,
}
impl VoteRecord {
    pub const MAX_SIZE: usize = 32 + 32 + 1;
}

// ---------------------------
//      ERROR CODES
// ---------------------------
#[error_code]
pub enum PollError {
    #[msg("Poll is already finished")]
    PollAlreadyFinished,
    #[msg("Poll option not found")]
    PollOptionNotFound,
    #[msg("Arithmetic overflow")]
    Overflow,
}

// ---------------------------
//    ACCOUNTS STRUCTS
// ---------------------------
#[derive(Accounts)]
pub struct InitializePoll<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Poll::MAX_SIZE
    )]
    pub poll: Account<'info, Poll>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Vote<'info> {
    #[account(mut)]
    pub poll: Account<'info, Poll>,

    #[account(mut)]
    pub voter: Signer<'info>,

    // We ensure the user hasn't voted by creating a new VoteRecord.
    // If it already exists, "init" will fail.
    #[account(
        init,
        payer = voter,
        space = 8 + VoteRecord::MAX_SIZE,
        seeds = [b"vote-record", poll.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinishPoll<'info> {
    // The old poll
    #[account(mut)]
    pub old_poll: Account<'info, Poll>,

    // The brand-new poll in case we need a tie-break
    #[account(
        init,
        payer = payer,
        space = 8 + Poll::MAX_SIZE
    )]
    pub tie_break_poll: Account<'info, Poll>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}