# Exploracion S5c, ronda 3: candidato 2g' (correccion de los defectos de B sobre 2f)

Subagente X, rama local `i07c-s5c-explor`, sin push. Inicio 2026-11-22 22:36, fin 2026-11-23 00:40 (reloj de la maquina). Todo es prototipo desechable; el parche es `candidato-3.diff` (arbol completo contra `origin/main` 1e2e393; los interruptores `NEXO_XV` siguen en el codigo, el comportamiento por defecto es 2g').

## 0. Resultado

**2g' tiene 0 regresiones de core frente a `main` en TODOS los corpus pedidos**, con `check`: corpus 1 (24.000), corpus 2 (16.000; perfiles `f0-0002-b-v2` y `f0-0003-b-v1`, que dan resultados identicos en las formas aceptadas por ambos), `corpus-b` (3.959 modelados; SHA 83f9e726...), `perm.json` (4.044), `corpus-t` (10.584) y la matriz de 45 (45/45 core). B reprodujo 2 regresiones en `corpus-b` y 7 en `matchexpr`: las tres causas (R-A, R-B, R-C) quedan corregidas con una pieza nueva, `retfail`, y otra, `matchpartial`; ademas se retiran dos piezas sin justificacion (R2 y `anchw`). La regla del E0103 queda unificada. No se encontro mecanismo pequeno para los 99 `NX-E1303` perdidos (sec. 6). La relacion `expected_type` no se toco (6 diferencias de relacion en el corpus 1 siguen, deuda aparte).

## 1. Piezas finales (8) y ablacion de cada una

Conjunto por defecto: `pendfa, retfa, retract, nojoin, atomic0, e0103rule, retfail, matchpartial`. Ablacion = el conjunto menos esa pieza, sobre c1+b+perm+t (core A = regresiones frente a `main`; "difiere" = programas cuya firma completa cambia respecto de 2g'); `matchpartial` sobre c2 en `f0-0003`. Programas minimos en `minimos-ablacion.txt` (prefijo `fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];`).

| pieza | que hace | sin ella: regresiones core | difieren | programa minimo (esperado = oraculo = main) |
|---|---|---|---|---|
| `pendfa` | el retorno temprano `anchor_pending/absorbed` de `if_finish_join` devuelve el testigo parcial `first_for_anchor.or(first).or(second)` | **399** (c1+b+perm) | 582 | `let a0=(if p then (if p then r else true) else r[0]);` -> solo 2 NX-E0103; sin ella: NX-E1303 sobre `true` |
| `retfa` | idem en el retorno de `anchor_error` | **8** | 154 | `var s=[];let a0=(if p then (if p then [q,r] else r) else s);r=[ar];` -> solo NX-E1303 `[128,129)`; sin ella: + NX-E0103 de `s` |
| `retract` | un contraste absorbido retira tambien `deferred_join`/`recursive_join`/`branch_join_scopes` del nodo | **1** | 111 | `let a0=(if p then (if p then 1 else r[0]) else (if p then r else q));r=q;` -> E1303 `[137,138)`; sin ella: 2 NX-E0103 |
| `nojoin` | con ancla presente la union no ejecuta su `unify` local | **1** | 32 | `var s=[];let a0=(if p then [q,r] else (if p then (if p then r else 1) else (if p then ar else [q,r])));` -> 3 NX-E0103; sin ella: E1303 espuria con el elemento fijado a `I64` |
| `atomic0` | el primer contraste incompatible restaura los nodos previos (igualdad atomica) | **54** | 639 | `let a0=(if p then r[0] else (if p then q else 1));r=[3];` -> solo NX-E0103 de `q`; sin ella: + NX-E0103 de `r` |
| `e0103rule` (parte `arrx`) | un array fallido tambien excluye al constructor exterior de su elemento en el cierre | **9** | 51 | `let a0=(if p then [q] else (if p then 2 else r[0]))==ar;r=ar;` -> solo NX-E0103 de `q`; sin ella: + el de `r` |
| `e0103rule` (parte `skip`) | no emitir `NX-E0103` cuando el constructor no tiene `array_origin` (no es un literal `[]`) | 0 | 124 (todos mejoras: `main` falla en ellos) | `let a0=(if p then r[0] else r);` -> 2 NX-E0103 (`[67,69)`, `[76,78)`); `main` y 2f dan un tercero sobre la hoja `r` |
| `retfail` (nueva) | el retorno por fallo de `if_finish_join` ofrece el testigo parcial de la union (`first_for_anchor`) y no el tipo de su segunda continuacion | **2** (corpus-b; son R-A y R-B) | 103 | R-A `let a0=(if p then (if p then [r] else (if p then ar else [[q]])) else r[0]);` -> solo NX-E0103 `[67,69)`; R-B `var s=[];let a0=(if p then (if p then r else (if p then 1 else q)) else (if p then s else 2));q=r;r=[true];` -> E1303 `[135,136)` y `[169,170)` |
| `matchpartial` (nueva) | un brazo de `match` cuyo valor es una union con testigo parcial (`PartialNormal`) se une igualmente con el brazo siguiente | **7** (c2, `matchexpr` en `f0-0003`) | 340 | `q=ar;let a0=match move o{Some(n)=>{value: (if p then (if p then 1 else s) else r);},None=>{value: [q];}};` (prefijo con `let o:Option<I64>=Some(1);` y `var s=[];`) -> NX-E1303 sobre `[q]`; sin ella se pierde |

Retiradas por la ablacion (sin ablacion no hay pieza): **R2** (`rejected_assignments` de V6): anadirla a 2g' no cambia la firma completa en ningun programa de c1+b+perm+t (0 diferencias en 42.628) y 2g' no tiene regresion sin ella; **`anchw`** (la union exterior adopta como ancla el testigo de un `if` interior fallido): quitarla no cambia ninguna firma en c1+b+perm+t y con 2g' sin ella c2 sigue en 0 regresiones; hacia delante `retfa`/`retfail` cubren la familia que `anchw` arreglaba. Ambas siguen como interruptores apagados en el prototipo.

## 2. Correccion de R-A, R-B y R-C (variantes medidas; maximo 3 por defecto)

- **R-A** (`(if p then (if p then [r] else (if p then ar else [[q]])) else r[0])`; tipo inventado A³): causa = la hoja `r[0]` (un resultado de proyeccion, `projection_items`) se contrastaba con el testigo y quedaba "compatible", y la `NX-E1303` pendiente de `ar` se completaba al cierre con tipos del testigo equivocado; ademas el tipo devuelto por la union interior fallida era el de su segunda continuacion (`I64`) y no su testigo `r`. Variantes: (1) `noprojB` (no contrastar hojas que son proyeccion): arregla R-A pero causa **159** regresiones en corpus-b; descartada. (2) `retfail` (devolver `first_for_anchor` tambien en el retorno por fallo): arregla R-A y R-B, 0 regresiones. Adoptada.
- **R-B** (`atomic0` pierde la segunda E1303 y emite `NX-E0103` de `s`): causa = el testigo de la union exterior era `I64` (la segunda continuacion de la interior) y no el elemento `r`; `atomic0` restauraba el contraste `s` contra `I64`. Variantes: (1) `atomic0e` (restaurar solo con error): arregla R-B pero reintroduce el `NX-E0102` espurio de `(if p then [q,r] else (if p then ar else r));r=q;q=[[ar]];`; (2) `atomic0p` (restaurar solo en planning): no arregla R-B; (3) `retfail`: arregla R-B conservando `atomic0`. Adoptada la (3).
- **R-C** (`match` como expresion, 7 programas): causa medida = la union del primer brazo vuelve a `branch_accept` con `FailedJoin` (por el tercer valor del retorno temprano `anchored=true`) y el `match` no une un brazo fallido con el siguiente. Variantes: (1) `pendnf` (`failed=false` en el retorno por pendiente): no arregla; (2) `pendsoft` (`failed=false` y `anchored=false`): arregla el minimo pero causa **152** regresiones en corpus-b; (3) `matchpartial` (el `match` une el brazo si tiene `PartialNormal`): arregla los 7 con 0 regresiones. Adoptada la (3). Con ella `matchexpr` pasa de 293 C / 54 D / 7 A (2f) a **0 A** y 365 B frente al analogo (`if p then U else HOJA`).
- Decision abierta (de B, no mia): si un brazo de `match` es "consumidor distinto" que no recibe el tipo de una union fallida (ADR l.361/363) sigue sin fijar; 2g' iguala a `main`.

## 3. Unificacion de la regla de finalizacion de `NX-E0103`

`arrx` y `e0103skip` pasan a una sola pieza `e0103rule`: `NX-E0103` solo sobre constructores fuente (`[]` con `array_origin`) cuyo elemento sigue sin constructor ni tipo, y un array cuyo elemento es un array no resuelto no emite el suyo (el interior es la causa; ADR l.301). Efecto sobre los 322 programas del corpus 1 que `main` y 2f fallan (`defectos-previos-322-r3.json`): **2g' arregla 173 de 322** (101 de los 105 "NX-E0103 de mas", los 25 dobles, los 14 "NX-E1303 de mas", 5, 4 y 4 mixtos; `e0103skip` sola arreglaba 102); quedan 149: 95 de los 99 "E1303 perdida", 21 "E0102 perdida", 21 de "primario distinto" y otros. Ningun programa que 2f acierta lo pierde 2g' (0).

## 4. Tablas A-D por corpus (core; frente a `main` y al oraculo)

A = `main` acierta y 2g' no; B = `main` falla y 2g' acierta; C = ambos fallan igual; D = ambos fallan distinto. Salida completa (core y full): `tabla3-final.md`.

| corpus | n | sin oraculo | ok | **A** | B | C | D |
|---|---|---|---|---|---|---|---|
| c1 | 24.000 | 0 | 21.091 | **0** | 2.760 | 140 | 9 |
| corpus-b | 4.000 | 41 | 3.066 | **0** | 545 | 342 | 6 |
| perm | 4.044 | 222 | 3.379 | **0** | 342 | 101 | 0 |
| corpus-t | 10.584 | 0 | 6.056 | **0** | 4.528 | 0 | 0 |
| c2 (suma de las 16 formas, `f0-0003`) | 16.000 | 191 | 14.232 | **0** | 1.503 | 72 | 2 |

Por forma en c2 (`tabla3-c2-core-f0003.md`): A = 0 en las 16 formas (`flat`, `blockstyle`, `fn`, `fnparam`, `if`, `while`, `match`, `fn+while`, `fn+match`, `annot`, `arg`, `ret`, `option`, `result`, `takeempty` y `matchexpr`). Con la comparacion completa (core + tipos + relacion) siguen A = 6 (c1), 2 (corpus-b), 6 (perm) y 18 (corpus-t): son la misma familia de relacion `expected_type` (no tocada) y las de B de `perm`/`corpus-t` ya estaban en 2f.

## 5. Matriz, `run`, pila, pasadas

- Matriz de 45 filas (`matriz45.py`): `main` 22 core / 21 full; **2g' 45 core / 38 full** (release y debug iguales).
- `run` sobre una submuestra de 6.964 programas (1 de cada 5 del corpus 2 en `f0-0003`, 1 de cada 2 de corpus-b y 1 de cada 6 de corpus-t), 4 binarios (main/2g' x debug/release): **debug = release en ambos (0 diferencias)**; main vs 2g' difieren en 1.294 programas, **0 sin explicar** por una diferencia previa del `check`.
- Pasadas de planning: maximo **3** en la submuestra (0: 30, 1: 13, 2: 6.892, 3: 29); en la ronda 2 ya eran 3 en 40.000.
- Pila (rustc 1.99.1, perfil de test, `/MAP` + dumpbin): `drive` 27.536 B, `resume_non_match` 33.080 B, pareja **60.616 B** (sin cambio; margen 5.944 B). `f5-stack65.py`: 5/5 en `option_match126` y `result_match126` a 65 KiB.
- Perfiles: las formas con `while`/`match`/`Option` no las acepta `f0-0002-b-v2` (NX-E0006); `f0-0003-b-v1` y `f0-0005-b-v1` dan resultados identicos (ronda 2).

## 6. Los 99 `NX-E1303` perdidos (hoja literal con error interno)

Sin mecanismo pequeno. Minimo: `let a0=(if p then 1 else [ar,aar]);` (el oraculo denuncia tambien `NX-E1303` sobre el literal; `main` y 2g' solo la `NX-E0102` interna). Intento medido (`failleaf`): contrastar la hoja con su tipo parcial tomado de `expression_types`; **no cambia nada** porque el marco del literal de array no deja registrado un tipo parcial cuando un elemento es incompatible (el tipo de la hoja llega `None`; el contraste nunca se ejecuta). El mecanismo exige guardar al cerrar el literal el tipo del primer elemento valido como "testigo parcial" de ese operando (analogo al `PartialNormal` de un `if`) y contrastarlo en `second_leaf_anchor`/`if_after_first`; es un cambio de marco de array (`array_next`), no de la union, y queda sin implementar por la regla de no ampliar piezas sin ablacion. 2g' arregla 4 de los 99 por otras vias.

## 7. Limites

- Mismos limites que INFORME-2 (corpus aleatorios de una funcion, `for`/`else if` sin parentesis fuera de perfil, oraculo analogo en el grupo D de c2, `run` sin oraculo).
- Ablacion de c2 solo para `matchpartial`; el resto de piezas se ablacionaron sobre c1+b+perm+t (42.628 programas) porque c2 no depende de ellas en las formas medidas (`FIN == HF2` y 0 regresiones en c2 con y sin `anchw`).
- Un 0 de regresiones no prueba su ausencia fuera de los corpus: el 0 de la muestra de la ronda 1 ocultaba 2 casos que B hallo con otra semilla; el corpus de B ya esta dentro de este criterio.
- Sin suite de tests ni `validate.sh` ni RD ni `twin`; el prototipo lleva `eprintln!` de depuracion y clones de `type_nodes` (coste).

## 8. Incidencias

1. El binario `f2` salio identico a `main` porque `cargo build` reutilizo el artefacto del mismo directorio de destino (mismo caso que `k2-dbg` en la ronda 2): se detecto con `cmp` y se reconstruyo con `CARGO_TARGET_DIR` propio por nombre (`build.sh NOMBRE FUENTE DESTINO`).
2. `ablate.py --only c2` corre solo el perfil `f0-0002` (que rechaza `match`): la primera ablacion de `matchpartial` dio "0 diferencias" por error; se repitio con `--prof2 f0-0003-b-v1` (7 regresiones al quitarla).
3. Dos variantes se descartaron por regresiones masivas (`noprojB` 159, `pendsoft` 152) sin integrarse: quedan solo como interruptores apagados.
4. Cerrojo `heavy.lock`: esperas de hasta ~6 min con `lockwait.py`; no se toco el archivo ni procesos ajenos. Los barridos de procesos cortos no pasan por el cerrojo.
5. Los resultados masivos viven en `.scratch/res3/` (no versionados) y se borraron al terminar junto con binarios y destinos; se regeneran con `eval3.py run` y `ablate.py`.

## 9. Recomendacion para A/B

1. Base de diseno: las 8 piezas de la sec. 1; cada una tiene un programa minimo cuya perdida produce una regresion de codigo/primario frente al oraculo (salvo la parte `skip` de `e0103rule`, que solo corrige defectos previos y debe ir como pieza aparte con su propio ADR).
2. R2 y `anchw` fuera del plan.
3. Pendientes de nucleo (Flujo A): (a) brazos de `match` como consumidor de una union fallida (l.361/363); (b) procedencia por capa de `expected_type`; (c) testigo parcial de un literal de array con error interno (99 programas).
