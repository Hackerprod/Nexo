# Validador D, paso 4.2: propuesta r1 de S5a.1 (variante C2)

VEREDICTO: FAIL (la propuesta aun no esta lista para 4.3). Resuelve los defectos 1-3 de mi informe sobre 2296dc8, pero deja programas que regresionan frente a main y la lista de 59 diferencias no tiene derivacion independiente completa.

Metodo: copias de git archive de 2875943 (base), 2296dc8 (V6), V6 con r1-variante-C2.diff aplicado literalmente (c2) y V6 sin retraccion (a1, ablacion). Release, check, perfil f0-0002-b-v2. Scripts en C:\nexo-work\tmp\validador-d-s5a1\ (var.py, diffrun.py, classify.py, dupc.py, reduce.py).

## (a) Ablacion: PASA
R1, R2, R2b, R3, R3b, R4, Z1 y Z2: con la variante sin retraccion (a1) la salida es identica a main; con V6 difieren. La retraccion de capas es la causa de los defectos 1-3. Q1 y los limites P1, P2, c3, P4, w1, w3 tambien difieren en a1: ahi interviene ademas la clausula failed, como dice C.

## (b) C2 frente a mis repros y limites: PASA. Mi barrido: FALLA
- R1, R1twin, R2, R2b, R3, R3b, R3c, R4, Z1, Z2 y Q1: C2 igual a main. P1, P2, P3, P4, P4pos, c3, w1, w3: C2 igual a main. generated1411, generated1681 y G1 siguen distintos de main (G1: main emite dos E1303 en una union, C2 una, que es lo que da el ADR).
- Mi diferencial (semilla 7391045882, SHA-256 60cf15df5daead8eaabe4cdca8f228febdc0e33277579b8c94800ff53da9b219, 4000 fuentes): main distinto de V6 en 93 y de C2 en 49; C2 es subconjunto de V6. Sin las asignaciones posteriores las 49 coinciden con main. Sin panicos, internal_error ni cuelgues. No repeti debug/release ni run para C2.
- Regresiones que persisten en C2 (main distinto, esperado derivable del ADR):
  1. NX-E0103 con primario sobre un nombre y no sobre un constructor (ADR 4.1.2.2: NX-E0103 sobre el constructor completo). Main no lo emite. Repro (180 B): fn Main(){let x=1;let p=true;let ar=[1];let aar=[[1]];let br=[true];var r=[];var q=[];let a0=((if x then 1 else [r,[[r],q]])==1);let a1=[(if true then 1 else r),1][x];r=ar;}
     C2: NX-E0103 [83,85), NX-E0102 [98,99) y NX-E0103 [117,118) (el token r). Main: NX-E0103 [74,76), NX-E0103 [83,85), NX-E0102 [98,99) y NX-E0102 [116,119). Un E0103 con primario no constructor es un primario erroneo de nucleo. Frecuencia 1 en 4000 (g02426).
  2. Segunda NX-E1303 con el mismo origen en una union plana (ADR 4.1.2.4: una sola por union). Main: una. Repro reducido (rep/R5.n del directorio de trabajo): fn Main(){let x=1;let p=true;let ar=[1];let aar=[[1]];let br=[true];var r=[];var q=[];var s=[];let a0=[(if 1 then (if br[x] then [[],[q]] else (if true then aar else s)) else (if br[x] then (s<q) else (if 1 then [[r]] else 1))),1];while (if false[x] then 1 else (([r,s]==ar)==aar)) {}r=3;q=ar;}
     C2 emite E1303 en [[r]] [212,217) y en el 1 [223,224) con el mismo origen; main solo en el 1. Es la forma del defecto 3 anterior dentro de un array; no pude reducir mas ni separar las dos uniones, por eso la derivacion es de confianza media. Frecuencia 1 en 4000 (g03010).
  Con estos dos programas no se cumple el criterio del orquestador (ninguna regresion frente a main con esperado derivado). Si el coordinador los considera un limite acotado, debe acotarse por barrido exacto, no por muestra.

## (c) Las 59 diferencias de C2
- Lista: no puedo confirmar que sea exhaustiva. r1-lista-diferencias.py solo cubre las 3.674 fuentes de C; en mi corpus C2 difiere en 49 fuentes que no estan en esa poblacion (otra poblacion, no una omision del script). Comprobacion mecanica sobre las 59 filas: todas cambian primario, tipos y relaciones (58 codigo y cardinalidad); ninguna cae en mis detectores (E0103 sobre un no constructor, E1303 duplicada nueva, E1303 perdida con mas E0103).
- Derivadas o confirmadas contra el ADR (legitimas), unas 24 filas: generated:0062 (aar fija alfa antes de las asignaciones posteriores, el 3 de r=[3] es la incompatible), generated:1681 (la segunda E1303 de una misma union desaparece), A, A_reordered, B_qr, F06, M2, O01, O03, O07, O08 (tablas ya validadas) y F01, F02, F04: patron comun, la primera hoja compatible fija el componente, la asignacion tardia incompatible se denuncia en su hoja (E0102 sobre el 3) y el E0103 espurio desaparece.
- INCONCLUSAS: generated:1256 y generated:1760 (C2 retira E0103, E1303 y un E0102 de un while con contexto Bool sin sustituirlos; podria ser la omision por componente pendiente de 4.1.2.2 pero no la derive) y, sin derivar una a una, F05, F09 a F19, O04 a O06 y las permutaciones. Quedan unas 35 filas sin derivacion independiente mia; no alcanzo las 59 ni las 50 pedidas.

## (d) Que pierde C2 frente al plan
- Repro A sin asignacion tardia (90 B): C2 igual a main (NX-E0103 [30,32)); el plan exige E1303 [83,86). Confirmado; por eso falla compatible_later_assignment_leaves_only_the_union_conflict (2184/2185 segun C).
- NX-E0103 del corpus twin de 27 casos: main 36, C2 3 (F08 1, F10 2); F11 sin E0103.
- Reducir S5a.1 a C2 y dejar el resto a S5c es coherente con el plan 9 (la condicion de reapertura se cumplio con V6), siempre que el test que falla se mueva a S5c con el requisito reescrito y se cierre o acote por barrido el residuo de (b).

## (e) Viabilidad para 4.3
No viable todavia. Falta:
1. Resolver o acotar exactamente por barrido las dos regresiones de (b).
2. Derivacion independiente de las filas INCONCLUSAS y diferencial amplio de C2 (3 perfiles, debug/release, run, mi corpus).
3. Campana twin de C2 (solo se corrio C1: flagged 3, other 0).
4. Pila (se espera 0 B: no toca drive ni resume_non_match) y mutacion (M1b y M1c de V6 ya no mueren por las mismas pruebas; hace falta un mutante por la compuerta rejected_assignments y otro por la retirada de failed).
5. Decidir el destino del test que falla.

Limpieza: copias y targets borrados; ningun proceso mio en segundo plano.
