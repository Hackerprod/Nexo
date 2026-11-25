# Propuesta (sin editar `Pending.md`): filas de I07c probablemente obsoletas o por actualizar

Revisión del 2026-11-25 sobre `origin/main` `ee74c0b` (solo lectura; el coordinador decide y edita).

| Fila de `Pending.md` | Propuesta | Motivo (verificable) |
|---|---|---|
| `S6COR-GUARDA-MATCH-AGRUPADA` | **cerrar** (marcar «CORREGIDA por S6COR-GUARDA-CLOSURE») | la fila `S6COR-GUARDA-CLOSURE` ya la declara corregida; la fuente del defecto (closure con efecto en guarda) da NX-E1305/guard_effect con el binario en caché de `5534709` (`f0-0003-b-v1`); la variante `MockUnknown()` con destino agrupado la cubre el mismo corte |
| `I07C-DEFECTOS-ABIERTOS` | **actualizar cifras** | dice «58 filas en 9 grupos, FAM-E twin 17»; el documento vigente da 49 filas abiertas en 7 grupos (FAM-E 16; filas 26–33 cerradas; ver `docs/validation/F0-0002-I07c-defectos-abiertos.md`) |
| `I07C-MUTACION-REGIONES` | **actualizar** | la fila cita 148/800; el último estado integrado es 552/800 y el hallazgo de memoria (M00160, M00253) y el watchdog por commit no constan |
| `I07C-CODEX-LIMITE` | **cerrar** | el límite expiró (Codex disponible desde 2026-11-24 04:43); ya no hay «espera a esa fecha» |
| `S6COR-ASIGNACION-DESTINO-AGRUPADO` | **mantener abierta**, anotar «CONFIRMADA 2026-11-25» | el repro `(q1)=…` sigue dando 3 NX-E0102 `[35,36)` `[39,43)` `[64,65)` con el binario de `5534709` |
| `S6COR-PENDIENTES` | revisar | si lista R000/R006/R015/R016 o M00013… duplica `S6COR-GUARDA-CLOSURE`/`I07C-S6COR2-DEUDA-MUTACION`; dejar una sola fila por deuda con dueño |
| `I07C-NEXE-SALIDA`, `I07C-PILA-S3-B1B`, `F5-PILA-MARGEN`, `I07C-Q-CURSORS`, `I07C-CURSOR-SIN-CLI` | mover a histórico | ya figuran como «cerrado»/«Cerrada»; no aportan pendiente |
| `I07C-MATCH-EXPR-UNIONES`, `I07C-PROCEDENCIA-EXPECTED` | mantener | su comportamiento sigue sin resolver en `main` hasta el rediseño; revisar el texto tras la decisión de lagunas del ADR |
| `I07C-MUT-EXT` | mantener (pausada) | decisión del coordinador de 2026-11-21; la campaña de regiones es otra |
| `S5-LIMITE-READPLAN`, `S5C-HALLAZGOS-ORACULO` | mantener | pertenecen al rediseño (no integrado) |

No se verificó fila por fila el resto de `F5-*` (otro carril).
