#![allow(clippy::result_large_err)]
use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenInterface;

declare_id!("Ajf6V1JscbNyFc5zyhm3DvDcZrNeqMvYsji2CrKvsum1");

#[program]
pub mod token_contract {
    use super::*;

    pub fn initialize_poll(ctx: Context<InitializePoll>, options: Vec<String>) -> Result<()> {
        ctx.accounts.poll.init(options)
    }

    pub fn vote(ctx: Context<Vote>, vote_id: u8, shareholder_owner: Pubkey, shareholder_voting_power: u128) -> Result<()> {
        let poll = &mut ctx.accounts.poll;
        // Use the Poll's `vote` method
        poll.vote(vote_id, shareholder_owner, shareholder_voting_power)
    }
    
    pub fn tally_votes(ctx: Context<TallyVotes>) -> Result<()> {
        let mut max_votes = 0;
        let mut winner_label = String::new();
        let poll_account = &ctx.accounts.poll;
    
        // Just print them to the program logs
        for option in &poll_account.options {
            msg!("Option {} has {} votes", option.id, option.votes);
            if option.votes > max_votes {
                max_votes = option.votes;
                winner_label = option.label.clone();
            }
        }
        msg!("The winner is option {}", winner_label);
    
        Ok(())
    }

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
        company.shareholder_count = 0;
        company.treasury = treasury;

        msg!("Company initialized successfully with token mint via token_contract");

        Ok(())
    }

    pub fn add_shareholder_by_company(
        ctx: Context<AddShareholderByCompany>, // company: Pubkey,
        shareholder_pk: Pubkey,
        voting_power: u128,
    ) -> Result<()> {
        let shareholder = &mut ctx.accounts.shareholder;
        shareholder.owner = shareholder_pk;
        shareholder.voting_power = voting_power;
        // shareholder.delegated_to = shareholder_pk; //change this to delegated_from and put the old shareholder_pk here
        let company = &mut ctx.accounts.company;
        company.shareholder_count += 1;
        shareholder.company = company.key();
        msg!("Shareholder added successfully");

        Ok(())
    }

    pub fn remove_shareholder(ctx: Context<RemoveShareholder>) -> Result<()> {
        let company = &mut ctx.accounts.company;
        let shareholder = &mut ctx.accounts.shareholder;
    
        // Ensure the signer's key matches the company's `authority` field
        require_keys_eq!(
            company.authority,
            ctx.accounts.authority.key(),
            CustomError::Unauthorized
        );
    
        // Decrement total count of shareholders
        require!(company.shareholder_count > 0, CustomError::Underflow);
        company.shareholder_count -= 1;
        
        // Optionally set the shareholder's voting_power to 0
        shareholder.voting_power = 0;
    
        // Optionally set the owner + delegated_to to some inert address, e.g. the company or default
        shareholder.owner = Pubkey::default();       // or Pubkey::default()
        // shareholder.delegated_to = Pubkey::default(); // or Pubkey::default()
    
        msg!("Shareholder removed from company. Company share count is now: {}", company.shareholder_count);
        Ok(())
    }

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

    pub fn finish_poll(ctx: Context<FinishPoll>) -> Result<()> {
        let poll = &mut ctx.accounts.poll;
        require_eq!(poll.finished, false,  PollError::PollAlreadyFinished);
    
        // The logic for finishing the poll, e.g. collecting final votes, etc.
        poll.finished = true;
    
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeCompany<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Company::MAX_SIZE,
        seeds = [b"company", payer.key().as_ref()],
        bump,
    )]
    pub company: Account<'info, Company>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct AddShareholderByShareholder<'info> {
    #[account(mut)]
    pub company: Account<'info, Company>,
    #[account(
        init,
        payer = payer,
        space = 8 + Shareholder::MAX_SIZE
    )]
    pub shareholder: Account<'info, Shareholder>,
    #[account(mut)]
    pub payer: Signer<'info>, //shareholder
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddShareholderByCompany<'info> {
    #[account(mut )]
    pub company: Account<'info, Company>,
    #[account(
        init,
        payer = payer,
        space = 8 + Shareholder::MAX_SIZE
    )]
    pub shareholder: Account<'info, Shareholder>,
    #[account(mut, signer)]
    pub payer: Signer<'info>, //company
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveShareholder<'info> {
    // The company from which we remove the shareholder
    #[account(
        mut,
        has_one = authority,
    )]
    pub company: Account<'info, Company>,

    // The specific Shareholder account to remove
    // Make sure it also belongs to the same company.
    #[account(
        mut,
        has_one = company  // ensures shareholder.company == company.key()
    )]
    pub shareholder: Account<'info, Shareholder>,

    #[account(mut)]
    pub authority: Signer<'info>, // The same wallet as company.authority
}

#[derive(Accounts)]
pub struct DelegateVoteRights<'info> {
    #[account(mut)]
    pub company: Account<'info, Company>,

    #[account(
        init,
        payer = payer,
        space = 8 + Shareholder::MAX_SIZE
    )]
    pub shareholder: Account<'info, Shareholder>,

    #[account(mut)]
    pub payer: Signer<'info>, // Must be the wallet paying for the new account creation
    pub system_program: Program<'info, System>,
}

// Data Structures
#[account]
pub struct Company {
    pub authority: Pubkey,
    pub name: String,
    pub symbol: String,
    pub total_supply: u128,
    pub token_mint: Pubkey,
    pub shareholder_count: u32,
    pub treasury: Pubkey,
}

impl Company {
    pub const MAX_SIZE: usize = 32 + 4 + 32 + 16 + 32 + 4 + 32;
}

#[account]
pub struct Shareholder {
    pub owner: Pubkey, //shareholder
    pub voting_power: u128,
    // pub delegated_to: Pubkey, //self or another shareholder
    pub company: Pubkey,
}

impl Shareholder {
    pub const MAX_SIZE: usize = 32 + 32 + 16+ 32;
}

#[error_code]
pub enum CustomError {
    #[msg("You are not the company authority")]
    Unauthorized,
    #[msg("Shareholder count underflow")]
    Underflow,
}

// Vote

impl Poll {
    pub const MAXIMUM_SIZE: usize = 1904;

    pub fn init(&mut self, options: Vec<String>) -> Result<()> {
        require_eq!(self.finished, false, PollError::PollAlreadyFinished);
        let mut c = 0;

        self.options = options
            .iter()
            .map(|option| {
                c += 1;

                PollOption {
                    label: option.clone(),
                    id: c,
                    votes: 0,
                }
            })
            .collect();
        self.finished = false;
        Ok(())
    }
    pub fn vote(&mut self, vote_id: u8, voter_key: Pubkey, voting_power: u128) -> Result<()> {
        // Check if the poll is still active
        require_eq!(self.finished, false, PollError::PollAlreadyFinished);

        // Validate if the vote ID corresponds to a valid option
        require_eq!(
            self.options.iter().any(|option| option.id == vote_id),
            true,
            PollError::PollOptionNotFound
        );

        // Ensure the voter has not voted already
        require!(
            !self.voters.iter().any(|voter| voter == &voter_key),
            PollError::UserAlreadyVoted
        );

        // Add the voter to the list
        self.voters.push(voter_key);

        // Update the votes for the selected option, weighted by voting power
        self.options.iter_mut().for_each(|option| {
            if option.id == vote_id {
                option.votes += voting_power as u64; // Assume `votes` uses `u64`
            }
        });

        msg!("Vote successfully cast with weight: {}", voting_power);

        Ok(())
    }
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PollOption {
    pub label: String, // up to 50 char
    pub id: u8,        // Option ID
    pub votes: u64,    // Updated to u64 for larger voting counts
}

#[account]
pub struct Poll {
    // Size: 1 + 299 + 1604 = 1904
    pub options: Vec<PollOption>, // 5 PollOption array = 4 + (59 * 5) = 299
    pub voters: Vec<Pubkey>,      // 50 voters array = 4 + (32 * 50) = 1604
    pub finished: bool,           // bool = 1
}

#[error_code]
pub enum PollError {
    PollAlreadyFinished,
    PollOptionNotFound,
    UserAlreadyVoted,
}

#[derive(Accounts)]
pub struct InitializePoll<'info> {
    #[account(init, payer = owner, space = 8 + Poll::MAXIMUM_SIZE)]
    pub poll: Account<'info, Poll>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct Vote<'info> {
    #[account(mut)]
    pub poll: Account<'info, Poll>,
    #[account(mut, has_one = owner)] // Ensure the shareholder is valid
    pub shareholder: Account<'info, Shareholder>,
    pub owner: Signer<'info>, // The voter must sign the transaction
}

#[derive(Accounts)]
pub struct FinishPoll<'info> {
    #[account(mut)]
    pub poll: Account<'info, Poll>,
}

#[derive(Accounts)]
pub struct TallyVotes<'info> {
    #[account(mut)]
    pub poll: Account<'info, Poll>,
}