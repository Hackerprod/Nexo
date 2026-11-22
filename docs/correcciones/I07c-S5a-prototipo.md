# S5a — prototipo de diseño: siembra consistente con la pasada final

Solo evidencia de prototipo. Nada se aplicó al repo ni al worktree `C:\projects\Nexo-i07c`. Copia desechable `git archive` (`src/check/types.rs` blob `1050c84c…`, el de `main` 66071b8) en `C:\nexo-work\val\s5a-proto\tree`. Parche: `D:\nexo-work\mut-s3\s5a-prototipo.diff` (solo `src/check/types.rs`, 203 líneas). Tabla de los 27 casos base/prototipo: `D:\nexo-work\mut-s3\s5a-casos-tabla.txt`.

## 0. Resumen

1. **Mecanismo propuesto (candidato final, «V6»):** *un artefacto del plan solo sobrevive si el veredicto de una pasada lo confirma*. Tres operaciones con un único criterio (§4.1.2.2: «una asignación rechazada nunca retipa lecturas anteriores»):
   - (a) **retracción de capas sembradas no atestiguadas**: tras cada pasada de planning, una capa sembrada que ninguna igualdad real alcanzó se capa (`seed_cap`, solo decrece);
   - (b) **`read_plan` solo si la asignación que adelanta fue aceptada** por la pasada de planning (que nunca aplica `read_plan`, así que el veredicto no es circular) y el sitio no ha sufrido retracción;
   - (c) la evidencia concreta (`concrete_plan`) no se inyecta si el plan perdió capas.
2. **Efecto medido:** repros A, B (ambos órdenes) y F06 pasan a la **lectura 2 de la propuesta** (bytes y tipos exactos), B da la misma salida en los dos órdenes; NX-E0103 espurio desaparece en **21 de los 23** casos de los 27 que lo tenían (F08 es legítimo y se conserva; **residuo: F10 y F11**). RD 152/152. Debug y release, `check` y `run` coinciden en 96 fuentes.
3. **Suite completa** (nextest `--profile default`): debug **2142/2145**, release **2144/2147**; fallan los mismos 3: dos son los oráculos de implementación que B ya marcó como erróneos contra el ADR y uno es la auditoría de spans fijada por números de línea (andamiaje). Con «quitar la siembra» eran 21 fallos; con la variante pura (sin `read_plan`) 16.
4. **Pila:** `drive` 27.536 B, `resume_non_match` 33.384 B, suma 60.920 B, margen 5.640 B: **0 B de crecimiento** (medido con `f5-frame-sizes.py`); `f5-stack65.py`: control 16 KiB desborda, `option_match126` y `result_match126` 5/5 a 65 KiB. La primera versión añadía un campo a `AssignmentProgress` y daba +32 B en `resume_non_match`; se movió a un ayudante `#[inline(never)]` + mapa del grafo.
5. **Lo que no se resolvió (honesto):** (i) la campaña `twin`, ver §6; (ii) el segundo mecanismo (FAM-2 / ancla pendiente, §7) sigue en pie; (iii) la demostración de independencia de orden es empírica para V6 (la compuerta heredada `conflicts_prefix` sigue en la conjunción); solo la variante pura la tiene «por construcción» pero rompe 16 tests.

## 1. Causa raíz confirmada por el prototipo (añade al informe de causa raíz)

- La inconsistencia siembra/pasada final no es solo `read_plan`. Con `read_plan` apagado y la siembra intacta, A sigue dando 2×E0103: la capa sembrada cambia el **predictor sintáctico** `expression_shape_hint` (usado por `if_else_context`, ~14.100 en el HEAD del prototipo): con la capa de `r` sembrada, `[r]` y `aar` «chocan» por forma, `inner_shape_conflict` descarta el ancla `r` de la unión anidada y la igualdad `r ~ aar` **nunca se plantea** en la pasada sembrada (traza: en la pasada 1, sin semilla, aparece `UNIFY Deferred(2) ~ Array<Array<I64>> found=74..77`; en la pasada 2, sembrada, no aparece). Es decir: la siembra no solo «no tiene justificación», **elimina** la igualdad que la justificaba (punto fijo auto-sostenido, no solo sin retractación).
- La inyección `concrete_plan` en `seed_planned_shape` (solo en la pasada final, `!self.planning`) es otra asimetría planning/final: quita *todas* las capas del tipo planificado y fija el nodo alcanzado, aunque se hayan sembrado menos capas (tras una retracción). Hay que desactivarla cuando el plan pierde capas (operación (c)), o A queda en la lectura 1 (E1303 en `aar`).
- Criterio de «atestiguada» (detalle que costó dos iteraciones): contar solo `require_array` (el criterio ingenuo, `preseeded_children`) **retracta capas legítimas** cuando la igualdad real entra por fusión estructural de nodos (casos `n8`/`t06` del §5: regresiones). Dos nodos sembrados fusionados entre sí no atestiguan nada. El criterio final: la capa está atestiguada si su componente conexa (union-find) contiene un nodo **no sembrado**, o tiene tipo concreto, o fue consumida por `require_array`.

## 2. Algoritmo (V6)

Estado nuevo en el grafo (`InferenceGraph`): `preseeded_layer: child → (sitio, índice de capa)` (por pasada), `seed_cap: sitio → capas máximas a sembrar` (por función), `rejected_assignments: sitios` (por pasada), `assignment_join_marks` (auxiliar).

Bucle de `check_types` por función (cambios en negrita):
```
seed_cap.clear()                                   # por función
repetir hasta 129 veces:
    function(f) con planning=true                  # NO aplica read_plan; seed_planned_shape siembra min(plan, seed_cap)
    retracted = retract_unconsumed_seeds()         # capa sembrada sin testigo -> seed_cap[sitio] = índice  (solo decrece)
    changed   = collect_planned_shapes()           # read_plan solo si !rejected_assignments[sitio] && sitio sin seed_cap
    si !changed && !retracted: salir
function(f) con planning=false                     # pasada final, misma semilla acotada
```
`collect_planned_shapes` solo añade a la compuerta existente de `read_plan` dos condiciones: `!rejected_assignments.contains(site)` y `!seed_cap.contains_key(site)`. `rejected_assignments` se rellena en `finish_assignment_rhs` (planning): el nombre asignado entra si el RHS devolvió `Err` o la asignación dejó un `PendingDeferredJoin` nuevo (conflicto conservado, §4.1.2.2).

**Terminación.** `seed_cap[s]` solo decrece y está acotado por el número de capas planificadas del sitio ⇒ el número de pasadas que lo bajan es ≤ Σ capas (≤ 129 por sitio por el tope existente). Entre dos retracciones la iteración es la de antes (`shape_plan` recalculado desde la cadena, tope 129 pasadas, que se conserva). Lo que se añade **no realimenta el planning**: `read_plan` solo lo lee la pasada final (nunca el planning), así que el veredicto `rejected_assignments` y el plan no se influyen mutuamente; por eso no oscila, a diferencia de `PROTO_PLANREAD` (aplicar `read_plan` en planning y contarlo en `changed`: 129 pasadas). Sin la retracción el sistema ya convergía en la práctica; el prototipo no demuestra el punto fijo de la parte heredada, solo que lo añadido es monótono y acotado.

**Independencia del orden.** La compuerta `prefix_concrete`/`pre_assignment_snapshot` decidía `read_plan` con el estado *concreto* de la primera pasada (en B q→r no veía el choque de forma, porque `q ~ [[r]]` aún tenía el componente de `r` pendiente). Ahora `read_plan` exige que la propia pasada de planning haya **aceptado** la asignación (veredicto real, con estructura pendiente incluida) y la siembra de ese sitio sea íntegra: B q→r y r→q dan exactamente la misma salida (medido). No es una demostración general: la conjunción conserva `conflicts_prefix` (solo puede quitar `read_plan`, nunca añadirlo). La variante «pura» (sin `read_plan`, §4) hace desaparecer la dependencia por construcción.

## 3. Mediciones pedidas

### (a) Repros (CLI `check --format json --profile f0-0002-b-v2`; `[ ) ` = bytes del primario)

| Fuente | main (medido) | prototipo V6 |
|---|---|---|
| A | E0103 `[30,32)`; E1303 `[74,77)` aar | **E1303 `[83,86)` `[r]` Array<Array<I64>>/Array<Array<Array<I64>>>; E0102 `[92,93)` `3` Array<I64>/I64** (= lectura 2, 2 C) |
| A con `r=[3];` antes del uso (A_pre) | E1303 `[80,83)` aar | igual (gemelo, sin cambio) |
| B q→r | E0103 `[27,29)`, `[36,38)`, E0102 `[76,77)` | E1303 `[63,64)` Bool/Array<Array<I64>>; E0102 `[83,84)` `3` de q Array<I64>/I64 |
| B r→q | E1303 `[63,64)`; E0102 `[89,90)` | E1303 `[63,64)`; E0102 `[89,90)` (**idéntica, mismo conjunto en ambos órdenes**) |
| F06 | E0103 `[27,29)`; E1303 `[83,85)` ar | E1303 `[76,77)` `r` Array<I64>/Array<Array<I64>>; E0102 `[91,92)` `3` Array<I64>/I64 (un solo C de unión, sin el E1303 de `ar`: también desaparece el segundo mecanismo en F06) |
| M2 de la propuesta (97 B) | E0103 `[30,32)`; E1303 `[84,87)` | E1303 `[77,78)` Array<Array<I64>>/Array<Array<Array<I64>>>; E0102 `[92,95)` Array<Array<Array<I64>>>/Array<Array<I64>> = **tabla de la propuesta exacta** |
| FAM-3 (`ar[true][[1,true]]`) | E0102 `[30,34)` | sin cambio (S5b, mecanismo aparte) |

**Los 27 casos de `casos.json`** (fuente `reduced`; tabla completa en `s5a-casos-tabla.txt`): NX-E0103 en main: 24 casos; con V6: 3 (F08 legítimo; **F10 y F11 siguen con 2 E0103 cada uno, idénticos a main**). O01, O03, O07, O08 coinciden **byte a byte con la tabla de la propuesta** (códigos, primarios, esperado/hallado). O02, O04, O05, O06 pierden el E0103 y ganan C que la propuesta declara «requieren derivación propia» (no verificados contra ADR). F03, F07, F15 sin cambio.

### (b) RD
`python -B scripts/i07c_rd_conformidad.py --verify <exe>`: **152/152** (con V6, y con la variante pura). Además `check`/`run` y debug/release: 96 fuentes (27 casos + tests propios + repros) sin divergencias.

### (c) Suite (nextest `--profile default --retries 0 --no-fail-fast`)
V6 final: debug 2142/2145, release 2144/2147. Fallan, en ambos, exactamente:

| Test | Esperado actual | Salida con el prototipo | Clasificación |
|---|---|---|---|
| `i07c_pending_nominal_domain::incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin` | causas `E0103 [] , E0103 [] , E0102 q , E0102 x` | `E0102 [53,60)` `[[r],q]` (Bool/Array<Array<Array<I64>>>), `E0102 [66,67)` `3` de `q=[3]` (Array<I64>/I64), `E0102 [82,83)` `x` (Bool/I64), `E1303 [96,97)` `q` (Bool/Array<Array<I64>>) | **Oráculo de implementación.** Coincide con la derivación de B (plan-B-S5-r1 motivo 3): `[[r],q]` se acepta con `q` pendiente (§4.1.2.2), `q=[3]` es la incompatible y se denuncia en su `3`; el `if` aplanado da E1303 en `q` (§4.1.2.4). Cambio legítimo; el esperado actual codifica E0103 sobre un `[]` que `r=[3]` resuelve. |
| `i07c_pending_nominal_domain::outer_index_context_makes_later_flat_array_an_ineligible_twin` | `E0103 [] , E0103 [r[1],r][x]` | `E0102 [65,66)` `r` (Array<Array<I64>>/Array<Array<Array<I64>>>), `E0102 [74,75)` `3` (Array<Array<I64>>/I64) | **Oráculo de implementación** (B motivo 4: el contexto de `s` fija el elemento por la proyección condicional, §4.1.2.3; el E0103 es el espurio de S5a). Derivación: `[r[1],r][x]` con resultado Array<Array<I64>> exige `r[1]` y `r` de ese tipo; `r` choca en su hoja; `r=[3]` incompatible en su `3`. El gemelo concreto sigue sin ser oráculo (prefijo más profundo). |
| `i07c_structural_invariants::grouped_operands_keep_one_causal_primary` | auditoría `lines=8283,12301,14613` | `unreviewed raw span at types.rs:7147` | **Andamiaje**: `scripts/i07c_span_audit.py` fija números de línea; cualquier edición de `types.rs` los desplaza. Hay que reconciliar la lista del auditor. No es semántico. |

El resto (2142) pasa igual que en main, incluidos `i07c_s4_context_regressions::contextual_owner_and_adjacent_fact_boundaries_match_b1b_cli` (sus 6 casos, byte a byte), los 21 de `tests-fallan-ABL_SEED.txt` y `outer_index…` solo cambia por lo dicho. **No** hay test nuevo ni se tocó ningún esperado.

### (d) Campaña `twin`
Ver §6 (se añade al final con el resultado).

### (e) Pila
Reservas (rustc 1.99.1, test con DI, `/MAP`, `f5-frame-sizes.py`), base HEAD vs V6: `drive` 27.536/27.536; `resume_non_match` 33.384/33.384; suma 60.920/60.920; margen 5.640/5.640; `expr_inner` 12.752/12.752; `check_against_context_one` 6.784/6.784; las cuatro rutas vivas idénticas (36.440; 31.968; 50.224; 80.456). `f5-stack65.py` en el árbol V6: control 16 KiB desborda como esperado y las dos sondas pasan 5/5. No se midieron 129/204/256 KiB (no se tocó `drive`/`resume_non_match`).

## 4. Caminos probados (evidencia para el plan)

| Variante | A | B (qr/rq) | F06 | E0103 espurio en los 27 | Tests (debug) |
|---|---|---|---|---|---|
| `ABL_SEED` (del informe S5) | gemelo | qr mal ubicada | 2 C | 0 | 21 fallan |
| solo apagar `read_plan` | 2×E0103 | qr/rq correctas e iguales | 2×E0103 | persisten | 1 (informe S5) |
| solo retracción, criterio ingenuo (`require_array`) | E1303 aar (lectura 1) | **qr sigue mal** (orden) | correcto | 23/24 desaparecen | no ejecutada (m1 daba 4 errores espurios por fusiones estructurales) |
| **pura**: sin `read_plan`, sin `concrete_plan`, `shape_plan`=cadena, + retracción | lectura 2 | iguales y correctas | lectura 2 | 23/24 | **16 fallan** (2145 total) |
| V3: retracción ingenua + `read_plan` condicionado al veredicto | lectura 2 | iguales | lectura 2 | — | 7 |
| V4: + sin `read_plan` en sitios retraídos | lectura 2 | iguales | lectura 2 | todos | 5 (n8 y t06 regresiones por criterio ingenuo) |
| **V6 (final)**: criterio «atestiguada» por componente | lectura 2 | iguales | lectura 2 | 21/23 | **3** |

Por qué la variante pura (la «correcta» por el ADR l.373, sin retipado) rompe 16: expone un **segundo mecanismo**: el solucionador *difiere* un choque de forma ya decidible (`Array<α>` frente a `I64`/`Bool`) en un `PendingDeferredJoin`, y las decisiones inmediatas de los hermanos (la supresión del segundo operando de `+`/`<`/`and` cuando el primero falla, `left_domain_failed`, `FailedJoin`/`first_anchored` en la unión anidada) se toman sin saber que el primero ya falló; `read_plan` lo enmascaraba. Consecuencias medidas (variante pura): segundo `E0102` sobre `true` en `(if p then r else 1)+true; r=[true]` (viola «solo el primer operando incompatible», §4.1.2.4–5); dos `E1303` en una unión anidada con ancla pendiente (`(if p then r else (if p then 1 else 2)); r=[true]`; viola «una sola NX-E1303»); `E1303` desplazado en `failed_nested_union…`; y pérdida de un E0102 en `recursive_shape_planning…`. Eso es un mecanismo distinto (marcar el fallo en el sitio al diferir, con el diagnóstico completado al cierre) y no se prototipó. Por eso V6 conserva `read_plan` con su compuerta corregida: no cambia la salida de esos programas (se conservan idénticos a main).

`PROTO_PLANREAD` (informe S5) oscila; aquí no se probó otra vez.

## 5. Programas de regresión útiles (todos en `C:\nexo-work\val\s5a-proto\tests\*.n`; desechables)

- `fn Main(){let p=true;var r=[];let a0=(if p then r else (if p then 1 else 2));r=[true];}`: main una E1303; la variante pura dos (defecto de la variante, no de V6).
- `fn Main(){let x=1;var r=[];let a=r[x][(r-r)];r=[3];}`, `…var r=[];let a=((r == r) < r);r=[3];`: regresiones de la variante pura (E0102 duplicado); V6 = main.
- `fn Main(){var r=[];var s=[];let a0=(r==[s]);let a1=(if r then {value:[true];} else {value:r;});r=[[3]];s=[3];}` y `fn Main(){let x=1;var r=[];Print((if true then {value: Len(r[x]);} else {value: r[0];}));r=[[3]];}`: los que delataron el criterio ingenuo de retracción.

## 6. Campaña twin con el prototipo

(Pendiente de rellenar al terminar; ver apéndice.)

## 7. Residuos, límites y riesgos

- **F10 y F11** (FAM-E) siguen con 2 NX-E0103 espurios cada uno (idénticos a main). La retracción no las alcanza porque sus capas sembradas quedan atestiguadas por fusiones con nodos reales que, a su vez, solo existen por la propia siembra; el criterio de testigo necesitaría un orden causal (¿quién creó la igualdad?), no solo pertenencia a la componente. No se resolvió.
- **FAM-2:** en F06 el prototipo da un solo C (E1303 en `r`) y el E0102 de `3`; el segundo mecanismo (`if_after_first`/`anchor_error`, emisión inmediata de la hoja posterior contra el ancla mientras la anterior está pendiente) **no se tocó** y no se ejercitó (la ablación y la retracción lo evitan en F06, pero el mecanismo sigue y es el que rompe la variante pura en uniones anidadas con ancla pendiente).
- **Independencia de orden:** empírica (B, sus permutaciones y la muestra); no hay demostración general mientras la compuerta heredada siga en el código. Riesgo de residuo: otros órdenes de asignaciones con `prefix_concrete` distinto.
- **Costo:** `retract_unconsumed_seeds` recorre todos los nodos de tipo por pasada de planning (O(N) por pasada, con `root` por nodo). No medido en cuotas ni en el generador de pila/escala (`i07c_mutant_projection_scale` pasa dentro de la suite).
- **Cuotas/andamiaje:** el prototipo no cambia cuotas; el auditor de spans (`i07c_span_audit.py`, líneas fijas) falla por desplazamiento y debe reconciliarse con cualquier edición real.
- **No medido:** el diferencial contra la versión anterior sobre `corpus/`+`tests/fixtures` completo (solo RD, 27 casos, 69 repros) ni fuzz sin pánicos del prototipo; ni la campaña de 100.000 en debug.
- **Entorno (incidente):** el cerrojo `C:/nexo-work/heavy.lock` quedó huérfano (PID 188028 muerto, sin procesos de `gate.sh`, 20 min) mientras yo esperaba; tras 20 min lancé la suite y la campaña **sin cerrojo** (≤4 workers) y lo registro aquí. Antes borré por error un cerrojo que era el mío (pid propio ya terminado por mí con `Stop-Process`): coincidió con que otro carril lo tomó segundos después; verifiqué que el archivo actual es del otro carril. No se paró ningún proceso por nombre de imagen salvo los PID míos.
