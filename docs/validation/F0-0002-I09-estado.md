# F0-0002 I09 - measured status

Figures come from versioned evidence and the generated mutation population. Independent review and phase approval remain pending.

| Check | Result |
|---|---|
| `cargo test --locked --offline` | completed; 1356 tests passed across packages |
| `cargo test --locked --offline --release` | completed; 1358 tests passed across packages |
| Normative RD check/run | 152/152, 152/152 |
| `sh scripts/validate.sh` in an isolated clean copy | completed |
| `cargo clippy --all-targets -- -D warnings` | completed |
| `cargo fmt --all -- --check` | completed |
| Typed debug campaign | completed; seed 10495373245766874321; 100000 programs; 86473 negative cases |
| Typed release campaign | completed; seed 10495373245766874321; 300000 programs; 259905 negative cases |
| Production mutation population | 153 |
| Real mutation run in a copy | completed; 149/153 production mutations detected in release; 4 documented equivalent |

Implementation notes describe design decisions, quota limits, and ADR gaps.
