# D1 - propuesta r1 con mecanismo r9 (HEAD 739cbe3)

Veredicto: FAIL (una objecion de nucleo al cortocircuito de lower_match.rs).

PASA: causa raiz (hallazgo C + mi repro), obligacion de patron en types.rs sin segundo solver, oraculos de F0-0003 3.2/8.2 ya validados (offsets recalculados), archivos r7 + lower_match.rs (+ i07c_tests.rs), mutaciones y diferencial (369 + fuzz) suficientes en forma. Lo del par vivo (NX-E0113) no lo he podido contradecir: el cortocircuito no toca Live.

FALLA: la condicion `selected.diverged() || (selected_ty == Ty::Never && Analyze)` omite los BRAZOS completos. En Analyze el lowering comprueba propiedad en codigo inalcanzable (corpus/fail/unreachable_resource_copy.n lo exige: NX-E0201). Repros, base 9b8e03e rechaza; con SOLO esa condicion aplicada (copia desechable, release) acepta:
 - fn Main(){let l0=(if true then {return;} else {return;});match l0{_=>{let r=MockOpen(1);let d=r;}}}
   esperado/base: NX-E0201 [94,95); obtenido: checked/unit.
 - mismo prefijo, brazo `let r=MockOpen(1);let o=move r;Inspect(r);`: base NX-E0202/definitely_consumed [109,110); obtenido: checked/unit.
(`let q:I64=true;` sigue rechazado: lo hace el checker.) No es "regresion del diferencial": las 369 fuentes no lo contienen. Viola "no cambian salidas fijadas / aceptar lo que se rechaza" y la promesa de r9 de conservar permisos del prefijo.

Accion: A/B deben rehacer el mecanismo IR para no saltar el analisis de brazos (p. ej. analizar brazos/propiedad sin construir match_tests/bind del descriptor), o justificar con ADR que el codigo de brazos tras un scrutinee Never no se analiza (hoy `match DV{...}` directo ya lo omite: incoherencia preexistente que no justifica ampliarla). Anadir los dos repros como regresion negativa aislada con positivo.
