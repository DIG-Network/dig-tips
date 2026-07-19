//! The dig-tips error taxonomy (SPEC §6).
//!
//! Tipping is a thin, honest wrapper over the CAT send builder, so the only fallible surface is the
//! on-chain spend construction — which fails exactly as [`dig_cat::CatError`] does (a zero-amount
//! tip, insufficient funds, too many inputs). dig-tips propagates those unchanged rather than
//! collapsing them into a degenerate empty spend: a tip that cannot be built is an `Err`, never a
//! silent no-op that would look like a successful (but empty) tip.

use thiserror::Error;

/// An error building a tip spend.
#[derive(Debug, Error)]
pub enum Error {
    /// The underlying CAT send builder failed (zero amount, insufficient funds, too many inputs, …).
    ///
    /// Surfaced verbatim so the caller can distinguish "nothing to tip with" from "tip declined by
    /// policy" (the latter is a [`crate::TipDecision`], not an error).
    #[error("tip spend build failed: {0}")]
    Cat(#[from] dig_cat::CatError),
}

/// The dig-tips result alias.
pub type Result<T> = std::result::Result<T, Error>;
