# C2 reconstrucción mínima r1 — en validación, sin PASS

Base `af19a34`; rama nueva `i07c-rediseno-c2-min`. El commit `dabf7eb`
lleva el mismo blob `src/check/types.rs` que `e401a9e`
(`8d3873129c9e8f1a319e14fc0f20218840a4bbb7`), sin los pasos posteriores.
El censo D12 es el conjunto fijo de 62 defectos del sondeo variado de 5.000
fuentes; las diez mejoras seleccionadas se miden aparte. La bisección completa
y la tabla semilla→primer commit están en `c2-commit-bisect-v1.json` y
`c2-commit-bisect-v1.md` (SHA-256 del primer blob:
`531fc4f4e1f681428e7d3e3a9afeb0461d06bf034ba7176274b890380f816f32`).

| Código medido | Owned C2: fallos entre 10/0/3 | Discriminadores: fallos de 38 | Defectos D12 activos de 62 | Diferencias históricas de 120 | Primeros defectos finales introducidos |
|---|---:|---:|---:|---:|---:|
| `dabf7eb` = `e401a9e` | 3: RC1, copia congelada, ST1 | 8 | 0 | 0 | 0 |
| `2841cc6` (P1d) | 0 | 36 | 41 | 0 | 38 (M1 22, M2 9, M3 1, M4 1, M5 5) |
| `e2d638f` (P1g) | 0 | 36 | 43 | 0 | 3 (M1 1, M2 2) |
| `b0358fa` (origen local) | 0 | 8 | 41 | 52 | 1 (M5 1) |
| `f5b12b1` (rama posterior) | 1: B_M1 | 8 | 61 | 74 | 20 (M1 1, M2 7, M3 6, M4 4, M5 2) |
| `960df52` (testigo concreto) | 0 | 0 | 61 | 68 | 0 |
| `d47a46f` (P-D11) | 0 | 0 | 62 | 64 | 0; `69d09c5` reintrodujo un defecto antes recuperado |

Owned se midió con `owned_rows_gate.py` y un previo C1 genuino de
`i07c-rediseno-c1-ref` (3/74, 0/45, 0/13, F06=main, 0 fallos).
Discriminadores usa `c2_discriminator_gate.py`; el censo y las diferencias
históricas salen de las 61 compilaciones secuenciales de la bisección. El
binario `dabf7eb` se recompiló en D: y su `c2_origin_iteration.py` dio
A/A-unmodeled/D/H-divergent 0/0/0/0, con cuatro sondas extra fallidas.
Los 64 miembros históricos finales no se reabren; esta tabla mide solo
cuántas filas difieren de main en cada revisión.

## Qué se atribuye a los cambios

`2841cc6` introduce `entry_projection_leaf` y usa su vista provisional en
el primer elemento de un literal seguido por un `if`. En RC1, la fuente
`fn Main(){var s=[];let a=[s[0],(if true then 1 else true)];s=[1];}`
pasa de NX-E0102 en `[52,56)` a la NX-E1303 normativa en `[52,56)`.
En la semilla 158 del sondeo, la fuente comienza con
`let a0=[t[1],(if false then [s] else t)]` y tiene otras dos igualdades
antes de `s=[1];u=[true]`; el mismo commit sustituye las E0103 iniciales
de main y añade NX-E0102 en `[170,171)` y `[183,187)`. D12 la clasifica
como conservación de main sin oráculo completo. El commit contiene tanto
la captura de contexto como la publicación de `expr_vars` y el marcado
`Deferred`; la bisección atribuye el primer cambio al commit, no prueba
qué subexpresión interna es suficiente por sí sola.

`f5b12b1` continúa probando ramas después de un error anterior de la unión
y recurre al `expression_variable` de una rama. Corrige el discriminador
`later_union_branch_preserves_deferred_equality`, pero en la semilla 830
añade una quinta NX-E0103 en `[182,189)` a las cuatro de main. El oráculo
completo D12 la considera defecto. El checkpoint `960df52` recupera B_M1
y los 38 discriminadores sin quitar los 61 defectos D12 restantes; por eso
trasladar esa secuencia completa no cumple el objetivo de la rama mínima.

Se probó en árbol aislado una variante de P1d que dejaba `Type::Deferred(id)`
local a la igualdad, sin escribir `expr_vars` ni marcar `Deferred`. Owned
seguía con 0 fallos, pero el censo dio 42 defectos D12, 18 sondas extra
fallidas y 36 discriminadores fallidos. Se restauró el árbol a `2841cc6`;
ninguna variante experimental entró en `i07c-rediseno-c2-min`.

Pendiente: reconstruir la condición causal de publicación de P1d por
igualdad/epoch y acotar la continuidad de la rama posterior de `f5b12b1`.
Solo después se añaden bloques a la rama mínima con medición por commit.
Los fuzz grandes, el clasificador final, la puerta cruzada completa, la
puerta mecánica y v15f siguen no evaluados para la rama mínima.
