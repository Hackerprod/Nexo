# I07c-S5c B1 — informe parcial de C (PARADA, plan §9)

Subagente C (Sonnet, desvío por presupuesto, `docs/F0_ACUERDOS.md` §10). Rama local `i07c-s5c-b1` (`origin/main` 1e2e393 + B1-S0). Sin push. Por indicación del coordinador no se borra nada y los cambios de S1 quedan **sin confirmar** en el árbol como evidencia. Complementa `docs/correcciones/I07c-C-S5c-hallazgo.md`.

## 1. Qué se hizo

- **B1-S0 (commit 764d254, «EN VALIDACION, no PASS»):** fixtures de la matriz (62 fuentes en `docs/validation/i07c-s5c/matriz/<id>.n`, SHA-256 en `matriz.sha256`), oráculo y documentos de E importados de 783415e, insumos de `i07c-s5a` y `i07c-s5`, nota de desvío en `F0_ACUERDOS.md` §10, línea base de pila (`pila-linea-base.txt`) y `tests/i07c_s5c_anchor.rs` con 25 guardas (los 21 OK de `main`, X_two_unions, P12 y dos controles), 25/25 verdes sobre el árbol de `main`. Defectos de forma de ese commit: el fichero de tests no está pasado por rustfmt y trae `&source` innecesarios (clippy); el árbol de trabajo ya lo corrige.
- **B1-S1 (sin confirmar):** ALT-2 en `src/check/types.rs` según el plan §3 (registro `union_denounced`, `AnchorOutcome`, `anchor_open`, `anchor_contrast`, `second_leaf_anchor`, `anchor_absorbed`, `#[inline(never)]` en `if_after_first`/`if_finish_join`/`if_else_context`, retirada de la rama de descarte y del código `ShapeHint`), más la desviación del hallazgo (hoja `else` que es variable pendiente se contrasta, solo con unión registrada). También sin confirmar: retirada de `T0-group-shape` (`scripts/j16_mutations.py`, fila de `Pending.md`), reconciliación del auditor de spans y su aserción, `scripts/i07c_s5c_diferencial.py` y su manifiesto `corpus-7391045882.json` (121.059 programas, SHA-256 `9ea3ae40…a1a8`), y la versión con negativos de `tests/i07c_s5c_anchor.rs` (42 tests).
- **No hecho** (por la parada): S2, mutación inversa B1-M1..M8 (script preparado, no ejecutado), campaña twin, corpus de 3.674 fuentes, 152 RD, fuzz, pasadas de planning, informe final.

## 2. Mediciones hasta ahora

- **Pila (rustc 1.99.1, `/MAP`, `f5-frame-sizes.py`):** base `drive` 27.536 B, `resume_non_match` 33.080 B, pareja 60.616 B; S1 idéntico (27.536 / 33.080 / 60.616 B, crecimiento 0 B, margen +5.944 B). `f5-stack65.py` no se ejecutó.
- **Matriz (45 filas + nuevas), `check`, release, base frente a S1 con la desviación, comparado con el oráculo (N y relación):** ningún OK de `main` perdido. Pasan a coincidir con el oráculo: R4, X_R1na, X_R1ord, X_flat3, Y_pend_if, Y_pend_if2, X_R1inc, X_join_local, X_compat_then_absorbed, X_pend_second_leaf, P03, P04, P05, P15, P16, P17 y además (no previstas por el plan para B1) A, A_noasg, A_reord, F06, M2, O01, O03, O07, O08. No cambian W0, P1, P2, P3(I), c3, w1, w3, B_qr (defectos previos). `generated1681`, P4, P4pos y R5 cambian (también con la letra del plan; sin oráculo validado).
- **Tests nuevos:** 41/42 verdes en la primera pasada; el rojo era W0nounion por usar `accepted` con `Print` (corregido en el generador, no reejecutado).
- **Suite debug completa:** 2.209 ejecutados, 2.202 pasan, 7 fallan: `i07c_causal_obligations::{adr_gap_nested_union_and_array_element_keep_current_causal_sites, failed_outer_condition_keeps_nested_join_constructor, mixed_nested_join_keeps_constructor_and_shape_sites, pending_inner_constructor_preserves_outer_branch_comparison, reduced_independent_sites_remain_causal}`, `i07c_pending_nominal_domain::failed_nested_union_does_not_anchor_an_outer_pending_array` y `i07c_structural_invariants::grouped_operands_keep_one_causal_primary` (este último, líneas del auditor desplazadas por una edición posterior). Con el oráculo: en las cuatro primeras la salida de S1 coincide con el oráculo (desaparece un E0103 que la unión ya fija, o un E1303 absorbido) y el test fija el comportamiento anterior; `reduced_independent_sites…` y `failed_nested_union…` revelan regresiones de S1 (§3), no se tocó ningún test existente.
- **Diferencial parcial** (`f2`+`perm2`+guardas T5, 2.223 programas, release, `check`, base frente a S1): 578 MEJORA DERIVADA, 26 MEJORA PARCIAL (el E0103 residual alrededor de `x=[]` ya estaba en `main`), 221 y 6 previos sin cambio, 0 DEFECTO. La familia con hojas `[[q]]`, `[q]`, `r[0]` anidadas y consumidores no estaba en mi alfabeto; el diferencial de `f3` se interrumpió sin resultados.

## 3. Regresiones (la familia de B y las halladas por mí)

Reproducidas con los binarios (`base`=main, `lit`=letra del plan, `b1`=con la desviación; oráculo de E). Prefijo `fn Main(){let p=true;let ar=[1];var q=[];var r=[];`:

| Programa | main = oráculo | `lit` | `b1` |
|---|---|---|---|
| `let a0=(if p then (if p then r[0] else r) else q);` | solo E0103 `[38,40)` | = main | añade E0103 `[47,49)` y `[89,90)` |
| `let a0=(if p then (if p then r else 1) else q);q=[[ar]];` | solo E1303 `[86,87)` (`1`) | E0103 `[47,49)` + E1303 sobre `q` `[94,95)` | igual que `lit` |
| `var r=[];let a2=(if c then r else true)[0];` (otro contexto) | solo E0103 | E0102 `not_indexable` sobre el `if` | igual |
| `var r=[];var q=[];let a=(if true then r else (if false then [[q]] else [[1]]));r=[3];q=[3];` | `main` solo E1303 `[[q]]` (el oráculo: E1303 `[[1]]` + E0102) | E0103 `[]` de `r` y de `[q]` nuevos | igual (con `q=[3];r=[3];` = oráculo) |

## 4. Causa y ablación

La regresión de B (filas 1 y 2) y el E0102 espurio (fila 3) tienen **una causa**: el retorno temprano de `if_finish_join` cuando `anchor_pending || anchor_absorbed` devuelve `(first.or(second), true, true)`. Si la primera hoja de la unión interior es una variable o un constructor pendiente, `first` es `None` y el tipo del `if` pasa a ser el de la **segunda** hoja (`1`, `true`), de modo que la unión exterior pierde el **testigo parcial** de la interior (`first_for_anchor`, ADR §4.1.2.5) y toma como ancla un tipo equivocado (o un consumidor ve `Bool`). Con la letra del plan ese retorno se extiende a la hoja `else` en estado `Open` con conflicto diferido (la fila 2); con la desviación, además, a la hoja `else` pendiente contrastada (la fila 1). `main` ya tenía ese retorno solo para la primera hoja con ancla de contexto.

**Ablación hecha** (copia local `.scratch/lit-src`, una compilación release, nada más): sustituir en ese retorno `first.or(second)` por `first_for_anchor.or(first).or(second)`. Resultado: las dos filas de B y la fila 3 vuelven a la salida de `main`/oráculo, y las 49 filas de la matriz siguen coincidiendo con el oráculo (sin DEFECTO). No se ejecutó nada más (ni suite ni diferencial), así que no consta si basta, ni qué hace con la fila 4.

La fila 4 (`[[q]]`/`[[1]]`, orden `r=[3];q=[3];`) es otra causa: depende del orden de las asignaciones (con `q=[3];r=[3];` coincide con el oráculo), es decir de la siembra de `concrete_plan` de `main` (evidencia adelantada de la asignación que el diseño §1 atribuye a V6/B2), que B1 no retira; no es efecto de la ablación anterior y queda abierta.

## 5. Archivos en el árbol (evidencia, sin confirmar)

`src/check/types.rs`, `scripts/i07c_span_audit.py`, `scripts/j16_mutations.py`, `scripts/i07c_s5c_diferencial.py`, `docs/validation/i07c-s5c/corpus-7391045882.json`, `docs/correcciones/I07c-C-S5c-hallazgo.md`, este informe, `Pending.md`, `tests/i07c_s5c_anchor.rs`, `tests/i07c_structural_invariants.rs`. Fuera del control de versiones, en `C:\projects\Nexo-i07c-c\.scratch\`: binarios `bin/{base,lit,b1,abl}-release.exe`, `lit-src` (copia con la ablación), `mut/run_mut.py`, generadores `gen/`, registros de pila (`frames-base.txt`, `frames-s1.txt`) y los targets `target-*` (≈ varios GB; no se borraron).
