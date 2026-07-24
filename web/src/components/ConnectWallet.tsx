import {
  useConnect,
  useConnectedWallet,
  useDisconnect,
  useWallets,
  useWalletStatus,
} from "@solana/kit-plugin-wallet/react";
import { client } from "../lib/client";

export function ConnectWallet() {
  const status = useWalletStatus(client);
  const wallets = useWallets(client);
  const connected = useConnectedWallet(client);
  const { dispatch: connect, isRunning: isConnecting } = useConnect(client);
  const { dispatch: disconnect, isRunning: isDisconnecting } =
    useDisconnect(client);

  if (status === "pending") {
    return null;
  }

  if (connected) {
    return (
      <div className="wallet-bar">
        <span className="wallet-address">{connected.account.address}</span>
        <button disabled={isDisconnecting} onClick={() => disconnect()}>
          Disconnect
        </button>
      </div>
    );
  }

  if (wallets.length === 0) {
    return <p className="wallet-bar">No Wallet Standard wallet detected.</p>;
  }

  return (
    <div className="wallet-bar">
      {wallets.map((wallet) => (
        <button
          key={wallet.name}
          disabled={isConnecting}
          onClick={() => connect(wallet)}
        >
          Connect {wallet.name}
        </button>
      ))}
    </div>
  );
}
