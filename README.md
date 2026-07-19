# dig-tips

The **DIG Network canonical tipping expert crate** — a pure, key-free, network-free
`SpendBundle`-builder for **tipping** on Chia.

`dig-tips` constructs the exact `CoinSpend`s to tip a CAT (including **$DIG**) to a recipient
address, and carries the honest, default-on **auto-tip** to the DIG treasury
([`dig_ecosystem#377`](https://github.com/DIG-Network/dig_ecosystem/issues/377)) as pure, capped,
declinable policy logic.

## Custody model (HARD invariants)

dig-tips **never holds a secret key, never signs, and never touches the network.** Every builder
takes only public inputs and appends unsigned coin spends to a caller-owned `SpendContext`; a
`required_signatures`-style function reports the exact signatures the caller must produce. The
consumer signs, assembles the `SpendBundle`, and broadcasts.

## The $DIG North Star

Tipping moves `$DIG` at every honest opportunity while keeping consumption frictionless: the
auto-tip is transparent, capped, one-click-off, and dismissible, and it **never** gates or slows a
content read.

## Installation

```toml
[dependencies]
dig-tips = "0.1"
```

## Usage

A tip is a single-payment CAT send. The canonical path decides (caps / threshold / approval) and only
then builds the spend, so a capped or declined tip is never constructed:

```rust
use dig_tips::{build_tip_if_allowed, AutoTipPolicy, LedgerSnapshot, Network, required_signatures};

let policy = AutoTipPolicy::dig_default(dig_asset_id); // default-on, tips the DIG treasury, capped
let ledger = LedgerSnapshot::default();               // today's counters (caller-owned)

let (decision, maybe_spend) =
    build_tip_if_allowed(&policy, primary_send_amount, &ledger, my_cats, owner_pk, change_hash)?;

if let Some(spend) = maybe_spend {
    let sigs = required_signatures(&spend.coin_spends, &Network::Mainnet)?;
    // caller signs `sigs`, assembles the SpendBundle, and broadcasts — dig-tips never signs.
}
```

For the pure policy/decision layer use `decide_auto_tip` / `decide_daily_tip` / `decide_manual_tip`
(+ `apply_tip` / `is_new_utc_day` for the caller's ledger), and `TipEvent::from_decision` to surface
the honest, user-facing record. The low-level `build_tip` builds an unconditional tip spend.

## Status

v0.1.0 foundation ([`dig_ecosystem#1231`](https://github.com/DIG-Network/dig_ecosystem/issues/1231)).
See `SPEC.md` for the normative contract.

## License

Licensed under either of Apache-2.0 or MIT at your option.
