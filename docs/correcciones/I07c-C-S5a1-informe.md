# I07c-S5a.1 — informe de C (implementación del plan validado)

**Nota de proceso (desvío por presupuesto).** Este corte lo implementa un agente Sonnet de la sesión del coordinador porque el fondo de la API de Codex se agota. Es re-validable con Codex y la mutación de la región tocada es obligatoria (hecha, ver §8). D y D' serán agentes nuevos. Plan: `docs/correcciones/I07c-S5-plan.md` §2 fila S5a.1, §3 (V6), §7, §8 filas 1–2, §9, §10. Git de solo lectura: no hay commit; el árbol queda para el coordinador. **Hay un hallazgo abierto** (`docs/correcciones/I07c-C-S5a1-hallazgo.md`): la cláusula `failed` del veredicto de asignación (plan §3 punto 2, implementada al pie de la letra) cambia la procedencia de `expected_type` en dos programas aleatorios frente a `main`, donde `main` es el correcto. No me aparto del plan; la decisión es de A/B.

Base: `2875943` (rama `i07c-estructural`). `src/check/types.rs` base `54c9dfed…afd93`; candidato `07959d3f…8778a`. Toolchain 1.99.1; `CARGO_TARGET_DIR`, `TEMP` y todo lo generado bajo `.scratch/` del worktree.

## 1. Entrega (archivos)

Producción: solo `src/check/types.rs` (+108/−4). Pruebas: `tests/i07c_s5_seed_consistency.rs` (nuevo, registrado en `tests/suite.rs` con `#[path]`), `tests/i07c_pending_nominal_domain.rs` (las dos filas de §8), `tests/i07c_structural_invariants.rs` (solo la aserción de líneas del auditor). Auditor: `scripts/i07c_span_audit.py` (solo claves de línea y las del autotest; comentario de inventario). Evidencia: `docs/validation/i07c-s5/s5a1-*` (generador y manifiesto congelado, diferencial, adjudicación, mutación, salidas de las campañas twin). No se tocó `tests/f0005_b1b.rs`, `tests/fixtures/i07c_rd.json`, `corpus/`, `docs/baseline/`, `docs/adr/` ni otro test.

## 2. Qué se implementó (V6, sin copiar el prototipo a ciegas)

Reescritura con el estilo del código, sin instrumentación ni variables de entorno de prototipo. Cambios respecto del parche medido: `retract_unconsumed_seeds` pasó de dos pasadas con vectores intermedios a una sola (misma semántica: el mínimo por sitio no depende del orden de iteración del `HashMap`); comentarios mínimos con el argumento de terminación en el bucle de `check_with_partial_limit`; `assignment_join_marks` solo se escribe en planning (ya estaba dentro de `if self.planning`). Mecanismo, tal cual el plan: (1) `retract_unconsumed_seeds` tras cada pasada de planning, atestiguada = componente con nodo no sembrado, tipo concreto o consumida por `require_array`; (2) compuerta de `read_plan` con `!rejected_assignments.contains(site) && !seed_cap.contains_key(site)` (`note_assignment_verdict` es `#[inline(never)]`); (3) `concrete_plan` no se inyecta si `cap < origins.len()`. `seed_cap` se limpia por función; el tope de 129 pasadas se conserva. Los dos campos de `drive`/`resume_non_match` no se tocaron.

## 3. Tests existentes cambiados (§8 filas 1–2 y 3)

Clasificación: oráculo de la **implementación** (el esperado actual codificaba el `NX-E0103` espurio). Los esperados nuevos están derivados del ADR (§§4.1.2.2–4.1.2.4, 4.1.2.7, 4.7) y coinciden con la derivación de B del plan; el binario solo corrobora (coincide exactamente, no hubo discrepancia).

| Test | Esperado ACTUAL | Esperado NUEVO (derivado) | Diff |
|---|---|---|---|
| `i07c_pending_nominal_domain::incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin` (107 B) | `E0103, E0103, E0102 q, E0102 x` | `E0102 [53,60)` `[[r],q]` (Bool / `Array<Array<Array<I64>>>`); `E0102 [66,67)` el `3` de `q=[3]` (`Array<I64>` / `I64`); `E0102 [82,83)` `x` (Bool / `I64`); `E1303 [96,97)` `q` (Bool / `Array<Array<I64>>`); cero E0103. Derivación: `[[r],q]` se acepta con `q` pendiente (§4.1.2.2) y su condición `while` incumple Bool; `q=[3]` es la igualdad incompatible (se denuncia en su `3`); `r=[3]` completa el elemento de `r`; el `if` da C en la primera hoja posterior `q` (§4.1.2.4). El gemelo concreto no es oráculo (§4.1.2.7). | El test usa ahora un ayudante local `typed_errors` (código, subcausa, bytes y los dos tipos), tres negativos aislados (quitar cada C por separado: `x`→`p`, hoja `q`→`p`, `q=[3]`→`q=[[3]]`) y el gemelo concreto sin cambio. |
| `…::outer_index_context_makes_later_flat_array_an_ineligible_twin` (78 B) | `E0103 [] , E0103 [r[1],r][x]` | `E0102 [65,66)` `r` (`Array<Array<I64>>` / `Array<Array<Array<I64>>>`) y `E0102 [74,75)` `3` (`Array<Array<I64>>` / `I64`); cero E0103. El contexto de `s` fija el elemento por la proyección condicional (§4.1.2.3) y `r=[3]` es incompatible. | Mismo ayudante; el gemelo concreto sigue sin cambio. |
| `i07c_structural_invariants::grouped_operands_keep_one_causal_primary` | `expression_primary: 3 lines=8279,12484,14797`, `delegated_primary 7078`, `argument_target_primary 14811` | `8345,12576,14889`, `7141`, `14903` | Andamiaje: reconciliación (ver §9). |

Las filas 4 (S5a.2), 5 (cambio legítimo de salida sin editar) y 7 (19 que V6 pasa, sin editar) no se tocan aquí; la suite completa pasa (§10), con lo que las 19 y las dos fixtures de la fila 5 siguen verdes sin edición.

## 4. Regresiones nuevas (`tests/i07c_s5_seed_consistency.rs`, 17 pruebas listadas por nextest)

Todas con `check` y `run`, perfil `f0-0002-b-v2`, esperados derivados de las propuestas validadas (`I07c-laguna-esperado-diferido-r1.md` y `…-o04-o05-o06-r1.md`) y del addendum, spans por bytes comprobados contra la fuente (cada token se compara con `&source[a..b]`), código, subcausa, etapa, operación, tipos y relaciones exactas, sin `NX-E0103` donde el plan lo prohíbe. Cubren: A (96 B; control `r=[[3]]`; sin asignación, 90 B; orden `[r]` antes de `aar`; gemelo concreto como contraste y no oráculo; positivo del addendum; aislado solo-E0102), B (ambos órdenes con el mismo conjunto y positivo `q=[[3]]`), F06 mínimo + positivo + control, M2 y D-REF (97 B / válido de 80 B), O01, O03, O07, O08 (fuentes literales de `casos.json`, esperados de la tabla de la propuesta), constructor (`var r=[];r=[3];` acepta; `r=[r]` un único `NX-E0103 [16,18)`), protección de FAM-2 (`…if p then r else (if p then 1 else 2)…r=[true]`: una sola E1303 en `1` `[66,67)`; la segunda del prototipo, `(r==[s])`, no entró: su esperado derivado completo no estaba en el plan y no se fija el comportamiento actual), corpus 26+1 (ausencia del `NX-E0103` espurio en los 25 casos; F08 exacto `[108,149)`; F10 cota 2 por verbo; **F11 ya no conserva el residuo**, 0 E0103 en la base actual) y permutaciones de hasta 3 asignaciones finales de cada caso (≥20 variantes) sin `NX-E0103` espurio. Más `retracted_seed_site_keeps_its_element_fixed_by_a_later_compatible_leaf` (150 B), que añadí porque la búsqueda de mutantes exhibió M2b (§8): esperado derivado de §§4.1.2.2/4.1.2.4 (E1303 `[80,81)`, E0102 `[98,99)`, E1303 `[136,139)`, E0102 `[146,147)`; aislado quitando `r=[r]`; positivo pareado).

Sin la corrección (copia con el `types.rs` de `HEAD` y estas pruebas): **15 de 55 fallan** (`rev-run.log`: las dos editadas de §8 y 13 nuevas; cuatro pasan igual (aislado solo-E0102, contraste del gemelo, constructor y protección FAM-2), como corresponde: son sin cambio frente a `main`). Con la corrección: 55/55.

**No fijados aquí** (plan §7): O02, O04, O05, O06 (S5a.2) y los C esperados de F01–F19 sin derivación completa: solo se afirma la ausencia del E0103 espurio. La tabla «id → prefijo → C/U obligatorio» del plan no la derivé de forma independiente antes de implementar (el prototipo ya existía): dejo los C de F01–F19 a D, como pide el plan. Estado E0103 base→candidato medido sobre `HEAD`: O01–O08, F01, F02, F04–F06, F09, F12–F14, F16–F19 pasan de 1–2 E0103 a 0; F08 sin cambio (1, `[108,149)`); F10 sin cambio (2: `[83,85)`, `[151,154)`); F03, F07, F11, F15 sin E0103 en ambos.

## 5. Pila (rustc 1.99.1, perfil test con debuginfo, `/MAP`, `f5-frame-sizes.py`)

Comandos: `RUSTFLAGS='-C link-arg=/MAP:<mapa>' cargo test --lib --no-run --locked --offline` en copia `git archive HEAD` (base) y en el árbol (candidato); después `python -B scripts/f5-frame-sizes.py <exe> <mapa> --dumpbin '<dumpbin 14.44.35207>'`. Base: exe `3c6184b5…d50a4`, mapa `b8ec14f4…15ed3`; candidato: exe `725f8bfc…063ca`, mapa `d9dde41f…c60d95`. Resultado idéntico: `drive` 27.536 B, `resume_non_match` 33.080 B, pareja **60.616 B** (margen **+5.944 B**), `expr_inner` 12.720, `check_against_context_one` 6.784, `enter_binary` 1.904; rutas vivas 36.440 / 31.968 / 50.192 / 80.120 (iguales en base y candidato). Crecimiento **0 B** por marco y por suma. `python -B scripts/f5-stack65.py`: `option_match126` y `result_match126` 5/5 a 65 KiB; `python -B docs/validation/i07c-s5/s5b1-stack-probes.py` (reutilizado): ambos 5/5 a 129, 204 y 256 KiB (exe sin mapa `16c6ee75…b986`).

## 6. RD y TD/DG

`python -B scripts/i07c_rd_conformidad.py --verify <exe>`: **152/152** en debug (`cand-debug.exe` `5fb2a09f…a5e2`) y release (`cand-release.exe` `998e4fad…0a6c`), 0 diferencias de instantánea. Los TD/DG están en la suite (verde) y no se tocó ningún oráculo protegido.

## 7. Diferencial contra la base

Generador `docs/validation/i07c-s5/s5a1-corpus.py` (semilla **20261122**, **3.200** programas únicos dirigidos a Array/Option/Result, `if` anidado, uniones con referencias pendientes, asignaciones tardías en 1–3 sentencias, gemelos, proyecciones y condiciones sobre pendientes); manifiesto `s5a1-corpus.json` **congelado antes de ejecutar** (SHA-256 `0dc46ac23c85d76f7f5c18fef43ab79978e8d7249338b3f23322f074010d27b0`, codificación longitud-prefijada). Más: los 27 casos twin y todas sus permutaciones de las 2–3 últimas asignaciones, 15 repros del plan, las 217 fuentes de `corpus/` y `tests/fixtures/`, y las 152 de RD: **3.674 fuentes**, `check` y `run`, 3 perfiles, debug y release, base y candidato = **88.176 invocaciones** (`docs/validation/i07c-s5/s5a1-differential.py`; binarios base `757e70a2…54bd6 / bc46facd…699969`, candidato `5fb2a09f… / 998e4fad…`; JSON crudo `diff-full.json` SHA-256 `66f763ed…0046`, 8 MB, en `.scratch/`).

Resultado: **804 invocaciones difieren, en 70 fuentes; 0 diferencias** en `corpus/`, fixtures y las 152 de RD, ni en ningún programa de protección. Las 70: 17 del corpus generado, 47 del twin (casos y permutaciones), 6 repros. Una por una (fuente, salida antigua/nueva, cláusula) en `docs/validation/i07c-s5/s5a1-diferencias-adjudicadas.json` (`s5a1-adjudicate.py`): **56 clase A** (el candidato retira `NX-E0103` de un `[]` de `var` que una asignación posterior fija; §4.1 y §4.1.2.2), **6 clase B** (mismo conjunto de E0103, cambian E1303/E0102 que dependían del retipado por una asignación rechazada; control: al quitar las asignaciones tardías base y candidato coinciden; §4.1.2.2 y §4.1.2.7), **8 manuales** (adjudicadas a mano con reducción: `repro:A_no_assignment`, F16 y sus 2 permutaciones, `generated:1256/1322/1760/2447`; derivación en el JSON), **0 sin explicar**. Todas las diferencias son uniformes en los 3 perfiles y en debug/release (los nueve casos con 8 invocaciones difieren solo en los dos perfiles que aceptan `Some`/`None`/`Ok`; no hay divergencia debug/release ni `check`/`run`). Límite honesto: la clase A se decide por regla estructural y con muestra manual, no por derivación independiente de las 56; y solo 17 de 3.200 programas generados cambian (la mayoría muere antes en nombres), de modo que la potencia del generador es modesta frente a la campaña twin.

Efecto sobre procedencia (relaciones `expected_type`) frente a `main`: 0 en las 3.674 fuentes del corpus; 2 de 8.000 programas aleatorios (semilla 21) cambian solo una relación; y 2 de ~72.000 exhiben un retroceso (hallazgo).

## 8. Mutación inversa de la región tocada (copia desechable `.scratch/mut-tree`, restauración por SHA-256)

Script `docs/validation/i07c-s5/s5a1-mutate.py`; blob inicial `07959d3f…8778a`, cada mutado con su SHA-256 en el JSON de resultados; veredicto MUERTO = fallo en las pruebas del corte (`--profile default`, filtro `i07c_s5_seed_consistency | i07c_pending_nominal_domain`). Segunda pasada con las pruebas finales (ver `s5a1-mutaciones.json`):

| Mutante | Cambio | SHA-256 mutado | Veredicto | Test que lo mata (primero de N) |
|---|---|---|---|---|
| `M1-retraction-off` | `retract_unconsumed_seeds` devuelve `false` sin retraer (cambio 1) | `5a0306244ebb399d…` | DEAD | `outer_index_context_makes_later_flat_array_an_ineligible_twin` (13) |
| `M1b-everything-witnessed` | toda capa se considera atestiguada (cambio 1) | `567e5dafb35859c3…` | DEAD | `outer_index_context_makes_later_flat_array_an_ineligible_twin` (13) |
| `M1c-nothing-witnessed-by-component` | se ignora la componente conexa como testigo (cambio 1) | `cd21c7111fe335c3…` | DEAD | `twin_corpus_has_no_spurious_unresolved_constructor` (1) |
| `M2a-read-plan-ignores-verdict` | `read_plan` ignora `rejected_assignments` (cambio 2) | `6395b3b194eab586…` | DEAD | `incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin` (4) |
| `M2b-read-plan-ignores-retraction` | `read_plan` ignora `seed_cap` (cambio 2) | `b9adb6bc80161c1b…` | DEAD | `retracted_seed_site_keeps_its_element_fixed_by_a_later_compatible_leaf` (1) |
| `M2c-verdict-only-on-error` | el veredicto solo usa `failed` (cambio 2) | `fd24596eb46ac456…` | DEAD | `incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin` (4) |
| `M2d-verdict-never-on-error` | el veredicto solo usa el crecimiento de uniones pendientes, sin `failed` (cambio 2) | `91aad30ad565ed12…` | **no muere con las pruebas del corte** (exhibido por programa, ver hallazgo; el mutante es mejor que el candidato en 2 programas y idéntico en 3.674 fuentes) | `—` (0) |
| `M3-concrete-plan-after-retraction` | se inyecta `concrete_plan` aunque el plan perdió capas (cambio 3) | `3ffb57e53be20135…` | DEAD | `deferred_union_with_assignment_inside_array_and_condition_o03` (3) |
| `M4-cap-ignored-when-seeding` | la siembra ignora el tope `seed_cap` (cambio 1) | `0c9f2f0d83f36561…` | DEAD | `outer_index_context_makes_later_flat_array_an_ineligible_twin` (13) |

Blob inicial `07959d3ff9ec01bb18bf04e8ff353cc062bd1b51323ffc7d314e859728e8778a`; tras cada mutante el archivo se restauró y se verificó el SHA-256 (el script aborta si no coincide). Los tres cambios de V6 tienen al menos un mutante muerto por una regresión nueva. M2b **sobrevivía en la primera pasada**: la búsqueda exploratoria (6.000 programas, no versionada) halló 3 exhibidores y de uno (150 B) salió `retracted_seed_site_keeps_its_element_fixed_by_a_later_compatible_leaf`, con esperado derivado del ADR; con él M2b muere. M2d no es una laguna de pruebas sino el hallazgo: no se declara SUPERVIVIENTE (eso exigiría la suite completa en debug y release con el mutante) porque el mutante es el que acierta en P1/P2; no se anota en `Pending.md` como superviviente sin programa (tiene programa). Las poblaciones `i07c_mut_freeze.py` no se usaron: son mutaciones dirigidas a mano de las tres reglas (plan §10: «inversión dirigida por regla tocada»).

## 9. Auditor de spans

`note_assignment_verdict` usa `span.start` del `Expr::Name` solo como clave de identidad de la variable (resolución `uses`), no como primario de diagnóstico: no es un sitio `.span()` crudo y el auditor no lo marca; no se añadió sitio nuevo. Los 14 sitios de `types.rs` se re-localizaron por expresión y contexto (no se relajó `expression`/`context` ni el `else: raise`; el sitio `prepare_for_type` se movió a mano porque su contexto está a más de 8 líneas). `python -B scripts/i07c_span_audit.py`: 14/14 + 5/5 + 1/1 = 20/20; `--self-test` PASS (13 reversiones rechazadas); la aserción de líneas de `tests/i07c_structural_invariants.rs` se actualizó.

## 10. Pruebas completas y puertas que ejecuta C

- `cargo fmt --all -- --check` OK; `cargo clippy --all-targets --offline -- -D warnings` OK.
- `cargo nextest run --profile default --offline --no-fail-fast` (debug, **entero**, tras el último cambio): **2.185/2.185 pasan**, 72 omitidos (ignorados por el perfil, ver `check-nextest-selection.py`). Release: **2.187/2.187**, 73 omitidos. `check-nextest-selection.py`: coherente (release completo=2187). Aviso de nextest `selection`: mi archivo lista 17 tests y el chequeo cuenta 10 atributos (cfg legítimo; solo cero es fatal).
- No ejecuté `validate.sh` ni `gate.sh` completos (los lanza el coordinador).

## 11. Campaña twin (`scripts/i07c_typed_campaign/main.rs` SHA-256 `974d51d0…f690`, `LIST_OTHER=1`)

Comandos: `i07c-typed-campaign twin 100000 10495373245766874321 4 3` (debug, exe `49fc2501…c9c9`) y `twin 300000 10495373245766874321 4 3` (release, exe `af230042…22bc`), compilados del árbol; la **base** con el mismo arnés compilado de `git archive HEAD` (release 300000, exe en `target-s5a1-twinbase`). Salidas en `docs/validation/i07c-s5/s5a1-twin-{debug-100000,release-300000,base-release-300000}.out` (SHA-256 `af335374…77c9`, `fb610b72…1a93`, `103ff863…f80b`).

| | `flagged` | `other_differences` | E0103 espurios en `flagged` |
|---|---|---|---|
| Base release 300000 | 17 | **8** (`29097, 39425, 98859, 136136, 166953, 191390, 241894, 283013`) | 16 |
| **Candidato release 300000** | **4** | **1** (`237611`) | 2 |
| **Candidato debug 100000** | **1** | **0** | 1 (el mismo de `18584`) |

El plan exige `other_differences=0` y ningún E0103 espurio salvo F08, F10/F11: **no se cumple del todo** y lo digo sin esconderlo:
- `idx 237611` (`other`, **nuevo frente a la base**): `…var r=[];let a0=[(r[0]),(if (x < 0) then r else [[2]]),[(0 - x),(0 - 0),Len(ar)]];let a1=1;r=[ar];}`. El candidato da **un** `NX-E0102 [93,121)` (la unión entera, `Array<I64>` / `Array<Array<I64>>`, relación `[86,90)`), el gemelo concreto reparte dos errores por hojas y la base da cinco. Derivación: el elemento de `a0` es `α` pendiente al entrar en la unión (sin contexto determinado; §4.7), la unión sintetiza `Array<Array<I64>>` (el `[[2]]` fija `α=Array<I64>`) y se denuncia como un todo frente a `α`; `r=[ar]` concuerda; el gemelo no es oráculo (§4.1.2.7: el adelanto vuelve determinado un contexto indeterminado al entrar). La base es la que retipaba por `read_plan` (§4.1.2.7, «no existe una regla de sustituir retrospectivamente»). Lo dejo a D.
- Flagged residuales con E0103: `idx 183938` es F10 (en la base también); `idx 18584` (101 B mínimo: `fn Main(){let p=true;var r=[];var q=[];let a=(if p then r[0] else (if p then q else r));r=[3];q=[3];}` → 2 `NX-E0103`) es la misma familia FAM-E (S5a.5), idéntica en la base. `idx 145869` = patrón F08 genuino (idéntico en la base). `idx 119045` es nuevo frente a la lista de flagged de la base: `mis[A]E0102` (el gemelo emite más errores; el candidato da un único E0102 con relación `[85,88)` correcta y la base daba además un E0102 retroactivo en el `3`); mismo análisis de no-oráculo.

## 12. Terminación y estrés (prueba exigida en §3 del plan)

No añadí la aserción in-código «una pasada de planning adicional no cambia `shape_plan`, `seed_cap` ni `rejected_assignments»: el bucle ya sale solo cuando `!changed && !retracted`, es decir, cuando la última pasada no cambió `shape_plan` ni `seed_cap`, y `rejected_assignments` se reconstruye por pasada. Medido con una copia desechable instrumentada (no entregada) sobre las 3.674 fuentes y 3 perfiles: **máximo 3 pasadas de planning** (cota 129), 27 fuentes con retracción; la distribución es `{0:1028, 1:382, 2:2215, 3:49}`. Las permutaciones de 2 y 3 asignaciones de cada caso twin están en la suite. Costo: `retract_unconsumed_seeds` es O(N) por pasada de planning; la suite completa no cambió de duración de forma apreciable (primera ejecución completa 217 s, ejecución final 201 s, misma máquina), `i07c_mutant_projection_scale` pasa.

## 13. Límites y hallazgos (sin esconder nada)

1. **Hallazgo abierto** de procedencia (`I07c-C-S5a1-hallazgo.md`): decide A/B si se quita `failed ||` o se registra como límite con fecha.
2. Independencia de orden: solo empírica (B q→r/r→q, permutaciones de la suite y 300.000 + 100.000 programas twin); `conflicts_prefix` sigue en la conjunción. No hay demostración general.
3. Residuo FAM-E: F10 y la reproducción mínima de 101 B (arriba); F11 ya no lo tiene en esta base (el plan §9 supone 2; la cota se cumple por defecto). F10/F11 siguen en el plan de S5a.5.
4. El segundo mecanismo de FAM-2 no se tocó; las dos regresiones de protección del plan: la de `if p then r else (if p then 1 else 2)` entró (una E1303); la de `(r==[s])` no, por falta de esperado derivado en el plan (sin cambio frente a `main` en el diferencial: `protect_equality` y `protect_len` forman parte del corpus y no difieren).
5. Programas de F01–F19 y de O02/O04/O05/O06 no tienen C esperado fijado (plan §7); D debe derivarlos.
6. El diferencial tiene baja potencia del generador (17/3.200) y adjudicación estructural de la clase A; la campaña twin es la evidencia de mayor volumen y tiene el `other` nuevo de arriba.
7. Mutación: ver §8.
8. Nada se confirmó ni se hizo `git` salvo lecturas. Procesos: ninguno por nombre de imagen; el cerrojo `heavy.lock` se usó para toda compilación fría y suite; tras mi trabajo no quedan procesos míos.
