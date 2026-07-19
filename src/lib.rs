//! # dig-tips — the DIG Network canonical tipping expert crate
//!
//! `dig-tips` is a **pure, key-free, network-free** SpendBundle-builder for TIPPING on Chia. It
//! constructs the exact [`CoinSpend`]s to tip a CAT (including `$DIG`) to a
//! recipient, and carries the honest, default-on **auto-tip** to the DIG treasury
//! (`dig_ecosystem#377`) as pure, capped, declinable policy logic. A tip IS a single CAT payment, so
//! the on-chain spend is delegated to [`dig_cat`] (the CAT byte-source-of-truth); dig-tips adds only
//! the tipping semantics — the recipient, the honesty policy, and the pure cap decisions.
//!
//! ## The custody model (HARD invariants)
//!
//! dig-tips **never holds a secret key, never signs, and never touches the network.** Every builder
//! takes only public inputs and appends unsigned coin spends; [`required_signatures`] reports the
//! exact signatures the caller must produce. The consumer signs, assembles the `SpendBundle`, and
//! broadcasts. dig-tips never signs an unbound / caller-controllable message.
//!
//! ## The $DIG North Star + honesty (CLAUDE.md §6.0 / dig_ecosystem#207)
//!
//! Tipping moves `$DIG` at every honest opportunity while keeping consumption frictionless. The
//! auto-tip is **default-on but transparent, capped, one-click-off, and dismissible**, and dig-tips
//! has **no read path** — it can never gate or slow a content read. The daily caps
//! ([`AutoTipPolicy::max_tips_per_day`] / [`AutoTipPolicy::max_amount_per_day`]) are enforced with
//! overflow-safe arithmetic so they cannot be bypassed, and the canonical guarded path
//! [`build_tip_if_allowed`] decides before it builds so a capped/declined tip is never constructed.
//!
//! ## Usage
//!
//! ```no_run
//! use dig_tips::{build_tip_if_allowed, AutoTipPolicy, LedgerSnapshot, Network, required_signatures};
//! # use dig_tips::{Bytes32, Cat, PublicKey};
//! # fn demo(dig_asset_id: Bytes32, owner_pk: PublicKey, change: Bytes32, my_cats: Vec<Cat>) {
//! let policy = AutoTipPolicy::dig_default(dig_asset_id); // default-on, tips the DIG treasury
//! let ledger = LedgerSnapshot::default();
//! let (decision, maybe_spend) =
//!     build_tip_if_allowed(&policy, 100_000, &ledger, my_cats, owner_pk, change).unwrap();
//! if let Some(spend) = maybe_spend {
//!     let sigs = required_signatures(&spend.coin_spends, &Network::Mainnet).unwrap();
//!     // caller signs `sigs`, assembles the SpendBundle, broadcasts.
//!     let _ = (decision, sigs);
//! }
//! # }
//! ```
//!
//! See `SPEC.md` for the normative contract.

#![forbid(unsafe_code)]

mod build;
mod decision;
mod error;
mod event;
mod policy;
mod sign;
mod treasury;

pub use build::{build_tip, build_tip_if_allowed, TipRequest};
pub use decision::{
    apply_tip, decide_auto_tip, decide_daily_tip, decide_manual_tip, is_new_utc_day, CapReason,
    LedgerSnapshot, TipDecision,
};
pub use error::{Error, Result};
pub use event::TipEvent;
pub use policy::{
    AutoTipPolicy, TipMode, DEFAULT_MAX_AMOUNT_PER_DAY, DEFAULT_MAX_TIPS_PER_DAY,
    DEFAULT_THRESHOLD, DEFAULT_TIP_AMOUNT,
};
pub use sign::{required_signatures, Network};
pub use treasury::{default_recipient, TREASURY_ADDRESS};

// Curated re-exports of the underlying chia types the API speaks in, via dig-cat, so a consumer need
// not depend on the SDK / chia-protocol directly for the common surface.
pub use dig_cat::{Bytes32, Cat, CatInfo, Coin, CoinSpend, PublicKey, UnsignedCatSpend};

/// The crate's semantic version, surfaced so a consumer can record which builder version produced a
/// tip.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The crate's semantic version (function form of [`VERSION`]).
#[must_use]
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_reported() {
        assert!(!version().is_empty());
        assert_eq!(version(), VERSION);
    }
}
