//! The single walk behind [`super::score_program`]: it names every function,
//! counts both metrics, and folds each function into its reporting key.
//!
//! v1 walked the tree once to collect function sites and then re-walked every
//! site's subtree once per metric. Here one pre-order pass keeps a stack of
//! open functions instead:
//!
//! - **Cyclomatic** branch points go to the innermost open function only;
//!   nested functions add nothing to their parent (ESLint `complexity`).
//! - **Cognitive** increments that depend on depth are stored as
//!   `Σ(1 + absolute depth)` plus a count, so a function's own score is
//!   `flat + Σ(1 + depth) − count × base` (its base being the depth its
//!   parameters and body sit at), and a closing function hands its sums to its
//!   parent, which is how a nested function folds into every function around
//!   it (SonarSource).

use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentExpression, BreakStatement, CatchClause, Class, ConditionalExpression, ContinueStatement, DoWhileStatement,
    ExportDefaultDeclaration, ExportDefaultDeclarationKind, Expression, ForInStatement, ForOfStatement, ForStatement, Function, FunctionType, IfStatement,
    LogicalExpression, LogicalOperator, MethodDefinition, MethodDefinitionType, ObjectProperty, PropertyDefinition, PropertyDefinitionType, Statement,
    SwitchCase, SwitchStatement, VariableDeclarator, WhileStatement,
};
use oxc_ast_visit::{Visit, walk};
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashMap;

use fmtkit_core::ComplexityScore;

use super::collate;
use super::lines::LineCursor;
use super::names::{ANONYMOUS, NameSource, method_accessor, property_accessor};

/// Walks one program and accumulates its reporting keys.
pub struct Scorer<'s> {
    lines: LineCursor<'s>,
    drafts: Vec<Draft>,
    keys: FxHashMap<String, usize>,
    frames: Vec<Frame>,
    /// The name of each enclosing class, innermost last (`''` when unnamed).
    classes: Vec<String>,
    /// The absolute cognitive depth of the node being visited.
    depth: u64,
}

/// One reporting key while sites are folded into it.
struct Draft {
    line: u32,
    cyclomatic: u32,
    cognitive: u32,
    /// Whether a named function created the key (an `<anonymous>` key is not).
    named: bool,
}

/// One open function.
struct Frame {
    draft: usize,
    /// Whether this function created its key and so sets its cognitive score.
    owns_cognitive: bool,
    /// The absolute depth of this function's parameters and body.
    base: u64,
    cyclomatic: u32,
    cost: Cost,
}

/// Cognitive increments of a function and everything nested in it.
#[derive(Clone, Copy, Default)]
struct Cost {
    /// Increments that ignore depth: `else`, `else if`, logical runs, labelled jumps.
    flat: u64,
    /// `Σ(1 + absolute depth)` over the depth-weighted increments.
    weighted: u64,
    /// How many depth-weighted increments there were.
    count: u64,
}

impl<'s> Scorer<'s> {
    pub fn new(source: &'s str) -> Self {
        Self { lines: LineCursor::new(source), drafts: Vec::new(), keys: FxHashMap::default(), frames: Vec::new(), classes: Vec::new(), depth: 0 }
    }

    /// The scores, ordered by line and then by name as v1 ordered them.
    pub fn finish(self, rel: &str) -> Vec<ComplexityScore> {
        let drafts = self.drafts;

        let mut scores: Vec<ComplexityScore> = self
            .keys
            .into_iter()
            .map(|(name, index)| {
                let draft = &drafts[index];

                ComplexityScore { key: format!("{rel}#{name}"), name, line: draft.line, cyclomatic: draft.cyclomatic, cognitive: draft.cognitive }
            })
            .collect();

        scores.sort_unstable_by(|left, right| left.line.cmp(&right.line).then_with(|| collate::compare(&left.name, &right.name)));

        scores
    }

    fn class_name(&self) -> &str {
        self.classes.last().map_or("", String::as_str)
    }

    fn render(&self, source: Option<NameSource<'_, '_>>) -> String {
        source.map_or_else(String::new, |source| source.render(self.class_name()))
    }

    /// Open a function starting at `start` whose own name is `own` (`''` when
    /// nothing names it).
    fn open(&mut self, start: u32, own: String) {
        let (draft, owns_cognitive) = if own.is_empty() { self.anonymous(start) } else { self.named(start, own) };

        self.depth += 1;
        self.frames.push(Frame { draft, owns_cognitive, base: self.depth, cyclomatic: 1, cost: Cost::default() });
    }

    /// Resolve a named function's key, keeping two same-named declarations on
    /// different lines apart as `name:line`.
    fn named(&mut self, start: u32, name: String) -> (usize, bool) {
        let line = self.lines.line_at(start);

        let key = match self.keys.get(&name) {
            Some(&index) if self.drafts[index].named && self.drafts[index].line != line => format!("{name}:{line}"),
            _ => name,
        };

        if let Some(&index) = self.keys.get(&key) {
            return (index, false);
        }

        (self.insert(key, Draft { line, cyclomatic: 0, cognitive: 0, named: true }), true)
    }

    /// Resolve an anonymous function's key: the key of the function around it
    /// (v1 used that function's *name*, which sent the anonymous children of a
    /// `name:line` key to `name`), or `<anonymous>` at the top level.
    fn anonymous(&mut self, start: u32) -> (usize, bool) {
        if let Some(owner) = self.frames.last() {
            return (owner.draft, false);
        }

        if let Some(&index) = self.keys.get(ANONYMOUS) {
            return (index, false);
        }

        let line = self.lines.line_at(start);

        (self.insert(ANONYMOUS.to_owned(), Draft { line, cyclomatic: 0, cognitive: 0, named: false }), false)
    }

    fn insert(&mut self, key: String, draft: Draft) -> usize {
        let index = self.drafts.len();

        self.drafts.push(draft);
        self.keys.insert(key, index);

        index
    }

    fn close(&mut self) {
        let Some(frame) = self.frames.pop() else {
            return;
        };

        let cost = frame.cost;
        let draft = &mut self.drafts[frame.draft];

        draft.cyclomatic = draft.cyclomatic.max(frame.cyclomatic);

        if frame.owns_cognitive {
            let cognitive = cost.flat + cost.weighted - cost.count * frame.base;

            draft.cognitive = u32::try_from(cognitive).unwrap_or(u32::MAX);
        }

        if let Some(parent) = self.frames.last_mut() {
            parent.cost.flat += cost.flat;
            parent.cost.weighted += cost.weighted;
            parent.cost.count += cost.count;
        }

        self.depth -= 1;
    }

    /// One cyclomatic branch point in the innermost function.
    fn decision(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.cyclomatic += 1;
        }
    }

    /// A cognitive increment of one plus the current depth.
    fn weighted(&mut self) {
        let depth = self.depth;

        if let Some(frame) = self.frames.last_mut() {
            frame.cost.weighted += 1 + depth;
            frame.cost.count += 1;
        }
    }

    /// A cognitive increment that ignores depth.
    fn flat(&mut self, increment: u64) {
        if let Some(frame) = self.frames.last_mut() {
            frame.cost.flat += increment;
        }
    }

    /// Visit with the depth one deeper.
    fn deeper(&mut self, visit: impl FnOnce(&mut Self)) {
        self.depth += 1;
        visit(self);
        self.depth -= 1;
    }

    fn function<'a>(&mut self, it: &Function<'a>, flags: ScopeFlags, source: Option<NameSource<'_, 'a>>) {
        // Overload signatures, `declare function`, and bodiless methods are
        // `TSDeclareFunction` / `TSEmptyBodyFunctionExpression` in ESTree, which
        // v1 walked as plain nodes.
        if !matches!(it.r#type, FunctionType::FunctionDeclaration | FunctionType::FunctionExpression) {
            walk::walk_function(self, it, flags);

            return;
        }

        let own = match &it.id {
            Some(id) => id.name.as_str().to_owned(),
            None => self.render(source),
        };

        self.open(it.span.start, own);
        walk::walk_function(self, it, flags);
        self.close();
    }

    fn arrow<'a>(&mut self, it: &ArrowFunctionExpression<'a>, source: Option<NameSource<'_, 'a>>) {
        let own = self.render(source);

        self.open(it.span.start, own);
        walk::walk_arrow_function_expression(self, it);
        self.close();
    }

    fn class<'a>(&mut self, it: &Class<'a>, source: Option<NameSource<'_, 'a>>) {
        let name = match &it.id {
            Some(id) => id.name.as_str().to_owned(),
            None => self.render(source),
        };

        self.classes.push(name);
        walk::walk_class(self, it);
        self.classes.pop();
    }

    /// Visit a declaration's value, naming it from `source` when it is a
    /// function or a class itself. Anything else (a parenthesised or cast
    /// function included) passes no name down, as in v1.
    fn named_expression<'a>(&mut self, expression: &Expression<'a>, source: NameSource<'_, 'a>) {
        match expression {
            Expression::FunctionExpression(it) => self.function(it, ScopeFlags::Function, Some(source)),
            Expression::ArrowFunctionExpression(it) => self.arrow(it, Some(source)),
            Expression::ClassExpression(it) => self.class(it, Some(source)),
            _ => self.visit_expression(expression),
        }
    }

    /// Score an `if` and its `else` ladder: the leading `if` costs one plus
    /// the depth, each `else if` and the final `else` cost one flat.
    fn if_chain(&mut self, it: &IfStatement<'_>, leading: bool) {
        self.decision();

        if leading {
            self.weighted();
        } else {
            self.flat(1);
        }

        self.visit_expression(&it.test);
        self.deeper(|this| this.visit_statement(&it.consequent));

        match &it.alternate {
            None => {}
            Some(Statement::IfStatement(alternate)) => self.if_chain(alternate, false),
            Some(alternate) => {
                self.flat(1);
                self.deeper(|this| this.visit_statement(alternate));
            }
        }
    }

    /// Visit one logical operand tree in source order, counting a branch per
    /// operator and a cognitive increment per run of one operator. Only bare
    /// logical children join the run; a parenthesised one starts its own.
    fn logical_chain(&mut self, it: &LogicalExpression<'_>, previous: &mut Option<LogicalOperator>, runs: &mut u64) {
        self.decision();
        self.logical_operand(&it.left, previous, runs);

        if *previous != Some(it.operator) {
            *runs += 1;
            *previous = Some(it.operator);
        }

        self.logical_operand(&it.right, previous, runs);
    }

    fn logical_operand(&mut self, operand: &Expression<'_>, previous: &mut Option<LogicalOperator>, runs: &mut u64) {
        match operand {
            Expression::LogicalExpression(inner) => self.logical_chain(inner, previous, runs),
            _ => self.visit_expression(operand),
        }
    }
}

impl<'a> Visit<'a> for Scorer<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.function(it, flags, None);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.arrow(it, None);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        self.class(it, None);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        self.visit_binding_pattern(&it.id);

        if let Some(annotation) = &it.type_annotation {
            self.visit_ts_type_annotation(annotation);
        }

        if let Some(init) = &it.init {
            self.named_expression(init, NameSource::Binding(&it.id));
        }
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator.is_logical() {
            self.decision();
        }

        self.visit_assignment_target(&it.left);
        self.named_expression(&it.right, NameSource::Target(&it.left));
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        self.visit_decorators(&it.decorators);
        self.visit_property_key(&it.key);

        // `TSAbstractMethodDefinition` was not a name source in v1.
        let source = (it.r#type == MethodDefinitionType::MethodDefinition).then_some(NameSource::Key {
            key: &it.key,
            accessor: method_accessor(it.kind),
            qualified: true,
        });

        self.function(&it.value, ScopeFlags::Function, source);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        self.visit_property_key(&it.key);
        self.named_expression(&it.value, NameSource::Key { key: &it.key, accessor: property_accessor(it.kind), qualified: false });
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        self.visit_decorators(&it.decorators);
        self.visit_property_key(&it.key);

        if let Some(annotation) = &it.type_annotation {
            self.visit_ts_type_annotation(annotation);
        }

        let Some(value) = &it.value else {
            return;
        };

        // `TSAbstractPropertyDefinition` was not a name source in v1.
        if it.r#type == PropertyDefinitionType::PropertyDefinition {
            self.named_expression(value, NameSource::Key { key: &it.key, accessor: "", qualified: true });
        } else {
            self.visit_expression(value);
        }
    }

    fn visit_export_default_declaration(&mut self, it: &ExportDefaultDeclaration<'a>) {
        match &it.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => self.function(function, ScopeFlags::Function, Some(NameSource::Default)),
            ExportDefaultDeclarationKind::ClassDeclaration(class) => self.class(class, Some(NameSource::Default)),
            ExportDefaultDeclarationKind::TSInterfaceDeclaration(interface) => self.visit_ts_interface_declaration(interface),
            declaration => self.named_expression(declaration.to_expression(), NameSource::Default),
        }
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.if_chain(it, true);
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        self.decision();
        self.weighted();
        self.visit_expression(&it.test);
        self.deeper(|this| {
            this.visit_expression(&it.consequent);
            this.visit_expression(&it.alternate);
        });
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.weighted();
        self.visit_expression(&it.discriminant);
        self.deeper(|this| this.visit_switch_cases(&it.cases));
    }

    fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
        if it.test.is_some() {
            self.decision();
        }

        walk::walk_switch_case(self, it);
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        self.decision();
        self.weighted();

        if let Some(init) = &it.init {
            self.visit_for_statement_init(init);
        }

        if let Some(test) = &it.test {
            self.visit_expression(test);
        }

        if let Some(update) = &it.update {
            self.visit_expression(update);
        }

        self.deeper(|this| this.visit_statement(&it.body));
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        self.decision();
        self.weighted();
        self.visit_for_statement_left(&it.left);
        self.visit_expression(&it.right);
        self.deeper(|this| this.visit_statement(&it.body));
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        self.decision();
        self.weighted();
        self.visit_for_statement_left(&it.left);
        self.visit_expression(&it.right);
        self.deeper(|this| this.visit_statement(&it.body));
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        self.decision();
        self.weighted();
        self.visit_expression(&it.test);
        self.deeper(|this| this.visit_statement(&it.body));
    }

    fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
        self.decision();
        self.weighted();
        self.deeper(|this| this.visit_statement(&it.body));
        self.visit_expression(&it.test);
    }

    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        self.decision();
        self.weighted();

        if let Some(param) = &it.param {
            self.visit_catch_parameter(param);
        }

        self.deeper(|this| this.visit_block_statement(&it.body));
    }

    fn visit_break_statement(&mut self, it: &BreakStatement<'a>) {
        if it.label.is_some() {
            self.flat(1);
        }
    }

    fn visit_continue_statement(&mut self, it: &ContinueStatement<'a>) {
        if it.label.is_some() {
            self.flat(1);
        }
    }

    fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
        let mut previous = None;
        let mut runs = 0;

        self.logical_chain(it, &mut previous, &mut runs);
        self.flat(runs);
    }
}
