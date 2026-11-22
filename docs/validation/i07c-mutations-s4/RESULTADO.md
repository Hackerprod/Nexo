# Resultado de la mutación de I07c-S4

Población congelada antes de ejecutar (`FREEZE.md`): 138 sustituciones de producción introducidas o modificadas por S4 respecto de `be8c47c`, todas en `src/check/types.rs`; SHA-256 de `population.json` `c8d9f0d55058b1c4eb898c111807fbc12799ee2c0aeb900a3184e292e32bbb6c`. Código evaluado: `6394b67` (S4.2); los commits posteriores solo tocan documentación, andamiaje y el merge de `main`.

## Resumen por clase (las 138 sustituciones, una sola clase cada una)

| Clase | Mutantes | Significado |
|---|---:|---|
| `detectada` | 84 | la campaña las detectó (prueba dirigida o suite completa) en los dos perfiles; incluye M00003, que dio `inconclusa` en release por plazo (100 s) con la máquina cargada y se repitió aislada con `scripts/mutant-verdict.py` (suite `default` íntegra): MUERTO en debug por `check::tests::adr_gap_primitive_domain_primary_is_first_offending_argument` (`caza/m00003-veredicto.json`) |
| `no_compila` | 7 | el mutante no compila: sustitución inválida |
| `sin_programa_tras_caza` | 47 | sobrevivieron a las pruebas dirigidas y a la suite completa en debug y release; ninguna caza con programas encontró diferencia; NO se afirma equivalencia |
| **Total** | **138** | |

Criterio de superviviente (AGENTS, «Rigor proporcional»): un superviviente solo bloquea si existe un programa o una regresión que lo distinga; los 47 `sin_programa_tras_caza` se registran en `Pending.md` (fila `I07C-S4-MUT-SIN-PROGRAMA`). 22 de ellos llevan además un argumento de invariante por mutante sobre el código (3 del primer lote y 19 del segundo): el último argumento de `merge_child` solo añade `padre.tainted |= hijo.tainted` cuando el bit ya se copió justo antes, o la rama retorna `Ok(Type::Tainted)` y el bit se marca después en `check_against_context_one`; los otros 25 no tienen argumento demostrado a partir del código.

## Método

- Campaña: `scripts/i07c_mutaciones.py --base be8c47c` con `I07C_MUT_DIR=docs/validation/i07c-mutations-s4`, 3 fragmentos, plazos ×3 y compilación sin información de depuración con `I07C_MUT_ENV_*` (andamiaje); los 100 primeros mutantes en release y debug y el resto en release; nivel dirigido y, para supervivientes, suite completa. `results.jsonl` une las 138 filas en el orden de la población.
- Caza de programas sobre los 47 supervivientes (11 + 36, en dos lotes): corpus congelado antes de ejecutar de 39.127 fuentes únicas (SHA-256 `d9a8c389bef1d6b1d860fdb52d79ba5b5a58b7ad980e3abf886ae33f4eed4684`, semilla `64193`, generador SHA-256 `7d00ac31cd38b2cfd4aec4c672c5b0d54799779d8f1996943840b10d6aa2a539`): corpus histórico de la caza de I07c (32.631), 3.040 fuentes dirigidas nuevas, la familia de S4 (3.072 + 384), las 152 filas RD, `corpus/` y `tests/fixtures/`. El perfil `f0-0005-b-v1` recorre el corpus entero y `f0-0003-b-v1` y `f0-0002-b-v2` las 3.040 dirigidas; se compara `exit` y el JSON completo de `check --format json`, y `run --format json` cuando alguno de los `check` acepta, contra el binario correcto de `6394b67` (`gate.sh --cached-bin release`).
- Resultado de la caza: 47 `SIN_DIFERENCIA` con 45.207 comparaciones fuente/perfil por mutante (2.124.729 en total: 497.277 en el primer lote y 1.627.452 en el segundo) y 0 programas discriminantes (`caza/programas-discriminantes*.json` vacíos). Cada mutante se aplicó al único sitio literal del blob base y se restauró por SHA-256 (`d5046aed…2ca6`).
- Incidencias del andamiaje (no afectan a la población ni a los veredictos): el primer lanzamiento de los fragmentos falló al arrancar por falta del directorio `target` y se relanzó; el segundo lote de la caza lo cortó dos veces el filtro de ciberseguridad de OpenAI y después Codex alcanzó su límite de uso, así que los 9 últimos mutantes los terminó el coordinador del carril con los mismos scripts (`caza/andamiaje/hunt_s4b_worker.py`, reanudable).

## Ficheros
- `population.json` y `FREEZE.md`: población y congelación.
- `results.jsonl` SHA-256 `ec0fba889c1e978fb786250e15d5c72de1776e12d86aff3448595f8aaf66e287`.
- `caza/`: `lote1-*` y `lote2-RESULTADO.md` (resultados por mutante con argumentos de invariante), `hunt-resultados-w0/w1.jsonl`, `programas-discriminantes*.json` (vacíos), `supervivientes-lote1/2.json`, `m00003-veredicto.json` y `andamiaje/` (scripts de la caza y congelación del corpus; son andamiaje, no código de producción).
