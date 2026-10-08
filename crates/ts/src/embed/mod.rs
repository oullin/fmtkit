//! Embedded languages in template literals, ported from oxfmt 0.71.0's
//! `core::embed` (`dispatcher.rs`, `services.rs`, `prettier_string.rs`).
//!
//! oxc_formatter decides which templates hold another language (`css`,
//! `styled.x`, `gql`, `graphql(…)`, `html`, `md`, `/* HTML */`, a JSX `css`
//! prop, `<style jsx>`, Angular `@Component`) and asks the session's
//! dispatcher to format them. This module is that dispatcher. A request that
//! does not format leaves its template as written; it never fails the file.
//!
//! oxfmt's JSDoc fence channel (the session's string embedder) is not ported:
//! oxc_formatter consults it only when JSDoc formatting is on, which oxfmt
//! leaves off by default and `[ts.format]` cannot turn on.

mod dispatcher;
mod text;

pub(crate) use dispatcher::services;

#[cfg(test)]
mod tests;
