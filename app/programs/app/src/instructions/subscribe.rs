use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    constants::{DELEGATE_SEED, DELEGATION_SEED, PLAN_SEED, SUBSCRIPTION_SEED},
    delegation,
    error::SubscriptionError,
    state::{Plan, SubscriberDelegation, Subscription},
};

#[derive(Accounts)]
pub struct Subscribe<'info> {
    #[account(mut)]
    pub subscriber: Signer<'info>,

    #[account(
        seeds = [PLAN_SEED, plan.merchant.as_ref(), &plan.plan_id.to_le_bytes()],
        bump = plan.bump,
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        init,
        payer = subscriber,
        space = 8 + Subscription::INIT_SPACE,
        seeds = [SUBSCRIPTION_SEED, plan.key().as_ref(), subscriber.key().as_ref()],
        bump,
    )]
    pub subscription: Account<'info, Subscription>,

    #[account(
        init_if_needed,
        payer = subscriber,
        space = 8 + SubscriberDelegation::INIT_SPACE,
        seeds = [DELEGATION_SEED, subscriber.key().as_ref(), plan.mint.as_ref()],
        bump,
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
    pub system_program: Program<'info, System>,
}

impl Subscribe<'_> {
    pub fn run(
        &mut self,
        allowance: u64,
        max_amount_per_period: u64,
        subscription_bump: u8,
        delegation_bump: u8,
    ) -> Result<()> {
        require!(allowance > 0, SubscriptionError::InvalidAllowance);
        require!(
            max_amount_per_period > 0,
            SubscriptionError::InvalidMaxAmount
        );
        require!(self.plan.is_active, SubscriptionError::PlanInactive);

        let now = Clock::get()?.unix_timestamp;
        require!(
            self.plan.applicable_amount(now)? <= max_amount_per_period,
            SubscriptionError::PriceAboveSubscriberMax
        );

        delegation::require_not_foreign(&self.subscriber_token_account, &self.delegate_authority)?;

        let committed_total = self
            .subscriber_delegation
            .committed_total
            .checked_add(allowance)
            .ok_or(SubscriptionError::AllowanceOverflow)?;

        self.subscriber_delegation.set_inner(SubscriberDelegation {
            subscriber: self.subscriber.key(),
            mint: self.plan.mint,
            committed_total,
            bump: delegation_bump,
        });

        delegation::approve(
            &self.token_program,
            &self.subscriber_token_account,
            &self.delegate_authority,
            &self.subscriber,
            committed_total,
        )?;

        self.subscription.set_inner(Subscription {
            plan: self.plan.key(),
            subscriber: self.subscriber.key(),
            next_charge_at: now,
            allowance_remaining: allowance,
            max_amount_per_period,
            bump: subscription_bump,
        });

        Ok(())
    }
}
