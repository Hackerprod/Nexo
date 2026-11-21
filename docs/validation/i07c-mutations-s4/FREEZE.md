# Población de mutación de I07c-S4 (congelada antes de ejecutar)

Mutación de las regiones de producción tocadas por S4 (hechos de visita en el grafo único, pila de generaciones, corrección de `check_against_context_one`), con población nueva y no reutilizada de campañas anteriores, como exige el plan (§9).

- Base de la población: `be8c47c` (main con B1b, sin S4).
- Código evaluado: `6394b67` (S4.2); los commits posteriores de la rama solo tocan documentación y andamiaje.
- Enumerador: `scripts/i07c_mutaciones.py --base be8c47c` con `I07C_MUT_DIR=docs/validation/i07c-mutations-s4` (mismos operadores fijos que las campañas anteriores).
- Población: 138 sustituciones, en `population.json`.
- SHA-256 de `population.json`: `c8d9f0d55058b1c4eb898c111807fbc12799ee2c0aeb900a3184e292e32bbb6c`.
- SHA-256 del blob `src/check/types.rs` en `6394b67`: `d5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6`.
- SHA-256 de `git diff be8c47c 6394b67 -- src/check src/ir`: `132f84af9e7ea1a031609a7f82068344048f8e78d3834e5dd2796234e12f16a9`.
- Exclusiones: ninguna. Entra toda condición, límite, guarda y borde tocado.
- Criterio: un superviviente solo bloquea con un programa discriminante o una regresión que lo distinga (AGENTS, «Rigor proporcional»); el resto se registra en `Pending.md`. Las filas `inconclusa` por contención o por plazo se repiten hasta clasificarse y no cuentan como detectadas.
- Veredicto de mutante: parada en el primer fallo con `scripts/mutant-verdict.py` / nextest `--profile default`; superviviente solo con la suite completa en debug y release.
