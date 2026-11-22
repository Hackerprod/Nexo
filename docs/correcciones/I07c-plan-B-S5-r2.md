# Informe B - plan I07c-S5 (ronda 2, completa). Veredicto: FAIL (2 bloqueantes documentales, el resto PASA)

Base verificada: HEAD 4d75e2c; blob types.rs = 1050c84c...caf34 (= main). Repros con CLI de main (1bd010c): A, A sin asignacion (E0103 solo), A con [r]/aar permutados,
B q->r / r->q, F06, M2, S5a.2 (101 B y 100 B), F5 (internal_error), fam2, bad_binary, FAM-3 y controles, selector pendiente (3 variantes).
Spans por bytes comprobados contra la fuente: t11 [53,60)/[66,67)/[82,83)/[96,97); t18 [65,66)/[74,75); index_selector r=[160,161), 0=[162,163), [148,150)=[] literal de la base
(CORRIGE mi ronda 1: ese E0103 no es el de r); A [83,86)/[92,93)/[74,77); F06 [76,77)/[91,92)/[51,55)/[83,85); M2 [77,78)/[92,95); S5a.2 [27,29)/[37,39), [79,82)/[88,89).
Esperados t11/t18/A/B/F06/M2 coinciden con mi derivacion en orden fuente (§4.1.2.2/.7, addendum de lagunas); los de la tabla V6 de O01/O03/O07 coinciden con la propuesta.

## Bloqueantes
1. S5b.1 deja sin esperado cuatro programas ("esperado a derivar por A antes de C"). Derivacion (ADR l.303 + §4.1.1 + N(d)), solo selector interno independiente, ningun C del indice exterior:
   - `fn Main(){let ar=[1];let z=(1+true)[[1,true]];}`: E0102 [30,34) `true` (I64/Bool) y E0102 [39,43) `true` (I64/Bool, expected_type [37,38)).
   - `fn Main(){let ar=[1];let z=ar[true][2+true];}`: E0102 [30,34) y E0102 del `true` de `2+true` (recalcular offsets).
   - `fn Main(){let ar=[1];let z=ar[true][0][[1,true]];}`: E0102 [30,34) y E0102 del `true` interior.
   - base Ok(Tainted) `let t=ar[true];let z=t[[1,true]];`: ya emite ambos en main (sin cambio).
2. §8 fila 4, segundo test (`tainted_left_operand_preserves_independent_right_join`): "solo el E0102" es impreciso. Fuente `let bad:I64=true;let x=(true+(if bad then false else 41));`: tras S5a.2 quedan DOS E0102 ([22,26) anotacion y [34,38) `true` de `+`) y desaparece el E1303 [63,65) (la `if` recibe I64 externo, §4.7 y addendum Y'; condicion `bad` contaminada). El plan debe fijar 2 diagnosticos con esos spans.

## No bloquea (registrar)
- V6 conserva `read_plan` para asignaciones ACEPTADAS: retipa lecturas anteriores (ADR l.373, ultima frase). Repro en main (y V6): `fn Main(){var r=[];let a=(if true then r else (if true then [true] else [1]));r=[1];}` da E1303 en `[true]` (expected Array<I64>); orden fuente exige E1303 en `[1]` y E0102 en el `1` de la asignacion. El plan solo declara la regla para asignaciones rechazadas; twin no lo detecta (el gemelo coincide). Debe quedar en Pending con fecha.
- S5a.3: el esperado nuevo de la fila `index_selector` lo deriva C; el informe debe validarlo un tercero (B/D) antes de editar el test. La regla "no comparar si un operando fallo" es del ADR (l.361 "Una operacion con operandos incompatibles no ofrece su tipo nominal de resultado..."); citar esa linea, no solo §4.1.1. `1+(true or false)` no es fila literal del ADR F5 (O06/O08 son analogas).
- Pila: el plan exige 0 B por marco pero cita 7.056 B (+272) de check_against_context_one con el intento S5a.3: decir como se absorbe (ayudante/autorizacion). "limite del script 62.000 B" no figura en scripts/ ni AGENTS.
- Generador del barrido (13 binarios, 2 unarios, 5 compuestas): lista concreta no especificada (andamiaje; semilla y SHA congelado antes de ejecutar si estan).
- Esperados de C de F01-F19 (salvo derivados) no estan derivados: correcto que no entren a tests; D debe derivarlos de forma independiente.
