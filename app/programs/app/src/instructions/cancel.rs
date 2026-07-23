use anchor_lang::prelude::*;

use crate::{
    constants::{PLAN_SEED, SUBSCRIPTION_SEED},
    state::{Plan, Subscription},
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
}

impl Cancel<'_> {
    pub fn run(&mut self) -> Result<()> {
        Ok(())
    }
}
