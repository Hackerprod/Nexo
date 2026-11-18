"""Run isolated S3 inverse mutations in an already validated disposable copy."""

import argparse
import hashlib
import os
from pathlib import Path
import subprocess


def digest(data):
    return hashlib.sha256(data).hexdigest()


def replace_once(source, old, new):
    assert source.count(old) == 1, (old[:80], source.count(old))
    return source.replace(old, new)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("disposable_tree", type=Path)
    parser.add_argument("--only", help="comma-separated mutation names")
    args = parser.parse_args()
    tree = args.disposable_tree.resolve(strict=True)
    path = tree / "src/check/types.rs"
    original = path.read_bytes()
    baseline = digest(original)
    env = os.environ.copy()
    env["CARGO_BUILD_JOBS"] = "1"
    env["CARGO_TARGET_DIR"] = "D:/nexo-work/target-i07c-c"
    env["TEMP"] = env["TMP"] = "D:\\nexo-work\\tmp"
    cargo = ["cargo", "test", "--locked", "--offline"]
    cursor_test = cargo + ["--lib", "solver_child_frames_keep_each_cursor_boundary"]
    mutations = []
    for site in ("RequireUnion", "UnionArrayA", "UnionArrayB", "UnionPayloadA", "UnionPayloadB"):
        old = b"request.visit = VisitCursor::default();"
        new = f"if request.site != SolverBindSite::{site} {{ request.visit = VisitCursor::default(); }}".encode()
        mutations.append(("cursor_" + site, old, new, cursor_test, {}))

    mutations.append((
        "recursive_unify_array",
        b"""frames.push(Frame::UnifyArrayDone {
                                        key,
                                        before_inner,
                                        pending_before,
                                        expected,
                                        found,
                                    });
                                    frames.push(Frame::Enter(SolverRequest::Unify(child)));""",
        b"""frames.push(Frame::UnifyArrayDone {
                                    key,
                                    before_inner,
                                    pending_before,
                                    expected,
                                    found,
                                });
                                outcome = self.unify_with_expected_origin(
                                    &child.visit, child.left, child.right,
                                    child.left_origin, child.right_origin,
                                    child.found, child.policy,
                                ).map(SolverValue::Type);""",
        cargo + ["--lib", "s3_solver_unify_depth_child"],
        {"I07C_S3_DIRECT_UNIFY": "1"},
    ))
    mutations.append((
        "unify_pair_charge",
        b"""SolverRequest::Unify(request) => {
                            if let Err(error) = self.charge_solver(&request.found) {
                                outcome = Err(error);
                                continue;
                            }""",
        b"SolverRequest::Unify(request) => {",
        cargo + ["--lib", "cached_arrays_charge_every_logical_pair_at_the_reduced_limit"], {},
    ))
    mutations.append((
        "deferred_export",
        b"""Type::Deferred(id) => {
                        let (resolved, _) = self.concrete(id)?;
                        values.push(syntax_type(&resolved)?);
                    }""",
        b"Type::Deferred(_) => return None,",
        cargo + ["--test", "i07c_pattern_deferred", "pattern_local_exports_closed_array_element"], {},
    ))
    mutations.append((
        "m00073_current_site_m00482",
        b"self.graph.concrete_plan.insert(site, concrete);",
        b"let _ = (site, concrete);",
        cargo + ["--test", "i07c_mutation_regressions", "independent_numeric_error_has_no_inferred_expected_origin"], {},
    ))
    mutations.append((
        "m00017_analog_not_original",
        b"if value != first {",
        b"if value == first {",
        cargo + ["--test", "i07c_mutation_regressions", "array_assignment_keeps_first_element_origin_after_conflict"], {},
    ))

    for name, old, new, command, additions in mutations:
        if args.only and name not in args.only.split(","):
            continue
        try:
            mutated = replace_once(original, old, new)
            path.write_bytes(mutated)
            case_env = env | additions
            result = subprocess.run(command, cwd=tree, env=case_env,
                                    capture_output=True, text=True, timeout=240)
            print(f"{name} baseline={baseline} mutant={digest(mutated)} exit={result.returncode}", flush=True)
            assert "could not compile" not in result.stderr, (name, result.stderr[-1500:])
            if name == "recursive_unify_array":
                assert "STATUS_STACK_OVERFLOW" in result.stderr, (name, result.stdout[-1500:], result.stderr[-1500:])
            else:
                assert "FAILED" in result.stdout, (name, result.stdout[-1500:], result.stderr[-1500:])
            if result.returncode == 0:
                print("mutation survived", result.stdout[-1500:], result.stderr[-1500:], sep="\n")
                raise SystemExit(1)
        finally:
            path.write_bytes(original)
            restored = digest(path.read_bytes())
            print(f"{name} restored={restored}", flush=True)
            assert restored == baseline


if __name__ == "__main__":
    main()
