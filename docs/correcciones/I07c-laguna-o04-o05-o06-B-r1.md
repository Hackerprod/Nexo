# Informe B - I07c-laguna-o04-o05-o06-r1.md (85 lineas)

Veredicto: FAIL (r1). Dos motivos bloqueantes, ambos de la propuesta (no de la regla de fondo). Lo demas PASA.
Evidencia: auxiliares chk.py, sweep3.py en C:/nexo-work/tmp/validador-b-lag2; CLI main f0-0002-b-v2. Yp denomina la lectura Y prima de la propuesta.

## (a) O05 / M4 - PASA
Derivacion propia (orden fuente): r:Array<ar>, s:Array<as>; en a=[[r],s] el elemento [r] fija la referencia Array<Array<ar>> y s la iguala: as:=Array<ar> (origen [60,63)). En b: referencia [s]=Array<Array<Array<ar>>>; aar:Array<Array<I64>> da conflicto de forma I64 vs Array<ar> (conocible con ar pendiente): unica E1303 en aar [96,99) con tipos completados al cierre. s=ar: Array<I64> contra Array<Array<ar>> incompatible: E0102 en ar [103,105) con origen [60,63), rechazada atomicamente. r=ar: ar:=I64. Sin E0103. Coincide con la propuesta (tipos, relaciones, orden).
Spans por bytes (112 B): [41,43) [50,52) [60,63) [64,65) [87,90) [96,99) [103,105) [108,110): correctos. main sin la union [s] (78 B): solo E0102 [69,71), expected_type [60,63): coincide con la regla. Controles medidos en main: [s] con s=aar solo E1303 en aar (113 B); s con s=ar solo E0102 en ar (110 B); positivo 111 B check 0 y run 0 unit. main original: E0103 [50,52) + E0102 en s [64,65): tipado retroactivo (l.301, l.355, l.373).

## (b) O04/O06
1. Contexto I64 del segundo operando tras fallar el primero: lo sostiene el texto. l.357 "Este limite de EMISION no cancela la restriccion I64 de cada operando independiente"; RD71 (x-(if true then ar else 279): la union del segundo operando recibe I64, M en la hoja ar, sin E1303); 4.7 (con tipo esperado no hay E1303). main es incoherente: 1+(if ...) distribuye, true+(if ...) no.
2. X frente a Yp: el texto sostiene UNA lectura, Yp (no hay laguna real). 13.17 (fila +/-: "primer operando incompatible de izquierda a derecha"), l.357 ("sin una segunda denuncia del dominio de la misma operacion"), N(d) 2.a frase ("las hojas excluidas por ese limite no son sitios independientes"), RD15/RD16. X choca con esas citas. Ademas main YA hace Yp en la forma sin pendientes: ar+(if true then true else false) da solo E0102 en ar; (if true then true else false)+1 da dos C en las hojas del operando que consume. La propuesta puede dejarlo como relleno, pero no hace falta llamarlo laguna.
3. Argumento "X rompe pending_join_drain_keeps_all_unresolved_array_diagnostics": cierto como hecho (main da 8; con X ganaria C), pero ese test es de mutacion/implementacion (cuenta 8 y E0103), no oraculo del ADR. Retirar como argumento normativo o marcarlo secundario. No bloquea.
4. B.3 cita "RD64, RD66" para "todas las hojas del operando que consume siguen siendo C": RD64 es anotacion y RD66 tiene una sola hoja. La afirmacion es correcta (N(d) 1.a frase; main: 2 C) pero la cita no la prueba. No bloquea.

## (c) Salidas completas
- O04 (101 B): spans [27,29) [37,39) [56,60) [80,83) [89,90) correctos. Mi derivacion: ar consume (E0102 [37,39), I64/Array<I64>, solo found_type); q[0]:=I64 (intrinseco); [2] y q incompatibles, no denunciadas; q=[3] compatible; sin E0103 ni E1303. Coincide. Medidos en main: positivo 91 B check 0 / run 0 unit; aislado con 1 en vez de ar (100 B): dos C [79,82) y [88,89) (main ya lo da); main original da E0103 + E0102 + E1303 (diverge, correcto).
- O06 (213 B): todos los spans de la tabla correctos por bytes. Mi derivacion independiente da las mismas 8 filas: E0102 aar [105,108) (q consumido, RD15); E0102 br [133,135); J en r [148,149) (D-REF: r[0] referencia, q fija ar:=Array<I64>, r incompatible; expected_type [119,123)); E0102 cond q [159,160), hojas r [166,167) y p [173,174); E0102 cond ar [182,184); E0102 de 3 [209,210) con origen q [141,142); sin E0103; sin E1303 en q ni true; operador < sin derivado.
- FALLA (motivo 2): ambiguedad en el texto del addendum. B.3 y la seccion D dicen que NO consume "un operando contaminado ... por un defecto interno (condicion)". En O06 el operando A=(if q then r else p) tiene la condicion q invalida y ademas hojas que emiten E0102 contra el I64; la tabla de la propia propuesta exige que A consuma (si no, ar y true de B se denunciarian). El texto no decide ese caso: o contradice la fila O06 o deja abierta la decision. Correccion textual: consume el operando que emite E0102 contra el I64 de esa operacion aunque tenga otros defectos internos; no consume el que solo tiene defectos de otra causa (condicion, operacion interior).

## (d) Barrido
- RD: 152 filas con main, 0 desvios de C; ninguna con la forma (primero denunciado + segundo if o literal con hojas incompatibles).
- 217 .n: solo equality_numeric_rhs y equality_numeric_rhs_concrete (declarados; con Yp queda solo el E0102 del primer operando).
- Literales Rust (tests/ y src/, 3352 fuentes, 30 con la forma): FALLA (motivo 1). B.5 afirma que ninguna fuente tiene la forma "primero denunciado + segundo if con hojas incompatibles" con esperado que Yp cambie. Hay dos tests de src/ir/i07c_tests.rs que si:
  * bad_binary_left_still_checks_independent_right_branch (l.951): fn Main(){let x=true+(if true then 1 else false);} espera E0102 [16,20) + E1303 [42,47); con la regla propuesta solo queda E0102 [16,20).
  * tainted_left_operand_preserves_independent_right_join (l.3900): fn Main(){let bad:I64=true;let x=(true+(if bad then false else 41));} espera 3 C incluida E1303 [63,65); con la regla desaparece.
  Repro: check de esas fuentes con main da 2 y 3 diagnosticos. Ambos tests codifican la lectura Y como "defecto independiente de la rama derecha (13.9)". Son tests de implementacion, no el ADR, y el texto (RD71 + l.357 limite de emision) respalda la lectura de la propuesta, pero esta debe listarlos como tests afectados (esperado a corregir en COR tras revision), no afirmar que el barrido no encuentra la forma, y reconocer que su propia regresion true+(if true then [1] else 2) sin E1303 los contradice.
- Ya listados: equality_numeric_rhs (OK), pending_join_drain (conserva 8), incompatible_prefix_... (conocido).

## (e) Limite O05 original (279 B)
No puedo derivar una salida completa: depende de AssertEq (igualdad antes del dominio), uniones con value-blocks a ambos lados, el literal [[r],r,s] con conflicto recursivo en r y varios pendientes enlazados, y del orden en que se fija cada incognita. Solo es seguro que el E0103 [92,94) es espurio (as se fija por [[[2]]][0] y por s=ar). El limite declarado es legitimo; no bloquea.

## Motivos del FAIL
1. Barrido B.5 erroneo: omite src/ir/i07c_tests.rs l.951 y l.3900, que contradicen la regla y la afirmacion de que no hay fuente con la forma.
2. Texto de consumo ambiguo y contradictorio con la fila O06.
