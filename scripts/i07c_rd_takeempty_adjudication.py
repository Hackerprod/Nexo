#!/usr/bin/env python3
"""Check the 13 c2 takeempty differences against independent I07c obligations.

The calls occur after q's assignment. Take([]) receives a concrete signature;
Take(q) is an independent equality and cannot change q after conflict (ADR
§4.1.2.1–2). Removing both calls therefore leaves the later union's constraints
unchanged. Derive those constraints with the disposable T5/N oracle, restore byte
offsets, and compare the saved sweep outputs; the call sites are compared apart.
"""
import json
from pathlib import Path

import i07c_rd_oracle as oracle
import i07c_rd_sweep as sweep

ROWS = (3254, 4551, 4836, 6747, 7822, 8711, 8853, 10674, 11188, 13127, 13557, 14009, 15312)
BASE = "main-44bef37452e4"
PROTO = "proto-57f9db1da0d2-projection"
OUT = Path("docs/validation/i07c-rediseno/adjudicacion-takeempty.json")


def main():
    rows = sweep.data("c2")  # Checks the frozen corpus SHA-256.
    base, proto = sweep.load(BASE, "c2"), sweep.load(PROTO, "c2")
    result = []
    for index in ROWS:
        row = rows[index]
        assert row["form"] == "takeempty", (index, row["form"])
        source = sweep.source(row)
        begin = source.index("fn Main()")
        body = source[begin:]
        calls = "Take([]);Take(q);"
        assert body.count(calls) == 1 and "q=" in body[:body.index(calls)], index
        removed_at = body.index(calls)
        call_start, call_end = begin + removed_at, begin + removed_at + len(calls)
        derived = oracle.derive(body.replace(calls, "", 1))
        assert not derived.get("unmodeled") and not derived.get("error"), index
        expected = []
        for diagnostic in derived["diagnostics"]:
            def remap(offset):
                return begin + offset + (len(calls) if offset >= removed_at else 0)
            a, b = diagnostic["primary"]
            relation = diagnostic.get("rel_expected")
            relation = [remap(relation[0]), remap(relation[1])] if relation else None
            expected.append([diagnostic["code"], diagnostic.get("subcause"), remap(a), remap(b),
                             diagnostic.get("expected_type") or None,
                             diagnostic.get("found_type") or None, relation])
        def split(signatures):
            return ([d for d in signatures if not call_start <= d[2] < call_end],
                    [d for d in signatures if call_start <= d[2] < call_end])
        main_union, main_calls = split(base[index])
        proto_union, proto_calls = split(proto[index])
        assert main_calls == proto_calls, (index, "call constraint changed")
        assert proto_union == expected, (index, "prototype differs from independent oracle")
        result.append({"index": index, "expected_union": expected, "main_union": main_union,
                       "proto_union": proto_union, "common_calls": main_calls})
        print(f"{index}: C and fields match; calls unchanged", flush=True)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps({"corpus_sha256": sweep.CORPUS_SHA256["c2"], "rows": result},
                              ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"adjudicated={len(result)} output={OUT}", flush=True)


if __name__ == "__main__":
    main()
