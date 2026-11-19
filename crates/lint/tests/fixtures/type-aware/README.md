A small tsconfig project for the type-aware rules that run through `tsrs`.
`src/main.ts` breaks one rule per line and takes the type of `save` from
`src/api.ts`, which no per-file linter can see; `clean` breaks none.
