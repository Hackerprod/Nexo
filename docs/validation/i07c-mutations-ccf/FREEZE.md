# Población acotada de mutación de I07c (congelada antes de ejecutar)

Decisión del coordinador del carril (revertible): la campaña de mutación normativa del corte se acota al código de producción
introducido o modificado por S1, S2, Bloque I y S3 respecto de `ccf6df1`; el código previo ya pasó campañas anteriores.
La campaña completa contra `b506d7d` sigue corriendo aparte («campaña extendida I07C-MUT-EXT»).

- Base de la población: `ccf6df1`.
- Código evaluado: `11a70ec` (S3); los commits posteriores de la rama solo tocan documentación y `scripts/i07c_mutaciones.py` (andamiaje).
- Enumerador: `scripts/i07c_mutaciones.py --base ccf6df1` con `I07C_MUT_DIR=docs/validation/i07c-mutations-ccf` (mismos operadores fijos que la campaña completa).
- Población: 739 sustituciones (`src/check/types.rs` 725, `src/ir/lower_match.rs` 14), en `population.json`.
- SHA-256 de `population.json`: `a6f21ab42c6bc0436f7b12f9053519824e819ae762f67f12f3bb8cb741a103f4`.
- SHA-256 de `git diff ccf6df1..HEAD -- src/check src/ir`: `0d7a26ccd607a73272ea7082c88951fb6d10950f80477df5d90d2a343894acfa`.
- Exclusiones: ninguna. Entra toda condición, límite, guarda y borde tocado, aunque parezca movimiento mecánico de S3.
- Criterio de superviviente: solo bloquea con programa discriminante o regresión; el resto se registra en `Pending.md`. Las filas `inconclusa` por contención se repiten hasta clasificarse y no cuentan como detectadas.
- Plazos: factor `I07C_MUT_TIMEOUT_SCALE=3` (andamiaje; no cambia población ni clasificación). Los primeros 100 mutantes corren en release y debug y el resto en release, como en la campaña completa.
