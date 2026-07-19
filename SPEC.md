# dig-tips — normative specification

> The authoritative contract an independent reimplementation of dig-tips could be built against.
> Layering: this `SPEC.md` is the repo's own contract; `dig-cat`'s `SPEC.md` is the underlying CAT
> contract dig-tips delegates to; the ecosystem `SYSTEM.md` maps the cross-repo interactions. They
> must agree.

## 1. Scope

`dig-tips` is the DIG Network canonical **tipping** expert crate: a pure, key-free, network-free
builder that constructs the exact Chia `CoinSpend`s to tip a CAT (including `$DIG`) to a recipient,
and defines the honest auto-tip policy logic (`dig_ecosystem#377`).

A **tip is a single-payment CAT send**: dig-tips does not re-derive any CAT spend bytes — it wraps
[`dig-cat`](https://crates.io/crates/dig-cat)'s `build_cat_spend` with exactly one payment and adds
only the tipping semantics (the recipient, the honesty policy, the pure cap decisions). dig-tips has
**no read path** and therefore cannot gate or slow content consumption.

## 2. Custody invariants (HARD)

- dig-tips **never** holds a secret key, **never** signs, **never** performs I/O or network access.
- Builders take only **public** inputs and append **unsigned** `CoinSpend`s to a caller-owned
  `SpendContext`.
- A `required_signatures`-style function reports the exact signatures the caller must produce; the
  caller signs, assembles the `SpendBundle`, and broadcasts.
- dig-tips **never** signs an unbound / caller-controllable message (the custody-oracle rule).

## 3. Honesty invariants (§6.0 / dig_ecosystem#207)

- The auto-tip is **default-on but transparent**, **capped**, **one-click-off**, and
  **dismissible**.
- Tipping **never** gates or slows a content **read**.
- The default auto-tip recipient is the canonical `dig-constants::DIG_TREASURY_INNER_PUZZLE_HASH`
  (reused, never hand-copied).
- The tip coin memo is **minimal** on-chain (NC-8).

## 4. Public surface

### 4.1 Treasury recipient

- `default_recipient() -> Bytes32` — the default auto-tip recipient. MUST equal
  `dig_constants::DIG_TREASURY_INNER_PUZZLE_HASH` byte-for-byte (reused, never copied).
- `TREASURY_ADDRESS: &str` — the bech32m `xch1…` form, re-exported from
  `dig_constants::DIG_TREASURY_ADDRESS`.

### 4.2 Policy

- `enum TipMode { Auto, Manual }` — whether tips fire automatically or require explicit approval.
- `struct AutoTipPolicy { enabled: bool, mode: TipMode, asset_id: Bytes32, recipient: Bytes32,
  tip_amount: u64, threshold: u64, max_tips_per_day: u32, max_amount_per_day: u64 }` — the honest,
  capped configuration. Every field is public (a settings UI renders + persists it directly).
- `AutoTipPolicy::dig_default(asset_id) -> Self` — enabled, `Auto`, `recipient == default_recipient()`,
  with the default caps (`DEFAULT_TIP_AMOUNT` = 1000, `DEFAULT_THRESHOLD` = 0,
  `DEFAULT_MAX_TIPS_PER_DAY` = 50, `DEFAULT_MAX_AMOUNT_PER_DAY` = 50000).
- `AutoTipPolicy::disabled(asset_id) -> Self` — `enabled = false` (the one-click-off), retaining the
  canonical recipient + caps.

### 4.3 Decisions (all PURE — no clock, no state, no I/O)

- `struct LedgerSnapshot { tips_today: u32, amount_today: u64 }` — today's caller-owned counters.
- `enum CapReason { FrequencyCap, AmountCap }`.
- `enum TipDecision { Tip { amount }, SkipDisabled, SkipBelowThreshold, SkipManualNotApproved,
  SkipCapReached { reason } }`.
- `decide_auto_tip(policy, primary_send_amount, ledger) -> TipDecision` — checks, in order: enabled →
  `Auto` mode → `primary_send_amount >= threshold` → caps. Tips `policy.tip_amount`.
- `decide_daily_tip(policy, ledger) -> TipDecision` — as above minus the threshold check.
- `decide_manual_tip(policy, requested_amount, approved, ledger) -> TipDecision` — checks enabled →
  `approved` → `requested_amount != 0` → caps. Independent of `mode` (an explicit user action). Tips
  `requested_amount`.
- `apply_tip(ledger, amount) -> LedgerSnapshot` — advances both counters with `saturating_add`.
- `is_new_utc_day(last_tip_unix, now_unix) -> bool` — pure day-rollover predicate for cap resets
  (`div_euclid` by 86400; correct for negative timestamps).

**Cap enforcement (custody-critical, MUST hold):** both cap checks use `checked_add` — the frequency
check on `tips_today + 1`, the amount check on `amount_today + amount`. An arithmetic overflow is
treated as **cap reached**, never as a wrap that slips past the ceiling. A `u64::MAX` running amount
therefore yields `SkipCapReached { AmountCap }`, never a `Tip`.

### 4.4 Events

- `enum TipEvent { TipPlanned { amount, recipient }, TipDeferredCap { reason }, TipDeclined,
  TipSkippedDisabled, TipSkippedBelowThreshold }`.
- `TipEvent::from_decision(&TipDecision, recipient) -> Option<TipEvent>` — maps `Tip → TipPlanned`,
  `SkipCapReached → TipDeferredCap`, `SkipManualNotApproved → TipDeclined`,
  `SkipDisabled → TipSkippedDisabled`, `SkipBelowThreshold → TipSkippedBelowThreshold`. Every decision
  maps to `Some` (the `Option` reserves room for a future silent decision).

### 4.5 Build

- `struct TipRequest { cats: Vec<Cat>, owner_pk: PublicKey, asset_id: Bytes32, recipient: Bytes32,
  amount: u64, change_p2_puzzle_hash: Bytes32 }`.
- `build_tip(req) -> Result<UnsignedCatSpend>` — wraps `dig_cat::build_cat_spend` with one
  `CatPayment::new(recipient, amount)` (minimal memo). Value is conserved (`plan.delta == 0`). It
  NEVER returns a degenerate empty spend — a zero amount or a shortfall is an `Err`.
- `build_tip_if_allowed(policy, primary_send_amount, ledger, cats, owner_pk, change_p2_puzzle_hash)
  -> Result<(TipDecision, Option<UnsignedCatSpend>)>` — the canonical GUARDED path: it decides first
  and builds the spend ONLY on `TipDecision::Tip`; every skip returns `(decision, None)` with no spend
  constructed, so a capped/declined tip can never be built.

### 4.6 Signing boundary (re-export)

- `required_signatures(&[CoinSpend], &Network) -> Result<Vec<RequiredSignature>>` and `enum Network`
  are re-exported verbatim from `dig-cat`. dig-tips adds no signing logic. The caller signs the
  reported messages, aggregates, and broadcasts.

### 4.7 Curated re-exports

`Bytes32`, `Coin`, `Cat`, `CatInfo`, `CoinSpend`, `PublicKey`, `UnsignedCatSpend` (via `dig-cat`), plus
`VERSION` / `version()`.

## 5. Conformance

- **Treasury pin (KAT).** `default_recipient() == dig_constants::DIG_TREASURY_INNER_PUZZLE_HASH` and
  `TREASURY_ADDRESS == dig_constants::DIG_TREASURY_ADDRESS`.
- **Cap overflow (KAT).** With `amount_today = u64::MAX`, `decide_auto_tip` returns
  `SkipCapReached { AmountCap }` (no panic, no wrap); with `tips_today = u32::MAX`, it returns
  `SkipCapReached { FrequencyCap }`.
- **On-chain round-trip.** A tip built by `build_tip`, signed for the reported signatures, validates on
  the Chia simulator (TESTNET11); value routes exactly (`amount` to the recipient, remainder to
  change); the owner key is a required signer.
- **Minimal memo (NC-8).** The recipient coin's `CREATE_COIN` memo list is exactly `[recipient]` — the
  recipient hint only, no identity, no extra memos.
- **Golden vector.** For a fixed owner seed + fixed input, the recipient CAT coin id is stable
  (`fixed_input_tip_is_a_stable_golden`), pinning the built spend against drift.

## 6. Error taxonomy

- `enum Error { Cat(dig_cat::CatError) }` — the sole fallible surface is the on-chain spend
  construction, surfaced verbatim from `dig-cat` (`ZeroAmount` for a zero tip, `InsufficientFunds` /
  `TooManyInputs` from coin selection). A policy skip is a `TipDecision`, NOT an error. dig-tips never
  collapses a build failure into a silent empty spend.
