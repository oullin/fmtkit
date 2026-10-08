//! The text passes. Each computes an [`EditSet`] against one parsed text; the
//! pipeline applies it and re-parses before the next pass.

mod blank_line;
mod body_wrap;
mod class_reorder;
mod declaration_reorder;
mod drizzle;
mod expanded_call;
mod fluent_chain;
mod lists;
pub(crate) mod spacing;

use fmtkit_core::EditSet;
use oxc_ast::ast::Program;

/// One formatting pass, named as v1 reported it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pass {
    BodyWrap,
    ClassReorder,
    DeclarationReorder,
    BlankLine,
    FluentChain,
    DrizzleQuery,
    ExpandedCall,
}

impl Pass {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::BodyWrap => "body-wrap",
            Self::ClassReorder => "class-reorder",
            Self::DeclarationReorder => "declaration-reorder",
            Self::BlankLine => "blank-lines",
            Self::FluentChain => "fluent-chains",
            Self::DrizzleQuery => "drizzle-queries",
            Self::ExpandedCall => "expanded-calls",
        }
    }

    /// The edits this pass makes to `text`, which `program` was parsed from.
    /// No-op edits are dropped, so an empty set means the pass is settled.
    /// The Drizzle and expanded-call passes leave declaration files alone.
    pub(crate) fn edits<'a>(self, text: &'a str, program: &'a Program<'a>, declaration: bool) -> EditSet {
        let mut edits = match self {
            Self::DrizzleQuery | Self::ExpandedCall if declaration => return EditSet::new(),
            Self::BodyWrap => body_wrap::edits(text, program),
            Self::ClassReorder => class_reorder::edits(text, program),
            Self::DeclarationReorder => declaration_reorder::edits(text, program),
            Self::BlankLine => blank_line::edits(text, program),
            Self::FluentChain => fluent_chain::edits(text, program),
            Self::DrizzleQuery => drizzle::edits(text, program),
            Self::ExpandedCall => expanded_call::edits(text, program),
        };

        edits.normalize(text);

        edits
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! Single-pass drivers mirroring the v1 pass tests: a pass over a text that
    //! does not parse proposes no edits.

    use std::path::Path;

    use fmtkit_core::{EditSet, Lang, is_declaration};
    use oxc_allocator::Allocator;

    use super::Pass;
    use crate::syntax::{parse, source_type};

    /// The edits `pass` proposes for `source`. Every input a pass test uses is
    /// also checked to be a fixed point of the whole pipeline after one run.
    pub(crate) fn compute(pass: Pass, rel: &str, source: &str) -> EditSet {
        let allocator = Allocator::default();
        let lang = Lang::from_path(Path::new(rel)).unwrap_or(Lang::Ts);
        let Some(source_type) = source_type(lang, rel) else {
            return EditSet::new();
        };

        crate::tests::assert_idempotent(rel, source);

        parse(&allocator, source, source_type).map_or_else(|_| EditSet::new(), |program| pass.edits(source, program, is_declaration(Path::new(rel))))
    }

    pub(crate) fn apply_once(pass: Pass, rel: &str, source: &str) -> String {
        compute(pass, rel, source).apply(source).expect("a pass's edits never overlap")
    }

    /// Re-run `pass` until it proposes nothing, at most five times.
    pub(crate) fn until_stable(pass: Pass, rel: &str, source: &str) -> String {
        let mut text = source.to_owned();

        for _ in 0..5 {
            let next = apply_once(pass, rel, &text);

            if next == text {
                break;
            }

            text = next;
        }

        text
    }

    pub(crate) fn wrap_fully(source: &str) -> String {
        until_stable(Pass::BodyWrap, "sample.ts", source)
    }

    /// The v1 fluent pipeline: fluent chains, Drizzle queries, expanded calls, once each.
    pub(crate) fn fluent(rel: &str, source: &str) -> String {
        [Pass::FluentChain, Pass::DrizzleQuery, Pass::ExpandedCall].into_iter().fold(source.to_owned(), |text, pass| apply_once(pass, rel, &text))
    }
}
