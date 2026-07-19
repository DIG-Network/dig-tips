//! # dig-tips — the DIG Network canonical tipping expert crate (genesis scaffold)
//!
//! `dig-tips` is a **pure, key-free, network-free** SpendBundle-builder for TIPPING on Chia. It
//! constructs the exact [`CoinSpend`](chia_protocol::CoinSpend)s to tip a CAT (including `$DIG`) to
//! a recipient address, and carries the honest, default-on **auto-tip** to the DIG treasury
//! (`dig_ecosystem#377`) as pure, capped, declinable policy logic.
//!
//! ## The custody model (HARD invariants)
//!
//! dig-tips **never holds a secret key, never signs, and never touches the network.** Every builder
//! takes only public inputs and appends unsigned coin spends to a caller-owned `SpendContext`; a
//! `required_signatures`-style function reports the exact signatures the caller must produce. The
//! consumer signs the reported messages, assembles the `SpendBundle`, and broadcasts.
//!
//! ## The $DIG North Star (CLAUDE.md §6.0)
//!
//! Tipping moves `$DIG` at every honest opportunity while keeping consumption frictionless: the
//! auto-tip is transparent, capped, one-click-off, and dismissible, and it NEVER gates or slows a
//! content read.
//!
//! This is the genesis scaffold (v0.0.0). The real surface lands in v0.1.0 — see `SPEC.md` and
//! `DIG-Network/dig_ecosystem#1231`.

#![forbid(unsafe_code)]

/// The crate's semantic version, surfaced so a consumer can record which builder version produced a
/// tip.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Genesis placeholder — replaced by the real tipping builders in v0.1.0. Present only so the
/// scaffold compiles and the CI gate set (fmt / clippy / build / docs / coverage) is green from the
/// first commit.
#[doc(hidden)]
pub fn scaffold_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_is_ready() {
        assert!(scaffold_ready());
    }

    #[test]
    fn version_is_exposed() {
        assert!(!VERSION.is_empty());
    }
}
