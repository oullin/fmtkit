// Package complexity scores functions and reports the ones that exceed the
// configured cyclomatic and cognitive limits.
//
// The package owns the policy — the two limits, the keyed allow list, and the
// findings they produce — while each language lane supplies the measurements.
// The Go lane is here (Scan walks *ast.FuncDecl with gocyclo and gocognit);
// the TypeScript lane is measured by the bundled sidecar and enters through
// the same Function values, so both languages meet exactly one rule set.
package complexity
