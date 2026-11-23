# Validacion B de I07c-laguna-t5-r1 (Flujo A) - veredicto: FAIL (3 motivos de documento; la conclusion de fondo se confirma)

Nota de metodo: lei la propuesta antes de derivar (el brief lo ordenaba al reves); mis esperados de 70+ programas
(`C:\nexo-work\tmp\validador-b-t5\s\*.n`: p01-p24, q01-q17, r01-r08, s01-s05, u01-u04, v01-v06, O01, O08) se derivaron
a mano del ADR (4.1.1, 4.1.2 cl.4/cl.5, N(a)-N(j), RD21/65/94/95/105/110/126) y luego se compararon con `main` y V6:
coinciden en todos y `main` == V6 en todos ellos (salvo O01/O08, ver 5).

## (1)/(2) El ADR fija la respuesta: PASA
Apoyo textual (ademas del de la propuesta): 4.1.2 cl.5 (4.1.2.5, "arista circular", I11a): "El fallo retira la evidencia del resultado
nominal de la operacion de los consumidores derivados de ese resultado, comprobados antes o despues"; y el ejemplo b2d (`[a0,r]` sin C en
`r` final). La propuesta no cita esa precision. RD110 (hoja de union `1`/`true+1` sin tipo para `==`) y RD21/22/76/77 (anotacion) confirman.
Contraejemplo buscado sin exito: ninguna clausula ni RD ofrece el nominal a elementos/hojas tras un fallo; el cierre indeciso lo condiciona a
"no haya fallado una obligacion de la operacion ni un hijo necesario" (v05 `[Len(q),true,1]` con q indeciso: ofrece I64, `true` [33,37), coincide).
La lectura "nominal por operador" no tiene apoyo: contradice cl.4 (`<`/`==`: "ni un resultado valido a la operacion"), cl.5 y RD110.
Discriminantes reales (C cambia de primario, no solo V): `fn Main(){let a=[true+1,true,1];}` -> C `[17,21)` y C `[29,30)` Bool/I64
(lectura B daria `[24,28)` y ninguno en `1`); `[x+1,true,1]` con `let x=true+1;` -> `[16,20)` y `[39,40)`; `var r=[];r=[true+1];` -> NX-E0103 `[16,18)` mas NX-E0102 `[22,26)`.
Resto (argumento, `a[true+1]`, `Len(true)`, `G(true+1)`, `AssertEq`, anotacion, if con contexto, `Array<Bool>` externo) coherentes con A.

## (3) Aclaracion derivada: FALLA por un span
Texto sin decision abierta salvo lo anotado en No bloqueante. Ejemplo valido `[1+2,3]` y `var q=[];let a=[q[0]<1,true];q=[1];` -> checked (verificado);
contraste `[q[0]<1,1]` -> unico E0102 `[33,34)` Bool/I64 con relaciones expected `[26,32)` y found `[33,34)` (verificado); corregido `[1+1,true]` -> `[21,25)` (verificado).
Invalido `[true+1,true]`: `[17,21)`, I64/Bool, relacion found `[17,21)`, sin expected (verificado). PERO "sin diagnostico en el segundo `true` `[22,26)`" es erroneo:
bytes `[22,26)` = `1,tr`; el segundo `true` es `[24,28)`.

## (4) Salidas protegidas: PASA
`i07c_rd_conformidad.py --verify cand-release.exe`: 152/152 pares check/run, 0 diferencias de instantanea. Barrida propia de los 243 `.n` versionados
(`git ls-files`, perfil f0-0002-b-v2) base vs V6: 225 iguales, 18 distintos, todos bajo `docs/validation/i07c-s5a/{d-repros,limites}` y `i07c-twin/repros` (familia S5, defectos ya
conocidos de `main` que V6 corrige; no son T5). Mas ~58 programas con la forma (operacion fallida alimentando elemento/hoja/argumento/contexto): 0 diferencias base/V6.

## (5) Reclasificacion en S5c
Filas afectadas de `docs/validation/i07c-s5c/orden-no-fijado.md`: T5 (unica). Casos: O01 (via `[2][0] + false[x]`) y O08. El oraculo (`i07c_s5c_oraculo.py`) no anade T5 a `no_fijado`
(solo T1/T2) y coincide con V6 por codigo/primario en O01 y O08 (`[93,99) [139,144) [191,192) [209,212)` y `[93,94) [100,101) [107,109) [130,131) [137,138) [155,158)`).
`main` difiere de ambos en O01/O08 (E0103 `[74,76)`, E1303 en otro primario) pero por la familia S5 (asignacion `r=aar`/ancla), no por T5. Ninguna diferencia de E atribuible a T5 queda como DEFECTO de V6; el diferencial de B1 no cambia por T5.
O08 no depende de una operacion: depende de cl.5 2.o parrafo (union erronea como primer elemento no transmite referencia); `[(if true then [1] else [true]),x]` -> solo E1303 `[48,54)` (u01). La propuesta solo reclasifica O01.

## Motivos del FAIL (accionables)
1. Span erroneo `[22,26)` por `[24,28)` en la fila 1 de la tabla 1.3 (col. derecha) y en el ejemplo invalido del texto propuesto para el addendum (linea "sin diagnostico en `[22,26)`"). Es texto destinado a ser normativo.
2. Reclasificacion incompleta en 5: la propuesta nombra solo O01; el oraculo de E y su validacion citan O01 y O08. Anadir O08 (fijado por cl.5 2.o parrafo, no por operacion) y retirar el aviso de T5 en `orden-no-fijado.md`, B-oraculo-r1 s.4/cond.4 y las menciones de T5 en los planes S5c.
3. 1.3/2 afirman que la lectura B "mueve presencia de C" con NX-E0102 en el segundo elemento/hoja; segun 4.1.2.5 ese diagnostico seria V (necesita el tipo de una expresion contaminada), presencia I. El efecto N de B se ve solo en programas como `[true+1,true,1]`, `[x+1,true,1]` y `var r=[];r=[true+1];` (NX-E0103). Sustituir/anadir esas filas como discriminantes; sin ellas la justificacion de "nucleo" no se sostiene.

## No bloqueante
- Anadir al texto del addendum el efecto sobre constructores irresueltos (`var r=[];r=[true+1];` conserva NX-E0103 `[16,18)`; N(h)/RD127 analogo) y la `if` con condicion fallida como unidad contaminada (`[(if 0 then 1 else 2),true,1]` -> `[21,22)` y `[43,44)`).
- Citar la precision de "arista circular" como apoyo; el ejemplo "bloque con valor" no tiene programa.
- Las 18 diferencias main/V6 fuera de T5 son del corte S5, no de esta propuesta.
