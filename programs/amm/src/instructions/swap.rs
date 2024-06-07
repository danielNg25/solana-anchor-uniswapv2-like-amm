// use crate::error::ErrorCode;
// use crate::instructions::mint_fee;
// use crate::state::Config;
// use crate::state::Pool;
// use anchor_lang::prelude::*;
// use anchor_spl::associated_token::AssociatedToken;
// use anchor_spl::token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer};
// use fixed::types::U128F0;
// use std::cmp::min;

// #[derive(Accounts)]
// pub struct Swap<'info> {
//     #[account( seeds = [b"config"], bump = config.bump)]
//     pub config: Box<Account<'info, Config>>,
//     pub mint0: Account<'info, Mint>,
//     pub mint1: Account<'info, Mint>,

//     #[account(mut)]
//     pub user: Signer<'info>,
//     #[account(
//         mut,
//         associated_token::mint = mint0,
//         associated_token::authority = user
//     )]
//     pub user_ata0: Box<Account<'info, TokenAccount>>,
//     #[account(
//         mut,
//         associated_token::mint = mint1,
//         associated_token::authority = user
//     )]
//     pub user_ata1: Box<Account<'info, TokenAccount>>,
//     #[account(
//         init_if_needed,
//         payer = user,
//         associated_token::mint = lp_mint,
//         associated_token::authority = user
//     )]
//     pub user_lp_ata: Box<Account<'info, TokenAccount>>,

//     #[account(mut, seeds= [b"pool", mint0.key().as_ref(), mint1.key().as_ref()], bump)]
//     pub pool: Box<Account<'info, Pool>>,

//     #[account(
//         init_if_needed,
//         payer = user,
//         associated_token::mint = lp_mint,
//         associated_token::authority = pool
//     )]
//     pub vault_lp: Box<Account<'info, TokenAccount>>,
//     #[account(
//         mut,
//         associated_token::mint = mint0,
//         associated_token::authority = pool
//     )]
//     pub vault0: Box<Account<'info, TokenAccount>>,
//     #[account(
//         mut,
//         associated_token::mint = mint1,
//         associated_token::authority = pool
//     )]
//     pub vault1: Box<Account<'info, TokenAccount>>,
//     #[account(mut, seeds = [b"lp_mint", pool.key().as_ref()], bump)]
//     pub lp_mint: Box<Account<'info, Mint>>,

//     pub associated_token_program: Program<'info, AssociatedToken>,
//     pub token_program: Program<'info, Token>,
//     pub system_program: Program<'info, System>,
// }

// pub fn swap(
//     ctx: Context<Swap>,
//     input_amount: u64,
//     output_amount: u64,
//     is_reverse: bool,
// ) -> Result<()> {
//     let pool: &Box<Account<Pool>> = &ctx.accounts.pool;

//     Ok(())
// }
