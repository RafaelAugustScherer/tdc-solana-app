import { useAction, useRequest } from "@solana/react";
import { address, type Address } from "@solana/kit";
import { getCreateAssociatedTokenIdempotentInstructionAsync } from "@solana-program/token";
import { useCallback, useState } from "react";
import { getCreatePlanInstructionAsync } from "../generated/instructions/createPlan";
import { getSetPlanActiveInstruction } from "../generated/instructions/setPlanActive";
import { getUpdatePriceInstruction } from "../generated/instructions/updatePrice";
import { PriceMode } from "../generated/types/priceMode";
import { fetchMerchantPlans } from "../lib/planAccounts";
import { client } from "../lib/client";
import { PRICE_CHANGE_NOTICE_SECONDS } from "../lib/constants";
import { ActionError } from "./ActionError";

export function MerchantDashboard({ merchant }: { merchant: Address }) {
  const source = useCallback(() => fetchMerchantPlans(merchant), [merchant]);
  const { data: plans, refresh, status } = useRequest(source);

  const createPlan = useAction(async (_signal, form: CreatePlanFormValues) => {
    const mint = address(form.mint);
    const createMerchantAta =
      await getCreateAssociatedTokenIdempotentInstructionAsync({
        payer: client.identity,
        owner: merchant,
        mint,
      });
    const createPlanIx = await getCreatePlanInstructionAsync({
      merchant: client.identity,
      mint,
      planId: BigInt(form.planId),
      amountPerPeriod: BigInt(form.amountPerPeriod),
      periodSeconds: BigInt(form.periodSeconds),
      priceMode: form.priceMode,
    });
    await client.sendTransaction([createMerchantAta, createPlanIx]);
    refresh();
  });

  const toggleActive = useAction(
    async (_signal, plan: Address, isActive: boolean) => {
      const ix = getSetPlanActiveInstruction({
        merchant: client.identity,
        plan,
        isActive,
      });
      await client.sendTransaction([ix]);
      refresh();
    },
  );

  const updatePrice = useAction(
    async (_signal, plan: Address, newAmount: bigint) => {
      const ix = getUpdatePriceInstruction({
        merchant: client.identity,
        plan,
        newAmount,
      });
      await client.sendTransaction([ix]);
      refresh();
    },
  );

  return (
    <section className="panel">
      <h2>Merchant dashboard</h2>
      <CreatePlanForm
        isRunning={createPlan.isRunning}
        onSubmit={createPlan.dispatch}
      />
      <ActionError error={createPlan.error} />

      {status === "fetching" && !plans ? <p>Loading plans…</p> : null}
      {plans && plans.length === 0 ? <p>No plans yet.</p> : null}

      <ul className="plan-list">
        {plans?.map((plan) => (
          <li key={plan.address} className="plan-card">
            <PlanRow
              plan={plan}
              onToggleActive={(isActive) =>
                toggleActive.dispatch(plan.address, isActive)
              }
              onUpdatePrice={(newAmount) =>
                updatePrice.dispatch(plan.address, newAmount)
              }
              isBusy={toggleActive.isRunning || updatePrice.isRunning}
              error={toggleActive.error ?? updatePrice.error}
            />
          </li>
        ))}
      </ul>
    </section>
  );
}

type CreatePlanFormValues = {
  planId: string;
  mint: string;
  amountPerPeriod: string;
  periodSeconds: string;
  priceMode: PriceMode;
};

function CreatePlanForm({
  isRunning,
  onSubmit,
}: {
  isRunning: boolean;
  onSubmit: (values: CreatePlanFormValues) => void;
}) {
  const [values, setValues] = useState<CreatePlanFormValues>({
    planId: String(Date.now()),
    mint: "",
    amountPerPeriod: "",
    periodSeconds: "",
    priceMode: PriceMode.Fixed,
  });

  return (
    <form
      className="create-plan-form"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit(values);
      }}
    >
      <label>
        Plan ID
        <input
          value={values.planId}
          onChange={(event) =>
            setValues({ ...values, planId: event.target.value })
          }
          required
        />
      </label>
      <label>
        Mint address
        <input
          value={values.mint}
          onChange={(event) =>
            setValues({ ...values, mint: event.target.value })
          }
          required
        />
      </label>
      <label>
        Amount per period
        <input
          value={values.amountPerPeriod}
          onChange={(event) =>
            setValues({ ...values, amountPerPeriod: event.target.value })
          }
          required
        />
      </label>
      <label>
        Period (seconds)
        <input
          value={values.periodSeconds}
          onChange={(event) =>
            setValues({ ...values, periodSeconds: event.target.value })
          }
          required
        />
      </label>
      <label>
        Price mode
        <select
          value={values.priceMode}
          onChange={(event) =>
            setValues({
              ...values,
              priceMode: Number(event.target.value) as PriceMode,
            })
          }
        >
          <option value={PriceMode.Fixed}>Fixed</option>
          <option value={PriceMode.Variable}>Variable</option>
        </select>
      </label>
      <button type="submit" disabled={isRunning}>
        Create plan
      </button>
    </form>
  );
}

function PlanRow({
  plan,
  onToggleActive,
  onUpdatePrice,
  isBusy,
  error,
}: {
  plan: Awaited<ReturnType<typeof fetchMerchantPlans>>[number];
  onToggleActive: (isActive: boolean) => void;
  onUpdatePrice: (newAmount: bigint) => void;
  isBusy: boolean;
  error: unknown;
}) {
  const [nextPrice, setNextPrice] = useState("");
  const nowSeconds = BigInt(Math.floor(Date.now() / 1000));
  const hasPendingPrice = plan.data.amountEffectiveAt > nowSeconds;
  const currentPrice = hasPendingPrice
    ? plan.data.previousAmount
    : plan.data.amountPerPeriod;

  return (
    <div>
      <p>
        Plan #{plan.data.planId.toString()} — {currentPrice.toString()} per{" "}
        {plan.data.periodSeconds.toString()}s —{" "}
        {plan.data.isActive ? "active" : "inactive"}
      </p>
      {hasPendingPrice ? (
        <p>
          Pending price {plan.data.amountPerPeriod.toString()} effective at{" "}
          {new Date(
            Number(plan.data.amountEffectiveAt) * 1000,
          ).toLocaleString()}
        </p>
      ) : null}
      <button
        disabled={isBusy}
        onClick={() => onToggleActive(!plan.data.isActive)}
      >
        {plan.data.isActive ? "Deactivate" : "Activate"}
      </button>
      {plan.data.priceMode === PriceMode.Variable ? (
        <form
          onSubmit={(event) => {
            event.preventDefault();
            onUpdatePrice(BigInt(nextPrice));
          }}
        >
          <label>
            New price ({PRICE_CHANGE_NOTICE_SECONDS}s notice)
            <input
              value={nextPrice}
              onChange={(event) => setNextPrice(event.target.value)}
            />
          </label>
          <button type="submit" disabled={isBusy}>
            Update price
          </button>
        </form>
      ) : null}
      <ActionError error={error} />
    </div>
  );
}
