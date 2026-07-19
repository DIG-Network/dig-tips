//! Tip events — the honest, user-facing record of what the policy decided.
//!
//! A [`TipEvent`] is what a consumer surfaces to the user (a toast, a log line, an audit entry) so
//! the default-on auto-tip stays TRANSPARENT: every decision — planned, deferred, or declined — has
//! a corresponding event. Events carry no key and no secret; a planned tip carries only the public
//! amount + recipient.

use chia_protocol::Bytes32;

use crate::decision::{CapReason, TipDecision};

/// A user-facing tip event derived from a [`TipDecision`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipEvent {
    /// A tip will be built: `amount` base units to `recipient`.
    TipPlanned {
        /// The planned tip amount, in base units.
        amount: u64,
        /// The recipient's inner (p2) puzzle hash.
        recipient: Bytes32,
    },
    /// A tip was deferred by a daily cap.
    TipDeferredCap {
        /// Which cap was hit.
        reason: CapReason,
    },
    /// A tip was declined — an unapproved manual tip.
    TipDeclined,
    /// No tip: the auto-tip is switched off.
    TipSkippedDisabled,
    /// No tip: the triggering send was below the threshold (or a zero manual amount).
    TipSkippedBelowThreshold,
}

impl TipEvent {
    /// Map a [`TipDecision`] to its user-facing event, using `recipient` for a planned tip.
    ///
    /// Returns `Some` for every decision (each outcome has an event worth surfacing); the `Option`
    /// return leaves room for a future "silent" decision without a breaking change.
    #[must_use]
    pub fn from_decision(decision: &TipDecision, recipient: Bytes32) -> Option<TipEvent> {
        let event = match *decision {
            TipDecision::Tip { amount } => TipEvent::TipPlanned { amount, recipient },
            TipDecision::SkipCapReached { reason } => TipEvent::TipDeferredCap { reason },
            TipDecision::SkipManualNotApproved => TipEvent::TipDeclined,
            TipDecision::SkipDisabled => TipEvent::TipSkippedDisabled,
            TipDecision::SkipBelowThreshold => TipEvent::TipSkippedBelowThreshold,
        };
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECIPIENT: Bytes32 = Bytes32::new([0x33u8; 32]);

    #[test]
    fn planned_carries_amount_and_recipient() {
        let e = TipEvent::from_decision(&TipDecision::Tip { amount: 100 }, RECIPIENT);
        assert_eq!(
            e,
            Some(TipEvent::TipPlanned {
                amount: 100,
                recipient: RECIPIENT
            })
        );
    }

    #[test]
    fn cap_maps_to_deferred() {
        let e = TipEvent::from_decision(
            &TipDecision::SkipCapReached {
                reason: CapReason::AmountCap,
            },
            RECIPIENT,
        );
        assert_eq!(
            e,
            Some(TipEvent::TipDeferredCap {
                reason: CapReason::AmountCap
            })
        );
    }

    #[test]
    fn manual_not_approved_maps_to_declined() {
        let e = TipEvent::from_decision(&TipDecision::SkipManualNotApproved, RECIPIENT);
        assert_eq!(e, Some(TipEvent::TipDeclined));
    }

    #[test]
    fn disabled_and_below_threshold_map_through() {
        assert_eq!(
            TipEvent::from_decision(&TipDecision::SkipDisabled, RECIPIENT),
            Some(TipEvent::TipSkippedDisabled)
        );
        assert_eq!(
            TipEvent::from_decision(&TipDecision::SkipBelowThreshold, RECIPIENT),
            Some(TipEvent::TipSkippedBelowThreshold)
        );
    }
}
