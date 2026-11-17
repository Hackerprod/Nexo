# I07c S1 — informe de implementación para D

Fecha: 2026-11-17. Base de código: `e6e5f82`; base documental y rama local: `bb40c55` (`i07c-estructural`). Sin commit ni push por instrucción del coordinador. La sonda de las causas sin traza se hizo en `D:\nexo-work\i07c-c-probe`; no contradijo las familias F1 del plan. Se conserva el hallazgo histórico `I07c-C-S1-hallazgo.md`, resuelto por las dos inserciones aprobadas en F0-0002 §4.1.2.3.

## Cambios

| Archivo | Cambio |
|---|---|
| `src/check/types.rs` | `InferenceGraph::register` registra las ocho clases de obligación con sitio fuente y estado; `OccurrenceState` reúne marcas, resultado condicional y procedencia nominal. `Checker` conserva el recorrido, cuotas, entrada y diagnósticos; los nodos, bindings, tipos por ocurrencia, planificación y obligaciones residen en el grafo. `capture_expression_dependencies` propaga las fuentes nominales por expresiones y aliases; `suppress_result_dependents` retira solo diagnósticos derivados de resultados refutados. `pending_nominal_available` conserva el testigo normal de `bottom_only`. El cierre de `DomainNominal` propaga dependencias fallidas hasta punto fijo y aplica primero las igualdades compatibles; las comparaciones guardadas conservan sus consumidores hasta decidir el dominio. La lectura de `Known.invalid` usa `get`, pues un nombre resuelto puede no tener binding activo. |
| `tests/i07c_deferred_constraints.rs` | Negativos y positivos de b2a/b2b/b2c/b2d/b2e/b2g/q4w, los dos pares independientes y los pares circulares de §4.1.2.3; comprueba código, subcausa, primario, token, etapa, operación, datos, relaciones y ausencia de campos inesperados. Comprueba supresión de C derivado sin perder C independiente. Fuzz dirigido reproducible: semillas 17021, 17022 y 10495373245766874321, 256 fuentes por semilla; `check_proposed`, `check_scalar` y `compile_scalar` en debug y release, sin pánicos ni errores internos. |

Los campos laterales `conditional_result_types`, `conditional_result_vars`, `failed_constructors`, `pending`, `pending_generation`, `has_pending_projection`, `pending_domain_calls`, `deferred_join_roots`, `branch_join_sites`, `recursive_join_roots`, `invalid`, `deferred_exprs`, `divergent_exprs`, `failed_join_exprs`, `failed_projection_results`, `context_rejections` y `partial_normal_exprs` dejan de existir en `Checker`; sus hechos están en obligaciones, ocurrencias, nodos o bindings del grafo. `tainted` y `numeric_domain_owner` quedan dentro de `InferenceGraph` como cursores transitorios del recorrido; no son mapas de veredicto ni se exportan. Los índices de planificación y deduplicación (`eligible_shapes`, planes, `nested_assignment_sites`, `preseeded_children`, `emitted_sites`) quedan dentro del grafo y se vacían/reconstruyen al iniciar cada función. D debe revisar especialmente su carácter derivado, las transiciones de obligaciones y el cierre compuesto según la tabla de 66 campos del plan; esta prueba mecánica no sustituye esa revisión.

## Repros antes/después

La base aceptaba los siguientes programas; S1 produce **un solo C** en `check` y `run`, con la misma salida en ambos modos. Los esperados provienen de F0-0002 §4.1.2.3 y §§13.15–13.17, no del binario.

| Fuente | Antes (`e6e5f82`) | Después, C único y primario |
|---|---|---|
| b2a | acepta | `NX-E0102/type_mismatch` `[67,113)` |
| b2b | acepta | `NX-E0102/type_mismatch` `[66,112)` |
| b2c | acepta | `NX-E0102/type_mismatch` `[66,113)` |
| b2d | acepta | `NX-E0102/not_indexable` `[148,149)` |
| b2e | acepta | `NX-E0102/type_mismatch` `[139,141)` |
| b2g | acepta | `NX-E0102/type_mismatch` `[144,147)` |
| q4w | acepta | `NX-E0102/type_mismatch` `[140,141)` |

Los pares circulares `Len(r)` y `1+Len(r)` rechazan con `NX-E0102/not_indexable` en `[67,68)` y `[69,70)`; sus positivos y los independientes (2)/(4) pasan. El consumidor derivado anterior a la refutación de dominio no emite un segundo C. B1a/b/c mantienen `internal_error` en esta solución; B1 y B3 son la frontera IR/local de patrón de S2, conforme a la aclaración 5 del coordinador. S1 no fija sus oráculos con tests nuevos.

## Mutación inversa y hashes

SHA-256 del blob base `git show e6e5f82:src/check/types.rs`: `ca98f61e9b9f04bf0d3419c2f1f16e763b10b943e5d1655ea3f688982a2785e5`. SHA-256 de `types.rs` corregido: `354cb74fae1547bcd9d51e1ce4a1f24a66162e05a1135bf818b74d8e52de749d`; test nuevo: `92342c71bb6e7d5f51cb45154acb897f55fce75a2603f888866aca10337cc8dd`.

| Inversión aislada, en copia desechable | SHA-256 mutante | Regresión que falla con mutante y pasa restaurada |
|---|---|---|
| Retirar `bottom_only` como testigo normal | `40d28b7c3e5bc0bd3367bf8a64c7c2250c1ebda054eb342231870d0c529083da` | `compatible_constraints_survive_indecision_and_reject_later_conflicts` |
| Retirar dependencia de procedencia nominal en supresión de C | `6e79e5f0f623dd66593591f011168daf2c92b64e82ad3186614cd4026b35664d` | `nominal_result_fixes_circular_domain_before_its_failure` (aparece C derivado extra en `[81,83)`) |
| Invertir activación del consumidor de dominio | `f832e28179beea3fb5e01e444a211302cc46a5ed2908c2de3df0cf92340dd1a1` | `nominal_results_reach_incompatible_consumers` |
| Recuperar la lectura indexada de local ausente | `5ba6b916eca1d7b5d892db91232f506c3013026c289cb0bf1bd7123e696ee135` | `nested_option_match_resource_and_mode_sweep` (pánico) |

Cada mutante se compiló y ejecutó por separado en release; después de cada uno se restauró el SHA corregido y el test pasó. Una primera variante de la segunda inversión no compilaba y **no** se cuenta como evidencia; se sustituyó por la inversión que compila y reproduce el C extra.

## Puertas y diferencial

`sh scripts/validate.sh` pasó sobre `git archive bb40c55` más los dos archivos S1 congelados, con `CARGO_TARGET_DIR=D:/nexo-work/target-i07c-c`, `TEMP=TMP=D:\nexo-work\tmp`, `--locked --offline`: fmt, Clippy `-D warnings`, todos los tests debug y release. `i07c_deferred_constraints`: 5/5 por perfil; `i07c_causal_obligations`: 60/60; `tests/f0007.rs`: 21/21 por perfil; RD: **152/152 pares `check`/`run`** y 0 diferencias de instantánea en ambos perfiles. Las regresiones de I10b para ticks **2/6/1** y **56/25/15**, T01–T14 de `j14_accounting.rs` y los límites de perfiles pasaron en ambos perfiles. `python scripts/objetivo.py`: **15/29**, ejecutan 01–15.

Se repitió `sh scripts/validate.sh` sobre el árbol de trabajo de entrega, con las mismas variables de entorno y Cargo sin red: salida 0, fmt, Clippy, debug y release verdes. Se repitieron `python scripts/objetivo.py` (15/29, 01–15) y `git diff --check` (salida 0).

Compilé por separado el candidato y `e6e5f82` (se forzó recompilación de la base al compartir `CARGO_TARGET_DIR`); SHA-256 de ejecutables: candidato `f23a0cc037ecb95d548d31648dbeb986af20866d7dd1059a2d88206737df27b9`, base `7d254f652eabe38e5ff71126dacd2ec0cc009b5ac48059d550a73d2926462316`. El diferencial de `corpus/pass`, `corpus/fail`, objetivos 01–15 y todos los `.n` bajo `tests/fixtures` abarcó **203 fuentes × 2 modos = 406 ejecuciones**, **0 diferencias** de código de salida y JSON. Además, las **152 expansiones RD × 2 modos = 304 ejecuciones** dieron **0 diferencias** contra la base. Total protegido: **355 fuentes y 710 ejecuciones sin cambio**. Las siete diferencias B2 anteriores son las correcciones normativas listadas arriba. `git diff --check` pasó.

`python scripts/i07c_verify_references.py` reporta tres nombres históricos de notas sin definición (`adr_gap_result_component_mismatch_keeps_written_expected_type`, `array_reaches`, `if_expr`); los tres faltan también en la fuente base, así que no se modificó ese arnés ajeno a S1.

## Pendiente tras S1

S2: exportación coherente de expresión/binding para B1, enlace del local de patrón B3, positivos §8.3 y ausencia de `InvalidIr`/`internal_error` en esa frontera. S3: eliminar la recursión por profundidad de tipo y medir 204/256 KiB. La campaña final de mutaciones y el fuzz masivo del corte corresponden al cierre después de S3; esta entrega contiene únicamente la mutación inversa dirigida y el fuzz del Bloque I. El coordinador repetirá repros y diferencial al rebasar sobre J16 antes de D, según su aclaración 1.
