#!/usr/bin/env python3
"""Población de mutaciones de producción para el corte I10a."""
from pathlib import Path

import i07c_mutaciones as engine

engine.BASE = "6cddc5a"
engine.OUTPUT = Path(__file__).resolve().parents[1] / "docs/validation/F0-0002-I10a-mutations.json"
engine.RESULTS_PATH = Path(__file__).resolve().parents[1] / "docs/validation/F0-0002-I10a-mutation-results.jsonl"
engine.DIRECTORIES = (
    "src/check.rs",
    "src/main.rs",
    "src/check/names.rs",
    "src/diagnostic.rs",
    "src/frontend/parse.rs",
    "src/ir/effects.rs",
    "src/ir/lower.rs",
    "src/ir/model.rs",
    "src/ir/mod.rs",
    "src/ir/ownership_flow.rs",
    "src/ir/ownership_flow/sparse.rs",
    "src/ir/verify.rs",
)
engine.DEFAULT_ONLY = "production"
engine.TARGETED_TEST = None

_base_candidates = engine.candidates


def _candidates(base: str) -> list[dict]:
    names_source = (engine.ROOT / "src/check/names.rs").read_text(encoding="utf-8")
    names_test_start = next(
        index for index, line in enumerate(names_source.splitlines(), 1)
        if line == "mod tests {" and index > 1
        and names_source.splitlines()[index - 2] == "#[cfg(test)]"
    )
    rows = [
        row for row in _base_candidates(base)
        if "|| let " not in row["replacement"]
        and "&& let " not in row["replacement"]
        and "move && {" not in row["replacement"]
        and row["replacement"].strip()
        and "or_else(&&" not in row["replacement"]
        and "then(&&" not in row["replacement"]
        and not (
            row["file"] == "src/ir/effects.rs"
            and row["operator"] == "le"
            and "source_returns.partition_point(|candidate| candidate.start <= span.end)"
            in row["literal"]
        )
        and not (
            row["file"] == "src/ir/verify.rs"
            and row["operator"] == "le"
            and ("self.entered[definition] <= self.entered[self.block_index]" in row["literal"]
                 or "self.exited[self.block_index] <= self.exited[definition]" in row["literal"])
        )
        and not (
            row["file"] == "src/check/names.rs"
            and row["line"] >= names_test_start
        )
        and not (
            row["file"] == "src/ir/effects.rs"
            and row["operator"] == "le"
            and ".filter(|candidate| span.start <= candidate.start" in row["literal"]
            and "candidate.end <= span.end" in row["literal"]
        )
        and not (
            row["file"] == "src/ir/effects.rs"
            and row["operator"] == "continue"
            and "indices.get(target)" in row["literal"]
        )
        and not (
            row["file"] == "src/ir/lower.rs"
            and row["operator"] == "and"
            and "ir_limit_span.is_none()" in row["literal"]
        )
        and not (
            row["file"] == "src/ir/lower.rs"
            and row["operator"] == "continue"
            and "ownership_global_limited" in row["literal"]
        )
    ]
    extras = [
        (
            "src/check/names.rs",
            "        // Parsed input has a normative nesting bound of 128. This depth is an\n"
            "        // internal recursion statistic, not a second analysis quota.\n"
            "        let _ = span;\n        self.depth += 1;",
            "        if self.depth >= 80 {\n"
            "            return Err(diagnostic(\"NX-E0003\", None, Stage::Types,\n"
            "                Operation::AnalysisLimit, span.clone()));\n"
            "        }\n        self.depth += 1;",
            "restore_depth_guard",
        ),
        (
            "src/frontend/parse.rs",
            "work.fetch_add(1, std::sync::atomic::Ordering::Relaxed);",
            "work.fetch_add(0, std::sync::atomic::Ordering::Relaxed);",
            "remove_parser_probe_charge",
        ),
        (
            "src/ir/ownership_flow/sparse.rs",
            "            for ins in &block.instructions {\n"
            "                budget.charge(&ins.span)?;\n"
            "                sparse_transfer(&mut state, ins, function);",
            "            for ins in &block.instructions {\n"
            "                let _ = &ins.span;\n"
            "                sparse_transfer(&mut state, ins, function);",
            "remove_sparse_instruction_charge",
        ),
        (
            "src/ir/effects.rs",
            "        ) || blocks_by_id\n"
            "            .get(&e.block.0)\n"
            "            .is_some_and(|block| e.index == block.instructions.len())",
            "        ) || false",
            "ignore_block_id_exit_index",
        ),
        (
            "src/ir/lower.rs",
            "    let entries = owned_entries;\n"
            "    if !diagnostics.is_empty() {\n"
            "        fn diagnostic_key(d: &Diagnostic)",
            "    let entries = owned_entries;\n"
            "    if diagnostics.is_empty() {\n"
            "        fn diagnostic_key(d: &Diagnostic)",
            "skip_ownership_errors_before_effects",
        ),
        (
            "src/check.rs",
            "            Err(_) => CompilationCheck {\n                inferred: types::InferredTypes::default(),\n                resolved: None,\n                errors: Vec::new(),\n                internal_error: Some(\"type checker worker unavailable\"),\n            },",
            "            Err(_) => panic!(\"type checker worker unavailable\"),",
            "misreport_checker_spawn_failure",
        ),
        (
            "src/ir/verify.rs",
            "        fn union(\n"
            "            left: &Option<std::sync::Arc<LiveNode>>,\n"
            "            right: &Option<std::sync::Arc<LiveNode>>,\n"
            "            bit: usize,\n"
            "        ) -> Option<std::sync::Arc<LiveNode>> {\n"
            "            super::count_cfg_work(1);",
            "        fn union(\n"
            "            left: &Option<std::sync::Arc<LiveNode>>,\n"
            "            right: &Option<std::sync::Arc<LiveNode>>,\n"
            "            bit: usize,\n"
            "        ) -> Option<std::sync::Arc<LiveNode>> {\n"
            "            super::count_cfg_work(0);",
            "remove_live_union_work_charge",
        ),
        (
            "src/ir/effects.rs",
            "    let mut distance = vec![usize::MAX; functions.len()];",
            "    let mut distance = vec![0; functions.len()];",
            "truncate_witness_dynamic_programming",
        ),
        (
            "src/ir/effects.rs",
            "if exit_sites\n                        .get(&(block.id.0, index))\n                        .is_some_and(|exit| exit.live_locals.contains(local))",
            "if f.exit_info.iter().any(|exit| { super::count_cfg_work(1); exit.block == block.id && exit.index == index && exit.live_locals.contains(local) })",
            "rescan_exit_info_per_droplocal",
        ),
        (
            "src/ir/effects.rs",
            "|| blocks_by_id\n            .get(&e.block.0)\n            .is_some_and(|block| e.index == block.instructions.len())",
            "|| std::iter::once(&f.block).chain(&f.blocks).find(|block| { super::count_cfg_work(1); block.id == e.block }).is_some_and(|block| e.index == block.instructions.len())",
            "rescan_blocks_per_exit",
        ),
        (
            "src/ir/effects.rs",
            "let block = blocks_by_id\n                .get(&exit.block.0)\n                .copied()\n                .ok_or(\"exit block\")?;",
            "let block = std::iter::once(&f.block).chain(&f.blocks).find(|block| { super::count_cfg_work(1); block.id.0 == exit.block.0 }).ok_or(\"exit block\")?;",
            "rescan_blocks_per_verified_exit",
        ),
        (
            "src/ir/effects.rs",
            "if *used >= limit {",
            "if *used > limit {",
            "effect_quota_allows_first_excess",
        ),
        (
            "src/ir/ownership_flow.rs",
            "        if local_denied || global_denied {\n            // A rejected unit is not part of the granted work balance.\n            // In particular, local rejections must not consume compilation quota.\n        } else {",
            "        self.used += 1;\n        self.global_used += 1;\n        if local_denied || global_denied {",
            "charge_rejected_ownership_unit",
        ),
        (
            "src/ir/effects.rs",
            "let prefix = (a.unknown, a_causes).cmp(&(b.unknown, b_causes));",
            "let prefix = (a.unknown, a.edges, a_causes).cmp(&(b.unknown, b.edges, b_causes));",
            "witness_omits_total_cause_priority",
        ),
        (
            "src/ir/effects.rs",
            "if exit_sites\n                        .get(&(block.id.0, index))\n                        .is_some_and(|exit| exit.live_locals.contains(local))",
            "if f.exit_info.iter().any(|exit| { super::count_cfg_work(1); exit.block == block.id && exit.index == index && exit.live_locals.contains(local) })",
            "S1_rescan_exit_info_for_droplocal",
        ),
        (
            "src/ir/effects.rs",
            "        if let Terminator::Return { span, .. } = &block.terminator {\n            super::count_cfg_work(1);",
            "        if let Terminator::Return { span, .. } = &block.terminator {\n            super::count_cfg_work(1);\n            let _scan = blocks.iter().filter(|b| matches!(b.terminator, Terminator::Return { .. })).count();\n            std::hint::black_box(_scan);",
            "S2_scan_blocks_per_exit",
        ),
        (
            "src/ir/effects.rs",
            "        if let Terminator::Return { span, .. } = &block.terminator {\n            super::count_cfg_work(1);",
            "        if let Terminator::Return { span, .. } = &block.terminator {\n            super::count_cfg_work(1);\n            let _rescan = return_spans(&source_function.body, &function.source_text);",
            "S3_rescan_source_returns_per_exit",
        ),
        (
            "src/ir/effects.rs",
            "let block = blocks_by_id\n                .get(&exit.block.0)\n                .copied()\n                .ok_or(\"exit block\")?;",
            "let block = std::iter::once(&f.block).chain(&f.blocks).find(|block| { super::count_cfg_work(1); block.id.0 == exit.block.0 }).ok_or(\"exit block\")?;",
            "S4_find_block_per_exit",
        ),
        (
            "src/ir/verify.rs",
            "        for &id in &loan_temporaries {\n            if live_out[index].contains(id) {\n                super::count_cfg_work(1);",
            "        for &id in &loan_temporaries {\n            super::count_cfg_work(1);\n            if live_out[index].contains(id) {",
            "S5_count_every_loan_temporary",
        ),
    ]
    for path, literal, replacement, operator in extras:
        source = (engine.ROOT / path).read_text(encoding="utf-8")
        assert source.count(literal) == 1, (path, operator)
        start = source.index(literal)
        rows.append({
            "file": path,
            "line": source.count("\n", 0, start) + 1,
            "column": start - source.rfind("\n", 0, start),
            "operator": operator,
            "literal": literal,
            "replacement": replacement,
            "site_count": 1,
        })
    # A mutation is identified by its actual one-site source rewrite. Keep the
    # first generated row when candidate sources yield an identical rewrite.
    unique = {}
    for row in rows:
        key = (row["file"], row["literal"], row["replacement"], row["operator"])
        unique.setdefault(key, row)
    rows = list(unique.values())
    rows.sort(key=lambda row: (row["file"], row["line"], row["column"], row["operator"]))
    for index, row in enumerate(rows, 1):
        row["id"] = f"M{index:05}"
    return rows


engine.candidates = _candidates

if __name__ == "__main__":
    engine.main()
