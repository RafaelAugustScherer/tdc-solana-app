use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    constants::{DELEGATE_SEED, DELEGATION_SEED, PLAN_SEED, SUBSCRIPTION_SEED},
    delegation,
    state::{Plan, SubscriberDelegation, Subscription},
};

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub subscriber: Signer<'info>,

    #[account(
        seeds = [PLAN_SEED, plan.merchant.as_ref(), &plan.plan_id.to_le_bytes()],
        bump = plan.bump,
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
        close = subscriber,
        seeds = [SUBSCRIPTION_SEED, plan.key().as_ref(), subscriber.key().as_ref()],
        bump = subscription.bump,
        has_one = plan,
        has_one = subscriber,
    )]
    pub subscription: Account<'info, Subscription>,

    #[account(
        mut,
        seeds = [DELEGATION_SEED, subscriber.key().as_ref(), plan.mint.as_ref()],
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

    #[account(address = plan.mint)]
    pub mint: Account<'info, Mint>,

    /// CHECK: seeds-derived delegate authority; named as the delegate, holds no data
    #[account(seeds = [DELEGATE_SEED], bump)]
    pub delegate_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

impl Cancel<'_> {
    pub fn run(&mut self) -> Result<()> {
        let committed_total = self
            .subscriber_delegation
            .committed_total
            .saturating_sub(self.subscription.allowance_remaining);
        self.subscriber_delegation.committed_total = committed_total;

        if delegation::is_held_by(&self.subscriber_token_account, &self.delegate_authority) {
            delegation::approve(
                &self.token_program,
                &self.subscriber_token_account,
                &self.delegate_authority,
                &self.subscriber,
                committed_total,
            )?;
        }

        Ok(())
    }
}
