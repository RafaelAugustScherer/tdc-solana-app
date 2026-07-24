mod common;

use app::PriceMode;
use common::{
    assert_error, cancel_ix, charge_ix, create_plan_ix, delegate_pda, delegation_pda, plan_pda,
    reauthorize_ix, set_allowance_ix, set_max_amount_ix, subscribe_ix, subscription_pda, Env,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const AMOUNT: u64 = 10_000_000;
const PERIOD: i64 = 30 * 24 * 60 * 60;
const ALLOWANCE: u64 = AMOUNT * 12;
const FUNDING: u64 = AMOUNT * 100;

struct Fixture {
    env: Env,
    merchant: Keypair,
    subscriber: Keypair,
    mint: Pubkey,
    plan: Pubkey,
    subscription: Pubkey,
    subscriber_ata: Pubkey,
    merchant_ata: Pubkey,
}

impl Fixture {
    fn new() -> Self {
        let mut env = Env::new();
        let merchant = env.funded_keypair();
        let subscriber = env.funded_keypair();
        let mint = env.create_mint(&merchant, 6);

        let subscriber_ata = env.create_ata(&subscriber, mint);
        let merchant_ata = env.create_ata(&merchant, mint);
        env.mint_to(&merchant, mint, subscriber_ata, FUNDING);

        let (plan, _) = plan_pda(&merchant.pubkey(), 1);
        env.send(
            &[create_plan_ix(
                &merchant.pubkey(),
                &plan,
                &mint,
                1,
                AMOUNT,
                PERIOD,
                PriceMode::Fixed,
            )],
            &[&merchant],
        )
        .unwrap();

        let (subscription, _) = subscription_pda(&plan, &subscriber.pubkey());

        Self {
            env,
            merchant,
            subscriber,
            mint,
            plan,
            subscription,
            subscriber_ata,
            merchant_ata,
        }
    }

    fn subscribe(
        &mut self,
        allowance: u64,
        max_amount: u64,
    ) -> Result<(), Box<litesvm::types::FailedTransactionMetadata>> {
        self.env.send(
            &[subscribe_ix(
                &self.subscriber.pubkey(),
                &self.plan,
                &self.subscription,
                &self.subscriber_ata,
                &self.mint,
                allowance,
                max_amount,
            )],
            &[&self.subscriber],
        )
    }

    fn charge(&mut self) -> Result<(), Box<litesvm::types::FailedTransactionMetadata>> {
        let cranker = self.env.funded_keypair();
        self.env.send(
            &[charge_ix(
                &self.plan,
                &self.subscription,
                &self.subscriber.pubkey(),
                &self.subscriber_ata,
                &self.merchant_ata,
                &self.mint,
            )],
            &[&cranker],
        )
    }

    fn set_max_amount(
        &mut self,
        new_max: u64,
    ) -> Result<(), Box<litesvm::types::FailedTransactionMetadata>> {
        self.env.send(
            &[set_max_amount_ix(
                &self.subscriber.pubkey(),
                &self.plan,
                &self.subscription,
                new_max,
            )],
            &[&self.subscriber],
        )
    }

    fn set_allowance(
        &mut self,
        new_allowance: u64,
    ) -> Result<(), Box<litesvm::types::FailedTransactionMetadata>> {
        self.env.send(
            &[set_allowance_ix(
                &self.subscriber.pubkey(),
                &self.plan,
                &self.subscription,
                &self.subscriber_ata,
                &self.mint,
                new_allowance,
            )],
            &[&self.subscriber],
        )
    }

    fn reauthorize(&mut self) -> Result<(), Box<litesvm::types::FailedTransactionMetadata>> {
        self.env.send(
            &[reauthorize_ix(
                &self.subscriber.pubkey(),
                &self.subscriber_ata,
                &self.mint,
            )],
            &[&self.subscriber],
        )
    }

    fn cancel(&mut self) -> Result<(), Box<litesvm::types::FailedTransactionMetadata>> {
        self.env.send(
            &[cancel_ix(
                &self.subscriber.pubkey(),
                &self.plan,
                &self.subscription,
                &self.subscriber_ata,
                &self.mint,
            )],
            &[&self.subscriber],
        )
    }

    fn committed(&self) -> u64 {
        self.env
            .committed_total(&self.subscriber.pubkey(), &self.mint)
    }

    fn delegated(&self) -> u64 {
        self.env.delegated_amount(self.subscriber_ata)
    }
}

// The cap

#[test]
fn subscribe_records_the_cap() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT * 2).unwrap();

    assert_eq!(
        fixture
            .env
            .subscription(fixture.subscription)
            .max_amount_per_period,
        AMOUNT * 2
    );
}

#[test]
fn subscribe_rejects_a_zero_cap() {
    let mut fixture = Fixture::new();
    let result = fixture.subscribe(ALLOWANCE, 0);

    assert_error(result, "InvalidMaxAmount");
}

#[test]
fn subscribe_rejects_a_plan_priced_above_the_offered_cap() {
    let mut fixture = Fixture::new();
    let result = fixture.subscribe(ALLOWANCE, AMOUNT - 1);

    assert_error(result, "PriceAboveSubscriberMax");
    assert!(fixture
        .env
        .token_account(fixture.subscriber_ata)
        .delegate
        .is_none());
}

#[test]
fn lowering_the_cap_below_the_price_pauses_charging() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();
    fixture.charge().unwrap();
    fixture.env.advance_clock(PERIOD);

    fixture.set_max_amount(AMOUNT - 1).unwrap();
    let before = fixture.env.balance(fixture.merchant_ata);
    let due_at = fixture
        .env
        .subscription(fixture.subscription)
        .next_charge_at;

    let result = fixture.charge();

    assert_error(result, "PriceAboveSubscriberMax");
    assert_eq!(fixture.env.balance(fixture.merchant_ata), before);
    assert_eq!(
        fixture
            .env
            .subscription(fixture.subscription)
            .next_charge_at,
        due_at
    );
}

#[test]
fn raising_the_cap_resumes_charging_immediately() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();
    fixture.charge().unwrap();
    fixture.env.advance_clock(PERIOD);
    fixture.set_max_amount(AMOUNT - 1).unwrap();
    fixture.charge().unwrap_err();

    fixture.set_max_amount(AMOUNT * 2).unwrap();
    fixture.charge().unwrap();

    assert_eq!(fixture.env.balance(fixture.merchant_ata), AMOUNT * 2);
}

#[test]
fn a_long_pause_resumes_on_exactly_one_charge() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();
    fixture.charge().unwrap();
    fixture.env.advance_clock(PERIOD);
    fixture.set_max_amount(AMOUNT - 1).unwrap();

    fixture.env.advance_clock(PERIOD * 3);
    fixture.set_max_amount(AMOUNT * 2).unwrap();
    fixture.charge().unwrap();

    assert_eq!(fixture.env.balance(fixture.merchant_ata), AMOUNT * 2);

    let result = fixture.charge();
    assert_error(result, "PeriodNotElapsed");
}

#[test]
fn only_the_subscriber_can_change_the_cap() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let stranger = fixture.env.funded_keypair();
    let result = fixture.env.send(
        &[set_max_amount_ix(
            &stranger.pubkey(),
            &fixture.plan,
            &fixture.subscription,
            1,
        )],
        &[&stranger],
    );

    assert!(result.is_err());
    assert_eq!(
        fixture
            .env
            .subscription(fixture.subscription)
            .max_amount_per_period,
        AMOUNT
    );
}

#[test]
fn set_max_amount_rejects_zero() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let result = fixture.set_max_amount(0);

    assert_error(result, "InvalidMaxAmount");
}

// Allowance

#[test]
fn raising_the_allowance_raises_the_delegation_by_the_same_amount() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    fixture.set_allowance(ALLOWANCE + AMOUNT).unwrap();

    assert_eq!(
        fixture
            .env
            .subscription(fixture.subscription)
            .allowance_remaining,
        ALLOWANCE + AMOUNT
    );
    assert_eq!(fixture.committed(), ALLOWANCE + AMOUNT);
    assert_eq!(fixture.delegated(), ALLOWANCE + AMOUNT);
}

#[test]
fn lowering_the_allowance_lowers_the_delegation_by_the_same_amount() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    fixture.set_allowance(AMOUNT).unwrap();

    assert_eq!(
        fixture
            .env
            .subscription(fixture.subscription)
            .allowance_remaining,
        AMOUNT
    );
    assert_eq!(fixture.committed(), AMOUNT);
    assert_eq!(fixture.delegated(), AMOUNT);
}

#[test]
fn set_allowance_rejects_zero() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let result = fixture.set_allowance(0);

    assert_error(result, "InvalidAllowance");
}

#[test]
fn only_the_subscriber_can_change_the_allowance() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let stranger = fixture.env.funded_keypair();
    let result = fixture.env.send(
        &[set_allowance_ix(
            &stranger.pubkey(),
            &fixture.plan,
            &fixture.subscription,
            &fixture.subscriber_ata,
            &fixture.mint,
            1,
        )],
        &[&stranger],
    );

    assert!(result.is_err());
}

#[test]
fn topping_up_an_exhausted_allowance_resumes_charging() {
    let mut fixture = Fixture::new();
    fixture.subscribe(AMOUNT, AMOUNT).unwrap();
    fixture.charge().unwrap();
    fixture.env.advance_clock(PERIOD);

    assert_error(fixture.charge(), "AllowanceExhausted");

    fixture.set_allowance(ALLOWANCE).unwrap();
    fixture.charge().unwrap();

    assert_eq!(fixture.env.balance(fixture.merchant_ata), AMOUNT * 2);
}

// committed_total reconciliation

#[test]
fn committed_total_aggregates_both_subscriptions_and_matches_the_delegation() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let other_merchant = fixture.env.funded_keypair();
    let (other_plan, _) = plan_pda(&other_merchant.pubkey(), 1);
    fixture
        .env
        .send(
            &[create_plan_ix(
                &other_merchant.pubkey(),
                &other_plan,
                &fixture.mint,
                1,
                AMOUNT,
                PERIOD,
                PriceMode::Fixed,
            )],
            &[&other_merchant],
        )
        .unwrap();

    let (other_subscription, _) = subscription_pda(&other_plan, &fixture.subscriber.pubkey());
    fixture
        .env
        .send(
            &[subscribe_ix(
                &fixture.subscriber.pubkey(),
                &other_plan,
                &other_subscription,
                &fixture.subscriber_ata,
                &fixture.mint,
                AMOUNT * 5,
                AMOUNT,
            )],
            &[&fixture.subscriber],
        )
        .unwrap();

    assert_eq!(fixture.committed(), ALLOWANCE + AMOUNT * 5);
    assert_eq!(fixture.delegated(), ALLOWANCE + AMOUNT * 5);
}

#[test]
fn a_charge_lowers_committed_total_and_the_delegation_together() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    fixture.charge().unwrap();

    assert_eq!(fixture.committed(), ALLOWANCE - AMOUNT);
    assert_eq!(fixture.delegated(), ALLOWANCE - AMOUNT);
}

#[test]
fn cancelling_lowers_committed_total_and_re_approves_the_remainder() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let other_merchant = fixture.env.funded_keypair();
    let (other_plan, _) = plan_pda(&other_merchant.pubkey(), 1);
    fixture
        .env
        .send(
            &[create_plan_ix(
                &other_merchant.pubkey(),
                &other_plan,
                &fixture.mint,
                1,
                AMOUNT,
                PERIOD,
                PriceMode::Fixed,
            )],
            &[&other_merchant],
        )
        .unwrap();
    let (other_subscription, _) = subscription_pda(&other_plan, &fixture.subscriber.pubkey());
    fixture
        .env
        .send(
            &[subscribe_ix(
                &fixture.subscriber.pubkey(),
                &other_plan,
                &other_subscription,
                &fixture.subscriber_ata,
                &fixture.mint,
                AMOUNT * 5,
                AMOUNT,
            )],
            &[&fixture.subscriber],
        )
        .unwrap();

    fixture.cancel().unwrap();

    assert_eq!(fixture.committed(), AMOUNT * 5);
    assert_eq!(fixture.delegated(), AMOUNT * 5);
}

#[test]
fn reauthorize_restores_the_delegation_after_a_revoke() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();
    fixture
        .env
        .revoke(&fixture.subscriber, fixture.subscriber_ata);

    assert_error(fixture.charge(), "DelegateRevoked");

    fixture.reauthorize().unwrap();

    assert_eq!(fixture.delegated(), ALLOWANCE);
    fixture.charge().unwrap();
    assert_eq!(fixture.env.balance(fixture.merchant_ata), AMOUNT);
}

#[test]
fn reauthorize_restores_an_under_approved_delegation() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let delegate = delegate_pda().0;
    fixture
        .env
        .approve(&fixture.subscriber, fixture.subscriber_ata, delegate, 1);

    fixture.reauthorize().unwrap();

    assert_eq!(fixture.delegated(), ALLOWANCE);
}

#[test]
fn reauthorize_refuses_to_overwrite_a_foreign_delegation() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let stranger = Pubkey::new_unique();
    fixture
        .env
        .approve(&fixture.subscriber, fixture.subscriber_ata, stranger, 7);

    let result = fixture.reauthorize();

    assert_error(result, "ForeignDelegate");
    let token_account = fixture.env.token_account(fixture.subscriber_ata);
    assert_eq!(token_account.delegate.unwrap(), stranger);
    assert_eq!(token_account.delegated_amount, 7);
}

#[test]
fn reauthorize_with_nothing_committed_is_a_no_op() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();
    fixture.cancel().unwrap();

    fixture.reauthorize().unwrap();

    assert_eq!(fixture.committed(), 0);
    assert_eq!(fixture.delegated(), 0);
}

#[test]
fn a_subscription_that_would_overflow_committed_total_is_rejected() {
    let mut fixture = Fixture::new();
    fixture.subscribe(u64::MAX, AMOUNT).unwrap();

    let other_merchant = fixture.env.funded_keypair();
    let (other_plan, _) = plan_pda(&other_merchant.pubkey(), 1);
    fixture
        .env
        .send(
            &[create_plan_ix(
                &other_merchant.pubkey(),
                &other_plan,
                &fixture.mint,
                1,
                AMOUNT,
                PERIOD,
                PriceMode::Fixed,
            )],
            &[&other_merchant],
        )
        .unwrap();

    let (other_subscription, _) = subscription_pda(&other_plan, &fixture.subscriber.pubkey());
    let result = fixture.env.send(
        &[subscribe_ix(
            &fixture.subscriber.pubkey(),
            &other_plan,
            &other_subscription,
            &fixture.subscriber_ata,
            &fixture.mint,
            1,
            AMOUNT,
        )],
        &[&fixture.subscriber],
    );

    assert_error(result, "AllowanceOverflow");
    assert!(!fixture.env.subscription_exists(other_subscription));
}

// cancel is never blocked

#[test]
fn cancel_succeeds_even_when_a_foreign_program_holds_the_delegate() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let stranger = Pubkey::new_unique();
    fixture
        .env
        .approve(&fixture.subscriber, fixture.subscriber_ata, stranger, 42);

    fixture.cancel().unwrap();

    assert!(!fixture.env.subscription_exists(fixture.subscription));
    assert_eq!(fixture.committed(), 0);

    let token_account = fixture.env.token_account(fixture.subscriber_ata);
    assert_eq!(token_account.delegate.unwrap(), stranger);
    assert_eq!(token_account.delegated_amount, 42);
}

#[test]
fn reauthorize_after_such_a_cancel_restores_only_the_remaining_commitment() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let other_merchant = fixture.env.funded_keypair();
    let (other_plan, _) = plan_pda(&other_merchant.pubkey(), 1);
    fixture
        .env
        .send(
            &[create_plan_ix(
                &other_merchant.pubkey(),
                &other_plan,
                &fixture.mint,
                1,
                AMOUNT,
                PERIOD,
                PriceMode::Fixed,
            )],
            &[&other_merchant],
        )
        .unwrap();
    let (other_subscription, _) = subscription_pda(&other_plan, &fixture.subscriber.pubkey());
    fixture
        .env
        .send(
            &[subscribe_ix(
                &fixture.subscriber.pubkey(),
                &other_plan,
                &other_subscription,
                &fixture.subscriber_ata,
                &fixture.mint,
                AMOUNT * 5,
                AMOUNT,
            )],
            &[&fixture.subscriber],
        )
        .unwrap();

    let stranger = Pubkey::new_unique();
    fixture
        .env
        .approve(&fixture.subscriber, fixture.subscriber_ata, stranger, 1);
    fixture.cancel().unwrap();

    fixture
        .env
        .revoke(&fixture.subscriber, fixture.subscriber_ata);
    fixture.reauthorize().unwrap();

    assert_eq!(fixture.delegated(), AMOUNT * 5);
}

// One pool per mint

#[test]
fn each_mint_gets_its_own_delegation_account() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let other_mint = fixture.env.create_mint(&fixture.merchant, 6);
    let other_ata = fixture.env.create_ata(&fixture.subscriber, other_mint);
    fixture
        .env
        .mint_to(&fixture.merchant, other_mint, other_ata, FUNDING);

    let (other_plan, _) = plan_pda(&fixture.merchant.pubkey(), 2);
    fixture
        .env
        .send(
            &[create_plan_ix(
                &fixture.merchant.pubkey(),
                &other_plan,
                &other_mint,
                2,
                AMOUNT,
                PERIOD,
                PriceMode::Fixed,
            )],
            &[&fixture.merchant],
        )
        .unwrap();

    let (other_subscription, _) = subscription_pda(&other_plan, &fixture.subscriber.pubkey());
    fixture
        .env
        .send(
            &[subscribe_ix(
                &fixture.subscriber.pubkey(),
                &other_plan,
                &other_subscription,
                &other_ata,
                &other_mint,
                AMOUNT * 5,
                AMOUNT,
            )],
            &[&fixture.subscriber],
        )
        .unwrap();

    assert_ne!(
        delegation_pda(&fixture.subscriber.pubkey(), &fixture.mint).0,
        delegation_pda(&fixture.subscriber.pubkey(), &other_mint).0
    );
    assert_eq!(fixture.committed(), ALLOWANCE);
    assert_eq!(
        fixture
            .env
            .committed_total(&fixture.subscriber.pubkey(), &other_mint),
        AMOUNT * 5
    );
    assert_eq!(fixture.delegated(), ALLOWANCE);
    assert_eq!(fixture.env.delegated_amount(other_ata), AMOUNT * 5);
}

#[test]
fn reauthorizing_one_mint_leaves_the_other_untouched() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let other_mint = fixture.env.create_mint(&fixture.merchant, 6);
    let other_ata = fixture.env.create_ata(&fixture.subscriber, other_mint);
    fixture
        .env
        .mint_to(&fixture.merchant, other_mint, other_ata, FUNDING);

    let (other_plan, _) = plan_pda(&fixture.merchant.pubkey(), 2);
    fixture
        .env
        .send(
            &[create_plan_ix(
                &fixture.merchant.pubkey(),
                &other_plan,
                &other_mint,
                2,
                AMOUNT,
                PERIOD,
                PriceMode::Fixed,
            )],
            &[&fixture.merchant],
        )
        .unwrap();
    let (other_subscription, _) = subscription_pda(&other_plan, &fixture.subscriber.pubkey());
    fixture
        .env
        .send(
            &[subscribe_ix(
                &fixture.subscriber.pubkey(),
                &other_plan,
                &other_subscription,
                &other_ata,
                &other_mint,
                AMOUNT * 5,
                AMOUNT,
            )],
            &[&fixture.subscriber],
        )
        .unwrap();

    fixture
        .env
        .revoke(&fixture.subscriber, fixture.subscriber_ata);
    fixture.reauthorize().unwrap();

    assert_eq!(fixture.delegated(), ALLOWANCE);
    assert_eq!(fixture.env.delegated_amount(other_ata), AMOUNT * 5);
}

#[test]
fn a_charge_on_one_mint_leaves_the_other_mints_pool_alone() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE, AMOUNT).unwrap();

    let other_mint = fixture.env.create_mint(&fixture.merchant, 6);
    let other_ata = fixture.env.create_ata(&fixture.subscriber, other_mint);
    fixture
        .env
        .mint_to(&fixture.merchant, other_mint, other_ata, FUNDING);

    let (other_plan, _) = plan_pda(&fixture.merchant.pubkey(), 2);
    fixture
        .env
        .send(
            &[create_plan_ix(
                &fixture.merchant.pubkey(),
                &other_plan,
                &other_mint,
                2,
                AMOUNT,
                PERIOD,
                PriceMode::Fixed,
            )],
            &[&fixture.merchant],
        )
        .unwrap();
    let (other_subscription, _) = subscription_pda(&other_plan, &fixture.subscriber.pubkey());
    fixture
        .env
        .send(
            &[subscribe_ix(
                &fixture.subscriber.pubkey(),
                &other_plan,
                &other_subscription,
                &other_ata,
                &other_mint,
                AMOUNT * 5,
                AMOUNT,
            )],
            &[&fixture.subscriber],
        )
        .unwrap();

    fixture.charge().unwrap();

    assert_eq!(fixture.committed(), ALLOWANCE - AMOUNT);
    assert_eq!(
        fixture
            .env
            .committed_total(&fixture.subscriber.pubkey(), &other_mint),
        AMOUNT * 5
    );
    assert_eq!(fixture.env.delegated_amount(other_ata), AMOUNT * 5);
}
