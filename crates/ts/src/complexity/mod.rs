//! Cyclomatic and cognitive complexity for scripts. Scores and keys match
//! fmtkit 1.x exactly (see `fixtures/complexity/`), except for the documented
//! key fix for anonymous functions nested in the second of two same-named
//! functions.
//!
//! - **Cyclomatic** follows ESLint's `complexity`: one, plus every `if`,
//!   ternary, loop, `catch`, `case` with a test, `&&`/`||`/`??`, and
//!   `&&=`/`||=`/`??=` the function itself owns.
//! - **Cognitive** follows SonarSource: each control structure costs one plus
//!   its depth, `else` and `else if` cost one, a run of one logical operator
//!   costs one, a labelled jump costs one, and nested functions deepen
//!   everything inside them.
//!
//! A named function is one key. An anonymous function reports under the
//! function around it (or `<anonymous>`), folding its cognitive cost in and
//! keeping its own cyclomatic number, of which the key reports the worst.
//!
//! The scores rely on the program being parsed with `preserve_parens` (the
//! oxc default, and v1's): `(a && b) && c` is two logical runs, and a
//! parenthesised function is anonymous.

mod collate;
mod lines;
mod names;
mod scorer;

#[cfg(test)]
mod tests;

use oxc_ast::ast::Program;
use oxc_ast_visit::Visit;

use fmtkit_core::ComplexityScore;

/// Score every function in `program`. `rel` prefixes every key.
pub fn score_program(rel: &str, program: &Program<'_>, source: &str) -> Vec<ComplexityScore> {
    let mut scorer = scorer::Scorer::new(source);

    scorer.visit_program(program);

    scorer.finish(rel)
}
