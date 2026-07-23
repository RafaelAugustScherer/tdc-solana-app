use anchor_lang::prelude::*;
use anchor_spl::token::{self, Approve, Mint, Token, TokenAccount};
use solana_program_option::COption;

use crate::{
    constants::{DELEGATE_SEED, PLAN_SEED, SUBSCRIPTION_SEED},
    error::SubscriptionError,
    state::{Plan, Subscription},
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
    pub fn run(&mut self, allowance: u64, bump: u8) -> Result<()> {
        require!(allowance > 0, SubscriptionError::InvalidAllowance);
        require!(self.plan.is_active, SubscriptionError::PlanInactive);

        let existing = match self.subscriber_token_account.delegate {
            COption::Some(current) if current == self.delegate_authority.key() => {
                self.subscriber_token_account.delegated_amount
            }
            COption::Some(_) => return err!(SubscriptionError::ForeignDelegate),
            COption::None => 0,
        };
        let total = existing
            .checked_add(allowance)
            .ok_or(SubscriptionError::AllowanceOverflow)?;

        token::approve(
            CpiContext::new(
                self.token_program.key(),
                Approve {
                    to: self.subscriber_token_account.to_account_info(),
                    delegate: self.delegate_authority.to_account_info(),
                    authority: self.subscriber.to_account_info(),
                },
            ),
            total,
        )?;

        self.subscription.set_inner(Subscription {
            plan: self.plan.key(),
            subscriber: self.subscriber.key(),
            next_charge_at: Clock::get()?.unix_timestamp,
            allowance_remaining: allowance,
            bump,
        });

        Ok(())
    }
}
