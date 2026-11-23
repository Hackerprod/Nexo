# I07c-S6COR-GUARDA-CLOSURE — informe de C (parcial no, completo: pendiente solo de validación)

Estado: implementación **en validación, sin PASS**. Rama local `i07c-s6cor-guarda-closure-impl` (desde `origin/main` `c76fa87` con el plan fusionado); sin push. Plan seguido sin desviaciones de mecanismo: `docs/correcciones/I07c-S6COR-GUARDA-CLOSURE-plan.md`. Decisiones del coordinador aplicadas: L1 = lectura A, L2 = lectura 1 (S2 en commit propio), L3 ya adoptado; aprobación de las 12 filas `head_*_guard` y de `GUARD_CONTRACT_CASES` con las cinco condiciones recibidas (cumplidas, ver §3).

**Nota de proceso.** Este corte lo ejecuta un agente Sonnet 5.5 por desvío por presupuesto; es re-validable con Codex tras 2026-11-24 04:43. Por eso se hizo la pasada de mutación propia de las regiones tocadas (§6).

## 1. Commits (uno por solución)

| Commit | Contenido |
|---|---|
| `2dbb093` C0 | `scripts/i07c_guarda_closure_diff.py`: generador del Apéndice A (840 fuentes, SHA-256 `b82fa323…b860` comprobado), fuzz del Apéndice C (3.000, SHA-256 `a797ef46…0e01` comprobado), modos `--corpus`, `--gen2 SEMILLA` (blob y SHA-256 de `gen2.py` y de cada salida comprobados), `--fuzz N`, `--files`, `--self-test` (hashes, criterios a/b/c sobre datos sintéticos, base==base). Sin producto ni tests. |
| `fc3e17a` S1 | `src/ir/effects.rs`: el bucle de guardas selecciona la arista de llamada como `find_witness` (`CallDirect` → `FunctionCall`; `CallClosure` → `$closure:<site>`, `ClosureCall`, declaración `Some(site)`); el paso causal antepuesto toma la operación de la arista y la entidad `function:<declaración>`. Pruebas en `tests/j05_match_guards.rs` (API, JNormal) y `tests/i07c_group_primary.rs` (CLI, `f0-0003-b-v1` y `f0-0005-b-v1`, check y run, control `f0-0002-b-v2`); reubicación de las 12 filas. |
| `afaa102` S2 | `CallSites::from_ast` devuelve también `closure_guards` (clave `closure.fn_span.start` = `site`); el bucle de `analyze` obtiene las guardas de una `fn` fuente o de una closure por el sufijo de `$closure:<site>` y pasa la obtención de `source_function` (solo para `with`) después del bucle. Prueba `guard_inside_a_closure_body_is_checked` (U01–U05). Clave del auditor `REVIEWED_EFFECTS`: **1223 → 1232** (medida en el commit; el plan estimaba 1226 y decía que C recalcula). |
| `0f6fd6a` | Regresión `direct_intrinsic_effect_in_guard_reports_its_own_required_effect` (superviviente R007 de la pasada propia, §6). Solo test. |

D1 (documentos) **no se hizo**: `Pending.md`/defectos abiertos/HANDOFF los actualiza el coordinador (instrucción). Ningún ADR ni `docs/baseline/` tocado. Ningún test existente se editó salvo la extracción de las 12 filas de `CASES` (aprobada).

## 2. Qué se probó (esperados transcritos del plan, no del binario)

- API (`j05_match_guards.rs`): `closure_call_in_guard_reports_guard_effect_at_the_callee_name` (T01, T02, T08, T09, T10, T12/T13 equivalente por operando, T20 sin argumentos, T11 enlace de patrón; primario por bytes, `data` completo por igualdad, cadena de operaciones y spans, `Entity` `function:<offset de fn>` del primer paso), `closure_call_in_guard_uses_transitive_and_independent_sites` (T03, T04, T14, T15, T21), `closure_call_in_guard_keeps_ownership_precedence` (O01–O03), `pure_closure_call_in_guard_is_accepted` (P01–P08 en la forma ejecutable; P06/P07 con el efecto fuera de la guarda), `guard_inside_a_closure_body_is_checked` (U01 con paridad contra la `fn`, U02–U05), `direct_intrinsic_effect_in_guard_reports_its_own_required_effect`.
- CLI (`i07c_group_primary.rs`): `closure_calls_in_guards_report_guard_effect_in_every_profile` (T01, T02, T03, T10, T11, T14, T16, T17, T18, T19, O04 con sus dos diagnósticos; positivos en ambos perfiles; `effect_model` según perfil; entidad del paso `closure.call`; control NX-E0006 `pending_scope=F0-0003` en `f0-0002-b-v2`).
- Negativo aislado con su positivo: cada negativo difiere de su positivo solo por el efecto del cuerpo (T01↔P01, T05↔P02, T07↔P03, T10↔P04).

## 3. Las 12 filas `head_*_guard` (aprobadas) — condiciones del coordinador

`GUARD_CONTRACT_CASES` (struct `GuardContractCase { case: Case, required_effect }`) con el test `guard_calls_report_the_contract_and_the_guard_effect_at_one_site`:
1. El ayudante `exactly_two_diagnostics` exige **exactamente dos** diagnósticos, el 1.º con el código del contrato de la fila (conserva literalmente los campos actuales: código, etapa, operación, primario, `data` por fragmentos, relación `contract`, cadena) y el 2.º `NX-E1305/guard_effect` con **igualdad** de todos los campos (`rule`, `explanation_id`, etapa `effects`, operación `effects.check`, `feature_id null`, `related_spans` vacío, `data` exacto `{effect_evidence, effect_model, required_effect}`, mismos pasos que el 1.º). No usa «contiene» ni `one_diagnostic`.
2. `repair` = el mismo programa sin `with` y con cuerpo `return true;` (aserto de que no contiene `with `, `Print` ni `MockUnknown`), `checked`/`unit` en check y run. El `control` de las 8 filas name/literal conserva su comprobación (primario de control, mismo código en el 1.º) más el segundo diagnóstico.
3. Los otros 113 casos, `one_diagnostic` y `effect_diagnostics_report_the_target_name_of_a_grouped_call` no cambian (solo se extrajeron las 12 filas de `CASES`).
4. Mutación inversa «quitar el 2.º diagnóstico» = M6 (§6): muere con este test.
5. Derivación por fila (primario = inicio de la guarda tras `true if ` + número de agrupaciones externas = cabeza de la llamada sin agrupaciones, F0-0002 lagunas «destino agrupado»; longitud 1 para el nombre `f`, 2 para `fn`/`if`); coincide con la tabla de §7.2 del plan, que se transcribió, y se comprobó por cálculo sobre las fuentes:

| Fila | inicio guarda | agrupaciones | primario | token | control |
|---|---|---|---|---|---|
| `no_io_name_1` | 82 | 1 | [83,84) | `f` | [82,83) |
| `no_io_name_2` | 82 | 2 | [84,85) | `f` | [82,83) |
| `no_io_literal_1` | 40 | 1 | [41,43) | `fn` | [40,42) |
| `no_io_literal_2` | 40 | 2 | [42,44) | `fn` | [40,42) |
| `no_io_if_1` | 82 | 1 | [83,85) | `if` | — |
| `no_io_if_2` | 82 | 2 | [84,86) | `if` | — |
| `no_suspend_name_1` | 92 | 1 | [93,94) | `f` | [92,93) |
| `no_suspend_name_2` | 92 | 2 | [94,95) | `f` | [92,93) |
| `no_suspend_literal_1` | 45 | 1 | [46,48) | `fn` | [45,47) |
| `no_suspend_literal_2` | 45 | 2 | [47,49) | `fn` | [45,47) |
| `no_suspend_if_1` | 92 | 1 | [93,95) | `if` | — |
| `no_suspend_if_2` | 92 | 2 | [94,96) | `if` | — |

`required_effect` del 2.º: `io` en las seis `no_io` (`Print`), `alloc` en las seis `no_suspend` (`MockUnknown`: primero de alloc, block, suspend, io; el 1.º lleva `suspend`), evidencia la del 1.º.

## 4. Puertas y diferenciales (todo con el binario del commit S2; base = `git archive c76fa87`)

- Suite completa `cargo nextest run --profile default`: debug **2.211/2.211** y release **2.213/2.213** con S1 (2.212/2.214 con S2: el único fallo previsto era el auditor por la clave de línea, corregida y comprobada con `python -B scripts/i07c_span_audit.py` y `--self-test` PASS; la suite con la clave nueva y el test añadido en `0f6fd6a` se ejecutó por filtros y en la pasada de mutación). **No** se ejecutó `validate.sh`/`gate.sh` completos (lo lanza el coordinador).
- Diferencial (`scripts/i07c_guarda_closure_diff.py`, base vs candidato, check y run, `f0-0003-b-v1` y `f0-0005-b-v1`, debug y release, debug==release en base y candidato): corpus de 840: cambian exactamente las clases esperadas (`closure_guard_eff` 360, `literal_guard_eff` 24, `closure_param/transitive/chain/capture_read/capture_move/pattern_bound/two_calls_eff` y `guard_in_closure_{fn,cl,direct}_eff` 12 c/u), 0 cambios en el resto, paridad con el gemelo `fn` (180 pares) sin discrepancias, 0 anomalías. Semillas `gen2` 6112023, 1357911 y 424242 (1.500 fuentes cada una; hashes del plan comprobados): 0 cambios, 0 anomalías, debug==release. Fuzz 3.000: 6.136 de 12.000 salidas cambian (igual que la medida de B), 0 anomalías, 0 rechazadas por la base que pasen a aceptadas, check/run coinciden, debug==release. `corpus/` (incluye `corpus/objetivo/`) y `tests/fixtures/` (217 fuentes): 0 cambios.
- RD: `python -B scripts/i07c_rd_conformidad.py --verify <debug>`: 152/152 pares, 0 diferencias.
- Pila: `python -B scripts/f5-stack65.py`: `option_match126` y `result_match126` 5/5 a 65 KiB. Marcos (`f5-frame-sizes.py`): ver §7.
- Auditor de spans: `python -B scripts/i07c_span_audit.py` y `--self-test` PASS con la clave 1232.
- Cifras protegidas (R0001/R0002 de tests y corpus, 2/6/1 y 56/25/15): los tests que las fijan están en la suite verde; el medidor de objetivos no se ejecutó aparte (queda para la puerta del coordinador).

## 5. Hallazgos para el coordinador

- La clave del auditor es **1232** (no 1226 del plan): el plan ya lo dejaba a recalcular.
- El plan §7.1 nombra `closure_calls_in_guards_report_guard_effect_in_every_profile` también para M1; con parada en el primer fallo M1/M2/M3 mueren en ese test de CLI antes que en el de API (ver §6, que además prueba M1/M2 con el test de API por separado).
- Ninguna divergencia del plan que obligue a `…-hallazgo.md`.

## 6. Mutación

Mutantes del plan, suite completa debug con parada en el primer fallo (SHA-256 inicial del archivo `f0a53089…a193` en todos; restaurado igual por SHA-256):

| Mutante | Cambio | SHA mutado | Veredicto |
|---|---|---|---|
| M1 | el bucle vuelve a ignorar `Op::CallClosure` | `730a6593…` | MUERTO (`closure_calls_in_guards_report_guard_effect_in_every_profile`; repetido solo con `closure_call_in_guard_reports_guard_effect_at_the_callee_name`: MUERTO) |
| M2 | paso antepuesto vuelve a `FunctionCall` | `38e3bd1d…` | MUERTO (también por el test de API aislado) |
| M3 | `declaración` ignorada (`function:0`) | `8d196a6d…` | MUERTO por la fila CLI de entidad |
| M4 | `from_ast` vuelve a perder las guardas de closure | `e3b75dbc…` | MUERTO (`guard_inside_a_closure_body_is_checked`) |
| M5 | el bucle salta las closures | `6c94f542…` | MUERTO (`guard_inside_a_closure_body_is_checked`) |
| M6 | quita el 2.º diagnóstico (no emite guard_effect si la función tiene `with`) | `15b9b72e…` | MUERTO (`guard_calls_report_the_contract_and_the_guard_effect_at_one_site`, aislado) |

**Pasada propia** (compensación por ser C un agente Sonnet; población congelada antes de mutar: 22 mutantes textuales sobre `CallSites::from_ast` y el bucle de guardas de `analyze`, SHA-256 de la población `0257db1e63a56f41141fdf459fb496253cfa44ae26fc56fb304c111a907988dd`; reglas: `&&`↔`||`, `<=`→`<`, `!=0`→`==0`, `==`→`!=`, `.min`→`.max`, `insert(0,)`→`push`, `chain(&f.blocks)` quitado, `unwrap_or(0)`/`map_or(0,)`→1, `Some(*site)`→`+1`, `rank`→`rank+1`, `*bit`→`0`, `continue;` quitado):

- MUERTOS (13): R001, R002, R005, R008, R009, R010, R013, R014, R017, R019, R020 (suite completa; la mayoría por las pruebas nuevas, R002/R005 por `f0005_b1b::f5_match_guard_effect_*`).
- No compilan, sin veredicto (6): R003, R004, R011, R012, R018, R021.
- R007 (`*bit`→0 en el testigo local): SUPERVIVIENTE con programa que lo exhibe (`fn Main(){match true{true if Print(1)==()=>{},_=>{}}}` daría `required_effect=alloc` en vez de `io`): se añadió la regresión de `0f6fd6a`, que pasa con el código y MATA el mutante.
- Supervivientes sin programa (suite completa en debug **y** release): R000 (`.min`→`.max` en el mapa de arranques de `CallSites`, línea previa al corte), R006 y R016 (`rank+1` solo desempata testigos del mismo sitio), R015 (`map_or(0,…)`→1 del id de entidad, inalcanzable porque toda `CallDirect` de una guarda apunta a una `fn` fuente). Se registran en `Pending.md` con fecha por el coordinador, sin bloquear (AGENTS «Mutaciones»). Resultado de R016 en release: ver §7.

## 7. Pendiente / notas

- Pendiente del coordinador: `Pending.md` (filas de la nota de escalado, supervivientes R000/R006/R015/R016, `S6COR-GUARDA-CLOSURE` y `S6COR-GUARDA-MATCH-AGRUPADA`), `docs/validation/F0-0002-I07c-defectos-abiertos.md` fila 33, HANDOFF; puerta completa `validate.sh`/`gate.sh`; revisión D/D' (y Codex tras 2026-11-24 04:43).
- Marcos de pila y R016: se añaden al final de este documento si se completaron (si no figuran, no se midieron; no hay cambio en `src/check/types.rs`).
