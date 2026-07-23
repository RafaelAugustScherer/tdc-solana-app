use anchor_lang::prelude::*;

use crate::{
    constants::{PLAN_SEED, PRICE_CHANGE_NOTICE_SECONDS},
    error::SubscriptionError,
    state::{Plan, PriceMode},
};

#[event]
pub struct PriceUpdated {
    pub plan: Pubkey,
    pub previous_amount: u64,
    pub amount_per_period: u64,
    pub amount_effective_at: i64,
}

#[derive(Accounts)]
pub struct UpdatePrice<'info> {
    pub merchant: Signer<'info>,

    #[account(
        mut,
        seeds = [PLAN_SEED, merchant.key().as_ref(), &plan.plan_id.to_le_bytes()],
        bump = plan.bump,
        has_one = merchant,
    )]
    pub plan: Account<'info, Plan>,
}

impl UpdatePrice<'_> {
    pub fn run(&mut self, new_amount: u64) -> Result<()> {
        require!(
            self.plan.price_mode == PriceMode::Variable,
            SubscriptionError::PlanPriceFixed
        );
        require!(new_amount > 0, SubscriptionError::InvalidAmount);

        let now = Clock::get()?.unix_timestamp;

        if now >= self.plan.amount_effective_at {
            self.plan.previous_amount = self.plan.amount_per_period;
            self.plan.previous_effective_at = self.plan.amount_effective_at;
        }

        self.plan.amount_per_period = new_amount;
        self.plan.amount_effective_at = now
            .checked_add(PRICE_CHANGE_NOTICE_SECONDS)
            .ok_or(SubscriptionError::ScheduleOverflow)?;

        emit!(PriceUpdated {
            plan: self.plan.key(),
            previous_amount: self.plan.previous_amount,
            amount_per_period: self.plan.amount_per_period,
            amount_effective_at: self.plan.amount_effective_at,
        });

        Ok(())
    }
}
