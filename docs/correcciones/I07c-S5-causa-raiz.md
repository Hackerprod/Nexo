# I07c S5 — medición de causa raíz (FAM-O / FAM-E / FAM-2 / FAM-3)

Base medida: `git rev-parse HEAD` = `4b73f04e898714644b6f63d9277a334b5fa05364` (rama `i07c-estructural`); blob `src/check/types.rs` SHA-256 `d5046aed...2ca6` (coincide con el del borrador S5). Todas las líneas citadas son de ese blob (`git show HEAD:src/check/types.rs`), no de la copia instrumentada.

Método: copia `git archive HEAD` en `C:\nexo-work\val\s5-diag\tree`, `eprintln!` bajo `NEXO_TRACE` (macro `tr!`), interruptores de ablación/prototipo por variable de entorno (`ABL_SEED`, `ABL_READ`, `PROTO_FAM3`, `PROTO_PLANREAD`, `PROTO_TRUNC`), build release `target-s5diag` (rustc 1.99.1, `--offline`). Con todas las variables apagadas la copia instrumentada pasa **2130/2130** tests nextest (release, `--profile default`) y RD 152/152: la instrumentación es neutra. Artefactos en `D:\nexo-work\mut-s3\s5-diag\`: diff completo de instrumentación (`types-instrumentacion-y-prototipos.diff`), trazas (`A/B_qr/B_rq/F06/FAM3.trace`), repros (`repros/`), lista de tests que fallan con `ABL_SEED` (`tests-fallan-ABL_SEED.txt`).

Las salidas CLI de main se reproducen exactamente (A, A_pre, B qr/rq, F06, FAM-3 y los tres controles de FAM-3).

---

## 0. Resumen ejecutivo

1. **Las hipótesis del borrador S5 §2/§4 (reescritura de `pending_before` en `UnifyAfterBind/Union/ArrayDone`, `failed_constructor`, drenado de joins, «obligación duradera» que se pierde) NO son la causa.** Medido: los joins se crean y se reescriben con valores idénticos (`REWRITE_AFTER_BIND` old==new en todos los casos), el rollback atómico no interviene (cero `AtomicEnd` con error en los repros), y `resolve_pending_joins` hace lo que dice el código: lo que falla es que **el lado del join/el elemento del constructor no se resuelve nunca**, porque la asignación tardía compatible **nunca se aplica al elemento** (se difiere como conflicto de forma). El cierre comprueba el estado *después* de intentar aplicar las asignaciones, y llega «sin resolver» porque la asignación no se aplicó.
2. **Causa raíz de FAM-O, FAM-E y la parte E0103 de FAM-2 (un solo mecanismo):** la pasada final del checker se **siembra** con capas de Array (`shape_plan` → `seed_planned_shape`, l.3359–3410, llamada en l.8636) calculadas por las pasadas de *planning* (l.15228–15237) bajo una semántica distinta de la de la pasada final (en planning NO se aplica `read_plan`, l.11796; en la final sí). La capa sembrada viene de una igualdad que en planning se aceptó (p. ej. `r ~ aar`) y que en la pasada final se **rechaza** porque `read_plan` hace que la lectura de `r` devuelva el tipo concreto de la asignación tardía (`Array<I64>`) sin tocar la variable. La capa queda sembrada y sin justificación (el plan se auto-sostiene: cada pasada de planning resiembra el plan anterior, y `chain` ya incluye esas capas, l.3185–3198); el elemento del `[]` del `var` pasa a ser un constructor de Array preexistente; `r=[3]` → `3` contra ese elemento = «conflicto de forma» → `defer_shape_conflict` (l.5051) → el elemento queda sin resolver → cierre: `NX-E0103`, y los joins pendientes cuyo lado nunca se resuelve se convierten en E0103 (l.7704–7790), desplazando el E1303.
3. **La dependencia del orden de asignaciones (FAM-E)** es la compuerta `conflicts_prefix`/`prefix_concrete`/`pre_assignment_snapshot` de `collect_planned_shapes` (l.3226–3262): se evalúa con el estado de la *primera pasada de planning*, que sigue el orden de las asignaciones; según el orden, `read_plan` se instala o no (medido, §3.2).
4. **FAM-2 (F06) tiene además un mecanismo propio, distinto:** emisión inmediata del E1303 de una hoja posterior contra el ancla mientras la hoja anterior sigue pendiente (`if_after_first`, l.13883+, `anchor_error` l.13849/13924). Ablando la siembra, F06 pasa de {E0103, E1303(ar)} a {E1303(r), E1303(ar)}: dos C donde el gemelo da uno.
5. **FAM-3 NO comparte mecanismo con las otras.** Es un salto de hijo en el marco `Frame::Index` de `drive` (l.9234–9338, rama `other =>` l.9325): si la **base devuelve `Err`** (no `Ok(Tainted)`), el selector no se visita (`selector: None`) y su diagnóstico independiente se pierde. Prototipo de 1 sitio lo corrige (§6), con una colisión en un test protegido.
6. **Un arreglo «quitar la siembra» no es válido tal cual**: ablar `seed_planned_shape` en la pasada final rompe **21 tests protegidos** y 2 fixtures (§7). Hay que hacer la siembra *consistente*, no eliminarla.

---

## 1. Repros: salidas obtenidas (CLI `check --format json --profile f0-0002-b-v2`)

| Fuente | main (medido) |
|---|---|
| A `var r=[]`…`r=[3]` tras el uso | E0103 `[30,32)`, E1303 `[74,77)` |
| A con `r=[3];` antes del uso | E1303 `[80,83)` (solo `aar`) |
| B orden q→r | E0103 `[27,29)`, E0103 `[36,38)`, E0102 `[76,77)` |
| B orden r→q | E1303 `[63,64)` (q del if), E0102 `[89,90)` (el `3` de q) |
| F06 | E0103 `[27,29)`, E1303 `[83,85)` (ar) |
| FAM-3 `ar[true][[1,true]]` | E0102 `[30,34)` solo |
| FAM-3 `[1,true]` / `ar[0][[1,true]]` | los dos `true` / `not_indexable`+`true` (emiten) |

## 2. Estructura real del checker que el borrador no recoge: planning + pasada final

`check_types` (l.15228–15237): por función, hasta 129 pasadas con `planning=true` (`checker.function(f)`; `collect_planned_shapes()` hasta punto fijo) y luego **una pasada final** con `planning=false`. `record_error` es no-op en planning (l.4235). Las trazas muestran siempre `=== PLANNING PASS ===` ×2 y `=== FINAL PASS ===` ×1 (A, B, F06).

Productos del planning que condicionan la pasada final (todos en `InferenceGraph`, l.2181–2192):
- `shape_plan` (capas de Array por `var`), sembradas por `seed_planned_shape` (l.3359–3410; llamada l.8636) creando nodos hijo con `array_child` y `preseeded_children`;
- `concrete_plan` (tipo concreto sembrado directamente en `node.concrete`, l.3402–3410);
- `read_plan` (tipo devuelto por una lectura anterior a la asignación tardía, **sin unificar con la variable**: l.11796–11801 `return Ok(plan.ty.clone())`);
- `assignment_type_plan` ← `intrinsic_assignment_type(value)` (l.3055; predicción sintáctica del tipo del RHS), `prefix_concrete`/`pre_assignment_snapshot` (l.8317–8365), `expression_shape_hint`/`shape_hints_conflict` (l.3661/3745, usados por `if_else_context` l.13943).

Esto es una segunda vía de derivación de tipos (lookahead sintáctico) paralela al solver; es el origen de las discrepancias.

## 3. Flujo de datos medido

### 3.1 A (FAM-O) — traza `A.trace`
Planning 1 (sin semilla, `r` sin tipo): `UNIFY Deferred(r) ~ Array<Array<I64>>` en `aar` (found 74..77) **aceptado** → `r: Array<Array<_>>`; `[r]` rechazado (E1303 en planning). `PLAN site=28 layers=2`.
Planning 2 (sembrada con 2 capas): se calcula `read_plan=(Array<I64>, blocked=[(61,86)], assignment_start=89)`, `layers=2` **sin cambio** → fin del bucle (el punto fijo no considera `read_plan`).
Pasada final: `SEED_PLANNED_SHAPE site=28 var=2 layers=2` (nodos 1→0→3); `READ_PLAN_HIT name_at=53 returns Array(I64) (var 2 unresolved)`; `UNIFY Array<I64> ~ Array<Array<I64>> found 74..77` → E1303 en `aar` (correcto, sin tocar `r`). Luego `r=[3]`: `UNIFY_ENTER Deferred(0) ~ I64 found 92..93` → `DEFER_SHAPE_CONFLICT left=Deferred(0) right=I64 ctor=0 array_child=Some(3)`: el elemento de `r` (nodo 0) ya es un Array sembrado → el `3` compatible NO se aplica. Cierre:
- `CLOSE_JOIN left=Deferred(0)->None; right=I64->Some(I64)` → rama «algún lado sin resolver» (l.7745–7785) → `unresolved_constructor_diagnostic(constructor, join.found)` con origen `array_origin` del nodo 0 = **origen de la capa sembrada = [74,77)** (no el `[]`) → `RECORD_ERROR NX-E0103 (74,77) dup_site=true` → **descartado por `emitted_sites`** (l.4263) porque ya está el E1303 en ese span;
- `CLOSE_CTOR span=30..32 child_concrete=None` → `NX-E0103 [30,32)` (l.7825–7850). 
Respuesta a «¿se comprueba antes o después de aplicar las asignaciones compatibles?»: **después**; la asignación se «aplicó» como join diferido, no como fijación del elemento.
Ablación (`ABL_SEED`): A → solo E1303 en `aar` [74,77) = gemelo (A_pre, offsets recalculados).

### 3.2 B (FAM-E) — `B_qr.trace` vs `B_rq.trace`
Diferencia causal medida en el planning (línea `PLAN site=34`, variable `q`):
- orden q→r: `prefix_concrete=None`, → `read_plan=Some((Array<I64>, blocked [(46,64)], 80))`, `concrete_plan=None`.
- orden r→q: `prefix_concrete=Some(Array<Array<I64>>)` (en planning 1, cuando se procesa `q=[3]`, la igualdad `[q,[r]]` + `r=[3]` ya dejó `q` concreta), `conflicts_prefix` (l.3240) = true → **no se instala `read_plan`**.
Pasada final q→r: `[q,[r]]` lee `q` por `READ_PLAN_HIT` (devuelve `Array<I64>`, sin unificar) → la igualdad `q ~ [r]` nunca se aplica a la variable → `E0102 [76,77)` (en `r`, sitio erróneo); `q` conserva 2 capas sembradas → `q=[3]` → `DEFER_SHAPE_CONFLICT left=Deferred(3) ctor=3 array_child=Some(6)` → `q` nunca resuelve. Cierre: `CLOSE_JOIN left=Bool; right=Deferred(5)->None` (el join del `if` `[63,64)`) → fallback E0103 con origen `[36,38)` (**el E1303 de q se convierte en E0103**: respuesta a «por qué desaparece/se traslada el E1303»), y `CLOSE_JOIN left=Deferred(3)->None` → E0103 con origen `[27,29)` = origen de capa sembrada que pertenece a **`r`** (nodos compartidos por la igualdad), aunque `r` está resuelto (`CLOSE_CTOR [27,29) child_concrete=Some(I64)`).
Pasada final r→q: sin `read_plan`; `q` queda `Array<Array<I64>>` por `[q,[r]]`; el `if` cierra con `ctor_concrete=Some(Array<Array<I64>>)` → `E1303 [63,64)`; `q=[3]` → `E0102 [89,90)`. Es el resultado correcto del ADR (adjudicación).
Ablación `ABL_SEED+ABL_READ`: ambos órdenes → {E1303 [63,64), E0102 en el `3` de q}, idénticos. `ABL_SEED` solo: q→r da E1303 correcto pero E0102 en `[76,77)` (efecto de `read_plan`); `ABL_READ` solo: q→r da E1303 [63,64) + E0102 `[83,84)` (el `3` de q, correcto).

### 3.3 F06 (FAM-2) — `F06.trace`
Planning 1: `has_recursive_join=true`, `layers=2`. Final: `SEED layers=2`; `READ_PLAN_HIT name_at=51` (`r[1]` base `Array<I64>`); `DEFER_SHAPE_CONFLICT left=I64 right=Deferred(2) ctor=1 branch=true found=76..77` (join de la hoja `r` interna con el ancla `I64` de `r[1]`); `if_after_first/if_finish_join`: `anchor_pending=true anchor_err=true` → el E1303 de `ar` (83..85) se **emite en el acto** contra el ancla `I64`; `r=[3]` → `DEFER_SHAPE_CONFLICT left=Deferred(0)` (capa sembrada); cierre: el join 76..77 tiene `right=Deferred(2)->None` → E0103 en lugar del E1303 de `r`.
Con `ABL_SEED`: el join 76..77 resuelve (`r: Array<I64>`) y emite E1303 en `r` (como el gemelo), **pero queda el E1303 de `ar`**: segundo defecto (mecanismo de ancla): el C de una hoja posterior se registra mientras la hoja anterior contra la misma ancla está pendiente (`anchor_pending`), sin condicionarlo a que la anterior sea compatible. El gemelo (`F06_pre`, `F06_twin`) da un solo C en la primera hoja `r`.

### 3.4 FAM-3 — `FAM3.trace`
```
INDEX_BASE_OK span=27 base=Array(I64)                       (ar[...])
UNIFY I64 ~ Bool found=30..34  -> Err(NX-E0102)             (selector `true` de ar[true])
INDEX_BASE_NOT_OK span=27 base=Err(NX-E0102 [30,34)) -> selector NOT visited
RECORD_ERROR NX-E0102 (30,34)
```
La base del índice externo es el índice interno, que devolvió `Err` (no `Tainted`). En `Frame::Index` (l.9234–9338) solo `Ok(base)` programa `Frame::IndexSelector`; la rama `other =>` (l.9325–9337) llama a `check_against_context_one(... IndexTarget{target: other, selector: None})` y `Expr::Index` (l.12152 `let base = base?;`) devuelve el error sin haber visitado el selector. Por eso `[1,true]` ni se evalúa. Controles: base `Ok(I64)` (`ar[0][...]`) sí visita el selector (`INDEX_BASE_OK base=I64` y E0102 del `true`). Contraste del borrador de FAM-3 («base contaminada») = `Err`, no `Tainted`: con `Ok(Tainted)` el selector sí se visita (l.12158–12167).
Otras formas con el mismo salto (medidas): `(1+true)[[1,true]]`, `ar[true][2+true]`, `ar[true][0][[1,true]]`, `ar[(1+true)][[1,true]]`, `ar[0][0][[1,true]]`: todas pierden el diagnóstico del selector. `Binary` no lo tiene (existe `BinaryIndependentLeftError`, l.9697+), ni `Array`/`Call`/`AssertEq` (`[ar[true],[1,true]]`, `AssertEq(ar[true],[1,true])`, `Len(ar[true])`+`Len([1,true])`, `ar[true]+[1,true][0]`: emiten ambos).

### 3.5 Muestra de casos.json (7): O03, O05, F01, F11, F16, F18 (+O01, F08 de control)
Patrón repetido en la pasada final de **todos**: `SEED_PLANNED_SHAPE` con más capas que el RHS de la asignación, luego `DEFER_SHAPE_CONFLICT left=Deferred(<elemento del var>) right=I64` con `array_child=Some(..)` (elemento sembrado como Array), luego `CLOSE_JOIN ... -> None` + `CLOSE_CTOR child_concrete=None` → E0103 con el elemento que la asignación compatible debía resolver. Con `ABL_SEED` desaparecen **todos** los E0103 espurios de la muestra (F08, E0103 legítimo, se conserva idéntico). `F16`, `F11` y `O01` conservan diferencias adicionales vs. adjudicación al ablar solo la siembra (efecto de `read_plan` o del ancla), no mapeo 1:1.

## 4. (a) Causa raíz, con líneas del HEAD

Un único defecto de diseño de la capa de planning:
1. `check_types` l.15228–15237: la semántica del planning difiere de la final (`read_plan` solo en final, l.11796; `record_error` no-op, l.4235).
2. `collect_planned_shapes` l.3134–3357: `shape_plan` = `origins` de la cadena del nodo (l.3185–3198), que incluye capas ya sembradas → **punto fijo auto-sostenido, sin retractación**; `read_plan`/`concrete_plan` no cuentan en `changed` (l.3354).
3. `seed_planned_shape` l.3359–3410: crea nodos hijo con `array_child` y `preseeded_children` (l.3407) **antes** de que la pasada final sepa si la igualdad que los justifica se acepta.
4. Lectura con `read_plan` (l.11796–11801): devuelve un tipo sin unificar la variable → la igualdad se rechaza en el sitio y la variable conserva la siembra de la igualdad «aceptada» del planning (contradicción interna del plan: capas=2, tipo=`Array<I64>`).
5. `Bind` de asignación: l.5530–5560 (ruta `array_constructed || preseeded_children.contains(child)` → `defer_shape_conflict`) y l.5740–5810 (Unify): el elemento sembrado se trata como constructor que choca con `I64` → conflicto diferido, **no se aplica la fijación compatible** (el elemento queda sin resolver).
6. Cierre: l.7704–7790 (joins con lado no resoluble → `unresolved_constructor_diagnostic(constructor, join.found)` l.7784, origen = `array_origin` de la capa sembrada, no el `[]` real, l.4877–4891) y l.7825–7850 (`pending_constructors`, `concrete(child).is_none()`). Dedup por span en `record_error` l.4263 oculta E0103/E1303 según el orden.
7. Dependencia de orden (FAM-E): `conflicts_prefix` l.3240, `prefix_concrete` l.8354–8365, `pre_assignment_snapshot` l.8317–8335 — snapshot tomado en la primera pasada de planning, en orden de fuente.

Por qué se emite E0103 con el elemento «resuelto»: no está resuelto en el grafo (el `3` fue a `defer_shape_conflict`); lo que está resuelto es la **lectura** de `r` por `read_plan` (tipo de otra vía). El propio diagnóstico de main lleva `found_type` ya resuelto porque lo lee de `read_plan`/`concrete_plan`, no del nodo.

## 5. (b) Familia: sitios con el mismo patrón

Mismo patrón («plan derivado en otra semántica se aplica sin justificación en la pasada final / se evita el solver»):
- `seed_planned_shape` (l.3359–3410), ambas mitades: capas (3375–3396) y `concrete_plan` directo a `node.concrete` (3402–3410).
- `collect_planned_shapes` completa (l.3134–3357): `read_plan` (3264), `concrete_plan` (3314, 3347, 3350), `shape_plan` (3354), gates `conflicts_prefix`/`conflicts_shape`/`conflicts_earlier_assignment`/`unresolved_peer`.
- `read_plan` consumido en `Expr::Name` (l.11796–11801) — único consumidor.
- Predictores sintácticos: `intrinsic_assignment_type` (l.3055), `expression_shape_hint`/`shape_hints_conflict` (l.3661/3745; `if_else_context` l.13943).
- Rutas de «conflicto de forma» que reciben un elemento sembrado: `defer_shape_conflict` (5 llamadas: 5514, 5743, 5771, 5851, 5923 del borrador = HEAD l.5498–5920 aprox.) y `unresolved_constructor_shape_conflict` (l.4893).
- Cierre que usa `array_origin` de una capa sembrada como origen primario del E0103: l.7784 y l.7825–7850.
- Ancla de unión (FAM-2, mecanismo distinto): `if_after_first` l.13883 (`anchor_error` 13849/13924, `anchor_pending`), `if_finish_join` l.13499 (`skip_failed_context_join`, `truncate_pending_joins` l.3499).
- FAM-3 (mecanismo distinto): `Frame::Index` l.9234–9338 (`other =>`), `Expr::Index` l.12152.

## 6. (c) ¿FAM-3 comparte mecanismo?
**No.** FAM-3 se reproduce sin ninguna variable `var`/asignación tardía/siembra/`read_plan` (traza: ni un `SEED`, ni un `DEFER_SHAPE_CONFLICT`); es un salto de hijo por propagación de `Err`. Ablar siembra/lectura no lo cambia (corpus: `ABL_*` no altera `FAM3.n`). Familia propia: marcos de `drive` donde un hijo `Err` impide visitar a los hermanos independientes — auditados: solo `Index` (selector). `Binary` ya lo resuelve con `BinaryIndependentLeftError`.

Prototipo (1 sitio, `PROTO_FAM3`, ya en el diff): en la rama `Err(error)` no-`AnalysisLimit` de `Frame::Index` programar `Frame::IndexSelector` con `base: Result<Type,_>` y contexto sin `Array` (`expected: None`, herencia `Empty`, igual que base no indexable); `Expr::Index`: `let base = match base { Err(e) => { registrar el error del selector si lo hay; return Err(e) } … }`. Resultado: `ar[true][[1,true]]` → E0102 `[30,34)` y `[39,43)`; los 5 controles ganan el diagnóstico del selector; RD 152/152; corpus (224 `.n`) sin diferencias; tests: **2129/2130** (cambia solo `i07c_s4_context_regressions::contextual_owner_and_adjacent_fact_boundaries_match_b1b_cli`, caso `index_selector`: aparece un E0102 extra `[162,163)` esperado `Array<Array<I64>>` hallado `I64`, que el oráculo «transcrito de be8c47c» no tiene). Es un cambio de salida protegida: **requiere decisión** (no se puede cambiar el esperado sin consulta, plan S5 §6).

## 7. (d) Hipótesis de mecanismo único y riesgos

### Evidencia de ablación/prototipos (sobre la pasada final; solo evidencia, nada aplicado)
| Variante | A | B qr/rq | F06 | muestra casos.json | tests protegidos |
|---|---|---|---|---|---|
| `ABL_SEED` (no sembrar capas/`concrete_plan` en la final) | solo E1303 aar (=gemelo) | E1303 q + E0102 (qr: sitio `[76,77)` erróneo) | E1303 r + E1303 ar (2 C) | 0 E0103 espurios; F08 intacto | **21 fallan** (+2 fixtures cambian, RD 152/152 intacto) |
| `ABL_READ` (sin `read_plan`) | 2×E0103 | E1303 q + E0102 en el `3` de q (ambos órdenes iguales, correcto) | 2×E0103 | | 1 falla (`incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin`) |
| ambos | E1303 `[r]` [83,86) + E0102 `3` | iguales y correctos | E1303 r + E0102 `3` | F01, F02 = P[1,0] de la adjudicación | |
| `PROTO_TRUNC` (truncar capas a la profundidad del tipo de `read_plan`) | E1303 `[r]` + E0102 | E1303 q + E0102 (`[76,77)`) | sin cambio | arregla A,F01,F02(parcial),O03,O05,F18; **no** F06,F16,F11,O01 | 1 falla (la misma) |
| `PROTO_PLANREAD` (aplicar `read_plan` también en planning + contarlo en `changed`) | sin mejora | sin mejora | sin mejora | | **oscila: 129 pasadas** (tope) y peor salida en A (`E0103 [74,77)`) |

Los 21 tests que rompe `ABL_SEED` (lista en `tests-fallan-ABL_SEED.txt`) son de `i07c_pending_nominal_domain` (14), `i07c_causal_obligations` (2), `i07c_mutation_regressions` (3), `i07c_normative_survivors` (1), `i07c_s4_context_regressions` (1) y dos unitarios (`check::types::quota_tests::i07c_survivor2_seeded_array_child_...`, `ir::i07c_tests::plus_distributed_left_failure_...`): la siembra codifica comportamiento legítimo (p. ej. fixtures `indexed_array_conflicting_rhs.n` y `deferred_array_and_condition_independent_sites.n`: pierde el E0102 `[53,56)` y cambia `related_spans`).

### Hipótesis de mecanismo único (en términos del modelo de hechos de S4)
Una capa del plan solo puede contribuir al grafo como **hecho cuyo cierre sigue la pasada**: la siembra de una capa de Array y la lectura anticipada de una asignación tardía deben ser *el mismo objeto* (un hecho/obligación del `InferenceGraph` con su propia justificación: la igualdad aceptada o la asignación), no tres estructuras (`shape_plan`, `read_plan`, `concrete_plan`) con puertas distintas. Operativamente:
1. **Justificación**: sembrar una capa solo si la igualdad que la originó se acepta **en la semántica de la pasada final** (con `read_plan` aplicado); recalcular `shape_plan` en cada pasada desde un grafo *sin sembrar* (romper el auto-sostén de la l.3185–3198) e incluir `read_plan`/`concrete_plan` en `changed` (sin oscilación: dar prioridad monótona a una sola fuente por sitio; la prueba `PROTO_PLANREAD` oscila porque cada fuente invalida a la otra).
2. **Aplicación**: cuando el RHS tardío es compatible con el tipo de la lectura, el `Bind/Unify` del elemento debe **fijar** el elemento (ruta compatible), no `defer_shape_conflict`; el conflicto diferido es solo para incompatibles (5530–5560, 5740–5810).
3. **Cierre**: `unresolved_constructor_diagnostic` debe usar el origen del **constructor propio** (`[]` del `var`), no `array_origin` de una capa sembrada (l.7784/4877), y un join cuyo lado pendiente se resuelve debe seguir a `compare_source_site` en vez de caer al fallback E0103.
4. FAM-2/ancla (aparte): el C de una hoja posterior contra el ancla debe quedar **condicionado** a que las hojas anteriores contra la misma ancla (pendientes) sean compatibles → registrarlo como obligación (junto al join pendiente), no `anchor_error` inmediato.
5. FAM-3 (aparte): el hermano independiente se visita siempre (marco `IndexSelector` con base `Result`).

### Riesgos medidos
- Salidas protegidas: RD 152/152 y corpus de 224 `.n` **no cambian** con ninguna ablación (ni siquiera con `ABL_SEED`), así que RD **no discrimina** la siembra; sí discriminan los tests listados arriba (21 con `ABL_SEED`, 1 con `ABL_READ`/`TRUNC`, 1 con `PROTO_FAM3`). TD/DG y `tests/f0005_b1b.rs` pasan en todas las variantes (no están entre los fallos; no se ejecutó `scripts/` de TD/DG por separado, solo el nextest completo).
- **Test que fija el síntoma**: `i07c_pending_nominal_domain::incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin` (tests/i07c_pending_nominal_domain.rs ~l.800–815) afirma `E0103,E0103,E0102,E0102` para `...while [[r],q] {}q=[3];let a=(1<(if x then p else q));r=[3];}`; con `ABL_READ`/`PROTO_TRUNC` el checker da `E0102 q, E0102 x, E1303 q` y **ese** test falla. Es el patrón «E0103 con `[]` resuelto» que la adjudicación declara defecto: su esperado habrá que reabrirlo (laguna ADR / adr_gap pin, ver AGENTS «Lagunas»).
- `i07c_s4_context_regressions` (oráculo de be8c47c) cambia con `ABL_SEED` y con `PROTO_FAM3`.
- Pila F5-pila: no medida (no se tocó `drive`/`resume_non_match` en los prototipos salvo el marco `Index` de FAM-3, que añade un brazo en `resume_non_match`: riesgo real contra las puertas 204/256 KiB; sin medir).
- Oráculo de A: dos lecturas razonables (reserva ya anotada en la adjudicación): `ABL_SEED+ABL_READ` (derivación pura por orden fuente) da E1303 en `[r]` `[83,86)` + E0102 en el `3`; el gemelo da E1303 en `aar` `[74,77)`. **No decidido por la medición**; requiere Flujo A.

## 8. Lo que no se pudo medir / límites
- No se implementó un arreglo que verde en todos los casos; `PROTO_TRUNC` arregla A,F01,F02,O03,O05,F18 pero no F06,F16,F11,O01 (plan incompleto); no hay prototipo de FAM-2/ancla.
- No se ejecutaron campañas `twin 300000/100000` ni TD/DG por script ni debug (solo release); sin medición de pila.
- Los E0102 mal ubicados de B q→r bajo `ABL_SEED` (`[76,77)`) se deben a `read_plan` (medido con `ABL_READ`), no verificado en el resto de la muestra.
- Cobertura de `casos.json`: 9 casos medidos (7 trazados), no los 27.
- La copia (`C:\nexo-work\val\s5-diag\tree`) y `target-s5diag` se borran al terminar; el diff de instrumentación queda en D:.
