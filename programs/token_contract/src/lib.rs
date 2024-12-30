#![allow(clippy::result_large_err)]
use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenInterface;

declare_id!("HKj8pzMK6w6pSbdtzj1Q315xFrtpMWypnzzUK4JV6SSB");

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
        shareholder.delegated_to = shareholder_pk;
        shareholder.is_whitelisted = true;
        let company = &mut ctx.accounts.company;
        company.shareholder_count += 1;
        shareholder.company = company.key();
        msg!("Shareholder added successfully");

        Ok(())
    }

    // pub fn add_shareholder_by_shareholder(
    //     ctx: Context<AddShareholderByShareholder>,
    //     voting_power: u128,
    //     shareholder_pk: Pubkey,
    // ) -> Result<()> {
    //     let shareholder = &mut ctx.accounts.shareholder;
    //     shareholder.owner = ctx.accounts.payer.key();
    //     shareholder.voting_power = voting_power;
    //     shareholder.delegated_to = shareholder_pk;
    //     shareholder.is_whitelisted = true;

    //     let company = &mut ctx.accounts.company;
    //     company.shareholder_count += 1;
    //     shareholder.company = company.key();

    //     msg!("Shareholder added successfully");

    //     Ok(())
    // }

    pub fn remove_shareholder(
        //TODO: WHAT ABOUT HIS VOTING POWER? u should transfer the tokens back to company or the shareholder to be delegated!!!
        ctx: Context<RemoveShareholder>,
    ) -> Result<()> {
        let shareholder = &mut ctx.accounts.shareholder;
        shareholder.delegated_to = ctx.accounts.payer.key(); //company wallet
        shareholder.is_whitelisted = false;

        let company = &mut ctx.accounts.company;
        company.shareholder_count -= 1;

        msg!("Shareholder removed successfully");

        Ok(())
    }
    
    pub fn finish_poll(ctx: Context<FinishPoll>) -> Result<()> {
        let poll = &mut ctx.accounts.poll;
        require_eq!(poll.finished, false, StarSollError::PollAlreadyFinished);
    
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

// #[derive(Accounts)]
// pub struct AddShareholderByShareholder<'info> {
//     #[account(mut)]
//     pub company: Account<'info, Company>,
//     #[account(
//         init,
//         payer = payer,
//         space = 8 + Shareholder::MAX_SIZE
//     )]
//     pub shareholder: Account<'info, Shareholder>,
//     #[account(mut)]
//     pub payer: Signer<'info>, //shareholder
//     pub system_program: Program<'info, System>,
// }

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
    #[account(mut)]
    pub company: Account<'info, Company>,
    #[account(
        init,
        payer = payer,
        space = 8 + Shareholder::MAX_SIZE
    )]
    pub shareholder: Account<'info, Shareholder>,
    #[account(mut)]
    pub payer: Signer<'info>, //company
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
    pub delegated_to: Pubkey, //self or another shareholder
    pub is_whitelisted: bool,
    pub company: Pubkey,
}

impl Shareholder {
    pub const MAX_SIZE: usize = 32 + 32 + 16 + 32 + 1 + 32;
}

// Vote

impl Poll {
    pub const MAXIMUM_SIZE: usize = 1904;

    pub fn init(&mut self, options: Vec<String>) -> Result<()> {
        require_eq!(self.finished, false, StarSollError::PollAlreadyFinished);
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
        require_eq!(self.finished, false, StarSollError::PollAlreadyFinished);

        // Validate if the vote ID corresponds to a valid option
        require_eq!(
            self.options.iter().any(|option| option.id == vote_id),
            true,
            StarSollError::PollOptionNotFound
        );

        // Ensure the voter has not voted already
        require!(
            !self.voters.iter().any(|voter| voter == &voter_key),
            StarSollError::UserAlreadyVoted
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
pub enum StarSollError {
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
    // some signer or authority, etc.
}