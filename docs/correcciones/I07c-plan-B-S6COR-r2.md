# Validacion B, ronda 2 (dirigida) del plan I07c-S6COR-plan.md

Base: HEAD 94804e2 (types.rs blob 1050c84c..., auditor scripts/i07c_span_audit.py blob fb9815ba... verificados). CLI: release.exe de main (cache 9d7ed0a5...).
Sondas: C:\nexo-work\tmp\validador-b-cor\ (probe.py y scripts de la ronda 1; salida reproducida en esta ronda).

## Veredicto: PASS del plan, con UNA puerta externa vinculante antes de C (fe de erratas, ver 1)

## (1) Fe de erratas: PASA como declaracion; NO aplicada todavia en el addendum
- Plan 1 declara dos sustituciones exactas: (a) linea 16 -> "primarios respectivos [20,23) y [21,24) ([1])..." ; (b) linea 12: "F0-0002 13.13, Convencion de spans" -> "13.6".
- Comprobado en docs/adr/F0-0002-lagunas.md (94804e2): las cadenas a sustituir existen literalmente en la linea 12 y en la linea 16; el fichero aun NO contiene "21,24" (grep = 0). Es accion del coordinador.
- Bytes: (([1])) => [1] en [21,24), grupo exterior [19,26) (CLI); el ADR sitúa "Convencion de spans" bajo 13.6 (l.1947), 13.13 (l.2425) es presupuestos.
- Las regresiones usan solo [21,24) (fila for doble). Tension textual: el plan dice "antes del PASS de B"; mi PASS es del contenido del plan y NO autoriza a C hasta que el coordinador aplique la fe de erratas y B la cotejе (1 minuto). Sin ella, FAIL documental segun el propio plan.

## (2) NX-E0415 con closure agrupada: PASA
- Reproducido: `fn G(){Print(1);}fn Main() with no_io{let f=fn (){G();};(f)();}` -> [56,56) vacio, stage effects, operation closure.call, contract [32,37); control f() [56,57); ((f))() [56,56).
- Plan 2 (fila effects) y 3 (Efectos) incluyen CallClosure (4055-4079 / callers 4042-4087), ClosureCall en pasos, ResumeGenerator decidido por origen (Next escrito -> next_head; for sintetico lower.rs:3413-3425 con token for, verificado: Op::ResumeGenerator con for_token). find_witness_enumerated de test usa el mismo selector.
- Fila 103 con [57,58); ((f))() [58,59). Filas 101/102: ((Print))(1) [23,28), ((F))() [40,41). Aritmetica correcta; los "antes" ([21,21), [38,38), [56,56)) reproducidos.
- Instruction.span de CallDirect/CallIntrinsic/CallClosure/GeneratorNext = span de la llamada (lower.rs 9431-9539): clave (file,start,end) global coherente.

## (3) Sitios anadidos/corregidos: PASA (todos reproducidos con el CLI en F5 o J segun fila)
- (a)+=2: antes [18,21), control a+=2 [18,19), despues [19,20); (a[0])+=2: antes [20,26), control [20,24), despues [21,25); positivos (a)=2 y (a[0])=2 checked. stage lower, op ir.lower, feature implementation_limit, pending_scope L08 (verificado). Fila lower.rs 1470-1550 corregida: ya no dice "no se normaliza"; exige dos valores por candidato (clave cruda para prioridad, span normalizado solo para emitir): decision cerrada.
- E1105 argumento: F(edit a,(a)) [56,59) -> [57,58), control [56,57), positivo F(edit a,1) checked; relacion origin y paso borrow.inout [49,55] verificados. F(edit a[0],(a[0])) [61,67) -> [62,66), control [61,65), paso [51,60] verificado.
- E0111: F(1,(2)) [32,35) -> [33,34) (control [32,33), doble ((2)) [32,37) -> [34,35)); Drop(edit (a)) [30,33) -> [31,32) (control [30,31)); Print(edit (a[0])) [31,37) -> [32,36) (control [31,35)); positivos checked.
- E0111 missing_argument closure (f)() F5 [34,37) -> [35,36); ((f))() [34,39) -> [36,37); control f() [34,35); positivo (f)(1) checked.
- names.rs:1192 sin etiqueta: prueba de inalcanzabilidad correcta (sin etiquetas el slot es monotono; con etiqueta previa el posicional da positional_after_named antes; un duplicado solo es alcanzable con etiqueta y usa label_span). Verificado leyendo names.rs 1165-1200; F(x:1,x:(2)) da duplicate_argument sobre la etiqueta (no desagrupar).
- Barrido adicional de ronda 1 (~170 programas) sin sitios fuera de la tabla.

## (4) Coordinacion con S6: PASA
- Orden S5 -> S6 -> COR declarado (1); S6 reconcilia sus 12 sitios sobre base integrada; COR no tapa la puerta de S6.
- Los 12 corregidos contra S6 3.3 y el auditor real: 7082 dentro; 12977 (-> 13094) fuera y preexistente internal_result. Equivalencias verificadas linea a linea en 94804e2: 7082, 8283, 11241, 12189, 12418, 12991, 13101, 13312, 13317, 13492, 14730, 14744 (todas coinciden con la expresion esperada).
- Clase tras COR por sitio en la tabla de 3; edicion del auditor autorizada y acotada (REVIEWED solo 7082/8283/12301/14613/14627; delegados contados; exit 1 si cambia/desaparece un marcador; auto-test con caso verde de 12, caso rojo de span nuevo y reversion por sitio; 7082 exige delegacion mas bucle unico en el helper de parse.rs). El auditor real (blob fb9815ba) tiene REVIEWED por numero, --self-test y el chequeo de bucle en 7082 tal como el plan lo describe.
- K1-d2: parte de primarios con grupos pasa a evaluable; 14613 (14730) NO EVALUADO solo esa rama.

## "No bloquea" de ronda 1: incorporados
- expression_primary_span `#[inline(never)]` (3) y medicion de expr_inner, drive, resume_non_match y las cuatro sumas de ruta con scripts/f5-frame-sizes.py y f5-stack65.py (presentes en 94804e2; verificado) (5.4).
- ownership_flow.rs:537-539 y sparse.rs:608-610 (orden y dedup por primario/code/operation; verificado) en 5.1.
- Clave (file,start,end) declarada global; "por funcion" solo ambito de recorrido (3).

## No bloquea (nuevo, menor)
- La cabecera de la tabla 2 dice "lineas de b7e9b27" pero las cifras de types.rs son de 94804e2; 3 aclara las equivalencias. La fila de missing_argument cita types.rs:14627 (numero historico; vigente 14744). Cosmetico.
- 5.4 dice que f5-frame-sizes.py/f5-stack65.py "no existen en b7e9b27" y 5 intro dice que ya estan en 94804e2: coherente, pero conviene una sola frase.
