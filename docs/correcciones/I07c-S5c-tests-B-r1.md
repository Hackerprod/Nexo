# Validador B: adjudicacion de tests de I07c-S5c (candidato-5) - INFORME

Metodo: derivacion propia desde ADR (C1/C1b/C1c l.301, C2/cl.4 l.353, N(d) l.283/l.337, l.367, T5); binarios debug de `git archive origin/main` (eeb3c9c) y de main+`candidato-5.diff`; CLI `check --profile f0-0002-b-v2` (f0-0003-b-v1 donde hay `{value:}`).
Limite de honestidad: ya habia leido INFORME-6 antes de derivar; derive cada fila desde el ADR y compare despues. El oraculo solo como contraste (no modela AssertEq, while, `{}`; es previo a T5).
Programas y salidas: C:\nexo-work\tmp\validador-b-tests\src\*.n, cmp.py, probe-*.log, tests-*.log.

## Fallos con candidato-5 (nextest --profile default, debug)
Candidato: 2167 ejecutados, 2150 pasan, 17 fallan, 72 omitidos; los 17 nombres son IDENTICOS a tests-candidato5.log (diff vacio). Base eeb3c9c: 2168 pasan, 0 fallan.
El candidato ELIMINA un test (2168 vs 2167): check::types::quota_tests::i07c_survivor2_nested_shape_hints_reach_the_leaf (prueba ShapeHint; matador registrado de M00123/M00125); INFORME-6 sec.1 no lo cita. El candidato lleva interruptores NEXO_XV (todos activos por defecto).

## Tabla (diagnosticos = codigo + primario en bytes)
1 mixed_nested_join...: derivo solo E1303 `[2,3]` [82,87) Array<Array<Bool>>/Array<I64> (C1c,C2). main anade E0103 [40,42). Respalda candidato (oraculo igual). Coincide: si. Edicion: esperado `[E1303 "[2,3]"]`. PASA.
2 adr_gap_nested_union...: solo E1303 `br` [76,78) Array<I64>/I64. main anade E0103 [48,50). Candidato. Coincide: si. Edicion: `[E1303 "br"]`. PASA.
3 failed_outer_condition...: E0102 `1` [43,44), E0102 `x` [62,63), E1303 `br` [78,80). main anade E0103 [50,52). Candidato (oraculo igual). Coincide: si. Edicion: quitar el E0103. PASA.
4 failed_outer_array...: E0103 `[]` [25,27) y E0102 `[1]` [49,52). `[t2]` tiene elemento t2 ya Array: sin obligacion propia (C1); la fila l.321 `var r=[];r=[r];` (unico E0103; comprobado en ambos binarios [16,18)) lo confirma. Reserva: el E0103 del literal exterior [34,53) (elemento alfa sin resolver, como `[q1[0],q1]`) no lo emiten main ni candidato cuando hay un elemento erroneo; el ADR no lo decide con claridad: INCONCLUSO esa subpregunta. Candidato en lo que cambia. Coincide: si. Edicion: 2 diagnosticos. PASA (con reserva).
5 linked_array... caso 2: unico E0103 `[]` interno de k2 [80,82) (k0=[k2] da a=Array<b>; `[k0,k0]` tiene elemento con constructor, C1/C1b). Candidato. Coincide: si. Edicion: caso 2 `[(80,82)]`. PASA.
6 cyclic_constructor...: `[43,45)` es una LECTURA (C1b): su E0103 es indefendible; main falla ahi. Pero los otros dos no salen de una derivacion unica: el gemelo con let `fn Main(){var m1=[];let a=[m1[7],m1];AssertEq([true],a);}` da en AMBOS binarios solo E0102 `m1` [33,35) (alfa=Bool por igualdad compatible, C2), mientras el inline `AssertEq([true],[m1[7],m1])` da E0103 [17,19)+[36,46) (cand) o +[43,45) (main). La seccion de lagunas sobre el literal heterogeneo ("no ofrece un tipo unico a ningun consumidor") respalda el inline; la fila let la contradice. Dos lecturas + implementacion inconsistente entre formas equivalentes (4.1.2.7). El oraculo no modela AssertEq (su [17,19) es parcial). Coincide: NO en que "[17,19),[36,46)" sea derivado del ADR; si en retirar [43,45). Edicion: no reescribir a "dos sitios" como esperado ADR; retirar solo [43,45) y registrar la discrepancia inline/let como laguna o defecto abierto. INCONCLUSO.
7 independent_causal_sites... `nested_constructor`: q0 queda Array<Array<b>> por la union; `[q0]` tiene elemento con constructor; solo E0103 `[]` interior de q1 [28,30). Candidato (oraculo igual). Coincide: si. Edicion: caso `[(28,30)]`. PASA.
8 deferred_projection... `repeated_constructor_sites`: E0103 `[]` [17,19) y `[q1[0],q1]` [27,37); `q1` [34,36) es lectura. Candidato (oraculo igual). Coincide: si. Edicion: caso `[17,19),[27,37)`. PASA.
9 unresolved_constructor_survives... (N(d)): gemelo con condiciones validas `fn Main(){var t0=[];(if true then{value:(if true then{value:(if true then{value:t0;}else{value:t0;});}else(if true then true else true));}else(5));}` (f0-0003-b-v1): cl.4 l.353 "si esa union interna discrepa, su primera continuacion valida aporta obligatoriamente el testigo parcial": referencia exterior Array<alfa> (t0); el conflicto de `5` queda pendiente y se omite; unico E0103 `[]` [17,19). Candidato = eso; main (E0103 + E1303 `5` [143,144) Bool) contradice la clausula. Fixture original con `if 1`: main y candidato tratan por igual una `if` con condicion fallida como testigo en continuaciones (`fn Main(){let q=(if true then (if 1 then 1 else 2) else true);}` -> E0102 `1` [34,35) + E1303 `true` [56,60) en ambos), pero T5 dice "una if con condicion fallida es una unidad contaminada que no establece referencia" (ejemplo solo en array): la lectura literal de T5 da lo de main. Coincide: parcial (INFORME-6 no considera T5). Edicion: no reescribir tal cual; sustituir por el gemelo valido (solo E0103 [17,19) en el candidato) y decidir aparte T5 para continuaciones. INCONCLUSO.
10 incompatible_prefix...: r:Array<a>, q:Array<b>; `[[r],q]` => b=Array<a>; condicion `while` espera Bool: E0102 literal [53,60) Bool/Array<Array<Array<I64>>>; `q=[3]` posterior incompatible (l.301 "nunca se usa una asignacion rechazada para cambiar el tipo de lecturas anteriores") => E0102 `3` [66,67) Array<I64>/I64; `r=[3]` da a=I64 (sin E0103, C1c); E0102 `x` [82,83); E1303 `q` [96,97) Bool/Array<Array<I64>>. Candidato identico incluidos tipos. Coincide: si. Edicion: `[E0102 "[[r],q]", E0102 "3", E0102 "x", E1303 "q"]`. PASA.
11 failed_nested_union_does_not_anchor...: ref Array<a> (r); `[[q]]` compatible; `[[1]]` primera incompatible: E1303 [81,86) Array<Array<Array<I64>>>/Array<Array<I64>>; `r=[3]` rechazada: E0102 `3` [92,93) Array<Array<I64>>/I64; `q=[3]` fija b. Candidato (oraculo igual, mismos tipos). Coincide: si. Edicion: esos dos. PASA.
12 anchored_first_branch...: ver (c). Coincide: si. Edicion: un solo E0102 `[136,137)`. PASA.
13 pending_inner_constructor...: ver (d). Coincide en la lectura B; discrepo en llamarla laguna pura: l.353+l.301+l.367 la fijan. PASA.
14-16 andamiaje: ver (e). PASA (andamiaje).
planned_projection_child...: no estaba en el encargo; INFORME-6 lo declara defecto abierto del candidato. NO EVALUADO.

## (c) anchored_first_branch
Programa: `fn Main(){var r=[];var s=[];let q=(if true then{value:s;}else (if true then{value:(if true then{value:r;}else{value:[];});}else{value:s<1;}));r=[[1]];s=[[]];}`.
Referencia = s. Aplanado posterior: `r` y `[]` compatibles; `s<1` tiene el operando `1` fallido (E0102 [136,137), Array<Array<I64>>/I64): resultado CONTAMINADO (T5: no es incompatible con la referencia y no recibe NX-E1303 ni NX-E0102 por ese resultado; l.367: no se usa el supuesto Bool). La envoltura tampoco se denuncia (cl.4). Todo se resuelve: sin E0103. Esperado = SOLO E0102 [136,137). Coincido con INFORME-6.
Analogo concreto `fn Main(){let q=(if true then 1 else (true<1));}`: segun T5 NO hay E1303 (solo E0102 `1` [43,44)); comprobado main = candidato. Igual `let s=[[1]];let q=(if true then s else (if true then s else s<1));` -> E0102 [72,73). El E1303 [134,137) de main es incoherente con su propio analogo concreto; el oraculo (previo a T5) da E1303 en ambos analogos y es el desfasado.

## (d) pending_inner_constructor_preserves_outer_branch_comparison
Programa (103 B): `fn Main(){let ar=[1];let c=true;var m1=(if true then {value:ar[0];} else (if c then [[]] else false));}`. main: E0103 [85,87) + E1303 `false` [94,99); candidato: solo E0103 [85,87).
Lectura A (main): `[[]]` pendiente no ofrece tipo; `false` es la primera incompatible. Lectura B (candidato): `[[]]` es incompatible por raiz, es la primera posterior incompatible y consume la unica E1303 de la union (l.353); su serializacion esta pendiente: l.301 la omite solo si el componente pendiente emite NX-E0103 y l.367 dice que un conflicto pendiente de serializacion cuenta como ya denunciado. Evidencia medida: si el constructor se completa despues (117 B, abajo) main Y candidato dan una sola E1303 sobre `[q]` y ninguna sobre `false`: es el mismo conflicto, solo cambia que se pueda serializar; main es inconsistente consigo mismo. A solo se sostiene si `[[]]` cuenta como literal con defecto interno; el ADR enumera defecto interno, lectura contaminada o union erronea, no un constructor irresuelto (que tiene su propia NX-E0103).

Texto propuesto para docs/adr/F0-0002-lagunas.md (relleno de laguna, revertible):

    ## Continuacion posterior con constructor pendiente en una union (relleno de laguna, revertible)
    Un literal de array cuyo unico defecto abierto es un constructor irresuelto (`[[]]`, `[q]` con `q` sin resolver) no es un
    literal con defecto interno de la seccion anterior: se contrasta con la referencia de la union como un literal sin defecto.
    Si es incompatible por su raiz (escalar frente a Array) es la primera continuacion posterior incompatible y consume la unica
    NX-E1303 de la union (4.1.2.4), aunque su `found_type` siga pendiente: ese conflicto se omite mientras el componente
    pendiente emita NX-E0103 (4.1.2.2) y cuenta como ya denunciado (4.1.2.5). Las hojas posteriores incompatibles con la misma
    referencia no reciben NX-E1303. Si una igualdad posterior completa el componente, la NX-E1303 se emite sobre ese mismo literal
    y no sobre otra hoja. `f0-0002-b-v2`, `check`, ASCII sin BOM ni LF final. Invalido (103 B):
    `fn Main(){let ar=[1];let c=true;var m1=(if true then {value:ar[0];} else (if c then [[]] else false));}` -> unico NX-E0103
    `[85,87)`; sin NX-E1303 sobre `[[]]` ni sobre `false` `[94,99)`. Completado despues (117 B):
    `fn Main(){let ar=[1];let c=true;var q=[];var m1=(if true then {value:ar[0];} else (if c then [q] else false));q=[1];}` ->
    unico NX-E1303 `[93,96)` (`[q]`, `I64`/`Array<Array<I64>>`). Valido (96 B): el primero con `(if c then 2 else 3)` -> `checked`.

Spans comprobados con main y candidato (el primer ejemplo es la diferencia; los otros dos coinciden en ambos).

## (e) andamiaje (I10a)
ADR l.375: los dos niveles de I07c rigen con los limites normativos de cuota del perfil normal f0-0002-v2 (11.8 A), no con los perfiles reducidos de I10a; la nota del propio test dice que los umbrales reducidos exactos son de I10a. Las cifras 1..=8, 30/31 y los numeros de linea no los enuncia el ADR: ANDAMIAJE.
- deferred_if_join_charges_the_reduced_boundary: medido con sonda en copia: main niega con limites 1..=11 y acepta con 12; el candidato niega 1..=7 y acepta con 8; ambos con exactamente un NX-E0003 observed=limit+1. El test con 1..=8 coincide con main por casualidad. Esperado derivado de 11.1: calcular `coste` = menor limite sin errores y exigir, para todo limite < coste, exactamente un NX-E0003 (type_solver_steps, limit, observed=limit+1) y sin errores con `coste` y con 1_000_000. Sin cifra fija.
- failed_branch_join_keeps_reduced_quota_error_prefix: la ejecucion completa SI contiene NX-E0006 [176,213) (ambos binarios), no es "no relacionado"; el candidato lo emite con limite 30 porque su secuencia de cargos esta desplazada 2 (cand@30 == main@32). Comprobado en 1..=60 en ambos: como mucho un NX-E0003 y todo diagnostico no-cuota del recorte pertenece a la ejecucion completa (subconjunto exacto, 11.1/13.9). Esperado derivado: para cada limite < coste, un solo NX-E0003 y subconjunto de la ejecucion completa; sin 30/31.
- structural_invariants::grouped_operands_keep_one_causal_primary: el auditor scripts/i07c_span_audit.py fija lineas de types.rs. Renumerar NO basta: con 7078->7069, 8279->8297, 9061/9062->9106/9107, 11300->11345, 12248->12294, 12484->12530, 13058->13104, 13168->13214, 13379->13425, 13384->13430, 13559->13605, 14797->15138, 14811->15152 el auditor aun falla con unreviewed raw span en 14788 (`let span = expr.span();` dentro de `xv("rmm3")`, codigo exploratorio) y hay coincidencias ambiguas (8279, 14797, los deferred_result): hace falta revision humana. Proponer indexar REVIEWED por funcion+expresion.

## Veredicto global: FAIL (de la propuesta de adjudicacion, no del candidato)
1. cyclic_constructor...: "[17,19),[36,46)" presentado como derivado del ADR no lo es. Repro: `fn Main(){var m1=[];AssertEq([true],[m1[7],m1]);}` (cand: E0103 [17,19),[36,46)) vs gemelo `fn Main(){var m1=[];let a=[m1[7],m1];AssertEq([true],a);}` (main y cand: solo E0102 [33,35)). Solo la retirada de [43,45) esta respaldada.
2. unresolved_constructor_survives...: INFORME-6 omite T5; sustituir por el gemelo valido y decidir T5 para continuaciones (en ambos binarios: E1303 `true` en `(if true then (if 1 then 1 else 2) else true)`).
3. El candidato elimina un test no citado (i07c_survivor2_nested_shape_hints_reach_the_leaf).
4. El auditor de spans no se arregla renumerando (14788 sin revisar; coincidencias ambiguas).
Resto: 11 filas coinciden (PASA); planned_projection NO EVALUADO. No bloqueante: NEXO_XV; oraculo previo a T5 e incompleto.
