use anchor_lang::prelude::*;
use anchor_spl::token::{self, Approve, Token, TokenAccount};
use solana_program_option::COption;

use crate::error::SubscriptionError;

pub fn is_held_by<'info>(
    token_account: &Account<'info, TokenAccount>,
    delegate: &UncheckedAccount<'info>,
) -> bool {
    matches!(token_account.delegate, COption::Some(current) if current == delegate.key())
}

pub fn require_not_foreign<'info>(
    token_account: &Account<'info, TokenAccount>,
    delegate: &UncheckedAccount<'info>,
) -> Result<()> {
    match token_account.delegate {
        COption::Some(current) if current != delegate.key() => {
            err!(SubscriptionError::ForeignDelegate)
        }
        _ => Ok(()),
    }
}

pub fn approve<'info>(
    token_program: &Program<'info, Token>,
    token_account: &Account<'info, TokenAccount>,
    delegate: &UncheckedAccount<'info>,
    owner: &Signer<'info>,
    amount: u64,
) -> Result<()> {
    token::approve(
        CpiContext::new(
            token_program.key(),
            Approve {
                to: token_account.to_account_info(),
                delegate: delegate.to_account_info(),
                authority: owner.to_account_info(),
            },
        ),
        amount,
    )
}
