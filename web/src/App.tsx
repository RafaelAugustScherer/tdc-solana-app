import { useConnectedWallet } from "@solana/kit-plugin-wallet/react";
import { address } from "@solana/kit";
import { useState } from "react";
import { ConnectWallet } from "./components/ConnectWallet";
import { MerchantDashboard } from "./components/MerchantDashboard";
import { MySubscriptions } from "./components/MySubscriptions";
import { PlanBrowser } from "./components/PlanBrowser";
import { client } from "./lib/client";

type Tab = "merchant" | "browse" | "subscriptions";

function App() {
  const connected = useConnectedWallet(client);
  const [tab, setTab] = useState<Tab>("browse");
  const [subscribedAt, setSubscribedAt] = useState(0);
  const connectedAddress = connected
    ? address(connected.account.address)
    : null;

  return (
    <div className="app">
      <header>
        <h1>Subscriptions</h1>
        <ConnectWallet />
      </header>

      {connectedAddress ? (
        <>
          <nav className="tabs">
            <button
              className={tab === "merchant" ? "active" : ""}
              onClick={() => setTab("merchant")}
            >
              Merchant
            </button>
            <button
              className={tab === "browse" ? "active" : ""}
              onClick={() => setTab("browse")}
            >
              Browse plans
            </button>
            <button
              className={tab === "subscriptions" ? "active" : ""}
              onClick={() => setTab("subscriptions")}
            >
              My subscriptions
            </button>
          </nav>

          {tab === "merchant" ? (
            <MerchantDashboard merchant={connectedAddress} />
          ) : null}
          {tab === "browse" ? (
            <PlanBrowser
              key={subscribedAt}
              onSubscribed={() => setSubscribedAt(Date.now())}
            />
          ) : null}
          {tab === "subscriptions" ? (
            <MySubscriptions subscriber={connectedAddress} />
          ) : null}
        </>
      ) : (
        <p>Connect a wallet to get started.</p>
      )}
    </div>
  );
}

export default App;
