# dig-tips — normative specification

> Genesis scaffold. This SPEC is completed in the v0.1.0 foundation PR
> (`DIG-Network/dig_ecosystem#1231`). It is the authoritative contract an independent
> reimplementation of dig-tips could be built against.

## 1. Scope

`dig-tips` is the DIG Network canonical **tipping** expert crate: a pure, key-free, network-free
builder that constructs the exact Chia `CoinSpend`s to tip a CAT (including `$DIG`) to a recipient,
and defines the honest auto-tip policy logic (`dig_ecosystem#377`).

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

_Defined in the v0.1.0 foundation PR._

## 5. Conformance

_Golden vectors + KATs defined in the v0.1.0 foundation PR._

## 6. Error taxonomy

_Defined in the v0.1.0 foundation PR._
