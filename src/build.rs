//! The tip spend builder — a single, minimal-memo CAT payment to the recipient.
//!
//! A tip IS a CAT send of one payment: dig-tips wraps [`dig_cat::build_cat_spend`] rather than
//! re-deriving any spend bytes (the CAT byte-source-of-truth lives in dig-cat / the SDK). The two
//! entry points are:
//!
//! - [`build_tip`] — the low-level builder. It always attempts the spend and propagates dig-cat's
//!   errors (a zero amount, insufficient funds); it NEVER returns a degenerate empty spend that
//!   would look like a successful no-op tip.
//! - [`build_tip_if_allowed`] — the canonical GUARDED path. It decides FIRST (caps, threshold,
//!   approval) and only builds when the decision is [`TipDecision::Tip`], so a capped/declined tip
//!   can never be built. This is the seam consumers should use for the honest auto-tip.

use chia_protocol::Bytes32;
use dig_cat::{build_cat_spend, Cat, CatPayment, PublicKey, SendCatRequest, UnsignedCatSpend};

use crate::decision::{decide_auto_tip, LedgerSnapshot, TipDecision};
use crate::error::Result;
use crate::policy::AutoTipPolicy;

/// A request to tip `amount` base units of `asset_id` to `recipient`, drawing from the owner's CATs.
///
/// The tip is a single CAT payment; any surplus returns to `change_p2_puzzle_hash`. The recipient's
/// coin carries only the recipient hint as its memo (NC-8 — no identity, no extra memos), because
/// the tip is built from [`CatPayment::new`] with no additional memos.
#[derive(Debug, Clone)]
pub struct TipRequest {
    /// The owner's spendable CAT coins of `asset_id` (each with a lineage proof).
    pub cats: Vec<Cat>,
    /// The public key controlling the input coins (authorizes the spend; it is a required signer).
    pub owner_pk: PublicKey,
    /// The asset id being tipped (e.g. the $DIG asset id).
    pub asset_id: Bytes32,
    /// The tip recipient's inner (p2) puzzle hash.
    pub recipient: Bytes32,
    /// The tip amount, in base units.
    pub amount: u64,
    /// Where change (selected total minus the tip) returns.
    pub change_p2_puzzle_hash: Bytes32,
}

/// Build the unsigned coin spends for a single tip payment.
///
/// Wraps [`dig_cat::build_cat_spend`] with exactly one [`CatPayment`] (minimal memo). Value is
/// conserved (`plan.delta == 0`); the returned spend is unsigned — the caller signs the messages
/// from [`crate::required_signatures`].
///
/// # Errors
/// Propagates the dig-cat error verbatim ([`dig_cat::CatError::ZeroAmount`] for a zero tip,
/// [`dig_cat::CatError::InsufficientFunds`] / [`dig_cat::CatError::TooManyInputs`] from selection).
/// A tip that cannot be built is an error, never a silent empty spend.
pub fn build_tip(req: TipRequest) -> Result<UnsignedCatSpend> {
    let unsigned = build_cat_spend(SendCatRequest {
        cats: req.cats,
        owner_pk: req.owner_pk,
        asset_id: req.asset_id,
        payments: vec![CatPayment::new(req.recipient, req.amount)],
        change_p2_puzzle_hash: req.change_p2_puzzle_hash,
    })?;
    Ok(unsigned)
}

/// The canonical guarded tip path: decide, THEN build only if allowed.
///
/// Runs [`decide_auto_tip`] for `primary_send_amount` against `ledger` + `policy`. On
/// [`TipDecision::Tip`] it builds the spend to the policy's recipient and returns
/// `(decision, Some(spend))`; on any skip it returns `(decision, None)` and builds NOTHING — the cap
/// cannot be bypassed because the spend is never constructed when the decision is a skip.
///
/// # Errors
/// Only the build can fail (see [`build_tip`]); a skip decision never errors.
pub fn build_tip_if_allowed(
    policy: &AutoTipPolicy,
    primary_send_amount: u64,
    ledger: &LedgerSnapshot,
    cats: Vec<Cat>,
    owner_pk: PublicKey,
    change_p2_puzzle_hash: Bytes32,
) -> Result<(TipDecision, Option<UnsignedCatSpend>)> {
    let decision = decide_auto_tip(policy, primary_send_amount, ledger);
    let TipDecision::Tip { amount } = decision else {
        return Ok((decision, None));
    };
    let spend = build_tip(TipRequest {
        cats,
        owner_pk,
        asset_id: policy.asset_id,
        recipient: policy.recipient,
        amount,
        change_p2_puzzle_hash,
    })?;
    Ok((decision, Some(spend)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chia_protocol::Coin;
    use chia_puzzle_types::LineageProof;
    use dig_cat::{CatError, CatInfo};
    use crate::error::Error;
    use chia_wallet_sdk::test::BlsPair;

    const ASSET: Bytes32 = Bytes32::new([0xABu8; 32]);
    const RECIPIENT: Bytes32 = Bytes32::new([0x77u8; 32]);

    /// A structurally-spendable CAT (fabricated lineage proof) — enough to drive the pure builder.
    fn spendable_cat(amount: u64) -> Cat {
        let coin = Coin::new(Bytes32::from([1u8; 32]), Bytes32::from([0xEEu8; 32]), amount);
        let proof = LineageProof {
            parent_parent_coin_info: Bytes32::from([1u8; 32]),
            parent_inner_puzzle_hash: Bytes32::from([2u8; 32]),
            parent_amount: amount,
        };
        Cat::new(coin, Some(proof), CatInfo::new(ASSET, None, Bytes32::from([3u8; 32])))
    }

    fn request(cats: Vec<Cat>, amount: u64) -> TipRequest {
        TipRequest {
            cats,
            owner_pk: BlsPair::new(1).pk,
            asset_id: ASSET,
            recipient: RECIPIENT,
            amount,
            change_p2_puzzle_hash: Bytes32::from([8u8; 32]),
        }
    }

    #[test]
    fn zero_amount_tip_is_an_error_not_an_empty_spend() {
        let err = build_tip(request(vec![spendable_cat(1_000)], 0)).unwrap_err();
        assert!(matches!(err, Error::Cat(CatError::ZeroAmount)));
    }

    #[test]
    fn insufficient_funds_surfaces_as_error() {
        let err = build_tip(request(vec![spendable_cat(10)], 1_000)).unwrap_err();
        assert!(matches!(err, Error::Cat(CatError::InsufficientFunds { .. })));
    }

    #[test]
    fn built_tip_conserves_value_and_pays_the_exact_amount() {
        let unsigned = build_tip(request(vec![spendable_cat(1_000)], 100)).unwrap();
        assert_eq!(unsigned.plan.delta, 0, "a tip conserves value");
        assert_eq!(unsigned.plan.outputs, 100);
        assert_eq!(unsigned.plan.change, 900);
    }

    #[test]
    fn if_allowed_builds_a_spend_when_the_decision_is_tip() {
        let policy = AutoTipPolicy {
            tip_amount: 100,
            threshold: 0,
            recipient: RECIPIENT,
            ..AutoTipPolicy::dig_default(ASSET)
        };
        let (decision, spend) = build_tip_if_allowed(
            &policy,
            1_000,
            &LedgerSnapshot::default(),
            vec![spendable_cat(1_000)],
            BlsPair::new(1).pk,
            Bytes32::from([8u8; 32]),
        )
        .unwrap();
        assert_eq!(decision, TipDecision::Tip { amount: 100 });
        let spend = spend.expect("a Tip decision must build a spend");
        assert_eq!(spend.plan.outputs, 100);
    }

    #[test]
    fn if_allowed_builds_nothing_when_capped() {
        let policy = AutoTipPolicy {
            tip_amount: 100,
            max_amount_per_day: 50,
            recipient: RECIPIENT,
            ..AutoTipPolicy::dig_default(ASSET)
        };
        let (decision, spend) = build_tip_if_allowed(
            &policy,
            1_000,
            &LedgerSnapshot::default(),
            vec![spendable_cat(1_000)],
            BlsPair::new(1).pk,
            Bytes32::from([8u8; 32]),
        )
        .unwrap();
        assert!(matches!(decision, TipDecision::SkipCapReached { .. }));
        assert!(spend.is_none(), "a capped tip must NOT build a spend");
    }
}
