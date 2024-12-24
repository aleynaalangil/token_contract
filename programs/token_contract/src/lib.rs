#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    self, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
};

declare_id!("2TE5kuPuKgoXoBEtDB6EhCvP5yVRJGiS2DnthZNw3oP4");

#[program]
pub mod token_contract { //token contract

    use super::*;

    pub fn create_token(_ctx: Context<CreateToken>, _token_name: String) -> Result<()> {
        msg!("Create Token");
        Ok(())
    }
    pub fn create_token_account(_ctx: Context<CreateTokenAccount>) -> Result<()> {
        msg!("Create Token Account");
        Ok(())
    }
    pub fn create_associated_token_account(
        _ctx: Context<CreateAssociatedTokenAccount>,
    ) -> Result<()> {
        msg!("Create Associated Token Account");
        Ok(())
    }
    pub fn transfer_token(ctx: Context<TransferToken>, amount: u64) -> Result<()> {
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.from.to_account_info().clone(),
            mint: ctx.accounts.mint.to_account_info().clone(),
            to: ctx.accounts.to_ata.to_account_info().clone(),
            authority: ctx.accounts.signer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
        token_interface::transfer_checked(cpi_context, amount, ctx.accounts.mint.decimals)?;
        msg!("Transfer Token");
        Ok(())
    }
    pub fn mint_token(ctx: Context<MintToken>, amount: u64) -> Result<()> {
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info().clone(),
            to: ctx.accounts.receiver.to_account_info().clone(),
            authority: ctx.accounts.signer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
        token_interface::mint_to(cpi_context, amount)?;
        msg!("Mint Token");
        Ok(())
    }

    pub fn initialize_company(
        ctx: Context<InitializeCompany>,
        name: String,
        symbol: [u8; 5],
        total_supply: u128,
        token_mint: Pubkey,
    ) -> Result<()> {
        // Create the company account
        let company = &mut ctx.accounts.company;
        //create a mint PDA from the company name
        // let token_mint = Pubkey::find_program_address(&[b"token-2022-token", ctx.accounts.payer.key().as_ref(), name.as_bytes()], &ctx.accounts.token_contract.key()).0;
        company.name = name;
        company.symbol = symbol;
        company.total_supply = total_supply;
        company.token_mint = token_mint;
        company.shareholder_count = 0;
        company.vote_account = ctx.accounts.vote_account.key();

        // Invoke the `create_token` function in `token_contract`
        let ctx = Context {
            program_id: &ctx.accounts.token_contract.key(),
            accounts: &mut CreateToken {
                signer: ctx.accounts.payer.clone(),
                mint: ctx.accounts.token_mint.clone(),
                system_program: ctx.accounts.system_program.clone(),
                token_program: ctx.accounts.token_program.clone(),
            },
            remaining_accounts: &[],
            bumps: CreateTokenBumps { mint: 0 },
        };

        let symbol_str = String::from_utf8(symbol.to_vec())
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotSerialize)?;
        create_token(ctx, symbol_str)?;

        msg!("Company initialized successfully with token mint via token_contract");

        Ok(())
    }

    pub fn add_shareholder(
        ctx: Context<AddShareholder>,
        _voting_power: u64,
        _is_whitelisted: bool,
    ) -> Result<()> {
        let shareholder = &mut ctx.accounts.shareholder;
        shareholder.owner = ctx.accounts.owner.key();
        shareholder.voting_power = 0;
        shareholder.delegated_to = Option::None;
        shareholder.is_whitelisted = false;

        let company = &mut ctx.accounts.company;
        company.shareholder_count += 1;

        msg!("Shareholder added successfully");

        Ok(())
    }

    pub fn update_shareholder_voting_power(
        ctx: Context<UpdateShareholder>,
        new_voting_power: u64,
    ) -> Result<()> {
        let shareholder = &mut ctx.accounts.shareholder;
        shareholder.voting_power = new_voting_power;

        msg!("Shareholder voting power updated successfully");

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(token_name: String)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 6,
        mint::authority = signer.key(),
        seeds = [b"token-2022-token", signer.key().as_ref(), token_name.as_bytes()],
        bump,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct CreateTokenAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        token::mint = mint,
        token::authority = signer,
        payer = signer,
        seeds = [b"token-2022-token-account", signer.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct CreateAssociatedTokenAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        associated_token::mint = mint,
        payer = signer,
        associated_token::authority = signer,
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]

pub struct TransferToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub from: InterfaceAccount<'info, TokenAccount>,
    pub to: SystemAccount<'info>,
    #[account(
        init,
        associated_token::mint = mint,
        payer = signer,
        associated_token::authority = to
    )]
    pub to_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct MintToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub receiver: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}

///new changes
/// 
// Accounts
#[derive(Accounts)]
pub struct InitializeCompany<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Company::MAX_SIZE
    )]
    pub company: Account<'info, Company>,
    #[account(mut)]
    pub token_mint: InterfaceAccount<'info, Mint>, // Correct type for mints
    #[account(mut)]
    /// CHECK: Manually validated account for vote account
    pub vote_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>, // Correct type for token program
    #[account(mut)]
    /// CHECK: This is the program that handles mint creation
    pub token_contract: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct AddShareholder<'info> {
    #[account(mut)]
    pub company: Account<'info, Company>,
    #[account(
        init,
        payer = payer,
        space = 8 + Shareholder::MAX_SIZE
    )]
    pub shareholder: Account<'info, Shareholder>,
    pub owner: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateShareholder<'info> {
    #[account(
        mut,
        has_one = owner
    )]
    pub shareholder: Account<'info, Shareholder>,
    pub owner: Signer<'info>,
}

// Data Structures
#[account]
pub struct Company {
    pub authority: Pubkey,
    pub name: String,
    pub symbol: [u8; 5],
    pub total_supply: u128,
    pub token_mint: Pubkey,
    pub shareholder_count: u32,
    pub vote_account: Pubkey,
}

impl Company {
    pub const MAX_SIZE: usize = 32 + 4 + 32 + 16 + 32 + 4 + 32; // Calculated max size
}

#[account]
pub struct Shareholder {
    pub owner: Pubkey,                // Shareholder's wallet
    pub voting_power: u64,            // Voting tokens owned
    pub delegated_to: Option<Pubkey>, // Delegation (optional)
    pub is_whitelisted: bool,         // Whitelisting status
}

impl Shareholder {
    pub const MAX_SIZE: usize = 32 + 8 + 1 + 1; // Calculated max size
}

impl<'info> InitializeCompany<'info> {
    pub fn into_mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.token_mint.to_account_info(),
            to: self.vote_account.to_account_info(),
            authority: self.payer.to_account_info(),
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}
