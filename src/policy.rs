//! The auto-tip policy — the honest, capped, declinable configuration (§6.0 / dig_ecosystem#377).
//!
//! An [`AutoTipPolicy`] is pure configuration: it says WHETHER to tip, HOW MUCH, and the hard daily
//! CAPS that bound the flow. It holds no key and does no I/O — the [`crate::decision`] functions
//! turn a policy plus a ledger snapshot into a pure [`crate::TipDecision`]. The caps exist so the
//! default-on auto-tip stays honest: a user (or a compromised loop) can never move more than the
//! configured ceiling per day, and the auto-tip is one flag away from off ([`AutoTipPolicy::disabled`]).

use chia_protocol::Bytes32;

use crate::treasury::default_recipient;

/// The default per-tip amount, in the CAT's base units.
///
/// $DIG is a 3-decimal CAT (1 $DIG = 1000 base units), so this is a 1 $DIG tip — a small, honest
/// contribution that moves $DIG without gating anything.
pub const DEFAULT_TIP_AMOUNT: u64 = 1_000;

/// The default minimum primary-send amount that triggers an auto-tip (base units).
///
/// `0` means "tip alongside any qualifying send" — the North-Star default of moving $DIG at every
/// honest opportunity. A consumer may raise it to only tip on larger sends.
pub const DEFAULT_THRESHOLD: u64 = 0;

/// The default cap on the NUMBER of auto-tips per UTC day.
pub const DEFAULT_MAX_TIPS_PER_DAY: u32 = 50;

/// The default cap on the total auto-tipped AMOUNT per UTC day (base units).
pub const DEFAULT_MAX_AMOUNT_PER_DAY: u64 = 50_000;

/// How the auto-tip fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipMode {
    /// Tip automatically when a qualifying event occurs (still capped + declinable).
    Auto,
    /// Never tip without an explicit per-tip approval from the user.
    Manual,
}

/// The honest, capped auto-tip configuration.
///
/// Every field is public so a consumer's settings UI can render + persist it directly (the
/// one-click-off is `enabled = false`; the caps are user-visible ceilings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoTipPolicy {
    /// The master switch. `false` is the one-click-off — no tip is ever decided while disabled.
    pub enabled: bool,
    /// Whether tips fire automatically ([`TipMode::Auto`]) or require explicit approval
    /// ([`TipMode::Manual`]).
    pub mode: TipMode,
    /// The CAT asset id being tipped (e.g. the $DIG asset id).
    pub asset_id: Bytes32,
    /// The tip recipient's inner (p2) puzzle hash (defaults to the DIG treasury).
    pub recipient: Bytes32,
    /// The amount each auto/daily tip sends, in base units.
    pub tip_amount: u64,
    /// The minimum primary-send amount that triggers an auto-tip (base units).
    pub threshold: u64,
    /// The hard cap on the number of tips per UTC day.
    pub max_tips_per_day: u32,
    /// The hard cap on the total tipped amount per UTC day (base units).
    pub max_amount_per_day: u64,
}

impl AutoTipPolicy {
    /// The default-on auto-tip for `asset_id`: enabled, [`TipMode::Auto`], to the canonical DIG
    /// treasury, with the sensible honest caps above.
    #[must_use]
    pub fn dig_default(asset_id: Bytes32) -> Self {
        Self {
            enabled: true,
            mode: TipMode::Auto,
            asset_id,
            recipient: default_recipient(),
            tip_amount: DEFAULT_TIP_AMOUNT,
            threshold: DEFAULT_THRESHOLD,
            max_tips_per_day: DEFAULT_MAX_TIPS_PER_DAY,
            max_amount_per_day: DEFAULT_MAX_AMOUNT_PER_DAY,
        }
    }

    /// The auto-tip switched OFF for `asset_id` — the one-click-off state (recipient + caps still
    /// carry the canonical defaults so re-enabling restores the honest configuration).
    #[must_use]
    pub fn disabled(asset_id: Bytes32) -> Self {
        Self {
            enabled: false,
            ..Self::dig_default(asset_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASSET: Bytes32 = Bytes32::new([0x11u8; 32]);

    #[test]
    fn dig_default_is_enabled_auto_and_tips_the_treasury() {
        let policy = AutoTipPolicy::dig_default(ASSET);
        assert!(policy.enabled);
        assert_eq!(policy.mode, TipMode::Auto);
        assert_eq!(policy.recipient, default_recipient());
        assert_eq!(policy.asset_id, ASSET);
    }

    #[test]
    fn disabled_is_off_but_keeps_the_canonical_defaults() {
        let policy = AutoTipPolicy::disabled(ASSET);
        assert!(!policy.enabled);
        assert_eq!(policy.recipient, default_recipient());
        assert_eq!(policy.max_amount_per_day, DEFAULT_MAX_AMOUNT_PER_DAY);
    }
}
