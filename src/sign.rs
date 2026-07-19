//! The key-free signing boundary (re-export).
//!
//! dig-tips never signs. A tip's signing requirement is exactly a CAT send's, so the boundary is
//! re-exported verbatim from dig-cat — dig-tips adds no signing logic of its own. The caller passes
//! the built spend's `coin_spends` to [`required_signatures`], signs the reported messages with its
//! own key material, aggregates, and broadcasts.

pub use dig_cat::{required_signatures, Network};
