//! What the Effect rules share, as upstream keeps it in `effect/shared/`.
//!
//! Upstream reads oxlint's ESTree, which has no parentheses. The readers here
//! look through parentheses wherever upstream reads a node as it is, and walk
//! ancestors through [`ancestors`](super::super::anti_slop::shared::ancestors),
//! which hides them too.

pub mod effect_modules;
pub mod tagged_values;
