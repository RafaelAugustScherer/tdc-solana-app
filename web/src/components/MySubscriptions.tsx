import { useAction, useRequest } from "@solana/react";
import { useCallback, useState } from "react";
import { TOKEN_PROGRAM_ADDRESS } from "@solana-program/token";
import { findAssociatedTokenPda } from "@solana-program/token";
import { fetchPlan } from "../generated/accounts/plan";
import { getCancelInstructionAsync } from "../generated/instructions/cancel";
import { getChargeInstructionAsync } from "../generated/instructions/charge";
import { getReauthorizeInstructionAsync } from "../generated/instructions/reauthorize";
import { getSetAllowanceInstructionAsync } from "../generated/instructions/setAllowance";
import { getSetMaxAmountInstructionAsync } from "../generated/instructions/setMaxAmount";
import { findSubscriberDelegationPda } from "../generated/pdas/subscriberDelegation";
import {
  fetchSubscriberSubscriptions,
  type Subscription,
} from "../lib/subscriptionAccounts";
import { client } from "../lib/client";
import type { Account, Address } from "@solana/kit";
import { ActionError } from "./ActionError";

export function MySubscriptions({ subscriber }: { subscriber: Address }) {
  const source = useCallback(
    () => fetchSubscriberSubscriptions(subscriber),
    [subscriber],
  );
  const { data: subscriptions, refresh, status } = useRequest(source);

  return (
    <section className="panel">
      <h2>My subscriptions</h2>
      {status === "fetching" && !subscriptions ? (
        <p>Loading subscriptions…</p>
      ) : null}
      {subscriptions && subscriptions.length === 0 ? (
        <p>No subscriptions yet.</p>
      ) : null}
      <ul className="plan-list">
        {subscriptions?.map((subscription) => (
          <li key={subscription.address} className="plan-card">
            <SubscriptionRow subscription={subscription} onChanged={refresh} />
          </li>
        ))}
      </ul>
    </section>
  );
}

function SubscriptionRow({
  subscription,
  onChanged,
}: {
  subscription: Account<Subscription>;
  onChanged: () => void;
}) {
  const planSource = useCallback(
    () => fetchPlan(client.rpc, subscription.data.plan),
    [subscription.data.plan],
  );
  const { data: plan } = useRequest(planSource);

  const [newAllowance, setNewAllowance] = useState(
    subscription.data.allowanceRemaining.toString(),
  );
  const [newMax, setNewMax] = useState(
    subscription.data.maxAmountPerPeriod.toString(),
  );

  const setAllowance = useAction(async (_signal) => {
    if (!plan) return;
    const [subscriberDelegation] = await findSubscriberDelegationPda({
      subscriber: subscription.data.subscriber,
      mint: plan.data.mint,
    });
    const ix = await getSetAllowanceInstructionAsync({
      subscriber: client.identity,
      plan: subscription.data.plan,
      subscriberDelegation,
      mint: plan.data.mint,
      newAllowance: BigInt(newAllowance),
    });
    await client.sendTransaction([ix]);
    onChanged();
  });

  const setMaxAmount = useAction(async (_signal) => {
    const ix = await getSetMaxAmountInstructionAsync({
      subscriber: client.identity,
      plan: subscription.data.plan,
      newMax: BigInt(newMax),
    });
    await client.sendTransaction([ix]);
    onChanged();
  });

  const reauthorize = useAction(async (_signal) => {
    if (!plan) return;
    const ix = await getReauthorizeInstructionAsync({
      subscriber: client.identity,
      mint: plan.data.mint,
    });
    await client.sendTransaction([ix]);
    onChanged();
  });

  const cancel = useAction(async (_signal) => {
    if (!plan) return;
    const [subscriberDelegation] = await findSubscriberDelegationPda({
      subscriber: subscription.data.subscriber,
      mint: plan.data.mint,
    });
    const ix = await getCancelInstructionAsync({
      subscriber: client.identity,
      plan: subscription.data.plan,
      subscriberDelegation,
      mint: plan.data.mint,
    });
    await client.sendTransaction([ix]);
    onChanged();
  });

  const charge = useAction(async (_signal) => {
    if (!plan) return;
    const [subscriberDelegation] = await findSubscriberDelegationPda({
      subscriber: subscription.data.subscriber,
      mint: plan.data.mint,
    });
    const [subscriberTokenAccount] = await findAssociatedTokenPda({
      owner: subscription.data.subscriber,
      mint: plan.data.mint,
      tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });
    const [merchantTokenAccount] = await findAssociatedTokenPda({
      owner: plan.data.merchant,
      mint: plan.data.mint,
      tokenProgram: TOKEN_PROGRAM_ADDRESS,
    });
    const ix = await getChargeInstructionAsync({
      plan: subscription.data.plan,
      subscription: subscription.address,
      subscriberDelegation,
      subscriberTokenAccount,
      merchantTokenAccount,
      mint: plan.data.mint,
    });
    await client.sendTransaction([ix]);
    onChanged();
  });

  const nowSeconds = BigInt(Math.floor(Date.now() / 1000));
  const isDue = subscription.data.nextChargeAt <= nowSeconds;
  const busy =
    setAllowance.isRunning ||
    setMaxAmount.isRunning ||
    reauthorize.isRunning ||
    cancel.isRunning ||
    charge.isRunning;
  const actionError =
    setAllowance.error ??
    setMaxAmount.error ??
    reauthorize.error ??
    cancel.error ??
    charge.error;

  return (
    <div>
      <p>
        Plan {subscription.data.plan} — allowance remaining{" "}
        {subscription.data.allowanceRemaining.toString()} — max{" "}
        {subscription.data.maxAmountPerPeriod.toString()} — next charge{" "}
        {new Date(
          Number(subscription.data.nextChargeAt) * 1000,
        ).toLocaleString()}
      </p>

      <form
        onSubmit={(event) => {
          event.preventDefault();
          setAllowance.dispatch();
        }}
      >
        <label>
          New allowance
          <input
            value={newAllowance}
            onChange={(event) => setNewAllowance(event.target.value)}
          />
        </label>
        <button type="submit" disabled={busy}>
          Set allowance
        </button>
      </form>

      <form
        onSubmit={(event) => {
          event.preventDefault();
          setMaxAmount.dispatch();
        }}
      >
        <label>
          New max amount per period
          <input
            value={newMax}
            onChange={(event) => setNewMax(event.target.value)}
          />
        </label>
        <button type="submit" disabled={busy}>
          Set max amount
        </button>
      </form>

      <button disabled={busy} onClick={() => reauthorize.dispatch()}>
        Reauthorize
      </button>
      <button disabled={busy} onClick={() => cancel.dispatch()}>
        Cancel
      </button>
      {isDue ? (
        <button disabled={busy} onClick={() => charge.dispatch()}>
          Trigger charge
        </button>
      ) : null}
      <ActionError error={actionError} />
    </div>
  );
}
