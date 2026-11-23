# Validador D: I07c-S5a.1 (commit 2296dc8101acddcc93451527e5dff77fe2bcf30d)

VEREDICTO GLOBAL: **FAIL**

Base de comparación: git archive 2875943 (CLI propio, debug y release). Candidato: git archive 2296dc8. Evidencia y scripts en C:\nexo-work\tmp\validador-d-s5a1\ (gen.py, freeze.txt, cmp.py, diffrun.py, matrix.py, reduce.py, mut.py, repros en rep/ y red/).

## Motivos del FAIL (núcleo; repro mínima, esperado derivado del ADR, obtenido)

Regla común (ADR 4.1.2.4 y F0-0002-lagunas.md «Ancla con componente pendiente»): en una unión if sin contexto externo determinado, la primera continuación normal válida (aquí q, var q=[] = Array<b>) establece la referencia; las continuaciones anidadas posteriores se aplanan tras fijarla (no se sintetiza otra referencia dentro de ellas); la unión emite UNA sola NX-E1303, sobre la primera continuación posterior incompatible; los tipos de la C se completan al cierre con las igualdades compatibles posteriores. Una asignación posterior compatible no es una asignación rechazada.

1. Referencia de la unión tomada de la hoja anidada (primario, código y tipos erróneos). Main correcto, candidato regresiona.
   Repro (104 B, rep/R1.n): fn Main(){let p=true;let ar=[1];let r=[[[1]]];var q=[];let a0=(if p then q else (if p then 1 else r));q=[[ar]];}
   Esperado: único NX-E1303 primario [91,92) (el 1), expected_type=Array<Array<Array<I64>>>, found_type=I64, relación expected [73,74) (q). La referencia es q; 1 es la primera incompatible; r (Array^3<I64>) es compatible y q=[[ar]] concuerda.
   Obtenido (candidato): NX-E1303 primario [98,99) (el r), expected_type=I64, found_type=Array<Array<Array<I64>>>, relación [91,92): trata el 1 de la unión anidada como referencia. Main da el esperado. Misma deriva con: var q=[];let a0=(if p then q else (if p then 1 else q));q=[[ar]]; (candidato E1303 en el último q; main en el 1).

2. E1303 obligatoria perdida y NX-E0103 espurio nuevo.
   Repro (111 B, rep/R2.n): fn Main(){let p=true;let ar=[1];var r=[];var q=[];let a0=(if p then q else (if p then r else 1));q=[[ar]];}
   Esperado: único NX-E1303 primario [93,94) (el 1), Array<Array<Array<I64>>>/I64, relación expected [68,69) (q); r es compatible con la referencia y no queda irresuelto: sin NX-E0103.
   Obtenido (candidato): solo NX-E0103 [38,40) (el [] de r), sin E1303. Main da exactamente el esperado. Contradice el objetivo de S5a.1: introduce un E0103 espurio y retira una C obligatoria (4.1.2.2: solo se omite una C si el componente pendiente emite E0103 al cierre; aquí q está completo).
   Variante con índice (124 B): fn Main(){let x=1;let p=true;let ar=[1];var r=[];var q=[];let a0=((if p then q else (if p then ar[0] else r)))[x];q=[[ar]];}
   Esperado: sin NX-E0102 sobre la unión (una unión con C no aporta evidencia a su consumidor, 4.1.2.5; el resultado no es I64); main: E1303 [95,100). Obtenido: pierde la E1303 de ar[0] y emite NX-E0102/not_indexable sobre toda la unión [67,108) con found_type=I64, expected_type=domain:array.

3. Segunda NX-E1303 en una misma unión (viola «denuncia única por unión»). Es la condición de reapertura de la sección 9 del plan («un programa que bajo S5a.1 emita dos E1303 en una unión anidada con ancla pendiente»); la afirmación del plan «no existe hoy ningún programa que lo exhiba bajo V6» es falsa.
   Repro (127 B, rep/R3.n): fn Main(){let p=true;let ar=[1];let aar=[[1]];var r=[];var q=[];let a=(if p then 1 else (if p then [q,r] else ar));q=[aar];}
   Esperado (4.1.2.4: «una sola NX-E1303... las demás hojas posteriores incompatibles con la misma referencia no reciben NX-E1303 propia»): una sola NX-E1303, en [q,r] [99,104).
   Obtenido (candidato): dos, [99,104) y ar [110,112), ambas con expected_type relacionado [81,82). Main: una. Variante (rep/R3b.n): var q=[];let a0=(if p then q else (if p then 1 else 2));q=[[ar]]; (candidato: E1303 en 1 y en 2; main: solo en 1). Hay programas con dos E1303 ya en main (p. ej. var q=[];let a=(if p then 1 else (if p then (if p then q else aar) else ar));q=[aar];); esos son previos y no se imputan; los de arriba son regresión del candidato.

4. El límite declarado NO es exacto. limites-conocidos.md declara que solo cambia la relación expected_type (más E0103 espurios retirados). Los repros 1-3 cambian código, primario o número de diagnósticos con una asignación posterior COMPATIBLE (no hay asignación rechazada posterior): fuera del alcance declarado. Además generated1411, generated1681 (en la tabla del propio documento) y P4 (E1303 adicional [130,131), tipos Array^5) cambian código/primario con esperado «NO DERIVADO». El plan (sec. 10) admite solo «E0103 espurio retirado» o consecuencia derivada del ADR.

5. Adjudicación de la clase A de C inexacta. El informe de C (sec. 7) y s5a1-diferencias-adjudicadas.json afirman 56 de clase A = «solo se retira NX-E0103». Reproducido con base y candidato propios sobre esas 56 fuentes (perfil de cada fila): solo 2 de 56 cambian únicamente por E0103; en 54 cambian además el primario/código de E1303/E0102 (p. ej. fila 0: main E1303 [130,133); candidato E1303 [121,124), E0102 [144,145) y [150,151)). Derivé del ADR la fila 0 (aar fija alfa antes de las asignaciones posteriores; r=[3] es la incompatible, lagunas «Ancla con componente pendiente») y es consecuencia legítima, pero la clasificación y el JSON son falsos para 54 filas y el plan exigía enumerar cada diferencia con su cláusula. El diferencial de C no sostiene «0 sin explicar».

Regresión exigida: tests que fallen sin el arreglo para 1, 2 y 3 (los 17 tests de tests/i07c_s5_seed_consistency.rs pasan con estos defectos presentes).

## Por requisito

| Req | Veredicto | Evidencia |
|---|---|---|
| 1 Fidelidad al plan 3 | PASA con reserva | git diff 2875943 2296dc8 -- src: solo src/check/types.rs; (a) retract_unconsumed_seeds, tope por sitio solo baja, take(cap); (b) rejected_assignments y seed_cap en la compuerta de read_plan; (c) retracted = cap < origins.len() evita concrete_plan; note_assignment_verdict es inline(never); sin variables de entorno, condicionales por codigo o patron, constantes ni codigo muerto. Archivos dentro del plan. Reserva: la asercion de terminacion exigida por el plan no esta en el codigo (C la sustituye por medicion con copia instrumentada, max. 3 pasadas); andamiaje. |
| 2 Filas 8.1 y 8.2 | PASA | Derivadas del ADR de forma independiente. Fila 1: [[r],q] se acepta con q pendiente; la condicion while incumple Bool (E0102 [53,60)); q=[3] incompatible en su 3 [66,67); x I64 frente a Bool [82,83); E1303 en q [96,97); sin E0103. Fila 2: la proyeccion condicional r[1] fija Array<Array<I64>> (4.1.2.3); la hoja r [65,66) es incompatible; r=[3] incompatible en su 3 [74,75). Coinciden con lo editado; no estan ajustadas al binario. |
| 3 Regresiones del plan | PASA | A (96 B): E1303 [83,86) rel [53,54) y E0102 [92,93) rel [74,77); sin asignacion: solo E1303; orden [r] antes de aar: E1303 [74,77) y E0102 [92,93) rel [83,86); B en ambos ordenes: E1303 [63,64) + E0102 del 3; F06: E1303 [76,77) + E0102 [91,92); D-REF invalido [77,78) y [92,95), valido sin diagnosticos; var r=[];r=[r]; unico E0103 [16,18); proteccion del segundo mecanismo: una sola E1303. Corpus 27: sin E0103 espurio salvo F08 y F10 (2); F11 sin E0103. Los defectos 1-3 son otros programas. |
| 4 Limite exacto | FALLA | Motivos 3 y 4. P1, P2, w1, w3, c3 reproducidos (solo relacion, como declara); P4pos identico a main; P4, generated1411 y generated1681 con codigo o primario distinto y no derivado. |
| 5 Diferencial propio | FALLA (por 1-3) | Semilla 7391045882, N=4000, SHA-256 congelado antes de ejecutar: 60cf15df5daead8eaabe4cdca8f228febdc0e33277579b8c94800ff53da9b219. 4000 fuentes x 3 perfiles x debug/release x check/run x base/candidato: 0 panicos, internal_error, InvalidIr o cuelgues; 0 divergencias debug/release; check/run solo difieren en 30 programas aceptados que fallan en ejecucion, identicos en base y candidato. base distinto de candidato en 93 fuentes (check, f0-0002-b-v2): 8 solo E0103 retirado; 24 solo relacion o datos; 6 solo conjunto de E0103; 55 cambian codigo o primario (81 firmas reducidas en pat.txt), 5 de ellas anaden E0103. Control: sin las asignaciones posteriores las 93 coinciden. Las que derive del ADR son consecuencia legitima (var r=[];r=[[]]; mueve el E0103 al [] interior [22,24), igual que el gemelo let r=[[]]; r=[r];r=[[ar]] denuncia la RHS [r] como RD75; AssertEq([r[x],r][x],r);r=[[ar]] segundo conflicto independiente); los motivos 1-3 no lo son. No repeti corpus/, fixtures ni los 152 RD con mi bateria: me apoyo en el registro GREEN (RD 152/152 en su log). |
| 6 Campana twin | PASA (reproduce lo declarado) | Release 300000, semilla 10495373245766874321, LIST_OTHER=1, arnes del candidato: flagged=4 (idx 119045, 18584, 145869, 183938) y other idx 237611, como declara C; F10 sin cambio frente a main; ningun E0103 espurio nuevo en flagged. No repeti debug 100000. La campana compara con el gemelo concreto y no ejercita las formas 1-3, por eso no las detecta. |
| 7 Pila | PASA | /MAP y f5-frame-sizes.py (dumpbin 14.44.35207), base y candidato identicos: drive 27.536, resume_non_match 33.080, pareja 60.616 B, margen 5.944 B (0 B de crecimiento). f5-stack65 forma parte de validate.sh y el registro GREEN lo cubre. |
| 8 Mutacion inversa propia (9 mutantes, restauracion SHA-256 07959d3f..8778a) | PASA, 2 supervivientes no bloqueantes | Muertos: D1 sin retraccion (outer_index_context_makes_later_flat_array_an_ineligible_twin), D2 read_plan ignora rejected_assignments (incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin), D3 ignora seed_cap (retracted_seed_site_keeps_its_element_fixed_by_a_later_compatible_leaf), D4 concrete_plan siempre (deferred_union_..._o03 y _o07), D5 siembra ignora el tope, D7 sin clausula de crecimiento de uniones, D9 toda capa testificada. Superviviente D6 (sin la clausula failed): es el hallazgo declarado por C (M2d). Superviviente D8 (el bucle de planning ignora la senal retracted): sin programa que lo exhiba; no bloqueante. |
| 9 Auditor, RD, puerta | PASA | i07c_span_audit.py exit 0 (20/20, total 184); --self-test PASS; gate.sh 2296dc8 --verify: registro GREEN verificado; log: 2185 debug y 2187 release pasan, RD 152/152; SHA-256 de scripts/validate.sh del commit = origin/main:scripts/validate.sh = 7a1c7a19...30df. |
| 10 Honestidad del informe de C | FALLA parcial | Terminacion y F11 correctos; independencia de orden: B1, B2, F06 y M2 coinciden. Clase A inexacta (motivo 5). De la muestra pedida de al menos 20 de las 56: derive la fila 0 completa y comprobe mecanicamente que 54 cambian algo mas que E0103; no derive las demas una a una (INCONCLUSO por fila), lo que no cambia el FAIL. |

## No bloqueante

- Superviviente D8 sin programa; D6 es el hallazgo de procedencia ya declarado.
- generated1411 y P4: diagnosticos distintos no derivados; pueden alinearse al corregir 1-3.
- Asercion de terminacion del plan no anadida (hay medicion con max. 3 pasadas).
- Programas con dos E1303 en union anidada con ancla pendiente ya presentes en main (no imputables a S5a.1).

## Para el implementador

Corregir 1-3 con un mecanismo unico en la raiz y regresiones que fallen sin el arreglo; rehacer la clasificacion del diferencial (motivo 5). Indicio de causa: las asignaciones posteriores compatibles se marcan como rechazadas cuando la union deja un PendingDeferredJoin (note_assignment_verdict), lo que desactiva read_plan y deja la referencia sin su componente completado. Si 1-3 comparten causa estructural con el limite de procedencia (S5a.5), AGENTS.md pide al coordinador reducir alcance o reabrir el diseno por Flujo A en lugar de iterar.

Limpieza: clones y targets bajo C:\nexo-work eliminados; ningun proceso mio en segundo plano.
