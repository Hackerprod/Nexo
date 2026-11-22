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


def symbol_address(map_text, name):
    found = [(symbol, int(address, 16)) for _, symbol, address in MAP_LINE.findall(map_text)
             if matches(symbol, name)]
    if len(found) != 1:
        raise ValueError(f"{name}: expected one exact section 0001 symbol, found {found}")
    return found[0]


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
    print("f5-frame-sizes selftest: both prologues, missing and ambiguous symbols ok")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path, nargs="?")
    parser.add_argument("map", type=Path, nargs="?")
    parser.add_argument("--dumpbin", default="dumpbin.exe")
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
        symbol, address = symbol_address(text, name)
        process = subprocess.run([args.dumpbin, "/DISASM",
                                  f"/RANGE:0x{address:X},0x{address + 192:X}",
                                  str(args.executable)], capture_output=True, text=True,
                                 timeout=60)
        if process.returncode:
            raise RuntimeError(f"dumpbin failed for {name}: {process.stderr}")
        sizes[name] = reservation(process.stdout)
        print(f"{name}\t{sizes[name]}\t{symbol}")
    pair = sizes["drive"] + sizes["resume_non_match"]
    print(f"pair={pair} margin={66560 - pair}")
    paths = (("drive", "resume_expression_match", "match_advance", "selection_type"),
             ("drive", "resume_statement_match", "selection_type"),
             ("drive", "resume_expression_match", "check_against_context_one", "expr_inner"),
             ("drive", "resume_non_match", "check_against_context_one", "expr_inner"))
    for path in paths:
        print(f"{'->'.join(path)}={sum(sizes[item] for item in path)}")


if __name__ == "__main__":
    main()
