# I07c-S5c - oraculo independiente frente a la matriz (E, Flujo B)

Oraculo: `scripts/i07c_s5c_oraculo.py` (derivado solo del ADR; no lee ni importa el verificador). Fuentes: `docs/validation/i07c-s5c/fuentes-recuperadas.json`, `docs/validation/i07c-s5a/{d-repros,limites}/`, `docs/validation/i07c-twin/casos.json`. Esperados de la matriz: columna «Esperado» de `docs/correcciones/I07c-S5c-diseno-ancla.md` (copiados de `rec_all.py` a `docs/validation/i07c-s5c/oraculo/matriz-esperados.json` por `extraer_esperados.py`, sin ejecutar binarios). Comparacion: cadena exacta por diagnostico (codigo, subcausa, primario por bytes, `expected_type`/`found_type`, relacion `expected_type`), como multiconjunto (la matriz lista sus diagnosticos por orden de cadena, no por 13.9; el oraculo emite siempre por 13.9: inicio, fin, codigo, subcausa).

**Resultado: 42/45 filas coinciden; 3 discrepan (W0, P2, w3).** Filas nuevas del plan 5 y observacion de 8 bis: 5/5 coinciden en primario y relacion.

## Discrepancias (3 filas) y quien tiene razon

Criterio: el ADR manda; no se fuerza ninguna fila. Los tres casos se verificaron contando bytes de la fuente y comprobando
la coherencia interna de los tipos de la propia fila de la matriz.

### W0 (relacion de la E1303: matriz `[60,64)`, oraculo `[59,63)`) - tiene razon el oraculo (error aritmetico de la matriz)

`fn Main(){let ar=[1];var r=[];var q=[];let a=(if true then true else q);...`: el prefijo `fn Main(){` ocupa 10 bytes,
`let ar=[1];` 11, `var r=[];` 9 y `var q=[];` 9, es decir 39; `let a=(` ocupa `[39,46)`, la condicion `true` `[49,53)`, la primera
continuacion `true` `[59,63)` y `q` `[69,70)`. La relacion `expected_type` es el operando de la primera continuacion
(ADR 4.1.2 clausula 6 / l.371: «para una union, el operando de la primera continuacion que la establece»), esto es `[59,63)`.
La matriz copio `[60,64)` de P1/c3/w1, que llevan un parentesis extra delante (`let a=((if ...`). Los binarios
`main` y V6 tambien imprimen `[59,63)`; el resto de la fila (E1303 `[69,70)` `Bool`/`Array<Array<Bool>>`, sin E0103) coincide.
Accion: corregir la columna esperada de W0 a `exp=[59,63)`.

### P2 y w3 (E1303 y E0102 sobre `r=[r]`) - tiene razon el oraculo; la matriz es internamente contradictoria

Fuente P2: `let a=(if p then [r,q] else (if (x<1) then r else [true]));q=[[3]];q=ar;r=[r];`. Derivacion:
1. `[r,q]` (primera continuacion): referencia = `r` (primer elemento con evidencia valida, 4.1.2 cl.4); la igualdad de `q` con ella se
   acumula (`alfa_r = alfa_q`); tipo del literal `Array<Array<alfa>>`. Ancla de la union = `[r,q]` `[75,80)`.
2. Aplanado en orden fuente: hoja `r` (`[101,102)`), luego `[true]` (`[108,114)`).
3. Con `q=[[3]]` (RHS completo fija `alfa_q := Array<I64>`) el tipo de `r` es `Array<Array<I64>>` y el ancla `Array<Array<Array<I64>>>`
   (la columna esperada de la matriz usa esas mismas cifras, `A^3`). La hoja `r` es incompatible con el ancla (`A^2` frente a `A^3`;
   ademas, antes de conocer `alfa`, `Array<alfa>` frente a `Array<Array<alfa>>` exige `alfa = Array<alfa>`: igualdad recursiva sin solucion
   finita, incompatible por 4.1.2 cl.2 y el addendum «no autoriza la igualdad recursiva»). Es la **primera** hoja posterior incompatible:
   la unica E1303 va sobre `r` `[101,102)` (4.1.2 cl.4, 4.7), con `expected_type=Array<Array<Array<I64>>>`, `found_type=Array<Array<I64>>`
   (tipo completo de la expresion senalada, 13.17/cl.6) y relacion `[75,80)`. `[true]` queda absorbida: no es sitio independiente ni D (cl.4).
4. `q=ar`: E0102 `[127,129)` esperado `Array<Array<I64>>` encontrado `Array<I64>`, relacion = RHS de `q=[[3]]` `[119,124)` (coincide con la matriz).
5. `r=[r]`: el elemento hoja `r` `[133,134)` recibe el tipo de elemento de `r`, `Array<I64>`, y su tipo es `Array<Array<I64>>`:
   E0102 `expected_type=Array<I64>`, `found_type=Array<Array<I64>>`; el origen de ese esperado es la igualdad posterior `q=[[3]]`
   (`[119,124)`), no el ancla.

La matriz (y `main`/V6, de cuya salida se copio; comprobado ejecutando `check` solo para este informe) dice E1303 sobre `[true]` `[108,114)` y
E0102 `[133,134)` con `Array<Array<I64>>`/`Array<Array<Array<I64>>>` y relacion `[75,80)`. Contradiccion interna comprobable sin el ADR: la E1303 de
la propia fila fija el ancla `A^3 = Array<tipo de r>`, es decir `tipo(r) = A^2`; la E0102 de la propia fila exige `found_type = A^3` para la
expresion `r`, es decir `tipo(r) = A^3`. Ambas no pueden valer. Ademas, si `r` fuese compatible con el ancla (`A^2 = A^3`) no habria
solucion finita. w3 es la misma estructura (cambia `q=[[x+1]]`). Cardinalidad (3 diagnosticos) y la E0102 de `q=ar` coinciden; difieren el
primario de la E1303 (`r` frente a `[true]`) y los tipos/relacion de la E0102 de `r=[r]`.
Accion: P2/w3 dejan de ser referencia de N para esos dos diagnosticos; se necesita adjudicacion de B (si B conviene con esta derivacion,
`main` y V6 tienen un defecto previo de nucleo: E1303 sobre una hoja distinta de la primera incompatible). No decidido aqui.


## Tabla fila a fila (45 filas)

Formato de cada diagnostico: `codigo[/subcausa] [inicio,fin) esperado|encontrado exp=[relacion)`; `A<..>` abrevia `Array<..>`.

| # | Fila | Esperado de la matriz | Oraculo | Coincide | Notas |
|---|---|---|---|---|---|
| 1 | R1 | `E1303 [91,92) A<A<A<I64>>>|I64 exp=[73,74)` | `E1303 [91,92) A<A<A<I64>>>|I64 exp=[73,74)` | si |  |
| 2 | R1twin | `E1303 [95,96) A<A<A<I64>>>|I64 exp=[77,78)` | `E1303 [95,96) A<A<A<I64>>>|I64 exp=[77,78)` | si |  |
| 3 | R3c | `E1303 [82,83) A<A<A<I64>>>|I64 exp=[64,65)` | `E1303 [82,83) A<A<A<I64>>>|I64 exp=[64,65)` | si |  |
| 4 | R2 | `E1303 [93,94) A<A<A<I64>>>|I64 exp=[68,69)` | `E1303 [93,94) A<A<A<I64>>>|I64 exp=[68,69)` | si |  |
| 5 | R2b | `E1303 [98,99) A<A<A<I64>>>|I64 exp=[73,74)` | `E1303 [98,99) A<A<A<I64>>>|I64 exp=[73,74)` | si |  |
| 6 | R3 | `E1303 [99,104) I64|A<A<A<A<I64>>>> exp=[81,82)` | `E1303 [99,104) I64|A<A<A<A<I64>>>> exp=[81,82)` | si |  |
| 7 | R3b | `E1303 [77,78) A<A<A<I64>>>|I64 exp=[59,60)` | `E1303 [77,78) A<A<A<I64>>>|I64 exp=[59,60)` | si |  |
| 8 | R4 | `E1303 [94,95) A<A<A<I64>>>|I64 exp=[76,77)` | `E1303 [94,95) A<A<A<I64>>>|I64 exp=[76,77)` | si |  |
| 9 | X_R1na | `E1303 [91,92) A<A<A<I64>>>|I64 exp=[73,74)` | `E1303 [91,92) A<A<A<I64>>>|I64 exp=[73,74)` | si |  |
| 10 | X_R1ord | `E1303 [98,99) A<A<A<I64>>>|I64 exp=[73,74)` | `E1303 [98,99) A<A<A<I64>>>|I64 exp=[73,74)` | si |  |
| 11 | X_flat3 | `E1303 [77,78) A<A<A<I64>>>|I64 exp=[59,60)` | `E1303 [77,78) A<A<A<I64>>>|I64 exp=[59,60)` | si |  |
| 12 | Y_pend_if | `E0103 [38,40) | ` | `E0103 [38,40) | ` | si |  |
| 13 | Y_pend_if2 | `E0103 [38,40) | ` | `E0103 [38,40) | ` | si |  |
| 14 | X_mix3 | `E1303 [104,105) A<A<A<I64>>>|I64 exp=[68,69)` | `E1303 [104,105) A<A<A<I64>>>|I64 exp=[68,69)` | si |  |
| 15 | X_R4asg_r | `E1303 [86,87) A<A<A<I64>>>|I64 exp=[68,69)` | `E1303 [86,87) A<A<A<I64>>>|I64 exp=[68,69)` | si |  |
| 16 | X_R1inc | `E0102/type [105,106) A<A<I64>>|I64 exp=[98,99)`<br>`E1303 [91,92) A<A<A<I64>>>|I64 exp=[73,74)` | `E1303 [91,92) A<A<A<I64>>>|I64 exp=[73,74)`<br>`E0102/type [105,106) A<A<I64>>|I64 exp=[98,99)` | si |  |
| 17 | X_R1pre | `E1303 [100,101) A<A<A<I64>>>|I64 exp=[82,83)` | `E1303 [100,101) A<A<A<I64>>>|I64 exp=[82,83)` | si |  |
| 18 | X_R2na | `E0103 [38,40) | `<br>`E0103 [47,49) | ` | `E0103 [38,40) | `<br>`E0103 [47,49) | ` | si |  |
| 19 | X_R4na | `E0103 [38,40) | `<br>`E0103 [47,49) | ` | `E0103 [38,40) | `<br>`E0103 [47,49) | ` | si |  |
| 20 | X_nest_first | `E1303 [77,78) A<A<A<I64>>>|I64 exp=[70,71)`<br>`E1303 [85,86) A<A<A<I64>>>|I64 exp=[60,78)` | `E1303 [77,78) A<A<A<I64>>>|I64 exp=[70,71)`<br>`E1303 [85,86) A<A<A<I64>>>|I64 exp=[60,78)` | si |  |
| 21 | Y_conc_if | `E1303 [67,68) A<I64>|I64 exp=[49,50)` | `E1303 [67,68) A<I64>|I64 exp=[49,50)` | si |  |
| 22 | Y_pend_if_asg | `E1303 [66,67) A<I64>|I64 exp=[48,49)` | `E1303 [66,67) A<I64>|I64 exp=[48,49)` | si |  |
| 23 | W0nounion | (sin diagnosticos) | (sin diagnosticos) | si |  |
| 24 | W0 | `E1303 [69,70) Bool|A<A<Bool>> exp=[60,64)` | `E1303 [69,70) Bool|A<A<Bool>> exp=[59,63)` | **no** | **DISCREPA**: ver seccion de discrepancias |
| 25 | A | `E0102/type [92,93) A<I64>|I64 exp=[74,77)`<br>`E1303 [83,86) A<A<I64>>|A<A<A<I64>>> exp=[53,54)` | `E1303 [83,86) A<A<I64>>|A<A<A<I64>>> exp=[53,54)`<br>`E0102/type [92,93) A<I64>|I64 exp=[74,77)` | si |  |
| 26 | A_reord | `E0102/type [92,93) A<I64>|I64 exp=[83,86)`<br>`E1303 [74,77) A<A<I64>>|A<A<A<I64>>> exp=[53,54)` | `E1303 [74,77) A<A<I64>>|A<A<A<I64>>> exp=[53,54)`<br>`E0102/type [92,93) A<I64>|I64 exp=[83,86)` | si |  |
| 27 | A_ctrl | `E1303 [83,86) A<A<I64>>|A<A<A<I64>>> exp=[53,54)` | `E1303 [83,86) A<A<I64>>|A<A<A<I64>>> exp=[53,54)` | si |  |
| 28 | A_noasg | `E1303 [83,86) A<A<I64>>|A<A<A<I64>>> exp=[53,54)` | `E1303 [83,86) A<A<I64>>|A<A<A<I64>>> exp=[53,54)` | si |  |
| 29 | B_qr | `E0102/type [83,84) A<I64>|I64 exp=[76,77)`<br>`E1303 [63,64) Bool|A<A<I64>> exp=[56,57)` | `E1303 [63,64) Bool|A<A<I64>> exp=[56,57)`<br>`E0102/type [83,84) A<I64>|I64 exp=[76,77)` | si | decision no fijada T1 (ver `orden-no-fijado.md`) |
| 30 | B_rq | `E0102/type [89,90) A<I64>|I64 exp=[76,77)`<br>`E1303 [63,64) Bool|A<A<I64>> exp=[56,57)` | `E1303 [63,64) Bool|A<A<I64>> exp=[56,57)`<br>`E0102/type [89,90) A<I64>|I64 exp=[76,77)` | si | decision no fijada T1 (ver `orden-no-fijado.md`) |
| 31 | F06 | `E0102/type [91,92) A<I64>|I64 exp=[83,85)`<br>`E1303 [76,77) A<I64>|A<A<I64>> exp=[51,55)` | `E1303 [76,77) A<I64>|A<A<I64>> exp=[51,55)`<br>`E0102/type [91,92) A<I64>|I64 exp=[83,85)` | si |  |
| 32 | M2 | `E0102/type [92,95) A<A<A<I64>>>|A<A<I64>> exp=[84,87)`<br>`E1303 [77,78) A<A<I64>>|A<A<A<I64>>> exp=[53,57)` | `E1303 [77,78) A<A<I64>>|A<A<A<I64>>> exp=[53,57)`<br>`E0102/type [92,95) A<A<A<I64>>>|A<A<I64>> exp=[84,87)` | si | decision no fijada T2 (ver `orden-no-fijado.md`) |
| 33 | O01 | `E0102/not_ [139,144) domain:array|Bool `<br>`E0102/type [209,212) A<A<A<I64>>>|A<A<I64>> exp=[198,203)`<br>`E0102/type [93,99) Bool|A<A<A<I64>>> `<br>`E1303 [191,192) A<A<I64>>|A<A<A<I64>>> exp=[166,170)` | `E0102/type [93,99) Bool|A<A<A<I64>>> `<br>`E0102/not_ [139,144) domain:array|Bool `<br>`E1303 [191,192) A<A<I64>>|A<A<A<I64>>> exp=[166,170)`<br>`E0102/type [209,212) A<A<A<I64>>>|A<A<I64>> exp=[198,203)` | si | decision no fijada T2 (ver `orden-no-fijado.md`) |
| 34 | O03 | `E0102/type [277,278) Bool|A<I64> `<br>`E0102/type [358,359) A<I64>|I64 exp=[203,206)`<br>`E1303 [212,215) A<A<I64>>|A<A<A<I64>>> exp=[185,186)` | `E1303 [212,215) A<A<I64>>|A<A<A<I64>>> exp=[185,186)`<br>`E0102/type [277,278) Bool|A<I64> `<br>`E0102/type [358,359) A<I64>|I64 exp=[203,206)` | si |  |
| 35 | O07 | `E0102/type [159,160) A<I64>|I64 exp=[140,143)`<br>`E1303 [149,152) A<A<I64>>|A<A<A<I64>>> exp=[119,120)` | `E1303 [149,152) A<A<I64>>|A<A<A<I64>>> exp=[119,120)`<br>`E0102/type [159,160) A<I64>|I64 exp=[140,143)` | si |  |
| 36 | O08 | `E0102/type [100,101) Bool|A<A<A<I64>>> `<br>`E0102/type [107,109) Bool|A<I64> `<br>`E0102/type [130,131) Bool|I64 `<br>`E0102/type [155,158) A<A<A<I64>>>|A<A<I64>> exp=[144,147)`<br>`E0102/type [93,94) Bool|A<A<A<I64>>> `<br>`E1303 [137,138) A<A<I64>>|A<A<A<I64>>> exp=[116,120)` | `E0102/type [93,94) Bool|A<A<A<I64>>> `<br>`E0102/type [100,101) Bool|A<A<A<I64>>> `<br>`E0102/type [107,109) Bool|A<I64> `<br>`E0102/type [130,131) Bool|I64 `<br>`E1303 [137,138) A<A<I64>>|A<A<A<I64>>> exp=[116,120)`<br>`E0102/type [155,158) A<A<A<I64>>>|A<A<I64>> exp=[144,147)` | si | decision no fijada T2 (ver `orden-no-fijado.md`) |
| 37 | M2_noasg | `E1303 [77,78) A<A<I64>>|A<A<A<I64>>> exp=[53,57)` | `E1303 [77,78) A<A<I64>>|A<A<A<I64>>> exp=[53,57)` | si |  |
| 38 | M2_ctrl | `E1303 [77,78) A<A<I64>>|A<A<A<I64>>> exp=[53,57)` | `E1303 [77,78) A<A<I64>>|A<A<A<I64>>> exp=[53,57)` | si |  |
| 39 | DREF_valid | (sin diagnosticos) | (sin diagnosticos) | si |  |
| 40 | P1 | `E0102/type [109,111) A<A<Bool>>|A<I64> exp=[93,101)`<br>`E1303 [70,71) Bool|A<A<Bool>> exp=[60,64)` | `E1303 [70,71) Bool|A<A<Bool>> exp=[60,64)`<br>`E0102/type [109,111) A<A<Bool>>|A<I64> exp=[93,101)` | si | decision no fijada T2 (ver `orden-no-fijado.md`) |
| 41 | c3 | `E0102/type [107,109) A<A<A<I64>>>|A<I64> exp=[93,99)`<br>`E1303 [70,71) Bool|A<A<A<I64>>> exp=[60,64)` | `E1303 [70,71) Bool|A<A<A<I64>>> exp=[60,64)`<br>`E0102/type [107,109) A<A<A<I64>>>|A<I64> exp=[93,99)` | si | decision no fijada T2 (ver `orden-no-fijado.md`) |
| 42 | w1 | `E0102/type [108,110) A<A<Bool>>|A<I64> exp=[93,100)`<br>`E1303 [70,71) Bool|A<A<Bool>> exp=[60,64)` | `E1303 [70,71) Bool|A<A<Bool>> exp=[60,64)`<br>`E0102/type [108,110) A<A<Bool>>|A<I64> exp=[93,100)` | si | decision no fijada T2 (ver `orden-no-fijado.md`) |
| 43 | P2 | `E0102/type [127,129) A<A<I64>>|A<I64> exp=[119,124)`<br>`E0102/type [133,134) A<A<I64>>|A<A<A<I64>>> exp=[75,80)`<br>`E1303 [108,114) A<A<A<I64>>>|A<Bool> exp=[75,80)` | `E1303 [101,102) A<A<A<I64>>>|A<A<I64>> exp=[75,80)`<br>`E0102/type [127,129) A<A<I64>>|A<I64> exp=[119,124)`<br>`E0102/type [133,134) A<I64>|A<A<I64>> exp=[119,124)` | **no** | decision no fijada T2 (ver `orden-no-fijado.md`) **DISCREPA**: ver seccion de discrepancias |
| 44 | P3 | `E0102/type [81,82) A<A<I64>>|A<A<A<A<I64>>>> exp=[57,58)`<br>`E0102/type [99,100) A<A<I64>>|I64 exp=[88,94)` | `E0102/type [81,82) A<A<I64>>|A<A<A<A<I64>>>> exp=[57,58)`<br>`E0102/type [99,100) A<A<I64>>|I64 exp=[88,94)` | si |  |
| 45 | w3 | `E0102/type [129,131) A<A<I64>>|A<I64> exp=[119,126)`<br>`E0102/type [135,136) A<A<I64>>|A<A<A<I64>>> exp=[75,80)`<br>`E1303 [108,114) A<A<A<I64>>>|A<Bool> exp=[75,80)` | `E1303 [101,102) A<A<A<I64>>>|A<A<I64>> exp=[75,80)`<br>`E0102/type [129,131) A<A<I64>>|A<I64> exp=[119,126)`<br>`E0102/type [135,136) A<I64>|A<A<I64>> exp=[119,126)` | **no** | decision no fijada T2 (ver `orden-no-fijado.md`) **DISCREPA**: ver seccion de discrepancias |

## Filas nuevas del plan 5 y observacion de 8 bis

Esperado transcrito del plan (primario y relacion; los tipos los deriva el oraculo).

| Fila | Esperado del plan | Oraculo | Coincide |
|---|---|---|---|
| X_join_local | E1303 [77,78) rel [59,60) | `E1303 [77,78) A<A<A<I64>>>|I64 exp=[59,60)` | si |
| X_compat_then_absorbed | E1303 [77,78) rel [59,60) | `E1303 [77,78) A<A<A<I64>>>|I64 exp=[59,60)` | si |
| X_pend_second_leaf | E1303 [95,96) rel [59,60) | `E1303 [95,96) A<I64>|I64 exp=[59,60)` | si |
| X_two_unions | E1303 [86,87) rel [68,69); E1303 [133,137) rel [115,116) | `E1303 [86,87) A<A<A<I64>>>|I64 exp=[68,69)`<br>`E1303 [133,137) A<A<A<I64>>>|Bool exp=[115,116)` | si |
| obs_8bis | 1 E1303 en la hoja `1` (`[95,96)`) | `E1303 [95,96) A<A<A<I64>>>|I64 exp=[59,60)` | si |

## Lo que el oraculo no modela

- Filas NO DERIVABLES de la matriz (R5, P4, P4pos, generated1411, generated1681): R5 usa `while`; P4/P4pos usan `if[]then[]else` sin
  espacios y `if()then` (el parser del subconjunto las rechaza con error de sintaxis fuera del subconjunto). generated1411/1681 se
  procesan (seccion final) pero **no se validan** contra la matriz (la matriz las declara sin derivacion cerrada).
- Fuera del subconjunto: asignacion indexada `r[i]=e` y anotaciones `Array<T>`; `AssertEq`; `Drop`; bucles; varias funciones con
  llamadas; cierres; `Never` salvo el literal `{return;}`; dominio `copyable_data` de `==` (se supone satisfecho) y dominio de `<` con
  operandos no I64; el cierre de dominio indeciso de 4.1.2.3 salvo lo que usan `Len` y `<`; relaciones condicionales de proyeccion con
  base que se vuelve escalar mas tarde (solo `not_indexable` inmediato). V (derivados) no se emiten; D (hojas adicionales de una misma
  restriccion) se listan en `d_optional` pero no se comparan (presencia D/V es I, 4.1.2 I(a)).
- Cuotas de solver, pilas y `NX-E0003`: fuera de alcance (andamiaje).


## Filas fuera de la matriz (salida del oraculo, sin validar)

- R5: error de sintaxis del subconjunto: sintaxis fuera del subconjunto: primario inesperado 'while' @231
- P4: error de sintaxis del subconjunto: sintaxis fuera del subconjunto: primario inesperado ')' @124
- P4pos: error de sintaxis del subconjunto: sintaxis fuera del subconjunto: primario inesperado ')' @124
- generated1411: `E0103 [78,80) | `; `E0102/type [103,104) Bool|I64 `; `E0102/not_ [142,147) domain:array|Bool `; `E1303 [156,157) A<A<I64>>|A<I64> exp=[111,131)`; `E1303 [201,206) A<A<I64>>|A<A<A<A<I64>>>> exp=[100,170)`; `E0102/type [217,218) I64|A<I64> exp=[192,195)`; `E0102/type [223,226) I64|A<I64> exp=[192,195)`; `E0102/type [231,237) I64|A<Bool> exp=[192,195)`
- generated1681: `E0102/type [73,74) Bool|I64 `; `E1303 [103,105) A<A<I64>>|A<I64> exp=[94,97)`; `E0102/type [116,122) Bool|A<I64> `; `E1303 [128,132) A<A<I64>>|I64 exp=[81,105)`; `E0102/type [145,146) I64|A<I64> exp=[150,152)`

## Adjudicación de B (2026-11-22, `docs/correcciones/I07c-S5c-oraculo-B-r1.md`)

W0, P2 y w3 se adjudican **a favor del oráculo**. La matriz tenía un error de bytes en W0 (`[59,63)`, no `[60,64)`) y se contradecía en P2 y w3 (su E1303 exige `tipo(r)=A²` y su E0102 de `r=[r]` exige `A³`). `matriz-esperados.json` y `rec_all.py` se corrigen **solo en esas tres filas**: W0 `exp=[59,63)`; P2 y w3: única E1303 sobre `r` `[101,102)` (`A³`/`A²`, rel. `[75,80)`), E0102 de `q=ar` (P2 `[127,129)`, w3 `[129,131)`; `A²`/`A¹`; rel. `[119,124)` y `[119,126)`) y E0102 de `r=[r]` (`[133,134)` y `[135,136)`; `A¹`/`A²`, misma relación). Con ello `--matrix` da 45/45. Los números de la tabla de arriba son los previos a la adjudicación.

