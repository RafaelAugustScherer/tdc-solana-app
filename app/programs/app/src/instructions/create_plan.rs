use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

use crate::{
    constants::PLAN_SEED,
    error::SubscriptionError,
    state::{Plan, PriceMode},
};

#[derive(Accounts)]
#[instruction(plan_id: u64)]
pub struct CreatePlan<'info> {
    #[account(mut)]
    pub merchant: Signer<'info>,

    #[account(
        init,
        payer = merchant,
        space = 8 + Plan::INIT_SPACE,
        seeds = [PLAN_SEED, merchant.key().as_ref(), &plan_id.to_le_bytes()],
        bump,
    )]
    pub plan: Account<'info, Plan>,

    pub mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl CreatePlan<'_> {
    pub fn run(
        &mut self,
        plan_id: u64,
        amount_per_period: u64,
        period_seconds: i64,
        price_mode: PriceMode,
        bump: u8,
    ) -> Result<()> {
        require!(amount_per_period > 0, SubscriptionError::InvalidAmount);
        require!(period_seconds > 0, SubscriptionError::InvalidPeriod);

        self.plan.set_inner(Plan {
            merchant: self.merchant.key(),
            mint: self.mint.key(),
            plan_id,
            amount_per_period,
            period_seconds,
            price_mode,
            is_active: true,
            bump,
        });

        Ok(())
    }
}
