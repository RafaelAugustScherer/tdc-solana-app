pub mod constants;
pub mod delegation;
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

    pub fn update_price(ctx: Context<UpdatePrice>, new_amount: u64) -> Result<()> {
        ctx.accounts.run(new_amount)
    }

    pub fn subscribe(
        ctx: Context<Subscribe>,
        allowance: u64,
        max_amount_per_period: u64,
    ) -> Result<()> {
        ctx.accounts.run(
            allowance,
            max_amount_per_period,
            ctx.bumps.subscription,
            ctx.bumps.subscriber_delegation,
        )
    }

    pub fn set_max_amount(ctx: Context<SetMaxAmount>, new_max: u64) -> Result<()> {
        ctx.accounts.run(new_max)
    }

    pub fn set_allowance(ctx: Context<SetAllowance>, new_allowance: u64) -> Result<()> {
        ctx.accounts.run(new_allowance)
    }

    pub fn reauthorize(ctx: Context<Reauthorize>) -> Result<()> {
        ctx.accounts.run()
    }

    pub fn charge(ctx: Context<Charge>) -> Result<()> {
        ctx.accounts.run(ctx.bumps.delegate_authority)
    }

    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        ctx.accounts.run()
    }
}
