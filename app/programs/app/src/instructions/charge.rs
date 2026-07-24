use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};
use solana_program_option::COption;

use crate::{
    constants::{DELEGATE_SEED, PLAN_SEED, SUBSCRIPTION_SEED},
    error::SubscriptionError,
    state::{Plan, Subscription},
};

#[derive(Accounts)]
pub struct Charge<'info> {
    #[account(
        seeds = [PLAN_SEED, plan.merchant.as_ref(), &plan.plan_id.to_le_bytes()],
        bump = plan.bump,
    )]
    pub plan: Account<'info, Plan>,

    #[account(
        mut,
        seeds = [SUBSCRIPTION_SEED, plan.key().as_ref(), subscription.subscriber.as_ref()],
        bump = subscription.bump,
        has_one = plan,
    )]
    pub subscription: Account<'info, Subscription>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = subscription.subscriber,
    )]
    pub subscriber_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = merchant_token_account.owner == plan.merchant
            @ SubscriptionError::WrongMerchantAccount,
        constraint = merchant_token_account.mint == plan.mint
            @ SubscriptionError::WrongMint,
    )]
    pub merchant_token_account: Account<'info, TokenAccount>,

    #[account(address = plan.mint)]
    pub mint: Account<'info, Mint>,

    /// CHECK: seeds-derived delegate authority; signs the transfer, holds no data
    #[account(seeds = [DELEGATE_SEED], bump)]
    pub delegate_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

impl Charge<'_> {
    pub fn run(&mut self, delegate_bump: u8) -> Result<()> {
        require!(self.plan.is_active, SubscriptionError::PlanInactive);

        let now = Clock::get()?.unix_timestamp;
        require!(
            now >= self.subscription.next_charge_at,
            SubscriptionError::PeriodNotElapsed
        );

        let amount = self.plan.amount_per_period;
        require!(
            self.subscription.allowance_remaining >= amount,
            SubscriptionError::AllowanceExhausted
        );

        let delegated = match self.subscriber_token_account.delegate {
            COption::Some(current) if current == self.delegate_authority.key() => {
                self.subscriber_token_account.delegated_amount
            }
            _ => 0,
        };
        require!(delegated >= amount, SubscriptionError::DelegateRevoked);

        let seeds: &[&[u8]] = &[DELEGATE_SEED, &[delegate_bump]];
        token::transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.key(),
                TransferChecked {
                    from: self.subscriber_token_account.to_account_info(),
                    mint: self.mint.to_account_info(),
                    to: self.merchant_token_account.to_account_info(),
                    authority: self.delegate_authority.to_account_info(),
                },
                &[seeds],
            ),
            amount,
            self.mint.decimals,
        )?;

        self.subscription.allowance_remaining = self
            .subscription
            .allowance_remaining
            .checked_sub(amount)
            .ok_or(SubscriptionError::AllowanceExhausted)?;

        self.advance_schedule(now)
    }

    fn advance_schedule(&mut self, now: i64) -> Result<()> {
        let elapsed = now
            .checked_sub(self.subscription.next_charge_at)
            .ok_or(SubscriptionError::ScheduleOverflow)?;
        let periods_to_advance = elapsed
            .checked_div(self.plan.period_seconds)
            .ok_or(SubscriptionError::ScheduleOverflow)?
            .checked_add(1)
            .ok_or(SubscriptionError::ScheduleOverflow)?;

        self.subscription.next_charge_at = self
            .subscription
            .next_charge_at
            .checked_add(
                periods_to_advance
                    .checked_mul(self.plan.period_seconds)
                    .ok_or(SubscriptionError::ScheduleOverflow)?,
            )
            .ok_or(SubscriptionError::ScheduleOverflow)?;

        Ok(())
    }
}
