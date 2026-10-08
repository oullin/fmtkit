//! Put each link of a call chain of two or more calls on its own line.

use fmtkit_core::{Edit, EditSet};
use oxc_ast::AstKind;
use oxc_ast::ast::{CallExpression, Comment, MemberExpression, Program};
use oxc_ast_visit::Visit;
use rustc_hash::FxHashMap;

use crate::syntax::{Chained, has_comment_between, indent_unit, line_indent};

/// The gap before every link becomes `\n<indent><op>`, where the indent is the
/// base call's line indent plus one unit.
pub(crate) fn edits<'a>(text: &'a str, program: &'a Program<'a>) -> EditSet {
    let mut collector = Chains { text, unit: indent_unit(text), comments: &program.comments, edits: FxHashMap::default() };

    collector.visit_program(program);

    let mut edits: Vec<Edit> = collector.edits.into_iter().map(|((start, end), text)| Edit::new(start, end, text)).collect();

    edits.sort_by_key(|edit| edit.start);

    edits.into_iter().collect()
}

struct Chains<'t> {
    text: &'t str,
    unit: &'t str,
    comments: &'t [Comment],
    edits: FxHashMap<(u32, u32), String>,
}

/// The gap `[start, end)` between a call and the next link's property.
struct Link {
    start: u32,
    end: u32,
    optional: bool,
}

impl Chains<'_> {
    fn link(&self, member: &MemberExpression<'_>, object: &CallExpression<'_>) -> Option<Link> {
        let property_start = match member {
            MemberExpression::StaticMemberExpression(member) => member.property.span.start,
            MemberExpression::PrivateFieldExpression(member) => member.field.span.start,
            MemberExpression::ComputedMemberExpression(_) => return None,
        };

        let object_end = object.span.end;

        if property_start <= object_end || has_comment_between(self.comments, object_end, property_start) {
            return None;
        }

        let mut operator = self.text[object_end as usize..property_start as usize].bytes().filter(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'));

        let optional = match (operator.next(), operator.next(), operator.next()) {
            (Some(b'.'), None, None) => false,
            (Some(b'?'), Some(b'.'), None) => true,
            _ => return None,
        };

        Some(Link { start: object_end, end: property_start, optional })
    }

    fn chain<'a>(&self, outer: &'a CallExpression<'a>) -> Option<(&'a CallExpression<'a>, Vec<Link>)> {
        let mut call = outer;
        let mut links = Vec::new();

        while let Chained::Member(member) = Chained::of(&call.callee) {
            let Chained::Call(object) = Chained::of(member.object()) else { break };

            links.push(self.link(member, object)?);
            call = object;
        }

        (links.len() >= 2).then_some((call, links))
    }
}

impl<'a> Visit<'a> for Chains<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let AstKind::CallExpression(call) = kind else { return };
        let Some((base, links)) = self.chain(call) else { return };
        let indent = line_indent(self.text, base.span.start);

        for link in links {
            let replacement = format!("\n{indent}{}{}", self.unit, if link.optional { "?." } else { "." });

            if self.text[link.start as usize..link.end as usize] != replacement {
                self.edits.insert((link.start, link.end), replacement);
            }
        }
    }
}
