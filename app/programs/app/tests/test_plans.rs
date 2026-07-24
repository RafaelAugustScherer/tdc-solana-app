mod common;

use app::PriceMode;
use common::{
    assert_error, create_plan_ix, plan_pda, set_plan_active_ix, Env, TOKEN_2022_PROGRAM_ID,
};
use solana_signer::Signer;

const AMOUNT: u64 = 10_000_000;
const PERIOD: i64 = 30 * 24 * 60 * 60;

#[test]
fn create_plan_stores_every_field() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
    let mint = env.create_mint(&merchant, 6);
    let (plan, bump) = plan_pda(&merchant.pubkey(), 1);

    env.send(
        &[create_plan_ix(
            &merchant.pubkey(),
            &plan,
            &mint,
            1,
            AMOUNT,
            PERIOD,
            PriceMode::Variable,
        )],
        &[&merchant],
    )
    .unwrap();

    let stored = env.plan(plan);
    assert_eq!(stored.merchant, merchant.pubkey());
    assert_eq!(stored.mint, mint);
    assert_eq!(stored.plan_id, 1);
    assert_eq!(stored.amount_per_period, AMOUNT);
    assert_eq!(stored.period_seconds, PERIOD);
    assert_eq!(stored.price_mode, PriceMode::Variable);
    assert!(stored.is_active);
    assert_eq!(stored.bump, bump);
}

#[test]
fn create_plan_rejects_zero_amount() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
    let mint = env.create_mint(&merchant, 6);
    let (plan, _) = plan_pda(&merchant.pubkey(), 1);

    let result = env.send(
        &[create_plan_ix(
            &merchant.pubkey(),
            &plan,
            &mint,
            1,
            0,
            PERIOD,
            PriceMode::Fixed,
        )],
        &[&merchant],
    );

    assert_error(result, "InvalidAmount");
}

#[test]
fn create_plan_rejects_zero_period() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
    let mint = env.create_mint(&merchant, 6);
    let (plan, _) = plan_pda(&merchant.pubkey(), 1);

    let result = env.send(
        &[create_plan_ix(
            &merchant.pubkey(),
            &plan,
            &mint,
            1,
            AMOUNT,
            0,
            PriceMode::Fixed,
        )],
        &[&merchant],
    );

    assert_error(result, "InvalidPeriod");
}

#[test]
fn create_plan_rejects_negative_period() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
    let mint = env.create_mint(&merchant, 6);
    let (plan, _) = plan_pda(&merchant.pubkey(), 1);

    let result = env.send(
        &[create_plan_ix(
            &merchant.pubkey(),
            &plan,
            &mint,
            1,
            AMOUNT,
            -PERIOD,
            PriceMode::Fixed,
        )],
        &[&merchant],
    );

    assert_error(result, "InvalidPeriod");
}

#[test]
fn reusing_a_plan_id_fails_and_leaves_the_original() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
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

    let result = env.send(
        &[create_plan_ix(
            &merchant.pubkey(),
            &plan,
            &mint,
            1,
            AMOUNT * 2,
            PERIOD,
            PriceMode::Fixed,
        )],
        &[&merchant],
    );

    assert!(result.is_err());
    assert_eq!(env.plan(plan).amount_per_period, AMOUNT);
}

#[test]
fn the_same_plan_id_under_a_different_merchant_is_a_distinct_plan() {
    let mut env = Env::new();
    let first = env.funded_keypair();
    let second = env.funded_keypair();
    let mint = env.create_mint(&first, 6);

    let (first_plan, _) = plan_pda(&first.pubkey(), 1);
    let (second_plan, _) = plan_pda(&second.pubkey(), 1);
    assert_ne!(first_plan, second_plan);

    for (merchant, plan) in [(&first, first_plan), (&second, second_plan)] {
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
            &[merchant],
        )
        .unwrap();
    }

    assert_eq!(env.plan(first_plan).merchant, first.pubkey());
    assert_eq!(env.plan(second_plan).merchant, second.pubkey());
}

#[test]
fn one_merchant_can_publish_several_plans_on_one_mint() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
    let mint = env.create_mint(&merchant, 6);

    let (monthly, _) = plan_pda(&merchant.pubkey(), 1);
    let (yearly, _) = plan_pda(&merchant.pubkey(), 2);

    env.send(
        &[
            create_plan_ix(
                &merchant.pubkey(),
                &monthly,
                &mint,
                1,
                AMOUNT,
                PERIOD,
                PriceMode::Fixed,
            ),
            create_plan_ix(
                &merchant.pubkey(),
                &yearly,
                &mint,
                2,
                AMOUNT * 10,
                PERIOD * 12,
                PriceMode::Variable,
            ),
        ],
        &[&merchant],
    )
    .unwrap();

    assert_eq!(env.plan(monthly).amount_per_period, AMOUNT);
    assert_eq!(env.plan(monthly).period_seconds, PERIOD);
    assert_eq!(env.plan(yearly).amount_per_period, AMOUNT * 10);
    assert_eq!(env.plan(yearly).period_seconds, PERIOD * 12);
}

#[test]
fn a_token_2022_mint_cannot_be_used_to_publish_a_plan() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
    let mint = env.create_mint_on(&merchant, 6, TOKEN_2022_PROGRAM_ID);
    let (plan, _) = plan_pda(&merchant.pubkey(), 1);

    let result = env.send(
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
    );

    assert!(result.is_err());
    assert!(env
        .svm
        .get_account(&plan)
        .is_none_or(|account| account.data.is_empty()));
}

#[test]
fn a_merchant_can_retire_and_republish_their_plan() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
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

    env.send(
        &[set_plan_active_ix(&merchant.pubkey(), &plan, false)],
        &[&merchant],
    )
    .unwrap();
    assert!(!env.plan(plan).is_active);

    env.send(
        &[set_plan_active_ix(&merchant.pubkey(), &plan, true)],
        &[&merchant],
    )
    .unwrap();
    assert!(env.plan(plan).is_active);
}

#[test]
fn only_the_merchant_can_retire_a_plan() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
    let stranger = env.funded_keypair();
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

    let result = env.send(
        &[set_plan_active_ix(&stranger.pubkey(), &plan, false)],
        &[&stranger],
    );

    assert!(result.is_err());
    assert!(env.plan(plan).is_active);
}

#[test]
fn an_account_that_is_not_a_plan_is_rejected() {
    let mut env = Env::new();
    let merchant = env.funded_keypair();
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

    let mut instruction = set_plan_active_ix(&merchant.pubkey(), &plan, false);
    instruction.accounts[1].pubkey = mint;
    let result = env.send(&[instruction], &[&merchant]);

    assert!(result.is_err());
    assert!(env.plan(plan).is_active);
}
