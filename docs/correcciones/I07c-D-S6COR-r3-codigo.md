# Informe de D, validacion del codigo corregido 9e91715b1b731e69b06745162066ca6fdffd980e

## VEREDICTO: PASS

(1) Defecto del FAIL anterior: CORREGIDO. CLI de base 1010938 y candidato, check y run, J (f0-0003-b-v1),
    debug y release (diferencial: 0 divergencias debug/release; v2 y F5 igual):
    `MockOpen(1);` control [21,29); `(MockOpen)(1);` intrinsic.call [22,30), cadena [intrinsic.call [22,30)];
    `((MockOpen))(1);` [23,31]; `(MockBlockingOpen)(1);` [22,38); `F((MockOpen)(1));` [51,59);
    `F(((MockOpen))(1));` [52,60); closure `let f=fn (){(MockOpen)(1);};f();`: closure.call [49,50) +
    intrinsic.call [34,42) (base [33,33)). Primario y paso coherentes, contract igual al control.
(2) Barridos: PASA. Base 1010938 vs candidato: mi barrido de familia (537), el de C (semilla 5150, 1.500, 18.000
    comparaciones, J y F5): 0 diferencias de testigo; solo primarios/pasos vacio -> nombre. Mi diferencial
    congelado (2.781 + 288 + 537 + 236 fixtures JSON + 217 .n, 3.968 fuentes unicas, corpus.jsonl con el mismo
    SHA-256 6686b417...fabe, semilla 90210417): 47.616 comparaciones, 36.464 identicas, 11.152 con movimiento
    de primario permitido (NX-E0006/E0105/E0111/E0202/E0412/E0414/E0415/E1104/E1105/E1107/E1305), 0 cambios
    de operation/cause_chain/code/subcausa/data/orden, 0 en programas sin grupo, 0 pánicos/internal_error/
    InvalidIr/cuelgues, 0 divergencias debug/release. RD 152/152 (debug y release), diferencias de snapshot 0.
(3) Fidelidad: PASA. `git diff 1552e81 9e91715 -- src` toca solo src/ir/effects.rs y es identico al mecanismo
    validado (y a mi prototipo): CallSites con segundo mapa nombre -> inicio de la llamada, order_start, usado
    en el atajo de aristas, a_offsets/b_offsets de compare_paths y find_witness_enumerated; fallback al inicio
    propio del paso; sin condicionales por codigo ni patron de fuente, sin tocar Cause/Witness/Op/JSON.
    Pruebas: 4 filas de E0415 pasan a intrinsic.call con spans del nombre ([22,30), [23,31), [51,59),
    [52,60)); filas nuevas de closure (check y run, 1 diagnostico, [49,50)+[34,42]; doble [53,54)+[35,43)),
    N1 (`move (a)` [59,60), `move ((a))` [60,61), control [58,59), reparacion valida) y N2 (Push [26,27),
    Pop [32,33), Next [60,61]; controles [25,26)/[30,31)/[59,60); reparacion `Iterator.Next(edit g)`
    aceptado; Push/Pop sin reparacion, comprobado en mi informe previo) con esperados que coinciden con la
    aritmetica de bytes y el control sin grupo, no con el binario. Dos tests unitarios nuevos en effects.rs.
(4) Mutacion inversa: PASA. Propias (parche exacto, SHA-256 inicial/mutado/restaurado verificado,
    `--profile default`): P2 vector b_offsets sin order_start: MUERTO (grouped_resource_constructor_keeps_the_witness_of_the_bare_call);
    P3 fallback `unwrap_or(0)`: MUERTO (idem y order_start_is_the_call_start_for_every_target_shape);
    P4 mapa con `call.end`: MUERTO (idem); P1 atajo de aristas con la clave del nombre: SUPERVIVIENTE
    EQUIVALENTE (entre llamadas hermanas del mismo cuerpo el orden por inicio del nombre y por inicio de la
    llamada coincide: una llamada solo contiene a otra en sus argumentos, que empiezan despues de su
    nombre; no hay programa que lo exhiba, ya anotado en el informe r1). R1-R4 de C: R1 MUERTO
    (order_start_is_the_call_start_for_every_target_shape), R2, R3 MUERTOS (grouped_resource_...), R4 MUERTO.
    N1/N2 del primer informe: M6 (pierna conflict de `move (a)`) MUERTO por i07c_group_primary::ownership_diagnostics_report_the_inner_place_or_value;
    M11 (brazo Inout intrinseco) MUERTO por call_binding_diagnostics_report_the_inner_target_and_argument.
(5) Pila: PASA. f5-frame-sizes.py (rustc 1.99.1, /MAP): base y candidato drive 27.536, resume_non_match 33.080,
    pareja 60.616, margen +5.944; expr_inner 12.752 -> 12.720; f5-stack65.py propio: control 16 KiB desborda,
    option_match126 y result_match126 65 KiB 5/5. Auditor exit 0, 14/14, sin pendiente_decision;
    --self-test PASS.
(6) Puerta: PASA. `sh scripts/gate.sh 9e91715b1b731e69b06745162066ca6fdffd980e --verify`: GREEN (arbol
    eae1ec204dd651a541ce0cdccd4108001ed0b258); log 2167/2167 debug y 2169/2169 release, f5-stack65 5/5+5/5;
    SHA-256 de scripts/validate.sh del commit = origin/main:scripts/validate.sh =
    7a1c7a199e4bcb09356bc6d56df8bb80556a5403f59a57cf502343deded130df.
(7) Fe de erratas de `move (a[0]);`: ya validada; sin cambios.

## No bloqueante
- N1/N2/N4 del informe anterior quedan cerrados (filas nuevas; los .pyc eliminados del arbol).
- El test unitario de closure en effects.rs solo comprueba que el ultimo paso es intrinsic.call; la
  igualdad completa la cubren las filas de i07c_group_primary.

Limpieza: copias y targets propios borrados; sin procesos propios vivos.
