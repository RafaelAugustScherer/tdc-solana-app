use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    constants::{DELEGATE_SEED, DELEGATION_SEED, PLAN_SEED, SUBSCRIPTION_SEED},
    delegation,
    error::SubscriptionError,
    state::{Plan, SubscriberDelegation, Subscription},
};

#[derive(Accounts)]
pub struct SetAllowance<'info> {
    pub subscriber: Signer<'info>,

    #[account(
        seeds = [PLAN_SEED, plan.merchant.as_ref(), &plan.plan_id.to_le_bytes()],
        bump = plan.bump,
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
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

impl SetAllowance<'_> {
    pub fn run(&mut self, new_allowance: u64) -> Result<()> {
        require!(new_allowance > 0, SubscriptionError::InvalidAllowance);

        delegation::require_not_foreign(&self.subscriber_token_account, &self.delegate_authority)?;

        let previous = self.subscription.allowance_remaining;
        let committed_total = if new_allowance >= previous {
            self.subscriber_delegation
                .committed_total
                .checked_add(new_allowance - previous)
                .ok_or(SubscriptionError::AllowanceOverflow)?
        } else {
            self.subscriber_delegation
                .committed_total
                .saturating_sub(previous - new_allowance)
        };

        self.subscriber_delegation.committed_total = committed_total;
        self.subscription.allowance_remaining = new_allowance;

        delegation::approve(
            &self.token_program,
            &self.subscriber_token_account,
            &self.delegate_authority,
            &self.subscriber,
            committed_total,
        )
    }
}
