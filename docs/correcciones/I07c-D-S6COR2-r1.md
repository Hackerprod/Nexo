# Validacion D de I07c-S6-COR-2 (commit 8d8a0d9ad467c749e0b0fec4ca14b1b66dc48d2f)

Veredicto: FAIL (un motivo bloqueante: cobertura de nucleo exigida por el plan, secc. 7; el producto no muestra defecto).

## Por requisito
1. Fidelidad al plan: PASA. Produccion solo src/ir/lower.rs y src/ir/lower_match.rs; un unico ayudante preexistente (expression_primary / expression_primary_span), sin condicionales por patron. El resto (tests/i07c_group_primary.rs, tests/i07c_structural_invariants.rs, scripts/i07c_span_audit.py, docs/validation, informe) esta en el plan.
2. Siete sitios con CLI propia: PASA, debug = release, base frente a candidato.
   - while (n<9999999){} primario [31,40); con ((..)) y con ( .. ) [32,41); control sin grupo [30,39). Base daba [30,41) y [30,43).
   - (n+1); [34,37). (n)=1; [34,35). if true then (n) else 2 [65,66). (x)=2 en guarda: NX-E1305/guard_mutation [53,54). (Drop)(a) en guarda: NX-E1305/guard_transfer [59,63) (base: checked/unit).
   - S5 (ADR secc. 11: agrupacion 0 ticks, nombre del destino 1): Id(1), (Id)(1), ((Id))(1), ( Id )(1) completan con N=43477 y fallan con 43478 (base con grupo: hasta 45453). Closure: primer N que falla 52631 para f(), (f)(), ((f))(), ( f )() (base 55555 con grupo).
3. Diferencial propio: PASA. Semilla 7310442, 7.240 fuentes (3.620 pares agrupado/gemelo sin grupo), SHA-256 del manifiesto (texto LF) af6c061aa56d7bfc40ea1a0492b23e333ba821cf23a241b36607bfb6d7947cf6, congelado antes de ejecutar (FREEZE.txt). 3 perfiles, check y run, release en todo y debug en todos los programas sin while y 1/3 de los while.
   - 0 diferencias base a candidato en programas sin grupo; 0 divergencias debug/release (43.020 comparaciones); 0 timeouts, panicos, internal_error, InvalidIr; 0 divergencias check/run.
   - 573 celdas agrupadas cambian: 397 solo primario, 64 salida (conteo S5 o aceptacion S7), 88 codigo (S7: NX-E0202/maybe_consumed pasa a NX-E1305/guard_transfer Drop), 24 cuyo primario es el nodo llamada (f )() (ancla de construccion mayor). En todas el candidato coincide con el gemelo salvo caracteres de grupo.
   - 2.540 sin cambio y distintas del gemelo: mismo codigo, subcausa y data; difieren texto de primario o relacionados con grupo en construcciones mayores (permitido por el addendum) y un artefacto de mi generador (a+(n+1) frente a a+n+1).
   - corpus/, corpus/objetivo/, tests/fixtures/ (217 ficheros .n), 3 perfiles, check y run, debug y release: 0 diferencias base a candidato. RD 152/152 en debug y release.
4. Pila, RD, auditor: PASA. src/check sin cambios; f5-stack65.py 5/5 y 5/5 a 65 KiB; auditor exit 0 y --self-test PASS; un .span() crudo nuevo en lower.rs y en lower_match.rs (copia) da exit 1. No ejecute f5-frame-sizes.py (types.rs identico).
5. Mutacion inversa propia: 7/7 muertas con SHA-256 restaurado. S1 por expression_statement_tick_*; S2 por while_condition_tick_*; S3 por assignment_tick_*; S4 por branch_/then_/else_operand_*; S5 por su propia regresion (grouped_call_target_*, grouped_closure_call_*) y por grouped_operands_keep_one_causal_primary (no es solo la segunda, como dice C); S6 por guard_mutation_reports_the_inner_place; S7 por guard_transfer_reports_the_inner_callee.
   Mutacion de regiones: reproducidos 13 de los 69 (M00001, 15, 27, 28, 32, 33, 35, 53, 55, 58, 59, 60, 63): mueren con el test indicado. Supervivientes sin programa: la afirmacion es falsa para 5 de 13 (motivo 1).
6. Puerta: PASA. gate.sh --verify GREEN; 2188/2188 debug y 2190/2190 release; SHA-256 de validate.sh del registro = blob = origin/main (7a1c7a19...).
7. Limites: el commit que no compila solo es 40aba0b (accepted no encontrado), no 4c734da como dice el informe de C (4c734da compila y pasa). Cada commit S1..S8 compila y pasa los tests de su familia y el auditor.

## Motivos del FAIL
1. Cinco de los 13 supervivientes registrados como sin programa si tienen programa. El plan (secc. 7) solo permite registrar los que no lo tienen; los demas deben morir con una regresion. Binario mutado frente al candidato, perfil f0-0003-b-v1 (ficheros en surv/):
   - M00061 (lower_match.rs l.225, sin frames.push del destino de Call en guarda), p61.n: fn Main(){let a=Some(1);let f=fn (){};match true{true if (if true then {(if true then {Drop(a);value:f;} else {value:f;})();value:true;} else {value:false;}) =>{},_=>{}}}. Esperado (F0-0003 secc. 3.3): check rejected NX-E1305/guard_transfer Drop. Candidato: lo da. Mutante: checked, exit 0 (ACEPTA).
   - M00038 (l.127, valor de return en guarda), p38.n: candidato 2 diagnosticos (guard_control, guard_transfer); mutante solo guard_control.
   - M00040 (l.135, valor de yield en guarda), p40.n: idem.
   - M00057 (l.215, operando de unario en guarda), p57b.n: candidato guard_transfer + guard_control; mutante solo guard_control.
   - M00006 (lower.rs l.8167, borrow false a true en sentencia de expresion), fn Main(){var a=[1,2,3];while true{a[0];}} con run: candidato R0001 primario [35,39); mutante [35,36).
   Accion: regresion que mate cada uno y corregir la lista de supervivientes (quedan M00047 y M00048 = for inalcanzable, NX-E0006 antes del lowering; M00013, M00021, M00023, M00029, M00051, M00054 sin programa hallado por mi).

## No bloqueante
- git revert individual de S1..S6 da conflictos de texto (GROUP_SITES del auditor y final de tests/i07c_group_primary.rs); resueltos a mano el auditor queda verde (probado S1: audit 0, self-test PASS). El plan admite resolucion manual.
- Identificador erroneo del commit que no compila en el informe de C (40aba0b, no 4c734da).
- M00029: sin diferencia en 40 programas con barrido de pad 0..7.
