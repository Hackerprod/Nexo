"""Inverse mutations of the I07c-S6-COR region (one per emitter family).

Each mutant undoes one site of the cut (returns the outer-group span, or removes the helper's
loop, or bypasses the call-site index) in a disposable copy of the working tree, runs the
regressions of the cut with nextest (`--profile default`, every selected executable, first
failure kills) and restores the file, verifying the SHA-256 of the restored bytes. A mutant that
does not compile is NO COMPILA, never "killed".

Usage: python -B mutate.py --tree DIR --target DIR --out result.json [--only ID ...] [--check]
`--check` only verifies that every `before` text occurs exactly once in the tree.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

FILTER = ("test(i07c_group_primary) | test(i07c_structural_invariants) | "
          "test(written_calls_resolve) | test(witness_search_and_enumerator) | "
          "test(expression_primary) | test(i07c_s5b_selector)")

HELPER_LOOP = "    while let Expr::Group(inner, _) = current {\n        current = inner;\n    }\n    current\n}"

# (id, family, file, before, after)
MUTANTS = [
    ("T1", "helper: no group removal", "src/frontend/parse.rs", HELPER_LOOP, "    current\n}"),
    ("T2", "types for_iteration", "src/check/types.rs",
     "Operation::TypesCheck,\n                    Self::primary_span(iterable),\n                );\n                error.feature_id = Some(\"for_iteration\".into());",
     "Operation::TypesCheck,\n                    iterable.span().clone(),\n                );\n                error.feature_id = Some(\"for_iteration\".into());"),
    ("T3", "types non_callable_target", "src/check/types.rs",
     "Operation::CallBind,\n                        Self::primary_span(target),",
     "Operation::CallBind,\n                        target.span().clone(),"),
    ("T4", "types closure bind_arguments target", "src/check/types.rs",
     "names::bind_arguments(params, &Self::primary_span(target), args)?",
     "names::bind_arguments(params, target.span(), args)?"),
    ("T5", "types primary_span delegation", "src/check/types.rs",
     "        crate::frontend::parse::expression_primary_span(e)\n    }",
     "        e.span().clone()\n    }"),
    ("N1", "names missing_argument target", "src/check/names.rs",
     "Self::bind(&sig, &primary, args)?", "Self::bind(&sig, target.span(), args)?"),
    ("N2", "names None non_callable", "src/check/names.rs",
     "argument_error(\"non_callable_target\", primary, None)",
     "argument_error(\"non_callable_target\", target.span().clone(), None)"),
    ("N3", "names Drop projection", "src/check/names.rs",
     "expression_primary_span(&arg.value),\n            );\n            d.feature_id",
     "arg.value.span().clone(),\n            );\n            d.feature_id"),
    ("N4", "names positional_after_named", "src/check/names.rs",
     "\"positional_after_named\",\n                        expression_primary_span(&a.value),",
     "\"positional_after_named\",\n                        a.value.span().clone(),"),
    ("N5", "names excess_positional_argument", "src/check/names.rs",
     "\"excess_positional_argument\",\n                        expression_primary_span(&a.value),",
     "\"excess_positional_argument\",\n                        a.value.span().clone(),"),
    ("N6", "names mode mismatch value", "src/check/names.rs",
     "                    _ => expression_primary_span(&a.value),",
     "                    _ => a.value.span().clone(),"),
    ("N7", "names mode mismatch prefix value", "src/check/names.rs",
     "Expr::Unary { value, .. } if actual != Mode::Read => {\n                        expression_primary_span(value)",
     "Expr::Unary { value, .. } if actual != Mode::Read => {\n                        value.span().clone()"),
    ("P1", "parse match move partial_transfer", "src/frontend/parse.rs",
     "self.feature(\"partial_transfer\", expression_primary_span(&scrutinee))",
     "self.feature(\"partial_transfer\", scrutinee.span().clone())"),
    ("P2", "parse move partial_transfer", "src/frontend/parse.rs",
     "self.feature(\"partial_transfer\", expression_primary_span(&val))",
     "self.feature(\"partial_transfer\", val.span().clone())"),
    ("L1", "lower NX-E1104 value", "src/ir/lower.rs",
     "}) => parse::expression_primary_span(value),", "}) => value.span().clone(),"),
    ("L2", "lower first_f5_capability place", "src/ir/lower.rs",
     "first = Some((place.span().start, parse::expression_primary_span(place)));",
     "first = Some((place.span().start, place.span().clone()));"),
    ("L3", "lower check_place_access place", "src/ir/lower.rs",
     "            parse::expression_primary_span(root)\n        } else {\n            parse::expression_primary_span(place)",
     "            parse::expression_primary_span(root)\n        } else {\n            place.span().clone()"),
    ("L5", "lower immutable_write root", "src/ir/lower.rs",
     "primary_span: Some(parse::expression_primary_span(root)),",
     "primary_span: Some(root.span().clone()),"),
    ("L6", "lower Drop((a)) primary", "src/ir/lower.rs",
     "let primary = parse::expression_primary_span(&args[0].value);",
     "let primary = args[0].value.span().clone();"),
    ("L7", "lower closure mode NX-E1107", "src/ir/lower.rs",
     "diagnostic::Operation::OwnershipCheck,\n                                    parse::expression_primary_span(target),",
     "diagnostic::Operation::OwnershipCheck,\n                                    target.span().clone(),"),
    ("L8", "lower rooted match-index origin", "src/ir/lower.rs",
     "let origin = parse::expression_primary_span(&selection.scrutinee);",
     "let origin = selection.scrutinee.span().clone();"),
    ("L9", "lower named closure receiver", "src/ir/lower.rs",
     "let primary = parse::expression_primary_span(target);\n            lower.check_available(local, primary.clone())?;",
     "let primary = target.span().clone();\n            lower.check_available(local, primary.clone())?;"),
    ("L13", "lower consume value primary", "src/ir/lower.rs",
     "let primary = parse::expression_primary_span(value);\n                self.check_available(id, primary.clone())?;",
     "let primary = value.span().clone();\n                self.check_available(id, primary.clone())?;"),
    ("M1", "lower_match consume branch", "src/ir/lower_match.rs",
     "let primary = parse::expression_primary_span(&selection.scrutinee);\n                self.check_available(id, primary.clone())?;\n                if self.locals[id.0]\n                    .parameter_mode\n                    .is_some_and(|mode| mode != ParameterMode::Consume)",
     "let primary = selection.scrutinee.span().clone();\n                self.check_available(id, primary.clone())?;\n                if self.locals[id.0]\n                    .parameter_mode\n                    .is_some_and(|mode| mode != ParameterMode::Consume)"),
    ("E1", "effects intrinsic first step", "src/ir/effects.rs",
     "Some(sites.head(&ins.span, &f.source_text))\n                                } else {",
     "Some(call_head(&ins.span, &f.source_text))\n                                } else {"),
    ("E2", "effects local primary fallback", "src/ir/effects.rs",
     "                        .unwrap_or_else(|| sites.head(&ins.span, &f.source_text))\n                } else if let Op::DropBinding",
     "                        .unwrap_or_else(|| call_head(&ins.span, &f.source_text))\n                } else if let Op::DropBinding"),
    ("E9", "effects dropped temporary producer", "src/ir/effects.rs",
     ".and_then(|span| sites.0.get(span)),",
     ".and_then(|_| Option::<&Span>::None),"),
    ("E3", "effects transitive first step", "src/ir/effects.rs",
     "                            sites.head(&instruction.span, &function.source_text)\n                        }),",
     "                            call_head(&instruction.span, &function.source_text)\n                        }),"),
    ("E4", "effects guard call", "src/ir/effects.rs",
     "let primary = call_sites.head(&instruction.span, &f.source_text);",
     "let primary = call_head(&instruction.span, &f.source_text);"),
    ("E5", "effects temporary drop head", "src/ir/effects.rs",
     "primary: sites.head(span, &f.source_text),\n                    operation: CauseOperation::ValueDrop,",
     "primary: call_head(span, &f.source_text),\n                    operation: CauseOperation::ValueDrop,"),
    ("E6", "effects index without calls", "src/ir/effects.rs",
     "                    if matches!(\n                        crate::frontend::parse::expression_primary(target),\n                        Expr::Name(..)\n                    ) {",
     "                    if false {"),
    ("E7", "effects index without closure bodies", "src/ir/effects.rs",
     "        for closure in super::lower::collect_closures(ast) {\n            scope_sites(&closure.body, &mut Vec::new(), &mut calls);\n        }",
     "        let _ = super::lower::collect_closures(ast);"),
    ("E8", "effects oracle step", "src/ir/effects.rs",
     "span: Some(sites.head(&instruction.span, &function.source_text)),\n                    entity: Some(Entity {\n                        kind: EntityKind::Function,\n                        id: format!(\"function:{declaration}\"),",
     "span: Some(call_head(&instruction.span, &function.source_text)),\n                    entity: Some(Entity {\n                        kind: EntityKind::Function,\n                        id: format!(\"function:{declaration}\"),"),
    ("S1", "S5b.1 selector of a failed base inherits the base taint", "src/check/types.rs",
     "deferred_element: false,\n            };\n            let selector_fact = checker.graph.child_fact(\n                visit,\n                span_key(index.span()),\n                FactRole::Selector,\n                FactInheritance::Empty,\n                FactInheritance::Inherit,",
     "deferred_element: false,\n            };\n            let selector_fact = checker.graph.child_fact(\n                visit,\n                span_key(index.span()),\n                FactRole::Selector,\n                FactInheritance::Inherit,\n                FactInheritance::Inherit,"),
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--tree", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--only", nargs="*")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    tree = Path(args.tree)
    env = dict(os.environ, CARGO_TARGET_DIR=args.target, NEXO_MUTANT="1")
    results = []
    for ident, family, rel, before, after in MUTANTS:
        if args.only and ident not in args.only:
            continue
        path = tree / rel
        original = path.read_bytes()
        text = original.decode("utf-8")
        crlf = "\r\n" in text
        norm = text.replace("\r\n", "\n")
        count = norm.count(before)
        record = {"id": ident, "family": family, "file": rel, "occurrences": count,
                  "sha_initial": sha(original)}
        if count != 1:
            record["verdict"] = "PATCH NOT APPLICABLE"
            results.append(record)
            print(ident, record["verdict"], count, flush=True)
            continue
        if args.check:
            record["verdict"] = "ok"
            results.append(record)
            continue
        mutated = norm.replace(before, after, 1)
        if crlf:
            mutated = mutated.replace("\n", "\r\n")
        record["sha_mutated"] = sha(mutated.encode("utf-8"))
        path.write_bytes(mutated.encode("utf-8"))
        try:
            done = subprocess.run(
                ["cargo", "nextest", "run", "--profile", "default", "--offline", "-E", FILTER],
                cwd=tree, env=env, capture_output=True, text=True, timeout=3000)
            output = done.stdout + done.stderr
            failed = [line.split("]", 1)[1].strip() for line in output.splitlines()
                      if line.strip().startswith("FAIL [")]
            if "could not compile" in output or "error[E" in output:
                record["verdict"] = "NO COMPILA"
                record["detail"] = [line for line in output.splitlines() if "error" in line][:3]
            elif done.returncode != 0 and failed:
                record["verdict"] = "MUERTO"
                record["killed_by"] = list(dict.fromkeys(failed))[:5]
            elif done.returncode == 0:
                record["verdict"] = "SUPERVIVIENTE (selected tests)"
            else:
                record["verdict"] = "INCONCLUSO"
                record["detail"] = output[-600:]
        finally:
            path.write_bytes(original)
            record["sha_restored"] = sha(path.read_bytes())
            assert record["sha_restored"] == record["sha_initial"], "restore failed"
        results.append(record)
        print(ident, record["verdict"], record.get("killed_by", [""])[0:1], flush=True)
        Path(args.out).write_text(json.dumps(results, indent=1), encoding="utf-8")
    Path(args.out).write_text(json.dumps(results, indent=1), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
