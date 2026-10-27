//! Opt-in, single-file TypeScript boundary policies ported from oc.

mod options;
mod shared;

use oxc_span::Span;

use super::{Context, Factory, Rule};
use options::{Options, matches};
use shared::Facts;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    ExportedClassCount,
    BarrelOnly,
    RelativeModule,
    ReservedInternalName,
    DynamicPropertyAccess,
    ExplicitAny,
    RuntimeTypeof,
    ZodInfer,
    NullableUnion,
    ReservedSpecifier,
}

pub const RULES: &[(&str, Factory)] = &[
    ("boundaries/exported-class-count", |options| build(Kind::ExportedClassCount, options)),
    ("boundaries/barrel-only", |options| build(Kind::BarrelOnly, options)),
    ("boundaries/relative-module", |options| build(Kind::RelativeModule, options)),
    ("boundaries/reserved-internal-name", |options| build(Kind::ReservedInternalName, options)),
    ("boundaries/dynamic-property-access", |options| build(Kind::DynamicPropertyAccess, options)),
    ("boundaries/explicit-any", |options| build(Kind::ExplicitAny, options)),
    ("boundaries/runtime-typeof", |options| build(Kind::RuntimeTypeof, options)),
    ("boundaries/zod-infer", |options| build(Kind::ZodInfer, options)),
    ("boundaries/nullable-union", |options| build(Kind::NullableUnion, options)),
    ("boundaries/reserved-specifier", |options| build(Kind::ReservedSpecifier, options)),
];

fn build(kind: Kind, options: &[serde_json::Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(Boundary { kind, options: Options::parse(options, kind)? }))
}

struct Boundary {
    kind: Kind,
    options: Options,
}

impl Rule for Boundary {
    fn name(&self) -> &'static str {
        RULES[self.kind as usize].0
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        let authored =
            std::path::Path::new(ctx.rel).extension().is_some_and(|extension| extension == "ts" || extension == "tsx") && !ctx.rel.ends_with(".d.ts");

        if self.options.exempt.matches(ctx.rel) || !authored {
            return;
        }

        let facts = ctx.shared(Facts::new);

        match self.kind {
            Kind::ExportedClassCount if !ctx.rel.ends_with("/index.ts") && facts.exported_classes.len() > 1 => ctx.report(
                facts.exported_classes[1],
                format!("{} exports {} classes; one exported class per file is permitted", ctx.rel, facts.exported_classes.len()),
            ),
            Kind::BarrelOnly if ctx.rel.ends_with("/index.ts") => {
                if let Some(span) = facts.non_barrel {
                    ctx.report(span, "index.ts barrels may contain only import and export statements");
                }
            }
            Kind::RelativeModule => {
                for (span, specifier) in &facts.specifiers {
                    if specifier.starts_with("./") || specifier.starts_with("../") {
                        ctx.report(*span, format!("relative module specifier is forbidden: {specifier}"));
                    }
                }
            }
            Kind::ReservedInternalName => {
                for name in &self.options.names {
                    let lower = name.to_lowercase();

                    if ctx.rel.to_lowercase().contains(&lower) {
                        ctx.report(Span::new(0, 0), format!("internal file names must omit {name}"));
                    }
                    for (span, identifier) in &facts.identifiers {
                        if identifier.contains(&lower) {
                            ctx.report(*span, format!("internal identifiers must omit {name}"));
                        }
                    }
                }
            }
            Kind::DynamicPropertyAccess => report(ctx, &facts.reflect_calls, "dynamic property access is forbidden; parse a zod DTO"),
            Kind::ExplicitAny => report(ctx, &facts.anys, "explicit any is forbidden; use a precise type or parse boundary input from unknown"),
            Kind::RuntimeTypeof => report(ctx, &facts.typeofs, "runtime typeof validation is forbidden; parse untrusted data with a zod DTO"),
            Kind::ZodInfer => {
                report(ctx, &facts.zod_helpers, "z.infer, z.input, and z.output are forbidden; write the boundary type explicitly");
                report(ctx, &facts.infer_imports, "type-only infer imports are forbidden; write the boundary type explicitly");
            }
            Kind::NullableUnion => report(ctx, &facts.nullish_unions, "null and undefined unions are forbidden; use the repository nullable types"),
            Kind::ReservedSpecifier => {
                for rule in &self.options.rules {
                    if rule.allow.matches(ctx.rel) {
                        continue;
                    }
                    for (span, specifier) in &facts.specifiers {
                        if matches(&rule.match_modules, specifier) && !matches(&rule.except, specifier) {
                            ctx.report_named(*span, &rule.name, &rule.message);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn report(ctx: &mut Context<'_, '_>, sites: &[Span], message: &str) {
    for span in sites {
        ctx.report(*span, message);
    }
}
