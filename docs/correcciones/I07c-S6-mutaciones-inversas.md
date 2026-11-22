# I07c-S6: mutaciones inversas dirigidas (plan §3.5, 15 filas)

Árbol medido: `git archive b7e9b27` en `C:\nexo-work\val\s6mut\tree` (copia desechable; `D:` solo para este informe y los JSON). `rustc 1.99.1`, `cargo-nextest 0.9.152`, perfil `default`, `--retries 0`, parada en el primer fallo. Una inversión por vez; el fichero se restauró por SHA-256 tras cada una (columna «rest.» = SHA restaurado igual al inicial; en todas fue sí).
SHA-256 iniciales: `src/check/types.rs` = `d5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6`; `src/check/names.rs` = `d38aef00ab7e288b3a1af58ca4d40beaf5d0cf3e9ae0da3eb110e1fbc041fdfc`. Parches (JSON con `file/before/after/occurrence/sha_initial/sha_mutated`) y resultados: `D:\nexo-work\mut-s3\s6mut\<ID>.patch.json`, `<ID>.result.json`, `<ID>.verdict.json` (+ logs).

## Resumen ejecutivo

1. **Inversión 1 (`primary_span`), la que C probó: el test `grouped_operands_keep_one_causal_primary` NO la distingue y es un hueco real del test, no equivalencia.** La suite completa mata el mutante (debug, 101 s) pero lo mata un test previo (`ir::i07c_tests::adr_gap_resolved_variable_within_element_keeps_current_union_site`), no uno de S6. Hay programas que el binario correcto y el mutante resuelven distinto (ver §3): `Len((q))`, `if … else (true)` y `(true)+(true)`. El test S6 solo ejercita restricciones de operando/elemento con **un** grupo en una posición; ninguna de las tres rutas afectadas. Propuesta de test en §4 (verificada: pasa en base, muere con el mutante).
2. De las 15 filas, **7 (1, 2, 5, 8, 11, 12, 15) mueren con su propio test** con la inversión del plan en el sitio indicado; **7 (3, 4, 6, 7, 9, 10, 13) solo mueren con su propio test tras re-situar la inversión** (el sitio que nombra el plan no es el que alcanza la fuente: funciones envoltorio o rutas redundantes, §2; la fila 9 exige invertir los tres sitios a la vez); **1 (fila 14) no la mata su test**. Ningún mutante es «NO COMPILA».
3. Redundancia (defensa en profundidad) en el código de producción: varias reglas se comprueban en 2–3 sitios; una inversión simple en uno solo sobrevive aunque el test sea bueno (filas 6, 7, 9). Se documenta qué combinación o qué sitio común las mata.
4. Lo no medido está marcado: **suite completa de los supervivientes dirigidos M01b, M03, M04, M09c** (ver §1.1): el cerrojo `heavy.lock` lo retuvo otra sesión (`i07c-s5a-proto`) durante más de media hora y no lo forcé ni lo borré.

## 1. Tabla de las 15 filas

Veredicto de la fila = el del mutante «representante» (negrita), que es el que cae en el código que realmente ejecuta la fuente del test. Las demás inversiones de la fila se listan en §1.1.

| # | Test nuevo (fila del plan) | Sitio invertido (representante) y línea | SHA-256 inicial → mutado (16 hex) | Resultado |
|---|---|---|---|---|
| 1 | `concrete_array_twin_accepts_matching_assignment` | **M01a** `compare_source_site` (`types.rs` 2037): `if left == right` → `… \|\| (*left == Bool && *right == I64)` | d5046aed… → 4fa970c4… | **MUERTO** por `i07c_normative_literals::concrete_array_twin_accepts_matching_assignment` |
| 2 | `independent_empty_array_survives_failed_nominal_domain` | **M02a** drenaje de cierre de función (`types.rs` 7840): `(!self.graph.failed_constructor(root) \|\| …)` → `(self.graph.failed_constructor(root) \|\| …)` (suprime el NX-E0103 del `[]` independiente) | d5046aed… → a31d87cf… | **MUERTO** por el propio test |
| 3 | `literal_alias_constraints_accept_equivalent_source_orders` | **M03b** `solve`/Bind concreto (`types.rs` 5534): `if previous != ty` → `if previous == ty` (rechaza igualdad compatible) | d5046aed… → 51194ca0… | **MUERTO** por el propio test. (M03 literal del plan en `UnionAfterBinds` 6388: superviviente dirigido, no alcanzado; suite completa NO MEDIDA, §1.1) |
| 4 | `td_alias_conflicts_have_cli_rejection_contract` | **M04c** `compare_source_site` (2037): aceptar `expected I64 / found Bool` | d5046aed… → faa7325d… | **MUERTO** por el propio test. (M04 y M04b: no alcanzados, §1.1) |
| 5 | `never_joins_run_to_unit_without_observations` | **M05** `solve`/Unify (5698): brazo `(Never, other) \| (other, Never)` pasa a tener guarda `!matches!(other, Bool \| Array(_))` (Never deja de ser fondo) | d5046aed… → fb55be2c… | **MUERTO** por el propio test |
| 6 | `bottom_numeric_operands_accept_literal_one` | **M06d** `compare_source_site` (2037): aceptar `expected I64 / found Bool` | d5046aed… → faa7325d… | **MUERTO** por el propio test. Los sitios «de dominio de `+`» que cita el plan (M06a 9392, M06c 9474 contexto I64 del operando derecho; M06b 12134 mismatch del operando izquierdo) **sobreviven al test**: M06a y M06c los mata la suite completa por otros tests (`check::tests::divergent_left_keeps_independent_numeric_operand_sites`, `check::tests::branch_restores_dependency_taint_after_invalid_return_expression`); M06b sobrevive a la suite completa debug y release (§1.1, §5) |
| 7 | `bottom_array_operands_accept_matching_last_element` | **M07c** `compare_source_site` (2037): aceptar `expected I64 / found Bool` | d5046aed… → faa7325d… | **MUERTO** por el propio test. Inversiones simples del código de elementos de array (M07a 12442, M07b 12712) **sobreviven al test**; la doble M07ab (a+b) lo mata. M07a la mata `check::tests::deferred_array_conflicts_keep_the_complete_source_operand_type`; M07b la mata el test S6 `grouped_operands_keep_one_causal_primary` (§1.1) |
| 8 | `rd87_run_emits_only_the_literal_print_observation` | **M08** `check_intrinsic_domain` (3922): `domain_accepts(domain, got)` → `… && !matches!(got, Array(_))` | d5046aed… → 22aba0dd… | **MUERTO** por el propio test |
| 9 | `previously_open_root_fixed_to_option_rejects_result_pattern` | **M09abc** (triple): `require_match_pattern_shapes` 13301 y 13341 + `pattern_type` 13438: admitir `Ok` con scrutinee `Option` | multi-sitio (SHA de M09: 497f2125…) | **MUERTO** por el propio test **solo con los tres sitios a la vez**. Cada uno por separado (M09, M09b, M09c) **sobrevive** (M09 y M09b: suite completa debug+release VERDE; M09c: dirigido) y M09+M09b también sobrevive: redundancia triple, ver §5 |
| 10 | `rd_causals_determine_rejection_with_optional_leaves` | **M10b** `branch_accept` (14144): descarta el primer C de ramas (`if progress.value_failed { record_error }`) conservando los posteriores (D) | d5046aed… → 695f3b8e… | **MUERTO** por el propio test. (La inversión del plan «selección en array» M10, 12504, no está en la ruta de RD83; la mata `concrete_array_twin…` y `bottom_array_operands…`) |
| 11 | `repeated_check_and_run_keep_diagnostics_deterministic` | **M11** `names.rs` 1301 `sort_diagnostics`: `sort_by(\|a, b\|` → `sort_by(\|b, a\|` (orden inverso) | d38aef00… → 42d43e70… | **MUERTO** por `i07c_structural_invariants::repeated_check_and_run_keep_diagnostics_deterministic` |
| 12 | `diagnostic_relations_have_valid_distinct_expected_spans` | **M12** `mismatch` (959): relación `ExpectedType` con `found_span.clone()` en vez de `span` | d5046aed… → 21a716d6… | **MUERTO** por el propio test (el literal §4.1.2.3 se evalúa por CLI, no es un JSON sintético) |
| 13 | `rd_optional_leaves_keep_the_causal_rejection` | **M13b** = M10b (misma inversión: `branch_accept` 14144) | d5046aed… → 695f3b8e… | **MUERTO** por el propio test. (M13 literal del plan, `array_accept` 12669, no está en la ruta de RD83: superviviente dirigido, no clasificado) |
| 14 | `grouped_operands_keep_one_causal_primary` | **M14** `primary_span` (7079): `while let Expr::Group(inner, _) = e {` → `… = e && false {` | d5046aed… → 10b837b8… | **El test NO la mata** (SUPERVIVIENTE dirigido). Suite completa: **MUERTO** (debug, 101 s) por `ir::i07c_tests::adr_gap_resolved_variable_within_element_keeps_current_union_site`: **insuficiencia del test S6**. Programas discriminantes y test propuesto en §3–4 |
| 15 | `literal_deep_assignment_exhausts_default_solver_budget` | **M15a** `charge_solver` (4303): `if self.solver_steps == limit` → `… && false` (omite el límite) | d5046aed… → 618a5e71… | **MUERTO** por el propio test. M15b (límite `*2`, 4299, d5046aed… → 1583be67…) también **MUERTO** por el propio test |

### 1.1 Otras inversiones ejecutadas (todas restauradas por SHA)

| ID | Sitio | Resultado dirigido | Suite completa (`scripts/mutant-verdict.py`, default, debug+release) |
|---|---|---|---|
| M01b | Bind `if previous != ty` → `… && !(previous == Bool && ty == I64)` (5534) | sobrevive (fila 1) | NO MEDIDA (cerrojo ocupado) |
| M03 | `UnionAfterBinds` `a_ty != b_ty` → `a_ty == b_ty` (6388) | sobrevive (fila 3) | NO MEDIDA (cerrojo ocupado) |
| M04 | mismo sitio, `&& false` (6388) | sobrevive (fila 4) | NO MEDIDA (cerrojo ocupado) |
| M04b | Bind `previous != ty && false` (5534) | sobrevive (fila 4) | NO MEDIDA |
| M06a | contexto I64 del operando derecho, sitio 1 (9392) | sobrevive (fila 6) | **MUERTO** (debug 7 s) por `check::tests::divergent_left_keeps_independent_numeric_operand_sites`; el test nuevo no es el que lo detecta |
| M06c | ídem, sitio 2 (9474) | sobrevive (fila 6) | **MUERTO** (debug 5 s) por `check::tests::branch_restores_dependency_taint_after_invalid_return_expression`; el test nuevo no lo detecta |
| M06b | `l != I64` → `l != I64 && l != Bool` (12134) | sobrevive (fila 6) | **SUPERVIVIENTE** (debug VERDE 275 s, release VERDE 243 s); sin programa discriminante (§5) |
| M07a | `array_next` sin referencia concreta en el último elemento (12442) | sobrevive (fila 7) | **MUERTO** (debug 8 s) por `check::tests::deferred_array_conflicts_keep_the_complete_source_operand_type`; el test nuevo no lo detecta |
| M07b | `array_accept` no registra el C del elemento posterior (12712) | sobrevive (fila 7) | **MUERTO** (debug 266 s) por el test S6 `i07c_structural_invariants::grouped_operands_keep_one_causal_primary` (caso `[1,(true)]`), no por el de la fila 7 |
| M07ab | M07a + M07b a la vez | **MUERTO** por `bottom_array_operands_accept_matching_last_element` | — |
| M09 | `require_match_pattern_shapes` 13301 (`\|\| true`) | sobrevive (fila 9) | **SUPERVIVIENTE** (debug 250 s, release 320 s) |
| M09b | `pattern_type` 13438 (`\|\| true`) | sobrevive (fila 9) | **SUPERVIVIENTE** (debug 328 s, release 386 s) |
| M09c | `require_match_pattern_shapes` 13341 (`kind != kind && false`) | sobrevive (fila 9) | NO MEDIDA (cerrojo ocupado) |
| M09ab | M09 + M09b | sobrevive | — |
| M09abc | M09 + M09b + M09c | **MUERTO** por el test de la fila 9 | — |
| M10 | `array_accept`: descarta `record_error` del hijo (12504) | sobrevive en RD83 (fila 10) | — (el sitio es real: lo mata `concrete_array_twin…` y `bottom_array_operands…`, ver M10-row1/M10-row7) |
| M10-row1 / M10-row7 | mismo parche M10 | **MUERTO** por los tests de las filas 1 y 7 | — |
| M10-row4 | mismo parche M10 contra fila 4 | sobrevive (no es su ruta) | — |
| M13 | `array_accept` pendiente: descarta el C (12669) | sobrevive en RD83 (fila 13) | — |
| M14p | M14 contra el test propuesto (§4) | **MUERTO** | — |

Los siguientes quedaron sin medir en suite completa: M01b, M03, M04, M04b, M09c, M10, M13 (cerrojo `heavy.lock` retenido por otra sesión; reintentos cada 60 s sin éxito durante > 30 min; no se tocó el fichero de cerrojo ni se paró ningún proceso ajeno). Su clasificación es «superviviente del test dirigido; no clasificado».

## 2. Por qué el plan §3.5 no se podía ejecutar tal cual

Para localizar el sitio real de cada fuente instrumenté temporalmente `record_error` con `#[track_caller]` y un interruptor por línea (copia restaurada por SHA, no forma parte de los mutantes). Hallazgos:

- Los nombres de función del plan (`unify_independent`, `compare_source_site` como «comparación», `bind_independent`, `union`) son envoltorios: `unify_independent` solo llama a `solve`, `union` es `#[cfg(test)]`. La comparación de hojas escalares vive en `compare_source_site` (2030) y en `solve` (Bind 5534, UnionAfterBinds 6388).
- Dónde se registra el C de cada fuente: `DV+true`, `true+DV`, `var v=DV;var w=v;v=3;w=true;`, `var a=[0];a[0]=true;` y toda asignación → `finish_assignment`/cierre de sentencia (`types.rs` ~8248, `record_error` de `outcome` de sentencia); `[DV,1,true]`, `[1,DV,true]`, `var r=[true];r=[1];` → `array_accept` (12511, `record_error` del hijo); RD83 → `branch_accept` (14145); `(true)==1`, `Len(x)` etc. según el caso. Por eso las inversiones «de dominio de `+`» o «de último elemento» en un único sitio no alcanzan el test.
- Varias reglas tienen **dos o tres comprobaciones independientes** (p. ej. el constructor `Ok` frente a `Option`: `require_match_pattern_shapes` por rama `Deferred/concreta` y `else if concrete_union(&current)` más `pattern_type`). Una inversión simple no cambia el resultado porque otra comprobación emite el mismo diagnóstico. Eso es robustez del producto, no debilidad del test, pero implica que el test de la fila 9 solo detecta el fallo conjunto.

## 3. Inversión 1 (`primary_span`): por qué el test no la distingue y qué sí la distingue

**Mutante M14** (`types.rs` 7079): `while let Expr::Group(inner, _) = e && false {` (el desempaquetado nunca ocurre; devuelve el span del grupo).

**Por qué el test pasa.** Medido con una traza en `primary_span` (`S6G`): el desempaquetado SÍ se ejecuta con las fuentes del test (2–8 veces por programa), pero ningún resultado de esas llamadas llega al primario ni a los campos que el test fija. Inferencia (no la comprobé llamada a llamada): en esas fuentes el C se emite al comparar el `Expr` interior, que ya no es un grupo, y el grupo solo se desempaqueta en llamadas cuyo resultado no se serializa. No es código inalcanzable (el binario cambia de salida con otras fuentes) ni otro código desenvuelve el grupo: **las fuentes del test no ejercen ninguna de las tres rutas siguientes donde el primario de un grupo sí se serializa**. Los usos reales de `primary_span` con grupo que cambian el diagnóstico son otros tres (batería diferencial base/mutante sobre 1800 programas dirigidos: 598 salidas distintas en `check`/`run`, todas en estas familias; clasificadas en `PRIMARY/CODE` 49 y `RELATED-ONLY` 90 para `f0-0002-b-v2 check`):

| Programa (perfil `f0-0002-b-v2`) | Binario correcto | Mutante M14 | Esperado transcribible |
|---|---|---|---|
| **D1** `fn Main(){let q=3;let a:Bool=Len((q));}` | NX-E0102/`not_indexable`, primario `[34,35)` (`q`), `data` `domain:array`/`I64`, relación `found_type` `[34,35)` | primario `[33,36)` (`(q)`); con `((q))` el correcto da `[35,36)` y el mutante `[33,38)` | Addendum `docs/adr/F0-0002-lagunas.md` «Primario de una expresión bajo agrupaciones» (se retiran todas las agrupaciones exteriores; rige «con independencia de la etapa» y para «argumentos»), sobre RD86(b) `F(let q=3;let a:Bool=Len(q);)` = X `[33,34)` `q` `domain:array`/`I64` R=— (§14.10), desplazado un byte por cada `(` |
| **D2** `fn Main(){let x=if true then 1 else (true);}` | NX-E1303, primario `[37,41)` (`true`), `expected_type` `I64` rel. `[29,30)`, `found_type` `Bool` rel. `[37,41)` | primario `[36,42)` (`(true)`) | Addendum (misma regla, vale para toda expresión primaria) + F0-0002 §13.15 fila NX-E1303 («`found_type`: operando incompatible de la continuación posterior») y §4.1.2.4 («una sola NX-E1303 sobre la primera continuación posterior incompatible») |
| **D3** `fn Main(){let x=(true)+(true);}` | **1** diagnóstico NX-E0102/`type_mismatch` `[17,21)` (`true` izquierdo), igual que `true+true` (`[16,20)`) | **2** diagnósticos: `[17,21)` y `[24,28)` | Addendum: «No cambia … el número de diagnósticos» y F0-0002 §4.1.2.1 cláusula 1 de N (grupos nunca amplían el primario) con RD80 («grupos transparentes en ambas posiciones», primer operando incompatible); N(d): una denuncia por operación. Es la discrepancia más grave: el mutante cambia la **cardinalidad** porque `left_domain_failed` y la deduplicación `numeric_domain_owner` comparan `error.primary_span == primary_span(left)` |

La batería compara `check` y `run` y marca cualquier diferencia de código de salida o de JSON. Salidas (binario correcto = `C:\nexo-work\val\s6mut\bin\base.exe`, mutante `…\bin\m14.exe`, ambos debug de la misma copia) medidas con `check` (D1 también con `run`: mismos diagnósticos, exit 1). Otra familia afectada (solo relaciones, `RELATED-ONLY`): `(true)==1` da relación `expected_type` `[100,104)` (hoja) en el correcto y `[99,105)` (grupo) en el mutante; el ADR permite el span de grupo en «roles relacionados» (addendum) y para las filas RD esa columna es I, así que **no la propongo como oráculo**.

## 4. Tests de S6 a revisar (propuestas concretas)

1. **`grouped_operands_keep_one_causal_primary` (fila 14): insuficiente. Añadir un test hermano** (verificado en la copia desechable: pasa en el árbol base (`b7e9b27`) y muere con M14; no se aplica al worktree):

```rust
#[test]
fn grouped_domain_union_and_operator_sites_keep_inner_primary() {
    // F0-0002-lagunas (primario bajo agrupaciones); §4.1.2.3 Len (RD86(b) agrupado);
    // §4.1.2.4 NX-E1303; N(d) una denuncia por operación.
    for (open, close, start) in [("(", ")", 34usize), ("((", "))", 35)] {
        let bad = format!("fn Main(){{let q=3;let a:Bool=Len({open}q{close});}}");
        assert_eq!(&bad[start..start + 1], "q");
        rejected(&bad, PROFILE, "NX-E0102", Some("not_indexable"), (start, start + 1),
            &[("expected_type", "domain:array"), ("found_type", "I64")],
            &[("found_type", start, start + 1)]);
    }
    accepted("fn Main(){let q=[3];let a:I64=Len((q));}", PROFILE);
    rejected("fn Main(){let x=if true then 1 else (true);}", PROFILE, "NX-E1303", None, (37, 41),
        &[("expected_type", "I64"), ("found_type", "Bool")],
        &[("expected_type", 29, 30), ("found_type", 37, 41)]);
    accepted("fn Main(){let x=if true then 1 else (1);}", PROFILE);
    rejected("fn Main(){let x=(true)+(true);}", PROFILE, "NX-E0102", Some("type_mismatch"), (17, 21),
        &[("expected_type", "I64"), ("found_type", "Bool")], &[("found_type", 17, 21)]);
    accepted("fn Main(){let x=(1)+(1);}", PROFILE);
}
```
   Medido: en `b7e9b27` + este test, 1/1 pasa; con M14, falla (M14p, MUERTO). Va en `tests/i07c_structural_invariants.rs`, sin tocar el auditor ni el ADR. El comentario del test existente que dice «solo restricciones» debe reconocer que el addendum rige para todo diagnóstico de expresión (el plan §3.5 ya lo advertía solo para los sitios 8283/12301/14613).
2. **Fila 7 `bottom_array_operands_accept_matching_last_element` y fila 6 `bottom_numeric_operands_accept_literal_one`:** detectan la inversión de la comparación de hojas (`compare_source_site`) pero no las de los sitios de contexto/registro (M06a/M06c/M07a/M07b), que las matan otros tests. No hay defecto: el comportamiento está cubierto por varios tests antiguos. No se pide cambio; si se quisiera que el test nuevo sea el que protege cada sitio hay que añadir fuentes donde la comparación del hijo no sea alcanzable y solo actúe el sitio concreto, lo que exige leer la ruta de ejecución por sitio (inviable como fuente de ADR).
3. **Fila 9 `previously_open_root_fixed_to_option_rejects_result_pattern`:** protege solo el fallo conjunto de tres comprobaciones redundantes. No requiere cambio.
4. **Filas 3, 4 (alias):** la inversión literal del plan en `UnionAfterBinds` (M03/M04) no está en la ruta de esas fuentes; mueren con el Bind (M03b) y con `compare_source_site` (M04c). Conviene que el comentario del test cite esos sitios y no «unify/union».
5. **Fila 13 `rd_optional_leaves_keep_the_causal_rejection`** y **fila 10:** correctas, ambas mueren con M10b/M13b (`branch_accept`).
6. **Plan §3.5:** corregir los sitios (§2) para filas 3, 4, 6, 7, 9, 10, 13.

## 5. Superviviente de suite completa: M06b, M09, M09b

- **M06b** (`l != I64 && l != Bool`, 12134): superviviente de la suite completa (debug y release). Inversión sobre un `return Err(mismatch(...))` seguido de `unreachable!("binary right operand is prepared by driver")`. Instrumentación (`eprintln!` en el brazo) y 8850 programas dirigidos (operadores `+ - * == != < and or` con operandos `true 1 v w DV [1] Len(a)`, con 0/1/2 grupos por lado, en `check` y `run`, perfiles `f0-0002-b-v2` y `f0-0003-b-v1`): **0 alcances**. Evidencia de rama inalcanzable (equivalente), no prueba formal.
- **M09 y M09b** (y M09ab): equivalentes **por redundancia**, no por inalcanzabilidad: los tres sitios (13301, 13341, 13438) emiten el mismo `NX-E1325/constructor_type`; invertir uno cualquiera no cambia nada observable, invertir los tres sí (M09abc muerto por su test). No hay programa que distinga una inversión simple.

## 6. Reproducción

- `C:\nexo-work\val\s6mut\mut.py <ID…>` aplica un parche de `mutants.json` y restaura por SHA-256; `C:\nexo-work\val\s6mut\verdict.sh <ID>` ejecuta `mutant-verdict.py` bajo `heavy-lock.py` (sesión `i07c-s6-mut`).
- Batería diferencial: `C:\nexo-work\val\s6mut\batt.py <mutante.exe> [N]`; análisis: `an.py`.
- Limitaciones: `mutant-verdict.py` usa 6 workers (no 4) por diseño; la medición ocurrió con otras dos sesiones activas en la máquina; la suite completa solo se midió en los mutantes citados.
