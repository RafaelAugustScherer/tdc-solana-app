use anchor_lang::prelude::*;

use crate::error::SubscriptionError;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum PriceMode {
    Fixed,
    Variable,
}

#[account]
#[derive(InitSpace)]
pub struct SubscriberDelegation {
    pub subscriber: Pubkey,
    pub mint: Pubkey,
    pub committed_total: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Subscription {
    pub plan: Pubkey,
    pub subscriber: Pubkey,
    pub next_charge_at: i64,
    pub allowance_remaining: u64,
    pub max_amount_per_period: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Plan {
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub plan_id: u64,
    pub amount_per_period: u64,
    pub amount_effective_at: i64,
    pub previous_amount: u64,
    pub previous_effective_at: i64,
    pub period_seconds: i64,
    pub price_mode: PriceMode,
    pub is_active: bool,
    pub bump: u8,
}

impl Plan {
    pub fn applicable_amount(&self, at: i64) -> Result<u64> {
        if at >= self.amount_effective_at {
            Ok(self.amount_per_period)
        } else if at >= self.previous_effective_at {
            Ok(self.previous_amount)
        } else {
            err!(SubscriptionError::PriceHistoryUnavailable)
        }
    }
}
