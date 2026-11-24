"""Memory guard for mutation workers (Windows): kernel-enforced Job Object limits plus a watchdog.

A mutant once turned a test into an unbounded allocation loop (tests `try_operator_requires_result_function`,
`try_result_type_requires_result_context`, `quota_tests::non_result_try_is_rejected_at_the_expression_boundary`:
26,9 + 22,0 + 6,4 GB of commit with <= 1,5 GB of working set), which exhausted the Windows commit limit of the
machine. Working set is NOT the right measure; this module uses commit (PrivatePageCount / job commit).

- `limit_job(popen, process_gb, job_gb)`: puts the worker process (and so every child it spawns) into a Job Object
  with a per-process and a per-job commit limit enforced by the kernel (allocations beyond it fail) and
  KILL_ON_JOB_CLOSE.
- `Guard.check(workers)`: processes with commit above `proc_gb` (except compilers: rustc/link/cc), a tree above
  `tree_gb`, or FreeVirtualMemory below `min_free_gb` -> the largest-commit process of OUR tree is killed by PID.
"""
import ctypes, json, subprocess
from ctypes import wintypes

kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)


class IO_COUNTERS(ctypes.Structure):
    _fields_ = [(n, ctypes.c_ulonglong) for n in ("r", "w", "o", "rb", "wb", "ob")]


class BASIC(ctypes.Structure):
    _fields_ = [("PerProcessUserTimeLimit", ctypes.c_longlong), ("PerJobUserTimeLimit", ctypes.c_longlong),
                ("LimitFlags", wintypes.DWORD), ("MinimumWorkingSetSize", ctypes.c_size_t),
                ("MaximumWorkingSetSize", ctypes.c_size_t), ("ActiveProcessLimit", wintypes.DWORD),
                ("Affinity", ctypes.c_size_t), ("PriorityClass", wintypes.DWORD), ("SchedulingClass", wintypes.DWORD)]


class EXTENDED(ctypes.Structure):
    _fields_ = [("Basic", BASIC), ("Io", IO_COUNTERS), ("ProcessMemoryLimit", ctypes.c_size_t),
                ("JobMemoryLimit", ctypes.c_size_t), ("PeakProcessMemoryUsed", ctypes.c_size_t),
                ("PeakJobMemoryUsed", ctypes.c_size_t)]


LIMIT_PROCESS_MEMORY, LIMIT_JOB_MEMORY, KILL_ON_JOB_CLOSE = 0x100, 0x200, 0x2000
GB = 2**30


def limit_job(popen, process_gb, job_gb):
    kernel32.CreateJobObjectW.restype = wintypes.HANDLE
    job = kernel32.CreateJobObjectW(None, None)
    info = EXTENDED()
    info.Basic.LimitFlags = LIMIT_PROCESS_MEMORY | LIMIT_JOB_MEMORY | KILL_ON_JOB_CLOSE
    info.ProcessMemoryLimit = int(process_gb * GB)
    info.JobMemoryLimit = int(job_gb * GB)
    if not kernel32.SetInformationJobObject(wintypes.HANDLE(job), 9, ctypes.byref(info), ctypes.sizeof(info)):
        raise OSError(ctypes.get_last_error(), "SetInformationJobObject")
    kernel32.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
    if not kernel32.AssignProcessToJobObject(wintypes.HANDLE(job), wintypes.HANDLE(int(popen._handle))):
        raise OSError(ctypes.get_last_error(), "AssignProcessToJobObject")
    return job  # keep the handle alive while the worker runs


def process_table():
    cmd = ("Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,PrivatePageCount"
           " | ConvertTo-Json -Compress")
    out = subprocess.run(["powershell", "-NoProfile", "-Command", cmd], capture_output=True, text=True).stdout
    try:
        return json.loads(out)
    except Exception:
        return []


def free_virtual_gb():
    out = subprocess.run(["powershell", "-NoProfile", "-Command",
                          "(Get-CimInstance Win32_OperatingSystem).FreeVirtualMemory"],
                         capture_output=True, text=True).stdout.strip()
    return int(out) / 2**20 if out.isdigit() else 0.0


def tree(root_pid, table):
    kids = {}
    for r in table:
        kids.setdefault(r["ParentProcessId"], []).append(r)
    by = {r["ProcessId"]: r for r in table}
    out, stack = [], [root_pid]
    while stack:
        pid = stack.pop()
        if pid in by:
            out.append(by[pid])
        stack += [c["ProcessId"] for c in kids.get(pid, [])]
    return out


COMPILERS = ("rustc", "link", "cc", "cl", "lld", "cargo")


def check(root_pids, proc_gb=6, tree_gb=20, min_free_gb=15, table=None, free=None):
    """Returns the list of (pid, reason, commit_gb) that must be killed (our own PIDs only)."""
    table = table if table is not None else process_table()
    free = free if free is not None else free_virtual_gb()
    kill = []
    members = []
    for root in root_pids:
        procs = tree(root, table)
        members += procs
        total = sum(p["PrivatePageCount"] or 0 for p in procs)
        for p in procs:
            commit = (p["PrivatePageCount"] or 0) / GB
            name = (p["Name"] or "").lower()
            if commit > proc_gb and not name.startswith(COMPILERS):
                kill.append((p["ProcessId"], f"commit de proceso {commit:.1f} GB ({p['Name']})", commit))
        if total / GB > tree_gb and procs:
            big = max(procs, key=lambda p: p["PrivatePageCount"] or 0)
            kill.append((big["ProcessId"], f"commit del arbol {total / GB:.1f} GB", (big["PrivatePageCount"] or 0) / GB))
    if free < min_free_gb and members:
        big = max(members, key=lambda p: p["PrivatePageCount"] or 0)
        kill.append((big["ProcessId"], f"FreeVirtualMemory {free:.1f} GB < {min_free_gb}", (big["PrivatePageCount"] or 0) / GB))
    return kill


def kill_pid(pid):
    subprocess.run(["taskkill", "/PID", str(pid), "/F"], capture_output=True)
