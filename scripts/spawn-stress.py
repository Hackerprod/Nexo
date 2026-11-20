#!/usr/bin/env python3
"""Sonda externa: spawn-stress.py EXE N WORKERS TIMEOUT nexo|nexo-valid|plain [ARG...].

En modo nexo reproduce la fuente de hangprobe/probe.py (rechazo esperado, exit 1).
En modo nexo-valid usa un programa válido (exit 0). Ambos crean una fuente por hilo.
Imprime un objeto JSON. Solo termina los PID que ella misma lanzó.
TIMEOUT está en segundos; usar 25 para la medición de cuelgues.
"""

import json
import os
import subprocess
import sys
import tempfile
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path


def main():
    if len(sys.argv) < 6:
        raise SystemExit(__doc__)
    exe, count, workers, timeout, mode = sys.argv[1:6]
    count, workers, timeout = int(count), int(workers), float(timeout)
    if count < 1 or workers < 1 or timeout <= 0 or mode not in ("nexo", "nexo-valid", "plain"):
        raise SystemExit("N y WORKERS deben ser positivos; TIMEOUT > 0; modo nexo|nexo-valid|plain")
    durations = []
    hung = []
    failures = []
    lock = threading.Lock()
    local = threading.local()
    started = time.monotonic()
    Path("C:/nexo-work/tmp").mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="nexo-spawn-", dir="C:/nexo-work/tmp") as directory:

        def one(index):
            if mode in ("nexo", "nexo-valid"):
                if not hasattr(local, "source"):
                    local.source = os.path.join(directory, f"{threading.get_ident()}.n")
                    source = ("fn Main(){match 1{(1,2)=>{},_=>{}}}"
                              if mode == "nexo" else "fn Main(){}")
                    Path(local.source).write_text(source, encoding="utf-8")
                command = [exe, "check", local.source, "--profile", "f0-0003-b-v1", "--format", "json"]
            else:
                command = [exe, *sys.argv[6:]]
            begin = time.monotonic()
            process = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                stdout, stderr = process.communicate(timeout=timeout)
                with lock:
                    durations.append(time.monotonic() - begin)
                    expected = 1 if mode == "nexo" else 0
                    if process.returncode != expected:
                        failures.append({"index": index, "code": process.returncode,
                                         "stderr": stderr.decode("utf-8", "replace")[:200],
                                         "stdout": stdout.decode("utf-8", "replace")[:200]})
            except subprocess.TimeoutExpired:
                with lock:
                    hung.append({"index": index, "pid": process.pid})
                process.kill()  # Únicamente el proceso creado en esta invocación.
                process.communicate()

        with ThreadPoolExecutor(max_workers=workers) as executor:
            list(executor.map(one, range(count)))
    durations.sort()
    print(json.dumps({"exe": exe, "mode": mode, "n": count, "workers": workers,
                      "timeout_s": timeout, "hung": len(hung), "hung_pids": hung[:10],
                      "failed": len(failures), "failures": failures[:10],
                      "wall_s": round(time.monotonic() - started, 1),
                      "p50_s": round(durations[len(durations) // 2], 3) if durations else None,
                      "max_s": round(durations[-1], 3) if durations else None}))


if __name__ == "__main__":
    main()
