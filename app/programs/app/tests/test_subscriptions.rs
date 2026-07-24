mod common;

use app::PriceMode;
use common::{
    assert_error, ata, cancel_ix, charge_ix, create_plan_ix, delegate_pda, plan_pda,
    set_plan_active_ix, subscribe_ix, subscription_pda, Env,
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
        Self::with_amount(AMOUNT)
    }

    fn with_amount(amount: u64) -> Self {
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
                amount,
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
    ) -> Result<(), Box<litesvm::types::FailedTransactionMetadata>> {
        self.subscribe_capped(allowance, u64::MAX)
    }

    fn subscribe_capped(
        &mut self,
        allowance: u64,
        max_amount_per_period: u64,
    ) -> Result<(), Box<litesvm::types::FailedTransactionMetadata>> {
        self.env.send(
            &[subscribe_ix(
                &self.subscriber.pubkey(),
                &self.plan,
                &self.subscription,
                &self.subscriber_ata,
                &self.mint,
                allowance,
                max_amount_per_period,
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
}

// Happy path

#[test]
fn subscribe_records_the_subscription_and_grants_the_delegation() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let subscription = fixture.env.subscription(fixture.subscription);
    assert_eq!(subscription.plan, fixture.plan);
    assert_eq!(subscription.subscriber, fixture.subscriber.pubkey());
    assert_eq!(subscription.allowance_remaining, ALLOWANCE);

    let token_account = fixture.env.token_account(fixture.subscriber_ata);
    assert_eq!(token_account.delegate.unwrap(), delegate_pda().0);
    assert_eq!(token_account.delegated_amount, ALLOWANCE);
}

#[test]
fn the_first_charge_is_due_immediately_and_moves_exactly_the_plan_amount() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let before = fixture.env.balance(fixture.subscriber_ata);
    fixture.charge().unwrap();

    assert_eq!(fixture.env.balance(fixture.merchant_ata), AMOUNT);
    assert_eq!(fixture.env.balance(fixture.subscriber_ata), before - AMOUNT);
    assert_eq!(
        fixture
            .env
            .subscription(fixture.subscription)
            .allowance_remaining,
        ALLOWANCE - AMOUNT
    );
}

#[test]
fn the_schedule_advances_from_the_due_date_not_the_charge_time() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();
    fixture.charge().unwrap();

    let after_first = fixture
        .env
        .subscription(fixture.subscription)
        .next_charge_at;

    fixture.env.advance_clock(PERIOD + 60);
    fixture.charge().unwrap();

    assert_eq!(
        fixture
            .env
            .subscription(fixture.subscription)
            .next_charge_at,
        after_first + PERIOD
    );
}

#[test]
fn cancel_closes_the_subscription_and_returns_rent() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let rent = fixture
        .env
        .svm
        .get_account(&fixture.subscription)
        .unwrap()
        .lamports;
    let before = fixture
        .env
        .svm
        .get_account(&fixture.subscriber.pubkey())
        .unwrap()
        .lamports;

    fixture.cancel().unwrap();

    assert!(!fixture.env.subscription_exists(fixture.subscription));
    let after = fixture
        .env
        .svm
        .get_account(&fixture.subscriber.pubkey())
        .unwrap()
        .lamports;
    assert!(after > before, "rent {rent} should have come back");
}

#[test]
fn a_cancelled_subscriber_can_subscribe_to_the_same_plan_again() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();
    fixture.cancel().unwrap();

    fixture.subscribe(ALLOWANCE).unwrap();

    assert!(fixture.env.subscription_exists(fixture.subscription));
}

// Schedule

#[test]
fn charging_before_the_period_elapses_fails_and_moves_nothing() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();
    fixture.charge().unwrap();

    let before = fixture.env.balance(fixture.merchant_ata);
    let result = fixture.charge();

    assert_error(result, "PeriodNotElapsed");
    assert_eq!(fixture.env.balance(fixture.merchant_ata), before);
}

#[test]
fn a_backlog_collapses_to_exactly_one_charge() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();
    fixture.charge().unwrap();

    let before = fixture.env.balance(fixture.merchant_ata);
    fixture.env.advance_clock(PERIOD * 3);
    fixture.charge().unwrap();

    assert_eq!(fixture.env.balance(fixture.merchant_ata), before + AMOUNT);

    let subscription = fixture.env.subscription(fixture.subscription);
    let now = fixture
        .env
        .svm
        .get_sysvar::<solana_clock::Clock>()
        .unix_timestamp;
    assert!(subscription.next_charge_at > now);
    assert!(subscription.next_charge_at <= now + PERIOD);
}

#[test]
fn two_charges_in_one_transaction_take_only_one_payment() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();
    fixture.charge().unwrap();
    fixture.env.advance_clock(PERIOD * 3);

    let cranker = fixture.env.funded_keypair();
    let instruction = charge_ix(
        &fixture.plan,
        &fixture.subscription,
        &fixture.subscriber.pubkey(),
        &fixture.subscriber_ata,
        &fixture.merchant_ata,
        &fixture.mint,
    );
    let result = fixture
        .env
        .send(&[instruction.clone(), instruction], &[&cranker]);

    assert_error(result, "PeriodNotElapsed");
    assert_eq!(fixture.env.balance(fixture.merchant_ata), AMOUNT);
}

#[test]
fn a_long_backlog_does_not_drift_the_billing_anniversary() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();
    fixture.charge().unwrap();

    let anchor_point = fixture
        .env
        .subscription(fixture.subscription)
        .next_charge_at;

    fixture.env.advance_clock(PERIOD * 10 + 12345);
    fixture.charge().unwrap();

    let next = fixture
        .env
        .subscription(fixture.subscription)
        .next_charge_at;
    assert_eq!((next - anchor_point) % PERIOD, 0);
}

// Delegation

#[test]
fn a_second_subscription_adds_to_the_delegation_and_both_charge() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let other_merchant = fixture.env.funded_keypair();
    let other_merchant_ata = fixture.env.create_ata(&other_merchant, fixture.mint);
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
                ALLOWANCE,
                u64::MAX,
            )],
            &[&fixture.subscriber],
        )
        .unwrap();

    assert_eq!(
        fixture
            .env
            .token_account(fixture.subscriber_ata)
            .delegated_amount,
        ALLOWANCE * 2
    );

    fixture.charge().unwrap();
    let cranker = fixture.env.funded_keypair();
    fixture
        .env
        .send(
            &[charge_ix(
                &other_plan,
                &other_subscription,
                &fixture.subscriber.pubkey(),
                &fixture.subscriber_ata,
                &other_merchant_ata,
                &fixture.mint,
            )],
            &[&cranker],
        )
        .unwrap();

    assert_eq!(fixture.env.balance(fixture.merchant_ata), AMOUNT);
    assert_eq!(fixture.env.balance(other_merchant_ata), AMOUNT);
}

#[test]
fn a_delegation_that_would_overflow_is_rejected() {
    let mut fixture = Fixture::new();
    fixture.subscribe(u64::MAX).unwrap();

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
            u64::MAX,
        )],
        &[&fixture.subscriber],
    );

    assert_error(result, "AllowanceOverflow");
}

#[test]
fn subscribe_refuses_to_overwrite_a_foreign_delegation() {
    let mut fixture = Fixture::new();
    let stranger = Pubkey::new_unique();

    fixture
        .env
        .approve(&fixture.subscriber, fixture.subscriber_ata, stranger, 500);

    let result = fixture.subscribe(ALLOWANCE);

    assert_error(result, "ForeignDelegate");
    assert_eq!(
        fixture
            .env
            .token_account(fixture.subscriber_ata)
            .delegate
            .unwrap(),
        stranger
    );
}

#[test]
fn a_revoked_delegation_stops_charging() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();
    fixture
        .env
        .revoke(&fixture.subscriber, fixture.subscriber_ata);

    let result = fixture.charge();

    assert_error(result, "DelegateRevoked");
}

#[test]
fn cancel_then_subscribe_restores_charging_after_a_revoke() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();
    fixture
        .env
        .revoke(&fixture.subscriber, fixture.subscriber_ata);

    fixture.cancel().unwrap();
    fixture.subscribe(ALLOWANCE).unwrap();

    fixture.charge().unwrap();
    assert_eq!(fixture.env.balance(fixture.merchant_ata), AMOUNT);
}

#[test]
fn the_pool_is_drawn_first_come_first_served() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let other_merchant = fixture.env.funded_keypair();
    let other_merchant_ata = fixture.env.create_ata(&other_merchant, fixture.mint);
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
                ALLOWANCE,
                u64::MAX,
            )],
            &[&fixture.subscriber],
        )
        .unwrap();

    let delegate = delegate_pda().0;
    fixture.env.approve(
        &fixture.subscriber,
        fixture.subscriber_ata,
        delegate,
        AMOUNT,
    );

    fixture.charge().unwrap();

    let cranker = fixture.env.funded_keypair();
    let result = fixture.env.send(
        &[charge_ix(
            &other_plan,
            &other_subscription,
            &fixture.subscriber.pubkey(),
            &fixture.subscriber_ata,
            &other_merchant_ata,
            &fixture.mint,
        )],
        &[&cranker],
    );

    assert_error(result, "DelegateRevoked");
}

#[test]
fn an_exactly_exhausted_delegation_clears_the_delegate() {
    let mut fixture = Fixture::new();
    fixture.subscribe(AMOUNT).unwrap();
    fixture.charge().unwrap();

    let token_account = fixture.env.token_account(fixture.subscriber_ata);
    assert_eq!(token_account.delegated_amount, 0);
    assert!(token_account.delegate.is_none());
    assert_eq!(
        fixture
            .env
            .subscription(fixture.subscription)
            .allowance_remaining,
        0
    );
}

// Account substitution

#[test]
fn a_charge_cannot_debit_another_wallets_token_account() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let victim = fixture.env.funded_keypair();
    let victim_ata = fixture.env.create_ata(&victim, fixture.mint);
    fixture
        .env
        .mint_to(&fixture.merchant, fixture.mint, victim_ata, FUNDING);
    let delegate = delegate_pda().0;
    fixture
        .env
        .approve(&victim, victim_ata, delegate, ALLOWANCE);

    let cranker = fixture.env.funded_keypair();
    let result = fixture.env.send(
        &[charge_ix(
            &fixture.plan,
            &fixture.subscription,
            &fixture.subscriber.pubkey(),
            &victim_ata,
            &fixture.merchant_ata,
            &fixture.mint,
        )],
        &[&cranker],
    );

    assert!(result.is_err());
    assert_eq!(fixture.env.balance(victim_ata), FUNDING);
}

#[test]
fn a_charge_cannot_debit_the_subscribers_other_token_account() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let auxiliary = fixture
        .env
        .create_auxiliary_token_account(&fixture.subscriber, fixture.mint);
    fixture
        .env
        .mint_to(&fixture.merchant, fixture.mint, auxiliary, FUNDING);
    let delegate = delegate_pda().0;
    fixture
        .env
        .approve(&fixture.subscriber, auxiliary, delegate, ALLOWANCE);

    let cranker = fixture.env.funded_keypair();
    let result = fixture.env.send(
        &[charge_ix(
            &fixture.plan,
            &fixture.subscription,
            &fixture.subscriber.pubkey(),
            &auxiliary,
            &fixture.merchant_ata,
            &fixture.mint,
        )],
        &[&cranker],
    );

    assert!(result.is_err());
    assert_eq!(fixture.env.balance(auxiliary), FUNDING);
}

#[test]
fn each_merchant_is_paid_only_from_the_subscribers_associated_account() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let before = fixture.env.balance(fixture.subscriber_ata);
    fixture.charge().unwrap();

    assert_eq!(fixture.env.balance(fixture.subscriber_ata), before - AMOUNT);
    assert_eq!(fixture.env.balance(fixture.merchant_ata), AMOUNT);
    assert_eq!(
        fixture
            .env
            .token_account(fixture.subscriber_ata)
            .delegated_amount,
        ALLOWANCE - AMOUNT
    );
}

#[test]
fn a_charge_cannot_be_redirected_to_another_merchant() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let thief = fixture.env.funded_keypair();
    let thief_ata = fixture.env.create_ata(&thief, fixture.mint);

    let cranker = fixture.env.funded_keypair();
    let result = fixture.env.send(
        &[charge_ix(
            &fixture.plan,
            &fixture.subscription,
            &fixture.subscriber.pubkey(),
            &fixture.subscriber_ata,
            &thief_ata,
            &fixture.mint,
        )],
        &[&cranker],
    );

    assert_error(result, "WrongMerchantAccount");
    assert_eq!(fixture.env.balance(thief_ata), 0);
}

#[test]
fn a_charge_with_the_wrong_mint_account_is_rejected() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let other_mint = fixture.env.create_mint(&fixture.merchant, 6);
    let cranker = fixture.env.funded_keypair();
    let result = fixture.env.send(
        &[charge_ix(
            &fixture.plan,
            &fixture.subscription,
            &fixture.subscriber.pubkey(),
            &fixture.subscriber_ata,
            &fixture.merchant_ata,
            &other_mint,
        )],
        &[&cranker],
    );

    assert!(result.is_err());
    assert_eq!(fixture.env.balance(fixture.merchant_ata), 0);
}

#[test]
fn a_merchant_account_on_another_mint_is_rejected() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let other_mint = fixture.env.create_mint(&fixture.merchant, 6);
    let other_ata = fixture.env.create_ata(&fixture.merchant, other_mint);

    let cranker = fixture.env.funded_keypair();
    let result = fixture.env.send(
        &[charge_ix(
            &fixture.plan,
            &fixture.subscription,
            &fixture.subscriber.pubkey(),
            &fixture.subscriber_ata,
            &other_ata,
            &fixture.mint,
        )],
        &[&cranker],
    );

    assert_error(result, "WrongMint");
}

#[test]
fn a_subscription_from_another_plan_is_rejected() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let (other_plan, _) = plan_pda(&fixture.merchant.pubkey(), 2);
    fixture
        .env
        .send(
            &[create_plan_ix(
                &fixture.merchant.pubkey(),
                &other_plan,
                &fixture.mint,
                2,
                AMOUNT,
                PERIOD,
                PriceMode::Fixed,
            )],
            &[&fixture.merchant],
        )
        .unwrap();

    let cranker = fixture.env.funded_keypair();
    let result = fixture.env.send(
        &[charge_ix(
            &other_plan,
            &fixture.subscription,
            &fixture.subscriber.pubkey(),
            &fixture.subscriber_ata,
            &fixture.merchant_ata,
            &fixture.mint,
        )],
        &[&cranker],
    );

    assert!(result.is_err());
    assert_eq!(fixture.env.balance(fixture.merchant_ata), 0);
}

// Guards and authorisation

#[test]
fn subscribe_rejects_a_zero_allowance() {
    let mut fixture = Fixture::new();
    let result = fixture.subscribe(0);

    assert_error(result, "InvalidAllowance");
}

#[test]
fn subscribe_rejects_an_inactive_plan() {
    let mut fixture = Fixture::new();
    fixture
        .env
        .send(
            &[set_plan_active_ix(
                &fixture.merchant.pubkey(),
                &fixture.plan,
                false,
            )],
            &[&fixture.merchant],
        )
        .unwrap();

    let result = fixture.subscribe(ALLOWANCE);

    assert_error(result, "PlanInactive");
    assert!(fixture
        .env
        .token_account(fixture.subscriber_ata)
        .delegate
        .is_none());
}

#[test]
fn retiring_a_plan_stops_charging_existing_subscriptions() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();
    fixture
        .env
        .send(
            &[set_plan_active_ix(
                &fixture.merchant.pubkey(),
                &fixture.plan,
                false,
            )],
            &[&fixture.merchant],
        )
        .unwrap();

    let result = fixture.charge();

    assert_error(result, "PlanInactive");
}

#[test]
fn a_charge_beyond_the_remaining_allowance_is_rejected() {
    let mut fixture = Fixture::new();
    fixture.subscribe(AMOUNT).unwrap();
    fixture.charge().unwrap();

    fixture.env.advance_clock(PERIOD);
    let result = fixture.charge();

    assert_error(result, "AllowanceExhausted");
}

#[test]
fn a_charge_beyond_the_token_balance_fails_cleanly() {
    let mut fixture = Fixture::with_amount(FUNDING * 2);
    fixture.subscribe(FUNDING * 4).unwrap();

    let result = fixture.charge();

    assert!(result.is_err());
    assert_eq!(fixture.env.balance(fixture.merchant_ata), 0);
}

#[test]
fn only_the_subscriber_can_cancel() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let stranger = fixture.env.funded_keypair();
    let result = fixture.env.send(
        &[cancel_ix(
            &stranger.pubkey(),
            &fixture.plan,
            &fixture.subscription,
            &fixture.subscriber_ata,
            &fixture.mint,
        )],
        &[&stranger],
    );

    assert!(result.is_err());
    assert!(fixture.env.subscription_exists(fixture.subscription));
}

#[test]
fn subscribing_twice_to_the_same_plan_fails() {
    let mut fixture = Fixture::new();
    fixture.subscribe(ALLOWANCE).unwrap();

    let result = fixture.subscribe(ALLOWANCE);

    assert!(result.is_err());
}

#[test]
fn a_subscriber_without_an_associated_token_account_cannot_subscribe() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
    let subscriber = env.funded_keypair();
    let mint = env.create_mint(&merchant, 6);

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
    let result = env.send(
        &[subscribe_ix(
            &subscriber.pubkey(),
            &plan,
            &subscription,
            &ata(&subscriber.pubkey(), &mint),
            &mint,
            ALLOWANCE,
            u64::MAX,
        )],
        &[&subscriber],
    );

    assert!(result.is_err());
}
