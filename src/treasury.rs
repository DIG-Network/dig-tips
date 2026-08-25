//! The canonical DIG treasury tip recipient.
//!
//! The default auto-tip destination is the DIG treasury, reused from [`dig_constants`] — the single
//! canonical home for that recipient. A WRONG recipient would silently misdirect every tip (a
//! custody break), so dig-tips never hand-copies the value: it re-exports the constant and pins the
//! reuse with a test.

use chia_protocol::Bytes32;

/// The default auto-tip recipient: the canonical DIG treasury inner (standard) puzzle hash.
///
/// Byte-identical to [`dig_constants::DIG_TREASURY_INNER_PUZZLE_HASH`] (reused, never copied). This
/// is the on-chain `p2_puzzle_hash` a tip's `CREATE_COIN` pays.
#[must_use]
pub fn default_recipient() -> Bytes32 {
    dig_constants::DIG_TREASURY_INNER_PUZZLE_HASH
}

/// The canonical DIG treasury address (bech32m `xch1…` form of [`default_recipient`]).
///
/// Re-exported from [`dig_constants::DIG_TREASURY_ADDRESS`] for display; the two forms cannot drift
/// (dig-constants proves the address decodes to the puzzle hash).
pub const TREASURY_ADDRESS: &str = dig_constants::DIG_TREASURY_ADDRESS;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_recipient_is_the_canonical_treasury() {
        assert_eq!(
            default_recipient(),
            dig_constants::DIG_TREASURY_INNER_PUZZLE_HASH,
            "the tip recipient must be the canonical treasury, never a copy"
        );
    }

    #[test]
    fn treasury_address_matches_the_canonical_constant() {
        assert_eq!(TREASURY_ADDRESS, dig_constants::DIG_TREASURY_ADDRESS);
    }

    /// The two assertions above compare this crate's re-export against `dig_constants`, so they
    /// hold for ANY value the upstream constant takes and would stay green if a dig-constants
    /// bump silently moved the treasury. That is a money destination -- every default tip this
    /// crate builds pays it -- so it is pinned here against the literal bytes as well. A
    /// deliberate treasury migration is expected to fail this test and be re-blessed knowingly.
    #[test]
    fn treasury_is_pinned_to_its_literal_bytes() {
        assert_eq!(
            hex::encode(default_recipient()),
            "ec7c304708c7d59c078d5ae098d0dea004decf47fa1cafebb266c10ad6466ce8",
            "the default tip recipient's puzzle hash changed"
        );
        assert_eq!(
            TREASURY_ADDRESS, "xch1a37rq3cgcl2ecpudttsf35x75qzdan68lgw2l6ajvmqs44jxdn5qv6pk3y",
            "the treasury address changed"
        );
    }
}
