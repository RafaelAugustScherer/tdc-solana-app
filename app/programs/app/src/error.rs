use anchor_lang::prelude::*;

#[error_code]
pub enum SubscriptionError {
    #[msg("Amount per period must be greater than zero")]
    InvalidAmount,
    #[msg("Period must be greater than zero seconds")]
    InvalidPeriod,
    #[msg("Allowance must be greater than zero")]
    InvalidAllowance,
    #[msg("Plan is not accepting charges")]
    PlanInactive,
    #[msg("The billing period has not elapsed yet")]
    PeriodNotElapsed,
    #[msg("Remaining allowance does not cover this charge")]
    AllowanceExhausted,
    #[msg("Total allowance would overflow")]
    AllowanceOverflow,
    #[msg("Another program holds the delegation on this token account")]
    ForeignDelegate,
    #[msg("The delegation was withdrawn or is too small to cover this charge")]
    DelegateRevoked,
    #[msg("Billing schedule arithmetic overflowed")]
    ScheduleOverflow,
    #[msg("Merchant token account is not owned by the plan merchant")]
    WrongMerchantAccount,
    #[msg("Token account mint does not match the plan mint")]
    WrongMint,
}
