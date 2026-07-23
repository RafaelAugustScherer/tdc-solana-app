use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use litesvm::{types::FailedTransactionMetadata, LiteSVM};
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

    pub fn plan(&self, address: Pubkey) -> app::Plan {
        let account = self.svm.get_account(&address).unwrap();
        app::Plan::try_deserialize(&mut account.data.as_slice()).unwrap()
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
