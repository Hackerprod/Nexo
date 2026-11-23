# I07c — Brief del rediseño de la inferencia monomórfica diferida (restauración atómica del tipo de elemento y ancla de unión)

Destinatario: Codex (modelo `gpt-6-sol`), Flujo A (arquitectura/ADR); después valida un subagente B. Disparador con fecha: Codex 2026-11-24 04:43, antes de F5 B2b+; dueño S5c-d3. Este documento es un brief, no un plan ni un diseño: no tiene PASS. Estado de los defectos: `docs/validation/F0-0002-I07c-defectos-abiertos.md` (§6 resume la exploración que motiva este brief). Todo el material citado está en `origin/i07c-estructural-r2` salvo indicación; léase con `git show origin/i07c-estructural-r2:<ruta>`.

## 1. Problema y alcance

`origin/main` falla en la inferencia diferida cuando una unión `if` o un literal de array contiene variables sin resolver (`var q=[]`) y hay asignaciones o lecturas posteriores. Familias abiertas (todas presentes en `main`, con repro exacto y esperado derivado en `docs/validation/F0-0002-I07c-defectos-abiertos.md`):

- **Campaña twin, 26 defectos** (§2.1 de esa lista): FAM-E 17, FAM-O 8, FAM-2 1 (F06; F08 es legítima). Síntomas: `NX-E0103` añadido sobre un `[]` que una asignación compatible posterior resuelve; `NX-E1303` o `NX-E0102` perdido o trasladado de hoja; salida dependiente del orden de las asignaciones.
- **Familia anidada de ancla de unión** (§2.2, 22 filas: S5c-ancla 12 y S5c-otros 10): `NX-E0103` espurio o `NX-E1303` extra o perdido por descarte del ancla de unión (también sin siembra), retipado por `read_plan`, primario movido por dos asignaciones rechazadas, relación `expected_type` errónea. Con la matriz de 45 filas corregida, `main` acierta 21 de 45 y falla 24 (`main` 22/45 en core y 21/45 en full según el oráculo de la exploración).
- **E0103 espurios**: en el corpus c1 (24.000), `main` coincide con el oráculo en 21.091 en core y falla 2.909 (INFORME); de ellos, 322 programas son defectos previos que `main` y el candidato 2f fallan a la vez (INFORME-2 §6), de los que 2g' arregla 173 (`docs/correcciones/I07c-S5c-d2-diseno.md` §2).
- Alcance también: S5a.2 (operandos `+`/`-`), S5a.3 (dominio lógico, `InvalidIr` en `fn Main(){let w=1+(true or false);}` con `f0-0005-b-v1`) y S5a.5 (procedencia de `expected_type`, regla A, clase I) quedan **asignados a este rediseño**. Fuera de alcance: S6-COR-2 (integrado), la guarda de `match` agrupada y la asignación a destino agrupado (filas 33 y 34 de §2.4, Flujo B aparte).

El rediseño debe cerrar o clasificar nominalmente cada fila de esa lista; no se admite «sin programa» definitivo ni filas sin adjudicar (directriz del propietario del 2026-11-21).

## 2. Restricciones duras

1. **ADR `docs/adr/F0-0002-second-slice.md`**, §4.1 (Resolución e inferencia), §4.1.1 (Inferencia monomórfica diferida) y §4.1.2 (Diagnósticos de restricciones diferidas), en particular las cláusulas 4.1.2.2 (fijación y conflicto; `NX-E0103` sobre el constructor completo; igualdad atómica entre tipos compuestos; una asignación rechazada no retipa lecturas anteriores), 4.1.2.3 (la hoja posterior se denuncia en la hoja), 4.1.2.4 (una `NX-E1303` por unión; las restricciones compatibles se conservan), 4.1.2.5 (testigo parcial; un conflicto pendiente de serialización cuenta como ya denunciado; N(d)) y 4.1.2.7 (no retipar retrospectivamente; formas equivalentes, p. ej. gemelo concreto/diferido). Citar por sección, no por número de línea.
2. **`docs/adr/F0-0002-lagunas.md`**: T5 (resultado de una operación con un operando fallido o contaminado, incluida una `if` con condición fallida); primario de una expresión bajo agrupaciones; literal de array con elemento erróneo (lectura N); estado de `match` con uniones (conservación, sin regla); origen de la relación `expected_type` (procedencia A, clase I); continuación posterior con constructor pendiente en una unión (`pending_inner`, lectura B).
3. **ADR `docs/adr/F0-0003*.md` §3.3** (Cobertura, guardas y uniones), para el `match`.
4. **`docs/adr/F0-0002-second-slice.md` §13.9** (Cardinalidad y orden de diagnósticos): toda diferencia de orden es defecto.
5. **No retipar retrospectivamente** (§4.1.2.7): la siembra y `read_plan` no sustituyen variables por el tipo de su última asignación.
6. **Sin parches por patrón de fuente**, sin flujos duplicados, sin interruptores de entorno ni `eprintln!`; una sola superficie; **una migración en un solo corte** (AGENTS.md, principios de ingeniería).
7. **Pila de TYPES (F5-pila-r1)**: toda rama nueva de `Checker::drive` o `drive::resume_non_match` entra mediante un ayudante `#[inline(never)]` por familia, con `FactId`, progreso y efectos de continuación explícitos. Margen de seguridad ≥ +1.000 B sobre 66.560 B (65 KiB). Línea base de la pareja `drive`+`resume_non_match`: 60.616 B medida en la exploración (`drive` 27.536 B, `resume_non_match` 33.080 B; INFORME-3 y INFORME-4); AGENTS.md cita 60.920 B para F5-pila-r1: **re-medir con rustc 1.99.1** (`scripts/f5-frame-sizes.py`, `scripts/f5-stack65.py`) antes de fijar la base. Objetivo de crecimiento ≤ 1.500 B por commit; **más de 2.500 B de crecimiento aborta el bloque**.
8. Terminación: el bucle de planning tiene tope 129; la exploración midió como máximo 3 pasadas; 5 o más pasadas es PARADA (diseño d2 §5). Tiempo: más de +10 % de `check` en release en los 3.674 o en el peor twin es PARADA.

Lecturas ya decididas por el coordinador (revertibles solo por Flujo A): **T5 literal** (también para continuaciones con condición fallida), **`pending_inner` lectura B**, **literal de array con elemento erróneo = N**; además `match` = conservación pura y procedencia A (ver §4).

## 3. Criterios de éxito medibles

Un diseño no es aceptable si no cumple todos; las cifras de partida están copiadas de `docs/validation/i07c-s5c-explor/INFORME-4.md` a `INFORME-8.md` y de `INFORME-3.md`.

**3.1 Cero regresiones de core frente a `main` en todos los corpus** (nivel core = código, subcausa, primario y cardinalidad; regresión = `main` coincide con el oráculo y el candidato no), con `check` y los corpus congelados antes de ejecutar:

| corpus | tamaño | notas |
|---|---:|---|
| c1 (`gen_family.py`, semilla 7391045) | 24.000 | SHA-256 `e78702d1ee98178fd2a5809ccfc49c35dcbc3b7438b9664072c5c42742d7cfe9`; perfil `f0-0002-b-v2` |
| c2 (`gen_family2.py`, semilla 20261123) | 16.000 | SHA-256 `514c929f76ef4f6fba3343647353bd420d9d931c03cf039051df5584427cd864`; 16 formas; perfiles `f0-0002-b-v2` y `f0-0003-b-v1`; 191 sin oráculo |
| corpus-b | 4.000 (3.959 modelados; 41 sin oráculo) | SHA-256 `83f9e726…` (prefijo citado en INFORME-3; el resto en el repositorio) |
| perm (`perm.json`) | 4.044 (222 sin oráculo) | permutaciones de 2 y 3 asignaciones |
| corpus-t | 10.584 | sin oráculo propio; medido frente a `main` |
| semillas de B 7021993, 4417061, 9908255 | 4.500 cada una (13.500) | formas planas y `blockstyle` con oráculo (731/367, 759/379, 744/364 programas); el resto de formas por identidad con `main` |
| `corpus-dirigido5` (`gen_dirigida5.py`, semilla 31415926) | 6.000 | SHA-256 `684d623488bd2e6ec882036a8bce8162177237ea38a72e4e6d57fb53f2dd5e09`; `var` con inicializador divergente, `return`, `break`, `while`; para aceptación |

Estado de partida del mejor prototipo (candidato-5, con `atomic0`): 0 en todos; sin `atomic0`: 56 en c1/t/B (+7 en c2), con base ALT-2 sola 1.007 (INFORME-8). El rediseño debe llegar a 0 **sin** depender de una pieza cuyo defecto propio sea `planned_projection…`. Además deben cerrarse corpus de semillas nuevas congeladas con SHA antes de medir (el diseño d2 §7 pide corpus adicionales para el cierre; los anteriores pasan a ser muestra de ajuste).

**3.2 Matriz de 45 filas**: 45/45 (core), esperados derivados por `scripts/i07c_s5c_oraculo.py` (corregido a T5 y N en d3) y validados por B (`docs/correcciones/I07c-S5c-oraculo-B-r1.md`); R4, P15 y P16 con sus positivos; los 6 de B1 (R4, X_R1na, X_R1ord, X_flat3, Y_pend_if, Y_pend_if2) correctos. Partida: 2g' y candidato-5 45/45 en core y 38/45 en full (los 7 de relación: B_rq, P1, c3, w1, P2, P3, w3; INFORME-3 §5).

**3.3 `match` = conservación pura** (decisión del coordinador): las tablas declaradas de `match` son datos de referencia, no reglas: la de 54 filas (`docs/validation/i07c-s5c/match-conservacion.json`, SHA-256 en `match-conservacion.sha256`) y la de 103 filas (`docs/validation/i07c-s5c-explor/match-conservacion-ext.json`: 51 de conservación; 3 `diferencias_declaradas_de_las_54`, 28 `diferencias_declaradas` y 21 `diferencias_declaradas_rmm3_fuera_2o_brazo`; INFORME-6 §5). El diseño debe decir **qué filas cambian y por qué (cláusula) y cuáles no cambian**; las 51 de conservación no deben cambiar. Partida: con ALT-2 la tabla de 54 da 3 filas distintas incluso con todo apagado (INFORME-5 §3); sobre la tabla de 103 filas, sin RM-M3 y con `atomic0` cambian 34 filas (9 del grupo de conservación) y sin RM-M3 y sin `atomic0`, 35 (9 de conservación); con el candidato-7 cambian 3 filas respecto de lo declarado (INFORME-8 hallazgo 3). Cualquier cambio exige texto de laguna por Flujo A.

**3.4 Aceptación idéntica a `main`** en `check` y `run`, en los 3 perfiles (`f0-0002-b-v2`, `f0-0003-b-v1`, `f0-0005-b-v1`), y **debug = release**. Partida: 0 cambios de aceptación en 88.128 programa-perfil (INFORME-5 §5) y en las 24 combinaciones de INFORME-6 §7; negativo obligatorio `fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=a;}` (`main` rechaza con `NX-E0103` `[66,67)`; el prototipo `skip` lo aceptaba).

**3.5 152 RD** (`python -B scripts/i07c_rd_conformidad.py --verify <nexo.exe>`: 152/152 y 0 diferencias de snapshot), `corpus/objetivo/` sin cambios de resultado, 3.674 fuentes, twin 300.000 y 100.000 y fuzz sin pánicos, `InvalidIr` ni `internal_error` (diseño d2 §7).

**3.6 Los 17 tests existentes que fallaban con el candidato-5** (INFORME-6 §1; adjudicados por B en `docs/correcciones/I07c-S5c-tests-B-r1.md`, tabla validada). Ningún test se edita sin aprobación expresa uno a uno; ninguno cambia de esperado sin derivación del ADR.

| test | disposición validada por B |
|---|---|
| `i07c_causal_obligations::mixed_nested_join_keeps_constructor_and_shape_sites` | reescribir: solo `NX-E1303` `[2,3]` |
| `…::adr_gap_nested_union_and_array_element_keep_current_causal_sites` | reescribir: solo `NX-E1303` `br` |
| `…::failed_outer_condition_keeps_nested_join_constructor` | reescribir: quitar el `NX-E0103` del vector |
| `…::failed_outer_array_keeps_nested_element_cause` | reescribir a 2 diagnósticos (`NX-E0103` `[25,27)` y `NX-E0102` `[49,52)`); reserva: el `NX-E0103` del literal exterior `[34,53)` queda INCONCLUSO |
| `…::pending_inner_constructor_preserves_outer_branch_comparison` | lectura B adoptada: solo `NX-E0103` `[85,87)` |
| `…::unresolved_constructor_survives_failed_nested_union_condition` | no reescribir tal cual: sustituir por el gemelo con condiciones válidas (solo `NX-E0103` `[17,19)`) y aplicar T5 literal a la continuación con condición fallida (esperado exacto de la `if` con condición fallida NO derivado) |
| `i07c_mutation_regressions::anchored_first_branch_keeps_independent_pending_join` | reescribir: solo `NX-E0102` `[136,137)` (la hoja `s<1` está contaminada, T5) |
| `…::linked_array_constructors_keep_each_causal_inference_site` | reescribir el caso 2: `[(80,82)]` |
| `…::cyclic_constructor_errors_keep_inference_code_at_each_site` | INCONCLUSO: retirar solo `[43,45)`; la inconsistencia inline/let (§4.1.2.7) es defecto con repro de las dos formas (ver §4 y pregunta 12) |
| `i07c_pending_nominal_domain::incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin` | B derivó el esperado de 4 diagnósticos (`NX-E0102` `[[r],q]`, `NX-E0102` `3`, `NX-E0102` `x`, `NX-E1303` `q`); sin `atomic0` el candidato da la salida de `main` y la derivación deja de aplicarse (INFORME-7 §3) |
| `…::planned_projection_child_does_not_use_failed_branch_as_witness` | **se arregla en el diseño** (esperado de `main` y del oráculo: `NX-E1303` en `q`); B no lo evaluó |
| `…::failed_nested_union_does_not_anchor_an_outer_pending_array` | reescribir: `NX-E1303` `[81,86)` y `NX-E0102` `[92,93)` |
| `i07c_normative_survivors::independent_causal_sites_survive_deferred_resolution` | reescribir el caso `nested_constructor`: `[(28,30)]` |
| `…::deferred_projection_and_constructor_programs_finish_with_causal_results` | reescribir el caso `repeated_constructor_sites`: `[17,19)` y `[27,37)` |
| `i07c_structural_invariants::grouped_operands_keep_one_causal_primary` | andamiaje: el auditor `scripts/i07c_span_audit.py` fija números de línea; renumerar no basta (B), indexar por función y expresión |
| `check::types::quota_tests::deferred_if_join_charges_the_reduced_boundary` | andamiaje: esperado relacional (menor límite sin errores), sin cifra fija |
| `…::failed_branch_join_keeps_reduced_quota_error_prefix` | andamiaje: esperado relacional, sin 30/31 |

Además, el candidato **eliminaba** `check::types::quota_tests::i07c_survivor2_nested_shape_hints_reach_the_leaf` (prueba `ShapeHint`; matador de M00123 y M00125): no se elimina sin sustituto.

**3.7 Oráculo** `scripts/i07c_s5c_oraculo.py`: a corregir a T5 y a N en d3; no modela `match`, `while`, `{}` ni `AssertEq`; sus esperados son previos a T5. **3.8 Campaña twin** (casos de `docs/validation/i07c-twin/casos.json`, 26 defectos sin `NX-E0103` espurio) y las filas de §2.1 y §2.2 de la lista de defectos, sin diagnósticos perdidos o trasladados.

## 4. Qué se aprendió (no repetir)

- **ALT-2 (registro de ancla por unión)** (`docs/correcciones/I07c-S5c-diseno-ancla.md`): con B1 sobre `main` sin V6 **regresiona 15 programas de la familia que `main` y el oráculo aciertan** (unión anidada en primera posición con ancla de testigo parcial; con prefijo `fn Main(){let p=true;let ar=[1];var q=[];var r=[];`, el cuerpo `let a0=(if p then (if p then r else 1) else q);q=[[ar]];}`: `main` da solo `NX-E1303` en `1`, ALT-2 da `NX-E0103` y `NX-E1303` en `q`); la desviación añade 12 más (N1 a N4) (`I07c-S5c-plan-enmienda-B-r1.md`).
- **V6 (retracción de capas sembradas)** (rama `i07c-estructural`, `2296dc8`): V6 original da 53 regresiones de core y 78 arreglos sobre c1; su retracción de capas «daña la procedencia» (INFORME §7.4); `docs/validation/i07c-s5a/limites-conocidos.md` registra los límites (P1, P2, w1, w3, c3, P12). El encargo la describe como regresión de uniones con asignación posterior compatible; esa derivación no se copió de una fuente (NO VERIFICADO).
- **Candidatos 2f y 2g'** (INFORME, INFORME-3): R1 (`pendfa`, `retfa`, `retfail`, `nojoin`, `atomic0`, `retract`) + RE (`arrx`, `skip`/`skipv3`); **pieza a pieza, cada una resulta necesaria por ablación** (INFORME-8 hallazgo 2: `pendfa` 316 arreglos y 868 regresiones al quitarla; `retfa` 142/26; `retract` 49/3; `nojoin` 178/1; `atomic0` 247/56; `e0103rule` 290/11; `retfail` 207/12; RM-M3 7/7). El diseño d2 declara que **no demuestra** que R1 sea una sola regla ni que `atomic0` y `retract` sean un mecanismo.
- **RM-M3** (`rmm3b/e/f`, `nojoinr`): conserva `main` en el `match` como expresión salvo 28 diferencias del segundo brazo (14 añadidas, 14 quitadas) y 21 fuera del segundo brazo; no es independiente de la base: sin él la tabla de conservación pura no se conserva (INFORME-8 hallazgo 3).
- **Por qué `atomic0` es soporte estructural**: sin él, la base ALT-2 da **1.007 regresiones de core** (c1 516, t 61, b 67, perm 51, B 13+10+8, c2 ~230); solo RM-M3 1.004, solo RE 990, solo R1 73, R1+RE 63, candidato-7 56+7. De las 56 regresiones del candidato-7, 54 ya aparecen con todas las piezas apagadas. Con `atomic0` hay 0, pero `atomic0` tiene su propio defecto en `planned_projection…` (programa mínimo de INFORME-6 §3). Ninguna variante (`atomic0e`, `atomic0e+atomic0p`, `a0np`, `a0sc`, `a0de`) cumple «ninguna pieza empeora lo que `main` acierta».
- **Mecanismo común de las 56 y de las 1.007**: tras denunciar una unión, una asignación posterior (`r=q;q=[[ar]]`) produce un `NX-E0102` de más. Ejemplos mínimos en `docs/validation/i07c-s5c-explor/ejemplos-regresiones-base.txt` (p. ej. t 3584: `main` y oráculo `NX-E0103` `[85,87)` y `NX-E1303` `[126,128)`; sin `atomic0`, además `NX-E0102` `[146,148)`).
- **Lo que enseñó el ADR sobre el ancla** (INFORME §7): (1) el ancla de una unión cuyo primer elemento es un `if` es el testigo parcial de la unión interior aunque haya fallado: hay que separar el estado de fallo/denuncia de la unión interior del testigo que ofrece a la exterior; (2) la unión debe tener **un solo** mecanismo de igualdad (el contraste contra el ancla), no un `join` local por pares; (3) el rechazo de una igualdad incompatible debe ser atómico en el grafo de tipos (nodos), no solo en las obligaciones; (4) una asignación posterior rechazada o diferida no debe habilitar la siembra anticipada; (5) la relación `expected_type` necesita su propia regla (procedencia A).
- **Lecturas decididas**: T5 literal (divergencia de `main` en `fn Main(){let q=(if true then (if 1 then 1 else 2) else true);}`: `NX-E0102` `[34,35)` y `NX-E1303` `[56,60)`; defecto de `main`); `pending_inner` lectura B; literal de array N; `match` conservación pura; `cyclic_constructor_errors…`: inconsistencia inline/let (§4.1.2.7), defecto con repro de las dos formas: `fn Main(){var m1=[];AssertEq([true],[m1[7],m1]);}` frente a `fn Main(){var m1=[];let a=[m1[7],m1];AssertEq([true],a);}` (fuente: `I07c-S5c-tests-B-r1.md`, fila 6).

## 5. Regla de oro

**El rediseño no se parchea pieza a pieza.** Debe derivar de **una regla del ADR demostrada**: enunciada, con ejemplos válidos e inválidos y diagnósticos esperados (código, subcausa, primario y relación), de modo que cada pieza sea consecuencia de la regla y no hallazgo de un corpus (el diseño d2 admite que sus piezas «se hallaron persiguiendo corpus»). Antes de que se escriba plan alguno:

1. Un **prototipo desechable** de la regla (no se integra, no se versiona como código de producción).
2. Un **barrido masivo contra todos los corpus del §3.1**, la matriz de 45 y las tablas de `match`, con 0 regresiones de core; si el barrido no llega a 0, el diseño no pasa a plan.
3. Solo entonces el Flujo B posterior: plan (A) → validación (B) → implementación (C, un commit por solución) → validación (D) → confirmación (D'), con la puerta completa `sh scripts/validate.sh`.

PARADA del propio rediseño: pieza que sea parche por patrón de fuente; core > 0 en un corpus nuevo con causa estructural común; un caso que el ADR derive de la regla y no se adjudique.

## 6. Material disponible y dónde

- **Informes de la exploración**: `docs/validation/i07c-s5c-explor/INFORME.md` y `INFORME-2.md` a `INFORME-8.md`; tablas `tabla-variantes.md`, `tabla3-final.md`, `tabla4-final.md`; `piezas-excluidas-cifras.json`, `ablacion-HF.json`, `clasificacion-56.json`, `clasificacion-56-minimos.json`, `atomic0-diffs.json`, `ejemplos-regresiones-base.txt`, `minimos-ablacion.txt`.
- **Candidatos** (parches desechables con interruptores `NEXO_XV`, contra `origin/main` `eeb3c9c` o `1e2e393`): `candidato-1.diff` a `candidato-7.diff` (el 3 es 2g'; el 4 añade RM-M3; el 5 añade `skipv3`; el 7 es el 5 sin `atomic0`; no hay `candidato-8.diff`). Rama de trabajo: `origin/i07c-s5c-explor`.
- **Generadores y corpus con SHA-256**: `gen_family.py`, `corpus-congelado.json`, `corpus-sha256.txt`; `gen_family2.py`, `corpus2-congelado.json`, `corpus2-sha256.txt`; `gen_dirigida5.py`, `corpus-dirigido5.json`, `corpus-dirigido5.sha256.txt`; `docs/validation/i07c-s5c/b-d2/` (`corpus-b.json`, `corpus-t.json`, `perm.json`, `gen2.py`, `gen3.py`, `permgen.py`); `docs/validation/i07c-s5c/b-d3/` (`fam.json`, `fam.sha256`, `gen.py`).
- **Arneses**: `eval3.py` (tablas A-D por nivel core/full), `matriz45.py`, `run3.py`/`run5.py`/`acept6.py` (aceptación y `run`), `passes.py` (pasadas), `match-conservacion-ext.py` (tabla de 103 filas, `--check`) y `match-conservacion-ext-mut.py`.
- **Oráculo y esperados**: `scripts/i07c_s5c_oraculo.py`, `docs/validation/i07c-s5c/oraculo/matriz-esperados.json`, `docs/validation/i07c-s5c/oraculo-vs-matriz.md`, `docs/correcciones/I07c-S5c-oraculo-B-r1.md`.
- **Diseños y validaciones**: `docs/correcciones/I07c-S5c-diseno-ancla.md` (ALT-2, matriz de 45), `I07c-S5c-d2-diseno.md` (**diseño d2 validado por B para R1 y RE**, `I07c-S5c-d2-diseno-B-r2.md`), `I07c-S5c-d2-plan.md` y su FAIL `I07c-S5c-d2-plan-B-r1.md` (9 motivos), `I07c-S5c-d3-diseno.md` y `…-B-r1.md`, `I07c-S5c-tests-B-r1.md`.
- **Planes e informes de B**: `docs/correcciones/I07c-S5c-plan*.md` (`-B-r1` a `-B-r6`, enmienda y `…-enmienda-B-r1.md`), `docs/validation/i07c-s5c/b-tests/`, `b-d2plan/`, `b-enmienda/`, `b-oraculo/`.
- **En `origin/main`**: `docs/validation/F0-0002-I07c-defectos-abiertos.md` (lista de defectos y §6), `docs/adr/F0-0002-lagunas.md`, `docs/adr/F0-0002-second-slice.md`, `scripts/f5-frame-sizes.py`, `scripts/f5-stack65.py`.

## 7. Entregables esperados de Codex y límites

**Entregables** (Flujo A; luego B valida y se itera hasta PASS):

1. Un **ADR o diseño** en `docs/adr/` (si añade o aclara reglas) o `docs/correcciones/` (si es un diseño de implementación), con la regla única, ejemplos válidos, inválidos y diagnósticos, citas por sección y los textos de laguna que proponga (con ejemplo válido e inválido).
2. El **prototipo desechable** y los comandos para reproducir el barrido.
3. La **tabla de barrido**: por corpus de §3.1, regresiones de core frente a `main` (A), arreglos (B), iguales (C) y distintos (D), nivel core y full; matriz de 45; tablas de `match` con las filas que cambian y su cláusula; aceptación en 3 perfiles, `check`/`run`, debug/release.
4. **Riesgos y límites** declarados (qué no cubren los corpus: una sola función `Main`, hasta 6 hojas y profundidad 3, sin funciones propias ni bucles en la familia base, etc.), la medida de pila y de pasadas del prototipo, y el desglose de cada pieza a su cláusula.
5. La disposición de cada test de §3.6 y de cada fila de la lista de defectos.

**No debe**: tocar `docs/baseline/`; integrar o hacer merge a `main`; hacer push forzado ni reescribir historial; editar tests existentes sin aprobación expresa; fijar cifras de andamiaje que el ADR no enuncie; usar `taskkill /IM` ni parar procesos ajenos; cambiar las lecturas ya decididas sin pasar por Flujo A.

## Preguntas de diseño abiertas

1. ¿Qué regla del ADR (§4.1.2.2, igualdad atómica) enuncia la restauración del tipo de elemento al fallar un contraste, de forma que cubra a la vez nodos, obligaciones y marcas (`atomic0` y `retract` son hoy dos mecanismos sin demostración de unidad)?
2. ¿Por qué una asignación posterior tras denunciar una unión produce un `NX-E0102` de más sin `atomic0` (los 5 mínimos de `ejemplos-regresiones-base.txt`) y qué estado debe dejar el contraste fallido para que no ocurra sin una pieza global?
3. ¿Qué propiedad distingue el defecto de `planned_projection…` (unión como elemento de un array con primer elemento pendiente `s[x]`, propiedad que la pasada de planificación no ve) de los casos que `atomic0` protege (uniones sin contexto esperado: W0, B_qr, P1, c3, w1)?
4. ¿Cómo se separa el estado de denuncia de la unión interior del testigo que ofrece a la exterior, sin regresionar la familia anidada en primera posición (los 15 programas de la enmienda de B)?
5. ¿Cómo se conserva el `match` como expresión sin RM-M3: tres filas de la tabla de 54 difieren con la sola infraestructura ALT-2 (la tabla está roja desde d2-1), y el contador `match_value_depth` del plan d2 dejó 44 de 930 `matchexpr` distintos de `main`? Cualquier cambio de la tabla exige texto de laguna por Flujo A.
6. ¿Qué se hace con los 21 programas fuera del segundo brazo donde RM-M3 ofrece un testigo que `main` no ofrece, con las 14 diferencias «añadidas» por imprecisión de `rmm3f` y con los 8 no adjudicables (INFORME-5 §2 y INFORME-6 §5)?
7. ¿Qué debe hacer el diseño con T5 literal para una continuación con condición fallida (`(if true then (if 1 then 1 else 2) else true)`)? El esperado exacto de diagnósticos no está derivado: hay que derivarlo del ADR.
8. ¿Cómo se resuelve `cyclic_constructor_errors…` (forma inline `AssertEq([true],[m1[7],m1])` frente a forma `let`, §4.1.2.7): qué esperado es el correcto para la forma inline y cómo se garantiza la misma salida en formas equivalentes?
9. ¿Qué mecanismo no basado en patrón de fuente reemplaza a `skip`/`skipv3` (no emitir `NX-E0103` sin `array_origin`) para que el negativo `var v=(if true then {return;}else{return;});var a=[v];v=a;` siga rechazándose? `skipv3` (guardar el diagnóstico y emitirlo solo si no hay otro error) puede emitirlo antes de errores posteriores del mismo cierre.
10. ¿Se resuelven en este rediseño los residuos R1b de d2 (dos `NX-E1303` en una misma unión; `NX-E1303` de menos con variable pendiente en el testigo) y el intercambio de diagnósticos del idx 2348 de la semilla 4417061 (pérdida de `NX-E1303` `[202,207)` atribuida a la infraestructura de d2-1)? El validador B midió que 2g' ya cierra (a) y (b); reconciliar con el diseño d2 §2.
11. ¿Se incluye S5a.5 (procedencia A de `expected_type`, clase I) en este corte o se aísla? Con el nivel full persistían 6 regresiones de relación en c1, 2 en corpus-b, 6 en perm y 18 en corpus-t (INFORME-3 §4), y `main` difiere de la regla A en 4 programas de B.
12. ¿Cómo se entregan S5a.2 (`+`/`-`: un solo `NX-E0102` sin E0103 ni E1303 en Y') y S5a.3 (`InvalidIr` en operandos de `or`), asignados a este rediseño, sin romper el negativo aislado por regla?
13. ¿Qué base de pila se adopta (60.616 B de la exploración frente a 60.920 B de AGENTS.md) y qué familia de ayudantes `#[inline(never)]` aloja el contraste, la restauración y el registro de ancla sin superar el crecimiento de 2.500 B?
14. ¿Cómo se acota la copia de `type_nodes` para la restauración (los prototipos clonan; el plan d2 pedía copia acotada a los nodos alcanzables, con parada por +10 % de tiempo de `check`)? ¿Está definido el cierre del conjunto alcanzable para tipos `Deferred` dentro de `concrete` y de las obligaciones?
15. ¿Qué cota de terminación se demuestra para el bucle de planning (hoy solo el tope 129 y 3 pasadas medidas; `read_plan` se recalcula por sitio en cada pasada)?
16. ¿Cómo se garantiza la independencia del orden de asignaciones (permutaciones de 2 y 3: `perm.json`, 4.044) sin retipar retrospectivamente (§4.1.2.7)?
17. ¿Cómo se cubre lo que los corpus no ejercitan (`match`, funciones propias, bucles, `Option`/`Result`, `value`-bloques, anotaciones `Array<…>`, `else if` sin paréntesis) cuando el oráculo no los modela? ¿Qué corpus nuevos con SHA se congelan para el cierre?
18. Para los tests de andamiaje (cuotas, auditor de spans), ¿qué esperado relacional sustituye a las cifras fijas (1..=8, 30/31 y números de línea), conforme a la clasificación núcleo/andamiaje de AGENTS.md?
