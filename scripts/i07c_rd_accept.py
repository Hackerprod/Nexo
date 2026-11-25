#!/usr/bin/env python3
"""Disposable, batched check/run acceptance and error differential for I07c."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time

import i07c_rd_sweep as sweep

PROFILES = ("f0-0002-b-v2", "f0-0003-b-v1", "f0-0005-b-v1")
THREAD = threading.local()
OUT = Path("docs/validation/i07c-rediseno/aceptacion")


def invoke(args):
    exe, flags, command, profile, row = args
    if not hasattr(THREAD, "directory"):
        THREAD.directory = tempfile.TemporaryDirectory(prefix="i07c-rd-accept-")
    source = Path(THREAD.directory.name) / "source.n"
    source.write_bytes(sweep.source(row).encode("utf-8"))
    env = os.environ.copy()
    if flags is not None:
        env["NEXO_XV"] = flags
    else:
        env.pop("NEXO_XV", None)
    with tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as stdout, tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as stderr:
        proc = subprocess.Popen([exe, command, "--format", "json", "--profile", profile, str(source)],
                                stdout=stdout, stderr=stderr, env=env)
        last_output, last_size = time.monotonic(), 0
        reason = None
        while proc.poll() is None:
            size = os.fstat(stdout.fileno()).st_size + os.fstat(stderr.fileno()).st_size
            if size != last_size:
                last_output, last_size = time.monotonic(), size
            if sweep.working_set(proc.pid) > sweep.MAX_WORKING_SET:
                reason = "working_set_over_6gb"
                break
            if time.monotonic() - last_output > sweep.SILENCE_LIMIT:
                reason = "silent_over_20min"
                break
            time.sleep(0.02)
        if reason:
            proc.kill()  # PID launched by this invocation.
            proc.wait()
            return {"inconclusive": reason, "pid": proc.pid}
        stdout.seek(0)
        stderr.seek(0)
        output, error = stdout.read(), stderr.read()
    try:
        doc = json.loads(output)
        kind = (doc.get("outcome") or {}).get("kind")
        diagnostics = doc.get("diagnostics", [])
        internal = proc.returncode not in (0, 1) or kind == "internal_error" or "InvalidIr" in output + error
        return {"exit": proc.returncode, "kind": kind,
                "accepted": proc.returncode == 0 and not diagnostics and kind != "rejected",
                "internal": internal}
    except (ValueError, TypeError, AttributeError):
        return {"inconclusive": "unparseable_json", "exit": proc.returncode,
                "out": output[-300:], "err": error[-300:]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("corpus")
    parser.add_argument("base_exe")
    parser.add_argument("proto_exe")
    parser.add_argument("--flags", required=True)
    parser.add_argument("--start", type=int, default=0)
    parser.add_argument("--end", type=int, required=True)
    parser.add_argument("--mode", choices=("release", "debug"), default="release")
    parser.add_argument("--series", default="")
    args = parser.parse_args()
    args.base_exe = str(Path(args.base_exe).resolve())
    args.proto_exe = str(Path(args.proto_exe).resolve())
    rows = sweep.data(args.corpus)
    end = min(args.end, len(rows))
    if args.start < 0 or args.start >= end:
        parser.error("invalid lot range")
    hashes = {}
    for role, exe in (("base", args.base_exe), ("proto", args.proto_exe)):
        with open(exe, "rb") as stream:
            hashes[role] = hashlib.file_digest(stream, "sha256").hexdigest()
    OUT.mkdir(parents=True, exist_ok=True)
    prefix = f"{args.series}-" if args.series else ""
    destination = OUT / f"{prefix}{args.corpus}-{args.mode}-{args.start:05d}-{end:05d}.json"
    if destination.exists():
        old = json.loads(destination.read_text(encoding="utf-8"))
        complete = len(old.get("checks", {})) == len(PROFILES) * 2
        clean = all(not check.get("inconclusive") for check in old.get("checks", {}).values())
        if (old.get("hashes") == hashes and old.get("flags") == args.flags
                and old.get("rows") == end - args.start and complete and clean):
            print(f"cached={destination}", flush=True)
            return
    result = {"corpus": args.corpus, "mode": args.mode, "start": args.start, "end": end,
              "rows": end - args.start, "hashes": hashes, "flags": args.flags, "checks": {}}
    for profile in PROFILES:
        for command in ("check", "run"):
            cases = {}
            for role, exe, flags in (("base", args.base_exe, None), ("proto", args.proto_exe, args.flags)):
                with ThreadPoolExecutor(max_workers=4) as pool:
                    cases[role] = list(pool.map(invoke, ((exe, flags, command, profile, row)
                                                         for row in rows[args.start:end])))
            inconclusive = [args.start + i for i, (b, c) in enumerate(zip(cases["base"], cases["proto"]))
                            if "inconclusive" in b or "inconclusive" in c]
            internal = [args.start + i for i, (b, c) in enumerate(zip(cases["base"], cases["proto"]))
                        if b.get("internal") or c.get("internal")]
            changes = [args.start + i for i, (b, c) in enumerate(zip(cases["base"], cases["proto"]))
                       if b.get("accepted") != c.get("accepted")]
            key = profile + "/" + command
            result["checks"][key] = {"acceptance_changes": changes, "internal": internal,
                                      "inconclusive": inconclusive,
                                      "base_accepted": sum(bool(v.get("accepted")) for v in cases["base"]),
                                      "proto_accepted": sum(bool(v.get("accepted")) for v in cases["proto"])}
            print(f"{args.corpus} {args.start}:{end} {args.mode} {key} changes={len(changes)} "
                  f"internal={len(internal)} inconclusive={len(inconclusive)}", flush=True)
            if inconclusive:
                destination.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
                raise RuntimeError(f"INCONCLUSIVE: {key} {inconclusive[:10]}")
    destination.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"completed={destination}", flush=True)


if __name__ == "__main__":
    main()
