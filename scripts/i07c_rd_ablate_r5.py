#!/usr/bin/env python3
"""Check a source witness for each inherited R5 mechanism."""

import json

import i07c_rd_drive6 as drive
import i07c_rd_sweep as sweep

WITNESSES = [
    ("pendfa", "c1", 4896, "f0-0002-b-v2", ["pendfa"]),
    ("retfa", "c1", 15495, "f0-0002-b-v2", ["retfa"]),
    ("retract", "c1", 4929, "f0-0002-b-v2", ["retract"]),
    ("nojoin+nojoinr", "c2", 6392, "f0-0003-b-v1", ["nojoin", "nojoinr"]),
    ("atomic0", "c1", 9440, "f0-0002-b-v2", ["atomic0"]),
    ("e0103rule", "b", 80, "f0-0002-b-v2", ["e0103rule"]),
    ("retfail", "c1", 14436, "f0-0002-b-v2", ["retfail"]),
    ("rmm3", "c2", 7454, "f0-0003-b-v1", ["rmm3b", "rmm3e", "rmm3f"]),
]


def core(items):
    return [entry[:4] for entry in items]


def run(executable, flags, profile, row):
    return sweep.run_one((executable, flags, profile, row))


def main():
    for piece, corpus, index, profile, removed in WITNESSES:
        row = sweep.data(corpus)[index]
        flags = ",".join(flag for flag in drive.FLAGS.split(",") if flag not in removed)
        base = run(drive.BASE, None, profile, row)
        complete = run(drive.CANDIDATE, drive.FLAGS, profile, row)
        ablated = run(drive.CANDIDATE, flags, profile, row)
        expected = sweep.expected(corpus, sweep.data(corpus))[index]
        print(json.dumps({"piece": piece, "corpus": corpus, "index": index,
                          "source": sweep.source(row), "expected": core(expected) if expected else None,
                          "main": core(base), "complete": core(complete),
                          "ablated": core(ablated),
                          "discriminates": complete != ablated}, ensure_ascii=False))
    source = "fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=a;}"
    flags = ",".join(flag for flag in drive.FLAGS.split(",") if flag != "skipv3")
    base = run(drive.BASE, None, "f0-0002-b-v2", source)
    complete = run(drive.CANDIDATE, drive.FLAGS, "f0-0002-b-v2", source)
    ablated = run(drive.CANDIDATE, flags, "f0-0002-b-v2", source)
    print(json.dumps({"piece": "skipv3", "corpus": "negative-recursive",
                      "source": source, "expected": core(base), "main": core(base),
                      "complete": core(complete), "ablated": core(ablated),
                      "discriminates": complete != ablated}, ensure_ascii=False))


if __name__ == "__main__":
    main()
