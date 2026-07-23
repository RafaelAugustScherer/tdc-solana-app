use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    constants::{DELEGATE_SEED, DELEGATION_SEED},
    delegation,
    state::SubscriberDelegation,
};

#[derive(Accounts)]
pub struct Reauthorize<'info> {
    pub subscriber: Signer<'info>,

    #[account(
        seeds = [DELEGATION_SEED, subscriber.key().as_ref(), mint.key().as_ref()],
        bump = subscriber_delegation.bump,
        has_one = subscriber,
        has_one = mint,
    )]
    pub subscriber_delegation: Account<'info, SubscriberDelegation>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = subscriber,
    )]
    pub subscriber_token_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    /// CHECK: seeds-derived delegate authority; named as the delegate, holds no data
    #[account(seeds = [DELEGATE_SEED], bump)]
    pub delegate_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

impl Reauthorize<'_> {
    pub fn run(&mut self) -> Result<()> {
        delegation::require_not_foreign(&self.subscriber_token_account, &self.delegate_authority)?;

        delegation::approve(
            &self.token_program,
            &self.subscriber_token_account,
            &self.delegate_authority,
            &self.subscriber,
            self.subscriber_delegation.committed_total,
        )
    }
}
