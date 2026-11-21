# Validacion D (paso 4.2, revalidacion) de I07c-S4-r1.md enmendada (162 lineas)

## Veredicto: PASS de propuesta (con observaciones no bloqueantes)

## Exigencias del coordinador
- (a) PASA. Seccion 3/6: mantiene la salida de B1b; N(b)/N(d) se registran aparte, sin cambiar el oraculo.
- (b) PASA. Seccion 5.4 exige >= 3.000 fuentes distintas con generador, corpus, semilla y ejecutables con SHA-256 congelados antes de ejecutar, check y run, debug y release, contra be8c47c y 3100a29. Declara 3.456 fuentes (3.072 + 384 de la ampliacion por if/Index) y 41.472 invocaciones limpias en la segunda pasada, y deja registrada la primera pasada con 4 timeouts de 10 s (no la cuenta como cumplimiento). Los hashes de 3100a29 (6edf41c6..., e8aabcf7...) coinciden con los que yo calcule en C:\nexo-work\bin\3100a29.
- (c) PASA. La tabla de 5.3 tiene 20 filas (19 nuevas + check_against_context_one), con prehash 0ac0805b, SHA-256 de cada mutante, restauracion verificada antes del siguiente, regresion CLI o prueba interna nombrada, y la regla de que no se declara MUERTO sin el universo default. La nota Call/UnionCall/TryAdd/IntrinsicOne/CallTarget/ClosureCall (owner ya None en la base, sin efecto observable) esta en la tabla de familia y en 5.3.
- (d) PASA. Regresion pareada negativo/positivo con esperados transcritos de be8c47c (5.1) y prueba interna de frontera (5.2).
- Correcciones de B: incorporadas (criterio "restaura ANTES / DESPUES del consumidor" al inicio de la seccion 2; fila If/Match con la ruta por "if" y valor Array/Index/and-or; clasificacion de las filas Some/TryAdd/Intrinseco como no expuestas; matriz de 5.4 con * / %, envoltorios [ ], Some, Ok, TryAdd, AssertEq, if, Len, indexacion).

## Reproduccion independiente (mis propios generadores y semillas; copias desechables de be8c47c y 8cd5afe)
Correccion probada: solo "fusionar el hijo comparison_parent tras expr_inner/Whole y seguir con el padre" (mi blob 5110e3cc..., distinto del prototipo de C).
1. Familia dirigida con mi semilla 9137 (generador propio, producto LEFT x RIGHT x OPS x WRAP, 6x11x9x12, incluye *, /, %, <, if con Index, [r,[r]], (r or [r])): 1.300 fuentes, todas llegan a tipos, check y run, debug y release, tres ejecutables (be8c47c, 3100a29, corregido): 0 diferencias, 0 panicos/internal_error. Control de sensibilidad: el S4 original (cache de gate.sh) da 56 diferencias en 300 de esas fuentes.
2. Mi bateria general anterior: 393 fuentes fijas (corpus, fixtures, 152 RD, TD/DG, literales ADR) check y run: 0 diferencias; 4.000 fuentes de gen2 (llamadas F/G, Len, Some/Ok, TryAdd, AssertEq, if, reasignaciones) y 3.200 de gen en f0-0005: 0 diferencias; repro de la propuesta: los 3 C de la base.
3. Mutaciones inversas (5, en copia desechable, restauradas por SHA-256 5110e3cc... tras cada una y verificado): 
   - helper original S4 (blob 252c55b2...): 56 diferencias en 300 fuentes de la familia y 4 C en el repro: detectado por CLI.
   - Index selector owner Inherit->Empty (mutante 1551a5e9...): sin diferencia en la familia, pero detectado por CLI en g78-466 (ver abajo).
   - StatementSite taint Empty->Inherit (a52657e4...): detectado por CLI (104 y 288 diferencias en mis baterias).
   - valor de rama If taint Empty->Inherit (b422bf8f...): detectado por CLI (232 de 400 fuentes de la familia).
   - RHS de asignacion taint Empty->Inherit (837f5358...): detectado por CLI (16 diferencias en gen2 s77).
   Los cinco compilan y cambian el binario; ninguno se declara MUERTO aqui segun la regla del plan (solo muestra de discriminacion por CLI).

## Observaciones no bloqueantes (mejoran la regresion; no cambian el veredicto)
1. La tabla 5.3 declara "sin fuente CLI aislada conocida" para las filas Index/selector, hijo de sentencia, RHS de asignacion y valor de rama If. Mis baterias exhiben fuentes CLI para las cuatro, que C puede usar como regresion de CLI ademas de la prueba interna:
   - Index/selector (g78-466, f0-0005, base != mutante en check): fn Main(){let x=1;let p=true;let c1=[1,2];var ar=[1];var br=[true];var aar=[[1]];var q=(if true then {return;} else {return;});var r=[];((x)[p]);r=([][[true]][[r,0]]+[r][(r and q)]);(TryAdd(q,aar));r=[[1]];}
   - Sentencia (s77-9): fn F(x:view Bool)->Bool{return x;}fn G(x:view I64)->I64{return x;}fn Main(){let p=true;let q=[1];var r=[];let z0=F(Ok(r));var w1=F([r]);let z2=TryAdd(G(Len(r)),AssertEq(r,[1]));r=[[true]];}
   - RHS de asignacion (s77-26): fn F(x:view Bool)->Bool{return x;}fn G(x:view I64)->I64{return x;}fn Main(){let p=true;let q=[1];var r=[];var w0=(1 and Some(r));w0=TryAdd(F([]),TryAdd(Len(r),[1]));let z2=(r[0] or TryAdd((Len(r) and F([r])),r[0]));r=[[true]];}
   - Valor de rama If: fn Main(){var b=true;var p=1;var r=[];let z=(if p then (AssertEq(p,1) or r) else r);r=1;r=[[1]];}
   (Los programas son del perfil f0-0005-b-v1; los esperados salen de be8c47c, no del mutante.)
2. La primera pasada de C tuvo cuatro timeouts de 10 s atribuidos a carga (las 24 invocaciones aisladas terminan en 0,01-0,06 s); en mis ~16.000 invocaciones de la familia con 6 hilos no hubo ningun timeout. Aceptable como esta registrado.
3. La nota de Call/UnionCall/...: tras la correccion el padre conserva el owner heredado en esos marcos, mientras la base comparaba con None. No hay programa que lo exhiba (mis 4.000 + 1.300 fuentes con llamadas y Some/Ok no difieren), y la propuesta lo cubre con las inversiones 8 a 13 de 5.3.

## Limpieza
Copias y targets propios borrados; sin procesos propios vivos.
