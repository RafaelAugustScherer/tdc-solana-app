import {
  getAddressEncoder,
  getBase58Decoder,
  parseBase64RpcAccount,
  type Address,
  type Base58EncodedBytes,
} from "@solana/kit";
import {
  SUBSCRIPTION_DISCRIMINATOR,
  decodeSubscription,
  type Subscription,
} from "../generated/accounts/subscription";
import { APP_PROGRAM_ADDRESS } from "../generated/programs";
import { SUBSCRIPTION_SUBSCRIBER_OFFSET } from "./accountLayout";
import { client } from "./client";

const base58 = getBase58Decoder();

function toBase58(bytes: Uint8Array): Base58EncodedBytes {
  return base58.decode(bytes) as Base58EncodedBytes;
}

export async function fetchSubscriberSubscriptions(subscriber: Address) {
  const rows = await client.rpc
    .getProgramAccounts(APP_PROGRAM_ADDRESS, {
      encoding: "base64",
      filters: [
        {
          memcmp: {
            offset: 0n,
            bytes: toBase58(SUBSCRIPTION_DISCRIMINATOR as Uint8Array),
            encoding: "base58" as const,
          },
        },
        {
          memcmp: {
            offset: SUBSCRIPTION_SUBSCRIBER_OFFSET,
            bytes: toBase58(
              getAddressEncoder().encode(subscriber) as Uint8Array,
            ),
            encoding: "base58" as const,
          },
        },
      ],
    })
    .send();

  return rows.map(({ account, pubkey }) => {
    const encoded = parseBase64RpcAccount(pubkey, account);
    return decodeSubscription(encoded);
  });
}

export type { Subscription };
