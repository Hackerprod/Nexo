# Validacion B del plan I07c-S6COR-plan.md (contexto limpio)

Base inspeccionada: blobs de 4fc71ba (hashes del plan verificados; lineas citadas verificadas en types.rs 8283/12301/14613/14627,
names.rs 1051-1226, parse.rs 3010/3325, lower.rs 4810/8703/9080/9190/10515, lower_match.rs 327-377/476-480, effects.rs 73-99/753/788/1711/4079/4240).
CLI: release.exe de main (cache 9d7ed0a5...). Sondas: C:\nexo-work\tmp\validador-b-cor\ (probe.py, rows.py, wrap*.py)

## Veredicto: FAIL (4 motivos bloqueantes)

## Verificado OK
- Los "antes" de las 18 filas de la tabla 4 reproducidos con el CLI (check; run en NX-E0415); controles sin grupo y positivos pareados coinciden.
- Aritmetica de offsets del plan correcta: en (([1])) el [1] ocupa [21,24); el literal [20,23) del addendum para el doble grupo es imposible.
- Cita: "Convencion de spans" esta bajo 13.6 (l.1947 del ADR); 13.13 (l.2425) trata presupuestos. El plan acierta.
- Inconsistencia fuera de alcance (a)[0] reproducida ([35,36) frente a [34,40)); nodo Expr::Index: correcto dejarla fuera.
- Mecanismo: una funcion en parse.rs junto a Expr::span, accesible desde check/ir/effects; Checker::primary_span delega. Realista.
- Efectos laterales declarados, PARAR ante salida protegida, B_COR y renumeracion, diferencial con enumeracion, mutacion congelada
  (DIRECTORIES excluye src/frontend y el freeze hashea solo src/check src/ir: verificado), RD/TD/DG, sondas de pila, gate.

## Bloquea

### 1. Fe de erratas del addendum pendiente (precondicion del propio plan, no cumplida)
- docs/adr/F0-0002-lagunas.md (4fc71ba) sigue con [20,23) para fn Main(){for x in (([1])){}} y cita "13.13 Convencion de spans".
- Bytes: "fn Main(){" 10, "for x in " 9, ( en 19, ( en 20, [ en 21 => [1] = [21,24). CLI da [19,26) para (([1])).
- El plan (1) la condiciona "antes del PASS de B y de C". Accion: coordinador tramita la fe de erratas ([21,24); cita 13.6); B la cotejara.

### 2. Llamada a closure agrupada bajo no_io: primario vacio, y el mecanismo de efectos la omite
- Repro: fn G(){Print(1);}fn Main() with no_io{let f=fn (){G();};(f)();} (f0-0003-b-v1, check y run).
- Obtenido: NX-E0415 operation=closure.call, primario [56,56) vacio y primer paso de cause_chain closure.call [56,56) (siguen [50,51) G y [7,12) Print).
- Esperado (addendum: nunca un primario vacio por empezar la llamada en parentesis; F0-0002 8.2 pasos de llamada de cause_chain): f = [57,58) en primario y primer paso. Control f(); = [56,57).
- Causa: effects.rs:4079 aplica call_head tambien a Op::CallClosure y Op::ResumeGenerator (y 4240 en el test). El plan (3, Efectos) limita el indice
  y el selector a Op::CallDirect/Op::CallIntrinsic y deja call_head para roles no llamada escrita; CallClosure es llamada escrita y conserva el defecto.
  Incluir CallClosure (y decidir explicitamente ResumeGenerator/Next) con fila de regresion.

### 3. Sitios con primario de grupo reproducibles que el plan no lista o contradice
Esperados derivados de la regla del addendum y del control sin grupo.
- a) F5: fn Main(){var a=1;(a)+=2;} NX-E0006 (lower.rs:1494, first_f5_capability): obtenido [18,21); control a+=2 [18,19); esperado [19,20).
  Tambien fn Main(){var a=[1];(a[0])+=2;} obtenido [20,26), esperado [21,25). El plan (fila lower.rs 1470-1550) dice que una capacidad sobre
  Place entero no se normaliza: contradice la regla.
- b) NX-E1105 por conflicto de argumento: fn F(x:edit I64,y:view I64){}fn Main(){var a=1;F(edit a,(a));} obtenido [56,59), control [56,57), esperado [57,58);
  con var a=[1] y F(edit a[0],(a[0])) obtenido [61,67), esperado [62,66). Cubierto en general por check_place_access (4501-4559) pero sin fila ni literal.
- c) NX-E0111 con repro y sin fila literal: excess_positional_argument fn F(x:view I64){}fn Main(){F(1,(2));} obtenido [32,35), esperado [33,34);
  argument_mode_mismatch sobre el valor de prefijo (names.rs:1215): fn Main(){var a=[1];Drop(edit (a));} obtenido [30,33), esperado [31,32);
  fn Main(){var a=[1];Print(edit (a[0]));} obtenido [31,37), esperado [32,36).
- d) El duplicado sin etiqueta (names.rs:1192) es inalcanzable por construccion (posicional tras nombrado = positional_after_named); declararlo, no dejarlo abierto.

### 4. Coordinacion con S6 y su auditor: orden no declarado y los 12 sitios mal enumerados
- El plan fija el orden solo respecto de F5-pila-r1 y S5; S6 (tests y scripts/i07c_span_audit.py de 12 sitios, I07c-S6-plan.md 3.3) esta en curso y COR no dice si va antes o despues.
- Los 12 de S6: 7082, 8283, 10957, 12072, 12301, 12874, 12984, 13195, 13200, 13375, 14613, 14627. El plan (2, fila No aplica directamente) cuenta
  10957;12072,12874,12977,12984;13195,13200,13375 + 8283/12301/14613/14627: incluye 12977 (S6 lo excluye, es internal_result) y omite 7082.
- S6 exige el bucle while-let Expr::Group en 7082 (primary_unwrapped); el plan manda que primary_span solo delegue y que 8283/12301/14613/14627 dejen de tener
  span() crudo; S6: si cambia un sitio, exige revision => exit 1 del auditor y del test que lo invoca. El plan solo autoriza editar el auditor para los
  pendiente_decision, no 7082 ni 14627 (argument_target) ni la autoprueba.
- Accion: declarar orden S6/COR; tabla de los 12 con su clase tras COR; alcance exacto de la edicion del auditor y su autoprueba; efecto sobre K1-d2 NO EVALUADO de S6.

## No bloquea
1. Pila: 12301 esta en expr_inner (reserva ~12,7 KB, en la ruta mas profunda de F5-pila-r1); el plan solo cita drive/resume_non_match. Declarar
   expression_primary_span con inline(never) y medir tambien expr_inner; la puerta real son las sondas 65 KiB 5/5.
2. 5.1 menciona solo names::sort_diagnostics; ownership_flow.rs:537-539 y sparse.rs:608-610 tambien ordenan y deduplican por primario.
3. 3 habla de indice por funcion pero la clave (file,start,end) es global; find_witness ya recibe el ast.
4. Literales posibles para grupo doble: ((Print))(1) Print [23,28); ((F))() F [40,41); ((f))() f [58,59).
5. Barrido de ~170 programas con grupos (tipos, nombres, propiedad, efectos, match) sin otros sitios fuera de los anteriores.
