"""Read explicit MSVC prologue reservations from an exact lib-test /MAP pair.

Usage: python -B scripts/f5-frame-sizes.py <test.exe> <test.map> [--dumpbin <path>]
These are reservations, not peak stack usage. Only symbols in section 0001 count.
"""

import argparse
import hashlib
from pathlib import Path
import re
import subprocess


NAMES = ("drive", "resume_non_match", "expr_inner", "match_advance",
         "selection_type", "check_against_context_one", "enter_assignment",
         "enter_neg_not", "enter_binary", "for_range_error", "index_range_error",
         "binary_range_error", "resume_statement_match", "resume_expression_match")
MAP_LINE = re.compile(r"^\s*0001:([0-9A-Fa-f]{8,16})\s+(\S+)\s+([0-9A-Fa-f]{8,16})\s+", re.M)
STACK_SMALL = re.compile(r"\bsub\s+rsp,\s*(?:0x)?([0-9A-Fa-f]+)h?\b", re.I)
STACK_BIG = re.compile(r"\bmov\s+eax,\s*(?:0x)?([0-9A-Fa-f]+)h?\b", re.I)
CHKSTK = re.compile(r"\bcall\s+[^\n]*chkstk", re.I)
STACK_ALLOC = re.compile(r"\bsub\s+rsp,\s*rax\b", re.I)


def reservation(disassembly):
    """Reject missing/ambiguous allocations rather than silently guessing a frame."""
    instructions = []
    started = False
    for line in disassembly.splitlines():
        if line.startswith("nexo::") and line.endswith(":"):
            if started:
                break
            started = True
        instruction = re.search(r"\b(?:mov\s+eax|sub\s+rsp|call)\b.*", line, re.I)
        if instruction:
            instructions.append(instruction.group())
    prologue = "\n".join(instructions[:16])
    values = []
    for match in STACK_SMALL.finditer(prologue):
        if not STACK_ALLOC.match(match.group()):
            values.append(int(match.group(1).rstrip("hH"), 16))
    big = STACK_BIG.search(prologue)
    if big and CHKSTK.search(prologue) and STACK_ALLOC.search(prologue):
        values.append(int(big.group(1).rstrip("hH"), 16))
    if len(values) != 1:
        raise ValueError(f"expected one explicit stack reservation, found {values}: {prologue}")
    return values[0]


def matches(symbol, name):
    # Rust v0 encodes each component by its decimal byte length. The function
    # symbol ends at that component; closures and cleanup thunks do not.
    rust = symbol.startswith("_RNv") and symbol.endswith(f"{len(name)}{name}")
    legacy = bool(re.search(r"(?:^|[?@])" + re.escape(name) + r"@", symbol))
    return rust or legacy


def symbol_address(map_text, name, allow_missing=False):
    """Dirección del único símbolo; None si falta y allow_missing (ausente, no ambiguo)."""
    found = [(symbol, int(address, 16)) for _, symbol, address in MAP_LINE.findall(map_text)
             if matches(symbol, name)]
    if not found and allow_missing:
        return None
    if len(found) != 1:
        raise ValueError(f"{name}: expected one exact section 0001 symbol, found {found}")
    return found[0]


PATHS = (("drive", "resume_expression_match", "match_advance", "selection_type"),
         ("drive", "resume_statement_match", "selection_type"),
         ("drive", "resume_expression_match", "check_against_context_one", "expr_inner"),
         ("drive", "resume_non_match", "check_against_context_one", "expr_inner"))


def summary(sizes):
    """Líneas de la pareja y de las rutas; `ausente` si falta algún marco necesario."""
    lines = []
    if "drive" in sizes and "resume_non_match" in sizes:
        pair = sizes["drive"] + sizes["resume_non_match"]
        lines.append(f"pair={pair} margin={66560 - pair}")
    else:
        lines.append("pair=ausente")
    for path in PATHS:
        missing = [item for item in path if item not in sizes]
        label = "->".join(path)
        if missing:
            lines.append(f"{label}=ausente ({', '.join(missing)})")
        else:
            lines.append(f"{label}={sum(sizes[item] for item in path)}")
    return lines


def selftest():
    small = "0000000140001000: 48 81 EC 80 00 00 00 sub rsp,80h"
    large = ("0000000140001000: B8 00 80 00 00 mov eax,8000h\n"
             "0000000140001005: E8 10 00 00 00 call __chkstk\n"
             "000000014000100A: 48 29 C4 sub rsp,rax")
    assert reservation(small) == 128
    assert reservation(large) == 32768
    fixture = " 0001:00001000 ?drive@Checker@types@check@nexo@@YAXXZ 0000000140002000 f test.obj\n"
    assert symbol_address(fixture, "drive")[1] == 0x140002000
    for invalid in ("", fixture + fixture):
        try:
            symbol_address(invalid, "drive")
        except ValueError:
            pass
        else:
            raise AssertionError("missing/ambiguous symbol accepted")
    # Por defecto un símbolo ausente falla; con allow_missing se informa como ausente.
    try:
        symbol_address("", "drive")
    except ValueError:
        pass
    else:
        raise AssertionError("missing symbol accepted without allow_missing")
    assert symbol_address("", "drive", allow_missing=True) is None
    try:
        symbol_address(fixture + fixture, "drive", allow_missing=True)
    except ValueError:
        pass
    else:
        raise AssertionError("ambiguous symbol accepted with allow_missing")
    assert symbol_address(fixture, "drive", allow_missing=True)[1] == 0x140002000
    full = {name: 100 for name in NAMES}
    full.update(drive=27536, resume_non_match=33384)
    lines = summary(full)
    assert lines[0] == "pair=60920 margin=5640", lines[0]
    assert all("ausente" not in line for line in lines)
    assert lines[4] == "drive->resume_non_match->check_against_context_one->expr_inner=61120", lines[4]
    base = {name: size for name, size in full.items()
            if name not in ("resume_statement_match", "resume_expression_match", "match_advance")}
    lines = summary(base)
    assert lines[0] == "pair=60920 margin=5640"
    assert lines[1] == ("drive->resume_expression_match->match_advance->selection_type="
                        "ausente (resume_expression_match, match_advance)"), lines[1]
    assert lines[4] == "drive->resume_non_match->check_against_context_one->expr_inner=61120"
    assert summary({"drive": 1})[0] == "pair=ausente"
    print("f5-frame-sizes selftest: both prologues, missing and ambiguous symbols, allow-missing ok")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path, nargs="?")
    parser.add_argument("map", type=Path, nargs="?")
    parser.add_argument("--dumpbin", default="dumpbin.exe")
    parser.add_argument("--allow-missing", action="store_true",
                        help="informar los símbolos ausentes como `ausente` en vez de fallar")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest:
        selftest()
        return
    if args.executable is None or args.map is None:
        parser.error("executable and its /MAP file are required")
    text = args.map.read_text(encoding="utf-8", errors="replace")
    print(f"executable_sha256={hashlib.sha256(args.executable.read_bytes()).hexdigest()} "
          f"map_sha256={hashlib.sha256(args.map.read_bytes()).hexdigest()}")
    sizes = {}
    for name in NAMES:
        found = symbol_address(text, name, args.allow_missing)
        if found is None:
            print(f"{name}\tausente")
            continue
        symbol, address = found
        process = subprocess.run([args.dumpbin, "/DISASM",
                                  f"/RANGE:0x{address:X},0x{address + 192:X}",
                                  str(args.executable)], capture_output=True, text=True,
                                 timeout=60)
        if process.returncode:
            raise RuntimeError(f"dumpbin failed for {name}: {process.stderr}")
        sizes[name] = reservation(process.stdout)
        print(f"{name}\t{sizes[name]}\t{symbol}")
    for line in summary(sizes):
        print(line)


if __name__ == "__main__":
    main()
