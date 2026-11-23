# Exploracion S5c: ancla de union (prototipos desechables, NO integrable)

Subagente X. Rama local `i07c-s5c-explor` (sobre `origin/main` = `1e2e393`), sin push. Inicio 2026-11-22 19:37 (reloj de la maquina), fin ver el pie. Nada de lo siguiente es codigo integrable: el parche es un prototipo con interruptores de entorno (`NEXO_XV`) para medir variantes con un solo binario.

## 0. Resultado en una frase

Ningun candidato llega a 0 en (A) con la comparacion COMPLETA (codigo, primario, tipos, relacion `expected_type`, cardinalidad): el mejor (candidato 2, variante 2f) deja **0 regresiones en codigo+primario+cardinalidad (nivel `core`)** y **6 sobre 24.000 con relacion**, todas del mismo tipo (la relacion `expected_type` de un `NX-E0102` de asignacion apunta a otro origen que el del oraculo). Por el criterio de corte del brief, la entrega es el informe medido mas el parche del mejor candidato y sus diferencias, como entrada para el rediseno; NO se afirma exito.

## 1. Corpus congelado y arnes (reproducible)

- Generador `gen_family.py` (semilla 7391045, 24.000 programas unicos), corpus `corpus-congelado.json`, SHA-256 `e78702d1ee98178fd2a5809ccfc49c35dcbc3b7438b9664072c5c42742d7cfe9` (fijado y commiteado ANTES de ejecutar ningun candidato sobre el; el mismo generador con otra semilla, 20261122, solo se uso para validar el arnes con `main`).
- Prefijo de cada programa: `fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];` (+ `var s=[];` si hace falta). Union = arbol binario de `if` entre parentesis, profundidad de anidamiento 1-3 (mezcla 1,1,2,2,2,3,3), hasta 6 hojas, hojas {`q`,`r`,`s`,`1`,`2`,`true`,`ar`,`aar`,`r3`,`[q,r]`,`r[0]`,`[q]`} (variable pendiente en `then` y `else`; anidados en primera, segunda y ultima posicion por azar), 0-2 asignaciones de 14 formas (compatibles e incompatibles, antes o despues de la union, ambos ordenes), contextos {`let`, `Print(...)`, elemento de array, `== ar`, `== aar`, anotacion `I64`}.
- Oraculo: `scripts/i07c_s5c_oraculo.py` (`derive`) de la rama `i07c-estructural-r2`, sin modificar (24.000/24.000 programas modelados). Firma comparada por diagnostico: codigo, subcausa, primario, `expected_type`, `found_type`, relacion `expected_type` (span), y cardinalidad; orden de la lista normalizado. Dos niveles: `core` = (codigo, subcausa, primario, cardinalidad); `full` = `core` + tipos + relacion.
- Comparacion contra `main` = `origin/main` 1e2e393 (binario `base-release.exe` de C, mismo arbol). `main` coincide con el oraculo en 20.096/24.000 (full) y 21.091/24.000 (core: `main` falla 2.909).
- Scripts: `run_family.py` (ejecuta N binarios con `NEXO_XV=...`), `analyze.py`, `tabla.py` (tabla A-D de todas las variantes: `tabla-variantes.md`), `matriz45.py` (las 45 filas), `passes.py` (pasadas de planning), `regresiones-A.json` (cada regresion (A) con oraculo, main y candidato).

## 2. Candidatos, mecanismo y variantes medidas

Todos los resultados son `check --format json --profile f0-0002-b-v2`, release, 24.000 programas. (A) = `main` acierta y el candidato no; (B) = `main` falla y el candidato acierta; (C) = ambos fallan igual; (D) = ambos fallan distinto. Tabla completa por variante y nivel: `tabla-variantes.md`. Resumen (A / B), nivel core | full:

| variante | mecanismo anadido | core A / B | full A / B |
|---|---|---|---|
| **Candidato 1** (ALT-2 S1 de C, `s1-arbol-sin-confirmar.diff`, + ablacion de C) | | | |
| 1a | el retorno temprano de `if_finish_join` (`anchor_pending \|\| anchor_absorbed`) devuelve `first_for_anchor.or(first).or(second)` (el testigo parcial, no `second`) | 10 / 2.413 | 16 / 2.423 |
| 1b | idem en el retorno de `anchor_error` | 12 / 2.488 | 16 / 2.537 |
| 1c | 1b + `retract_pending_joins` (el contraste absorbido borra tambien las marcas `deferred_join`/`recursive_join`/`branch_join_scopes` del nodo) + `anchw` (una union cuya primera continuacion es un `if` fallido con testigo parcial usa ese testigo como ancla propia, con el span del `if` interior) | 6 / 2.538 | 8 / 2.588 |
| 1d | 1c + `nojoin` (con ancla presente la union no ejecuta su `unify_independent` local) | 5 / 2.557 | 7 / 2.607 |
| (descartada) 1x | `anchored=false` al devolver un testigo parcial | 2.257 | 2.140 |
| **Candidato 2** (candidato 1 + parte de V6) | | | |
| V6 original | referencia | 53 / 78 | 84 / 76 |
| 2a | 1c + V6 completo (retraccion `seed_cap` + `rejected_assignments`) | 4 / 2.550 | 14 / 2.614 |
| 2b | 1d + V6 completo | 3 / 2.568 | 13 / 2.632 |
| 2c | 1d + solo `rejected_assignments` de V6 (sin retraccion de capas) | 2 / 2.568 | 4 / 2.618 |
| 2d | 2c + `atomic0` (el primer contraste incompatible restaura los nodos previos: igualdad atomica) | 1 / 2.579 | 7 / 2.659 |
| **2f** | 2d + `arrx` (en `finalize`, un `array` fallido tambien excluye al constructor exterior de su elemento) | **0 / 2.587** | **6 / 2.667** |
| 2g | 2f + `atomic` (el contraste absorbido restaura nodos y obligaciones) | 0 / 2.587 | 7 / 2.672 |
| **Candidato 3** (eliminar la siembra anticipada, sobre 1c) | | | |
| 3a | `seed_planned_shape` no inyecta la evidencia concreta (`concrete_plan`) | 13 / 2.538 | 33 / 2.609 |
| 3b | `seed_planned_shape` no siembra capas (`noshape`) | 15 / 2.537 | 35 / 2.608 |
| (descartada) `defmark` sobre 2f | marcar `deferred_join` en cada contraste compatible con pendiente | 1 | 30 |

Aviso de tope: el brief admite 3 candidatos y <=3 variantes de ajuste por candidato; aqui hay 3 candidatos pero mas de 3 variantes encadenadas en los candidatos 1 y 2 (cada una anade un mecanismo y se midio entera). Se declara.

Diferencias respecto al bloque de la matriz: **matriz de 45 filas** (script `matriz45.py`, esperados derivados por el oraculo): `main` 22 core OK / 21 full OK; ALT-2 de C (1a) 38 / 37; V6 26 / 25; **2f: 45 / 45 core**, 38 full (los 7 que difieren solo en relacion: B_rq, P1, c3, w1, P2, P3, w3; B_rq es regresion de relacion frente a `main`; P1, c3, w1, P2, w3 fallan en `main` por codigo). R4, P15 y P16 (filas de B) quedan verdes en 2f.

## 3. Mecanismo del candidato 2f (texto, no plan)

Sobre `origin/main`, en `src/check/types.rs` (diff `candidato-2.diff`; los interruptores de entorno y la funcion `xv` solo sirven para medir variantes, el comportamiento por defecto es el de 2f):
1. Registro de ancla por union (`union_denounced`), `anchor_open`, `anchor_contrast` (unico punto donde una hoja se iguala con el ancla), `second_leaf_anchor` (hoja `else`, incluida la variable pendiente) y `if_else_context` que propaga el ancla a los `if` posteriores anidados (ALT-2 de C, sin la rama de descarte). La primera hoja incompatible marca la union como denunciada; las siguientes se rechazan sin diagnostico propio.
2. La union anidada en primera posicion devuelve a la exterior su testigo parcial (`first_for_anchor`) tambien en los retornos de error/pendiente/absorbido, y la exterior lo adopta como ancla propia (origen = span del `if` interior) aunque el interior haya fallado (`anchw`): esto es lo que arregla la familia de B (anidado en 1.a posicion con hoja `else` externa `q` pendiente).
3. Con ancla presente la union no ejecuta su `unify` local (`nojoin`): el `join` local de una union cuyo primer operando ya es un testigo anclado igualaba hojas del segundo nivel con el testigo parcial del primero (causa de que `(if p then [q,r] else (if p then (if p then r else 1) else (if p then ar else ar)))` fije el elemento a `I64`).
4. Igualdad atomica: un contraste absorbido borra los efectos de su `unify` (joins, `deferred_join`, `recursive_join`, `branch_join_scopes`); el primero incompatible restaura los nodos previos (`atomic0`). Esto retira la fuga de capas rechazadas hacia `shape_plan` (la causa de un `NX-E0102` espurio en `r=q` tras `(if p then [q,r] else (if p then ar else r))`).
5. De V6 solo se conserva `rejected_assignments` (una asignacion rechazada o diferida en planning no habilita `read_plan`; `note_assignment_verdict`). La retraccion de capas sembradas (`seed_cap`) NO se usa: arregla `(if p then r[0] else ...); r=[3]` pero rompe relaciones (rel `None` o el origen equivocado) y anade E1303 espurios (20290, 21644).
6. `arrx`: en la emision de `NX-E0103`, el constructor exterior cuyo elemento es un array no resuelto se excluye aunque el interior este marcado fallido (`has_cause`), porque el oraculo solo emite `NX-E0103` para elementos que son variable desnuda.

## 4. Regresiones y causas (ejemplos minimos)

Prefijo `fn Main(){let p=true;let ar=[1];var q=[];var r=[];`. Todos verificados con binarios; "main = oraculo" en cada uno.

| candidato | programa | oraculo = main | candidato | causa |
|---|---|---|---|---|
| 1a | `let a0=(if p then (if p then r else (if p then true else r[0])) else r);r=q;q=[[3]];` y `(if p then r else (if p then true else r[0]));r=q;q=[[3]];` | solo E1303 en `true` | + E0102 espurio en `r=q` | contraste absorbido de `r[0]` (recursivo): deja `recursive_join` y capas en `shape_plan`; arreglado por `retract` |
| 1a | `let a0=(if p then (if p then r[0] else (if p then r else aar)) else r);` | 2 E1303 (el segundo sobre la `r` exterior) | pierde el segundo | la union exterior no adopta el testigo del `if` interior fallido (`anchored=true` bloquea el ancla); arreglado por `anchw` |
| 1c | `var s=[];let a0=(if p then (if p then [q,r] else s) else (if p then (if p then r else 1) else (if p then ar else [q,r])));` | E0103 en q y r | solo E1303 en `[q,r]` | el `join` local de la union de segundo nivel iguala su testigo con `ar`; arreglado por `nojoin` |
| 1d | `let a0=(if p then (if p then r[0] else r[0]) else (if p then q else 1));r=[3];` | solo E0103 en `q` | + E0103 en `r` | la asignacion `r=[3]` ya no la absorbe el plan (siembra de forma); V6 `rejected_assignments` lo arregla |
| 2c | `let a0=(if p then [q,r] else (if p then ar else r));r=q;q=[[ar]];` | solo E1303 en `ar` | + E0102 en `q` de `r=q` | fuga de nodos del primer incompatible; `atomic0` |
| 2d | `let a0=(if p then [q] else (if p then 2 else r[0]));r=ar;` | solo E0103 en `q` | + E0103 en `r` | `mark_failed_constructor` por la asignacion; `arrx` |
| 2f (residual) | ver sec. 5 | | | procedencia de la relacion |

## 5. Regresiones que quedan (2f, 6/24.000 con relacion, 0 con core)

Todas son `NX-E0102` de asignacion con el mismo codigo, primario, tipos y cardinalidad, y distinta relacion `expected_type` (cuerpo tras el prefijo; salidas completas en `regresiones-A.json`):

1. `let a0=(if p then (if p then r else q) else (if p then [q,r] else true));q=[3];r=[true];` oraculo rel `[154,157)` (el `[3]` de `q=[3]`); 2f rel `[115,116)` (la hoja `q`).
2. `var s=[];let a0=(if p then (if p then [q,r] else s) else r);q=[3];s=[true];` oraculo `[117,122)` (el operando `[q,r]`); 2f `[141,144)` (el `[3]`).
3. `var s=[];r=[3];let a0=(if p then s else (if p then aar else [q,r]));q=[[ar]];` oraculo `[142,143)` (`r` dentro de `[q,r]`); 2f `[130,133)` (`aar`).
4. `let a0=(if p then (if p then (if p then r else r[0]) else (if p then [q] else r)) else [q])==aar;q=[[3]];r=q;` oraculo `[148,151)` (`[q]`); 2f `[178,183)` (`[[3]]`).
5. `var s=[];let a0=(if p then (if p then q else true) else r[0])==aar;q=[3];r=[true];`
6. `let a0=(if p then (if p then q else (if p then r[0] else r)) else 1)==aar;q=ar;r=ar;`

Causa probable (no demostrada): en `main` la evidencia concreta de una asignacion posterior no se siembra cuando la cadena tiene un `PendingDeferredJoin` (`has_deferred_join`), y el origen del tipo queda en el operando de la union; con ALT-2 las hojas compatibles se acumulan sin join pendiente, se siembra `concrete_plan` y el origen pasa a ser la RHS de la asignacion o la hoja. Es la familia que el diseno ya aislaba como procedencia (ALT-4/S5a.5): la regla de relacion (ADR l.371, "operando de la primera continuacion" para la union; origen concreto "siguiendo aliases" para la asignacion) no la implementa el verificador en terminos de orden fuente. `defmark` (marcar `deferred_join` en cada contraste compatible) lo intenta y empeora (30).

Fallos de (C)/(D) que NO son regresion (el candidato no los introduce): 322 programas del corpus siguen sin coincidir en core (2f) porque `main` ya falla; la categoria mayor es `NX-E0103` con primario en un nombre (p. ej. `(if p then r[0] else r)` da un E0103 en la hoja `r`, 105 programas) y perdida de la segunda/tercera `E1303` de uniones anidadas (99). Es el defecto previo que B ya senalo (E0103 con primario en un nombre).

## 6. Terminacion y pila (candidato 2f)

- Pasadas de planning por funcion sobre los 24.000 programas: maximo 3 (histograma 1: 75, 2: 23.886, 3: 39); tope 129 sin acercarse.
- Pila (rustc 1.99.1, perfil de test con debuginfo, `/MAP` + `scripts/f5-frame-sizes.py` con `dumpbin` de VS 2022 BuildTools): `drive` 27.536 B, `resume_non_match` 33.080 B, pareja **60.616 B** (identica a la linea base; crecimiento 0 B, margen 5.944 B). `scripts/f5-stack65.py`: ver el pie.
- Campana `twin 100000 ... 4 3`: NO ejecutada (el barrido no dio 0 en (A) con relacion, y el prototipo no es integrable).

## 7. Lo que se aprendio del ADR sobre el ancla

1. El ancla de una union cuya primera continuacion es otro `if` es el testigo parcial de la union interior, aunque esa union haya fallado; el `anchored=true` que ve la exterior no puede significar "no hay ancla". Hay que separar (a) el estado de fallo/denuncia de la union interior de (b) el testigo que ofrece a la exterior.
2. La union debe tener UN solo mecanismo de igualdad (el contraste contra el ancla); el `join` local por pares duplica igualdades y las aplica entre testigos distintos (2f lo apaga con ancla presente).
3. El rechazo de una igualdad incompatible debe ser atomico en el grafo de tipos (nodos), no solo en las obligaciones: sin ello las capas que construye el unificador antes de fallar llegan a `shape_plan` y reaparecen sembradas.
4. Una asignacion posterior rechazada o diferida no debe habilitar la siembra anticipada (`rejected_assignments` de V6 basta); la retraccion de capas de V6 es la parte que daña la procedencia.
5. La relacion `expected_type` de una asignacion no es un efecto colateral de a quien se le siembra la evidencia: necesita su propia regla (orden fuente) para no depender de si hay joins pendientes. Es lo unico que impide 0 en (A) con relacion.

## 8. Limites del barrido (lo que NO cubre)

- 24.000 programas de una sola funcion `Main`, hasta 6 hojas y profundidad 3, de 12 hojas fijas; sin `match`, sin funciones propias, sin bucles, sin `Option`/`Result`, sin `value`-bloques, sin `else if` sin parentesis, sin anotaciones `Array<...>`; las constantes `ar`, `aar`, `r3` son de `I64`; contextos externos limitados a {`let`, `Print`, elemento de array, `==`, anotacion `I64`} (la anotacion siempre falla por tipo). Solo `check`, release; no se comparo `run`, ni debug/release.
- El oraculo modela un subconjunto y sus decisiones "no fijadas" (p. ej. testigo de variable fijada en elemento de literal) se toman con la lectura mas cercana; esperados no validados por un segundo oraculo. 24.000 es una muestra aleatoria (semilla unica), no la familia exhaustiva: las 6 regresiones restantes se encontraron por muestreo y puede haber mas del mismo tipo.
- No se ejecuto la suite de tests, ni `validate.sh`, ni el corpus de 3.674 fuentes, los 152 RD, el fuzz ni la campana twin. El prototipo lleva `eprintln!` de depuracion y clones de `type_nodes` (coste), y `xv()` lee el entorno: no apto para medir tiempos.
- No se aplico el flujo A/B: este documento es evidencia para el rediseno, no plan.

## 9. Recomendacion

1. Tomar 2f como base de diseno: es el unico mecanismo que lleva a 0 las regresiones de codigo+primario+cardinalidad (y gana 2.587 programas que `main` falla en core) con crecimiento de pila 0 y 3 pasadas como maximo.
2. Antes de llevarlo a plan, A/B deben decidir la regla de procedencia de `expected_type` (6 residuales) y resolverla en el ADR con ejemplo valido e invalido (laguna de nucleo con mas de una lectura razonable: va por Flujo A); sin esa regla 0 en (A) con relacion no es alcanzable con ALT-2 solo.
3. No usar la retraccion de capas de V6 ni la eliminacion de la siembra (candidato 3): ambas empeoran.
4. Los defectos de `main` que 2f deja intactos (E0103 con primario en un nombre; perdida de E1303 en uniones profundas) conviene tratarlos aparte (322 programas).

## Pie (cierre de la ronda 1)

- `scripts/f5-stack65.py` sobre 2f: `option_match126` y `result_match126` a 65 KiB, 5/5 cada uno (exit 0).
- Fin de la ronda 1: 2026-11-22 21:46 (reloj de la maquina). La ronda 2 (formas nuevas) esta en `INFORME-2.md`.
