mod common;

use app::PriceMode;
use common::{
    assert_error, charge_ix, create_plan_ix, plan_pda, set_max_amount_ix, subscribe_ix,
    subscription_pda, update_price_ix, Env,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const AMOUNT: u64 = 100;
const DAY: i64 = 24 * 60 * 60;
const PERIOD: i64 = 30 * DAY;
const ALLOWANCE: u64 = AMOUNT * 1_000;
const FUNDING: u64 = AMOUNT * 10_000;
const CAP: u64 = AMOUNT * 100;

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
    fn new(price_mode: PriceMode) -> Self {
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
                price_mode,
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

    fn variable() -> Self {
        Self::new(PriceMode::Variable)
    }

    fn subscribe(
        &mut self,
        cap: u64,
    ) -> Result<(), Box<litesvm::types::FailedTransactionMetadata>> {
        self.env.send(
            &[subscribe_ix(
                &self.subscriber.pubkey(),
                &self.plan,
                &self.subscription,
                &self.subscriber_ata,
                &self.mint,
                ALLOWANCE,
                cap,
            )],
            &[&self.subscriber],
        )
    }

    fn update_price(
        &mut self,
        new_amount: u64,
    ) -> Result<(), Box<litesvm::types::FailedTransactionMetadata>> {
        self.env.send(
            &[update_price_ix(
                &self.merchant.pubkey(),
                &self.plan,
                new_amount,
            )],
            &[&self.merchant],
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

    fn merchant_balance(&self) -> u64 {
        self.env.balance(self.merchant_ata)
    }
}

// Mode enforcement

#[test]
fn a_fixed_plan_cannot_change_its_price() {
    let mut fixture = Fixture::new(PriceMode::Fixed);

    let result = fixture.update_price(AMOUNT * 2);

    assert_error(result, "PlanPriceFixed");
    assert_eq!(fixture.env.plan(fixture.plan).amount_per_period, AMOUNT);
}

#[test]
fn only_the_merchant_can_change_the_price() {
    let mut fixture = Fixture::variable();
    let stranger = fixture.env.funded_keypair();

    let result = fixture.env.send(
        &[update_price_ix(
            &stranger.pubkey(),
            &fixture.plan,
            AMOUNT * 2,
        )],
        &[&stranger],
    );

    assert!(result.is_err());
    assert_eq!(fixture.env.plan(fixture.plan).amount_per_period, AMOUNT);
}

#[test]
fn update_price_rejects_zero() {
    let mut fixture = Fixture::variable();

    let result = fixture.update_price(0);

    assert_error(result, "InvalidAmount");
}

// The notice window

#[test]
fn an_untouched_plan_reads_as_a_constant_price() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();

    fixture.charge().unwrap();
    assert_eq!(fixture.merchant_balance(), AMOUNT);

    fixture.env.advance_clock(PERIOD * 5);
    fixture.charge().unwrap();
    assert_eq!(fixture.merchant_balance(), AMOUNT * 2);
}

#[test]
fn a_charge_inside_the_notice_window_is_billed_the_old_price() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();

    fixture.env.advance_clock(PERIOD - DAY / 2);
    fixture.update_price(AMOUNT * 3).unwrap();
    fixture.env.advance_clock(DAY / 2);
    fixture.charge().unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT * 2);
}

#[test]
fn a_charge_after_the_notice_window_is_billed_the_new_price() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();

    fixture.update_price(AMOUNT * 3).unwrap();
    fixture.env.advance_clock(PERIOD * 2);
    fixture.charge().unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT + AMOUNT * 3);
}

#[test]
fn a_superseded_announcement_never_applies_to_anyone() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();

    fixture.update_price(AMOUNT * 5).unwrap();
    fixture.env.advance_clock(DAY / 2);
    fixture.update_price(AMOUNT * 2).unwrap();

    let plan = fixture.env.plan(fixture.plan);
    assert_eq!(plan.previous_amount, AMOUNT);
    assert_eq!(plan.amount_per_period, AMOUNT * 2);

    fixture.env.advance_clock(PERIOD * 2);
    fixture.charge().unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT + AMOUNT * 2);
}

#[test]
fn each_charge_is_billed_the_amount_in_force_at_its_own_due_date() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();

    fixture.update_price(AMOUNT * 2).unwrap();
    fixture.env.advance_clock(PERIOD);
    fixture.charge().unwrap();
    assert_eq!(fixture.merchant_balance(), AMOUNT + AMOUNT * 2);

    fixture.env.advance_clock(PERIOD);
    fixture.charge().unwrap();
    assert_eq!(fixture.merchant_balance(), AMOUNT + AMOUNT * 4);
}

#[test]
fn a_lowered_price_takes_effect_after_the_notice_day() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();

    fixture.update_price(AMOUNT / 2).unwrap();
    fixture.env.advance_clock(PERIOD * 2);
    fixture.charge().unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT + AMOUNT / 2);
}

// Crank timing cannot be gamed

#[test]
fn an_uncranked_charge_is_billed_the_price_in_force_when_it_fell_due() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();
    fixture.env.advance_clock(PERIOD);

    fixture.env.advance_clock(DAY * 2);
    fixture.update_price(AMOUNT * 10).unwrap();
    fixture.env.advance_clock(DAY * 2);
    fixture.charge().unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT * 2);
}

#[test]
fn an_overdue_charge_settles_once_and_advances_by_whole_periods() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();

    let due_at = fixture
        .env
        .subscription(fixture.subscription)
        .next_charge_at;

    fixture.env.advance_clock(PERIOD * 3);
    fixture.charge().unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT * 2);
    let next = fixture
        .env
        .subscription(fixture.subscription)
        .next_charge_at;
    assert_eq!((next - due_at) % PERIOD, 0);

    assert_error(fixture.charge(), "PeriodNotElapsed");
}

// Price history that cannot answer

#[test]
fn two_matured_changes_over_an_uncranked_period_refuse_the_charge() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();
    fixture.env.advance_clock(PERIOD);

    fixture.update_price(AMOUNT * 2).unwrap();
    fixture.env.advance_clock(DAY * 2);
    fixture.update_price(AMOUNT * 3).unwrap();
    fixture.env.advance_clock(DAY * 2);

    let before = fixture.merchant_balance();
    let due_at = fixture
        .env
        .subscription(fixture.subscription)
        .next_charge_at;

    let result = fixture.charge();

    assert_error(result, "PriceHistoryUnavailable");
    assert_eq!(fixture.merchant_balance(), before);
    assert_eq!(
        fixture
            .env
            .subscription(fixture.subscription)
            .next_charge_at,
        due_at
    );
}

#[test]
fn a_stale_schedule_does_not_break_the_plan_for_other_subscribers() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();
    fixture.env.advance_clock(PERIOD);

    fixture.update_price(AMOUNT * 2).unwrap();
    fixture.env.advance_clock(DAY * 2);
    fixture.update_price(AMOUNT * 3).unwrap();
    fixture.env.advance_clock(DAY * 2);

    assert_error(fixture.charge(), "PriceHistoryUnavailable");

    let newcomer = fixture.env.funded_keypair();
    let newcomer_ata = fixture.env.create_ata(&newcomer, fixture.mint);
    fixture
        .env
        .mint_to(&fixture.merchant, fixture.mint, newcomer_ata, FUNDING);
    let (newcomer_subscription, _) = subscription_pda(&fixture.plan, &newcomer.pubkey());
    fixture
        .env
        .send(
            &[subscribe_ix(
                &newcomer.pubkey(),
                &fixture.plan,
                &newcomer_subscription,
                &newcomer_ata,
                &fixture.mint,
                ALLOWANCE,
                CAP,
            )],
            &[&newcomer],
        )
        .unwrap();

    let cranker = fixture.env.funded_keypair();
    fixture
        .env
        .send(
            &[charge_ix(
                &fixture.plan,
                &newcomer_subscription,
                &newcomer.pubkey(),
                &newcomer_ata,
                &fixture.merchant_ata,
                &fixture.mint,
            )],
            &[&cranker],
        )
        .unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT + AMOUNT * 3);
}

#[test]
fn one_matured_change_is_always_answerable() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();
    fixture.env.advance_clock(PERIOD);

    fixture.update_price(AMOUNT * 7).unwrap();
    fixture.env.advance_clock(DAY * 3);

    fixture.charge().unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT * 2);
}

// Interaction with the subscriber cap

#[test]
fn a_price_raised_above_the_cap_pauses_the_subscription() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(AMOUNT * 2).unwrap();
    fixture.charge().unwrap();

    fixture.update_price(AMOUNT * 3).unwrap();
    fixture.env.advance_clock(PERIOD * 2);

    let due_at = fixture
        .env
        .subscription(fixture.subscription)
        .next_charge_at;
    let result = fixture.charge();

    assert_error(result, "PriceAboveSubscriberMax");
    assert_eq!(fixture.merchant_balance(), AMOUNT);
    assert_eq!(
        fixture
            .env
            .subscription(fixture.subscription)
            .next_charge_at,
        due_at
    );
}

#[test]
fn raising_the_cap_resumes_a_subscription_paused_by_a_price_rise() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(AMOUNT * 2).unwrap();
    fixture.charge().unwrap();
    fixture.update_price(AMOUNT * 3).unwrap();
    fixture.env.advance_clock(PERIOD * 2);
    fixture.charge().unwrap_err();

    fixture
        .env
        .send(
            &[set_max_amount_ix(
                &fixture.subscriber.pubkey(),
                &fixture.plan,
                &fixture.subscription,
                AMOUNT * 5,
            )],
            &[&fixture.subscriber],
        )
        .unwrap();
    fixture.charge().unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT + AMOUNT * 3);
}

#[test]
fn a_rise_pauses_one_subscriber_and_not_another() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(AMOUNT * 2).unwrap();

    let generous = fixture.env.funded_keypair();
    let generous_ata = fixture.env.create_ata(&generous, fixture.mint);
    fixture
        .env
        .mint_to(&fixture.merchant, fixture.mint, generous_ata, FUNDING);
    let (generous_subscription, _) = subscription_pda(&fixture.plan, &generous.pubkey());
    fixture
        .env
        .send(
            &[subscribe_ix(
                &generous.pubkey(),
                &fixture.plan,
                &generous_subscription,
                &generous_ata,
                &fixture.mint,
                ALLOWANCE,
                AMOUNT * 10,
            )],
            &[&generous],
        )
        .unwrap();

    fixture.charge().unwrap();
    let cranker = fixture.env.funded_keypair();
    fixture
        .env
        .send(
            &[charge_ix(
                &fixture.plan,
                &generous_subscription,
                &generous.pubkey(),
                &generous_ata,
                &fixture.merchant_ata,
                &fixture.mint,
            )],
            &[&cranker],
        )
        .unwrap();
    assert_eq!(fixture.merchant_balance(), AMOUNT * 2);

    fixture.update_price(AMOUNT * 5).unwrap();
    fixture.env.advance_clock(PERIOD * 2);

    assert_error(fixture.charge(), "PriceAboveSubscriberMax");

    let cranker = fixture.env.funded_keypair();
    fixture
        .env
        .send(
            &[charge_ix(
                &fixture.plan,
                &generous_subscription,
                &generous.pubkey(),
                &generous_ata,
                &fixture.merchant_ata,
                &fixture.mint,
            )],
            &[&cranker],
        )
        .unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT * 2 + AMOUNT * 5);
}

#[test]
fn the_cap_is_compared_against_the_amount_actually_transferred() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(AMOUNT * 4).unwrap();
    fixture.charge().unwrap();

    fixture.update_price(AMOUNT * 5).unwrap();
    fixture.env.advance_clock(DAY * 2);
    fixture.env.advance_clock(PERIOD);

    fixture.update_price(AMOUNT).unwrap();

    let before = fixture.merchant_balance();
    let result = fixture.charge();

    assert_error(result, "PriceAboveSubscriberMax");
    assert_eq!(fixture.merchant_balance(), before);
}

// No regression

#[test]
fn charge_takes_no_write_on_the_plan() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();

    let generous = fixture.env.funded_keypair();
    let generous_ata = fixture.env.create_ata(&generous, fixture.mint);
    fixture
        .env
        .mint_to(&fixture.merchant, fixture.mint, generous_ata, FUNDING);
    let (generous_subscription, _) = subscription_pda(&fixture.plan, &generous.pubkey());
    fixture
        .env
        .send(
            &[subscribe_ix(
                &generous.pubkey(),
                &fixture.plan,
                &generous_subscription,
                &generous_ata,
                &fixture.mint,
                ALLOWANCE,
                CAP,
            )],
            &[&generous],
        )
        .unwrap();

    let before = fixture.env.plan(fixture.plan);
    let cranker = fixture.env.funded_keypair();
    fixture
        .env
        .send(
            &[
                charge_ix(
                    &fixture.plan,
                    &fixture.subscription,
                    &fixture.subscriber.pubkey(),
                    &fixture.subscriber_ata,
                    &fixture.merchant_ata,
                    &fixture.mint,
                ),
                charge_ix(
                    &fixture.plan,
                    &generous_subscription,
                    &generous.pubkey(),
                    &generous_ata,
                    &fixture.merchant_ata,
                    &fixture.mint,
                ),
            ],
            &[&cranker],
        )
        .unwrap();

    assert_eq!(fixture.merchant_balance(), AMOUNT * 2);

    let after = fixture.env.plan(fixture.plan);
    assert_eq!(after.amount_per_period, before.amount_per_period);
    assert_eq!(after.amount_effective_at, before.amount_effective_at);
    assert_eq!(after.previous_amount, before.previous_amount);
    assert_eq!(after.previous_effective_at, before.previous_effective_at);
}

#[test]
fn a_failed_charge_leaves_the_plan_and_the_next_price_intact() {
    let mut fixture = Fixture::variable();
    fixture.subscribe(CAP).unwrap();
    fixture.charge().unwrap();

    let before = fixture.env.plan(fixture.plan);
    assert_error(fixture.charge(), "PeriodNotElapsed");
    let after = fixture.env.plan(fixture.plan);
    assert_eq!(after.amount_per_period, before.amount_per_period);
    assert_eq!(after.previous_amount, before.previous_amount);

    fixture.env.advance_clock(PERIOD);
    fixture.charge().unwrap();
    assert_eq!(fixture.merchant_balance(), AMOUNT * 2);
}
