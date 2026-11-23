# Exploracion S5c, ronda 2: ampliacion del barrido (2f en formas nuevas)

Subagente X, rama local `i07c-s5c-explor` (sobre `origin/main` 1e2e393), sin push. Prototipo = el 2f del INFORME.md (los interruptores `NEXO_XV` siguen en el arbol; por defecto el binario es 2f). Inicio de esta ronda: 2026-11-22 21:48 (reloj de la maquina); fin: 22:35. Nada es codigo integrable.

## 0. Resultado en una frase

En las formas nuevas **2f no tiene regresiones en `core` en 15 de las 16 formas, en los 3 perfiles, en `check` y `run`, debug y release**. La unica familia con regresion es el **`match` usado como expresion cuyo primer brazo es una union fallida**: 7 programas (0,8 % de los 869 comparables de esa forma; los mismos 7 en `f0-0003` y `f0-0005`; `f0-0002` rechaza `match`), una sola causa (ver sec. 4). Segun el brief, no se itera mas: se entrega la evidencia. Ademas: la matriz de 45 filas sigue en 45/45 core, los programas que `main` acepta/rechaza no cambian (0 cambios de aceptacion), `run` no cambia salida ni exit en ningun programa aceptado, y las pasadas de planning siguen en 3 como maximo.

## 1. Corpus 2 congelado

- `gen_family2.py`, semilla 20261123, **16.000 programas unicos**, SHA-256 `514c929f76ef4f6fba3343647353bd420d9d931c03cf039051df5584427cd864` (`corpus2-sha256.txt`), fijado y commiteado antes de ejecutar nada sobre el.
- Uniones y variables pendientes de la familia 1 como nucleo (mismas hojas y 14 formas de asignacion, ahora antes o despues de la union, contextos `let`, `Print`, elemento de array, `== ar`, `== aar`, hasta 6 hojas, profundidad 1-3, `var s=[]` opcional, hojas con bloque `{ value: X; }` en la forma `blockstyle`).
- Grupo W (envoltorio con tipado equivalente; el oraculo deriva el programa plano y sus spans se trasladan por segmentos): `fn` (funcion propia sin parametros llamada desde `Main`), `fnparam` (con parametro `view I64`), `while p{...break;}`, `if p{...}` (sentencia), `match move o{Some(n)=>{...},None=>{}}`, `fn+while`, `fn+match`, `blockstyle`, `flat`. 9.592 programas modelados por el oraculo.
- Grupo D (contexto nuevo): `annot` (`let a0:Array^n<I64>=U;`, n=1..4, incluye anotaciones compatibles), `arg` (`Take(U)` con `fn Take(a:view Array^n<I64>)`), `ret` (`return U;` con tipo de retorno Array), `option` (`Some(U)` con `Option<Array^n<I64>>`), `result` (`return Ok(U);` con `Result<Array^n<I64>,I64>`), `matchexpr` (`match` como expresion: `value: U` en un brazo y una hoja en el otro), `takeempty` (`Take([]);Take(q);` mas una union con la variable pendiente `q`: una funcion que recibe un array vacio y otra que recibe la variable pendiente).
- Sondeos previos (registrados y descartados del corpus): `for` da `NX-E0006` (caracteristica fuera del perfil) en los tres perfiles, incluso con `0..<2` en `f0-0005-b-v1`; `else if` sin parentesis en posicion de expresion da `NX-E0002` (D15 §3.2 lo prohibe: no es una laguna); un bloque `{value:...}` suelto como expresion da `NX-E0002` (solo existe como rama de `if`); los parametros deben llevar modo (`view`). `while`, `match`, `Option` y `match` como expresion dan `NX-E0006` en `f0-0002-b-v2` (formas contadas como "rechazado por perfil"); `f0-0003-b-v1` y `f0-0005-b-v1` los aceptan y dan resultados identicos entre si.

## 2. Oraculos usados (limite explicito)

- W: oraculo del ADR sobre el programa plano + traslado de spans por segmentos (`decl`, `pre`, `ctx`, `post`; el texto de la union es identico). Equivale a afirmar que envolver en funcion/bucle/if/match no cambia el tipado de la union (es la hipotesis del barrido; si `main` y 2f coinciden entre si y difieren del oraculo en una forma entera, se vera como columna (C)).
- D: **oraculo analogo** (`analyze2d.py`, `oracle2.py` = oraculo sin modificar + soporte de anotaciones `Array<...>`): `annot/arg/ret/option/result` se modelan como contexto externo determinado (`let a0:Array^n<I64>=U;`); `takeempty` como `q=<literal de n capas>;` antes de la union; `matchexpr` como `(if p then U else HOJA)`. Es equivalencia por construccion, **no** derivada del ADR para esas formas; se compara en core y en "typed" (core + tipos), sin relacion `expected_type` (los spans de anotacion/parametro/`return` no se trasladan). Para `matchexpr` la analogia es debil (293 programas fallan igual en `main` y 2f: las reglas de union de `match` no son las del `if`).

## 3. Tablas A-D por forma (core; A = main acierta y 2f no; B = main falla y 2f acierta; C = ambos fallan igual; D = ambos fallan distinto)

Salida completa por perfil en `tabla2-formas.md` (W) y `tabla2-grupoD.md` (D). Resumen en `f0-0003-b-v1` (identico en `f0-0005`; `f0-0002` salvo las formas rechazadas):

| forma | n | core ok / A / B / C / D |
|---|---|---|
| flat | 1.059 | 940 / **0** / 102 / 16 / 1 |
| blockstyle | 1.072 | 983 / **0** / 73 / 15 / 1 |
| fn | 1.142 | 1.024 / **0** / 99 / 18 / 1 |
| fnparam | 1.065 | 959 / **0** / 89 / 15 / 2 |
| if | 1.045 | 931 / **0** / 101 / 13 / 0 |
| while | 1.020 | 918 / **0** / 87 / 11 / 4 |
| match | 1.043 | 943 / **0** / 84 / 15 / 1 |
| fn+while | 1.086 | 985 / **0** / 87 / 14 / 0 |
| fn+match | 1.060 | 944 / **0** / 100 / 16 / 0 |
| annot (analogo) | 900 | 876 / **0** / 24 / 0 / 0 |
| arg (analogo) | 890 | 864 / **0** / 26 / 0 / 0 |
| ret (analogo) | 848 | 809 / **0** / 39 / 0 / 0 |
| option (analogo) | 929 | 899 / **0** / 30 / 0 / 0 |
| result (analogo) | 946 | 917 / **0** / 29 / 0 / 0 |
| takeempty (analogo; 132 sin analogo parseable) | 967 | 744 / **0** / 70 / 20 / 1 |
| matchexpr (analogo debil; 59 sin analogo) | 928 | 489 / **7** / 26 / 293 / 54 |

Notas: en W tambien con la comparacion completa (core + tipos + relacion) A = 0 en todas las formas. En `annot/arg/ret/option/result` 2f coincide con el oraculo analogo en el 100 % de los programas; los 24-39 que `main` falla por forma son un solo defecto: `main` emite, ademas de los `NX-E0102` de cada hoja, un `NX-E0102` duplicado sobre la envoltura (el `if` interior o la union entera), que la clausula 4 del ADR §4.1.2 ("sin duplicar el sitio por su envoltura") prohibe. En `f0-0002-b-v2`: `fn`, `fnparam`, `if`, `blockstyle`, `flat`, `annot`, `arg`, `ret`, `result`, `takeempty` comparados con A = 0; `match`, `while`, `fn+match`, `fn+while`, `option` y `matchexpr` rechazados por perfil (`NX-E0006`).

Cambios de aceptacion entre `main` y 2f (`check`, 3 perfiles, 16.000 programas): **0**.

## 4. Regresiones en core (A = 7, una sola causa)

Todas en `matchexpr` (ids en `regresiones2-matchexpr.json`: 1727, 3516, 6257, 7266, 8614, 11187, 14160; mismos en `f0-0003` y `f0-0005`). Ejemplo minimo (reduccion automatica, `main` = analogo; perfil `f0-0003-b-v1`), cuerpo tras `let o:Option<I64>=Some(1);let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];`:

```
q=ar;let a0=match move o{Some(n)=>{value: (if p then (if p then 1 else s) else r);},None=>{value: [q];}};
```
`main`: `NX-E0103` sobre `q`, `NX-E0103` sobre `r` y **`NX-E1303` sobre `[q]`** (el segundo brazo es incompatible con la referencia `I64` del primero; es lo que da tambien el analogo `(if p then U else [q])`). 2f: solo los dos `NX-E0103`.
Otros dos minimos: `r=ar;...{value: (if p then (if p then q else 2) else 1);},None=>{value: [q,r];}` (2f pierde la `NX-E1303` sobre `[q,r]`) y `var s=[];...{value: (if p then (if p then 1 else ar) else r);},None=>{value: [q];}};q=[3];` (pierde la sobre `[q]`).
Causa (hipotesis razonada leyendo `if_finish_join`/`branch_accept`, no confirmada con una medicion): ocurre con TODOS los interruptores apagados, es decir ya en el ALT-2 S1 base (medido); cuando la union de un brazo termina con una continuacion pendiente (`anchor_pending`), `if_finish_join` devuelve de forma temprana `(tipo, failed = true, anchored = true)`, mientras que `main` devuelve `failed = false` (con `anchored = pending_join`) porque el `join` local no ha fallado, solo ha dejado un `PendingDeferredJoin`. `branch_accept` propaga `FailedJoin` al valor del brazo y la seleccion de `match` no contrasta el brazo siguiente con un valor marcado como fallido, de modo que se pierde su `NX-E1303`. La union `if` sola no lo muestra porque no hay un segundo operando fuera de la union. (No se ha medido el arreglo, por la instruccion de detenerse.)

## 5. `run` (debug y release) y estabilidad

- `run` sobre todos los programas que `main` acepta en `check` (803 en `f0-0002`, 1.292 en `f0-0003`/`f0-0005`) mas 600 rechazados por perfil, 4 binarios (main/2f x debug/release): **debug = release en todos los casos para ambos**, y main vs 2f solo difieren (24 / 48 / 48 programas) donde ya difiere el `check` (0 diferencias sin explicar por `check`); salida, exit, eventos y simulador (`R0102` en 47/82 programas, identicos) iguales.
- Matriz de 45 filas con `matriz45.py`: `main` 22 core / 21 full; **2f 45 core / 38 full** (mismas 7 diferencias de relacion que en la ronda 1); 2f debug identico a 2f release.
- Pasadas de planning en corpus 2 (`f0-0003`): maximo **3** (0: 59, 1: 35, 2: 15.882, 3: 24); corpus 1: maximo 3.
- Pila: `f5-frame-sizes.py` sobre el arbol final (con `e0103skip` anadido, apagado): `drive` 27.536 B, `resume_non_match` 33.080 B, pareja **60.616 B** (sin cambio, margen 5.944 B); `f5-stack65.py` 5/5 en `option_match126` y `result_match126` a 65 KiB (medido en la ronda 1 sobre 2f; los cambios posteriores no tocan `drive`).

## 6. Los 322 programas del corpus 1 que `main` y 2f fallan en core (`defectos-previos-322.json`)

Cada entrada lleva el cuerpo, la categoria, el esperado del oraculo, la salida de `main`, de 2f y de 2f con `e0103skip`. Categorias (322): 105 `NX-E0103` de mas; 99 pierden una `NX-E1303`; 25 pierden `NX-E0102`; 25 `NX-E0103` dobles de mas; 25 mismo codigo con otro primario; 14 `NX-E1303` de mas; resto 29 mixtas.

- **No son la regla del ancla** (los dos grandes). (a) `NX-E0103` de mas (105 + 25 = 130): el nodo que `unresolved_constructor_diagnostic` recibe no tiene `array_origin` (es el elemento de `r[0]`, no un literal `[]`) y el diagnostico cae sobre el span de la hoja (`found`); el oraculo solo emite `NX-E0103` sobre literales con elemento variable. Medicion con un interruptor (`e0103skip`: no emitir cuando el constructor no tiene `array_origin`): **arregla 102 de 322 (84 de 105 + 18 de 25) y 0 regresiones** sobre los 24.000 del corpus 1 (programas que 2f acierta y `e0103skip` no: 0). Es regla de cierre/finalizacion, independiente de la union. (b) pierden una `NX-E1303` (99): una hoja que es un literal de array con un error interno (`[ar,aar]`, `[q,r]` con `q`,`r` de profundidad distinta) no se contrasta con la referencia de la union en `main` ni en 2f; el oraculo la sigue denunciando con su tipo parcial. Minimo: `q=[[3]];r=[3];let a0=(if p then 1 else [q,r]);` oraculo: `NX-E0102` en `r` y `NX-E1303` sobre `[q,r]`; `main` = 2f: solo la `NX-E0102`. Tampoco es la regla del ancla (ocurre con una union de dos hojas sin variables pendientes en la union).
- Resto (25 `NX-E0102` perdidas, 25 primario distinto, 14 `NX-E1303` de mas...): no se agruparon por causa en esta ronda; quedan listados con su esperado para B.

## 7. Las 6 regresiones de relacion `expected_type` (opcional)

Agrupadas por lo que dice el oraculo (detalle en INFORME.md sec. 5 y `regresiones-A.json`):
- Grupo 1 (4 programas: 2882, 10768, 14822, 22074): el conflicto es de capa (un `NX-E0102` de la asignacion contra un Array ya fijado por la union) y el oraculo senala el operando de la union que establecio esa capa (`[q]`, `[q,r]`, la union interior, `q`); 2f senala la RHS de la asignacion (`[3]`, `[[3]]`) o la inicializacion de `ar` (`[1]` en una `let` anterior): sigue alias hasta el origen concreto.
- Grupo 2 (2 programas: 8107, 13262): el conflicto es escalar (`I64` contra `Bool`/Array) y el oraculo senala la restriccion que fijo el escalar (`[3]` de `q=[3]`; `r` dentro de `[q,r]`); 2f senala una hoja de la union (`q`, `aar`).
Regla candidata (hipotesis, NO medida): la relacion de un `NX-E0102` de asignacion es el origen de la **capa en conflicto** (la primera igualdad, en orden fuente, que establecio ese constructor o ese escalar), no el origen del tipo completo ni el de la raiz concreta. Un interruptor que marque `deferred_join` en cada contraste compatible (`defmark`, ronda 1) no la implementa y empeora (30 regresiones de relacion); la regla pide guardar el origen por capa en el nodo (hoy un solo `concrete.origin`). No se anadio ninguna variante nueva.

## 8. Limites del nuevo barrido

- 16.000 programas, una semilla, muestra aleatoria no exhaustiva; `for` y `else if` sin parentesis no se pueden probar (los perfiles/sintaxis los rechazan); sin `Option`/`Result` como valores de la union (solo como contexto `Some(U)`/`Ok(U)`), sin generadores ni closures, sin cuerpos de `match` con `Result`, sin `value` como bloque suelto.
- El oraculo del grupo D es analogo (sec. 2): un 0 en A es "2f coincide con el analogo", no "2f coincide con el ADR". La forma `matchexpr` no tiene oraculo fiable; el unico veredicto firme sobre ella es la comparacion con `main` y las 7 regresiones.
- `run` solo compara debug/release y main/2f sobre programas ya aceptados (el simulador no cambia con el verificador): no hay oraculo de `run`.
- No se ejecuto la suite de tests, `validate.sh`, los 152 RD ni `twin` (el prototipo no es integrable y tiene `eprintln!` y clones de grafo).

## 9. Incidencias de esta ronda

1. **Cerrojo `heavy.lock`**: la sesion `i07c-s6cor2-c` (campana de mutacion) lo retoma sin hueco; el `heavy-lock.py run` (un solo intento) no lo conseguia nunca. Solucion: `lockwait.py` (en `.scratch/`, importa `scripts/heavy-lock.py` y llama a su `acquire`/`release`) con reintento de 20 ms; esperas de 0 a 326 s por compilacion. No se borro ni modifico el archivo de cerrojo ni se paro ningun proceso ajeno; los unicos procesos parados fueron mis propios `build.sh` (por PID).
2. Los barridos de `check`/`run` (6 hilos de procesos cortos) no pasaron por el cerrojo (no son compilaciones ni campanas de mutacion). Se declara.
3. `cargo build` del arbol de trabajo en debug devolvio "Finished 0.00s" reutilizando el artefacto de `base-src` (mismo directorio de destino): el binario `k2-dbg` salia identico a `main-dbg`; se detecto comparando salidas y se reconstruyo con destino propio (`target-k2-dbg`).
4. Un `Edit` con Python en cp1252 dejo un byte `§` invalido en `types.rs` (error de compilacion); corregido antes de medir nada.
5. Se borraron del arbol los `res-*.json` masivos de la ronda 1 (60 MB) tras resumirlos; siguen en commits locales intermedios (no empujados).
6. Los resultados de esta ronda (`c2-check.json`, `c2-run.json`) viven en `.scratch/` (no versionados, ~70 MB); se regeneran con `run2.py check|run`.

## 10. Recomendacion para A/B

1. 2f sobrevive a las formas nuevas salvo el `match`-expresion con primer brazo union fallida: el diseno debe especificar que una union **pendiente** no marca el valor del brazo como fallido (como `main`) y que ese estado se separa del de union **denunciada** (la misma separacion que pide la sec. 7.1 del INFORME.md).
2. Tratar aparte, como reglas propias (no ancla): finalizacion de `NX-E0103` (arregla 102/322 con una linea, sin regresiones), hoja-literal con error interno que sigue contrastandose (99/322) y procedencia por capa de `expected_type` (6).
