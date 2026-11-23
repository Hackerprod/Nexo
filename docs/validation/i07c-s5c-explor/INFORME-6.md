# INFORME-6 — Ronda 6 (final): suite real, adjudicacion test a test, tabla de 103 filas, aceptacion

EXPLORACION, no integrable. Inicio 2026-11-23 02:36, fin 04:50. Arbol: `origin/main` eeb3c9c + `candidato-5.diff` (copia desechable). El candidato final es el
candidato-5 SIN cambios de codigo (`candidato-6.diff` == `candidato-5.diff`): ninguna variante medida cumplio 0 regresiones de core (sec. 3). Nada se ha
editado en tests existentes; todas las propuestas de edicion de tests quedan para aprobacion expresa UNO A UNO.

## 1. Suite real del candidato-5 (`cargo nextest run --profile default --offline --no-fail-fast`, debug)

2167 ejecutados: 2150 pasan, **17 fallan**, 72 omitidos (perfil). Log integro: `tests-candidato5.log`. Frente a los 19 de B (candidato-3): **ya no fallan** los 2 de
`ir::i07c_tests` (`adr_gap_recursive_array_unification_reports_inference_error`, `deferred_join_has_one_diagnostic_per_source_site`), arreglados por `skipv3`; los otros 17
coinciden con la lista de B y con mi clasificacion de INFORME-5 sec. 6. Salida esperada (esperado del test) / obtenida (candidato) de cada uno, del log:

| test | esperado (test) | obtenido (candidato) |
|---|---|---|
| causal_obligations::mixed_nested_join_keeps_constructor_and_shape_sites | E0103 `[]`, E1303 `[2,3]` | E1303 `[2,3]` |
| causal_obligations::adr_gap_nested_union_and_array_element_keep_current_causal_sites | E0103 `[]`, E1303 `br` | E1303 `br` |
| causal_obligations::failed_outer_condition_keeps_nested_join_constructor | E0102 `1`, E0103 `[]`, E0102 `x`, E1303 `br` | sin el E0103 |
| causal_obligations::failed_outer_array_keeps_nested_element_cause | 3 diagnosticos (E0103 `[]`, E0103 `[t2]`, E0102 `[1]`) | 2 (sin E0103 `[t2]`) |
| causal_obligations::pending_inner_constructor_preserves_outer_branch_comparison | 2 (E0103 `[]`, E1303 `false` I64/Bool) | 1 (solo E0103) |
| causal_obligations::unresolved_constructor_survives_failed_nested_union_condition | E0103 `[]`, E0102 `1`, E1303 `[5]` | E0102 `1`, E1303 `true` |
| mutation_regressions::anchored_first_branch_keeps_independent_pending_join | 3 (E1303 `[63,139)`, E1303 `[134,137)`, E0102 `[136,137)`) | 1 (solo E0102) |
| mutation_regressions::linked_array_constructors_keep_each_causal_inference_site | E0103 `[80,82)`, `[130,137)` | solo `[80,82)` |
| mutation_regressions::cyclic_constructor_errors_keep_inference_code_at_each_site | E0103 `[17,19)`, `[36,46)`, `[43,45)` | sin `[43,45)` |
| pending_nominal_domain::incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin | E0103 x2, E0102 `q`, E0102 `x` | E0102 `[[r],q]`, E0102 `3`, E0102 `x`, E1303 `q` |
| pending_nominal_domain::planned_projection_child_does_not_use_failed_branch_as_witness | E1303 en `q` `[169,170)` | E0102 en `q` (mismo expected/found) |
| pending_nominal_domain::failed_nested_union_does_not_anchor_an_outer_pending_array | solo E1303 `[[q]]` | E1303 `[[1]]` + E0102 `3` |
| normative_survivors::independent_causal_sites_survive_deferred_resolution | (caso `nested_constructor`) E0103 `[28,30)`, `[73,77)` | solo `[28,30)` |
| normative_survivors::deferred_projection_and_constructor_programs_finish_with_causal_results | (caso `repeated_constructor_sites`) E0103 `[17,19)`,`[27,37)`,`[34,36)` | sin `[34,36)` |
| structural_invariants::grouped_operands_keep_one_causal_primary | auditor de lineas OK | `reviewed span changed at types.rs:7078` (numeros de linea fijados) |
| quota_tests::deferred_if_join_charges_the_reduced_boundary | NX-E0003 con limite 1..=8 | con limite 8 termina sin error (el candidato necesita <= 7 cargas) |
| quota_tests::failed_branch_join_keeps_reduced_quota_error_prefix | sin NX-E0006 `[176,213)` con limite 30/31 | NX-E0006 `[176,213)` presente con limite 30 (ver limites) |

## 2. `anchored_first_branch…` (BLOQUEANTE segun el coordinador): adjudicacion y 3 variantes medidas

Programa (completo): `fn Main(){var r=[];var s=[];let q=(if true then{value:s;}else (if true then{value:(if true then{value:r;}else{value:[];});}else{value:s<1;}));r=[[1]];s=[[]];}`
(`f0-0002-b-v2`). `main`: E1303 `[63,139)` Array<Array<I64>>/Bool, E1303 `[134,137)` idem, E0102 `[136,137)`. Candidato: solo E0102 `[136,137)`. Oraculo: E1303 `[134,137)` + E0102.

Causa: la hoja `s<1` tiene un operando fallido (E0102 en `1`); el candidato (`nojoin`) no la contrasta con el ancla (`second_leaf_anchor` sale con `failed`). Solo `nojoin`
cambia el resultado; con todas las piezas apagadas se obtiene `main`.

Variantes (3, todas medidas con CLI; la 3 ademas en los corpus):
1. `failleaf2` (contrastar la hoja fallida con el tipo de `expression_types`): sin efecto (la expresion fallida no registra tipo). Resultado = candidato.
2. `nojoin2`/`nojoin3` (no aplicar `nojoin` cuando la segunda hoja fallo): produce solo `E0103 [25,27)` (peor; reaparece el efecto de quitar `nojoin`). Descartada.
3. `failleaf3` (sintetizar `Bool` para una hoja fallida que es comparacion, y contrastarla): restituye E1303 `[134,137)`. **Cambia programas concretos donde `main` y candidato
   coinciden y el ADR (T5) fija lo contrario**: `fn Main(){let q=(if true then 1 else (true<1));}` y `fn Main(){let s=[[1]];let q=(if true then s else (if true then s else s<1));}`:
   `main` = candidato = solo el E0102 del operando; con `failleaf3` se anade una E1303 sobre la hoja. Medida en los corpus: 0 diferencias frente al candidato-5 (los corpus no ejercitan
   la forma), 0 regresiones de core; la divergencia solo se ve con programas dirigidos.

Adjudicacion por el ADR (cita exacta, `docs/adr/F0-0002-lagunas.md`, seccion «Resultado de una operacion con un operando fallido o contaminado», T5): «El resultado de `+`, `-`, `<`, `==`, `AssertEq`
… con un operando que ha fallado una obligacion de la operacion … esta contaminado: carece de tipo nominal y de evidencia valida para cualquier consumidor, tambien si el consumidor es … una hoja de `if` …
Un elemento o una continuacion asi no establece la referencia de homogeneidad ni la de la union, no es incompatible con ella y no recibe NX-E1303 ni NX-E0102 por ese resultado». Y
`F0-0002-second-slice.md` 4.1.2.5 (l.367): «El testigo parcial … procede de una continuacion valida independiente, no del tipo o valor de la expresion fallida … No se usa el supuesto resultado Bool
de esa operacion invalida para … reemplazar el C».
Lectura: `s<1` esta contaminada (su `1` incumple la obligacion de `<`), luego **no recibe E1303**; el candidato es coherente con T5 y con `main` en el analogo concreto. El E1303 `[134,137)` del
oraculo y del test procede de una lectura previa a T5 (el oraculo da tambien E1303 en los analogos concretos donde `main` no la da). **No hay variante que lo restituya sin romper T5**; la pieza que lo causa
es `nojoin`, y retirarla no lo arregla (variante 2). Decision pendiente del coordinador: (a) aceptar la lectura T5 y aprobar reescribir el test (esperado: solo E0102 `[136,137)`), o (b) declarar T5
no aplicable a hojas con dominio pendiente (Flujo A) y entonces retirar `nojoin`+reponer R-A/R-B por otro mecanismo. Yo no lo fijo.

## 3. Variantes medidas para `planned_projection…`/`incompatible_prefix…`

`planned_projection`: el candidato cambia el CODIGO en `q` `[169,170)` (E1303 -> E0102) y anade E0102 en `q` `[183,184)` y `s` `[190,191)` cuando el array tiene un primer elemento `s[x]`
y dos uniones (programa minimo de 3 elementos: `fn Main(){let x=1;var q=[];var s=[];let a=[s[x],(if 0 then true else q),(if x then q else s)];q=[true];s=[[true]];}`). Oraculo: E1303 en `q`
(coincide con `main`), pero el oraculo difiere de ambos en el resto. Pieza causante: `atomic0` (sin ella = `main`).
Variantes: `atomic0e` (restaurar solo con error) devuelve `main` aqui y en `incompatible_prefix`, pero mide **56 regresiones de core** (c1 4, s7021993 1, s9908255 1, t 50) y empeora `failed_nested`
(E0103 nuevos): RECHAZADA. `atomic0p`: sin efecto. Combinacion `atomic0e,atomic0p`: medida, tambien 56 regresiones de core (mismos indices): RECHAZADA.

Ninguna de las variantes (`atomic0e`, `atomic0p`, `atomic0e+atomic0p`) conserva 0 regresiones; `planned_projection` queda como defecto abierto del candidato (causa `atomic0`), no corregido.

## 4. Adjudicacion de los tests restantes (con cita)

Citas comunes (`F0-0002-second-slice.md`, 4.1.2.2 l.301): (C1) «La obligacion de resolver el elemento de un constructor Array se comprueba siempre al cierre de la funcion … si su elemento es una variable
sin constructor que ninguna restriccion compatible resuelve, emite NX-E0103 sobre ese constructor completo»; (C1b) «un diagnostico por constructor con obligacion propia de resolucion, aunque varios constructores
compartan la incognita por unificacion; no uno por componente compartido ni otro por cada lectura o uso»; (C1c) «Las igualdades compatibles se acumulan … Si ninguna restriccion compatible lo completa, … emite NX-E0103».
(C2) 4.1.2.4 l.353: «Las restricciones compatibles de todos los hijos se conservan siempre, se emitan o no D.»

### 4.1 Los 8 que fijan comportamiento erroneo de `main` (tabla; ningun test editado)

| test (fichero) | que fija | programa completo | `main` | candidato | cita / oraculo | propuesta |
|---|---|---|---|---|---|---|
| mixed_nested_join_keeps_constructor_and_shape_sites (tests/i07c_causal_obligations.rs) | E0103 en `[]` pese a la union fallida | `fn Main(){let b=false;let o=(if b then [[]] else (if b then [[false],[true]] else [2,3]));}` | E0103 `[40,42)`; E1303 `[82,87)` | solo E1303 `[82,87)` | `[[false],[true]]` es compatible con la referencia `[[]]` y fija el elemento (C1c, C2); el E0103 solo se emite si ninguna restriccion compatible lo completa. Oraculo: solo E1303 `[82,87)` Array<Array<Bool>>/Array<I64> | reescribir esperado: solo E1303 `[2,3]` |
| adr_gap_nested_union_and_array_element_keep_current_causal_sites (causal_obligations) | idem (nombre `adr_gap_*`) | `fn Main(){let c=true;let br=1;let m0=(if c then [] else (if c then [1] else br));}` | E0103 `[48,50)`; E1303 `[76,78)` | solo E1303 `[76,78)` | `[1]` completa el elemento de `[]` (C1c, C2). Oraculo: solo E1303 `[76,78)` Array<I64>/I64 | reescribir: solo E1303 `br` (la parte del array no cambia) |
| failed_outer_condition_keeps_nested_join_constructor (causal_obligations) | E0103 `[]` pese a condicion fallida | `fn Main(){let x=1;let br=[true];var m0=(if 1 then [] else (if x then [1] else br))[1];}` | E0102 `1`, E0103 `[50,52)`, E0102 `x`, E1303 `br` | sin el E0103 | idem (C1c, C2). Oraculo: E0102 `[43,44)`, E0102 `[62,63)`, E1303 `[78,80)` | reescribir: quitar el E0103 del vector |
| failed_outer_array_keeps_nested_element_cause (causal_obligations) | E0103 en `[t2]` | `fn Main(){let x=1;var t2=[];t2[1]=[t2[x],[t2],[x,[1]]];}` | E0103 `[25,27)`, `[41,45)`, E0102 `[49,52)` | sin `[41,45)` | `[t2]` tiene elemento `t2`, que ya tiene constructor Array: no es «variable sin constructor» (C1); la incognita compartida ya la denuncia `[]` (C1b). Oraculo: no modela (`t2[1]=`) | reescribir: 2 diagnosticos |
| linked_array_constructors_keep_each_causal_inference_site (mutation_regressions) | E0103 en `[k0,k0]` | caso 2: `fn Main(){var k0=[[]];var k1=[(if false then {return;} else {return;})];var k2=[[]];k0=[k2];AssertEq(k1,k2);let w2=[k1,k0];let w5=[k0,k0];}` | E0103 `[80,82)`, `[130,137)` | solo `[80,82)` | `[k0,k0]` tiene por elemento k0 (con constructor): C1, C1b. Oraculo (parcial): E0103 `[80,82)` | reescribir caso 2: `[(80,82)]` |
| cyclic_constructor_errors_keep_inference_code_at_each_site (mutation_regressions) | E0103 en una lectura | `fn Main(){var m1=[];AssertEq([true],[m1[7],m1]);}` | E0103 `[17,19)`, `[36,46)`, `[43,45)` | sin `[43,45)` | `[43,45)` es la lectura `m1`, no un constructor: «ni otro por cada lectura o uso» (C1b). Oraculo: E0103 `[17,19)` | reescribir: dos sitios |
| independent_causal_sites_survive_deferred_resolution, caso `nested_constructor` (normative_survivors) | E0103 en `[q0]` | `fn Main(){var q0=[];var q1=[[]];let z=(if true then q1 else q0);AssertEq([q0],[q0,true]);}` | E0103 `[28,30)`, `[73,77)` | solo `[28,30)` | `[q0]` con elemento q0 (con constructor) (C1, C1b). Oraculo: solo `[28,30)` | reescribir el caso |
| deferred_projection_and_constructor_programs_finish_with_causal_results, caso `repeated_constructor_sites` (normative_survivors) | E0103 en la segunda lectura | `fn Main(){let q1=[];let a2=[q1[0],q1];}` | E0103 `[17,19)`, `[27,37)`, `[34,36)` | sin `[34,36)` | `[34,36)` es la lectura `q1` (C1b). Oraculo: `[17,19)`, `[27,37)` | reescribir el caso |

### 4.2 Con cita del ADR en contra del candidato o no modelados — ADJUDICACION

* `unresolved_constructor_survives_failed_nested_union_condition` (N(d)). Programa: `fn Main(){var t0=[];(if true then{value:(if true then{value:(if 1 then{}else t0);}else(if true then true else true));}else([5]));}`.
  `main`: E0103 `t0` `[17,19)`, E0102 `1`, E1303 `[5]` (Bool/Array<I64>). Candidato: E0102 `1`, E1303 `true` `[100,104)` (Array<I64>/Bool), sin E0103. Lo pierde el test: el E0103 de `[]` y mueve la E1303.
  Pieza: `retfail` (sin ella = `main`). Con condicion valida (`if true`) la diferencia es la misma, luego no depende de la condicion fallida.
  Cita N(d) (`second-slice.md` clausula (d), l.283): «Un sitio independiente prueba su incompatibilidad sin usar un resultado ya denunciado o contaminado …»; l.337: «Un constructor Array independiente … cuyo elemento siga irresuelto conserva su NX-E0103».
  Derivacion: la primera continuacion de la union interior es `t0`, de tipo Array<α> (raiz constructor conocido, l.353: «un contexto esta determinado … cuando su raiz es un escalar o constructor conocido»), que fija la referencia; `true` es la
  primera posterior incompatible: una E1303 en `true` con origen `t0`; `[5]` es compatible con esa referencia y fija α=I64, luego `[]` queda resuelto (C1c) y no hay E0103. El candidato coincide con esta derivacion; `main` usa `true` como referencia. **Lectura
  del candidato respaldada por l.353; el test fija la de `main`.** Es una decision de lectura (sin oraculo: no parsea `{}`); la dejo para el coordinador, junto a `pending_inner`. Propuesta si se aprueba: esperado E0102 `1`, E1303 `true`.
* `incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin`. Programa `fn Main(){let x=1;let p=true;var r=[];var q=[];while [[r],q] {}q=[3];let a=(1<(if x then p else q));r=[3];}`; `main` E0103 x2, E0102 `q` `[58,59)`, E0102 `x`;
  candidato E0102 `[[r],q]` `[53,60)` Bool/Array<Array<Array<I64>>>, E0102 `3` `[66,67)`, E0102 `x`, E1303 `q` `[96,97)`. Pieza `atomic0`. Derivacion (a mano; el oraculo no parsea `while`): por orden fuente, `[[r],q]` precede a `q=[3]`; la primera
  igualdad compatible que determina q es la del literal (q: Array<Array<α>>); `q=[3]` es la posterior incompatible y se rechaza en `3` (l.301: «Nunca se usa una asignacion rechazada para cambiar el tipo de lecturas anteriores»); `r=[3]` completa α=I64, luego no hay E0103 (C1c);
  la condicion `while` exige Bool y el literal es Array: E0102 en el literal. El candidato coincide. La propia pareja concreta del test (segundo programa) da lo mismo en ambos. INCONCLUSO-hacia-candidato: sin oraculo; propongo reescribir con el esperado del candidato si el coordinador acepta la derivacion.
* `planned_projection_child_does_not_use_failed_branch_as_witness`: **el ADR/oraculo apoya a `main` en el codigo de `q`** (oraculo: E1303 `[69,70)` en el programa minimo; candidato E0102). Defecto del candidato en una familia estrecha (array con primer elemento pendiente `s[x]` y dos uniones). Reparacion medida: ver sec. 3 (`atomic0e` falla con 56 regresiones). **Defecto abierto del candidato**, registrado con causa (`atomic0`).
* `failed_nested_union_does_not_anchor_an_outer_pending_array`. Programa `fn Main(){var r=[];var q=[];let a=(if true then r else (if false then [[q]] else [[1]]));r=[3];q=[3];}`; `main` E1303 `[[q]]` `[70,75)`; candidato E1303 `[[1]]` `[81,86)` + E0102 `3` `[92,93)`. **Oraculo = candidato** exactamente
  (E1303 `[81,86)` Array<Array<Array<I64>>>/Array<Array<I64>>; E0102 `[92,93)`). Cita: l.353 «la primera continuacion con evidencia valida establece la referencia … Si esa union interna discrepa, su primera continuacion valida aporta obligatoriamente el testigo parcial». Propuesta: reescribir con ese esperado.
  El analogo concreto (sin `var`) coincide en ambos.
* `pending_inner_constructor_preserves_outer_branch_comparison` (lecturas en conflicto, para Flujo A). Programa `fn Main(){let ar=[1];let c=true;var m1=(if true then {value:ar[0];} else (if c then [[]] else false));}`; `main`: E0103 `[85,87)` + E1303 `false` `[94,99)` I64/Bool; candidato y oraculo: solo E0103 `[85,87)`.
  Lectura A (test/`main`): el literal `[[]]` con constructor irresuelto no ofrece tipo a la union; `false` es la primera continuacion posterior incompatible con `ar[0]` (I64): E1303 en `false`. Fundamento: la seccion de lagunas «los demas defectos … se contrastan entre si como si el literal no estuviera».
  Lectura B (candidato/oraculo): `[[]]` es incompatible con I64 (Array frente a escalar), pero su serializacion esta pendiente (variable sin resolver) y «Un conflicto pendiente de serializacion cuenta como ya denunciado» (l.367, 13.9): la union ya esta denunciada y `false` no recibe otra E1303 (una por union, l.353).
  Efecto: A emite E0103 + E1303(`false`); B emite solo E0103. Aceptacion igual (ambas rechazan). A Flujo A.

### 4.3 Andamiaje (via rapida; tabla aparte)

| test | fichero | por que falla | propuesta |
|---|---|---|---|
| quota_tests::deferred_if_join_charges_the_reduced_boundary | src/check/types.rs | cifra de cuotas: el candidato deniega con limites 1..=7 (medido) y termina con 8; el ADR dice «los umbrales reducidos exactos pertenecen a I10a» | limite superior 7 (o calculado) |
| quota_tests::failed_branch_join_keeps_reduced_quota_error_prefix | src/check/types.rs | con limite 30 el candidato produce NX-E0006 `[176,213)` y deniega NX-E0003 hasta 37 (medido, `main` hasta menos) | revisar: ver limites |
| structural_invariants::grouped_operands_keep_one_causal_primary | tests/i07c_structural_invariants.rs + scripts/i07c_span_audit.py | el auditor fija numeros de linea de `types.rs`; el codigo anadido los desplaza | actualizar los numeros fijados |

## 5. Tabla de conservacion extendida: 103 filas (51 + 3 + 28 + 21) y las 8 no adjudicables

`match-conservacion-ext.py` (`--regenerar MAIN CAND`, `--check EXE [--main] [--tabla]`) cubre 103 filas: `conservacion` 51, `diferencias_declaradas_de_las_54` 3, `diferencias_declaradas` 28 y
`diferencias_declaradas_rmm3_fuera_2o_brazo` 21 (fuente: `match-conservacion-ext-decl21.json`, las 21 con veredicto «regresion» de `adjudicacion-157-completa.json`, clase `anadido_rmm3_fuera_2o_brazo`, con la salida de `main` y la del candidato en cada fila).
Las 8 no adjudicables van aparte en `match-conservacion-ext-noadj.json` (fuentes) y `match-conservacion-ext-noadj-salidas.json` (salida de `main` y del candidato; ni mejora ni regresion); `--check` tambien las verifica.
SHA-256 de `match-conservacion-ext.json`: `c03c2cfb16b2a205af10ed9cf132a26773b425f04eb6259669d00abbd43669c2`. `--check` con el candidato: 103 filas, sin diferencias; `--check --main` con main: idem.
`match-conservacion-ext-mut.py`: 7 mutaciones (fila de menos, de mas, outcome, diagnostico anadido, perfil, fila declarada, fila RM-M3) fallan con exit 1 y el control pasa (`match-conservacion-ext-mut.out`).

## 6. Corpus (candidato-5 frente a main, `eval3.py`, nivel core)

Regresiones de core (A) = **0** en c1 (24.000), c2, corpus-b (4.000), perm (4.044), corpus-t (10.584) y las 3 semillas de B. Cifras A/B/C/D por corpus en la salida de `eval3.py table main c5`. `failleaf3`: tablas identicas al candidato-5
(los corpus no ejercitan la forma; ver sec. 2). `atomic0e`: 56 regresiones (rechazada).

## 7. Aceptacion (`acept6.py`, salida `acept6.out`)

`check` y `run`, 3 perfiles (`f0-0002-b-v2`, `f0-0003-b-v1`, `f0-0005-b-v1`), binarios release de `main` y candidato-5 (con `skipv3`): familia dirigida `var`+`if`/`return`/`break` (6.000 programas, seed 31415926) y las 3 semillas de B (4.500 cada una):
**0 cambios de aceptacion en las 24 combinaciones** (`check`/`run` x 3 perfiles x 4 conjuntos), 0 errores de ejecucion. Aceptan (main = candidato): dirigida 299/408/408; B-7021993 180/259/259; B-4417061 204/290/290; B-9908255 187/266/266.
Diferencial debug/release de `run` (familia dirigida, `f0-0003-b-v1`), para `main` y para el candidato: **0 divergencias** (6.000 programas cada uno). El negativo de `skip` (`fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=a;}`): `main` y candidato rechazan igual
(E0103 `[66,67)`) en `check` y `run` en los 3 perfiles. Los negativos de B y de la familia dirigida quedan como los programas de `corpus-dirigido5.json` y de los corpus de B; no se anadieron tests (exploracion).

## 8. Limites

* No medido: la matriz de 45 y el apilado (`f5-stack65`) no se repitieron en la ronda 6 (el codigo no cambio respecto al candidato-5; valen las cifras de INFORME-5).
* Diferencial debug/release solo de `run` sobre la familia dirigida en `f0-0003-b-v1` (no en los otros perfiles ni en las semillas de B).
* `anchored_first_branch…` NO corregido: es una decision de lectura de T5 (sec. 2), no un defecto medible con otra pieza; a decidir por el coordinador.
* `planned_projection…` defecto abierto (familia estrecha, sec. 3); `incompatible_prefix…` y `unresolved_constructor…` apoyados por derivacion a mano (el oraculo no parsea `while` ni `{}`).
* Los limites de cuota (`failed_branch…`): medido que con limite 30 aparece un NX-E0006 `[176,213)` que el test prohibe; no investigue si es solo desplazamiento del umbral o un defecto de `skipv3`/`nojoin` (andamiaje segun el ADR, no bloquea por la regla de rigor).
* Tabla de 103 filas: las 21 se declaran como diferencias sin cita (main = oraculo en U); las 8 no adjudicables sin veredicto.
* La suite es debug; no se ejecuto en release.
