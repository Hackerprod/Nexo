# Exploracion S5c, ronda 4: pieza RM-M3 (el `match` como expresion se comporta como `main`)

Subagente X, rama local `i07c-s5c-explor`, sin push. Inicio 2026-11-23 00:52, fin 02:15 (reloj de la maquina). Prototipo desechable; parche `candidato-4.diff` (arbol completo contra `origin/main` 1e2e393; interruptores `NEXO_XV` en el codigo, comportamiento por defecto = 2g' + RM-M3).

## 0. Resultado

- `matchpartial` (M1) se retira: aparece solo como interruptor apagado. Se anaden **tres interruptores nuevos que forman RM-M3**: `rmm3b`, `rmm3e`, `rmm3f`, y se corrige una regresion de aceptacion que esos barridos destaparon en `nojoin` (`nojoinr`).
- **0 regresiones de core frente a `main`** en TODOS los corpus: c1 (24.000), c2 (16.000, perfiles `f0-0002` y `f0-0003`), `corpus-b`, `perm`, `corpus-t`, y los tres corpus nuevos de B (semillas 7021993, 4417061, 9908255; formas planas y `blockstyle` con oraculo) y matriz 45/45 core. Sin RM-M3 reaparecen exactamente los 7 casos R-C (ablacion).
- Ejemplo «Denunciada» de B: 2g' + RM-M3 = `main` (E0103 `[64,66)`, `[73,75)` y solo E1303 `[132,133)`); ejemplo «Pendiente» (R-C): 2g' + RM-M3 = `main` (E1303 sobre `[q]`).
- **No se cumple la identidad total con `main` en el `match` como expresion**: en los 4 corpus quedan **28 programas** (de 3.201 con `matchexpr`) donde difieren los diagnosticos del SEGUNDO brazo (14 que 2g' anade y 14 que quita) y **157** donde difiere algun diagnostico de un `matchexpr` de una forma que no difiere en su analogo sin `match`; las causas estan clasificadas en la sec. 4. La aceptacion no cambia en ningun `match` (0). Por la regla de corte (3 variantes) se entrega la diferencia con su causa y se para.

## 1. Pieza RM-M3 (mecanismo)

`main` decide si el valor de un brazo de `match` entra en la union de brazos con el indicador de fallo de la expresion del brazo (`FailedJoin`, el tercer valor del retorno de `if_finish_join`). 2g' devuelve siempre `(tipo, failed=true, anchored=true)` en el retorno temprano de una union con discrepancia pendiente o absorbida, y el `match` descarta el brazo (`!failed && ...` en `match_advance`). RM-M3:
1. En `if_finish_join`, cuando la union tiene discrepancia **propia pendiente** (`anchor_pending`, sin `anchor_error`, sin `anchor_absorbed`) y su primera continuacion es un `if` anidado, registra la ocurrencia en `graph.pending_unions` (`rmm3e` + `rmm3f`).
2. En `match_advance`, un brazo marcado fallido que esta en `pending_unions` se une igualmente con el brazo siguiente (`rmm3b`); un brazo cuya union ya dio una `NX-E1303` propia (denunciada) o cuya discrepancia es de una union simple sigue sin ofrecer tipo, como en `main`.
3. `nojoinr`: `nojoin` solo omite el `unify` local de la union cuando el ancla esta **registrada** en la propia union (`union_denounced`); un ancla heredada del contexto (el brazo anterior de un `match`) no registra las hojas posteriores, y sin el `unify` local `(if p then q else r)` dejaba `q`/`r` sin resolver (aceptacion cambiada: `main` acepta, la variante daba NX-E0103).

## 2. Variantes medidas (maximo 3; las intermedias se declaran)

| variante | idea | resultado |
|---|---|---|
| V1 `rmm3` | el brazo se ofrece si es parcial (`PartialNormal`) y no hay `NX-E1303` ya registrada dentro de su span | arregla el minimo pero **2 regresiones de core** (3516, 6257: el brazo tenia E1303 interna inmediata y `main` lo ofrece igualmente) |
| V2 `rmm3b`+`rmm3e` | se ofrece si la union exterior tiene discrepancia propia pendiente (sin `anchor_error` propio) | 0 regresiones; 7 R-C arreglados; **78** diferencias en el 2.o brazo (64 anade, 14 quita): anade en uniones SIMPLES `(if p then x else var)` que `main` no ofrece |
| V3 `rmm3f` (= V2 + primera continuacion `if` anidado) | solo para uniones con primer operando anidado | 0 regresiones; **28** diferencias en el 2.o brazo (14/14); incluida en el estado final |
| (intermedias, no cuentan como pieza) | `rmm3c`: simular los indicadores de `main` por union: 661 diferencias (peor); `rmm3d`: variante de V2 sin efecto (105) | descartadas |

Referencia: con RM-M3 apagado quedan 41 diferencias del 2.o brazo (10 anade, 31 quita) y 7 regresiones de core; con `matchpartial` (M1) 1.060 (1.053 anade).

## 3. Ablacion y programas minimos

| pieza | sin ella | minimo (perfil `f0-0003-b-v1`; Σ = `fn Main(){let o:Option<I64>=Some(1);let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];`) |
|---|---|---|
| RM-M3 (`rmm3b`+`rmm3e`+`rmm3f`) | **7 regresiones de core** en c2 (1727, 3516, 6257, 7266, 8614, 11187, 14160) | Σ+`q=ar;let a0=match move o{Some(n)=>{value: (if p then (if p then 1 else s) else r);},None=>{value: [q];}};}` -> `main`: E0103 `[102,104)`, `[111,113)` y E1303 `[212,215)`; sin RM: pierde la E1303 |
| `nojoinr` | **4 cambios de aceptacion** (corpus nuevos: idx 1554, 2414 de 7021993; 2080 de 4417061; 2251 de 9908255) | Σ+`let a0=match move o{Some(n)=>{value: (if p then ar else ar);},None=>{value: (if p then q else r);}};}` -> `main` acepta; sin `nojoinr`: E0103 sobre `r` |

Control «Denunciada»: Σ'=`fn Main(){let o:Option<I64>=Some(1);let p=true;let ar=[1];var q=[];var s=[];`+`let a0=match move o{Some(n)=>{value: (if p then ar else 1);},None=>{value: true;}};}` -> `main` y 2g'+RM: solo E1303 `[132,133)` (ademas de los dos E0103); con `matchpartial` se anade E1303 `[151,155)`.

## 4. Diferencias restantes en el `match` como expresion (clasificadas)

El `match` como sentencia (`match` y `fn+match`): **0** diferencias propias en los 4 corpus. En `matchexpr`:
- **28 diferencias en el 2.o brazo**, por dos causas: (a) 14 «quita» (p. ej. c2 7888, 13377): `main` ofrece como tipo del brazo la segunda continuacion concreta (`I64`, `Array<Bool>`) y 2g' ofrece el testigo parcial de la primera, que es una variable pendiente (`s`, `r`), con lo que el brazo siguiente se une sin conflicto; ADR l.353 y D-REF fijan que la primera continuacion con evidencia valida, aun con incognitas, es la referencia (lo mismo que ya hacen `pendfa`/`retfail`), asi que es una MEJORA con cita (§4.1.2.5, l.353); no es adjudicable por oraculo (el oraculo no modela `match`). (b) 14 «anade» (p. ej. c2 7698: union de tres niveles con `else s` pendiente): `main` no ofrece el brazo y RM-M3 si (imprecision de `rmm3f`, que aproxima el indicador de `main` por «primera continuacion anidada y discrepancia propia pendiente»); es la diferencia residual que no se ha podido igualar con otra variante sin romper la igualdad en R-C.
- **157 diferencias «propias del match»** segun `matchdiff.py` (delta de codigos main→2g' en el `matchexpr` distinto del delta en su analogo `(if p then U else HOJA)`): la mayoria son `NX-E0103` quitados (`e0103rule`, l.301) y E1303/E0102 de la union interna que `main` pierde y 2g' recupera (la misma mejora que en las formas planas); no estan adjudicadas una a una. Solo las 28 anteriores afectan al segundo brazo.
- La eleccion M3 **contradice ADR l.367** (B lo anoto): un conflicto pendiente de serializacion cuenta como denunciado; RM-M3 trata distinto «pendiente» y «denunciada» solo para el consumidor `match`, como hace `main`. Queda como laguna de Flujo A (B ya la tiene).

## 5. Tablas A-D por corpus (core; frente a `main` y oraculo)

Completa en `tabla4-final.md`. Corpus con oraculo directo:

| corpus | n | sin oraculo | ok | **A** | B | C | D |
|---|---|---|---|---|---|---|---|
| c1 | 24.000 | 0 | 21.091 | **0** | 2.760 | 140 | 9 |
| corpus-b | 4.000 | 41 | 3.066 | **0** | 545 | 342 | 6 |
| perm | 4.044 | 222 | 3.379 | **0** | 342 | 101 | 0 |
| corpus-t | 10.584 | 0 | 6.056 | **0** | 4.528 | 0 | 0 |
| s7021993 plano / blockstyle | 731 / 367 | 0 | 571 / 309 | **0 / 0** | 147 / 57 | 10 / 1 | 3 / 0 |
| s4417061 plano / blockstyle | 759 / 379 | 0 | 597 / 307 | **0 / 0** | 152 / 71 | 9 / 1 | 1 / 0 |
| s9908255 plano / blockstyle | 744 / 364 | 0 | 619 / 306 | **0 / 0** | 122 / 56 | 3 / 1 | 0 / 1 |
| c2 (16 formas, `f0-0003`) | 16.000 | 191 | ver `tabla4-final.md` | **0** | | | |

(En los corpus de B solo las formas plana y `blockstyle` tienen oraculo directo; el resto —`fn`, `fnparam`, `while`, `if`, `match`, `matchexpr`, `annot`, `ret`— se miden por identidad con `main` en las secciones 3 y 4.) Matriz de 45 filas: **45/45 core** (38 full: las mismas 7 de relacion), release y debug.

## 6. `run`, pila, pasadas

- `run` (submuestra de 6.964 programas, main/2g'+RM x debug/release): debug = release en ambos; main vs 2g' difieren en 1.242 programas, **0 sin explicar** por `check`.
- Pasadas de planning: maximo **3** (0: 30, 1: 12, 2: 6.893, 3: 29).
- Pila: `drive` 27.536 B, `resume_non_match` 33.080 B, pareja **60.616 B** (sin cambio; margen 5.944 B). `f5-stack65.py`: 10/10 a 65 KiB. Nota: la cadena `drive->resume_expression_match->match_advance->selection_type` pasa de 36.440 a 36.984 B (+544 B por el cuerpo nuevo de `match_advance`); sigue lejos del limite y no toca el par vigilado.

## 7. Limites

- 157 diferencias «propias del match» sin adjudicar una a una; las 14 «anade» del 2.o brazo son una imprecision conocida de `rmm3f`. Ningun oraculo modela `match`; el analogo `(if p then U else HOJA)` no es equivalente a la semantica de `main` (293 C en c2 lo muestran).
- Corpus de B: solo las formas plana y `blockstyle` tienen oraculo; `match`/`matchexpr` se miden por identidad con `main`.
- Los resultados masivos y binarios viven en `.scratch/` y se borran al terminar; se regeneran con `eval3.py`, `matchdiff.py`, `armdiff.py`, `run3.py`.
- Sin suite de tests, `validate.sh`, RD ni `twin`.

## 8. Incidencias

1. Una primera lectura mia de «M3 = errores ya registrados en el span» (V1) daba 2 regresiones: `main` ofrece brazos con E1303 interna inmediata; la distincion real es la de la propia union exterior, no la del arbol.
2. Se anadio un `eprintln!` de depuracion a una copia de `main` (`base-src2`, no versionada) para leer los indicadores reales de `match_advance`; esa copia se borra.
3. `ablate.py`/`eval3.py` ya manejaban el perfil `f0-0003` para c2; los corpus de B se corren siempre con `f0-0003`.
4. Cerrojo `heavy.lock` con espera; no se toco ni se paro nada ajeno. Los barridos de procesos cortos no pasan por el cerrojo.
