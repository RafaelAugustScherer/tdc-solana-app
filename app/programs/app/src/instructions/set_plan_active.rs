use anchor_lang::prelude::*;

use crate::{constants::PLAN_SEED, state::Plan};

#[derive(Accounts)]
pub struct SetPlanActive<'info> {
    pub merchant: Signer<'info>,

    #[account(
        mut,
        seeds = [PLAN_SEED, merchant.key().as_ref(), &plan.plan_id.to_le_bytes()],
        bump = plan.bump,
        has_one = merchant,
    )]
    pub plan: Account<'info, Plan>,
}

impl SetPlanActive<'_> {
    pub fn run(&mut self, is_active: bool) -> Result<()> {
        self.plan.is_active = is_active;

        Ok(())
    }
}
