//! The per-file step loop.
//!
//! One allocator serves the whole file and is reset before every parse. The
//! text is parsed once per change: a step that proposes nothing moves the
//! cursor on without a re-parse. Every parse after the first is a check of
//! the step that produced it: it must parse, and it must have the
//! [fingerprint](crate::fingerprint) of the input.
//!
//! A round is one pass over the schedule. A round that changed the text is
//! followed by another over its output, until a round changes nothing, so
//! the result is a fixed point: oxfmt and the passes do not always reach
//! one in a single round (an embedded template that grows a line, a comment
//! that moves). A round that changes nothing costs nothing extra, so an
//! already formatted file is still parsed exactly once.

use std::path::Path;

use fmtkit_config::TsFormat;
use fmtkit_core::{LineIndex, is_declaration};
use oxc_allocator::Allocator;
use oxc_formatter::JsFormatOptions;
use oxc_formatter_core::SessionServices;
use oxc_span::SourceType;

use crate::embed;
use crate::fingerprint::fingerprint;
use crate::format::{format, js_options, template_line_breaks};
use crate::passes::Pass;
use crate::syntax::{ParseFailure, parse};
use crate::{Formatted, TsError, complexity};

/// One pipeline step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Pass(Pass),
    /// `oxc_formatter`, reported under v1's name for it.
    Format,
}

impl Step {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Pass(pass) => pass.name(),
            Self::Format => "oxfmt",
        }
    }
}

/// A step and the most times it may change the text in a row; a run that
/// changes nothing ends the stage early.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Stage {
    step: Step,
    budget: u8,
}

impl Stage {
    const fn once(step: Step) -> Self {
        Self { step, budget: 1 }
    }

    const fn until_stable(step: Step) -> Self {
        Self { step, budget: STABLE_BUDGET }
    }
}

/// The re-run bound for body wrapping and the reorders. The reorders take
/// the outermost of nested candidates per run, so nesting needs re-runs.
const STABLE_BUDGET: u8 = 5;

/// v1's segment pipeline (the `blank-lines` CLI).
pub(crate) const SEGMENT: [Stage; 4] = [
    Stage::until_stable(Step::Pass(Pass::BodyWrap)),
    Stage::until_stable(Step::Pass(Pass::ClassReorder)),
    Stage::until_stable(Step::Pass(Pass::DeclarationReorder)),
    Stage::once(Step::Pass(Pass::BlankLine)),
];

/// v1's fluent pipeline (the `fluent-chains` CLI).
#[cfg(test)]
pub(crate) const FLUENT: [Stage; 3] =
    [Stage::once(Step::Pass(Pass::FluentChain)), Stage::once(Step::Pass(Pass::DrizzleQuery)), Stage::once(Step::Pass(Pass::ExpandedCall))];

/// v1's `format-all` schedule: segment, oxfmt, fluent, segment.
pub(crate) const FULL: [Stage; 12] = [
    SEGMENT[0],
    SEGMENT[1],
    SEGMENT[2],
    SEGMENT[3],
    Stage::once(Step::Format),
    Stage::once(Step::Pass(Pass::FluentChain)),
    Stage::once(Step::Pass(Pass::DrizzleQuery)),
    Stage::once(Step::Pass(Pass::ExpandedCall)),
    SEGMENT[0],
    SEGMENT[1],
    SEGMENT[2],
    SEGMENT[3],
];

/// Where a run is in its schedule.
struct Cursor {
    stage: usize,
    runs: u8,
}

/// The most rounds that may follow a round that changed the text before the
/// pipeline gives up on reaching a fixed point.
pub(crate) const EXTRA_ROUNDS: usize = 3;

/// The step named by the error when no fixed point is reached.
pub(crate) const IDEMPOTENCY: &str = "idempotency";

/// Run `schedule` over `source` until a round changes nothing. Declaration
/// files are only validated and scored: v1 neither passed nor printed them.
/// `applied` is every step that changed the text in any round, in order.
pub(crate) fn run(rel: &str, source_type: SourceType, source: &str, options: &TsFormat, score: bool, schedule: &[Stage]) -> Result<Formatted, TsError> {
    let round = Round::new(rel, source_type, options, score, schedule);
    let mut allocator = Allocator::default();

    fixed_point(source, |text| round.run(&mut allocator, text))
}

/// Run `round` over `source`, then over each output that differs from its
/// input, at most [`EXTRA_ROUNDS`] more times. A round must report no
/// applied steps exactly when its output equals its input. The result has
/// the last output and the union of every round's steps in order.
pub(crate) fn fixed_point(source: &str, mut round: impl FnMut(&str) -> Result<Formatted, TsError>) -> Result<Formatted, TsError> {
    let mut formatted = round(source)?;

    if formatted.applied.is_empty() {
        return Ok(formatted);
    }

    for _ in 0..EXTRA_ROUNDS {
        let next = round(&formatted.output)?;

        if next.applied.is_empty() {
            return Ok(Formatted { applied: formatted.applied, ..next });
        }

        for step in next.applied {
            if !formatted.applied.contains(&step) {
                formatted.applied.push(step);
            }
        }

        formatted.output = next.output;
    }

    Err(TsError::Invariant { step: IDEMPOTENCY, message: format!("the output still changes after {EXTRA_ROUNDS} extra rounds") })
}

/// A single round of `schedule` over `source`, so that tests see the stage
/// budgets that the extra rounds of [`run`] would otherwise hide.
#[cfg(test)]
pub(crate) fn run_round(rel: &str, source_type: SourceType, source: &str, schedule: &[Stage]) -> Result<Formatted, TsError> {
    Round::new(rel, source_type, &TsFormat::default(), false, schedule).run(&mut Allocator::default(), source)
}

/// What every round of one [`run`] shares.
struct Round<'r> {
    rel: &'r str,
    source_type: SourceType,
    score: bool,
    schedule: &'r [Stage],
    declaration: bool,
    js_options: JsFormatOptions,
    services: SessionServices,
}

impl<'r> Round<'r> {
    fn new(rel: &'r str, source_type: SourceType, options: &TsFormat, score: bool, schedule: &'r [Stage]) -> Self {
        let declaration = is_declaration(Path::new(rel));
        let schedule = if declaration { &[] } else { schedule };
        let js_options = js_options(options);
        let services = embed::services(options, &js_options);

        Self { rel, source_type, score, schedule, declaration, js_options, services }
    }

    /// One pass of the schedule over `source`. `applied` is empty exactly
    /// when the output equals `source`; scores are only computed then, since
    /// only such a round ends the run.
    fn run(&self, allocator: &mut Allocator, source: &str) -> Result<Formatted, TsError> {
        let Self { rel, source_type, score, schedule, declaration, ref js_options, ref services } = *self;
        let mut text = source.to_owned();
        let mut applied: Vec<&'static str> = Vec::new();
        let mut cursor = Cursor { stage: 0, runs: 0 };
        let mut last: Option<Step> = None;
        let mut expected: Option<u64> = None;

        loop {
            allocator.reset();

            let program = match parse(allocator, &text, source_type) {
                Ok(program) => program,
                Err(failure) => {
                    return Err(match last {
                        Some(step) => invariant(step, failure.message),
                        None => syntax_error(&text, failure),
                    });
                }
            };

            let print = fingerprint(&text, program);

            match (expected, last) {
                (None, _) => expected = Some(print),
                (Some(input), Some(step)) if input != print => return Err(invariant(step, "the output is not the same program as the input".to_owned())),
                _ => {}
            }

            let mut next = None;

            while let Some(stage) = schedule.get(cursor.stage) {
                if cursor.runs >= stage.budget {
                    cursor = Cursor { stage: cursor.stage + 1, runs: 0 };

                    continue;
                }

                cursor.runs += 1;

                let output = match stage.step {
                    Step::Pass(pass) => {
                        let edits = pass.edits(&text, program, declaration);

                        if edits.is_empty() { None } else { Some(edits.apply(&text).map_err(|conflict| invariant(stage.step, conflict.to_string()))?) }
                    }
                    Step::Format => {
                        let input = template_line_breaks(&text, program);

                        Some(format(allocator, &input, source_type, js_options.clone(), services).map_err(|message| invariant(stage.step, message))?)
                    }
                };

                match output {
                    Some(output) if output != text => {
                        next = Some(output);
                        last = Some(stage.step);

                        if !applied.contains(&stage.step.name()) {
                            applied.push(stage.step.name());
                        }

                        break;
                    }
                    _ => cursor = Cursor { stage: cursor.stage + 1, runs: 0 },
                }
            }

            let Some(next) = next else {
                if text == source {
                    applied.clear();
                }

                let complexity = if score && applied.is_empty() { complexity::score_program(rel, program, &text) } else { Vec::new() };

                return Ok(Formatted { output: text, applied, complexity });
            };

            text = next;
        }
    }
}

/// A parse failure of the input, with a 1-based line and byte column.
pub(crate) fn syntax_error(text: &str, failure: ParseFailure) -> TsError {
    let (line, column) = LineIndex::new(text).line_col(failure.offset.min(u32::try_from(text.len()).unwrap_or(u32::MAX)));

    TsError::Syntax { line, column, message: failure.message }
}

fn invariant(step: Step, message: String) -> TsError {
    TsError::Invariant { step: step.name(), message }
}
