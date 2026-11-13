#!/usr/bin/env python3
"""Población de mutaciones de producción para el corte I09 de efectos."""

from pathlib import Path

import i07c_mutaciones as engine

engine.BASE = "d9853c8"
engine.OUTPUT = Path(__file__).resolve().parents[1] / "docs/validation/i09-mutations/population.json"
engine.DIRECTORIES = ("src/ir/effects.rs",)
engine.DEFAULT_ONLY = "production"
engine.TARGETED_TEST = None


TARGETS = (
    ("src/ir/effects.rs", "    candidates.sort_by(|a, b| (&a.0, &a.1, &a.2, &a.3).cmp(&(&b.0, &b.1, &b.2, &b.3)));", "    candidates.sort_by(|a,b| (&a.0, std::cmp::Reverse(a.1), &a.2, &a.3).cmp(&(&b.0, std::cmp::Reverse(b.1), &b.2, &b.3)));", "X01_prefer_more_edges"),
    ("src/ir/effects.rs", "                Op::DropBinding(local) => (", "                Op::DropLocal(local) => (", "explicit_drop_shape"),
    ("src/ir/effects.rs", "        Ty::Result(ok, err) => resource_effect(ok) | resource_effect(err),", "        Ty::Result(_ok, err) => resource_effect(err),", "result_variant_union"),
    ("src/ir/effects.rs", "                        resource_effect(&f.locals[local.0].ty)", "                        0", "droplocal_effect"),
    ("src/ir/effects.rs", "    candidates.sort_by(|a, b| (&a.0, &a.1, &a.2, &a.3).cmp(&(&b.0, &b.1, &b.2, &b.3)));", "    candidates.sort_by(|a,b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)).then_with(|| b.2.cmp(&a.2)).then_with(|| a.3.cmp(&b.3)));", "X02_reverse_offsets"),
    ("src/ir/effects.rs", "            super::model::ExitClass::Try | super::model::ExitClass::Fail\n", "            super::model::ExitClass::Try\n", "X09_remove_fail_exit"),
    ("src/ir/effects.rs", "        (Ty::Result(ok, _), Some(true)) => resource_effect(ok),", "        (Ty::Result(_, err), Some(true)) => resource_effect(err),", "variant_evidence_wrong_branch"),
    ("src/frontend/parse.rs", "                span: token.span,", "                span: Span { file: token.span.file.clone(), start: token.span.start, end: token.span.end + 1 },", "restriction_span_off_by_one"),
    ("src/ir/verify.rs", "        .any(|f| f.effects.is_some())", "        .any(|f| f.effects.is_none())", "skip_effect_metadata_verification"),
    ("src/diagnostic.rs", "        negative: \"fn Main(){MockRequest();}\",", "        negative: \"fn Main(){}\",", "implementation_limit_fixture_positive"),
    ("src/interp/machine.rs", "                            Intrinsic::DropData | Intrinsic::MockUnknown => Value::Unit,", "                            Intrinsic::DropData => Value::Unit, Intrinsic::MockUnknown => { self.fault(\"R0301\", &ins.span, vec![]); break; },", "mock_unknown_runtime_fault"),
)

SNIPPETS = (
    ("src/ir/effects.rs", "            if parameter.mode == super::model::ParameterMode::Consume", "            if true", "parameter_mode_owns_only_move"),
    ("src/ir/effects.rs", "            temp_spans[id.0] = Some(ins.span.clone());", "            temp_spans[id.0] = Some(Span { file: ins.span.file.clone(), start: ins.span.start, end: ins.span.start });", "temporary_source_span_is_retained"),
    ("src/ir/effects.rs", "                                let temp_span = arg\n                                    .and_then(|id| temp_spans.get(id.0).and_then(Option::as_ref));\n                                chain.push(Cause {\n                                    kind: CauseKind::Drop,\n                                    operation: CauseOperation::ValueDrop,\n                                    span: Some(ins.span.clone()),", "                                let temp_span = arg\n                                    .and_then(|id| temp_spans.get(id.0).and_then(Option::as_ref));\n                                chain.push(Cause {\n                                    kind: CauseKind::Drop,\n                                    operation: CauseOperation::ValueDrop,\n                                    span: temp_span.cloned().or_else(|| Some(ins.span.clone())),", "temporary_drop_span_covers_call"),
    ("src/ir/effects.rs", "                        } else if let Op::DropBinding(local) = ins.op {\n                            chain.push(Cause {\n                                kind: CauseKind::Drop,\n                                operation: CauseOperation::ValueDrop,\n                                span: Some(ins.span.clone()),", "                        } else if let Op::DropBinding(local) = ins.op {\n                            chain.push(Cause {\n                                kind: CauseKind::Drop,\n                                operation: CauseOperation::ValueDrop,\n                                span: Some(call_head(&ins.span, &f.source_text)),", "binding_drop_span_covers_call"),
)


base_candidates = engine.candidates


def candidates(base: str) -> list[dict]:
    population = [
        item for item in base_candidates(base)
        if not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "and"
            and "id.0 < temp_" in item["literal"]
        )
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "and"
            and item["literal"].strip() in {
                "&& id.0 < owned_temps.len()",
                "&& !matches!(",
            }
        )
        # temp_count is max(output TempId) + 1 and every valid temp reference
        # has a defining output. These mutations cannot change the predicate's
        # behavior on verified IR (the initialization is overwritten as well).
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "false"
            and item["literal"] == "    let mut owned_temps = vec![false; temp_count];"
        )
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "lt"
            and item["literal"] == "            && id.0 < owned_temps.len()"
        )
        # On an owned Inspect temporary, this only appends the same drop cause
        # already emitted by its DropValue. Cause chains deduplicate that row;
        # on a borrowed temporary the predicate remains false.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "true"
            and item["literal"] == "                                    if owned_temps.get(id.0) != Some(&true) {"
        )
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "lt"
            and "id.0 <= temp_" in item["replacement"]
        )
        # These predicates do not change the function-wide union: exit_info
        # bijection guarantees a DropLocal group, while return cleanup already
        # records any local that remains live at another exit.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "and"
            and "e.index < block.instructions.len()" in item["literal"]
        )
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "and"
            and "e.live_locals.contains(local)" in item["literal"]
        )
        # These guards only append a cause; per-effect causes are deduplicated
        # before diagnostics, while the witness summary is stored separately.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] in {"eq", "ne"}
            and "if known & bit" in item["literal"]
            and "unknown & bit" in item["literal"]
        )
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "and"
            and "e.index == b.instructions.len()" in item["literal"]
        )
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "lt"
            and "e.index <= block.instructions.len()" in item["replacement"]
        )
        # maybe_owned and gone are disjoint by the ExitInfo invariant; this
        # branch is unreachable for every verified exit.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "continue"
            and "exit.definitely_dropped.contains(local)" in item["literal"]
        )
        # Replacing && with || before a let-chain produces invalid Rust.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "and"
            and "if let Op::CallDirect { function, .. }" in item["literal"]
        )
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "and"
            and "&& let Some(target) = functions" in item["literal"]
        )
        # Inspect is verifier-constrained to exactly one argument, so changing
        # continue to break in its sole argument loop is observationally equal.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "continue"
            and (
                "owned_temps.get(id.0)" in item["literal"]
                or item["literal"] == "                                    else {\n                                        continue;\n                                    };"
            )
        )
        # These replacements turn closure syntax into invalid Rust syntax.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "or"
            and item["literal"] in {
                "                                (owned_temps.get(id.0) == Some(&true)).then(|| {",
                "                                    span: temp_span.cloned().or_else(|| Some(ins.span.clone())),",
            }
        )
        # Removing the only push that fixes the Vec element type is rejected
        # by inference before any effect behavior can be exercised.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "drop_state"
            and item["literal"] == "                finish.push(node);"
        )
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "drop_state"
            and item["literal"] == "        local.push(function_intrinsic_effect(f));"
        )
        # Removing these sole Vec element pushes prevents Rust type inference,
        # so they do not represent behavioral mutations.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "drop_state"
            and item["literal"] in {
                "                    edges[index].push(target);",
                "            reverse[*target].push(source);",
                "            errors.push(*d);",
            }
        )
        # These two comparator replacements are malformed Rust syntax.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "or"
            and item["literal"] in {
                "            .then_with(|| a.code.cmp(&b.code))",
                "            .then_with(|| a.subcause.cmp(&b.subcause))",
                "    let empty_bits = || vec![0_u64; word_count];",
                "            std::iter::from_fn(move || {",
            }
        )
        # The queue membership bit only avoids duplicate scheduling. Clearing
        # it can add work items but cannot change the fixed-point solution.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "true"
            and item["literal"].strip() == "let mut queued = vec![true; count];"
        )
        # Omitting the in-queue mark or writing false can only enqueue duplicate
        # block IDs. Every value change still schedules its successors, and each
        # duplicate applies the same monotone transfer, so the fixed point is
        # unchanged; these variants measure scheduling overhead only.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["literal"].strip() == "queued[*successor] = true;"
            and (
                item["operator"] == "drop_assignment"
                or (item["operator"] == "true" and item["replacement"].endswith("= false;"))
            )
        )
        # This assignment is inside the already-stable branch; `next == old`,
        # so omitting the copy cannot change the fixed point.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "drop_assignment"
            and item["literal"] == "                summaries = next;"
        )
        # Verified CallDirect targets are guaranteed to exist by verify_program;
        # this fallback cannot be reached by a source-compiled valid program.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "continue"
            and item["literal"] == "                else {\n                    continue;\n                };"
        )
        # `return_spans` walks only the AST body bounded by `span`, so every
        # returned span satisfies both containment clauses; replacing `&&`
        # with `||` cannot change the selected AST return.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "and"
            and "span.start <= r.start && r.end <= span.end" in item["literal"]
        )
        # Every verified lowered function originates in the source AST.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "continue"
            and item["literal"] == "        else {\n            continue;\n        };"
        )
        # The grammar rejects unrestricted mixed with another restriction.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "continue"
            and 'restriction.name == "unrestricted"' in item["literal"]
        )
        # Parameter.span starts at the parameter identifier (the grammar
        # requires the identifier first), so its start equals the narrower
        # token span used by parameter Entity IDs.
        and not (
            item["file"] == "src/ir/lower.rs"
            and item["operator"] == "drop_assignment"
            and item["literal"] == "            lower.locals[id.0].name_span = lower.name_spans.last().unwrap().clone();"
        )
        # Parsed return statements are nonempty and require a semicolon, so
        # both conjunction and disjunction evaluate true for every valid AST.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "and"
            and "span.end > span.start && source.as_bytes().get(span.end as usize - 1)" in item["literal"]
        )
        # The parser admits only the effect names declared in EFFECTS.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "continue"
            and item["literal"] == "            else {\n                continue;\n            };"
        )
        # The byte lookup in the same guard rejects an index equal to EOF.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "ge"
            and "exit.span.start >= f.source_text.len()" in item["literal"]
        )
        # Return statement spans are strictly inside their enclosing block span.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "le"
            and "span.start <= r.start && r.end <= span.end" in item["literal"]
        )
        # A depth-first stack processes a pushed node before queued siblings;
        # neighbours are deduplicated and already-seen nodes are not pushed.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "continue"
            and "std::mem::replace(&mut seen[node], true)" in item["literal"]
        )
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "continue"
            and "std::mem::replace(&mut seen[root], true)" in item["literal"]
        )
        # Removing this assignment makes Option<Span> ambiguous. The directed
        # variant sets a zero-width span so compilation succeeds and the tests
        # exercise the observable source span.
        and not (
            item["file"] == "src/ir/effects.rs"
            and item["operator"] == "drop_assignment"
            and item["literal"] == "            temp_spans[id.0] = Some(ins.span.clone());"
        )
    ]
    for file, literal, replacement, operator in TARGETS:
        lines = (engine.ROOT / file).read_text(encoding="utf-8").splitlines()
        found = [i for i, line in enumerate(lines, 1) if line + ("\n" if literal.endswith("\n") else "") == literal]
        if len(found) != 1:
            raise RuntimeError(f"expected one source site for {operator}: {len(found)}")
        population.append({
            "file": file,
            "line": found[0],
            "column": 1,
            "operator": operator,
            "literal": literal,
            "replacement": replacement,
            "site_count": 1,
            "id": f"M{len(population) + 1:05d}",
        })
    for file, literal, replacement, operator in SNIPPETS:
        source = (engine.ROOT / file).read_text(encoding="utf-8")
        if source.count(literal) != 1:
            raise RuntimeError(f"expected one source site for {operator}: {source.count(literal)}")
        offset = source.index(literal)
        line = source.count("\n", 0, offset) + 1
        line_start = source.rfind("\n", 0, offset) + 1
        population.append({
            "file": file,
            "line": line,
            "column": offset - line_start + 1,
            "operator": operator,
            "literal": literal,
            "replacement": replacement,
            "site_count": 1,
            "id": f"M{len(population) + 1:05d}",
        })
    for index, item in enumerate(population, 1):
        item["id"] = f"M{index:05d}"
    for item in population:
        if item.get("operator") == "implementation_limit_fixture_positive":
            item["historical"] = "I11a implementó la capacidad mock; se conserva solo como registro de mutación histórica."
    return population


engine.candidates = candidates


if __name__ == "__main__":
    engine.main()
