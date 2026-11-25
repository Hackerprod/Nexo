#!/usr/bin/env python3
"""Reduce match/if internal_error family without changing implementation."""
import hashlib
import json
from pathlib import Path

from i07c_rd_drive_r6 import BASE, CANDIDATE, FLAGS
from i07c_rd_pass_errors_r6 import run


CASES = {
    "a": "fn Main(){let o:Option<I64>=Some(1);var q=[];var r=[];let a=match move o{Some(n)=>{value:(if true then q else q);},None=>{value:r;}};r=[1];q=[1];}",
    "b": "fn Main(){let o:Option<I64>=Some(1);var q=[];var r=[];let a=match move o{Some(n)=>{value:(if true then q else q);},None=>{value:r;}};r=[1];}",
    "c": "fn Main(){let o:Option<I64>=Some(1);var q=[];var r=[];let a=match move o{Some(n)=>{value:(if true then q else q);},None=>{value:r;}};q=[1];}",
    "d": "fn Main(){let o:Option<I64>=Some(1);var q=[];var r=[];let a=match move o{Some(n)=>{value:(if true then q else q);},None=>{value:r;}};}",
    "e": "fn Main(){let o:Option<I64>=Some(1);var q=[];var r=[];let a=match move o{Some(n)=>{value:(if true then q else r);},None=>{value:q;}};q=[[1]];}",
    "f": "fn Main(){let o:Option<I64>=Some(1);var q=[];var r=[];let a=match move o{Some(n)=>{value:(if true then q else r);},None=>{value:q;}};}",
    "g": "fn Main(){let o:Option<I64>=Some(1);var q=[];let a=match move o{Some(n)=>{value:(if true then q else q);},None=>{value:q;}};q=[1];}",
    "h": "fn Main(){let o:Option<I64>=Some(1);var q=[];let a=match move o{Some(n)=>{value:(if true then q else q);},None=>{value:[1];}};q=[1];}",
    "i": "fn Main(){let o:Option<I64>=Some(1);var q=[];let a=match move o{Some(n)=>{value:(if true then q else q);},None=>{value:[1];}};}",
}


def main():
    rows = []
    for name, source in CASES.items():
        main = run(BASE, source, "check", None)
        r6 = run(CANDIDATE, source, "check", FLAGS)
        row = {"name": name, "bytes": len(source.encode()),
               "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
               "source": source, "main": main, "r6": r6}
        rows.append(row)
        print(name, row["bytes"], main["outcome"]["kind"], r6["outcome"]["kind"])
    output = Path("docs/validation/i07c-rediseno/r6-internal-error-minima.json")
    output.write_text(json.dumps({"rows": rows}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
