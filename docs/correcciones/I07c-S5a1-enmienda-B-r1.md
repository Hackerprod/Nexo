# Validador B: enmienda r1 al plan de I07c-S5a.1 (veredicto por asignacion)

VEREDICTO: FAIL (un defecto de nucleo reproducible en la ruta que la enmienda conserva a proposito; el resto PASA).

Metodo: binarios release de main (HEAD 516828b, types.rs sha 54c9dfed), del candidato (types.rs sin confirmar) y de una variante propia que implementa literalmente el punto 2 de la enmienda (unas 12 lineas en copia desechable). Perfil f0-0002-b-v2. Copias y targets borrados.

## Requisitos

1. Derivacion independiente: PASA. ADR F0-0002 l.301 y l.309 y el addendum dan por bytes: P1 [93,101); P2 [119,124); P3 [88,94) (el r [81,82) conserva [57,58) por testigo de contexto); P4 [165,168). Coincide con la enmienda. Binarios: P1/P2 main = [93,101)/[119,124), candidato [85,86)/[76,77); P3 main [57,58), candidato [88,94); P4 main [102,107) con Array^3 y sin E1303 en [130,131), candidato [165,168) y Array^5.
2. Laguna/Flujo A: PASA. l.301 y el addendum fijan la procedencia en P1-P4.
3. Mecanismo, V6 y pila: PASA salvo el motivo 1. Variante propia: P1/P2 = main, P3/P4 = candidato; sin failed (M2d) reproduce main en P4; clave por binding (candidato) muere en P1/P2. La clave es la misma que usa assignment_type_plan. note_assignment_verdict sigue inline(never), solo anade un parametro. Terminacion sin cambio. Pila: NO EVALUADO. Independencia de orden: PASA, 49 permutaciones sobre P1 y variantes: variante = main en todas; el candidato difiere en 3.
4. Regresiones del plan 7 y tests de 8: PASA. Diferencial propio (3.674 fuentes, 3 perfiles, check y run): variante vs candidato solo 2 diferencias (generated:1411 y 1681), como dice la enmienda. Con los tests del arbol de trabajo sobre la variante (filtro i07c_*, corpus; 245 tests): 242 pasan, incluidos los dos tests de 8 editados por C; el unico fallo (grouped_operands_keep_one_causal_primary, reviewed span changed en types.rs:7141) es la auditoria de spans por numero de linea desplazada por mis lineas, no semantico.
5. Las 4 fuentes devueltas a main: INCONCLUSO, no bloqueante. generated:1681 difiere solo en una E1303 adicional en [138,139), clase D informativa (4.1.2 N(j)). generated:1411: difieren tipos esperados y el conjunto de E0102; mi derivacion no cierra. Deben quedar fuera de tests/ y registradas en Pending.md con fecha.
6. Regresiones nuevas: PASA con salvedad. Esperados de P1-P3 verificados con la variante (check y run rechazan); positivos pareados verificados (main = variante). M2e muere con P1/P2, M2d con P4. Observacion: el positivo de P4 no es control limpio: da E0102 [166,167) en r=[q] que el programa completo no da (la asignacion rechazada cambia el tipo de r); identico en main y variante, no lo causa la enmienda; registrarlo como limite.
7. other_differences 237611: PASA (derivable). r[0] es alfa sin raiz al entrar en la union, se sintetiza sin contexto (4.7), [[2]] fija el elemento de r como Array<I64> y la union (Array<Array<I64>>) choca contra alfa como un todo: un E0102 [93,121) con relacion [86,90), como da el candidato. El gemelo concreto no es oraculo (4.1.2.7). main da cinco E0102 por hojas.

## Motivos del FAIL

1. DEFECTO DE NUCLEO (procedencia): la ruta sin entrada en assignment_type_plan, que la enmienda conserva por binding, mantiene la regresion de P1. Repro w1 (P1 con q=[[1<2]] en lugar de q=[[true]]):
   fn Main(){let ar=[1];var r=[];var q=[];let a=((if true then true else q)==ar);Print([r,q]);q=[[1<2]];r=[];q=ar;}
   Esperado (ADR l.301 y l.309, metamorfico con P1): NX-E0102/type_mismatch [108,110) ar, Array<Array<Bool>>/Array<I64>, expected_type=[93,100). Obtenido en candidato y en la variante de la enmienda: expected_type=[85,86) (el r de Print([r,q])). main da [93,100) en ese diagnostico (pero emite una E0102 espuria [87,88) sobre q por la asignacion rechazada, que candidato y variante eliminan bien). Con un q=[[true]] aceptado despues pasa igual ([85,86) en lugar de [93,102)). Otro caso: P2 con q=[[x+1]] en lugar de [[3]] (w3.nx): el E0102 de ar queda con expected_type=[76,77) en lugar de [119,126).
   Accion: la enmienda debe cubrir el sitio sin assignment_type_plan (RHS no intrinseco) o registrar la laguna con ejemplo valido e invalido; no basta decir que unificarlo exigiria otro diferencial. Sustituir la clave de esa ruta por first_assignment_rhs_starts[site] NO arregla w1 (probado): la causa no es solo la clave (quiza q=[[1<2]] queda marcada rechazada por el criterio de PendingDeferredJoin nuevo, o read_plan no se crea). A debe establecerlo y anadir w1 y w3 (con positivo pareado sin q=ar;) como regresiones.
2. (Condicion) El mecanismo unico no se sostiene con dos criterios (clave por RHS con plan, por binding sin plan); resolver 1 debe dejar uno.

## No bloqueante

- scripts/i07c_span_audit.py (anclas por linea) hay que regenerarlo tras aplicar la enmienda.
- Medir la reserva de pila al cierre del bloque.
- Registrar con fecha en Pending.md: expected_type de las E1303 de P4; el positivo de P4; generated:1411/1681 y las 2 aleatorias; P5 y [133,134) de P2.
- Apoyo: C:\nexo-work\tmp\validador-b-s5a1-enmienda\ (w1.nx, w3.nx, p4.nx, p4pos.nx, t1.nx).
