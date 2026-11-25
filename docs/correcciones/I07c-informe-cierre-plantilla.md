# I07c — Plantilla del informe de cierre para el propietario

Estado: **plantilla**, no es el informe. Solo contiene los datos que ya constan en `origin/main` (verificados el 2026-11-25 contra `ee74c0b`);
lo marcado «POR COMPLETAR» depende del rediseño de la restauración atómica y de la re-validación con Codex y no se rellena antes. Este documento
no declara PASS ni aprueba la fase (la aprobación de F0..F7 es del propietario).

## 1. Qué se integró (con commit en `main`)
| Corte | Qué cambia | Integración | Validación | Desvío por presupuesto |
|---|---|---|---|---|
| S4 | contexto decidido por nodo; hijo vacío único | `1bd010c` | D y D' PASS sobre `6394b67` | no |
| S5b.1 (FAM-3) | el selector de un índice se evalúa aunque su base falle | `6e9623e`, `1010938` | POR COMPLETAR (citar informes D/D' del corte) | POR COMPLETAR |
| S6 | pruebas que cierran los huecos de la matriz | `1ad7b30` | D y D' PASS sobre `4d75e2c` | no |
| S6-COR | primario de una expresión bajo agrupaciones (tipos, nombres, propiedad, efectos) | `2a7891d` | D' PASS sobre `aa4da68` | **sí** (Sonnet) |
| S6-COR-2 (S0..S10) | primarios, ticks y guardas bajo agrupaciones; lowering | `9dbcc99` | D PASS dirigido, D' PASS de confirmación; puerta GREEN `8e6f777` | **sí** (Sonnet) |
| F5-L04b | CFG de `while true` literal; barrera L08 | `784505b`..`d676aef` | cadena C → D FAIL (hueco n5) → fix4 → D2 → fix5; D' PASS condicionado (Pending `F5-WHILE-TRUE-RETURN`); puerta GREEN `d676aef` | **sí** (Sonnet) |
| S6COR-GUARDA-CLOSURE | closure con efecto en guarda de `match`: NX-E0414/E0415 + NX-E1305 mismo primario | `80820d1` (ff de `0b07fba`) | puerta GREEN sobre `5534709`; D y D' PASS | **sí** (Sonnet) |
| Mutación de regiones (S6-COR, S5b.1) | 3 regresiones de cobertura (M00158, M00156, M00067) | `7a7d304` y siguientes (solo tests y docs) | puerta GREEN sobre `4b190cd` | **sí** (Sonnet) |
| Lagunas del ADR | T5, `pending_inner`, literal de array (N), procedencia de `expected_type` (A), `match` (conservación), guarda de `match` con closure | `docs/adr/F0-0002-lagunas.md` | Flujo A/B según cada addendum | no |

## 2. Cifras (completar con medición fresca sobre el `main` final)
- Puerta completa: debug **POR COMPLETAR** / release **POR COMPLETAR** tests (último registro GREEN citado: S6COR-GUARDA-CLOSURE, debug 2.213 y release 2.215 ejecutables en el clon de D').
- RD 152/152 sin diferencias; pila F5: `drive` 27.536 B, pareja `drive`+`resume_non_match` 60.616 B (margen 5.944 B ante 66.560 B), `option_match126`/`result_match126` 5/5 a 65 KiB.
- Campaña de mutación de regiones: 800 mutantes congelados (S6-COR y S5b.1; semilla 20261123); ejecutados **POR COMPLETAR** (552 en `ee74c0b`; hito del 2026-11-25).

## 3. Defectos de núcleo: cerrados y abiertos (con dueño)
Fuente única: `docs/validation/F0-0002-I07c-defectos-abiertos.md` (verificada por ejecución).
- Cerrados: F03; filas 26–32 (S6-COR-2); fila 33 (guarda de `match` con closure, S6COR-GUARDA-CLOSURE).
- Abiertos: **49 filas** en 7 grupos (FAM-E 16, FAM-O 8, FAM-2 1, S5c-ancla 12, S5c-otros 10, F5-operand-domain 1, asignación a destino agrupado 1). Dueño: **rediseño de la restauración atómica y el ancla de unión** (Flujo A; brief `I07c-rediseno-restauracion-atomica-brief.md`; estado: R6 en B, **POR COMPLETAR**), salvo la fila 34 (asignación a destino agrupado; Flujo B con regresión) y la fila de `InvalidIr` (S5a.3, tras el rediseño).
- Regla de AGENTS.md: un defecto de núcleo bloquea siempre; este informe lista lo que existe y no decide I07c.

## 4. Deuda de mutación (con fecha y dueño)
- S6-COR-2: M00013, M00021, M00023, M00051, M00054 (sin programa), M00047/M00048 (`for` inalcanzable).
- S6COR-GUARDA-CLOSURE: R000, R006, R015, R016. F5-L04b: n10 (argumento de equivalencia).
- Regiones S6-COR/S5b.1: supervivientes sin programa listados en `docs/validation/i07c-mut-regiones/INFORME.md`; hallazgo de memoria: S5b.1 M00160 y M00253 (asignación sin límite, INCONCLUSO «memoria»).
- Dueño: mutación con Codex tras 2026-11-24 04:43 (brief de re-validación `I07c-revalidacion-codex-brief.md`). **POR COMPLETAR**: estado de esa re-validación.

## 5. Desvíos por presupuesto y su re-validación con Codex
Implementación y validación por agentes Sonnet 5.5 entre 2026-11-22 y 2026-11-24 por falta de crédito de API y cuota de Codex (nota en `docs/F0_ACUERDOS.md` §10). Condición: re-validación con Codex (`gpt-6-sol`) de S6-COR, S6-COR-2, F5-L04b y S6COR-GUARDA-CLOSURE y mutación de sus regiones. Resultado: **POR COMPLETAR**.

## 6. Riesgos y límites
- Los mecanismos S5a/S5c no están integrados: sin la restauración atómica, `main` conserva las familias twin documentadas (límites de V6 en `docs/validation/i07c-s5a/limites-conocidos.md`).
- Incidentes de máquina: agotamiento del límite de commit de Windows (2026-11-23/24) que cortó campañas; el watchdog por commit (`memguard.py`) y las reglas de memoria lo mitigan.
- El PASS de un corte vale para el corte, no para la fase.

## 7. Qué se pide al propietario
**POR COMPLETAR** (cierre de I07c, aprobación de fase y decisión sobre el pagefile).
