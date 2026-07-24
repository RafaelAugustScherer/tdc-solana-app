use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum PriceMode {
    Fixed,
    Variable,
}

#[account]
#[derive(InitSpace)]
pub struct Plan {
    pub merchant: Pubkey,
    pub mint: Pubkey,
    pub plan_id: u64,
    pub amount_per_period: u64,
    pub period_seconds: i64,
    pub price_mode: PriceMode,
    pub is_active: bool,
    pub bump: u8,
}
