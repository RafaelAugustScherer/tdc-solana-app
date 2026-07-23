#![allow(dead_code)]

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use litesvm::{types::FailedTransactionMetadata, LiteSVM};
use solana_clock::Clock;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

pub const TOKEN_PROGRAM_ID: Pubkey = anchor_spl::token::ID;
pub const TOKEN_2022_PROGRAM_ID: Pubkey = anchor_spl::token_2022::ID;

pub struct Env {
    pub svm: LiteSVM,
}

impl Env {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new().with_default_programs();
        svm.add_program(app::ID, include_bytes!("../../../../target/deploy/app.so"))
            .unwrap();

        Self { svm }
    }

    pub fn funded_keypair(&mut self) -> Keypair {
        let keypair = Keypair::new();
        self.svm
            .airdrop(&keypair.pubkey(), 100_000_000_000)
            .unwrap();
        keypair
    }

    pub fn send(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<(), Box<FailedTransactionMetadata>> {
        self.svm.expire_blockhash();
        let blockhash = self.svm.latest_blockhash();
        let message =
            Message::new_with_blockhash(instructions, Some(&signers[0].pubkey()), &blockhash);
        let transaction =
            VersionedTransaction::try_new(VersionedMessage::Legacy(message), signers).unwrap();

        self.svm
            .send_transaction(transaction)
            .map(|_| ())
            .map_err(Box::new)
    }

    pub fn create_mint(&mut self, authority: &Keypair, decimals: u8) -> Pubkey {
        self.create_mint_on(authority, decimals, TOKEN_PROGRAM_ID)
    }

    pub fn create_mint_on(
        &mut self,
        authority: &Keypair,
        decimals: u8,
        token_program: Pubkey,
    ) -> Pubkey {
        let mint = Keypair::new();
        let space = spl_token_interface::state::Mint::LEN;
        let lamports = self.svm.minimum_balance_for_rent_exemption(space);

        let create = solana_system_interface::instruction::create_account(
            &authority.pubkey(),
            &mint.pubkey(),
            lamports,
            space as u64,
            &token_program,
        );
        let mut initialize = spl_token_interface::instruction::initialize_mint2(
            &TOKEN_PROGRAM_ID,
            &mint.pubkey(),
            &authority.pubkey(),
            None,
            decimals,
        )
        .unwrap();
        initialize.program_id = token_program;

        self.send(&[create, initialize], &[authority, &mint])
            .unwrap();

        mint.pubkey()
    }

    pub fn create_ata(&mut self, owner: &Keypair, mint: Pubkey) -> Pubkey {
        let instruction =
            spl_associated_token_account_interface::instruction::create_associated_token_account(
                &owner.pubkey(),
                &owner.pubkey(),
                &mint,
                &TOKEN_PROGRAM_ID,
            );

        self.send(&[instruction], &[owner]).unwrap();

        ata(&owner.pubkey(), &mint)
    }

    pub fn create_auxiliary_token_account(&mut self, owner: &Keypair, mint: Pubkey) -> Pubkey {
        let account = Keypair::new();
        let space = spl_token_interface::state::Account::LEN;
        let lamports = self.svm.minimum_balance_for_rent_exemption(space);

        let create = solana_system_interface::instruction::create_account(
            &owner.pubkey(),
            &account.pubkey(),
            lamports,
            space as u64,
            &TOKEN_PROGRAM_ID,
        );
        let initialize = spl_token_interface::instruction::initialize_account3(
            &TOKEN_PROGRAM_ID,
            &account.pubkey(),
            &mint,
            &owner.pubkey(),
        )
        .unwrap();

        self.send(&[create, initialize], &[owner, &account])
            .unwrap();

        account.pubkey()
    }

    pub fn mint_to(&mut self, authority: &Keypair, mint: Pubkey, to: Pubkey, amount: u64) {
        let instruction = spl_token_interface::instruction::mint_to(
            &TOKEN_PROGRAM_ID,
            &mint,
            &to,
            &authority.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        self.send(&[instruction], &[authority]).unwrap();
    }

    pub fn approve(&mut self, owner: &Keypair, account: Pubkey, delegate: Pubkey, amount: u64) {
        let instruction = spl_token_interface::instruction::approve(
            &TOKEN_PROGRAM_ID,
            &account,
            &delegate,
            &owner.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        self.send(&[instruction], &[owner]).unwrap();
    }

    pub fn revoke(&mut self, owner: &Keypair, account: Pubkey) {
        let instruction = spl_token_interface::instruction::revoke(
            &TOKEN_PROGRAM_ID,
            &account,
            &owner.pubkey(),
            &[],
        )
        .unwrap();

        self.send(&[instruction], &[owner]).unwrap();
    }

    pub fn advance_clock(&mut self, seconds: i64) {
        let mut clock: Clock = self.svm.get_sysvar();
        clock.unix_timestamp += seconds;
        self.svm.set_sysvar(&clock);
    }

    pub fn plan(&self, address: Pubkey) -> app::Plan {
        let account = self.svm.get_account(&address).unwrap();
        app::Plan::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn subscription(&self, address: Pubkey) -> app::Subscription {
        let account = self.svm.get_account(&address).unwrap();
        app::Subscription::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn subscriber_delegation(&self, address: Pubkey) -> app::SubscriberDelegation {
        let account = self.svm.get_account(&address).unwrap();
        app::SubscriberDelegation::try_deserialize(&mut account.data.as_slice()).unwrap()
    }

    pub fn committed_total(&self, subscriber: &Pubkey, mint: &Pubkey) -> u64 {
        self.subscriber_delegation(delegation_pda(subscriber, mint).0)
            .committed_total
    }

    pub fn delegated_amount(&self, address: Pubkey) -> u64 {
        self.token_account(address).delegated_amount
    }

    pub fn subscription_exists(&self, address: Pubkey) -> bool {
        self.svm
            .get_account(&address)
            .is_some_and(|account| !account.data.is_empty())
    }

    pub fn token_account(&self, address: Pubkey) -> spl_token_interface::state::Account {
        let account = self.svm.get_account(&address).unwrap();
        spl_token_interface::state::Account::unpack(&account.data).unwrap()
    }

    pub fn balance(&self, address: Pubkey) -> u64 {
        self.token_account(address).amount
    }
}

pub fn ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
        owner,
        mint,
        &TOKEN_PROGRAM_ID,
    )
}

pub fn subscription_pda(plan: &Pubkey, subscriber: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"subscription", plan.as_ref(), subscriber.as_ref()],
        &app::ID,
    )
}

pub fn delegate_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"delegate"], &app::ID)
}

pub fn delegation_pda(subscriber: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"delegation", subscriber.as_ref(), mint.as_ref()],
        &app::ID,
    )
}

pub fn subscribe_ix(
    subscriber: &Pubkey,
    plan: &Pubkey,
    subscription: &Pubkey,
    subscriber_token_account: &Pubkey,
    mint: &Pubkey,
    allowance: u64,
    max_amount_per_period: u64,
) -> Instruction {
    Instruction {
        program_id: app::ID,
        accounts: app::accounts::Subscribe {
            subscriber: *subscriber,
            plan: *plan,
            subscription: *subscription,
            subscriber_delegation: delegation_pda(subscriber, mint).0,
            subscriber_token_account: *subscriber_token_account,
            mint: *mint,
            delegate_authority: delegate_pda().0,
            token_program: TOKEN_PROGRAM_ID,
            system_program: solana_system_interface::program::ID,
        }
        .to_account_metas(None),
        data: app::instruction::Subscribe {
            allowance,
            max_amount_per_period,
        }
        .data(),
    }
}

pub fn charge_ix(
    plan: &Pubkey,
    subscription: &Pubkey,
    subscriber: &Pubkey,
    subscriber_token_account: &Pubkey,
    merchant_token_account: &Pubkey,
    mint: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: app::ID,
        accounts: app::accounts::Charge {
            plan: *plan,
            subscription: *subscription,
            subscriber_delegation: delegation_pda(subscriber, mint).0,
            subscriber_token_account: *subscriber_token_account,
            merchant_token_account: *merchant_token_account,
            mint: *mint,
            delegate_authority: delegate_pda().0,
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: app::instruction::Charge {}.data(),
    }
}

pub fn cancel_ix(
    subscriber: &Pubkey,
    plan: &Pubkey,
    subscription: &Pubkey,
    subscriber_token_account: &Pubkey,
    mint: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: app::ID,
        accounts: app::accounts::Cancel {
            subscriber: *subscriber,
            plan: *plan,
            subscription: *subscription,
            subscriber_delegation: delegation_pda(subscriber, mint).0,
            subscriber_token_account: *subscriber_token_account,
            mint: *mint,
            delegate_authority: delegate_pda().0,
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: app::instruction::Cancel {}.data(),
    }
}

pub fn update_price_ix(merchant: &Pubkey, plan: &Pubkey, new_amount: u64) -> Instruction {
    Instruction {
        program_id: app::ID,
        accounts: app::accounts::UpdatePrice {
            merchant: *merchant,
            plan: *plan,
        }
        .to_account_metas(None),
        data: app::instruction::UpdatePrice { new_amount }.data(),
    }
}

pub fn set_max_amount_ix(
    subscriber: &Pubkey,
    plan: &Pubkey,
    subscription: &Pubkey,
    new_max: u64,
) -> Instruction {
    Instruction {
        program_id: app::ID,
        accounts: app::accounts::SetMaxAmount {
            subscriber: *subscriber,
            plan: *plan,
            subscription: *subscription,
        }
        .to_account_metas(None),
        data: app::instruction::SetMaxAmount { new_max }.data(),
    }
}

pub fn set_allowance_ix(
    subscriber: &Pubkey,
    plan: &Pubkey,
    subscription: &Pubkey,
    subscriber_token_account: &Pubkey,
    mint: &Pubkey,
    new_allowance: u64,
) -> Instruction {
    Instruction {
        program_id: app::ID,
        accounts: app::accounts::SetAllowance {
            subscriber: *subscriber,
            plan: *plan,
            subscription: *subscription,
            subscriber_delegation: delegation_pda(subscriber, mint).0,
            subscriber_token_account: *subscriber_token_account,
            mint: *mint,
            delegate_authority: delegate_pda().0,
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: app::instruction::SetAllowance { new_allowance }.data(),
    }
}

pub fn reauthorize_ix(
    subscriber: &Pubkey,
    subscriber_token_account: &Pubkey,
    mint: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: app::ID,
        accounts: app::accounts::Reauthorize {
            subscriber: *subscriber,
            subscriber_delegation: delegation_pda(subscriber, mint).0,
            subscriber_token_account: *subscriber_token_account,
            mint: *mint,
            delegate_authority: delegate_pda().0,
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: app::instruction::Reauthorize {}.data(),
    }
}

pub fn plan_pda(merchant: &Pubkey, plan_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"plan", merchant.as_ref(), &plan_id.to_le_bytes()],
        &app::ID,
    )
}

pub fn create_plan_ix(
    merchant: &Pubkey,
    plan: &Pubkey,
    mint: &Pubkey,
    plan_id: u64,
    amount_per_period: u64,
    period_seconds: i64,
    price_mode: app::PriceMode,
) -> Instruction {
    Instruction {
        program_id: app::ID,
        accounts: app::accounts::CreatePlan {
            merchant: *merchant,
            plan: *plan,
            mint: *mint,
            token_program: TOKEN_PROGRAM_ID,
            system_program: solana_system_interface::program::ID,
        }
        .to_account_metas(None),
        data: app::instruction::CreatePlan {
            plan_id,
            amount_per_period,
            period_seconds,
            price_mode,
        }
        .data(),
    }
}

pub fn set_plan_active_ix(merchant: &Pubkey, plan: &Pubkey, is_active: bool) -> Instruction {
    Instruction {
        program_id: app::ID,
        accounts: app::accounts::SetPlanActive {
            merchant: *merchant,
            plan: *plan,
        }
        .to_account_metas(None),
        data: app::instruction::SetPlanActive { is_active }.data(),
    }
}

pub fn assert_error(result: Result<(), Box<FailedTransactionMetadata>>, expected: &str) {
    let failure = result.expect_err("expected the transaction to fail");
    let logs = failure.meta.logs.join("\n");
    assert!(
        logs.contains(expected),
        "expected `{expected}` in logs, got:\n{logs}"
    );
}
