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

## Status

Genesis scaffold (v0.0.0). The v0.1.0 foundation lands via
[`dig_ecosystem#1231`](https://github.com/DIG-Network/dig_ecosystem/issues/1231). See `SPEC.md` for
the normative contract.

## License

Licensed under either of Apache-2.0 or MIT at your option.
