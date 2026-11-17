# I07c — validación B del plan (ronda 1)

Plan validado: docs/correcciones/I07c-plan.md (rama i07c-estructural, base e6e5f82). Solo lectura. Repros ejecutados con el binario release de D:\nexo-work\target-i07c-c0 (compilado en el ciclo 0 desde la base), check y run, perfil f0-0003-b-v1.

## Veredicto global: FAIL

El plan es sólido en estructura, pero deja decisiones abiertas y contiene una enmienda (vía b) incoherente con el catálogo. Todos los motivos son corregibles en el propio .md.

## Causas raíz y citas (§3 del plan): PASA con una imprecisión

- Correctas: finish_deferred_binary_right 9547 y bloque 9588–9617 (insert en 9608); expr_inner 8846–8873 y 8828–8893; pending_nominal_available 2866–2873; finish_statement_at_site 5695; exportación 11830–11857 y 11857–11868; resolved_join_side Never 3695; function 5636–5675; bind 3321, concrete_with_depth 3150, selection_type 3276, resolved_syntax_type 3253, unify_with_expected_origin 3724; lower.rs:7344 es exactamente el InvalidIr("unresolved checker array type").
- Imprecisa: check_against_context_one "8465". La función empieza en 8301 y la omisión por pending_domain_calls está en 8474. Heredado del ciclo 0.
- Reproducidos en la base: b1a internal_error; b2a–b2e, b2g y q4w aceptados (checked/unit); match Some([]){Some(v)=>{let a:Array<I64>=v;},_=>{}} internal_error y gemelo Some([1]) checked; g1_some84/126 checked con pila por defecto. TD01–TD03 y DG07/DG08 ya pasan en la base con sus relaciones.

## Motivos del FAIL (bloqueantes)

1. B1: el oráculo de b1a no existe y el plan no lo ve. r/b1a.n (var r=[];r=[(r==r)==true];r=[true];) es idéntico a ir1.n, es decir, es la laguna §8.3 que el plan declara sin decidir. El plan lista B1 como criterio bloqueante de S2 sin enlazarlo con §8.3. Hace falta tabla por repro: b1a, ir1, ir1c, ir1d dependen de §8.3 y exigen Flujo A antes de fijar oráculo; b1b y b1c no autorreferencian y su gemelo concreto acepta (esperado: aceptar). Evidencia: D:\nexo-work\i07c-c0\r\b1a.n, ir1.n, b1b_t.n, b1c_t.n.
2. Causa raíz de B1, B3 y [DV] vs [] está INCONCLUSO pero se afirma "familia única" y que el mecanismo cubre B3. I07c-PENDIENTE §4 exige causa raíz probada. Para B3 hay causa legible que el plan no recoge: pattern_type guarda variable Some(id) solo si selected es directamente Deferred (types.rs:9975–9985); con Some([]) el tipo es Array(Deferred), variable=None; la exportación (11846–11852) hace syntax_type(&known.ty)=None y da Unresolved; lower.rs:1708 lo convierte en Ty::Never y de ahí el InvalidIr (7344). Es un defecto de enlace de variable en el local del patrón, no una obligación de dominio pendiente; la familia única no está probada para B3. Dejar la instrumentación a S1 tras el PASS permite que la familia se contradiga con un plan ya validado; la sonda es barata (copia desechable) y debe hacerse antes de este PASS.
3. Mecanismo no verificable por D. §4 dice "se eliminan, sustituyen o reducen pending_domain_calls, conditional_result_types y equivalentes". Hay al menos 7 mapas/banderas (failed_join_exprs, deferred_exprs, divergent_exprs, context_rejections, failed_constructors, ...) y las variantes de PendingConstraint. Falta lista nominal de qué desaparece, qué queda como índice derivado sin autoridad y la comprobación estática que aplicará D; sin ella la fidelidad al plan (rutas duplicadas, código muerto) no se puede revisar.
4. Vía (b): el texto "exactamente el siguiente" contradice el catálogo y el ADR. Usa NX-E0113/cannot_infer_type, pero docs/adr/F0-CODIGOS.md:61 fija NX-E0113 con Subcause = null (cannot_infer_type no existe en el repositorio fuera del plan) y atribuye el Array/constructor Array irresuelto a NX-E0103 (ADR §4.1 y N(h)). La enmienda convierte var r=[];...Len(r) en NX-E0113, contradiciendo ese reparto, y cambia RD87(a)/(b) (aceptados). El plan solo cubre el primario. Corregir el texto o rebajarlo a esquema no vinculante.
5. Lagunas §8 y tests adr_gap_*: instrucción contradictoria a C. "No se conserva como oráculo normativo ni se cambia sin Flujo A" no dice qué hace C si el nuevo mecanismo altera la salida actual (DV[0] acepta, [DV][0] NX-E0103; unión anidada NX-E0103 + NX-E1303). Fijar: o C conserva exactamente esa salida (diferencial) y la registra, o Flujo A antes de S1. AGENTS: los adr_gap_* existentes se resuelven cuando se toca su corte, y este es su corte.
6. Los oráculos b2a–b2g no son literales del ADR (grep vacío); derivan de §4.1.2.3 + N(a) + gemelo §4.1.2.7 + pares de fondo §14.10. §2 del plan afirma "esperados transcritos del ADR". Declarar la cadena de derivación y comprobar las dos condiciones de §4.1.2.7 por repro. Reconciliar además la frase del plan "Never no borra el resultado nominal del operador restante" con la cláusula del ADR "ni una expresión Never como si continuara normalmente".
7. Contrato de cierre: criterios omitidos sin declarar. El plan no dice qué hace con contrato §3.3 (umbrales 400.000/100.000; el plan sube a 1.000.000, pero debe declarar que sustituye), §3.4 (modelo ejecutable de 100.000), §3.5 (prueba estructural de un solo punto de primario y un diagnóstico por sitio) ni la base de la población de mutantes (contrato: b506d7d; plan: no fijada).
8. Presupuesto y ubicación de campañas ambiguos. El plan dice "máximo dos rondas por bloque"; AGENTS dice por corte. Tampoco dice si el fuzz de 1.000.000 por perfil corre en Bloque I, II o ambos, ni cuál es el "final del corte" para la mutación única.
9. Parada y clasificación incoherentes sobre pila. §1 clasifica ≤204 KiB como andamiaje; §7 hace del desbordamiento de 204 KiB motivo de no cierre. El origen es la puerta operativa del coordinador de J16 (J16-r1; contrato normativo 256 KiB, F0-0003 §13.2); citar así y distinguir 256 (normativo) de 204 (operativo). El ciclo 0 no midió 204 (NO EVALUADO), correcto en el plan.
10. Herencia omitida: F0-0007. El mecanismo reescribe la zona que corrigió F0-0007 (atomicidad Result A/T01, occurs check NB-I11b-12, union_inner, tests/f0007.rs). §5 y la puerta diferencial no lo nombran.

## No bloqueante (registrable)

- Cita 8465 a 8474 (fn en 8301).
- S3 enumera cinco rutas; hay otras recursivas por profundidad de tipo en types.rs: result_shape_conflict 3613, inferred_array_depth 4414, compound_depth 4867, check_depth_now 4817, y las funciones libres syntax_type 97 y pretty 138. Usarlas como semilla de la búsqueda estática.
- La base e6e5f82 no incluye ce84ba8 (añade PENDIENTE §7, requisito de pila) ni eb0a323 (names.rs, parse.rs, tests/j16_diagnostics.rs); indicar rebase previo a C.
- scripts/i07c_verify_references.py menciona campos laterales y no está en la lista de archivos de cierre.
- Offsets de TD: partir del literal del ADR con la expansión de DV de §14.10.

## Cobertura verificada sin hallazgo

Transcripción de §4.1.1, §4.1.2.3 (cierre de dominio indeciso), N(a)–N(j), TD01–TD07, DG07/DG08 y §17.2 exacta en lo leído; herencia 2/6/1 y 56 (§11.7), 152 RD y 15/29 (HANDOFF) coherentes; vía (b) explícitamente no por defecto y condicionada a propietario y Flujo A; tres lagunas §8 bien descritas y no decididas (salvo el enlace de §8.3 con B1, motivo 1); archivos permitidos existen o son "nuevo"; el arnés i07c_typed_campaign no compila por run_scalar(&p) sin MockEnvironment (confirmado).
