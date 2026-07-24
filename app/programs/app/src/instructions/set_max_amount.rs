use anchor_lang::prelude::*;

use crate::{
    constants::{PLAN_SEED, SUBSCRIPTION_SEED},
    error::SubscriptionError,
    state::{Plan, Subscription},
};

#[derive(Accounts)]
pub struct SetMaxAmount<'info> {
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
}

impl SetMaxAmount<'_> {
    pub fn run(&mut self, new_max: u64) -> Result<()> {
        require!(new_max > 0, SubscriptionError::InvalidMaxAmount);

        self.subscription.max_amount_per_period = new_max;

        Ok(())
    }
}
