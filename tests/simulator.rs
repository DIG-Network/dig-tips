//! End-to-end tip round-trips on the in-process Chia Simulator + on-chain conformance.
//!
//! These prove the unsigned spends dig-tips builds are real, broadcast-ready transactions: they
//! validate on the simulator once the caller signs the reported signatures, value routes exactly to
//! the recipient + change, the recipient coin carries ONLY the minimal recipient-hint memo (NC-8),
//! and a fixed-input tip produces a stable coin (the golden regression pin). The simulator validates
//! against TESTNET11, so signatures are requested for [`Network::Testnet11`].

use chia_protocol::Bytes32;
use chia_wallet_sdk::signer::RequiredSignature;
use chia_wallet_sdk::test::{BlsPair, Simulator};
use chia_wallet_sdk::types::{run_puzzle, Condition, Conditions};
use clvm_traits::{FromClvm, ToClvm};
use clvmr::{Allocator, NodePtr};

use dig_cat::{cat_puzzle_hash, issue_cat, CatPayment, IssueCatRequest, TailKind};
use dig_tips::{build_tip, default_recipient, required_signatures, Cat, Network, TipRequest};

/// Whether `reqs` contains a BLS signature required from `public_key`.
fn has_bls_signer(
    reqs: &[RequiredSignature],
    public_key: chia_wallet_sdk::prelude::PublicKey,
) -> bool {
    reqs.iter().any(|r| match r {
        RequiredSignature::Bls(b) => b.public_key == public_key,
        RequiredSignature::Secp(_) => false,
    })
}

/// A stand-in $DIG asset issued to the owner: mint `amount` and return the (asset_id, minted Cat).
fn issue_to(sim: &mut Simulator, owner: &BlsPair, amount: u64) -> anyhow::Result<(Bytes32, Cat)> {
    let funding = sim.new_coin(owner.puzzle_hash, amount);
    let result = issue_cat(IssueCatRequest {
        funding_coin: funding,
        funder_pk: owner.pk,
        amount,
        recipients: vec![CatPayment::new(owner.puzzle_hash, amount)],
        tail: TailKind::SingleIssuance,
    })?;
    sim.spend_coins(result.unsigned.coin_spends, std::slice::from_ref(&owner.sk))?;
    Ok((result.asset_id, result.unsigned.children[0]))
}

/// Total unspent base units of `asset_id` owned by `p2_puzzle_hash` on the simulator.
fn balance(sim: &Simulator, p2_puzzle_hash: Bytes32, asset_id: Bytes32) -> u64 {
    sim.unspent_coins(cat_puzzle_hash(p2_puzzle_hash, asset_id), false)
        .iter()
        .map(|c| c.amount)
        .sum()
}

#[test]
fn tip_routes_value_and_validates_on_chain() -> anyhow::Result<()> {
    let mut sim = Simulator::new();
    let owner = BlsPair::new(1);
    let recipient = default_recipient();
    let minted = 100_000u64;

    let (asset_id, cat) = issue_to(&mut sim, &owner, minted)?;

    let tip = build_tip(TipRequest {
        cats: vec![cat],
        owner_pk: owner.pk,
        asset_id,
        recipient,
        amount: 1_000,
        change_p2_puzzle_hash: owner.puzzle_hash,
    })?;

    assert_eq!(tip.plan.delta, 0, "a tip conserves value");
    assert_eq!(tip.plan.outputs, 1_000);
    assert_eq!(tip.plan.change, 99_000);

    // The owner must be a required signer; signing exactly the reported sigs validates on chain.
    let reqs = required_signatures(&tip.coin_spends, &Network::Testnet11)?;
    assert!(has_bls_signer(&reqs, owner.pk), "owner must sign the tip");
    sim.spend_coins(tip.coin_spends, std::slice::from_ref(&owner.sk))?;

    assert_eq!(
        balance(&sim, recipient, asset_id),
        1_000,
        "treasury received the tip"
    );
    assert_eq!(
        balance(&sim, owner.puzzle_hash, asset_id),
        99_000,
        "owner kept the change"
    );
    Ok(())
}

#[test]
fn recipient_coin_memo_is_minimal_nc8() -> anyhow::Result<()> {
    let mut sim = Simulator::new();
    let owner = BlsPair::new(2);
    let recipient = default_recipient();

    let (asset_id, cat) = issue_to(&mut sim, &owner, 100_000)?;
    let tip = build_tip(TipRequest {
        cats: vec![cat],
        owner_pk: owner.pk,
        asset_id,
        recipient,
        amount: 1_000,
        change_p2_puzzle_hash: owner.puzzle_hash,
    })?;

    // Run the tip's lead coin puzzle and read back the CREATE_COIN to the recipient. Its memos MUST
    // be exactly `[recipient]` — the recipient hint only, no identity, no extra memos (NC-8).
    let memos = recipient_create_coin_memos(&tip.coin_spends[0], recipient, asset_id)?;
    assert_eq!(
        memos,
        vec![recipient],
        "the tip coin memo must be the recipient hint ONLY (NC-8)"
    );
    Ok(())
}

#[test]
fn fixed_input_tip_is_a_stable_golden() -> anyhow::Result<()> {
    let mut sim = Simulator::new();
    let owner = BlsPair::new(42);
    let recipient = default_recipient();

    let (asset_id, cat) = issue_to(&mut sim, &owner, 100_000)?;
    let tip = build_tip(TipRequest {
        cats: vec![cat],
        owner_pk: owner.pk,
        asset_id,
        recipient,
        amount: 1_000,
        change_p2_puzzle_hash: owner.puzzle_hash,
    })?;

    // The tip always pays the canonical treasury.
    assert_eq!(recipient, default_recipient());

    // The recipient coin (a CAT of `asset_id` at the recipient's p2 hash) is deterministic for a
    // fixed owner seed + fixed input — pin its coin id so any drift in the built spend is caught.
    let recipient_coin = tip
        .children
        .iter()
        .find(|c| c.coin.puzzle_hash == cat_puzzle_hash(recipient, asset_id))
        .expect("a recipient CAT coin must be created")
        .coin;
    assert_eq!(recipient_coin.amount, 1_000);
    assert_eq!(
        hex::encode(recipient_coin.coin_id()),
        GOLDEN_RECIPIENT_COIN_ID,
        "the fixed-input tip's recipient coin drifted"
    );
    Ok(())
}

/// The pinned recipient coin id for the [`fixed_input_tip_is_a_stable_golden`] vector.
const GOLDEN_RECIPIENT_COIN_ID: &str =
    "d0b6b4aaa4675998a9489555cbe6570edb9d81f19cbad7a227087f9545d62f7e";

/// Run `spend`'s puzzle + solution and return the memos of the CREATE_COIN paying the recipient's
/// CAT coin (i.e. `cat_puzzle_hash(recipient, asset_id)`), decoded as a list of `Bytes32`.
fn recipient_create_coin_memos(
    spend: &chia_protocol::CoinSpend,
    recipient: Bytes32,
    asset_id: Bytes32,
) -> anyhow::Result<Vec<Bytes32>> {
    let mut alloc = Allocator::new();
    let puzzle = spend.puzzle_reveal.to_clvm(&mut alloc)?;
    let solution = spend.solution.to_clvm(&mut alloc)?;
    let output = run_puzzle(&mut alloc, puzzle, solution)?;
    let conditions = Conditions::<NodePtr>::from_clvm(&alloc, output)?;

    let target = cat_puzzle_hash(recipient, asset_id);
    for condition in conditions {
        if let Condition::CreateCoin(cc) = condition {
            if cc.puzzle_hash == target {
                return decode_memos(&alloc, cc.memos);
            }
        }
    }
    anyhow::bail!("no CREATE_COIN to the recipient found");
}

/// Decode a CREATE_COIN memo field into a list of `Bytes32` (empty when there are no memos).
fn decode_memos(
    alloc: &Allocator,
    memos: chia_puzzle_types::Memos<NodePtr>,
) -> anyhow::Result<Vec<Bytes32>> {
    match memos {
        chia_puzzle_types::Memos::Some(ptr) => Ok(Vec::<Bytes32>::from_clvm(alloc, ptr)?),
        chia_puzzle_types::Memos::None => Ok(Vec::new()),
    }
}
