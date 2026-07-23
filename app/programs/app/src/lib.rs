pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("HfGNUm2CyE8jY4oK96LSG3NCxLRcgJAHhQjVckixhopo");

#[program]
pub mod app {
    use super::*;

    pub fn create_plan(
        ctx: Context<CreatePlan>,
        plan_id: u64,
        amount_per_period: u64,
        period_seconds: i64,
        price_mode: PriceMode,
    ) -> Result<()> {
        ctx.accounts.run(
            plan_id,
            amount_per_period,
            period_seconds,
            price_mode,
            ctx.bumps.plan,
        )
    }

    pub fn set_plan_active(ctx: Context<SetPlanActive>, is_active: bool) -> Result<()> {
        ctx.accounts.run(is_active)
    }
}
