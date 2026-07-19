//! The pure tip-decision core — the unbypassable caps.
//!
//! Every function here is a PURE transform: it reads a policy plus a [`LedgerSnapshot`] (today's
//! running counters) and returns a [`TipDecision`]. There is no clock, no state, no I/O — the caller
//! owns the ledger and the wall clock. Keeping the caps pure is what makes them auditable and
//! testable, including the custody-critical overflow case: the cap math uses `checked_add` on BOTH
//! the amount ledger AND the tip counter, so a `u64::MAX` running total can never wrap below the cap
//! and silently unlock an unlimited tip.

use crate::policy::{AutoTipPolicy, TipMode};

/// The number of seconds in a UTC day, used to detect a day rollover for cap resets.
const SECONDS_PER_DAY: i64 = 86_400;

/// Today's running tip counters (owned + persisted by the caller, reset at the UTC day boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LedgerSnapshot {
    /// How many tips have already fired today.
    pub tips_today: u32,
    /// The total amount already tipped today (base units).
    pub amount_today: u64,
}

/// Why a tip was deferred by a cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapReason {
    /// The per-day tip COUNT cap ([`AutoTipPolicy::max_tips_per_day`]) is reached.
    FrequencyCap,
    /// The per-day AMOUNT cap ([`AutoTipPolicy::max_amount_per_day`]) would be exceeded.
    AmountCap,
}

/// The outcome of a pure tip decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipDecision {
    /// Tip `amount` base units to the policy's recipient.
    Tip {
        /// The amount to tip, in base units.
        amount: u64,
    },
    /// The auto-tip is switched off ([`AutoTipPolicy::enabled`] is `false`).
    SkipDisabled,
    /// The triggering send was below [`AutoTipPolicy::threshold`] (or the requested amount was zero).
    SkipBelowThreshold,
    /// A [`TipMode::Manual`] tip was not (yet) approved by the user.
    SkipManualNotApproved,
    /// A daily cap prevents the tip.
    SkipCapReached {
        /// Which cap was hit.
        reason: CapReason,
    },
}

/// Decide whether to auto-tip alongside a primary CAT send of `primary_send_amount`.
///
/// Fires only when the policy is enabled, in [`TipMode::Auto`], the send meets the threshold, and
/// neither daily cap is hit. The tip amount is the policy's fixed [`AutoTipPolicy::tip_amount`].
#[must_use]
pub fn decide_auto_tip(
    policy: &AutoTipPolicy,
    primary_send_amount: u64,
    ledger: &LedgerSnapshot,
) -> TipDecision {
    if !policy.enabled {
        return TipDecision::SkipDisabled;
    }
    if policy.mode != TipMode::Auto {
        return TipDecision::SkipManualNotApproved;
    }
    if primary_send_amount < policy.threshold {
        return TipDecision::SkipBelowThreshold;
    }
    decide_within_caps(policy, policy.tip_amount, ledger)
}

/// Decide whether to fire a scheduled daily tip (not tied to a send).
///
/// Identical to [`decide_auto_tip`] minus the per-send threshold check.
#[must_use]
pub fn decide_daily_tip(policy: &AutoTipPolicy, ledger: &LedgerSnapshot) -> TipDecision {
    if !policy.enabled {
        return TipDecision::SkipDisabled;
    }
    if policy.mode != TipMode::Auto {
        return TipDecision::SkipManualNotApproved;
    }
    decide_within_caps(policy, policy.tip_amount, ledger)
}

/// Decide a user-initiated manual tip of `requested_amount`, gated on explicit `approved`.
///
/// A manual tip is an explicit user action, so it is independent of [`AutoTipPolicy::mode`] — but it
/// still honours the master switch, requires approval, rejects a zero amount, and is bound by the
/// same daily caps (an explicit tip cannot bypass the honesty ceiling).
#[must_use]
pub fn decide_manual_tip(
    policy: &AutoTipPolicy,
    requested_amount: u64,
    approved: bool,
    ledger: &LedgerSnapshot,
) -> TipDecision {
    if !policy.enabled {
        return TipDecision::SkipDisabled;
    }
    if !approved {
        return TipDecision::SkipManualNotApproved;
    }
    if requested_amount == 0 {
        return TipDecision::SkipBelowThreshold;
    }
    decide_within_caps(policy, requested_amount, ledger)
}

/// The shared cap gate: tip `amount` unless a daily cap forbids it.
///
/// Overflow-safe by construction — the frequency check uses `checked_add(1)` on the counter and the
/// amount check uses `checked_add(amount)` on the running total, and an overflow (`None`) is treated
/// as "cap reached", never as a wrap that would slip past the ceiling.
fn decide_within_caps(policy: &AutoTipPolicy, amount: u64, ledger: &LedgerSnapshot) -> TipDecision {
    let would_exceed_frequency = ledger
        .tips_today
        .checked_add(1)
        .map_or(true, |n| n > policy.max_tips_per_day);
    if would_exceed_frequency {
        return TipDecision::SkipCapReached {
            reason: CapReason::FrequencyCap,
        };
    }

    let would_exceed_amount = ledger
        .amount_today
        .checked_add(amount)
        .map_or(true, |total| total > policy.max_amount_per_day);
    if would_exceed_amount {
        return TipDecision::SkipCapReached {
            reason: CapReason::AmountCap,
        };
    }

    TipDecision::Tip { amount }
}

/// Fold a fired tip of `amount` into the ledger, advancing both counters.
///
/// Uses `saturating_add` so a pathological counter can never wrap; in practice the caps make
/// saturation unreachable, but the ledger update stays total and panic-free regardless.
#[must_use]
pub fn apply_tip(ledger: LedgerSnapshot, amount: u64) -> LedgerSnapshot {
    LedgerSnapshot {
        tips_today: ledger.tips_today.saturating_add(1),
        amount_today: ledger.amount_today.saturating_add(amount),
    }
}

/// Whether `now_unix` falls on a later UTC day than `last_tip_unix` — i.e. the daily counters should
/// reset before the next decision.
///
/// Pure and clock-free: the caller supplies both timestamps. Uses `div_euclid` so it is correct for
/// pre-epoch (negative) timestamps too.
#[must_use]
pub fn is_new_utc_day(last_tip_unix: i64, now_unix: i64) -> bool {
    now_unix.div_euclid(SECONDS_PER_DAY) > last_tip_unix.div_euclid(SECONDS_PER_DAY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::treasury::default_recipient;
    use chia_protocol::Bytes32;

    const ASSET: Bytes32 = Bytes32::new([0x22u8; 32]);

    fn policy() -> AutoTipPolicy {
        AutoTipPolicy {
            tip_amount: 100,
            threshold: 10,
            max_tips_per_day: 3,
            max_amount_per_day: 250,
            ..AutoTipPolicy::dig_default(ASSET)
        }
    }

    #[test]
    fn below_threshold_skips() {
        let d = decide_auto_tip(&policy(), 5, &LedgerSnapshot::default());
        assert_eq!(d, TipDecision::SkipBelowThreshold);
    }

    #[test]
    fn disabled_skips() {
        let d = decide_auto_tip(&AutoTipPolicy::disabled(ASSET), 1_000, &LedgerSnapshot::default());
        assert_eq!(d, TipDecision::SkipDisabled);
    }

    #[test]
    fn manual_mode_auto_path_needs_approval() {
        let p = AutoTipPolicy {
            mode: TipMode::Manual,
            ..policy()
        };
        let d = decide_auto_tip(&p, 1_000, &LedgerSnapshot::default());
        assert_eq!(d, TipDecision::SkipManualNotApproved);
    }

    #[test]
    fn within_caps_tips_the_policy_amount() {
        let d = decide_auto_tip(&policy(), 1_000, &LedgerSnapshot::default());
        assert_eq!(d, TipDecision::Tip { amount: 100 });
    }

    #[test]
    fn frequency_cap_blocks_at_the_limit() {
        let ledger = LedgerSnapshot {
            tips_today: 3,
            amount_today: 0,
        };
        let d = decide_auto_tip(&policy(), 1_000, &ledger);
        assert_eq!(
            d,
            TipDecision::SkipCapReached {
                reason: CapReason::FrequencyCap
            }
        );
    }

    #[test]
    fn amount_cap_blocks_when_the_next_tip_would_exceed_it() {
        let ledger = LedgerSnapshot {
            tips_today: 1,
            amount_today: 200, // 200 + 100 > 250
        };
        let d = decide_auto_tip(&policy(), 1_000, &ledger);
        assert_eq!(
            d,
            TipDecision::SkipCapReached {
                reason: CapReason::AmountCap
            }
        );
    }

    #[test]
    fn amount_overflow_is_treated_as_cap_reached_not_a_wrap() {
        // A pathological running total must NOT wrap below the cap and silently unlock a tip.
        let ledger = LedgerSnapshot {
            tips_today: 0,
            amount_today: u64::MAX,
        };
        let d = decide_auto_tip(&policy(), 1_000, &ledger);
        assert_eq!(
            d,
            TipDecision::SkipCapReached {
                reason: CapReason::AmountCap
            }
        );
    }

    #[test]
    fn frequency_overflow_is_treated_as_cap_reached() {
        let p = AutoTipPolicy {
            max_tips_per_day: u32::MAX,
            ..policy()
        };
        let ledger = LedgerSnapshot {
            tips_today: u32::MAX,
            amount_today: 0,
        };
        let d = decide_auto_tip(&p, 1_000, &ledger);
        assert_eq!(
            d,
            TipDecision::SkipCapReached {
                reason: CapReason::FrequencyCap
            }
        );
    }

    #[test]
    fn daily_tip_ignores_the_send_threshold() {
        let d = decide_daily_tip(&policy(), &LedgerSnapshot::default());
        assert_eq!(d, TipDecision::Tip { amount: 100 });
    }

    #[test]
    fn manual_tip_requires_approval_and_nonzero() {
        let l = LedgerSnapshot::default();
        assert_eq!(
            decide_manual_tip(&policy(), 50, false, &l),
            TipDecision::SkipManualNotApproved
        );
        assert_eq!(
            decide_manual_tip(&policy(), 0, true, &l),
            TipDecision::SkipBelowThreshold
        );
        assert_eq!(
            decide_manual_tip(&policy(), 50, true, &l),
            TipDecision::Tip { amount: 50 }
        );
    }

    #[test]
    fn manual_tip_still_honours_the_amount_cap() {
        let l = LedgerSnapshot {
            tips_today: 0,
            amount_today: 200,
        };
        assert_eq!(
            decide_manual_tip(&policy(), 100, true, &l),
            TipDecision::SkipCapReached {
                reason: CapReason::AmountCap
            }
        );
    }

    #[test]
    fn apply_tip_advances_both_counters() {
        let l = apply_tip(LedgerSnapshot::default(), 100);
        assert_eq!(l.tips_today, 1);
        assert_eq!(l.amount_today, 100);
    }

    #[test]
    fn apply_tip_saturates_rather_than_wrapping() {
        let l = apply_tip(
            LedgerSnapshot {
                tips_today: u32::MAX,
                amount_today: u64::MAX,
            },
            100,
        );
        assert_eq!(l.tips_today, u32::MAX);
        assert_eq!(l.amount_today, u64::MAX);
    }

    #[test]
    fn new_utc_day_detects_the_midnight_rollover() {
        // 2024-01-01T23:59:00Z -> 2024-01-02T00:01:00Z crosses UTC midnight.
        let before = 1_704_153_540; // 23:59:00
        let after = 1_704_153_720; // 00:02:00 next day
        assert!(is_new_utc_day(before, after));
        assert!(!is_new_utc_day(before, before + 10));
    }

    #[test]
    fn tip_decision_targets_the_treasury_recipient() {
        // The policy default recipient is the treasury; the decision itself carries only the amount,
        // the recipient comes from the policy — assert they line up for the caller.
        assert_eq!(policy().recipient, default_recipient());
    }
}
