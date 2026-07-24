import { createClient } from "@solana/kit";
import { solanaRpc } from "@solana/kit-plugin-rpc";
import { walletSigner } from "@solana/kit-plugin-wallet";

const rpcUrl = import.meta.env.VITE_RPC_URL ?? "http://127.0.0.1:8899";
const chain = (import.meta.env.VITE_CHAIN ??
  "solana:devnet") as `solana:${string}`;

export const client = createClient()
  .use(walletSigner({ chain }))
  .use(solanaRpc({ rpcUrl }));

export type AppClient = typeof client;
