# Solana for Web2 Developers
_Superteam Brasil · TDC Floripa 2026 — reference extraction of the slide deck ([viewable version](solana-for-web2-developers.html))._

---

## Slide 1 — Title
TDC Floripa 2026
A pragmatic introduction
# solana for web2 developers.
The chain where your existing skills are 90% of the job — and how to start earning with them.

60 minutes
5 parts
1 devnet demo

---

## Slide 2 — Agenda
Agenda
### The next sixty minutes

01
Blockchain, minus the hype
what it is, and why it's worth the overhead
10 min
02
The 90/10 rule
where the work — and the money — actually is
10 min
03
Solana fundamentals
the mental models under the hood
15 min
04
The 2026 stack
x402, Pay.sh, Anchor & Pinocchio
15 min
05
Let's build
solana-ai-kit + Jupiter + Helius, live on devnet
10 min
Superteam Brasil — TDC Floripa 2026

---

## Slide 3 — §1 Blockchain
Part one
≈ 10 min
01
### Blockchain, minus the hype

A database nobody owns
Decentralized programs
Tokens & DeFi
Where it's already real

---

## Slide 4 — A database nobody owns
01 · Blockchain
### A database nobody owns

An append-only database, replicated across thousands of machines that don't trust each other. Consensus replaces the DBA.

#### Append-only

Rows are never updated in place. History is a git log you can't force-push.

#### Replicated

Every node holds the full state. There is no primary to page at 3am.

#### Verifiable

Every write is signed by its author and re-executed by everyone. Trust is checked, not assumed.

web2 brain
Kafka's append-only log + Postgres replication + git's immutable history — minus the trusted operator.

Superteam Brasil — TDC Floripa 2026

---

## Slide 5 — What the overhead buys
01 · Blockchain
### What you get for the overhead

Consensus is slower and pricier than your RDS instance. You pay that to get properties no cloud can sell you.

#### No superuser

Nobody can flip a kill switch, seize a balance, or revoke your API key. Not even the authors.

#### Neutral uptime

The network runs as long as anyone runs it. No vendor to sunset your backend.

#### Permissionless users

Any wallet on Earth is a signup. Your market is default-global from day one.

#### Free composability

Every deployed program is a public API. Integration is a function call, not a BD deal.

Superteam Brasil — TDC Floripa 2026

---

## Slide 6 — Entries with superpowers
01 · Blockchain
### Some entries have superpowers

Most rows hold data. Some hold executable code — programs.
Deploy once and your logic lives inside the database itself, replicated across every node.

#### Public by default

The bytecode sits on the ledger — anyone can read, audit, and verify exactly what runs.

#### Callable by anyone

Any wallet, any app, any other program can invoke it. A global function registry — no API gateway, no keys.

#### Runs as deployed

Thousands of machines execute the same bytes. Nobody hot-patches the logic behind your users' backs.

web2 brain
Stored procedures — if stored procedures were open-source and callable by the entire internet. That redefines what a "service" is. Side by side with your day job →

Superteam Brasil — TDC Floripa 2026

---

## Slide 7 — Code as public utility
01 · Blockchain
### Code as a public utility

A "smart contract" is just a service anyone can call, whose state anyone can read, and whose deploys are public.

Your web2 service
An onchain program
Hosting
Your AWS account
Every validator, everywhere
Auth
API keys you issue
Signatures users already hold
State
Private database
Public accounts, readable by anyone
Deploys
Can break clients quietly
Visible upgrades — authority can be burned
Billing
You invoice monthly
Caller pays per execution, settled instantly
Superteam Brasil — TDC Floripa 2026

---

## Slide 8 — Tokens & DeFi
01 · Blockchain
### Tokens and DeFi, briefly

#### Tokens

Rows in the shared ledger. An asset minted in one app works in every other app — zero integration work.

#### DeFi

Financial primitives as open services: swap, lend, borrow, stream. Composable like Unix pipes — and the pipeline is atomic.

#### Stablecoins

Digital dollars — the killer app. USDC on Solana settles in about a second for under a cent. It's also how you'll get paid.

The insight for engineers: because state is shared and public, every app extends every other app
. There are no walled-garden APIs to beg for access.

Superteam Brasil — TDC Floripa 2026

---

## Slide 9 — Where it's real
01 · Blockchain
### Where it's already real

3.77B
user transactions on Solana in June 2026 — a monthly record
165M+
x402 machine payments since 2025 — APIs paid per call, no invoices
<$0.01
typical transaction fee — micropayments finally pencil out
Building on these rails
Mastercard · Worldpay · Western Union · Google Cloud · Visa · Stripe
Tokenizing funds onchain
BlackRock · Apollo · Franklin Templeton — real funds, live on Solana
Superteam Brasil — TDC Floripa 2026

---

## Slide 10 — Uma Rede

---

## Slide 11 — Dominando os mercados

---

## Slide 12 — Números de 2025

---

## Slide 13 — 10x TPS

---

## Slide 14 — Escala comprovada

---

## Slide 15 — 9 das 22

---

## Slide 16 — §2 The 90/10 rule
Part two
≈ 10 min
02
### The 90/10 rule

Anatomy of a dApp
Your skills, mapped
Compose > write
Where the money is

---

## Slide 17 — Anatomy of a Solana app
02 · The 90/10 rule
### Anatomy of a Solana app

90%
you already know how to build this
React / Next frontend
Wallet UX & onboarding
Backend APIs & webhooks
Indexing & caching
TS client — @solana/kit
RPC, infra, monitoring
10%
the new part
Onchain program — Rust, small
…often one you didn't write: Token, Jupiter, Metaplex
The chain is your backend. The product is everything around it — built with your stack.
Superteam Brasil — TDC Floripa 2026

---

## Slide 18 — The 90%
02 · The 90/10 rule
### The 90%: skills you already have

React & TypeScript
→
Wallet flows, transaction UX, optimistic UI
REST & GraphQL APIs
→
Indexers, webhooks, trading & data services
Postgres & Redis
→
Offchain caches and read models of onchain state
DevOps & SRE
→
RPC infrastructure, monitoring, key operations
Product & design sense
→
The scarcest skill in all of crypto
Robust client code — retries, confirmation states, error recovery — is the moat.
Chains don't have loading spinners; your app does.

Superteam Brasil — TDC Floripa 2026

---

## Slide 19 — The 10%
02 · The 90/10 rule
### The 10%: the onchain program

A program is a small state machine: validate accounts, mutate bytes, emit events. Hundreds of lines, not thousands.
It changes rarely and gets audited hard — closer to a database schema than to app code.

One deployment serves every user. State lives in their
accounts, not on your servers.

Rust is the price of admission here — but AI assistants have collapsed that curve. More in part five.

the app
everything your users touch — your stack
your_program.so
~180 KB of Rust — the schema of your product
Superteam Brasil — TDC Floripa 2026

---

## Slide 20 — Composing beats writing
02 · The 90/10 rule
### Composing beats writing

You don't write Postgres to use a database. Most Solana apps ship with zero custom programs
.

Payments app
USDC transfers via the Token program
Trading UI
Jupiter swap API + your UX ideas
NFT storefront
Metaplex mints, listings, royalties
Portfolio dashboard
RPC reads + indexer webhooks
Creator tipping
Pay links + transfer instructions
Paid API
x402 middleware on any endpoint
custom Rust: 0 lines
— all six of these. Write a program when your idea needs new rules, not new screens.
Superteam Brasil — TDC Floripa 2026

---

## Slide 21 — Where the money is
02 · The 90/10 rule
### Where the money is

#### Bounties & grants

Scoped tasks on Superteam Earn, paid in USDC — typically US$500–10k. Frontend listings outnumber Rust ones.

#### Hackathons

Colosseum runs Solana's global hackathons — prize money, and winning teams get accelerator funding.

#### Freelance & agency

Protocols ship fast and permanently need client devs who understand wallets. Rates carry a web3 premium.

#### Full-time

Solana teams hire remote and global. Client-side roles outnumber program roles — exactly your profile.

The asymmetry: demand is crypto-native, supply of solid web2 engineers is not.
You are the scarce resource here.

Superteam Brasil — TDC Floripa 2026

---

## Slide 22 — Founder track
02 · The 90/10 rule
### Or found the thing yourself

Building is a track of its own: go the venture route — or raise from your future users, in public, on-chain.

#### Venture track

Win a Colosseum hackathon → accelerator, pre-seed capital, and a cohort that ships. The classic path, compressed.

#### Public raise — MetaDAO

Commit-based ICOs: USDC pledged in public, auto-refunded if the minimum fails, treasuries governed by decision markets.

$32.7M
Credible Finance
committed vs a $4M cap — stablecoin credit rails, 8x oversubscribed
$20.9M
Rip Cars
vs a $250K minimum — collectible-car gacha, ~84x oversubscribed
$6.7M
p2p.me
Pix & UPI ↔ USDC ramp — backed by Multicoin and Coinbase Ventures
$RAWR
Jurassic Finance
fractional museum-grade dinosaur fossils. Yes, really.
The point isn't that everything moons — it's that capital formation is permissionless now too.
Ship something people want; the raise is a public market, not a pitch meeting.

Superteam Brasil — TDC Floripa 2026

---

## Slide 23 — Superteam Earn
02 · The 90/10 rule
### Superteam Earn, in practice

1
Browse
earn.superteam.fun — open bounties with scope and reward up front
2
Ship
Submit work. No interviews, no résumés — output only
3
Win
Sponsors pick winners in public. Losing still builds a visible track record
4
Get paid
USDC straight to your wallet — the fundamentals you'll learn in part three
Superteam is a global network of regional communities. Brasil is yours
— Portuguese-speaking, IRL events, and reputation compounds: winners get DMs.

Scan for live bounties
earn.superteam.fun
Superteam Brasil — TDC Floripa 2026

---

## Slide 24 — §3 Fundamentals
Part three
≈ 15 min
03
### Solana fundamentals

Accounts
Programs
Transactions
PDAs
CPI
Tokens
Wallets
Devnet

---

## Slide 25 — Why Solana
03 · Fundamentals
### Why Solana specifically

400ms
block time — interacting with the chain feels like calling a normal API
<$0.01
per transaction — UX patterns like "sign every action" actually work
1,000
epochs crossed in July 2026 — six years of the counter never resetting
1 state
no L2s, no bridges — your users are never "on the wrong network"
Superteam Brasil — TDC Floripa 2026

---

## Slide 26 — Everything is an account
03 · Fundamentals
### Everything is an account

Chain state is one giant key-value store. Keys are addresses; values are accounts
— some lamports, some bytes, and an owner.

address (key)
owner
what it is
7xKX…gAsU
System Program
your wallet — holds SOL
BQcd…Mn2e
Token Program
your USDC balance
Chat…9dPk
your program
a user's profile record
web2 brain
A filesystem: accounts are files, programs are processes, and only the owning process may write its files. Programs are accounts too — code lives in the same store.

Superteam Brasil — TDC Floripa 2026

---

## Slide 27 — Programs are stateless
03 · Fundamentals
### Programs are stateless

A program keeps nothing between calls. Every bit of state arrives in the accounts passed with the transaction.

Ethereum model
Contract = code + storage fused together. An object. Each token deploys its own copy.

Solana model
Code and data separated. A function over accounts. One Token program runs every
token on the chain.

Think stateless microservice over Redis: scale by adding keys, not instances.

Statelessness is what makes the next slide — parallel execution — possible.

Superteam Brasil — TDC Floripa 2026

---

## Slide 28 — Transactions
03 · Fundamentals
### Transactions declare everything

Every transaction lists each account it will read or write — before it runs
. So the runtime schedules like a database, not a queue.

runs in parallel
Tx A
writes alice.usdc

Tx B
writes bob.usdc

queued behind A
Tx C
also writes alice.usdc — conflicts, so it waits

Row-level locking, chain-wide: non-overlapping transactions execute simultaneously.

Atomic batches: a transaction holds many instructions — create account, swap, transfer — all succeed or all revert.

You compose those batches client-side, in TypeScript
— 90% territory again.

Superteam Brasil — TDC Floripa 2026

---

## Slide 29 — PDAs
03 · Fundamentals
### PDAs: addresses you compute

A Program Derived Address comes from hashing seeds + program id. No private key exists
— the owning program signs for it.

```
const [profilePda] = getProgramDerivedAddress({
 seeds: ["profile"
, user.address], // derive, don't look up
programAddress: MY_PROGRAM,
});
```
Namespaced keys
Like user:42:profile
in Redis — deterministic, no lookup table

Per-user records
Profiles, positions, game state — one PDA per (user, thing)

Escrows & vaults
Only code can sign → a PDA can hold funds nobody can run off with

Superteam Brasil — TDC Floripa 2026

---

## Slide 30 — CPI
03 · Fundamentals
### CPI: programs calling programs

Cross-Program Invocation — synchronous calls between programs, inside one atomic transaction.

Your program
"swap then deposit"

→
Jupiter
routes the swap

→
Token program
moves the funds

one atomic transaction — partial failure cannot exist
Like synchronous service-to-service calls — but the distributed-transaction problem is solved by the runtime.

Your PDA signs the inner call — that's how an escrow pays out without any human key.

Superteam Brasil — TDC Floripa 2026

---

## Slide 31 — Tokens solved
03 · Fundamentals
### Tokens are a solved problem

You don't deploy a token contract. You create a mint
inside the shared, audited Token program.

Mint account
the currency itself — supply, decimals, authorities
Token account
one holder's balance of one mint — a row per (user, currency)
Token-2022 extensions
features as flags: transfer fees, transfer hooks, confidential amounts, allowlists
$ spl-token create-token
— one command. Extensions are how enterprises do compliant money on public rails.
Superteam Brasil — TDC Floripa 2026

---

## Slide 32 — Wallets
03 · Fundamentals
### Wallets are your auth layer

A wallet is a keypair with UX. One signature is authentication, authorization, and payment
in a single primitive.

#### Login

"Sign this message" replaces the password database you never wanted to own.

#### Consent

Every state change is user-approved — OAuth scopes, but per action and legible.

#### Embedded

Passkey and email wallets hide seed phrases entirely — users never know they're on a chain.

Wallet UX is where apps win or die — and it's frontend work.
The 90% keeps showing up.

Superteam Brasil — TDC Floripa 2026

---

## Slide 33 — Devnet
03 · Fundamentals
### Devnet: staging with free money

localnet
Your machine. Instant, offline, cheatcodes — surfpool & LiteSVM even fork mainnet state.

devnet
Public staging. Free SOL from the faucet, real RPCs, real explorers. Today's playground.
mainnet-beta
Production. Real money, identical code — promotion is a config change, not a rewrite.

today
Everything we build runs on devnet. Worst case: we lose fake money.

Superteam Brasil — TDC Floripa 2026

---

## Slide 34 — §4 The 2026 stack
Part four
≈ 15 min
04
### The 2026 stack

Client SDKs
x402
Pay.sh
Anchor & Pinocchio

---

## Slide 35 — Client stack
04 · The 2026 stack
### The client stack

@solana/kit
The modern TypeScript SDK — tree-shakeable, zero-dependency. Replaces web3.js.
wallet adapter
One integration, every wallet — browser extension, mobile, embedded.
codama
Program IDL in, fully-typed TS client out. Generated, not handwritten.
RPC + indexers — Helius
Managed RPC, webhooks, and the DAS API — one indexed read for any wallet, token, or NFT.
TypeScript all the way down. This is the 90% — and it's hiring.
Superteam Brasil — TDC Floripa 2026

---

## Slide 36 — Client recipes
04 · The 2026 stack
### Client-side, concretely

A trading bot
→
@solana/kit + Jupiter Swap API + Helius streams. No frontend — a Node process and a keypair.
A consumer app
→
Next.js + wallet adapter + embedded wallets; Helius DAS for every balance, token, and NFT read.
A data product
→
Helius webhooks feeding Postgres read models behind your own API. Sell the insight, not the node.
An agent that pays its way
→
Your LLM + an x402 client + a Pay.sh wallet — it buys the data it needs, per call. More in a second.
Four shippable products, zero custom programs. "Client-side" is a full career here
— not a consolation prize.

Superteam Brasil — TDC Floripa 2026

---

## Slide 37 — x402
04 · The 2026 stack
### x402: HTTP 402, finally

"402 Payment Required" was reserved in the 90s and sat unused for ~30 years. Now it does what it says.

GET /api/report
→
402 + quote
→
sign USDC payment
→
retry + X-PAYMENT
→
200 OK
The payment is the credential.
No API keys, no accounts, no subscriptions — built for agents and humans alike.

165M+ payments and ~$50M volume by April 2026 — Solana is the #1 chain by x402 dollar volume.
An open standard since July 2026, governed by the x402 Foundation under the Linux Foundation.

Visa, Mastercard, Stripe, AWS and Solana sit among its 40 founding members. This is not a crypto side quest.

Superteam Brasil — TDC Floripa 2026

---

## Slide 38 — Pay.sh
04 · The 2026 stack
### Pay.sh: a wallet for your agent

Solana Foundation × Google Cloud, May 2026: agents discover, access, and pay for APIs per request — in USDC on Solana.

75+ APIs at launch
Gemini, BigQuery, Vertex AI, Cloud Run — plus 50+ community providers
Wallet = identity
No Google account, no credential rotation — the x402 payment authorizes the call
Funded in ~60 seconds
Card or stablecoin in; from then on the agent is economically autonomous
Runs in your tools
CLI plugs into Claude Code, Gemini, Codex — a few lines to connect an agent
flip side
List your own API on these rails and every agent on Earth is a potential customer.

Superteam Brasil — TDC Floripa 2026

---

## Slide 39 — Anchor
04 · The 2026 stack
### Anchor: the default framework

Macros generate the checks you'd forget: ownership, signers, seeds, rent.

IDL for free — Codama turns it into the typed TS client your frontend consumes.

The ecosystem default: docs, examples, auditors, and AI assistants all speak it.

Declarative constraints, imperative logic — like an ORM for chain state.

```
#[program]
pub mod counter {
 pub fn increment(ctx: Context<Inc>) -> Result<()> {
 ctx.accounts.counter.count += 1;
 Ok(())
 }
}

#[derive(Accounts)]
pub struct Inc<'info> {
 #[account(mut, seeds = [b"counter"], bump)]
pub counter: Account<'info, Counter>,
} // the macro writes the checks

```
Superteam Brasil — TDC Floripa 2026

---

## Slide 40 — Pinocchio
04 · The 2026 stack
### Pinocchio: close to the metal

Zero-dependency, zero-copy library from Anza — reads the transaction payload in place, no deserialization.

Every check is yours: owner, signer, sizes. Total control, total responsibility.

Built for hot paths: token programs, AMMs, anything compute-bound. Compute units are real money at scale.

same instruction, optimized builds
Anchor
281 CU
Pinocchio
108 CU

```
entrypoint!(process);

fn process(id: &Pubkey, accounts: &[AccountInfo],
 data: &[u8]) -> ProgramResult {
 let [counter] = accounts else {
 return Err(NotEnoughAccountKeys.into());
 };
 // every check is yours: owner? signer? size?
let mut bytes = counter.try_borrow_mut_data()?;
 bytes[0] = bytes[0].wrapping_add(1);
 Ok(())
}
```
Superteam Brasil — TDC Floripa 2026

---

## Slide 41 — Choosing a framework
04 · The 2026 stack
### Choosing your framework

Anchor v1
Learn here. Ship here.
Safety rails, ecosystem gravity, AI assistants trained on a decade of examples.

Pinocchio
Optimize here.
When compute is the product: exchanges, token infra, high-frequency paths.

Anchor v2
Watch here.
Ground-up redesign in progress by the maintainers — born in the Kit/Codama era, aiming near-native performance.

The pragmatic path: start with Anchor, read Pinocchio to understand what the macros hide, track v2.
Superteam Brasil — TDC Floripa 2026

---

## Slide 42 — §5 Let's build
Part five
≈ 10 min + hands-on
05
### Let's build

solana-ai-kit
Jupiter + Helius DAS
a DEX, em português

---

## Slide 43 — solana-ai-kit
05 · Let's build
### solana-ai-kit: a Solana expert in your terminal

Superteam Brasil's open-source AI config kit for Claude Code & Codex — so your assistant writes Solana like it means it.

15
agents — solana-architect, anchor-engineer, defi-engineer, qa…
30
commands — deploy, audit, scaffold flows
7
MCP servers — docs, explorers, RPCs wired in
+
rules & skills — Rust, Anchor, Pinocchio, TS style law, security gates
inside Claude Code
/plugin marketplace add solanabr/solana-ai-kit
/plugin install solana-ai-kit@stbr
github.com/solanabr/solana-ai-kit
Superteam Brasil — TDC Floripa 2026

---

## Slide 44 — Today's build: a DEX
05 · Let's build
### Today's build: a DEX on devnet

A full-stack decentralized exchange — Jupiter
routes the swaps, Helius DAS
feeds the data, e a interface é em português
.

1
Scaffold
create-solana-dapp — Next.js + wallet adapter, pointed at devnet
2
Data
Helius DAS API — token lists, balances, metadata and prices for the pairs
3
Swap
Jupiter quote + swap APIs build the transaction; the user's wallet signs
4
Interface
pares, cotações, slippage, confirmações — trading UX caprichada, em português
5
Ship
deploy, trade with devnet wallets, watch every fill on the explorer
Zero lines of Rust — Jupiter and the Token program are our 10%. Claude Code + solana-ai-kit write most of the rest; we review. Laptops out.
Superteam Brasil — TDC Floripa 2026

---

## Slide 45 — Ask the kit
05 · Let's build
### Your first week? Ask the kit

No generic syllabus. solana-ai-kit keeps memory
— tell it what you know and it plans around your stack, your pace, your quirks.

> tenho 8 anos de React, zero Rust, 5h por semana. monta meu plano de estudos
✔ plan drafted — client-first, wallet UX on day 1, Anchor on day 5
✔ saved to memory — the plan adapts as you ship
It remembers
— your background, what clicked, what didn't. Every session resumes where your brain left off.

It calibrates
— React devs start at wallet UX, backend folks at indexers, the Rust-curious go straight to programs.

It answers to you
— ask for code reviews, quizzes, or a re-plan when the week explodes.

Superteam Brasil — TDC Floripa 2026

---

## Slide 46 — Superteam Academy
05 · Let's build
### Keep leveling: Superteam Academy

Gamified tracks written by ecosystem experts — from first wallet to first protocol.

Missions pay in on-chain tokens
as you complete them — learning that literally earns.

Certificates mint to your wallet
— verifiable proof of skill, composable like everything else here.

Scan to enroll

---

## Slide 47 — Close
TDC Floripa 2026
### bora construir.
Your stack already works here. Bring it — the 10% is the easy part to learn, and the 90% is already yours.

earn.superteam.fun
github.com/solanabr/solana-ai-kit
solana.com/developers
Superteam Brasil — come say hi
Made with Claude Design
