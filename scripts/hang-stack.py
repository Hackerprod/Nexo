"""Captura la pila del hilo principal de un proceso colgado (solo lectura; suspende y reanuda el hilo).
Uso: python -B scripts/hang-stack.py <pid>. Solo observa; no termina el proceso."""
import ctypes, ctypes.wintypes as wt, sys, os, struct

pid = int(sys.argv[1])
k32 = ctypes.WinDLL("kernel32", use_last_error=True)
psapi = ctypes.WinDLL("psapi", use_last_error=True)
dbg = ctypes.WinDLL("dbghelp", use_last_error=True)
ntdll = ctypes.WinDLL("ntdll")

PROCESS_ALL = 0x1F0FFF
THREAD_ALL = 0x1F03FF
TH32CS_SNAPTHREAD = 0x4

class CONTEXT(ctypes.Structure):
    _fields_ = [("P1Home", ctypes.c_uint64), ("P2Home", ctypes.c_uint64), ("P3Home", ctypes.c_uint64),
                ("P4Home", ctypes.c_uint64), ("P5Home", ctypes.c_uint64), ("P6Home", ctypes.c_uint64),
                ("ContextFlags", wt.DWORD), ("MxCsr", wt.DWORD),
                ("SegCs", wt.WORD), ("SegDs", wt.WORD), ("SegEs", wt.WORD), ("SegFs", wt.WORD),
                ("SegGs", wt.WORD), ("SegSs", wt.WORD), ("EFlags", wt.DWORD),
                ("Dr0", ctypes.c_uint64), ("Dr1", ctypes.c_uint64), ("Dr2", ctypes.c_uint64),
                ("Dr3", ctypes.c_uint64), ("Dr6", ctypes.c_uint64), ("Dr7", ctypes.c_uint64),
                ("Rax", ctypes.c_uint64), ("Rcx", ctypes.c_uint64), ("Rdx", ctypes.c_uint64),
                ("Rbx", ctypes.c_uint64), ("Rsp", ctypes.c_uint64), ("Rbp", ctypes.c_uint64),
                ("Rsi", ctypes.c_uint64), ("Rdi", ctypes.c_uint64),
                ("R8", ctypes.c_uint64), ("R9", ctypes.c_uint64), ("R10", ctypes.c_uint64),
                ("R11", ctypes.c_uint64), ("R12", ctypes.c_uint64), ("R13", ctypes.c_uint64),
                ("R14", ctypes.c_uint64), ("R15", ctypes.c_uint64), ("Rip", ctypes.c_uint64),
                ("Rest", ctypes.c_byte * 1024)]

class THREADENTRY32(ctypes.Structure):
    _fields_ = [("dwSize", wt.DWORD), ("cntUsage", wt.DWORD), ("th32ThreadID", wt.DWORD),
                ("th32OwnerProcessID", wt.DWORD), ("tpBasePri", wt.LONG), ("tpDeltaPri", wt.LONG),
                ("dwFlags", wt.DWORD)]

class SYMBOL_INFO(ctypes.Structure):
    _fields_ = [("SizeOfStruct", wt.ULONG), ("TypeIndex", wt.ULONG), ("Reserved", ctypes.c_uint64 * 2),
                ("Index", wt.ULONG), ("Size", wt.ULONG), ("ModBase", ctypes.c_uint64), ("Flags", wt.ULONG),
                ("Value", ctypes.c_uint64), ("Address", ctypes.c_uint64), ("Register", wt.ULONG),
                ("Scope", wt.ULONG), ("Tag", wt.ULONG), ("NameLen", wt.ULONG), ("MaxNameLen", wt.ULONG),
                ("Name", ctypes.c_char * 512)]

k32.OpenProcess.restype = ctypes.c_void_p
psapi.EnumProcessModules.argtypes = [ctypes.c_void_p, ctypes.c_void_p, wt.DWORD, ctypes.POINTER(wt.DWORD)]
psapi.GetModuleFileNameExW.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_wchar_p, wt.DWORD]
psapi.GetModuleInformation.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p, wt.DWORD]
k32.ReadProcessMemory.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]
k32.OpenThread.argtypes = [wt.DWORD, wt.BOOL, wt.DWORD]
k32.SuspendThread.argtypes = [ctypes.c_void_p]
k32.ResumeThread.argtypes = [ctypes.c_void_p]
k32.GetThreadContext.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
k32.CloseHandle.argtypes = [ctypes.c_void_p]
k32.CreateToolhelp32Snapshot.argtypes = [wt.DWORD, wt.DWORD]
k32.Thread32First.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
k32.Thread32Next.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
dbg.SymInitializeW.argtypes = [ctypes.c_void_p, ctypes.c_wchar_p, wt.BOOL]
dbg.SymFromAddr.argtypes = [ctypes.c_void_p, ctypes.c_uint64, ctypes.POINTER(ctypes.c_uint64), ctypes.c_void_p]
dbg.SymLoadModuleExW.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_wchar_p, ctypes.c_wchar_p, ctypes.c_uint64, wt.DWORD, ctypes.c_void_p, wt.DWORD]
k32.OpenThread.restype = ctypes.c_void_p
k32.CreateToolhelp32Snapshot.restype = ctypes.c_void_p
hp = k32.OpenProcess(PROCESS_ALL, False, pid)
if not hp:
    sys.exit(f"OpenProcess fallo {ctypes.get_last_error()}")

# módulos
mods = (ctypes.c_void_p * 256)()
need = wt.DWORD()
psapi.EnumProcessModules(hp, mods, ctypes.sizeof(mods), ctypes.byref(need))
modlist = []
for i in range(need.value // ctypes.sizeof(wt.HMODULE)):
    name = ctypes.create_unicode_buffer(512)
    psapi.GetModuleFileNameExW(hp, mods[i], name, 512)
    mi = (ctypes.c_uint64 * 3)()  # base, size, entry
    psapi.GetModuleInformation(hp, mods[i], ctypes.byref(mi), 24)
    modlist.append((mods[i] or 0, mi[1], name.value))
modlist = [(m[0] or 0, m[1], m[2]) for m in modlist]

def find_mod(addr):
    for b, s, n in modlist:
        if b <= addr < b + s:
            return n, addr - b
    return None, None

# hilos
snap = k32.CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)
te = THREADENTRY32()
te.dwSize = ctypes.sizeof(te)
tids = []
ok = k32.Thread32First(snap, ctypes.byref(te))
while ok:
    if te.th32OwnerProcessID == pid:
        tids.append(te.th32ThreadID)
    ok = k32.Thread32Next(snap, ctypes.byref(te))
print("hilos:", tids)

exe = next((n for b, s, n in modlist if n.lower().endswith("nexo.exe")), None)
exe_base = next((b for b, s, n in modlist if n.lower().endswith("nexo.exe")), 0)
dbg.SymSetOptions(0x2 | 0x10000 | 0x80000000)  # UNDNAME|DEFERRED|...
dbg.SymInitializeW(hp, os.path.dirname(exe), False)
dbg.SymLoadModuleExW.restype = ctypes.c_uint64
mb = dbg.SymLoadModuleExW(hp, None, exe, None, exe_base, 0, None, 0)
print("dbghelp modulo cargado:", hex(mb) if mb else f"fallo {ctypes.get_last_error()}")

def sym(addr):
    si = SYMBOL_INFO()
    si.SizeOfStruct = 88
    si.MaxNameLen = 512
    disp = ctypes.c_uint64()
    if dbg.SymFromAddr(hp, addr, ctypes.byref(disp), ctypes.byref(si)):
        return si.Name.decode("utf-8", "replace") + f"+0x{disp.value:x}"
    return None

for tid in tids:
    ht = k32.OpenThread(THREAD_ALL, False, tid)
    k32.SuspendThread(ht)
    ctx = CONTEXT()
    ctx.ContextFlags = 0x10001F  # CONTEXT_ALL (AMD64)
    if not k32.GetThreadContext(ht, ctypes.byref(ctx)):
        print("GetThreadContext fallo", ctypes.get_last_error())
    else:
        name, off = find_mod(ctx.Rip)
        print(f"tid {tid}: RIP=0x{ctx.Rip:x} en {os.path.basename(name) if name else '?'}+0x{off or 0:x}  RSP=0x{ctx.Rsp:x}")
        buf = ctypes.create_string_buffer(8 * 512)
        got = ctypes.c_size_t()
        k32.ReadProcessMemory(hp, ctx.Rsp, buf, len(buf), ctypes.byref(got))
        qs = struct.unpack("<%dQ" % (got.value // 8), buf.raw[:got.value])
        print("candidatos de retorno (modulo+offset / simbolo):")
        seen = 0
        for q in qs:
            n, o = find_mod(q)
            if n:
                s = sym(q) if n.lower().endswith("nexo.exe") else None
                print(f"   0x{q:x}  {os.path.basename(n)}+0x{o:x}  {s or ''}")
                seen += 1
                if seen >= 40:
                    break
    k32.ResumeThread(ht)
    k32.CloseHandle(ht)
