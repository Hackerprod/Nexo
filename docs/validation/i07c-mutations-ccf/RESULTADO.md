# Resultado de la mutación acotada de I07c

Población congelada antes de ejecutar (`FREEZE.md`): 739 sustituciones de producción introducidas o modificadas por S1, S2, Bloque I y S3 respecto de `ccf6df1`; SHA-256 de `population.json` `a6f21ab42c6bc0436f7b12f9053519824e819ae762f67f12f3bb8cb741a103f4`. Código evaluado: `11a70ec` (S3). Los literales de M00616 y M00617 están guardados con el carácter `§` mal codificado (`Â§`); la campaña los aplicó bien porque cada fragmento re-enumera la población en memoria, y la caza posterior tolera ese caso.

## Resumen por clase (los 739 mutantes, una sola clase cada uno)

| Clase | Mutantes | Significado |
|---|---:|---|
| `detectada` | 387 | la campaña los detectó (prueba dirigida o suite completa) en todos los perfiles ejecutados |
| `detectada_por_regresion_nueva` | 135 | sobrevivían a la campaña; una regresión añadida en los lotes 1 a 6 (solo pruebas) los mata y pasa con el código correcto |
| `detectada_por_no_terminacion` | 10 | con el mutante un test no termina; con el código correcto termina en segundos (tabla más abajo) |
| `no_compila` | 97 | el mutante no compila: sustitución inválida |
| `equivalente_con_argumento` | 15 | argumento de invariante por mutante en los informes de los lotes; sin diferencia en el corpus de la caza |
| `sin_programa_tras_caza` | 95 | ningún programa encontrado: ni los intentos dirigidos de los lotes ni 32.631 programas de la caza distinguen el mutante; NO se afirma equivalencia |
| **Total** | **739** | |

Criterio de superviviente (AGENTS, «Rigor proporcional»): un superviviente solo bloquea si existe un programa o regresión que lo distinga; los 95 `sin_programa_tras_caza` y los 15 equivalentes se registran en `Pending.md`. Los supervivientes cuyo programa se encontró (incluido M00436, hallado por D') tienen su regresión.

## Método
- Campaña: `scripts/i07c_mutaciones.py --base ccf6df1` en 6 fragmentos (`I07C_MUT_DIR=docs/validation/i07c-mutations-ccf`, plazos ×3 y compilación sin información de depuración con `I07C_MUT_ENV_*`, andamiaje); los 100 primeros en release y debug, el resto en release; nivel dirigido y, para supervivientes, suite completa. `results.jsonl` une las 739 filas en el orden de la población.
- Inconclusas por contención o por el cuelgue de salida de `nexo.exe` (defecto corregido en `main` 9be9f76): se repitieron sobre un árbol con la misma población (SHA idéntico) y el arreglo de salida aplicado.
- Caza con generador: 112 mutantes (los «sin programa» de los lotes 2 a 5); corpus de 32.631 programas únicos (SHA-256 `2e8029194601ee8c0ac15b275af246daa831319f87dcaf1474aa536193d98bc8`): 40 semillas (101 a 140) × 900 del generador de D (`andamiaje/fuzz-generador-D.py`), los fuentes de `corpus/` y `tests/fixtures/`, las 152 filas RD y el programa de D'. Se compara `check` (y `run` para `lower_match.rs`) del mutante contra el binario correcto, con parada tras 5 diferencias.
- Resultado de la caza: 10 discriminados y 102 sin diferencia. Los 10 discriminados (M00436, M00466, M00474, M00475, M00494, M00588, M00655, M00701, M00702, M00703) están en `caza/lote6-programas.json` con su programa y la salida de referencia, y su regresión está en `tests/i07c_solver_depth.rs` (lote 6).

## Detección por no terminación
Con el código correcto: `check::tests` release 41,9 s y debug 6,0 s; `i07c_tests` debug 5,9 s y release 2,1 s; biblioteca completa debug 132,1 s; los cinco tests de integración implicados en M00676 y M00732 pasan en 0,013 a 0,050 s. Con cada mutante el mismo filtro no termina en 300 s (biblioteca: 900 s; integración: 600 s por test).

| Mutante | Evidencia |
|---|---|
| M00083 | release check::tests: NO_TERMINA_EN_300s |
| M00120 | debug check::tests: NO_TERMINA_EN_300s; release check::tests: NO_TERMINA_EN_300s |
| M00121 | debug lib completa: NO_TERMINA_EN_900s |
| M00124 | release i07c_tests: NO_TERMINA_EN_300s |
| M00176 | debug check::tests: NO_TERMINA_EN_300s; release check::tests: NO_TERMINA_EN_300s |
| M00177 | debug check::tests: NO_TERMINA_EN_300s; release check::tests: NO_TERMINA_EN_300s |
| M00477 | debug check::tests: NO_TERMINA_EN_300s; release check::tests: NO_TERMINA_EN_300s |
| M00479 | debug check::tests: NO_TERMINA_EN_300s; release check::tests: NO_TERMINA_EN_300s |
| M00676 | debug nextest suite completa: TIMEOUT por test 600 s: nexo::i07c_pattern_deferred unreachable_match_arms_still_check_resource_ownership |
| M00732 | debug nextest suite completa: TIMEOUT por test 600 s: nexo::i07c_deferred_constraints pattern_shape_conflicts_belong_to_later_equality |

M00106 (debug) no es de esta clase: su filtro `i07c_tests` falla con el mutante tras 231,7 s y queda `detectada`. M00387, M00432 y M00538 dieron suite completa en verde con `mutant-verdict.py`; M00432 lo mata además un test del lote 3 y M00387 y M00538 se clasifican equivalentes con argumento (lote 5).

## Ficheros
- `results.jsonl` SHA-256 `194ab9243e1a1f81c2c7884f64c4811bc46163ed8e3097d9d156e9ce384f72d6`
- `clasificacion.json` SHA-256 `481d804b287eff89d926ce0f56a0dba1bea3a56462818498755d7f00f24cb1b1`
- `no-terminacion/verify-hang.jsonl` SHA-256 `a3501717de0678612aae94b70b60a2d396308441450e652c393d4c072e2059e6`
- `no-terminacion/verify-lib.jsonl` SHA-256 `ac7f25fdc7c9a32453ee8279e9edea150b8a062675f422e4a08b62e30342b477`
- `no-terminacion/verify-nextest.jsonl` SHA-256 `d3718a3690fcc9db8be69dc3501a44afc9a294839fe405827471c77186406596`
- `no-terminacion/mutant-verdict-resumen.jsonl` SHA-256 `a05bb580556f7022a44f015a2d7213ad987056c9e381516d96fd81d2c1344098`
- `caza/hunt-resultados.jsonl` SHA-256 `7e425d088562ffc0ccd9296076095489a959242dfaa7621f968a95869c477267`
- `caza/lote6-programas.json` SHA-256 `9fb1e39a9b11183fa0da542c4255e6befdf3e926387f8df37adb189212ed6cf3`
- `andamiaje/`: scripts usados (caza, verificaciones, construcción de esta evidencia); son andamiaje, no código de producción.
