import { generateKeyPairSync, type JsonWebKey } from "node:crypto";
import {
  address as toAddress,
  getBase58Decoder,
  type Address,
} from "@solana/kit";

export type MockWallet = {
  name: string;
  address: Address;
  publicKeyBytes: Uint8Array;
  initScript: string;
};

function base64UrlToBytes(value: string): Uint8Array {
  return Uint8Array.from(Buffer.from(value, "base64url"));
}

export function createMockWallet(chain: string, name: string): MockWallet {
  const { publicKey: publicJwk, privateKey: privateJwk } = generateKeyPairSync(
    "ed25519",
    {
      publicKeyEncoding: { type: "spki", format: "jwk" },
      privateKeyEncoding: { type: "pkcs8", format: "jwk" },
    },
  ) as unknown as { publicKey: JsonWebKey; privateKey: JsonWebKey };

  const publicKeyBytes = base64UrlToBytes(publicJwk.x as string);
  const address = toAddress(getBase58Decoder().decode(publicKeyBytes));

  const initScript = `(() => {
    const CHAIN = ${JSON.stringify(chain)};
    const NAME = ${JSON.stringify(name)};
    const ADDRESS = ${JSON.stringify(address)};
    const PUBLIC_KEY_BYTES = new Uint8Array(${JSON.stringify(Array.from(publicKeyBytes))});
    const PRIVATE_JWK = ${JSON.stringify(privateJwk)};

    let signingKeyPromise = null;
    function getSigningKey() {
      if (!signingKeyPromise) {
        signingKeyPromise = crypto.subtle.importKey(
          "jwk",
          PRIVATE_JWK,
          { name: "Ed25519" },
          false,
          ["sign"],
        );
      }
      return signingKeyPromise;
    }

    function readShortU16(bytes, offset) {
      let value = 0;
      let shift = 0;
      let length = 0;
      for (;;) {
        const byte = bytes[offset + length];
        value |= (byte & 0x7f) << shift;
        length += 1;
        if ((byte & 0x80) === 0) break;
        shift += 7;
      }
      return [value, length];
    }

    function bytesEqual(a, b) {
      if (a.length !== b.length) return false;
      for (let i = 0; i < a.length; i++) {
        if (a[i] !== b[i]) return false;
      }
      return true;
    }

    function findSignerIndex(messageBytes) {
      let offset = 0;
      if ((messageBytes[0] & 0x80) !== 0) offset += 1;
      const numRequiredSignatures = messageBytes[offset];
      offset += 3;
      const [numAccounts, lengthBytes] = readShortU16(messageBytes, offset);
      offset += lengthBytes;
      for (let i = 0; i < Math.min(numAccounts, numRequiredSignatures); i++) {
        const candidate = messageBytes.subarray(offset + i * 32, offset + i * 32 + 32);
        if (bytesEqual(candidate, PUBLIC_KEY_BYTES)) return i;
      }
      return -1;
    }

    async function signOne(transactionBytes) {
      const [numSignatures, sigLenBytes] = readShortU16(transactionBytes, 0);
      const signaturesStart = sigLenBytes;
      const messageStart = signaturesStart + numSignatures * 64;
      const messageBytes = transactionBytes.subarray(messageStart);
      const signerIndex = findSignerIndex(messageBytes);
      if (signerIndex === -1) {
        throw new Error("Mock wallet: account is not a required signer of this transaction");
      }
      const signingKey = await getSigningKey();
      const signature = new Uint8Array(await crypto.subtle.sign("Ed25519", signingKey, messageBytes));
      const signed = transactionBytes.slice();
      signed.set(signature, signaturesStart + signerIndex * 64);
      return signed;
    }

    const listeners = new Set();

    const account = {
      address: ADDRESS,
      publicKey: PUBLIC_KEY_BYTES,
      chains: [CHAIN],
      features: ["solana:signTransaction"],
      label: "Playwright mock wallet",
    };

    const wallet = {
      version: "1.0.0",
      name: NAME,
      icon: "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciLz4=",
      chains: [CHAIN],
      accounts: [account],
      features: {
        "standard:connect": {
          version: "1.0.0",
          connect: async () => ({ accounts: wallet.accounts }),
        },
        "standard:disconnect": {
          version: "1.0.0",
          disconnect: async () => {},
        },
        "standard:events": {
          version: "1.0.0",
          on: (event, listener) => {
            listeners.add(listener);
            return () => listeners.delete(listener);
          },
        },
        "solana:signTransaction": {
          version: "1.0.0",
          supportedTransactionVersions: ["legacy", 0],
          signTransaction: async (...inputs) => {
            const outputs = [];
            for (const input of inputs) {
              const signedTransaction = await signOne(new Uint8Array(input.transaction));
              outputs.push({ signedTransaction });
            }
            return outputs;
          },
        },
      },
    };

    function registerWallet() {
      const callback = (api) => api.register(wallet);
      try {
        window.dispatchEvent(new CustomEvent("wallet-standard:register-wallet", { detail: callback }));
      } catch {}
      window.addEventListener("wallet-standard:app-ready", (event) => callback(event.detail));
    }

    registerWallet();
  })();`;

  return { name, address, publicKeyBytes, initScript };
}
