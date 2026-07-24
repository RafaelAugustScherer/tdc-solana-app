import {
  airdropFactory,
  appendTransactionMessageInstructions,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  generateKeyPairSigner,
  getSignatureFromTransaction,
  lamports,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  type Address,
  type KeyPairSigner,
} from "@solana/kit";
import {
  findAssociatedTokenPda,
  getCreateAssociatedTokenIdempotentInstructionAsync,
  getInitializeMintInstruction,
  getMintToInstruction,
  getMintSize,
  TOKEN_PROGRAM_ADDRESS,
} from "@solana-program/token";
import { getCreateAccountInstruction } from "@solana-program/system";

const rpcUrl = process.env.VITE_RPC_URL ?? "http://127.0.0.1:8899";
const rpcSubscriptionsUrl = rpcUrl
  .replace("http://", "ws://")
  .replace(/:8899$/, ":8900");

const rpc = createSolanaRpc(rpcUrl);
const rpcSubscriptions = createSolanaRpcSubscriptions(rpcSubscriptionsUrl);
const sendAndConfirm = sendAndConfirmTransactionFactory({
  rpc,
  rpcSubscriptions,
});
const airdrop = airdropFactory({ rpc, rpcSubscriptions });

export async function fundAddress(address: Address, sol: number) {
  await airdrop({
    commitment: "confirmed",
    lamports: lamports(BigInt(sol) * 1_000_000_000n),
    recipientAddress: address,
  });
}

async function sendInstructions(
  payer: KeyPairSigner,
  instructions: Parameters<typeof appendTransactionMessageInstructions>[0],
) {
  const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();
  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayerSigner(payer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    (m) => appendTransactionMessageInstructions(instructions, m),
  );
  const signed = await signTransactionMessageWithSigners(message);
  await sendAndConfirm(signed as Parameters<typeof sendAndConfirm>[0], {
    commitment: "confirmed",
  });
  return getSignatureFromTransaction(signed);
}

export async function createTestMint(): Promise<{
  mint: Address;
  authority: KeyPairSigner;
}> {
  const authority = await generateKeyPairSigner();
  await fundAddress(authority.address, 10);

  const mint = await generateKeyPairSigner();
  const space = BigInt(getMintSize());
  const rent = await rpc.getMinimumBalanceForRentExemption(space).send();

  await sendInstructions(authority, [
    getCreateAccountInstruction({
      payer: authority,
      newAccount: mint,
      lamports: rent,
      space,
      programAddress: TOKEN_PROGRAM_ADDRESS,
    }),
    getInitializeMintInstruction({
      mint: mint.address,
      decimals: 6,
      mintAuthority: authority.address,
    }),
  ]);

  return { mint: mint.address, authority };
}

export async function mintTokensTo(
  mint: Address,
  authority: KeyPairSigner,
  owner: Address,
  amount: bigint,
) {
  const [ata] = await findAssociatedTokenPda({
    owner,
    mint,
    tokenProgram: TOKEN_PROGRAM_ADDRESS,
  });

  await sendInstructions(authority, [
    await getCreateAssociatedTokenIdempotentInstructionAsync({
      payer: authority,
      owner,
      mint,
    }),
    getMintToInstruction({
      mint,
      token: ata,
      mintAuthority: authority.address,
      amount,
    }),
  ]);

  return ata;
}
