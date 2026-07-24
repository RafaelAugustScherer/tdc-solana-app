import { useAction, useRequest } from "@solana/react";
import { useState } from "react";
import { getCreateAssociatedTokenIdempotentInstructionAsync } from "@solana-program/token";
import { getSubscribeInstructionAsync } from "../generated/instructions/subscribe";
import { findSubscriberDelegationPda } from "../generated/pdas/subscriberDelegation";
import { fetchActivePlans, type Plan } from "../lib/planAccounts";
import { client } from "../lib/client";
import type { Account } from "@solana/kit";
import { ActionError } from "./ActionError";

export function PlanBrowser({ onSubscribed }: { onSubscribed: () => void }) {
  const { data: plans, status } = useRequest(fetchActivePlans);

  return (
    <section className="panel">
      <h2>Plan browser</h2>
      {status === "fetching" && !plans ? <p>Loading plans…</p> : null}
      {plans && plans.length === 0 ? <p>No active plans yet.</p> : null}
      <ul className="plan-list">
        {plans?.map((plan) => (
          <li key={plan.address} className="plan-card">
            <SubscribeRow plan={plan} onSubscribed={onSubscribed} />
          </li>
        ))}
      </ul>
    </section>
  );
}

function SubscribeRow({
  plan,
  onSubscribed,
}: {
  plan: Account<Plan>;
  onSubscribed: () => void;
}) {
  const nowSeconds = BigInt(Math.floor(Date.now() / 1000));
  const currentPrice =
    plan.data.amountEffectiveAt > nowSeconds
      ? plan.data.previousAmount
      : plan.data.amountPerPeriod;

  const [allowance, setAllowance] = useState("");
  const [maxAmountPerPeriod, setMaxAmountPerPeriod] = useState(
    currentPrice.toString(),
  );

  const subscribe = useAction(async (_signal) => {
    const subscriber = client.identity;
    const mint = plan.data.mint;
    const [subscriberDelegation] = await findSubscriberDelegationPda({
      subscriber: subscriber.address,
      mint,
    });
    const createAta = await getCreateAssociatedTokenIdempotentInstructionAsync({
      payer: subscriber,
      owner: subscriber.address,
      mint,
    });
    const subscribeIx = await getSubscribeInstructionAsync({
      subscriber,
      plan: plan.address,
      subscriberDelegation,
      mint,
      allowance: BigInt(allowance),
      maxAmountPerPeriod: BigInt(maxAmountPerPeriod),
    });
    await client.sendTransaction([createAta, subscribeIx]);
    onSubscribed();
  });

  return (
    <div>
      <p>
        Plan #{plan.data.planId.toString()} by {plan.data.merchant} —{" "}
        {currentPrice.toString()} per {plan.data.periodSeconds.toString()}s
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          subscribe.dispatch();
        }}
      >
        <label>
          Allowance
          <input
            value={allowance}
            onChange={(event) => setAllowance(event.target.value)}
            required
          />
        </label>
        <label>
          Max amount per period
          <input
            value={maxAmountPerPeriod}
            onChange={(event) => setMaxAmountPerPeriod(event.target.value)}
            required
          />
        </label>
        <button type="submit" disabled={subscribe.isRunning}>
          Subscribe
        </button>
      </form>
      <ActionError error={subscribe.error} />
    </div>
  );
}
