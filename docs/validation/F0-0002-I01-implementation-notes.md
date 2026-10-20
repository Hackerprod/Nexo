# I01 implementation notes

I01 implements the diagnostic schema and presentation layer in
`src/diagnostic.rs`, the `nexo explain` command in `src/main.rs`, and its
library export in `src/lib.rs`. `tests/i01_diagnostic.rs` checks constructed
NX-E and R diagnostics, required and forbidden fields, closed vocabularies,
catalog identities and availability, coordinate contracts, exact JSON/text
output, Unicode spans, bounded input, and command usage errors. The existing
CLI and corpus suites continue to cover the pre-I01 behavior.

The registry contains 70 NX-E identities and 113 code/subcause combinations.
Registry tests compare those combinations with a literal expectation table;
catalog tests exercise every emitible combination through validation and JSON
serialization. Published examples and fixture pairs are compared with literal
strings. The repair tests check applicability validation, profile identity,
edit ordering, overlap, and unique IDs. The input tests distinguish the byte
limit from UTF-8 decoding and cover incomplete sequences at EOF.

## Boundaries and unresolved specification details

I01 serializes objects built by tests; compiler and runtime code do not yet
emit these diagnostics. Entries without published source pairs remain empty,
and repair consequences are included only where the ADR specifies them.
Invalid UTF-8 is represented as byte input and source views without decoded
text render coordinates as `0:0`. In §14.6, deletion examples do not specify
whitespace trimming, so the paired output preserves the remaining source
bytes literally. `NX-E0102/field_requires_grouped_call` is outside the
selected output profile. The ADR's catalog and field contracts do not provide
fixtures for every historical or deferred entry; these are not inferred.

`NX-E0003` stage selection and the closed `budget_kind` vocabulary are checked
in separate schema clauses because they express different constraints. The
feature coordinate selection handles the stage-specific `async` and
`partial_transfer` cases before the general parse-stage set.

## Reproduction and status

Run `sh scripts/validate.sh` from the repository root. It runs formatting,
strict clippy, and locked offline tests in debug and release. Run
`sh scripts/compare-head.sh` to compare `check` and `run` output against the
HEAD archive for the corpus, input boundaries, malformed UTF-8, and usage
errors. These commands provide local implementation evidence; I01 remains
pending independent review; local command results do not approve the cut or
phase. No commit is claimed for this local implementation.

Emission rules in §13.15 and empty insertions at the same point are not validated by the schema; they will be checked in the emitters. Compiler and runtime emitters are outside I01.
