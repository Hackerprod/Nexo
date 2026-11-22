# Informe B ronda 2 (dirigida) - I07c-laguna-esperado-diferido-r1.md (110 lineas)

Veredicto: FAIL, un unico motivo (error factual en el ejemplo valido de D-REF). Todo lo demas PASA.

## (1) D-REF
- Registrada como relleno revertible, con pregunta, decision, apoyo literal y alternativa descartada (`r[0]` [53,57)) para Flujo A: PASA. Citas literales verificadas contra el ADR: l.353 (referencia de la union; "si aun es variable, se relaciona..."), l.301 (igualdad incompatible rechazada sin desconectar; primera igualdad compatible), l.355, l.303 (solo niega E0103 por variable hipotetica). Sin contradiccion con 4.1.2.3, 4.1.2.5, l.355 (la referencia variable no distribuye contexto concreto) ni RD.
- Ejemplo invalido M2 (97 bytes): spans comprobados por bytes: r[0] [53,57), r [77,78), aar [84,87), aar RHS [92,95). Salida de main sin D-REF: la propuesta no la contradice. Control `...r=[[[3]]]` verificado en main: solo E1303 [77,78) Array<Array<I64>>/Array<Array<Array<I64>>>, rel [53,57): PASA.
- FALLA: el ejemplo valido (79 bytes) `fn Main(){let aar=[[1]];var r=[];let a=(if true then r[0] else aar);r=[[[3]]];}` se declara "salida 0, sin diagnosticos". `check` da 0/checked/[], pero `run` da salida 1, outcome simulator_fault, R0102 `value.index` primario [54,57) (r[0] sobre un array vacio se EVALUA porque la condicion es `true`). Los demas ejemplos del addendum distinguen check/run; este no, y el texto "(medido en main)" solo vale para check. Correccion minima: cambiar `true` por `false` (`if false then r[0] else aar`, 80 bytes; verificado: check 0 checked, run 0 unit, diagnostics []) o acotar el ejemplo a `check`.

## (2) Cabecera
Separa validado (repro A, M1, O03, O07) de decision (D-REF: M2, O01, O08, O02) y pendientes (O05, O04, O06 con pregunta literal para Flujo A): PASA.

## (3) Regresiones y nota del test
(i) A sin asignacion (90 bytes): J [83,86) [r], rel [53,54): spans correctos; main hoy solo E0103. (ii) `[r]` antes de `aar` (96 bytes): J [74,77) [r], M [92,93) con rel aar [83,86): spans por bytes correctos; main hoy E0103 + J erronea en aar [83,86). (iii) gemelo `var r=[3]` (91 bytes): main solo J en aar [75,78), marcado no oraculo: correcto. Nota del test `i07c_pending_nominal_domain::incompatible_prefix_...`: coincide con mi derivacion de ronda 1: PASA.

## No bloquea
El apartado de barrido (linea 102) conserva cifras (37 fuentes, 127 filas) que no he re-verificado; mi barrido independiente (152 RD sin desvios, 217 .n, literales de tests) coincide en la conclusion.
