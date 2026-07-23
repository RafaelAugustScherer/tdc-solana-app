use anchor_lang::prelude::*;

#[error_code]
pub enum SubscriptionError {
    #[msg("Amount per period must be greater than zero")]
    InvalidAmount,
    #[msg("Period must be greater than zero seconds")]
    InvalidPeriod,
}
