# KIR

> KIR is extracted from [ksim-rs](https://github.com/KEKE-space/ksim-rs) into a standalone crate.

## Features

- [x] Free-style parse-print (`#[derive(ParsePrint)]`)
- [x] S-expression-style parse-print (`#[derive(SExpr)]`)

## TODO

- [ ] Support more Id types, e.g. `RuleId`, in addition to `ValueId`.
  > One possible implementation is on `dyn-dispatch` branch, which is not pretty though.
- [ ] Move `Type` from `kir` to the irs that use it.


## Examples

- [sexpr](examples/sexpr)
- [ksim-ir](examples/ksim-ir)
