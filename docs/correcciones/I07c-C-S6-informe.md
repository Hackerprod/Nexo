# I07c-S6 — informe de C (solo pruebas; entrega sin commit)

Base local `e72b45f051885aad3f6a0191702e3e8a3925e9c5`, árbol Git
`b12f96bee1b5bebec50591b862a9ba0b2a119bce`; upstream al inicio 0/0.
El plan de S6 tiene PASS de B en `I07c-plan-B-S6-r4.md`. Solo se editaron los
ficheros de pruebas/arneses autorizados. No se tocó `src/`, el fixture RD ni
la matriz borrador. Los otros ficheros sin seguimiento observados pertenecen
a otras sesiones y se dejaron intactos.

## Matriz de 19 filas: esperado independiente y resultado sobre esta base

`F(body)` = `fn Main(){` + cuerpo + `}`; `DV` =
`(if true then {return;} else {return;})`. Las fuentes y los esperados se
tomaron de los apartados del ADR citados, no del binario. **Verde** significa
que pasó la prueba dirigida ejecutable; **I revisada** no eleva una instantánea
a obligación normativa. Ninguna fila se declara cerrada por este informe.

| Fila | Prueba / esperado ADR (sección) | Base |
|---|---|---|
| N-a.3 | `rd_causals_determine_rejection_with_optional_leaves` y RD: RD83 C `NX-E0102/type_mismatch` en `false` `[79,84)` incluso con hojas D; RD101 C `NX-E0103` `[16,18)` aunque omita V; control homogéneo aceptado (§§4.1.2 N(a), 14.10 RD83/RD101). | Verde; **PARCIAL**: añadir/quitar D/V manteniendo restricciones fijas NO EVALUADO. |
| N-e.2 | `repeated_check_and_run_keep_diagnostics_deterministic`: mismos bytes JSON/exit/stderr al repetir verbos con fichero idéntico; sin duplicación, orden de RD83/b2d/TD (§§4.1.2 N(e), 13.9, 14.10 RD83). | Verde. |
| N-i.2 | `diagnostic_relations_have_valid_distinct_expected_spans` y filtro RD: en el literal `r=[(r==r)==true];r=[1]` C en `1` `[39,40)`, origen `expected_type=[21,35)` y `found_type=[39,40)`; RD solo verifica límites/autorrelación, no rol AST (§§4.1.2 N(i), 4.1.2.3, 13.15). | Verde; evidencia estructural RD parcial. |
| I-a | `rd_snapshots_are_reviewable_without_reclassifying_c`: RD83 C1/D2 y RD101 solo U son referencia de presencia I, sujeta a N(j) (§§4.1.2 I(a), 14.10). | I revisada; sin delta. |
| I-b | Mismo test RD: relaciones/`data` de los 152 casos son instantánea I, no oráculo N por igualdad; gramática y relaciones imposibles sí bloquean (§§4.1.2 I(b), 14.10). | I revisada; sin delta. |
| I-c | `rd_optional_leaves_keep_the_causal_rejection`: RD83 C primer `false` y referencia D2 solo informativa; reparación de ambos `false` por `1` aceptada (§§4.1.2 I(c), 4.1.2.4–5, 14.10 RD83). | Verde para C/positivo; cardinalidad D I. |
| K1-d2 | `grouped_operands_keep_one_causal_primary`: hoja `v`/`true`, no paréntesis, en restricciones y positivos; auditor 12 sitios (§4.1.2.1, contrato de cierre §3.5). | Verde para restricciones. **NO EVALUADO** para 8283, 12301, 14613: Flujo A pendiente; no test rojo de analogía. |
| L3-15 | `concrete_array_twin_accepts_matching_assignment`: `var r=[true];r=[true];` checked/unit; gemelo `r=[1]` C `NX-E0102/type_mismatch` en `[26,27)`, `Bool/I64`, origen `[16,22)` (§4.1.2.3, tabla de gemelo; §13.15). | Verde. |
| L3-19 | `independent_empty_array_survives_failed_nominal_domain`: b2d + `let free=[]` conserva C `NX-E0102/not_indexable` `r` `[148,149)` y U `NX-E0103` `[]` `[174,176)`; controles de b2d y reparación (§4.1.2.3 b2d y constructor independiente; §13.17). | Verde. |
| K4-f | `rd_optional_leaves_keep_the_causal_rejection`: C1 sobre primer `false` `[79,84)`; D2 es presencia I; positivo con dos `1` (§§4.1.2.4–5, 14.10 RD83). | Verde para N; D informativo. |
| T-TD-ord | `literal_alias_constraints_accept_equivalent_source_orders`: TD01–03 positivos con asignaciones/aliases permutados aceptados (§14.10 TD01–03). | Verde para orden **de fuente**; inversión interna/representante **INCONCLUSA**. |
| T-TD-cli | `td_alias_conflicts_have_cli_rejection_contract`: TD01–03 y DG07–08: rechazo CLI único `NX-E0102/type_mismatch`, primario RHS final, `I64/Bool` o `Bool/I64`, origen del literal/anotación; exit 1 y canales vacíos; controles aceptados (§14.10 TD01–03; §14.11 DG07–08; §13.15). | Verde. |
| T-TD04e | `never_joins_run_to_unit_without_observations`: `(DV)==true`, `true==DV`, `[DV,true]`, `[true,DV]` checked/unit, canales vacíos; `DV+true` rechazo en `true` (§14.10 TD04/05 y pares de fondo). | Verde. |
| T-F1p | `bottom_numeric_operands_accept_literal_one`: ambos órdenes `DV+1` aceptados y `DV+true` rechazados sobre `true`, `I64/Bool`, `found_type` en token (§14.10 pares de fondo; §13.15). | Verde. |
| T-F2p | `bottom_array_operands_accept_matching_last_element`: `[DV,1,1]`/`[1,DV,1]` aceptados; último `true` rechaza `I64/Bool` y origen primer `1` (§§4.1.1, 4.1.2.4, 14.10 pares de fondo). | Verde. |
| RD-I | Filtro RD y `rd_snapshots_are_reviewable_without_reclassifying_c`: las 152 expansiones conservan C exactos; presencia/`data`/relaciones restantes se reportan como I (§§4.1.2 I, 14.10 regla de columnas). | **152/152** check/run; I revisada, delta I = 0. |
| RD-ejec | `rd87_run_emits_only_the_literal_print_observation`: cuatro RD87 checked/unit; solo RD87(c) `stdout=["[3]"]`, otros canales normativos vacíos; sin fijar `observations` de (c) (§§12.4, 14.10 RD87). | Verde. |
| P-8 | `previously_open_root_fixed_to_option_rejects_result_pattern`: variante de raíz abierta con `Option<I64>` previamente fijado; `Ok(n)` C `NX-E1325/constructor_type`, `[87,92)`, `constructor=prelude:Ok`, `scrutinee_type=Option<I64>`; `Some(n)` aceptado, perfil `f0-0003-b-v1` (§§3.2, 8.2/M05 de F0-0003, deducción del plan §3.3). | Verde. |
| Q-3 | `literal_deep_assignment_exhausts_default_solver_budget`: fuente de 93.307 bytes, 64 niveles, 15.400 asignaciones × 65 = 1.001.000; único `NX-E0003` en tipos con `type_solver_steps/compilation`, `limit=1.000.000`, `observed=1.000.001`, primario no vacío en fuente; control pequeño checked (`check`, grafía histórica `fn main`) (§§11.1, 11.8.A, 13.13). | Verde. |

## Auditor, RD, ejecución y límites

- `python -B scripts/i07c_span_audit.py`: exit 0, **12/12** sitios revisados individualmente; `pendiente_decision` imprime expresamente 8283 y 12301 con repro observado, 14613 sin repro. `--self-test`: PASS en copia temporal; un `.span()` crudo nuevo antes de `mod quota_tests` da exit 1 con `unreviewed raw span`. No convierte K1-d2 en PASS.
- `python -B scripts/i07c_rd_conformidad.py --verify C:/projects/Nexo-i07c/.scratch/target-s6/debug/nexo.exe`: exit 0, `RD normative: 152/152 check/run pairs`, `RD snapshot differences: 0`. `--self-test`: PASS (C faltante, tripleta inválida, sitios vacíos/fuera de fuente/duplicados, tipo pendiente, autorrelación y relación fuera de fuente rechazados; extra válido y `found_type=primario` aceptados). No se cambió el SHA del fragmento ni el fixture.
- `cargo nextest run --profile default -E 'test(i07c_normative_literals::) + test(i07c_structural_invariants::) + test(i07c_solver_quota_literal::) + test(i07c_rd_conformidad::)'`: **18/18** dirigidos, debug; incluye un lote adicional de ocho semillas explícitas sin pánico ni `internal_error`. `cargo fmt --all -- --check` y `git diff --check`: exit 0. `scripts/check-nextest-selection.py`: exit 0, los tres módulos nuevos registrados. No se lanzó la puerta completa, reservada al coordinador.
- `cargo clippy --all-targets -- -D warnings`: exit 0 bajo `heavy-lock.py` después de esperar a que los otros carriles soltaran el cerrojo; no se detuvo ningún PID ajeno.
- El `run` del control histórico Q de `fn main` no es oráculo de §11.8.A: `run` pediría `Main`; tanto el control como el límite se comprueban con `check` conforme a la receta. No se reinterpretó la salida del producto como esperado.

## Inversiones de reglas y SHA-256

**BLOQUEO DE PLAN §3.5 — una inversión compilada pero no distinguida por su
test; 14 sin ejecutar.** En una copia desechable bajo `.scratch/tmp-s6/mutant`
se invirtió solamente el bucle de `Checker::primary_span`: `while let
Expr::Group(inner, _) = e {` pasó a `while let Expr::Group(inner, _) = e &&
false {`. Blob inicial `src/check/types.rs` SHA-256
`d5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6`;
blob mutado `10b837b8dd4fd15c4fb1b48eff5fd08a52bec9c7dc1256d59cba8d44ba99f02a`.
`cargo nextest run --profile default -E
'test(i07c_structural_invariants::grouped_operands_keep_one_causal_primary)'`
compiló en el clon bajo `heavy-lock.py` y el test **pasó 1/1**. La inversión
no resultó distinguible por la fuente dirigida; **NO EVALUADO** para
sensibilidad y no se llama SUPERVIVIENTE porque no se ejecutó toda la suite
debug/release del mutante. La condición fue restaurada y el SHA-256 del clon
volvió a `d5046aed…b34682ca6`, idéntico al blob de `HEAD`.

Según el propio §3.5, esto requiere volver a A/B para revisar la fuente o
el mecanismo del plan antes de acreditar `grouped_operands_keep_one_causal_primary`;
no improvisé otra mutación ni modifiqué el esperado. Las restantes **14**
inversiones individuales de L/E/Q permanecen NO EVALUADAS. El arnés R tiene
autopruebas sintéticas adicionales, no sustituyen mutaciones del núcleo.
Blobs iniciales para la futura restauración por SHA-256:
`HEAD:src/check/types.rs` =
`d5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6`;
`HEAD:src/check/names.rs` =
`d38aef00ab7e288b3a1af58ca4d40beaf5d0cf3e9ae0da3eb110e1fbc041fdfc`.
Cada fila de §3.5 queda **NO EVALUADA** respecto de la sensibilidad exigida; la
suite verde dirigida no acredita su sensibilidad. No se reclama PASS de S6 ni
se entrega a D antes de completar esta puerta.

## Repros y pendientes normativos

No hubo prueba nueva roja por incumplimiento normativo tras calcular los
offsets de las fuentes D15; no se añadió `#[ignore]` ni se alteró el
esperado del ADR. Para la **laguna sin esperado decidido** (no un defecto
adjudicado), `fn Main(){for x in ([1]){}}` con
`check --format json --profile f0-0003-b-v1` da `NX-E0006/for_iteration`,
`types/types.check`, `pending_scope=later_adr`, primario obtenido `[19,24)`
`([1])`; la alternativa derivada por analogía sería `[20,23)` `[1]`, **no
normativa todavía** (F0-0003 §2.2). `fn Main(){let x=1;(x)(2);}` obtiene
`NX-E0111/non_callable_target` `[18,21)`; 14613 no tiene repro de rama.
Decisión por Flujo A antes de cualquier test rojo o corte `I07c-S6-COR`.

N-a.3 universal sigue PARCIAL; T-TD-ord interna INCONCLUSA; K1-d2 en esos
primarios NO EVALUADO. La matriz solo la actualizará el coordinador al cierre
con estos estados y la partición N/I; no declarar «19/19 N».
