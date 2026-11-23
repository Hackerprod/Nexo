# INFORME-8 — Ronda 8: subconjunto mayor de piezas sin `atomic0` con 0 regresiones de core (PARCIAL; se completa abajo)

EXPLORACION, no integrable. Inicio 2026-11-23 07:04. Binarios: `main` (eeb3c9c) y candidato-5 con interruptores `NEXO_XV` (v8). Corpus: c1, c2 (`f0-0003`), corpus-b, perm, corpus-t y las 3 semillas de B (nivel core frente al oraculo; `eval3.py`).

## Hallazgo 1 (decisivo): la base ALT-2 sin `atomic0` ya regresiona; ningun subconjunto de piezas la salva

| conjunto (todo sin `atomic0`) | regresiones de core (A) frente a `main`, todos los corpus |
|---|---|
| base ALT-2 sola (`NEXO_XV=` vacio: todas las piezas apagadas) | **1007** (c1 516, t 61, b 67, perm 51, B 13+10+8, c2 ~230) |
| solo RM-M3 (`rmm3b,e,f`) | 1004 |
| solo RE (`e0103rule,skipv3`) | 990 |
| solo R1 (`pendfa,retfa,retract,nojoin,nojoinr,retfail`) | 73 |
| R1+RE (sin match) | 63 |
| R1+RE+RM-M3 (candidato-7, sin `atomic0`) | 56 en c1/t/B (+7 en c2 segun la tabla por forma) |
| R1+RE+RM-M3 **con** `atomic0` (candidato-5) | 0 |

De las 56 regresiones del candidato-7, **54 ya aparecen con TODAS las piezas apagadas** (la base no conmutable difiere de `main`) y solo 2 dependen de una pieza (`retfail`: c1 5498, s9908255 3976; ademas c1 19152 con `retract` y s7021993 745 con `pendfa`/`nojoin` aislados).
Quitar una sola pieza del candidato-7 no restituye `main` salvo en 3 de 56; anadir `atomic`, `defmark` o `noseed` tampoco (0 de 56); `atomic0` restituye `main` en las 56 (`clasificacion-56.json`, `clasificacion-56-minimos.json`).
Es decir: el registro de ancla por union (ALT-2) necesita la atomicidad del contraste (`atomic0`) para no dejar ligaduras parciales; **no hay subconjunto de las piezas R1/RE/match que lo sustituya**. Mecanismo visible en los 5 minimos (`ejemplos-regresiones-base.txt`):
tras denunciar la union, una asignacion posterior (`r=q;q=[[ar]]`) produce un NX-E0102 extra (p.ej. t 3584: `main` y oraculo = E0103 `[85,87)`, E1303 `[126,128)`; sin `atomic0` anade E0102 `[146,148)`).

Por el criterio de parada de la ronda, **no hay subconjunto sin `atomic0` con 0 regresiones de core** (nada integrable de d2); los defectos quedan en la lista de `main`.

## Hallazgo 2: piezas excluidas — defectos de `main` que quedan sin arreglar (cifras medidas hoy, todos los corpus, nivel core)

Medicion: conjunto completo del candidato-5 (con `atomic0`) frente al mismo conjunto sin una pieza (hojas de abajo: `no_<pieza>`), contando programas de los corpus donde `main` falla frente al oraculo, el completo acierta y el conjunto sin la pieza falla (**arreglos perdidos**), y programas donde `main` acierta y el conjunto sin la pieza falla (**regresiones**). Salida completa con 2 ejemplos minimos por pieza (fuente, `main`, oraculo, sin pieza, completo): `ex8.out`; cifras en `piezas-excluidas-cifras.json`.

| pieza | arreglos perdidos | regresiones que aparecen | ejemplo minimo (corpus, indice) |
|---|---|---|---|
| `pendfa` | 316 | 868 | c1 4896 |
| `retfa` | 142 | 26 | c1 6347 |
| `retract` | 49 | 3 | c1 4929 |
| `nojoin` (+`nojoinr`) | 178 | 1 | c2 6392 |
| `atomic0` | 247 | 56 | c2 2321 |
| `e0103rule` | 290 | 11 | b 80 |
| `retfail` | 207 | 12 | c1 14436 |
| RM-M3 (`rmm3b/e/f`) | 7 | 7 | c2 10405 |
| `skipv3` | 0 en corpus (su efecto es de aceptacion) | 0 | `fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=a;}` (main rechaza con E0103 `[66,67)`) |

(los conjuntos «sin la pieza» parten del completo con `atomic0`; las cifras de «arreglos perdidos» de una pieza no son aditivas.)

## Hallazgo 3: suite real y tabla declarada

* Suite real (debug, `--profile default`) del candidato-7 = candidato-5 sin `atomic0`: 2167 ejecutados, 15 fallan (`tests-candidato7.log`): quota 2, causal_obligations 6, mutation_regressions 3 (anchored_first…, cyclic…, linked_array…), normative_survivors 2, `failed_nested_union…` y `grouped_operands…`; ya no fallan `incompatible_prefix…` ni `planned_projection…`. Con `atomic0` fallan 17 (INFORME-6). No hay subconjunto elegido, luego no se repite la suite para otro.
* Tabla declarada (`match-conservacion-ext.py --check`, 103 filas): con el candidato-7 (sin `atomic0`, con RM-M3) cambian **3** filas respecto de lo declarado (`dif-s7021993-1253-quitado`, `dif-s4417061-1190-quitado`, `dif-s4417061-4363-quitado`). Sin RM-M3 y con `atomic0` cambian 34 filas, **9 de ellas del grupo `conservacion`** (la base ALT-2 sin RM-M3 ya altera 9 de las 51 filas de conservacion respecto de `main`); sin RM-M3 y sin `atomic0`, 35 (9 de conservacion). Es decir RM-M3 no es independiente de la base: sin el, la tabla de conservacion pura no se conserva, y habria que regenerar la tabla declarada.
* Pila y pasadas de planning: **no medidas** (no hay subconjunto elegido). El codigo del candidato-7 es el del candidato-5 salvo la lista por defecto; la ronda 5 midio el par `drive`+`resume_non_match` en 60.616 B y 3 pasadas maximas.
* `candidato-8.diff`: **no se entrega** (no hay subconjunto con 0 regresiones; el parche limpio sin `xv(...)` no tiene contenido que ofrecer). Nada integrable de d2.

## Limites

Acotado por el criterio de parada: no se midieron aceptacion `check`/`run` ni la matriz de 45 para subconjuntos (todos tienen regresiones de core). Las guardas estrechas de la tarea (2) no se probaron: las 54 de 56 regresiones sin `atomic0` no dependen de ninguna pieza (la base con todo apagado ya difiere de `main`), de modo que ninguna guarda en una pieza puede evitarlas; solo se evitan restituyendo la atomicidad (`atomic0` o su rediseno por Flujo A). Dos de las 56 si dependen de `retfail` (c1 5498, s9908255 3976), una de `retract` (c1 19152) y una de `pendfa`/`nojoin` (s7021993 745); no se mide una guarda para ellas porque quedan 52 independientes de las piezas. Las 6 filas de la tabla del hallazgo 1 provienen de `eval3.py table main <tag>`; las filas por forma de c2 se agregan al total.
