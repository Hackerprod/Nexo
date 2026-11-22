# Validación B del plan I07c-S6-COR-2 — ronda 1: **FAIL** (cinco enmiendas de plan, sin decisión de diseño abierta)

Base: `git archive HEAD` (`d45dfca`) y un candidato prototipado con los 8 cambios del plan, en copias aparte ya borradas. Guiones de barrido: `C:\nexo-work\tmp\vb\` (no versionados).

## Requisitos

**(1) Esperado derivado de §13.13 y del addendum: PASA.** §13.13 solo fija ubicación para 6 presupuestos estructurales y exime de igualdad de `primary_span` a `type_solver_steps`, `effect_fixpoint_steps`, `ir_items` y `ownership_flow_steps`; no fija ningún span con grupo para R0001/R0002, así que no exige Flujo A. La lectura del addendum («una cuota en un sitio distinto» = «sitio que no es la expresión agrupada») es defendible; «ejecución» solo tiene contenido si los orígenes de tick se desagrupan. Lectura contraria con apoyo literal débil: §11 «Sentencia de expresión: expresión completa» y «Las agrupaciones conservan spans fuente»; no la considera razonable; si el propietario la prefiriera, S1–S4 son revertibles. No marca Flujo A.

**(2) Los 7 sitios: PASAN (reproducidos; idénticos en `run`/`check`, debug/release).**

| Sitio | Programa | Base | Con grupo quitado |
|---|---|---|---|
| S1 | `(n+1);` | R0001 `[33,38)` | `[33,36)` |
| S2 | `while (n<9999999){}` | `[30,41)` | `[30,39)` |
| S3 | `(n)=1;` | `[33,36)` | `[33,34)` |
| S4 | `if true then (n) else 2` | `[64,67)` | `[64,65)` |
| S5 | `n=n+1;(Id)(1);` | R0001 desde N=45454 | N=43478, igual que `Id(1)` |
| S5 | `(f)()` con closure | 55555 | 52631, igual que `f()` |
| S6 | `(x)=2` en guarda | `guard_mutation` `[52,55)` | `x` |
| S7 | `(Drop)(a)` en guarda | aceptado | `guard_transfer` `Drop` |

El esperado de S7 sale de F0-0003 §3.3 («consumos intrínsecos como Drop» en una guarda dan `guard_transfer`). En el candidato, `((x))` y `((Drop))` dan lo mismo.

Completitud: 81 + 22 = 103 accesos crudos (12 delegados coinciden con el plan); ~30 leídos en contexto, ninguno clasificado como identidad, clave o IR es un sitio con primario o tick agrupable. Baterías propias (varios cientos de plantillas): primarios con grupo exterior en base: 18, todos en S1–S4; diferencias de conteo en base: solo S5; en el candidato: 0 primarios con grupo exterior y 0 en 217 fuentes × 3 perfiles × check/run, debug y release. Cubren R0001, R0002 (frames, celdas, observaciones), R0101, R0102, R0103, R0201, eventos, `match`, llamadas, índices, rangos, operadores, closures, `?`, `Option`/`Result`. Queda inconclusa la repetición del barrido de conteo sobre el candidato (cortada por tiempo); S5 se verificó directamente. `for` no llega al lowering (3 fuentes por perfil publicado: `NX-E0006`, `NX-E0002` o `unknown_profile`).

**(3) Mecanismo único y efectos laterales: PASA.** Mismo ayudante, sin parches por patrón ni rutas duplicadas. Candidato frente a base sin diferencias en corpus `.n` (217 × 3 × check/run, debug/release), 152 RD y 15 filas de presupuesto reducido; `nextest --lib` 1278/1278 (incluye `i10b_tests` 2/6/1/56/25/15 y R0001, `work_oracle_tests`) y `--test suite` 889/889; ningún test existente cambia. Nada cambia en programas sin grupo. S5 y S7 no están acoplados a S1–S4/S6: bloques {S1,S2,S3,S4,S6} y {S5,S7}.

**(4) Regresiones, auditor, diferencial, pila: PASA con reparos** (motivos 3 y 5). No se toca `drive`/`resume_non_match`. Baseline congelado por (archivo, expresión) más anclas positivas basta para que falle ante un acceso crudo nuevo (andamiaje).

**(5) Deuda de `for` y PARADAS: FALLA** (motivo 1).

## Motivos del FAIL

1. **Falta el desvío por presupuesto.** El plan no dice que el implementador es un agente Sonnet por desvío de presupuesto ni que es re-validable con Codex tras el 2026-11-24.
2. **Deuda de `for` sin fecha ni texto para `Pending.md`**: fecha de registro 2026-11-22 y fila: al activar `for_iteration`, aplicar `expression_primary_span` en el origen del cargo `ArrayIterationLength` (~3160) y en el primario de la barrera. Además la lectura del addendum debe pasar de opcional a entregable: sección nueva de `F0-0002-lagunas.md` con ejemplo válido e inválido (R0001 con `while (n<N)`, `(Id)(1)`, `(Drop)(a)` en guarda).
3. **El orden de commits no hace S1–S4 revertibles por separado**: el commit final del auditor lleva baseline y anclas de S1–S7; al revertir S1–S4 el auditor falla. Anclas y baseline por solución dentro de cada commit, o declarar el procedimiento de reversión. El espejo de S4 (`statement_leaf`, ~10036) devuelve `InvalidIr` tras añadir el cargo y no es observable: su mutación inversa no puede morir; el plan debe eximirlo o sacarlo de alcance.
4. **«Sin conflicto con S5a.1» es falso para ficheros**: `scripts/i07c_span_audit.py` y `tests/suite.rs` los modifica S5a.1; orden: S5a.1 integra primero, S6-COR-2 hace `git fetch`, merge y reconcilia el auditor, con el commit del auditor de este corte al final.
5. **Faltan la lista explícita de protegidos y la PARADA ligada a ellos**: cifras 2/6/1 y 56/25/15 en `src/ir/i10b_tests.rs` (l.50/52, 162, 207–208, 325, 822, 859, 863) y `src/ir/work_oracle_tests.rs`; R0001/R0002 de `i10b_tests.rs` y de `tests/{analysis_profiles_and_quotas,corpus,i10b_campaign,i11a,j14_accounting,j16_integrated,run_cli_diagnostics_and_cells}.rs`; fuentes con destino agrupado: `tests/i07c_group_primary.rs`, `src/check/tests.rs:813`, `src/check/tests.rs:2245`, `src/frontend/tests.rs:3588`, `tests/fixtures/i07c/tainted_array_quota.n`; todos con esperado actual igual al nuevo. La PARADA debe nombrar estas cifras y los R0001 de `corpus/` y `corpus/objetivo/`.

## No bloquea
- S5 se deriva de §11 («Los paréntesis… no crean nodos cobrables»), no del addendum; el plan puede decirlo para la trazabilidad.
- El comparador de B no cubre diferencias de `related` más allá de los primarios.
