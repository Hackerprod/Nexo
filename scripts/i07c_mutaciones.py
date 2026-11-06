#!/usr/bin/env python3
"""Enumera y ejecuta la población cerrada de mutaciones de I07c.

Operadores fijos: ==/!=, </<=, &&/||, true/false, supresión de
record_error, inserciones y asignaciones de estado, intercambio de ordered(0/1) en
primary_span y unify, continue/break, y += 1/-= 1 de pasos. Se recorren
solo tokens de código en líneas nuevas o modificadas respecto de b506d7d.
"""

from __future__ import annotations

import argparse
import contextlib
import functools
import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BASE = "b506d7d"
OUTPUT = ROOT / "docs/validation/i07c-mutations/population.json"
DIRECTORIES = ("src/check/", "src/ir/")
DEFAULT_ONLY = "all"
TARGETED_TEST: str | None = None
SKIP_FILES: set[str] = set()

TOKENS = (
    ("eq", re.compile(r"(?<![=!<>])==(?![=>])"), "!="),
    ("ne", re.compile(r"(?<![=!<>])!=(?![=>])"), "=="),
    # rustfmt separates comparison operators from both operands. Generic
    # delimiters (Vec<T>, for<'a>, etc.) are punctuation, not comparisons.
    ("lt", re.compile(r"(?<=\s)<(?=\s)"), "<="),
    ("le", re.compile(r"(?<=\s)<=(?=\s)"), "<"),
    ("and", re.compile(r"&&"), "||"),
    ("or", re.compile(r"\|\|"), "&&"),
    ("true", re.compile(r"\btrue\b"), "false"),
    ("false", re.compile(r"\bfalse\b"), "true"),
    ("continue", re.compile(r"\bcontinue\b"), "break"),
    ("break", re.compile(r"\bbreak\b"), "continue"),
    ("increment", re.compile(r"\+=\s*1\b"), "-= 1"),
    ("decrement", re.compile(r"-=\s*1\b"), "+= 1"),
)


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True,
                          capture_output=True, text=True, encoding="utf-8").stdout


def changed_lines(base: str) -> dict[str, set[int]]:
    paths = tuple(path.removesuffix("/") for path in DIRECTORIES)
    diff = git("diff", "--unified=0", base, "--", *paths)
    changed: dict[str, set[int]] = {}
    path = ""
    for line in diff.splitlines():
        if line.startswith("+++ b/"):
            path = line[6:]
        elif line.startswith("@@"):
            match = re.search(r"\+(\d+)(?:,(\d+))?", line)
            if match and path:
                start = int(match.group(1))
                count = int(match.group(2) or 1)
                changed.setdefault(path, set()).update(range(start, start + count))
    for path in git("ls-files", "--others", "--exclude-standard", "--", *paths).splitlines():
        if path.endswith(".rs"):
            changed[path] = set(range(1, len((ROOT / path).read_text(encoding="utf-8").splitlines()) + 1))
    return {path: lines for path, lines in changed.items()
            if path.endswith(".rs") and path.startswith(DIRECTORIES)
            and path not in SKIP_FILES}


def code_mask(source: str) -> str:
    """Conserva posiciones y saltos; oculta comentarios y literales Rust."""
    out = list(source)
    i = 0
    state = "code"
    depth = 0
    raw_hashes = 0
    while i < len(source):
        ch = source[i]
        next_ch = source[i + 1] if i + 1 < len(source) else ""
        if state == "code":
            if ch == "/" and next_ch == "/":
                state = "line"
                out[i] = out[i + 1] = " "
                i += 2
                continue
            if ch == "/" and next_ch == "*":
                state = "block"
                depth = 1
                out[i] = out[i + 1] = " "
                i += 2
                continue
            raw = re.match(r'(?:b)?r(#{0,16})"', source[i:])
            if raw:
                raw_hashes = len(raw.group(1))
                state = "raw"
                for j in range(i, i + len(raw.group(0))):
                    out[j] = " "
                i += len(raw.group(0))
                continue
            if ch == '"':
                state = "string"
                out[i] = " "
                i += 1
                continue
            if ch == "'" and i + 2 < len(source) and source[i + 2] == "'":
                out[i:i + 3] = "   "
                i += 3
                continue
        elif state == "line":
            if ch == "\n":
                state = "code"
            else:
                out[i] = " "
            i += 1
            continue
        elif state == "block":
            if ch == "/" and next_ch == "*":
                depth += 1
                out[i] = out[i + 1] = " "
                i += 2
                continue
            if ch == "*" and next_ch == "/":
                depth -= 1
                out[i] = out[i + 1] = " "
                i += 2
                if depth == 0:
                    state = "code"
                continue
            if ch != "\n":
                out[i] = " "
            i += 1
            continue
        elif state == "string":
            if ch == "\\" and next_ch:
                out[i] = out[i + 1] = " "
                i += 2
                continue
            if ch == '"':
                state = "code"
            out[i] = " "
            i += 1
            continue
        elif state == "raw":
            ending = '"' + "#" * raw_hashes
            if source.startswith(ending, i):
                for j in range(i, i + len(ending)):
                    out[j] = " "
                i += len(ending)
                state = "code"
                continue
            if ch != "\n":
                out[i] = " "
            i += 1
            continue
        i += 1
    return "".join(out)


def unique_literal(source: str, start: int, end: int, replacement: str) -> tuple[str, str]:
    left = source.rfind("\n", 0, start) + 1
    right = source.find("\n", end)
    if right < 0:
        right = len(source)
    while source.count(source[left:right]) != 1:
        prev = source.rfind("\n", 0, max(0, left - 1)) + 1 if left else 0
        nxt = source.find("\n", right + 1)
        left = prev
        right = len(source) if nxt < 0 else nxt
        if left == 0 and right == len(source):
            break
    literal = source[left:right]
    assert source.count(literal) == 1
    return literal, literal[:start - left] + replacement + literal[end - left:]


def candidates(base: str) -> list[dict]:
    result: list[dict] = []
    changed = changed_lines(base)
    for path, lines in sorted(changed.items()):
        source = (ROOT / path).read_text(encoding="utf-8")
        mask = code_mask(source)
        line_starts = [0] + [i + 1 for i, char in enumerate(source) if char == "\n"]
        for line in sorted(lines):
            if line > len(line_starts):
                continue
            begin = line_starts[line - 1]
            stop = line_starts[line] - 1 if line < len(line_starts) else len(source)
            visible = mask[begin:stop]
            matches: list[tuple[int, int, str, str]] = []
            for name, pattern, replacement in TOKENS:
                matches.extend((begin + m.start(), begin + m.end(), name, replacement)
                               for m in pattern.finditer(visible))
            for match in re.finditer(r"self\.record_error\([^;]*\);", visible):
                matches.append((begin + match.start(), begin + match.end(), "drop_record", ""))
            for match in re.finditer(
                r"\b[A-Za-z_][\w.()&\[\],]*\.(?:insert|push)\([^;]*\);", visible
            ):
                matches.append((begin + match.start(), begin + match.end(), "drop_state", ""))
            assignment = re.fullmatch(
                r"\s*(?!let\b|type\b|const\b|static\b|pub\b|return\b)"
                r"(?:self\.)?[A-Za-z_][\w.()&\[\]*,\s]*\s=\s[^;]+;\s*",
                visible,
            )
            if assignment:
                matches.append((begin + assignment.start(), begin + assignment.end(),
                                "drop_assignment", ""))
            for match in re.finditer(r"(?:Self::)?primary_span\(ordered\(([01])\)\)", visible):
                old = match.group(1)
                new = "1" if old == "0" else "0"
                matches.append((begin + match.start(1), begin + match.end(1),
                                "swap_primary", new))
            for start, end, operator, replacement in sorted(set(matches)):
                literal, mutated = unique_literal(source, start, end, replacement)
                result.append({"file": path, "line": line,
                               "column": start - begin + 1, "operator": operator,
                               "literal": literal, "replacement": mutated,
                               "site_count": 1})
        for call in re.finditer(r"\bself\.unify\s*\(", mask):
            argument_start = call.end()
            nesting = 1
            comma_positions = []
            pos = argument_start
            while pos < len(mask) and nesting:
                if mask[pos] in "([{":
                    nesting += 1
                elif mask[pos] in ")]}":
                    nesting -= 1
                elif mask[pos] == "," and nesting == 1:
                    comma_positions.append(pos)
                pos += 1
            if len(comma_positions) < 2:
                continue
            first_end, second_end = comma_positions[:2]
            affected_lines = set(range(source.count("\n", 0, call.start()) + 1,
                                       source.count("\n", 0, second_end) + 2))
            if not (affected_lines & lines):
                continue
            first = source[argument_start:first_end].strip()
            second = source[first_end + 1:second_end].strip()
            if not first or not second or first == second:
                continue
            original = source[argument_start:second_end]
            prefix = re.match(r"\s*", original).group(0)
            replacement = prefix + second + ", " + first
            literal, mutated = unique_literal(source, argument_start, second_end, replacement)
            result.append({"file": path, "line": source.count("\n", 0, call.start()) + 1,
                           "column": call.start() - (source.rfind("\n", 0, call.start()) + 1) + 1,
                           "operator": "swap_unify", "literal": literal,
                           "replacement": mutated, "site_count": 1})
    result = [item for item in result if not is_test_only(item)]
    result.sort(key=lambda item: (item["file"], item["line"], item["column"], item["operator"]))
    for index, item in enumerate(result, 1):
        item["id"] = f"M{index:05}"
    assert len({(item["file"], item["literal"], item["replacement"]) for item in result}) == len(result)
    assert all(not is_test_only(item) for item in result)
    return result


def enumerate_population(base: str, write: bool = True) -> list[dict]:
    population = candidates(base)
    if write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps({"base": base, "operators": [name for name, *_ in TOKENS]
                                      + ["drop_record", "drop_state", "drop_assignment", "swap_primary", "swap_unify"],
                                      "mutations": population}, ensure_ascii=False, indent=2) + "\n",
                          encoding="utf-8", newline="\n")
    return population


@functools.lru_cache(maxsize=None)
def test_only_regions(path: str) -> tuple[tuple[int, int], ...]:
    if path.endswith(("_tests.rs", "/tests.rs", "_campaign.rs")):
        included_campaign = path.endswith("_campaign.rs")
        parent = ROOT / ("src/check/types.rs" if included_campaign else
                         "src/check.rs" if path.startswith("src/check/") else "src/ir/mod.rs")
        module = Path(path).stem
        declaration = (re.compile(r'#\[cfg\(test\)\]\s*include!\("' + re.escape(Path(path).name) + r'"\);')
                       if included_campaign else
                       re.compile(r"#\[cfg\(test\)\]\s*mod\s+" + re.escape(module) + r"\s*;"))
        assert declaration.search(parent.read_text(encoding="utf-8")), (
            f"{path}: the parent does not compile this module exclusively under cfg(test)"
        )
        return ((0, (ROOT / path).stat().st_size + 1),)
    source = (ROOT / path).read_text(encoding="utf-8")
    masked = code_mask(source)
    regions = []
    for attribute in re.finditer(r"#\[cfg\(test\)\]", masked):
        start = attribute.end()
        while True:
            start = re.search(r"\S", masked[start:]).start() + start
            extra = re.match(r"#\[[^\]]+\]", masked[start:])
            if extra is None:
                break
            start += extra.end()
        # A cfg attribute gates one Rust item or statement. Function/module
        # bodies and cfg-gated if/block expressions close at their outer brace;
        # declarations and let statements close at their outer semicolon.
        head = masked[start:start + 100]
        braced = bool(re.match(r"(?:pub(?:\([^)]*\))?\s+)?(?:fn|mod|impl|struct|enum|trait|if|loop|while|match)\b|\{", head))
        parens = brackets = braces = 0
        end = None
        for pos in range(start, len(masked)):
            ch = masked[pos]
            if ch == "(":
                parens += 1
            elif ch == ")":
                parens -= 1
            elif ch == "[":
                brackets += 1
            elif ch == "]":
                brackets -= 1
            elif ch == "{" and parens == brackets == 0:
                braces += 1
            elif ch == "}" and parens == brackets == 0:
                braces -= 1
                if braced and braces == 0:
                    end = pos + 1
                    break
            elif ch == ";" and parens == brackets == braces == 0:
                end = pos + 1
                break
            elif ch == "," and not braced and parens == brackets == braces == 0:
                # cfg(test) also gates struct fields and match arms, whose
                # item delimiter is a comma rather than a semicolon.
                end = pos + 1
                break
            assert parens >= 0 and brackets >= 0 and braces >= 0, path
        assert end is not None, f"{path}: unclosed cfg(test) item"
        regions.append((attribute.start(), end))
    return tuple(regions)


def is_test_only(item: dict) -> bool:
    path = item["file"]
    offset = source_line_offsets(path)[item["line"] - 1]
    return any(start <= offset < end for start, end in test_only_regions(path))


@functools.lru_cache(maxsize=None)
def source_line_offsets(path: str) -> tuple[int, ...]:
    source = (ROOT / path).read_text(encoding="utf-8")
    return (0,) + tuple(i + 1 for i, char in enumerate(source) if char == "\n")


def run_limited(command: list[str], tree: Path, env: dict, seconds: int):
    """Devuelve None al agotar plazo y termina solo el subárbol creado."""
    options = ({"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP}
               if os.name == "nt" else {"start_new_session": True})
    with tempfile.TemporaryFile() as output_file:
        process = subprocess.Popen(command, cwd=tree, env=env,
                                   stdout=output_file, stderr=subprocess.STDOUT, **options)
        try:
            process.wait(timeout=seconds)
        except subprocess.TimeoutExpired:
            if os.name == "nt":
                subprocess.run(["taskkill", "/T", "/F", "/PID", str(process.pid)],
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
            else:
                import signal
                os.killpg(process.pid, signal.SIGKILL)
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
            return None
        output_file.seek(0)
        return process.returncode, output_file.read()


def assert_disposable_inputs(tree: Path) -> None:
    """A reused tree must contain every input consumed by the complete suite."""
    def files(base: Path) -> dict[str, str]:
        names = {}
        for folder in ("src", "tests", "corpus", "scripts", "docs/adr"):
            for path in (base / folder).rglob("*"):
                relative = path.relative_to(base)
                if not path.is_file() or any(part in {"target", "__pycache__"}
                                             for part in relative.parts):
                    continue
                names[str(relative)] = hashlib.sha256(path.read_bytes()).hexdigest()
        for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
            names[name] = hashlib.sha256((base / name).read_bytes()).hexdigest()
        return names

    root_files = files(ROOT)
    copy_files = files(tree)
    mismatches = sorted(name for name in root_files.keys() | copy_files.keys()
                        if root_files.get(name) != copy_files.get(name))
    assert not mismatches, f"copia desechable desfasada: {mismatches[:25]}"


def run_population(population: list[dict], debug_sample: int, fast_only: bool,
                   output_name: str, resume: bool, only: str, full_suite: bool,
                   reuse_tree: Path | None = None, target_dir: Path | None = None,
                   full_suite_timeout: int = 900, mutation_profile: str = "release") -> None:
    protected = {path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
                 for path in {item["file"] for item in population}}
    (ROOT / ".scratch").mkdir(exist_ok=True)
    temporary_context = (contextlib.nullcontext(str(reuse_tree)) if reuse_tree else
                         tempfile.TemporaryDirectory(prefix="i07c-mutations-", dir=ROOT / ".scratch"))
    with temporary_context as temporary:
        tree = Path(temporary) / "tree"
        if reuse_tree:
            target = target_dir or Path(temporary) / "target"
            assert tree.is_dir() and target.is_dir()
            assert protected == {path: hashlib.sha256((tree / path).read_bytes()).hexdigest()
                                 for path in protected}, "copia desechable con sustitución activa"
            assert_disposable_inputs(tree)
            for path in protected:
                (tree / path).touch()
        else:
            tree.mkdir()
            for folder in ("src", "tests", "corpus", "docs", "scripts"):
                shutil.copytree(ROOT / folder, tree / folder,
                                ignore=shutil.ignore_patterns("target", "__pycache__"))
            for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
                shutil.copy2(ROOT / name, tree / name)
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str((target_dir or Path(temporary) / "target").resolve())
        env["CARGO_INCREMENTAL"] = "1"
        # Two independent shards share 24 logical CPUs; cap each test binary's
        # worker pool while letting the complete suite run in parallel.
        env["RUST_TEST_THREADS"] = "8"
        profiles = (["debug"] if mutation_profile == "debug" else
                    (["release", "debug"] if mutation_profile == "both" else ["release"]))
        if only != "test":
            for profile in profiles:
                command = ["cargo", "test", "--locked", "--offline", "--lib", "--no-run"]
                if profile == "release":
                    command.append("--release")
                prepared = subprocess.run(command, cwd=tree, env=env,
                                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                if prepared.returncode:
                    raise RuntimeError(prepared.stdout.decode("utf-8", "replace"))
        output = OUTPUT.parent / output_name
        completed: dict[str, dict] = {}
        if resume and output.exists():
            entries = {item["id"]: item for item in
                       json.loads((OUTPUT.parent / "population.json").read_text(encoding="utf-8"))["mutations"]}
            for line in output.read_text(encoding="utf-8").splitlines():
                row = json.loads(line)
                item = entries[row["id"]]
                assert row["mutation_sha256"] == hashlib.sha256(json.dumps(item, sort_keys=True).encode()).hexdigest(), "results no corresponden a la población"
                assert row["id"] not in completed, "resultado duplicado"
                completed[row["id"]] = row
        targeted_baseline: dict[str, float] = {}
        if TARGETED_TEST:
            for profile in (["release", "debug"] if debug_sample else ["release"]):
                baseline_command = ["cargo", "test", "--locked", "--offline"]
                if profile == "release":
                    baseline_command.append("--release")
                baseline_command.extend(["--test", TARGETED_TEST, "--", "--quiet"])
                built = run_limited(baseline_command, tree, env, 180)
                assert built is not None and built[0] == 0, "prueba dirigida base fallida"
                start = time.monotonic()
                measured = run_limited(baseline_command, tree, env, 180)
                assert measured is not None and measured[0] == 0, "prueba dirigida base fallida"
                targeted_baseline[profile] = time.monotonic() - start
        effects_baseline: dict[str, float] = {}
        if only == "production":
            for profile in profiles:
                baseline_command = ["cargo", "test", "--locked", "--offline", "--lib"]
                if profile == "release":
                    baseline_command.append("--release")
                baseline_command.extend(["ir::effects::tests", "--", "--quiet"])
                start = time.monotonic()
                baseline = run_limited(baseline_command, tree, env, 180)
                elapsed = time.monotonic() - start
                assert baseline is not None and baseline[0] == 0, "pruebas base de efectos fallidas"
                effects_baseline[profile] = elapsed
        with output.open("a" if resume else "w", encoding="utf-8", newline="\n") as log:
            for index, item in enumerate(population):
                if item["id"] in completed:
                    continue
                if only == "test" and not is_test_only(item):
                    continue
                if only == "production" and is_test_only(item):
                    continue
                path = tree / item["file"]
                original = path.read_bytes()
                source = original.decode("utf-8")
                assert source.count(item["literal"]) == 1, item["id"]
                path.write_text(source.replace(item["literal"], item["replacement"], 1),
                                encoding="utf-8", newline="\n")
                try:
                    results = {}
                    witnesses = {}
                    path_name = item["file"]
                    test_only = is_test_only(item)
                    if path_name.startswith("src/frontend/"):
                        filters = ["frontend::"]
                    elif path_name.endswith("machine.rs"):
                        filters = ["interp::machine::tests"]
                    elif path_name.endswith("diagnostic.rs"):
                        filters = ["diagnostic::tests"]
                    elif path_name.endswith("effects.rs"):
                        filters = ["ir::effects::tests"]
                    elif path_name == "src/ir/verify.rs":
                        filters = ["ir::effects::tests"]
                    elif path_name.endswith("i07a_tests.rs"):
                        filters = ["i07a_tests"]
                    elif path_name.endswith("i07b_tests.rs"):
                        filters = ["i07b_tests"]
                    elif path_name.endswith("check/tests.rs"):
                        filters = ["check::tests"]
                    elif path_name.endswith("i07c_quota_campaign.rs"):
                        filters = ["quota_tests"]
                    elif path_name.endswith("i07c_tests.rs"):
                        filters = ["i07c_tests"]
                    elif path_name.endswith("types.rs"):
                        filters = ["check::tests", "i07c_tests"]
                    elif path_name.startswith("src/ir/"):
                        filters = ["i07a_tests", "i07b_tests", "i07c_tests"]
                    else:
                        filters = ["check::"]
                    mutant_profiles = (profiles if mutation_profile != "both" or index < debug_sample
                                       else ["release"])
                    for profile in mutant_profiles:
                        if test_only:
                            # This source is excluded from both production
                            # profiles by cfg(test). Its candidate cannot
                            # change either compiled production binary.
                            results[profile] = "equivalente_test"
                            continue
                        command = ["cargo", "test", "--locked", "--offline", "--lib"]
                        if profile == "release":
                            command.append("--release")
                        results[profile] = "superviviente"
                        build_command = command + ["--no-run"]
                        built = run_limited(build_command, tree, env, 180)
                        if built is None:
                            results[profile] = "inconclusa"
                            witnesses[profile] = "compilación del mutante: plazo de 180 s agotado"
                        elif built[0]:
                            build_output = built[1].decode("utf-8", "replace")
                            if "error[E" in build_output or "could not compile" in build_output:
                                results[profile] = "no_compila"
                            else:
                                results[profile] = "inconclusa"
                            witnesses[profile] = "compilación del mutante"
                        if results[profile] != "superviviente":
                            continue
                        if TARGETED_TEST:
                            targeted = ["cargo", "test", "--locked", "--offline"]
                            if profile == "release":
                                targeted.append("--release")
                            targeted.extend(["--test", TARGETED_TEST, "--", "--quiet"])
                            done = run_limited(targeted, tree, env, 60)
                            if done is None:
                                quick_base = targeted_baseline[profile] < 10
                                results[profile] = "detectada" if quick_base else "inconclusa"
                                witnesses[profile] = (f"{TARGETED_TEST}: no termina en 60 s; "
                                                      f"base {targeted_baseline[profile]:.2f} s"
                                                      if quick_base else f"{TARGETED_TEST}: plazo de 60 s agotado")
                            elif done[0]:
                                output_text = done[1].decode("utf-8", "replace")
                                results[profile] = "no_compila" if "could not compile" in output_text else "detectada"
                                failed = re.findall(r"(?m)^---- (.+?) stdout ----$", output_text)
                                witnesses[profile] = failed[0] if failed else TARGETED_TEST
                        # Once a test kills a mutation, the remaining suite
                        # cannot change its classification. Survivors continue
                        # through every test binary below.
                        for filter_name in filters:
                            if results[profile] != "superviviente":
                                break
                            timeout = (
                                30 if filter_name == "ir::effects::tests" else
                                (60 if filter_name == "quota_tests" else (80 if filter_name == "i07c_tests" else 100))
                            )
                            done = run_limited(command + [filter_name, "--quiet"], tree, env, timeout)
                            if done is None:
                                if filter_name == "ir::effects::tests" and effects_baseline.get(profile, 180) < 10:
                                    results[profile] = "detectada"
                                    witnesses[profile] = (
                                        f"{filter_name}: no termina en {timeout} s; "
                                        f"la base termina en {effects_baseline[profile]:.2f} s"
                                    )
                                else:
                                    results[profile] = "inconclusa"
                                    witnesses[profile] = f"{filter_name}: plazo de {timeout} s agotado"
                                break
                            returncode, output_bytes = done
                            output_text = output_bytes.decode("utf-8", "replace")
                            if "error[E" in output_text or "could not compile" in output_text:
                                results[profile] = "no_compila"
                            elif returncode:
                                results[profile] = "detectada"
                                failed = re.findall(r"(?m)^---- (.+?) stdout ----$", output_text)
                                witnesses[profile] = failed[0] if failed else filter_name
                            if results[profile] != "superviviente":
                                break
                        if results[profile] == "superviviente" and not fast_only:
                            complete_command = ["cargo", "test", "--locked", "--offline"]
                            if profile == "release":
                                complete_command.append("--release")
                            if not full_suite:
                                complete_command.append("--lib")
                            done = run_limited(complete_command + ["--", "--quiet"], tree, env,
                                               full_suite_timeout)
                            if done is None:
                                results[profile] = "inconclusa"
                                witnesses[profile] = f"cargo test: plazo de {full_suite_timeout} s agotado"
                                continue
                            returncode, output_bytes = done
                            output_text = output_bytes.decode("utf-8", "replace")
                            if "error[E" in output_text or "could not compile" in output_text:
                                results[profile] = "no_compila"
                            else:
                                results[profile] = "detectada" if returncode else "superviviente"
                                if returncode:
                                    failed = re.findall(r"(?m)^---- (.+?) stdout ----$", output_text)
                                    witnesses[profile] = failed[0] if failed else "cargo test --full-suite"
                    log.write(json.dumps({"id": item["id"],
                                          "mutation_sha256": hashlib.sha256(json.dumps(item, sort_keys=True).encode()).hexdigest(),
                                          "equivalence": "Solo modifica pruebas; el código de producción compilado sin cfg(test) es idéntico." if test_only and "equivalente_test" in results.values() else None,
                                          "witness": witnesses,
                                          "result": results},
                                         ensure_ascii=False) + "\n")
                    log.flush()
                finally:
                    path.write_bytes(original)
                assert hashlib.sha256(path.read_bytes()).hexdigest() == hashlib.sha256(original).hexdigest()
    assert protected == {path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
                         for path in protected}, "fuente original alterada"


def checked_results(path: Path, population: list[dict]) -> dict[str, dict]:
    candidates = {item["id"]: item for item in population}
    rows: dict[str, dict] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        row = json.loads(line)
        item = candidates[row["id"]]
        expected = hashlib.sha256(json.dumps(item, sort_keys=True).encode()).hexdigest()
        assert row["mutation_sha256"] == expected, row["id"]
        assert row["id"] not in rows, row["id"]
        rows[row["id"]] = row
    return rows


def shard_path(shard: int, shards: int) -> Path:
    return OUTPUT.parent / f"results-{shard:02d}-of-{shards:02d}.jsonl"


def seed_shards(population: list[dict], shards: int, overwrite: bool = False) -> None:
    assert shards > 1
    rows = checked_results(OUTPUT.parent / "results.jsonl", population)
    for shard in range(shards):
        selected = [item for index, item in enumerate(population) if index % shards == shard]
        path = shard_path(shard, shards)
        assert overwrite or not path.exists(), f"already exists: {path}"
        path.write_text("".join(json.dumps(rows[item["id"]], ensure_ascii=False) + "\n"
                                for item in selected if item["id"] in rows),
                        encoding="utf-8", newline="\n")
    print(f"Seeded {shards} shards with {len(rows)} saved results")


def merge_shards(population: list[dict], shards: int) -> None:
    assert shards > 1
    rows: dict[str, dict] = {}
    for shard in range(shards):
        selected = [item for index, item in enumerate(population) if index % shards == shard]
        part = checked_results(shard_path(shard, shards), selected)
        assert not (rows.keys() & part.keys())
        rows.update(part)
    assert len(rows) == len(population), f"missing {len(population) - len(rows)} mutations"
    merged = OUTPUT.parent / "results.jsonl"
    staging = merged.with_suffix(merged.suffix + ".tmp")
    staging.write_text(
        "".join(json.dumps(rows[item["id"]], ensure_ascii=False) + "\n"
                for item in population), encoding="utf-8", newline="\n")
    staging.replace(merged)
    print(f"Merged {shards} shards with {len(rows)} results")


def verify_integration(population: list[dict], ids: list[str], test: str | None,
                       unit_filter: str | None, unit_prefix: str | None,
                       reuse_tree: Path, shards: int,
                       profile: str, results_copy: Path | None = None,
                       mutant_timeout: int = 180,
                       ignored_unit: bool = False,
                       target_dir: Path | None = None) -> None:
    """Reejecuta supervivientes contra una regresión dirigida versionada."""
    assert sum(bool(name) for name in (test, unit_filter, unit_prefix)) == 1
    assert not ignored_unit or unit_filter or unit_prefix
    tree = reuse_tree / "tree"
    target = target_dir if target_dir is not None else reuse_tree / "target"
    target = target.resolve(strict=True)
    assert ROOT.resolve() in target.parents
    assert tree.is_dir() and target.is_dir()
    entries = {item["id"]: item for item in population}
    assert len(ids) == len(set(ids)) and all(id in entries for id in ids)
    protected = {path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
                 for path in {item["file"] for item in population}}
    assert protected == {path: hashlib.sha256((tree / path).read_bytes()).hexdigest()
                         for path in protected}
    assert_disposable_inputs(tree)
    for path in protected:
        (tree / path).touch()
    if test:
        tests = test.split(",")
        assert len(tests) == len(set(tests))
        for name in tests:
            assert re.fullmatch(r"[A-Za-z0-9_]+", name)
            test_source = ROOT / "tests" / f"{name}.rs"
            assert test_source.is_file()
            shutil.copy2(test_source, tree / "tests" / test_source.name)
            (tree / "tests" / test_source.name).touch()
            if name == "i07c_rd_conformidad":
                (tree / "scripts").mkdir(parents=True, exist_ok=True)
                (tree / "tests/fixtures").mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / "scripts/i07c_rd_conformidad.py",
                             tree / "scripts/i07c_rd_conformidad.py")
                shutil.copy2(ROOT / "tests/fixtures/i07c_rd.json",
                             tree / "tests/fixtures/i07c_rd.json")
                (tree / "scripts/i07c_rd_conformidad.py").touch()
                (tree / "tests/fixtures/i07c_rd.json").touch()
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target)
    env["RUST_TEST_THREADS"] = "4"
    prefix = ["cargo", "test", "--locked", "--offline"]
    if profile == "release":
        prefix.append("--release")
    ignored_tests = {"i07c_mutant_projection_scale"}
    all_ignored = bool(test) and all(name in ignored_tests for name in tests)
    command = ([*prefix,
                *(part for name in tests for part in ("--test", name)),
                "--", "--ignored", "--nocapture"] if all_ignored else
               [*prefix,
                *(part for name in tests for part in ("--test", name)),
                "--", "--nocapture"] if test else
               [*prefix, "--lib", unit_filter,
                "--", "--exact", *(["--ignored"] if ignored_unit else []), "--nocapture"] if unit_filter else
               [*prefix, "--lib", unit_prefix,
                "--", *(["--ignored"] if ignored_unit else []), "--nocapture"])
    baseline = run_limited(command, tree, env, 180)
    assert baseline is not None and baseline[0] == 0, baseline
    # Compile and link in the first pass; compare the mutant's bounded
    # execution with a second, already-built baseline run.
    baseline_start = time.monotonic()
    baseline = run_limited(command, tree, env, 180)
    assert baseline is not None and baseline[0] == 0, baseline
    baseline_seconds = time.monotonic() - baseline_start
    print(f"Base {test or unit_filter or unit_prefix}: {baseline_seconds:.2f} s", flush=True)
    # A mutant's first cargo invocation may spend the whole timeout compiling.
    # Build it separately so a timeout in the measured invocation witnesses
    # execution of the test, not compiler or linker work.
    build_command = command[:command.index("--")] + ["--no-run"]
    results_path = results_copy or OUTPUT.parent / "results.jsonl"
    rows = checked_results(results_path, population)
    for id in ids:
        item = entries[id]
        assert rows[id]["result"].get(profile) in ("superviviente", "inconclusa", "detectada")
        source_path = tree / item["file"]
        original = source_path.read_bytes()
        source = original.decode("utf-8")
        assert source.count(item["literal"]) == 1
        source_path.write_text(source.replace(item["literal"], item["replacement"], 1),
                               encoding="utf-8", newline="\n")
        try:
            built = run_limited(build_command, tree, env, 180)
            measured = run_limited(command, tree, env, mutant_timeout) if built is not None and built[0] == 0 else None
        finally:
            source_path.write_bytes(original)
        assert hashlib.sha256(source_path.read_bytes()).hexdigest() == hashlib.sha256(original).hexdigest()
        if built is None:
            result, witness = "inconclusa", f"{test or unit_filter or unit_prefix}: compilación agotó 180 s"
        elif built[0]:
            build_output = built[1].decode("utf-8", "replace")
            result = "no_compila" if "error[E" in build_output or "could not compile" in build_output else "inconclusa"
            witness = f"{test or unit_filter or unit_prefix}: compilación falló"
        elif measured is None:
            if baseline_seconds < 10:
                result, witness = ("detectada", f"{test or unit_filter or unit_prefix}: no termina en {mutant_timeout} s; "
                                   f"base {baseline_seconds:.2f} s")
            else:
                result, witness = "inconclusa", f"{test or unit_filter or unit_prefix}: plazo agotado"
        else:
            code, output = measured
            message = output.decode("utf-8", "replace")
            if "error[E" in message or "could not compile" in message:
                result, witness = "no_compila", test or unit_filter or unit_prefix
            elif code:
                failed = re.findall(r"(?m)^test ([^ ]+) \.\.\. FAILED$", message)
                result, witness = "detectada", f"{test or unit_filter or unit_prefix}: {', '.join(failed) or 'cargo test'}"
                mismatch = re.search(r"program (\d+): ([^\r\n]+)", message)
                if mismatch:
                    witness += f"; programa {mismatch.group(1)}: {mismatch.group(2)}"
            else:
                result, witness = "superviviente", f"{test or unit_filter or unit_prefix}: sin diferencia observable"
        rows[id]["result"][profile] = result
        rows[id].setdefault("witness", {})[profile] = witness
        name = test or unit_filter or unit_prefix
        rechecks = rows[id].setdefault("rechecks", [])
        rechecks[:] = [entry for entry in rechecks
                       if (entry.get("test"), entry["profile"]) != (name, profile)]
        rechecks.append({"test": name, "profile": profile,
                         "result": result, "witness": witness})
        for output_path, selected in [(results_path, population), *(() if results_copy or shards == 1 else (
            (shard_path(shard, shards),
             [item for index, item in enumerate(population) if index % shards == shard])
            for shard in range(shards)))]:
            staging = output_path.with_suffix(output_path.suffix + ".tmp")
            staging.write_text(
                "".join(json.dumps(rows[item["id"]], ensure_ascii=False) + "\n"
                        for item in selected if item["id"] in rows), encoding="utf-8", newline="\n")
            staging.replace(output_path)
        print(f"{id}: {result} ({witness})", flush=True)
    assert protected == {path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
                         for path in protected}


def normalize_timeout_rows(population: list[dict], shards: int) -> None:
    """El plazo agotado no demuestra que una prueba haya matado al mutante."""
    results_path = OUTPUT.parent / "results.jsonl"
    rows = checked_results(results_path, population)
    changed = []
    for row in rows.values():
        for profile, witness in row.get("witness", {}).items():
            if ("no termina en" in witness and "base " not in witness
                    and row["result"].get(profile) == "detectada"):
                row["result"][profile] = "inconclusa"
                row["witness"][profile] = witness.replace("no termina en", "plazo agotado:")
                changed.append((row["id"], profile))
    for output_path, selected in [(results_path, population), *(
        (shard_path(shard, shards),
         [item for index, item in enumerate(population) if index % shards == shard])
        for shard in range(shards))]:
        staging = output_path.with_suffix(output_path.suffix + ".tmp")
        staging.write_text(
            "".join(json.dumps(rows[item["id"]], ensure_ascii=False) + "\n"
                    for item in selected if item["id"] in rows), encoding="utf-8", newline="\n")
        staging.replace(output_path)
    print(f"Plazos reclasificados como inconclusos: {changed}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=BASE)
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--debug-sample", type=int, default=100)
    parser.add_argument("--limit", type=int, help="límite de ejecución para comprobar el arnés")
    parser.add_argument("--fast-only", action="store_true", help="solo pruebas dirigidas para sondeo")
    parser.add_argument("--full-suite", action="store_true", help="ejecuta biblioteca e integración para cada superviviente dirigido")
    parser.add_argument("--full-suite-timeout", type=int, default=900,
                        help="plazo por perfil de la suite completa en segundos")
    parser.add_argument("--shards", type=int, default=1)
    parser.add_argument("--shard", type=int, default=0)
    parser.add_argument("--seed-shards", action="store_true")
    parser.add_argument("--refresh-shards", action="store_true")
    parser.add_argument("--merge-shards", action="store_true")
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--reuse-tree", type=Path, help="copia desechable existente con target propio")
    parser.add_argument("--target-dir", type=Path,
                        help="target ya existente dentro del repositorio para un worker aislado")
    parser.add_argument("--integration-test", help="binarios de integración separados por comas para supervivientes")
    parser.add_argument("--unit-filter", help="test unitario exacto para supervivientes")
    parser.add_argument("--unit-prefix", help="prefijo de tests unitarios para supervivientes")
    parser.add_argument("--ignored-unit", action="store_true",
                        help="ejecuta una prueba unitaria ignorada en recheck dirigido")
    parser.add_argument("--mutation-profile", choices=("debug", "release", "both"), default="release")
    parser.add_argument("--profile", choices=("debug", "release"), default="release",
                        help="perfil de la reejecución dirigida")
    parser.add_argument("--ids", help="IDs separados por comas para --integration-test")
    parser.add_argument("--results-copy", type=Path,
                        help="resultado aislado para dos diferenciales simultáneos")
    parser.add_argument("--mutant-timeout", type=int, default=180,
                        help="plazo por mutante en reejecuciones dirigidas")
    parser.add_argument("--normalize-timeouts", action="store_true")
    parser.add_argument("--only", choices=("all", "test", "production"), default=DEFAULT_ONLY)
    args = parser.parse_args()
    assert args.full_suite_timeout > 0
    target = None
    if args.target_dir is not None:
        assert args.reuse_tree is not None
        target = args.target_dir.resolve(strict=True)
        assert ROOT.resolve() in target.parents
    population = enumerate_population(args.base, args.shard == 0)
    assert sum((args.seed_shards, args.refresh_shards, args.merge_shards)) <= 1
    if args.seed_shards or args.refresh_shards:
        seed_shards(population, args.shards, overwrite=args.refresh_shards)
        return
    if args.merge_shards:
        merge_shards(population, args.shards)
        return
    if args.normalize_timeouts:
        normalize_timeout_rows(population, args.shards)
        return
    if args.integration_test or args.unit_filter or args.unit_prefix:
        assert args.ids and args.reuse_tree
        verify_integration(population, args.ids.split(","), args.integration_test, args.unit_filter,
                           args.unit_prefix,
                           args.reuse_tree, args.shards, args.profile, args.results_copy,
                           args.mutant_timeout, args.ignored_unit, target)
        return
    print(f"Población cerrada: {len(population)} sustituciones", flush=True)
    if args.run:
        assert 0 <= args.shard < args.shards
        selected = [item for index, item in enumerate(population)
                    if index % args.shards == args.shard]
        if args.limit:
            selected = selected[:args.limit]
        output_name = ("results.jsonl" if args.shards == 1 else
                       f"results-{args.shard:02d}-of-{args.shards:02d}.jsonl")
        run_population(selected, args.debug_sample, args.fast_only, output_name, args.resume,
                       args.only, args.full_suite, args.reuse_tree, args.target_dir,
                       args.full_suite_timeout, args.mutation_profile)


if __name__ == "__main__":
    main()
