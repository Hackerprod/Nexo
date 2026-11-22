# I07c-S5b.1 — implementación y comprobación de C

**Base:** `9bc45f74affa454481e8cbde1458817ef26b69a7` (`HEAD` local y remoto de la rama al empezar); SHA-256 del blob Git de `src/check/types.rs`: `1050c84c83a0f99898731b1a9a829091471e30158303199677ca8f740c7caf34`. Árbol sin commit de C. Durante la sesión el coordinador avanzó `HEAD` a `4acee20` por un merge documental; el blob base de `types.rs` continúa con el mismo SHA-256 y el diferencial usa la copia archivada de `9bc45f7`. Plan: `I07c-S5-plan.md` §5 y §8 fila 6; PASS de B en `I07c-plan-B-S5-r4.md`. Requisitos de núcleo: visita y diagnósticos de selector; andamiaje: JSON, mediciones y arnés.

## Corrección y oráculo cambiado

Con base `Err` no `AnalysisLimit`, `Frame::Index` ahora agenda una única visita independiente del selector (sin contexto `I64`, herencia de taint vacía) por `index_failed_base`, ayudante `#[inline(never)]`. `IndexSelectorProgress` transporta el resultado de la base; `Expr::Index` registra el error propio del selector y devuelve el de la base. `AnalysisLimit` del selector sigue teniendo prioridad. `finish_index_selector`, también `#[inline(never)]`, mantiene el `Box` sin desreferenciar en `resume_non_match`; evita crecimiento de ese marco. No se cambió el comportamiento de base `Ok(Tainted)` ni de proyección pendiente.

**Diff explícito de la fila `index_selector`** de `i07c_s4_context_regressions::contextual_owner_and_adjacent_fact_boundaries_match_b1b_cli`: entre el tercer diagnóstico antiguo `[151,157)` y el cuarto `[166,180)` se intercala **solamente** `NX-E0102/type_mismatch`, `types/types.check`, primario `[162,163)` (`0`), `data={"expected_type":"Array<Array<I64>>","found_type":"I64"}`, relaciones `expected_type [160,161)` (`r`) y `found_type [162,163)` (`0`). Los otros seis diagnósticos, incluidos `NX-E0103 [148,150)` del `[]` literal de la base, conservan contenido y orden; las otras cinco entradas de la prueba no se alteran. El comentario del test cita F0-0002 §§4.1.1, 4.1.2.3 y 4.1.2.4/N(d): la base fallida no borra el error interior del selector y el primer elemento compatible `r` fija el tipo contra el cual se compara `0`. La aserción sigue comparando el JSON completo de `check` y `run`.

`tests/i07c_s5b_selector.rs` (registrado en `tests/suite.rs`) fija las cuatro fuentes negativas de §5, sus positivos pareados, el contraste `Ok(Tainted)`, la proyección pendiente y los controles. Perfil `f0-0002-b-v2`; comprueba `check` y `run`, rechazo, orden, regla/subcausa, tipo, relaciones y bytes. En la base archivada los cuatro negativos producen solo el primer primario: `[30,34)` (ambos verbos, debug); tras la corrección producen también `[39,43)`, `[39,43)`, `[38,42)` y `[42,46)`, respectivamente. Con el binario release se repitió la misma comprobación antes/después.

## Medición de pila (rustc 1.99.1 MSVC, perfil test DI, `/MAP`)

Comando: `RUSTFLAGS='-C link-arg=/MAP:C:/projects/Nexo-i07c/.scratch/tmp-s5b1/{base,candidate2}.map' cargo test --lib --no-run --locked --offline`; después `python -B scripts/f5-frame-sizes.py <exe> <map> --dumpbin 'C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64/dumpbin.exe'`. El self-test del extractor pasó. Base compilada desde `git archive HEAD`; candidato del árbol de trabajo.

| Marco / ruta | Base | Candidato final | Cambio |
|---|---:|---:|---:|
| `drive` | 27.536 B | 27.536 B | 0 |
| `resume_non_match` | 33.384 B | 33.080 B | −304 B |
| Pareja | 60.920 B | 60.616 B | −304 B |
| Margen frente a 66.560 B | +5.640 B | +5.944 B | +304 B |
| `check_against_context_one` / `expr_inner` | 6.784 / 12.752 B | 6.784 / 12.752 B | 0 |
| Ruta `drive→resume_non_match→check_against_context_one→expr_inner` | 80.456 B | 80.152 B | −304 B |

Base: ejecutable/mapa SHA-256 `feb8cd3f…eab4ea` / `cd634d93…221e29d`; candidato: `90ede2bd…05ae13` / `0a3212f2…a6fb4`. La primera extracción, antes de mover `IndexSelector` al ayudante, dio pareja 61.096 B (+176 B): se corrigió antes de entregar. Comando `python -B scripts/f5-stack65.py`: control 16 KiB desborda según lo esperado; `option_match126` y `result_match126` TYPES pasan **5/5 a 65 KiB**. `python -B docs/validation/i07c-s5/s5b1-stack-probes.py`: ambos pasan **5/5 a 129, 204 y 256 KiB**; ejecutable sin `/MAP` SHA-256 `188fee3d234030a4b5d5442bc196f1a2f48ab9799f12ec59640830f71e84cf98`.

## RD y diferencial completo

`python -B scripts/i07c_rd_conformidad.py --verify C:/projects/Nexo-i07c/.scratch/target-s5b1/debug/nexo.exe`: filtro PASS, cero diferencias de snapshot, **152/152 pares check/run**.

Generador reproducible `docs/validation/i07c-s5/s5b1-corpus.py`; congelado antes de ejecutar: semilla **20261121**, **3.000 fuentes distintas**, SHA-256 **`294bd7b32f67d1b5186d894651673a01b9f3cc55e751e0e743720dd799e3ae05`** (codificación y conteo en `s5b1-corpus.json`). Incluye bases `Err`, `Ok(Tainted)` y pendiente, anidamiento, operadores, agrupaciones y constructores heterogéneos. Adicionales: **217** fuentes de `corpus/**/*.n` y `tests/fixtures/**/*.n`; **152** expansiones RD extraídas de la tabla del ADR. Total **3.369 fuentes / 80.856 ejecuciones** (dos binarios × debug/release × `check`/`run` × tres perfiles `f0-0002-b-v2`, `f0-0003-b-v1`, `f0-0005-b-v1`). Binarios de base archivada SHA-256 debug `9b94d3331453f1daa6df50928fa51b089a4e4b1b822c2660056842a48ce5576d`, release `539cfa6d8b63b297fc780d3c91dc9459ff0444af49733a22941b84d60df89f0a`; candidatos finales debug `fa6e4f914790f20b6e0849e301d361e6645d2c50caf34d5b107d37b32cd891c4`, release `0c259e72bd39c8532fd9e02710eb0fc51690ee1402430d5aa092b09068c70dc9`.

| Familia de fuentes | Fuentes | Invocaciones base+candidato | Diferencias | Adjudicación |
|---|---:|---:|---:|---|
| Generadas | 3.000 | 72.000 | 14.400 (1.200 fuentes × 12) | Un único `NX-E0102/type_mismatch` añadido *dentro* del selector de base fallida; exit, stdout/stderr, demás JSON y diagnósticos previos idénticos |
| Corpus, fixtures y RD | 369 | 8.856 | **0** | Incluidas las 152 RD |

**Enumeración individual:** `docs/validation/i07c-s5/s5b1-diferencias.json` (SHA-256 `eb66e7120d2d95934ef351b4d271abcfa9a7a562e7eee4d17e500e93fbe839a1`) contiene las 1.200 fuentes, cada diagnóstico anterior y añadido (código, subcausa, span, `data`, relaciones) y las 12 invocaciones correspondientes. `s5b1-adjudicate.py` verificó en cada una que al quitar *solo* el diagnóstico añadido se obtiene el JSON completo antiguo; que el primario añadido cae en el selector, que el error de base es anterior, que la regla es la del plan y que las doce invocaciones de cada fuente coinciden. Los siete resultados por rango, con salidas completas, quedaron en `.scratch/tmp-s5b1/final-*.json` (artefactos regenerables mediante `s5b1-differential.py`).

Durante el primer intento del rango 2000–2500 hubo un timeout de 30 s de **la base release**, después de 200 fuentes; el arnés no guardaba aún el ID al producirse la excepción. Se añadió el registro explícito de fuente y se repitió íntegro ese rango: terminó con 500 fuentes, 12.000 invocaciones y las diferencias adjudicadas. El evento inicial no se cuenta como PASS ni como fallo del candidato; el programa exacto de aquel timeout no quedó recuperable. No se ocultó ni se reintentó silenciosamente.

## Mutación inversa y pruebas dirigidas

En copia desechable de `git archive HEAD`, con los tests nuevos, se invirtió exclusivamente la guarda `Err(error) if error.operation != Operation::AnalysisLimit` por `Err(error) if false && error.operation != Operation::AnalysisLimit`. SHA-256 del archivo previo **`d23936902e68ec191da5a22f932581ec3a85f0f0f86192d83553e329a0751e25`**, mutado **`6123acb66a81c229ce8b36e4dfcf07e3264ac5bdf7e5a36c553edb0fbdc7c6b6`**; `cargo nextest run --profile default --offline -E 'test(i07c_s5b_selector)'`: fallan las **cuatro** regresiones de base `Err`, pasan los dos controles. `s5b1-mutate.py --restore` restauró y verificó exactamente el SHA inicial. La fuente del árbol de trabajo nunca se mutó.

`cargo fmt --all -- --check`: PASS. `cargo clippy --all-targets --offline -- -D warnings`: PASS. Selección `cargo nextest run --profile default --offline -E 'test(i07c_s5b_selector) or test(i07c_s4_context_regressions) or test(i07c_structural_invariants) or test(i07c_rd_conformidad)'`: **15/16**; único fallo `i07c_structural_invariants::grouped_operands_keep_one_causal_primary`: auditor de líneas crudas detecta `types.rs:11304` (desplazamiento por los ayudantes). El contenido del sitio (`self.active_if_span.replace(inner.span().clone())`) no cambió. El perímetro prohíbe editar `scripts/i07c_span_audit.py` o ese test: el coordinador debe reconciliar la lista de líneas; hasta entonces **la selección exigida no está verde**. No se ejecutó `validate.sh` ni `gate.sh` completos.

**Límites:** no se midió TD/DG con sus scripts por separado; el diferencial incluye el corpus y fixtures del proyecto, pero los scripts TD/DG quedan a cargo de la puerta del coordinador. El timeout inicial de base no pudo atribuirse a un ID concreto. El auditor de líneas impide entregar un árbol con todas las pruebas dirigidas verdes sin una reconciliación autorizada.
