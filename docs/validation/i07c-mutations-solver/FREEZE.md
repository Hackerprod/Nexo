# Población de mutación de `94804e2` respecto de `ccf6df1` (congelada antes de ejecutar)

Mutacion del nucleo del solver de inferencia diferida (region estable de types.rs lineas 4234-6762: record_error a solve, incluidos require_*, concrete, bind, unify, defer_shape_conflict y solve) sobre el main con F5-pila-r1. Excluye la planificacion/siembra (S5a), primary_span (S6-COR) y drive/expr_inner, que los cortes S5/S6-COR/B2b reescriben; esas regiones se mutaran tras sus cortes.

- Base de la población: `ccf6df1845a1f18b0c8356ab904be3add5ae0206`.
- Código evaluado: `94804e2eba5aef471b0e442831c8bdab2d3521a3`.
- Enumerador: `scripts/i07c_mutaciones.py --base <base>` ejecutado en un worktree desechable del commit evaluado con `I07C_MUT_DIR=docs/validation/i07c-mutations-solver` (`scripts/i07c_mut_freeze.py`).
- Población: 295 sustituciones (`src/check/types.rs` 295).
- Operadores: `and` 47, `break` 3, `continue` 37, `drop_assignment` 58, `drop_state` 66, `eq` 10, `false` 8, `increment` 1, `le` 4, `lt` 2, `ne` 16, `or` 33, `true` 10.
- SHA-256 de `population.json`: `24b2323f91e7421fda150b1735f6474ec1c78931352d4881d42f86f556c7df15`.
- SHA-256 del blob `src/check/types.rs` en el commit evaluado: `1050c84c83a0f99898731b1a9a829091471e30158303199677ca8f740c7caf34`.
- SHA-256 de `git diff ccf6df1 94804e2 -- src/check src/ir`: `db851b8b7c218a8a043d708639e76ac064e3d77206aee5c3a9d5ce90dfa7cf7e`.
- Criterio: un superviviente solo bloquea con un programa discriminante o una regresión que lo distinga (AGENTS, «Rigor proporcional»); el resto se registra en `Pending.md`. Las filas `inconclusa` por contención o plazo se repiten hasta clasificarse.
- Para ejecutar: `I07C_MUT_DIR=docs/validation/i07c-mutations-solver python scripts/i07c_mutaciones.py --base ccf6df1 --run --resume --full-suite --mutation-profile both --only production --shards N --shard k --target-dir <target>`, en un clon del commit evaluado con la población idéntica (comprobar el SHA-256 anterior).
