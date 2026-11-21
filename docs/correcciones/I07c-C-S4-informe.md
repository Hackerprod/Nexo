# I07c S4.1 — informe de reanudación de C

Fecha: 2026-11-21. Rama `i07c-estructural`, HEAD `c7a1b4d` al iniciar; `git fetch` confirmó `0/0` frente a `origin/i07c-estructural-r2`. Sin commit ni push. Base funcional de la migración: `dd26cad` y enmienda r4 validada por B. SHA-256 del blob `HEAD:src/check/types.rs`: `c9c0d57a84118ed6c9fa508165a6cf1b8451f8274675fc288425c7ede23b1ae6`; archivo final de C: `252c55b2b0ab8fe59090e5dc9aef925d64b5ee293e7bce3629cd74e7c9a9a9cb`. Base `be8c47c` de la comparación: `aad0c50fc49c219f4d08a22e40bb36513de203644725b4a14ea405700e1079e3` para `types.rs`.

## Cambio y auditoría del plan

La implementación migrada ya estaba en HEAD. Esta reanudación cambia solamente instrumentación y aserciones bajo `#[cfg(test)]` de `src/check/types.rs`: la traza de `Frame::Enter(SolverRequest::Bind)` registra el ID efectivo desestructurado como `visit`, su padre y ambas generaciones, además de taint y dueño. Las cinco aristas vacías y `InheritedPayload` exigen ID distinto, parentesco y generación igual; cada padre conserva sus hechos al reanudarse. No se ha cambiado la política de producción, otra fuente, una fixture ni `tests/f0005_b1b.rs`.

Revisión del mapa §5 y de la tabla §3: `InferenceGraph` es el único dueño de `VisitFact`, `active_facts` y la pila `visit_generations`; `FactId` en continuaciones identifica entradas, sin copiar taint/dueño. `begin_visit` apila generación y `end_visit` retira solo esa generación. `fact`/`fact_mut` rechazan ID cerrado y consulta a una visita exterior suspendida. `drive` y `with_independent_fact` consumen el resultado antes de cerrar en éxito, error o cuota. La condición, sentencias, ramas, asignación, Array, Index, Binary aritmético/lógico, Call, constructores, TryAdd, IntrinsicOne y AssertEq usan las fronteras de herencia/fusión de §3; el diferencial protege sus salidas, incluido RD71 y las rutas B1b de retorno anticipado.

`SolverBind::child` y los cuatro `push_bind` solo encolan padre y sitio. El único despacho que crea el hecho hijo de `Bind` es `Frame::Enter`: `RequireUnion`, `UnionArrayA/B`, `UnionPayloadA/B` empiezan vacíos; `InheritedPayload` y `Ordinary` heredan. `EndFact` retira el hijo tras su continuación. En `AtomicEnd` el clon conserva el registro activo del padre; los relojes `Rc<Cell<u64>>` comparten avance entre clon y original, por lo que el rollback no reutiliza ID. Las pruebas de clon/rollback, cuota y reentrada cubren estos invariantes. No aparece una llamada recursiva nueva por profundidad de tipo en `bind`, `require_union`, `union_inner`, `unify`, `selection_type`, `resolved_syntax_type` o `concrete_with_depth`; la unificación directa de 126 Array pasó en hilo de 204 KiB.

Grep final: cero `VisitCursor`, `solver_snapshot`, `visit.tainted`, `visit.numeric_domain_owner` y `parent_visit`. La regex ampliada no encontró campos `prior_taint`, `lhs_tainted`, `left_tainted`, `right_tainted` ni `previous_owner` en `Checker` o marcos. Excepciones justificadas: `VisitFact.tainted/numeric_domain_owner` son entradas activas del grafo; `PendingDeferredJoin.numeric_domain_owner` es el dato duradero de la obligación, estampado antes de cerrar la visita y leído al drenar el join; `prior_taint` son variables locales leídas del grafo al decidir una fusión, y `right_tainted: bool` es un parámetro de decisión de `finish_independent_binary_rhs`, no estado guardado en un marco. Los flags de ocurrencia y obligación son duraderos y no duplican el registro activo. `git diff --check` y `cargo fmt --all -- --check` dieron exit 0.

## Oráculos de núcleo y pruebas

| Requisito | Resultado de C | Evidencia |
|---|---|---|
| Dos C independientes y positivos aislados | PASA | F0-0002 §4.1.2: `true+1` y `false+1` dan dos `NX-E0102/type_mismatch` en `[16,20)` y `[37,42)`; cada corrección elimina solo su C. `tests/i07c_deferred_constraints.rs`, suite debug/release. |
| Array diferido y constructor propio | PASA | Primer incompatible `1` da C en `[39,40)`, Bool/I64 y origen `[21,35)`; el autorreferente conserva `NX-E0103` en `[]` `[16,18)`. Positivos pareados en el mismo archivo y fixture `independent_unresolved_constructor_in_branch.n` en el diferencial. |
| Dominio, RD71 y selector reevaluado | PASA | RD71 conserva un solo C en `ar` `[136,138)`; RD80(b) conserva M sobre `true`; RD114 conserva solo M `[85,89)` y RD117 X sobre `q` más J sobre `true`, sin M. `i07c_rd_conformidad.py` reprodujo 152/152 pares en debug y release, cero diferencias de instantánea. |
| Generaciones, sentencias, reentrada, cuota y rollback | PASA | `drive_facts_close_after_successive_visits`, `fact_generations_preserve_outer_values_and_reject_closed_ids`, `deferred_selector_reentry_keeps_outer_facts_for_array_and_scalar_bindings`, `nested_visit_quota_closes_inner_facts_without_changing_outer_cause` e `i07c_survivor3_failed_atomic_union_preserves_quota_error`; cero hechos al cerrar la visita exterior y rechazo de ID antiguo. |
| Seis aristas de solver | PASA en prueba dirigida | `solver_bind_dispatch_applies_each_fact_boundary` inspecciona el ID que usa `Bind`, parentesco/generación, taint y dueño; los seis cambios inversos aislados fallaron en su aserción y restauraron el SHA-256 final. El veredicto Nextest del universo `default` se registra abajo. |
| TD/DG y B1b | PASA | Diferencial dirigido de 42 fuentes oficiales TD01–TD07 y DG01–DG08 con positivos/variantes y sin errores de sintaxis accidentales: cero diferencias de exit, stdout y stderr en `check`/`run` debug/release; concatenación SHA-256 `557de41c4f79f42999762276e26c8cffc2771b66d5c1a9a17ca5a3cc2966b474`. `f0005_b1b.rs` sin edición: 21/21 debug y 21/21 release. |

La puerta rápida final (`sh scripts/validate-fast.sh`) terminó exit 0: fmt, Clippy `-D warnings`, selección comprobada y 2097/2097 tests debug más 2099/2099 release del perfil `fast`. Es evidencia de iteración, no PASS de la puerta completa. Los tests de `src/ir/i10b_tests.rs` incluidos en ambas suites conservan los ticks normativos 2/6/1 y 56/25/15: función vacía, delta de asignación indexada, bloque independiente, bucle finito y variantes `Result` con/sin propagación. El medidor de `corpus/objetivo/` informó 15/29, sin programa nuevo esperado para S4.

El diferencial estricto contra los binarios de referencia B1b del commit `3100a29` cubrió 369 fuentes (217 corpus/fixtures y 152 RD), `check` y `run`, debug y release: cero diferencias de exit, stdout y stderr, cero divergencias debug/release, JSON inválido, `internal_error`, `InvalidIr` o timeout. SHA-256 ordenado de fuentes: `e639eee74fa50842c04f5d009a91e5c85b7335afbd1940359def62de0ffe517a`. El generador congelado de D (`fuzz-generador-D.py`, SHA-256 `328ba0488cc11f0fbc39184b4d9b571559b8a627bb3cb3985ca0a3faa3d57ef2`) produjo 5 × 960 fuentes, SHA-256 concatenado `c50d8c3265c3ac7de674b98a8e4224e26650be1653c41d4a69ba62f363a10a77`: cero diferencias y cero pánicos, errores internos o cuelgues. Ambas pasadas finales usaron binarios CLI con SHA-256 estable antes/después: referencia debug `6edf41c63b45fbd00bd7d0acf43901f5539d463a60fa5cfd10f2c68ae0930169`, release `e8aabcf7b94d3948cdad0d15ae74e8d847f8df68899b52c41bab58f9a9d6b112`; S4 debug `76af078b4fd9d72babc206009c2f83dbf6b2ae0bfcf43c08c67643b4296c2d75`, release `ae04c1078cfe96686eaa167b628e92da10ab6fa731d36c6d4b1c9f5ad6a6a0de`. La comparación no reintentó timeouts.

## Pila y tamaños

`rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1`, MSVC, seis workers, copia limpia `be8c47c` y S4 con el mismo arnés `scripts/i07c_stack_probe/probe.py` (SHA-256 `37940070a7fa1cb0b646e14cb98c84a14d9ffbf6327b9e344fd21c4d13237f43`). Perfil test con debuginfo: `RUSTFLAGS` vacío y `CARGO_PROFILE_TEST_DEBUG` sin definir; sin debuginfo: `RUSTFLAGS='-C debuginfo=0'`, `CARGO_PROFILE_TEST_DEBUG=0`. En cada árbol/perfil se compiló `cargo test --lib --no-run --profile test` y `cargo build --bin nexo --profile test`. Cada tamaño candidato 65/129/193/257 KiB se ejecutó cinco veces por fila en TYPES y CHECKER. Todos los códigos de fallo de pila fueron `3221225725`; los candidatos que pasan lo hacen 5/5.

| J16-S1C | Con DI | Sin DI |
|---|---:|---:|
| some84 | 193 | 193 |
| some126 | 193 | 193 |
| array126 | 129 | 129 |
| array_bool126 | 193 | 193 |
| none_array126 | 193 | 193 |
| option_match126 | 129 | 65 |
| result_match126 | 129 | 65 |
| some_pattern126 | 129 | 65 |

Cada celda es el primer candidato que pasó 5/5 **idéntico en base y S4 y en TYPES y CHECKER**. En especial, con DI `option_match126` y `result_match126` fallan a 65 y pasan a 129 KiB en ambos árboles; sin DI pasan a 65 KiB en ambos. Las ocho filas pasaron además a 204 KiB debug y 256 KiB debug/release, en ambos modos, con diagnósticos iguales al hilo amplio. La serie S3 de 65 KiB sin perfil identificado queda solo como antecedente conforme a r4; se conserva el coste de marcos medido en `I07c-C-S4-diagnostico-pila.md`.

| Árbol/perfil test | SHA-256 ejecutable de test | SHA-256 `nexo.exe` |
|---|---|---|
| `be8c47c`, DI | `2da94a5dc858cceeb52da57a29f2e74ef49925fe4560ee20100ca8d070297723` | `21d140620bac255b396e7ebefbbfaf4d51087309d012cd47d9a76eade05f321c` |
| S4, DI | `8758109736273b80b6960e165ce2ec8d31ee4aa12ca0735ae08c71cae1c2d7be` | `b41ab98b91a02a2668a69962dabdb0a87e8233400c6751afc7fef5d3a8bafc03` |
| `be8c47c`, sin DI | `a7971eacff2fd9391e0595ec875566d3f249c28ec7212604f0943b60db4cbbe3` | `2ece63e229150f08b2dc11d0591efee4126135e1d352f9b97cd8158e7306c6ff` |
| S4, sin DI | `9e313bd4e90470ed367f0e1ce5bb6f9a56a78a43bd87d2a1ae7353f6d8876e92` | `365a0704710631b6a2705570f84cad38c7a19a7b961c47e4a9875b69fa9b2dab` |

`rustc -Z print-type-sizes` midió la librería sin `cfg(test)` en dev y release; los dos perfiles dan los mismos tamaños de tipos. Los perfiles de test con `cfg(test)` se midieron por separado y no se mezclan con producción.

| Tipo, bytes | Base producción | S4 producción | Base `cfg(test)` | S4 `cfg(test)` |
|---|---:|---:|---:|---:|
| `SolverBind` | 184 | 136 | 248 | 136 |
| `SolverUnify` | 208 | 184 | 240 | 184 |
| `SolverUnion` | 88 | 64 | 120 | 64 |
| `SolverUnionJoin` | 640 | 616 | 672 | 616 |
| `SolverRequest` | 208 | 184 | 248 | 184 |
| `solve::Frame` | 648 | 624 | 680 | 624 |
| `drive::Frame` | 88 | 88 | 88 | 88 |
| `InferenceGraph` | 1160 | 1248 | 1160 | 1248 |
| `Checker` | 1608 | 1696 | 1608 | 1696 |
| `VisitCursor` / `FactId` | 32 | 8 | 64 | 8 |
| `VisitFact` | — | 72 | — | 72 |

Variantes heap del `solve::Frame` (bytes de payload de variante, iguales en dev y release de producción): `UnionAfterBinds` y `UnionMerge` 647→623; `UnifyUnionNext` 311→287; `UnifyAfterUnion` y `UnifyAfterBind` 255→231; `Enter` 215→191; `BindUnionAfterRequire` 199→151; `UnionInner` 95→71. Sin cambio: `RequireUnionFinish` 143, `UnifyArrayDone` 87, `BindUnionFinish`/`BindArrayDone`/`UnifyUnionWrap` 55, `AtomicEnd` 15. `EndFact` nuevo ocupa 15. El máximo del enum baja 648→624 B. El crecimiento de `InferenceGraph`/`Checker` (+88 B) corresponde al registro y relojes, y no representa una copia de cursor en cada marco.

## Mutación inversa y límites

Cada sitio se mutó por separado en una copia desechable del árbol final: los cinco independientes pasaron de `Empty` a `Inherit` y `InheritedPayload` de `Inherit` a `Empty`. La prueba dirigida del despacho falló para los seis, sin fallo de compilación; entre mutantes se restauró y verificó SHA-256 `252c55b2b0ab8fe59090e5dc9aef925d64b5ee293e7bce3629cd74e7c9a9a9cb`.

| Sitio | SHA-256 mutante | Dirigido | Nextest `default` completo |
|---|---|---|---|
| `RequireUnion` | `260d6720cf245956e08b89e85cd17b95d78d1fb471fc792489fe7f334bb2ca3f` | FALLA aserción | MUERTO: exit 100, mismo test |
| `UnionArrayA` | `0d15480bff1bb3faad6b0b57431970cf5d6ed629a678e3f22fbf31b881c593f4` | FALLA aserción | MUERTO: exit 100, mismo test |
| `UnionArrayB` | `3c4a84e06a19634da4670e03f036f394d3654b6264c01ef088682ad6869a7322` | FALLA aserción | MUERTO: exit 100, mismo test |
| `UnionPayloadA` | `cef6091588c36508e269e48265ba6f41aebaf7bc896d60d8c389a001de1b7197` | FALLA aserción | MUERTO: exit 100, mismo test |
| `UnionPayloadB` | `9ccb31b51dca87db906f1b81cf62fc8c67229275432096bcaf14ffc88945f4ed` | FALLA aserción | MUERTO: exit 100, mismo test |
| `InheritedPayload` | `229ab075ea5318b5373cf084a39390e9996a5ded90fbe68692733c5e9c6abbb2` | FALLA aserción | MUERTO: exit 100, mismo test |

Para cada mutante, `cargo nextest run --profile default --retries 0` inicio el universo sin filtro y se detuvo en el primer fallo de `check::types::quota_tests::solver_bind_dispatch_applies_each_fact_boundary` (exit 100); no se uso la parte no ejecutada para declarar supervivencia. La fuente recupero el SHA-256 final tras cada intento.

No se ejecutaron `validate.sh`, `gate.sh` ni la campaña de mutación de población, por instrucción del coordinador. La puerta completa en clon limpio, la campaña única de núcleo y la validación independiente D/D' quedan para los roles correspondientes. No se detectó salida protegida que exigiera volver al plan ni se abrió un nuevo hallazgo. El worktree se entrega sin commit ni push.
