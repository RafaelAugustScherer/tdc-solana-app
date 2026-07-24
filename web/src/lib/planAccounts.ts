import {
  getAddressEncoder,
  getBase58Decoder,
  getBooleanEncoder,
  parseBase64RpcAccount,
  type Address,
  type Base58EncodedBytes,
} from "@solana/kit";
import {
  PLAN_DISCRIMINATOR,
  decodePlan,
  type Plan,
} from "../generated/accounts/plan";
import { APP_PROGRAM_ADDRESS } from "../generated/programs";
import { PLAN_IS_ACTIVE_OFFSET, PLAN_MERCHANT_OFFSET } from "./accountLayout";
import { client } from "./client";

const base58 = getBase58Decoder();

function toBase58(bytes: Uint8Array): Base58EncodedBytes {
  return base58.decode(bytes) as Base58EncodedBytes;
}

function discriminatorFilter() {
  return {
    memcmp: {
      offset: 0n,
      bytes: toBase58(PLAN_DISCRIMINATOR as Uint8Array),
      encoding: "base58" as const,
    },
  };
}

async function fetchPlansWithFilters(
  filters: ReturnType<typeof discriminatorFilter>[],
) {
  const rows = await client.rpc
    .getProgramAccounts(APP_PROGRAM_ADDRESS, {
      encoding: "base64",
      filters,
    })
    .send();

  return rows.map(({ account, pubkey }) => {
    const encoded = parseBase64RpcAccount(pubkey, account);
    return decodePlan(encoded);
  });
}

export async function fetchActivePlans() {
  return fetchPlansWithFilters([
    discriminatorFilter(),
    {
      memcmp: {
        offset: PLAN_IS_ACTIVE_OFFSET,
        bytes: toBase58(getBooleanEncoder().encode(true) as Uint8Array),
        encoding: "base58" as const,
      },
    },
  ]);
}

export async function fetchMerchantPlans(merchant: Address) {
  return fetchPlansWithFilters([
    discriminatorFilter(),
    {
      memcmp: {
        offset: PLAN_MERCHANT_OFFSET,
        bytes: toBase58(getAddressEncoder().encode(merchant) as Uint8Array),
        encoding: "base58" as const,
      },
    },
  ]);
}

export type { Plan };
