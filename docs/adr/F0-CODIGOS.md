# F0-CODIGOS — Catálogo único de diagnósticos de Nexo

Estado: catálogo integrado con F0-0002, enmendado el 2026-10-22; pendiente de revisión independiente.
Este archivo es el único registro de significado, propietario y subcausas.
Las tablas de los ADRs referencian entradas; no crean catálogos paralelos.
No modifica los diagnósticos emitidos por F0-0001 ni la base 0.2.

El registro vigente contiene **70 códigos NX-E** y siete códigos experimentales de `F0-0002-sim`; los inventarios menores en las secciones históricas describen sus fechas, no el registro actual. Las inserciones de F0-0002 enlazan su sección normativa. Los detalles del perfil se definen una sola vez en ese ADR y se incorporan aquí por referencia.

**Superficie: sujeto a F0-D15.** Grafías, tokens, ediciones y spans de ejemplos anteriores se conservan como referencias documentales sujetas a la sintaxis oficial única. No habilitan compatibilidad ni otra sintaxis activa.

## Esquema y estabilidad

1. Forma canónica: `NX-Edddd`. E identifica un error de entrada o análisis estático;
   los cuatro dígitos son un identificador permanente, no un número de línea.
   Rangos por área mantenida (no por etapa del compilador): 00 entrada/léxico/
   parser/límites; 01 nombres/tipos/acceso/propiedades de tipo; 02 propiedad;
   04 contratos de efectos y forma de sus cotas; 11 closures; 12 async;
   13 expresiones/módulos. NX-E0110 pertenece al sellado de propiedades de tipo;
   NX-E0417 a la estructura de cotas de efectos. Se conservan sus números.
   Los huecos no se asignan por inferencia ni se reutilizan.
2. Una entrada tiene una regla única o un conjunto **cerrado** de subcausas
   estables. Si la tabla enumera subcausas, el campo JSON `subcause` es
   obligatorio y contiene exactamente uno de esos nombres snake_case.
   Para entradas sin nombres enumerados se usa `subcause: null`.
   Un conjunto de cardinalidad uno también exige su nombre: NX-E1301 lleva
   siempre `subcause: "implicit_closure_body"`, nunca null.
   Añadir una subcausa requiere cambiar este catálogo; texto libre no la sustituye.
3. La notación documental `NX-E1105/active_borrow_conflict` representa dos
   campos: code = NX-E1105 y subcause = active_borrow_conflict. El sufijo no es
   parte del código. Los tests comparan ambos; nunca el mensaje traducido.
4. Propietario es quien mantiene la regla, no el único ADR que puede emitirla.
   Todos usan la misma entrada, incluidas afinidad NX-E1109 y firma async NX-E1202.
5. Salvo indicación expresa, **todos los códigos canónicos son propuestos**.
   NX-E0412 ya aparece como propuesta ilustrativa en roadmap §6.9 (línea 938);
   NX-E0413–NX-E0416 se proponen aquí, no proceden de la base.
   Ninguno de estos ADRs demuestra que el compilador ya los emita.
6. Los fallos internos del simulador/runtime no son errores estáticos del
   programa. Una transición imposible sin fixture fuente queda NO EVALUADA,
   sin inventar un NX-E para hacer pasar una prueba de modelo por compilación.

## Registro canónico

| Código | Propietario | Regla o subcausas cerradas |
|---|---|---|
| NX-E0001 | F0-0001/frontend | Token no reconocido por la gramática del corte. |
| NX-E0002 | F0-0001/frontend | Secuencia que no satisface la producción sintáctica esperada. |
| NX-E0003 | F0-0001/frontend | Presupuesto explícito de análisis excedido; budget_kind usa exclusivamente la enumeración cerrada de este catálogo. Subcause = null. |
| NX-E0004 | F0-0001/entrada | `source_unreadable`: fallo de apertura/lectura de fuente; `invalid_utf8`: bytes leídos y de tamaño permitido que no decodifican como UTF-8. |
| NX-E0005 | D13 §2, sintaxis común | Falta terminador de sentencia, declaración simple o requisito sin cuerpo; MissingTerminator reconoce la recuperación rechazada (D13 G.2). Token/span exigidos, reparación explícita y prioridad sobre errores de flujo derivados. Subcause = null. |
| NX-E0006 | F0-0002/alcance del prototipo | Construcción reconocida pero fuera del perfil experimental solicitado. Subcause = null. Exige profile_id y feature_id registrado para ese perfil; no acredita una prohibición del lenguaje completo ni una regla semántica todavía no implementada. Procedencia: [F0-0002 §§2.2, 3.4 y 14.6](F0-0002-second-slice.md). |
| NX-E0101 | F0-0001/tipos; ampliación de resolución F0-0002 | Nombre de valor no declarado, incluido un binding usado como expresión o un destino de llamada no declarado. Subcause = null. En F0-0002, use_kind distingue value y call_target; un nombre resuelto a valor no invocable usa NX-E0111/non_callable_target. Procedencia: [F0-0002 §4.5](F0-0002-second-slice.md). |
| NX-E0102 | F0-0001/tipos | `type_mismatch`: tipo incompatible; `not_indexable`: indexación no aplicable; `resource_required`: operación que requiere recurso; `field_requires_grouped_call`: miembro campo usado en forma de llamada de método x.m(); la invocación del campo se escribe (x.m)() (D13 G.1, C52d/C52c). |
| NX-E0103 | F0-0001/tipos | Elemento de array vacío no inferible sin anotación. |
| NX-E0104 | F0-0001/nombres; ampliación de declaraciones F0-0002 | Declaración duplicada en el mismo ámbito: binding, función o parámetro. Subcause = null. En F0-0002 exige declaration_kind = binding, function o parameter, span de la segunda declaración y primera declaración relacionada. Procedencia: [F0-0002 §4.5](F0-0002-second-slice.md). |
| NX-E0105 | F0-0001/acceso | Acceso de modificación solicitado sobre binding inmutable, incluida asignación, receptor inout y match inout. |
| NX-E0107 | F0-0001/números | Literal fuera del rango i64 del corte. |
| NX-E0110 | Propiedades selladas, mantenimiento D11 | Una impl o derivación de usuario intenta afirmar una propiedad/protocolo reservado al compilador (incluidos Call* y SuspendSafe). |
| NX-E0111 | F0-0002/llamadas | Asociación o destino de llamada inválidos: `missing_argument`, `excess_positional_argument`, `unknown_argument_label`, `duplicate_argument`, `positional_after_named`, `argument_mode_mismatch`, `non_callable_target`. Una incompatibilidad de tipos con asociación válida conserva NX-E0102/type_mismatch; copiar un argumento no copiable conserva NX-E0201. Procedencia: [F0-0002 §§4.2 y 13.9](F0-0002-second-slice.md). |
| NX-E0112 | F0-0002/entrada ejecutable | Entrada solicitada no disponible: `missing_main`, `invalid_main_signature`. No se emite por ausencia de main al comprobar un módulo de biblioteca. Procedencia: [F0-0002 §4.4](F0-0002-second-slice.md). |
| NX-E0113 | F0-0002/inferencia local | No existe una determinación única de los tipos requeridos por un constructor de valor del subconjunto, después de aplicar el contexto local disponible. Subcause = null. No se inventa un tipo comodín. El array vacío conserva NX-E0103 y la firma de closure conserva NX-E1111. Procedencia: [F0-0002 §§4.1.1 y 13.17](F0-0002-second-slice.md). |
| NX-E0201 | Propiedad común, mantenimiento D11 | Intento de copia lógica de valor no copiable, sea recurso, closure, futuro o agregado. |
| NX-E0202 | Propiedad común, mantenimiento D11 | Uso que exige valor disponible tras consumo: `definitely_consumed` si se consumió en todos los caminos pertinentes; `maybe_consumed` si al menos una continuación lo consumió. Transferencia, drop y extracción consumidora son orígenes, no códigos diferentes. |
| NX-E0203 | F0-0001/alcance | Limitación experimental: `unsupported_move_position`, `unsupported_resource_position`, `unsupported_resource_rebinding`, `unsupported_transfer_kind`. No acredita una prohibición del lenguaje completo. |
| NX-E0412 | Efectos comunes, mantenimiento D12 | No se verifica el contrato declarado no_alloc. Procedencia: ejemplo de roadmap §6.9. |
| NX-E0413 | Efectos comunes, mantenimiento D12 | **Propuesto:** no se verifica el contrato declarado no_block. |
| NX-E0414 | Efectos comunes, mantenimiento D12 | **Propuesto:** no se verifica el contrato declarado no_suspend. |
| NX-E0415 | Efectos comunes, mantenimiento D12 | **Propuesto:** no se verifica el contrato declarado no_io. |
| NX-E0416 | Efectos comunes, mantenimiento D12 | **Propuesto:** no se verifica el contrato declarado no_heap_alloc. |
| NX-E0417 | Gramática de efectos común, mantenimiento D11 §5.2 | `duplicate_operation`, `invalid_operation`, `duplicate_restriction`, `missing_body_operation`, `operation_list_required`: estructura contextual de WithClause inválida; `missing_with_clause`: cota Call* sin with; `empty_operation_list`: mapa with {} sin operaciones; `unrestricted_mixed`: unrestricted acompañado por una restricción distinta en su lista. Las tres últimas formas se reconocen antes de rechazarlas. Missing_with_clause y empty_operation_list preceden a ausencias de fases; invalid_operation precede a missing_body_operation; las omisiones con código específico prevalecen sobre missing_body_operation. Una repetición precede a unrestricted_mixed. |
| NX-E1101 | D11 §2 | Binding libre ausente de lista exhaustiva de capturas. |
| NX-E1102 | D11 §3 | Binding capturado más de una vez en la misma lista. |
| NX-E1104 | D11 §7 | Escape de préstamo retenido por una closure. |
| NX-E1105 | D11 §8 | `active_borrow_conflict`: acceso incompatible con préstamo vivo; `permission_escalation`: derivar permiso más fuerte del recibido; `overlapping_mutable_borrows`: crear segundo préstamo mutable solapado. |
| NX-E1107 | D11 §5 | Modo explícito de receptor insuficiente para la invocación. |
| NX-E1108 | D11 §6 | Contenido no satisface independencia transitiva exigida. |
| NX-E1109 | D11 §6, regla compartida D09 | Dominio destino incompatible con afinidad/transferencia transitiva. D12/D13 reutilizan esta entrada. |
| NX-E1110 | D11 §10, D10 | Generador F2 intenta retener una captura no propietaria. |
| NX-E1111 | D11 §1 | Inferencia local de firma de closure sin solución única. |
| NX-E1112 | D11 §5.2 | Firma pública invocable omite una cota de operación requerida por su gramática. |
| NX-E1113 | D11 §5/§5.2 | Argumento no implementa el protocolo invocable nominal exigido por la cota (por ejemplo, solo CallOnce ante Call). |
| NX-E1114 | D11 §5.2 | Cota invocable privada escrita omite call obligatorio. |
| NX-E1201 | D12 §1 | Await fuera de cuerpo async. |
| NX-E1202 | D12 §4 | Firma pública async o tipo Future<T> escrito omite una de create/resume/cancel/drop. D13 reutiliza esta entrada; async local exige solo resume con NX-E0417/missing_body_operation. |
| NX-E1204 | D12 §9 | No se deriva SuspendSafe para un campo/permiso retenido en suspensión, por incompatibilidad o contrato desconocido; property_evidence y cause_chain explican el fallo. suspension_kind = await o yield es dato, no otra regla. |
| NX-E1205 | D12 §2/§3 | Futuro/handle retiene una procedencia o grupo que no vive hasta su uso. |
| NX-E1206 | D12 §1 | Mezcla excluida de async y generadores: `async_generator_declaration`, `yield_in_async`. |
| NX-E1208 | D12 §8 | Préstamo cancelable a operación sin contrato suficiente de retiro de retención. |
| NX-E1210 | D12 §3 | Propagación ? aplicada a TaskOutcome sin tratar Completed/Cancelled. |
| NX-E1211 | D12 §5 | Violación intrínseca: destrucción automática de estado de futuro puede suspender. No requiere un with escrito por el usuario. |
| NX-E1212 | D12 §7 | request_cancel se dirige a un grupo distinto del propietario del handle. |
| NX-E1213 | D12 §1, D10 | Yield dentro de un cuerpo invocable ordinario: su invocable léxico más próximo no es gen fn ni async. Incluye closures ordinarias anidadas en gen. En async se conserva NX-E1206/yield_in_async. |
| NX-E1301 | D13 §2 | Forma corta de closure reconocida por ImplicitClosureBody y rechazada por faltar return: `implicit_closure_body`; span del operando completo y => relacionado, nunca NX-E0002 por esa forma. Una expresión final analizada como sentencia válida no es esta forma: su caída sin retorno usa NX-E1317. |
| NX-E1302 | D13 §1/§4 | `missing_else`: if expresión sin else; `missing_branch_value`: continuación sin value:; `invalid_value_target`: value: sin bloque destinatario directo; `missing_value_operand`: value: sin expresión, también por salto tras :; `non_final_branch_value`: value: no es la última sentencia directa de su rama. Value sin : es Name ordinario (D13 §1). |
| NX-E1303 | D13 §3 | Tipos de caminos normales no unificables conforme a igualdad nominal. |
| NX-E1304 | D13 §4 | `non_exhaustive`: faltan casos; `unreachable_arm`: todos los valores del patrón están cubiertos por la unión de patrones de brazos anteriores sin guarda; no exige un único brazo anterior que lo cubra (D13 §4). |
| NX-E1305 | D13 §4 | Guarda no permitida: `guard_effect`, `guard_mutation`, `guard_transfer`. |
| NX-E1307 | D13 §6 | Procedencia del valor de rama insuficiente para su destino, cuando el portador no tiene diagnóstico específico de closure/futuro. |
| NX-E1308 | D13 §9 | `private_access`: acceso fuera del dominio privado; `private_reexport`: intento de elevar visibilidad mediante pub use. |
| NX-E1309 | D13 §10 | `alias_collision`: alias local de declaraciones distintas; `duplicate_import`: mismo import repetido. |
| NX-E1310 | D13 §9 | `unexported_type`: tipo público sin ruta; `missing_provenance`: omisión de procedencia requerida en resultado público; `missing_signature_type`: tipo público omitido; `missing_effect_bound`: límite público ordinario omitido. |
| NX-E1311 | D13 §11 | `undeclared_dependency`: uso transitivo no declarado; `incompatible_versions`: revisiones incompatibles; `package_cycle`: ciclo de paquetes. |
| NX-E1312 | D13 §11/D08 | Paquete no es propietario autorizado del trait ni del tipo nominal exterior de impl. |
| NX-E1313 | D13 §11/D08 | Implementaciones canónicas potencialmente solapadas sin prueba de disjunción. |
| NX-E1314 | D13 §10/D08 | Múltiples candidatos de trait/extensión igualmente aplicables. |
| NX-E1315 | D13 §10 | Método de extensión existente pero no activado explícitamente. |
| NX-E1316 | D13 §8 | Inicializador ConstDecl de módulo no admisible como valor independiente del subconjunto comptime puro (incluidos recurso vivo o efectos). |
| NX-E1317 | D13 §1 | Función/closure ordinaria o async con resultado final distinto de unidad alcanza salida normal sin return. Excluye gen fn: -> T es Item y su final/bare return termina la secuencia (D13 §1.1). |
| NX-E1318 | D13 §6 | Duración insuficiente de préstamo en portador general/genérico, sin código específico de closure, futuro o rama aplicable. |
| NX-E1319 | D13 §6 | Retener procedencia prestada en un argumento/receptor propietario sin declarar retains: por almacenamiento directo o transitivamente mediante una llamada directa/indirecta cuyo contrato retiene (D13 §6, C47b). |
| NX-E1320 | D13 §6 | Cláusula from/result_from/retains escrita no cubre los orígenes o destino reales del cuerpo; aplica también en privado. |
| NX-E1321 | D13 §8 | `duplicate_module_file`: mismo archivo canónico declarado dos veces; `path_collision`: rutas normalizadas distintas con igual clave portable, aunque el disco las identifique; `module_file_missing`: mod sin archivo requerido. |
| NX-E1322 | D13 §10 | Método duplicado en bloques extend del mismo módulo y tipo nominal. |
| NX-E1323 | D13 §8 | `alias_cycle`: expansión circular de aliases; `constant_cycle`: dependencia circular de constantes; `recursive_value_layout`: tamaño de tipo recursivo sin indirección. |
| NX-E1324 | D13 §1.1/D10 | Return con operando dentro del cuerpo de generador; su tipo declarado es Item, no un resultado final de return. Incluye operando unidad. |

Una cota de efectos fallida conserva la causa `known_effect` o
`unknown_effect` en effect_evidence, no como código inventado. Siempre lleva
contract_origin (span de la anotación o del tipo indirecto). NX-E1211 lleva
intrinsic_rule = future_drop_no_suspend, sin fingir un span de with inexistente.
Si una misma operación viola la regla intrínseca y una anotación no_suspend,
NX-E1211 es principal y el contrato es ubicación relacionada; no hay duplicado.
Si una reserva incumple no_alloc y no_heap_alloc, NX-E0412 es principal.

Cuando concurren errores se prioriza la causa más próxima: disponibilidad del
valor antes de intentar copiarlo; permiso antes de escape; portador closure/
futuro antes de escape genérico de rama. Diagnósticos de rutas independientes
no se eliminan por esta prioridad. El dato de cadena explica el recorrido.
La conformidad del protocolo Call* se comprueba antes de sus cotas de efectos:
NX-E1113 evita una cascada por comparar operaciones de un argumento no conforme.
Las afirmaciones manuales de propiedades selladas se rechazan por NX-E0110
antes de intentar derivar sus campos o resolver métodos inexistentes.
En módulos, path_collision compara rutas portables antes de alias físicos;
duplicate_module_file precede a la duplicación de nombre que provoca la misma mod.

### Precisiones de F0-0002

Procedencia: [F0-0002 §§4–6, 13.1, 13.9 y 13.15](F0-0002-second-slice.md).

Las subcausas de NX-E1105 se reutilizan para préstamos de argumentos y représtamos directos: active_borrow_conflict, permission_escalation y overlapping_mutable_borrows. No se crean códigos distintos por no intervenir una closure.

NX-E0202 conserva definitely_consumed y maybe_consumed en ramas y retroaristas. El diagnóstico de bucle identifica el consumo y la retroarista causante; no supone una única iteración.

Para una misma sentencia ValueStmt reconocida sin operando, la prioridad es invalid_value_target, missing_value_operand, non_final_branch_value y errores de flujo derivados. En `{ value: }`, el diagnóstico principal es NX-E1302/missing_value_operand sobre value:, y no se duplica NX-E0005 por el terminador ausente de esa misma sentencia vacía. En `{ value: 1 }` corresponde NX-E0005 sobre }. La prioridad no suprime errores sintácticos independientes ni permite aceptar una recuperación.

Sin tipo esperado, la discrepancia entre tipos sintetizados de continuaciones de un if usa NX-E1303. Con un tipo esperado propagado desde retorno, anotación, asignación o parámetro, el valor incompatible usa NX-E0102/type_mismatch; no se duplica NX-E1303 por esa misma causa. Los defectos estructurales de ramas preceden a incompatibilidades derivadas de un valor inexistente.

En NX-E0111, la asociación se comprueba antes de tipos y efectos. Se informa primero el defecto encontrado en el orden escrito de argumentos; los parámetros ausentes se informan después de recorrer los argumentos presentes. Missing_argument señala el callee y relaciona el parámetro ausente; excess_positional_argument señala el argumento excedente; unknown_argument_label y duplicate_argument señalan la etiqueta; positional_after_named señala el argumento posicional; argument_mode_mismatch señala el argumento; non_callable_target señala el destino. Un destino que no resuelve nombre usa NX-E0101.

NX-E0112/missing_main usa el rango vacío de EOF y la entrada solicitada. Invalid_main_signature señala el nombre main y relaciona su firma.

NX-E0113 señala el constructor cuyo tipo permanece indeterminado y enumera los componentes no resueltos. Una anotación candidata solo se anuncia aplicable después de recomprobar el archivo completo.

Para comparaciones de F0-0002, tipos concretos de operandos distintos producen NX-E0102/type_mismatch sobre el operando derecho. Si los tipos coinciden pero el operador no los admite, el span es el operador. No se intenta copiar el operando para producir antes NX-E0201. El dominio exacto se fija en F0-0002 §4.6.

DG06–DG08 fijan sus primarios, relaciones y cadenas literalmente en F0-0002 §14.11. DG04/DG07/DG08 divergen del código actual: disponibilidad causal y origen del tipo esperado deben corregirse, respectivamente, en I07b e I07c. DG05/DG06 se cierran con propiedad en I07b; no se pospone esta obligación a cuotas de ejecución.

### Feature IDs de NX-E0006 para f0-0002-b-v2

El conjunto cerrado contiene **30 feature_id**: 29 exclusiones de alcance de B y `implementation_limit`, frontera temporal de capacidades pendientes. Todos conservan `subcause = null`. La tabla enumera los identificadores en orden por bytes. Procedencia: [F0-D15 §§3.6, 6.1, 6.5 y 7.1](F0-D15-sintaxis-oficial.md).

Procedencia: [F0-0002 §§2.2, 3.4, 6.6, 13.6 y 14.6](F0-0002-second-slice.md).

Los feature_id siguientes son identificadores cerrados del perfil, no subcausas de NX-E0006. Los perfiles reducidos publicados en F0-0002 §11 heredan esta misma lista.

| feature_id | Construcción diferida o capacidad pendiente |
|---|---|
| async | Async, await y tipos u operaciones reconocidos de tareas/futuros. |
| attributes | Atributos, incluido @derive. |
| closure | Literal de closure y sus capturas. |
| compound_assignment | Asignación compuesta, incluido +=. |
| comptime | Evaluación o constantes de módulo mediante comptime. |
| default_argument | Valor por defecto de parámetro. |
| extensions | Extend, use extension o llamada cualificada de extensión. |
| float_literal | Literal decimal reconocido; semántica flotante no implementada. |
| for_iteration | For y su protocolo de iteración. |
| function_value | Uso almacenado, retornado o pasado de una función como valor. |
| generator | Gen fn, yield y operaciones de generador. |
| implementation_limit | Capacidad pendiente de implementación en I08, I09, I10a, I10b o I11a; frontera sobre AST/tipos resueltos, no prohibición del lenguaje (F0-D15 §7.1). |
| inferred_non_unit_result | Función sin anotación de resultado que requiere inferir un resultado distinto de unidad. |
| loop_control | Break o continue. |
| match | Selección por patrones; `pending_scope = F0-0003` identifica el diseño de la selección necesaria para `next`/`Option`, sin prometer toda la selección por patrones de la base en ese corte. |
| methods | Selección o invocación de métodos/campos fuera de las proyecciones indexadas del perfil; `self` como expresión de receptor reconocida, con span de ese token y etapa `parse`. En posición que exige Name, `self` conserva NX-E0002. |
| modules_visibility | Mod, use, pub o rutas de módulos adicionales fuera del contexto de tipo descrito abajo. |
| no_heap_alloc | Contrato reconocido, fuera del conjunto implementado por este perfil. |
| option | Tipo o constructores intrínsecos de Option. |
| partial_transfer | Transferencia con `move` o destrucción con `Drop` de una proyección de un propietario existente, en vez de un binding completo (F0-0002 §6.6; F0-D15 §§6.1 y 6.5). |
| provenance_clauses | From, result_from o retains escritos. |
| range | Rango reconocido mediante `..<` o `..=`, sin ejecución de rangos en el perfil. |
| receiver_mode | Prefijo `edit`/`move` aplicado al receptor de una llamada completa. |
| region | Regiones y su grupo de tareas. |
| resource_array | Array cuyo elemento contiene transitivamente un recurso. |
| text | Cadenas e interpolación. |
| traits_generics | Traits, impl, genéricos, where o despacho dinámico. |
| tuple | Tipo, expresión o patrón de producto distinto de unidad/agrupación. |
| unsupported_type | Ruta de tipo reconocida que no pertenece a la lista aceptada ni tiene una exclusión más específica; incluye `U32`, `U64`, `F32`, `String` y `app.Error`. |
| user_types | Struct, enum o alias declarado por el usuario. |

`indirect_call` se retira de la lista emitible propuesta para este perfil. No existe un valor invocable aceptado que permita aislar esa causa: la introducción de ese valor falla antes por closure o function_value. Un valor aceptado no invocable usa NX-E0111/non_callable_target. No se exige una fixture imposible ni se acredita soporte indirecto.

El span señala el introductor reconocido, con estas precisiones: inferred_non_unit_result señala el nombre de la función y relaciona el retorno no unitario; resource_array señala la construcción completa; partial_transfer señala la proyección; unsupported_type señala la ruta de tipo completa. En contexto de tipo, app.Error usa unsupported_type, no modules_visibility ni NX-E0101.

La forma documental es `code = NX-E0006, subcause = null, feature_id = nombre`. No se escribe `NX-E0006/nombre`, pues la barra queda reservada a subcausas del catálogo. Explanation_id permanece NX-E0006; feature_id selecciona el detalle de alcance.

Una fila de alcance necesita un positivo que elimine o sustituya explícitamente la característica. F0-0002 §14.6 publica esos pares con spans. Acreditan el diagnóstico de perfil, no la semántica de la característica diferida.

### Destrucción estática de Result

Procedencia: [F0-0002 §8.5](F0-0002-second-slice.md).

El resumen estático de drop sobre Result<T,E> une los efectos de destrucción de ambas variantes, conocidos y desconocidos, salvo evidencia explícita de variante única. La ejecución destruye solo el payload activo. Esa evidencia no altera la copiabilidad del tipo completo.


### Asociación y etapas de F0-0002

Procedencia: [F0-0002 §§3.5–3.6 y 13.9](F0-0002-second-slice.md).

En una llamada se emite únicamente el primer defecto de asociación encontrado al recorrer los argumentos escritos. Después de ese defecto no se emiten parámetros ausentes derivados ni errores de tipos de la misma llamada. Solo una asociación válida de todos los argumentos presentes permite buscar el primer parámetro ausente por orden de declaración.

Por tanto, F2-S03 contiene únicamente unknown_argument_label y F2-S05 únicamente positional_after_named. Llamadas independientes pueden producir varios diagnósticos de la etapa arguments.

La lista y el orden entre etapas se fijan en F0-0002 §13.9: una etapa con errores impide las posteriores; los errores independientes de una etapa semántica se conservan, evitando cascadas. Los formatos y nombres JSON exactos se fijan en §§13.6–13.8.

Los tokens `*` y `!=` en posición de operador binario usan NX-E0002, no NX-E0006. Exigen parser_context = unsupported_binary_operator, span del operador y expected_tokens = ["+", "-", "<", "=="]. Esa causa precede al terminador ausente derivado. La lista de feature_id permanece en 30 entradas.


### Operadores y proyecciones de F0-0002

Procedencia: [F0-0002 §§3.6, 6.6 y 13.12](F0-0002-second-slice.md).

La exclusión binaria mediante NX-E0002 incluye exactamente `*`, `!=`, `>`, `<=` y `>=`, con `parser_context = unsupported_binary_operator` y `expected_tokens = ["+","-","<","=="]`. Precede a NX-E0005 derivado y no emite una reparación de terminador. El cierre de tipos seguido de `=` conserva la separación contextual de `>=` descrita en F0-0002 §3.6.

`partial_transfer` identifica transferencia o destrucción parcial según su entrada anterior. `consume a[0]` conserva etapa `parse`; `drop(a[0])`, una vez resuelto el intrínseco, usa etapa `arguments`. Ambos señalan la proyección y relacionan el prefijo o destino correspondiente. No se añade un feature_id.

Un prefijo `consume` o `inout` cuyo operando no es Place usa NX-E0002 con `parser_context = consume_place` o `inout_place` y `expected_tokens = ["Place"]`. Los receptores prefijados de llamada conservan primero su exclusión `receiver_mode`.

La tabla normativa de etapas para todos los códigos, subcausas y características emitibles es F0-0002 §13.12. Las demás entradas del catálogo no se convierten en emisiones del perfil.

Reutiliza NX-E1105 para préstamos directos, NX-E0202 para disponibilidad por caminos, NX-E0412–NX-E0415 para contratos y NX-E1302/NX-E1317 para valores y retornos.

La prioridad NX-E0105/NX-E0202 es causal: se suprime solo el diagnóstico derivado de indisponibilidad y se conservan fallos independientes. La reinicialización completa de `var` no lee el valor consumido. Se suprime NX-E0202 derivada únicamente de exigir un valor inexistente por Never en un uso hipotético; se conservan consumos y errores independientes (§§6.2 y 13.9). Las variables diferidas son monomórficas y conservan relaciones entre bindings copiados; Never, incógnita y contaminación son estados diferentes y nunca aparecen como tipos en `data`. Procedencia: [F0-0002 §§4.1.1, 13.9, 13.17 y 14.10–14.11](F0-0002-second-slice.md).

### Presupuestos cerrados de NX-E0003

NX-E0003 tiene una sola regla: exceder un límite explícito de análisis.
Subcause es null; **budget_kind es una enumeración cerrada**, no texto libre
ni una extensión tácita del catálogo. Son los únicos valores admitidos:

| budget_kind | Contabilidad y procedencia | Umbral del corte / estado |
|---|---|---|
| source_bytes | Bytes de fuente antes de decodificar (CLI) o analizar. | F0-0001: 1.048.576 bytes. |
| syntax_nesting | Anidamiento de llamadas de entrada del parser. | F0-0001: 128. |
| addition_chain_length | Operadores + de una cadena. | F0-0001: 128; guarda inalcanzable con el AST actual, véase K20. |
| index_chain_length | Selectores [] de una cadena. | F0-0001: 128; guarda inalcanzable con el AST actual, véase K21. |
| expression_tree_depth | Altura compuesta del AST de expresión. | F0-0001: 128. |
| array_type_depth | Profundidad inferida de Array a través de bindings. | F0-0001: 128. |
| comptime_steps | Pasos del evaluador puro D03. | Propuesto; perfil explícito del futuro evaluador. |
| comptime_depth | Profundidad de evaluación D03. | Propuesto; perfil explícito del futuro evaluador. |
| comptime_memory_bytes | Bytes contabilizados por el evaluador D03. | Propuesto; perfil explícito del futuro evaluador. |
| type_solver_steps | Pasos de inferencia/unificación local de tipos. | Propuesto para D11/D13; perfil del solucionador. |
| effect_fixpoint_steps | Pasos del punto fijo de resúmenes de efectos. | Propuesto para D12; perfil del solucionador. |
| trait_solver_steps | Pasos de requisitos/coherencia de traits. | Propuesto para D08/D11/D13; perfil del solucionador. |
| generic_instantiations | Instancias genéricas durante especialización. | Propuesto para D08; perfil de instanciación. |
| value_type_depth | Máximo número de envoltorios Array/Result en un camino de tipo, incluidas inferencias a través de bindings y firmas. | F0-0002 v2: 128. Procedencia: F0-0002 §11.1. |
| function_count | Declaraciones de función reconocidas en el archivo. | F0-0002 v2: 1.024. Procedencia: F0-0002 §11.1. |
| ownership_flow_steps | Unidades dispersas de resumen, transferencia, diagnóstico y unión por diferencias. | F0-0002 v2: 1.000.000 por función y 4.000.000 globales; §§11.1 y 11.8.B. |
| ir_items | Registros de función, bloque, slot, instrucción, operando, préstamo y limpieza insertados. | F0-0002 v2: 262.144. Procedencia: F0-0002 §11.1. |

Limit, observed y profile_id son datos obligatorios; sus valores concretos
no crean subcausas. Se debe publicar el perfil antes de ejecutar el caso.
Los siete presupuestos de la fila histórica K19 no acreditaban un evaluador/solver existente en F0-0001. F0-0002 concreta type_solver_steps y effect_fixpoint_steps en §§11.1 y 11.8; los otros cinco conservan su pendiente. Registrar otro
budget_kind requiere modificar este catálogo. El presupuesto de ejecución
del simulador **no** se mezcla aquí: mantiene R0001. Los tipos desconocidos
no se aceptan como seguros al agotar ningún presupuesto.

### Perfil de análisis de F0-0002

Procedencia normativa: [F0-0002 §§11.1, 11.4–11.5, 11.8 y 13.12–13.13](F0-0002-second-slice.md). El perfil vigente es `f0-0002-v2`. Sustituye la contabilidad densa de propiedad del antecedente v1; no crea un dialecto. Las unidades, recorrido por prioridad de índice con punto fijo local (equivalente a RPO solo en los CFG estructurados descritos), reinicios por identidad de función y acumulación global se definen exclusivamente en §11.1. Tipos y efectos conservan 1.000.000 unidades globales; IR, 262.144. Propiedad tiene 1.000.000 locales y 4.000.000 globales. Las guardas estructurales conservan los límites de esa tabla, con altura compartida por parser/lowering.

NX-E0003 conserva `subcause=null`, `operation=analysis.limit`, `limit`, `observed` y `profile_id`. El perfil incorpora `budget_scope=function|compilation`, `incomplete_analysis` igual a `stage` y `analysis_function` exclusivamente en propiedad local. No se añade un budget_kind para cada ámbito. El intento excedido es `observed=limit+1`; si coinciden límites local y global prevalece el global. Se conservan diagnósticos de estados convergidos —el prefijo estable definido en F0-0002 §11.1 si se corta el punto fijo—, una entrada por uso, como subconjunto exacto de la ejecución sin cuota; se añade el límite y se ordena por span. El primario de un límite local es no vacío y está dentro de la función identificada por analysis_function, en el cargo rechazado; B-D1 exige además el sufijo anidado y la lista ordenada de F0-0002 §11.8.B. Tras límite local se analizan funciones posteriores dentro del saldo global. No se entrega artefacto verificado con análisis incompleto.

Los perfiles reducidos y generadores de §11.4 son obligatorios: `f0-0002-test-types-32-v2`, `f0-0002-test-flow-32-v2`, `f0-0002-test-flow-global-32-v2`, `f0-0002-test-effects-32-v2` y `f0-0002-test-ir-32-v2`. El positivo mínimo es `fn main(){}`; los negativos aislantes usan 64 bindings anotados, 64 inicializaciones, 64 llamadas directas o 64 bindings con slots, según la guarda. El perfil flow local reduce solo su límite por función; flow-global reduce solo el global. Fronteras 31/32/33 del contador y alcanzabilidad desde fuente se informan separadas: un fallo por otra guarda no satisface la fila.

Las familias normales son las de §11.8: A, 93.307 bytes, excede tipos; C, 18.240 bytes, excede efectos; D, 840.882 bytes, excede IR. La antigua fuente B de 23.491 bytes ahora debe aceptarse (B-A1); B-A2–B-A4 son positivos adicionales y B-L1/B-G1/B-D1 prueban límite local, global y conservación, con tamaños y esperados en §11.8.B. Se elimina la obligación de cobrar todos los componentes visibles y la garantía indiscriminada por tamaño de fuente. Las expectativas normativas se distinguen de las mediciones explícitas de B-D1 en §11.8.B: 71.843/71.880 bytes, 61.407 ítems (61.421 contando la IR parcial de later) y rechazo del cargo de propiedad 1.000.001; no acreditan implementación de los metadatos v2.

B-G1 concatena 14 funciones N más main: 168.813 bytes, 162.309 ítems de IR y 4.282.686 unidades de propiedad en el recorrido completo medido. El arnés de F0-0002 §11.8.B baja toda la IR construible antes de propiedad, informa ir_items acumulado y usa la convención de operandos allí declarada, también para B-D1. El cargo global 4.000.001 se rechaza en f13 sin agotar la cuota local ni ir_items; la cuota global instrumentada no acredita su implementación en el compilador. El propietario de esa obligación es I10a (§17.2).

Para `source_bytes`, `syntax_nesting`, `function_count`, `array_type_depth`, `value_type_depth` y `expression_tree_depth` se compara span exacto. Para los cuatro presupuestos costosos se comparan código, etapa y todos los datos de presupuesto, y se exige un primario del archivo, válido y no vacío, con fronteras UTF-8; no igualdad de offsets entre modelo y builds. Esa excepción no cambia unidades ni alcanzabilidad. Source_bytes usa `[limit,limit+1)` antes de decodificar; syntax_nesting cuenta Block/Expr/Type; expression_tree_depth usa la misma altura en parser/lowering y las fronteras 127/128 de §13.13.

Si coinciden array_type_depth y value_type_depth prevalece Array. Para aislar profundidad de valor se parte de Result y se añaden 128 envoltorios Array mediante bindings. Addition_chain_length e index_chain_length siguen históricos sin emisiones independientes. La implementación completa de cuotas globales corresponde a I10a.

### Contrato de negativos léxicos y sintácticos

Todo caso cuyo esperado sea NX-E0001/NX-E0002 exige además etapa, archivo,
primary_span UTF-8 [inicio, fin), unexpected_token (o bytes/carácter inválido),
expected_tokens y evidencia del prefijo reconocido. Un fallo anterior,
aunque conserve el código genérico, **no satisface la fixture**. El positivo
pareado debe reconocer ese mismo prefijo y completar el análisis. Las filas
S37/S37c/S37d/S56e/S57, S27l/S27v/S31/S32/S32m/S39/S33m/S44 y K01/K02
concretan las ubicaciones, sin depender
del texto traducido. NX-E0005 tiene la misma disciplina y mayor especificidad
para el terminador ausente: no se sustituye por NX-E0002.
En NX-E0005, expected_tokens = [";"] y expected_tokens_kind =
canonical_insertion identifican el token canónico de reparación, **no** una
lista exhaustiva que excluya los saltos insertables de D13 §2/§2.1. En los
casos genéricos que enumeran tokens admisibles se usa expected_tokens_kind =
admissible_set. NX-E1301 es un diagnóstico específico sobre una forma
reconocida: S25/S03 fijan span del operando y ubicación relacionada de =>.

## Conciliación con F0-0001 y números retirados

La búsqueda de esta iteración se hizo sobre los archivos completos de src/
y tests/, contrastados con docs/validation/F0-0001-adversarial.py, mediante:

```text
rg -n '[ER][0-9]{4}' src tests docs/validation/F0-0001-adversarial.py
```

Inventario de emisiones propias de F0-0001: **13 códigos E y cinco códigos R**.
Se inspeccionaron las llamadas a Diagnostic::new y eprintln de los cinco
archivos src, no solo las expectativas de tests ni este ADR. No hay E0106.
Diagnostic::new acepta una cadena suministrada por su cliente; los códigos
arbitrarios que otro cliente pudiera construir no son emisiones del prototipo.
La CLI también escribe ayuda de uso y eventos mock-resource sin código de
diagnóstico; exit status 1/2 no son identificadores de regla.

F0-0001 conserva estos códigos sin prefijo. No se cambian corpus ni resultados
retrospectivamente. Una herramienta que compare generaciones debe leer
diagnostic_namespace = F0-0001 o Nexo-proposed-v1 y conservar legacy_code;
nunca reconoce la causa por traducción del mensaje. Este mapa es documental
para una migración futura, no un renombrado implementado.

| Código observado en F0-0001 | Correspondencia propuesta | Condición exacta y origen |
|---|---|---|
| E0001 | NX-E0001 | syntax::lex: carácter no admitido fuera de comentario. |
| E0002 | NX-E0002 | syntax::Parser: producción esperada incumplida; incluye sentencia/expresión no admitida, nombre reservado, token faltante o contenido tras la única fn main del corte. |
| E0003 | NX-E0003 | Fuente mayor de 1.048.576 bytes (main o syntax::parse); anidamiento del parser, altura compuesta de AST o profundidad inferida de Array mayor de 128. Las guardas de cadena aditiva/indexada también existen, pero son inalcanzables antes del fallo por altura; no son dos emisiones observables adicionales. Los seis budget_kind conservan el mapa de guardas fuente, cuatro alcanzables y dos latentes (K20/K21). |
| E0004, apertura/lectura | NX-E0004/source_unreadable | main.rs:19: File::open o read_to_end devuelve Err; exit 2. No requiere bytes inválidos. tests/cli.rs:61 comprueba archivo ausente. |
| E0004, decodificación | NX-E0004/invalid_utf8 | main.rs:27 llama String::from_utf8; el brazo Err está en :29 y emite en :30, después de lectura correcta y tamaño admisible; exit 2. adversarial.py:85 usa bytes fn main(){}// seguidos de FF. |
| E0101 | NX-E0101 | check::find: binding no hallado en ámbitos visibles. |
| E0102, expect | NX-E0102/type_mismatch | check::expect: tipo encontrado distinto del requerido; incluye elementos heterogéneos, argumentos de + e índice no i64. |
| E0102, indexación | NX-E0102/not_indexable | check::expr sobre no-array o check::block al agotar dimensiones del destino antes de agotar índices. |
| E0102, inspect | NX-E0102/resource_required | check::block: inspect de un binding disponible que no es Resource. |
| E0102, consume | NX-E0203/unsupported_transfer_kind | check::consume, llamado por move/drop sobre datos: restricción experimental a Resource. D11 admite transferencia/destrucción de otros valores; no migrar esta limitación como prohibición general de tipos. |
| E0103 | NX-E0103 | check::expr: literal array sin primer elemento; anotaciones no soportadas en el corte. |
| E0104 | NX-E0104 | check::block: otro binding del mismo nombre en el ámbito actual. |
| E0105 | NX-E0105 | check::block: asignación cuyo binding raíz no es mutable. |
| E0107 | NX-E0107 | syntax::atom: literal positivo o negativo no representable como i64. |
| E0201 | NX-E0201 | check::expr: nombre Resource en contexto de copia de expresión; no cambia por envolverlo en array. |
| E0202 | NX-E0202/definitely_consumed | check::get: binding marcado no vivo por move/drop anterior. maybe_consumed es ampliación por ramas, sin evidencia en el checker secuencial. |
| E0203, move | NX-E0203/unsupported_move_position | check::expr: move fuera del inicializador completo de un binding. |
| E0203, mock_open | NX-E0203/unsupported_resource_position | check::expr: mock_open fuera del inicializador completo; recurso anidado aún no modelado. |
| E0203, reasignación | NX-E0203/unsupported_resource_rebinding | check::block: destino mutable Resource; antes de comprobar su RHS. |

Para entradas demasiado grandes, main comprueba tamaño **antes** de decodificar:
incluso si la lectura acotada corta UTF-8, emite E0003, no E0004/invalid_utf8.
No se inventa un span de texto decodificado para source_unreadable; primary_span
es null y se conserva input_path. Para invalid_utf8 se identifica el rango
de bytes inválido en el archivo, sin línea/columna calculada sobre texto válido.

Registro de la familia experimental R; propietario único **F0-0001/runtime**.
No se asigna destino NX-E: son fallos dinámicos del intérprete de pruebas,
no errores estáticos ni implementación de Result/panic/cancelación de Nexo.
Se conservan exactamente bajo diagnostic_namespace = F0-0001, con estas
condiciones; así ningún código emitido queda fuera del mapa:

| Código conservado | Condición exacta de emisión | Evidencia fuente/prueba |
|---|---|---|
| R0001 | Machine::charge no puede restar amount del presupuesto restante de 1.000.000 unidades. Limpieza no se interrumpe. | runtime.rs:66; corpus.rs:194. |
| R0101 | checked_add de i64 devuelve None por desbordamiento. | runtime.rs:112; corpus.rs:70,84–85,173–177. |
| R0102 | Índice leído/asignado no convertible a usize o fuera de la longitud del array. | runtime.rs:126,130,217,223; corpus.rs:82–83,95. |
| R0201 | assert_eq recibe dos datos comprobados del mismo tipo que difieren en ejecución. | runtime.rs:233; adversarial.py:30. |
| R0301 | Se ejecuta la sentencia experimental fail(). | runtime.rs:259; corpus.rs:76. |

### Runtime experimental de F0-0002

Procedencia: [F0-0002 §§7.2, 11.2–11.7, 12.4 y 13.5](F0-0002-second-slice.md).

No se modifican las condiciones ni los resultados históricos de F0-0001.

Diagnostic_namespace = F0-0002-sim identifica los fallos experimentales del nuevo intérprete. Se conservan R0001, R0101, R0102, R0201 y R0301 con su regla: agotamiento de trabajo, desbordamiento de suma, índice inválido, aserción fallida y fail simulado. El perfil determina la nueva contabilidad de trabajo.

Se añaden:

| Código | Propietario | Condición exacta |
|---|---|---|
| R0002 | F0-0002/interp | Se excede una cuota estructural de ejecución. runtime_budget_kind pertenece exclusivamente a call_frames, live_value_cells u observation_bytes. Exige limit, observed, profile_id y span de la operación desencadenante. No utiliza budget_kind. |
| R0103 | F0-0002/interp | La resta comprobada de dos i64 no es representable como i64. No se realiza wrapping. |

Los límites de `f0-0002-v2` son 1.000.000 unidades de trabajo, 256 frames, 65.536 celdas vivas y 65.536 bytes de observaciones comprometidos. Se cuenta salida más eventos, incluidos prefijos y LF, mediante la codificación virtual de F0-0002 §12.4; no se cuenta el envoltorio JSON de transporte.

La ocupación de observaciones suma bytes emitidos y reservas de cierre. Antes de adquirir un recurso se reserva el tamaño exacto de su cierre. La limpieza no se interrumpe por agotamiento de trabajo ni omite los eventos reservados.

Observed es el cargo u ocupación propuestos que excederían el límite. Una reserva rechazada no modifica la ocupación comprometida. El primer cargo excedido de trabajo tiene observed = limit + 1; una reserva estructural agregada puede excederlo en más de uno.

Se publican estos perfiles adicionales, que heredan todo lo demás de `f0-0002-v2`:

| profile_id | Único cambio |
|---|---|
| f0-0002-test-frames-8-v2 | call_frames = 8. |
| f0-0002-test-cells-64-v2 | live_value_cells = 64. |
| f0-0002-test-observations-64-v2 | observation_bytes = 64. |

F0-0002 §11 publica recetas aislantes para cada cuota normal y fronteras en los perfiles reducidos. La duplicación histórica de arrays migra a R0002 con runtime_budget_kind = live_value_cells. Una prueba separada de bucle sin crecimiento conserva el objetivo R0001 y limpieza del recurso.

Un Err(E) producido o propagado por el programa no es un diagnóstico R ni NX-E. Main devolviendo Err(i64) termina con salida 1 y outcome.kind = err. Try_add devuelve Err(30) por desbordamiento y no R0101. El operador ? no captura fallos del simulador.

Una IR inválida o transición imposible se informa como fallo interno de implementación, sin asignarle un NX-E o R de programa.

### Limitaciones históricas que deja de emitir F0-0002

Procedencia: [F0-0002 §§13.2–13.5](F0-0002-second-slice.md).

NX-E0203 permanece en el catálogo con sus cuatro subcausas históricas, pero no se emite bajo los perfiles de F0-0002. F0-D15 §§6.1 y 6.5 no eliminan ni renombran estas subcausas; en explain conservan `availability = historical` y `example_profile_id = null`. F2-F01 deja de ser negativa porque `move` es oficial en B. Procedencia: [F0-D15 §§6.1, 6.4–6.5 y 7.3](F0-D15-sintaxis-oficial.md).

| Subcausa histórica | Destino en F0-0002 |
|---|---|
| unsupported_move_position | Subcausa histórica no emitible. El binding completo se transfiere con `move` en contextos propietarios; proyecciones y receptores prefijados conservan partial_transfer y receiver_mode (F0-D15 §6.5). |
| unsupported_resource_position | Temporales, argumentos, retornos y Result con recursos pasan a estar admitidos. Arrays que contienen recursos usan NX-E0006, feature_id = resource_array. |
| unsupported_resource_rebinding | Retirada: se permite reasignación completa de var con limpieza. Un binding inmutable conserva NX-E0105. |
| unsupported_transfer_kind | Subcausa histórica no emitible: `move` y `Drop` de datos propietarios están permitidos en B (F0-D15 §§6.1 y 6.5). |

La restricción E0002 «única fn main» también se retira del perfil nuevo. Varias funciones son válidas; check admite biblioteca sin main; run usa NX-E0112 para su entrada. La sintaxis inválida conserva NX-E0002, los terminadores reconocidos NX-E0005 y las características reconocidas diferidas NX-E0006.

F0-0002 §13 conserva una tabla de los 13 E y cinco R y la migración de los 15 archivos del corpus, incluidos los pasivos con move, nested_resource.nx y use_after_move.nx. No se modifica retrospectivamente ninguna expectativa de F0-0001.


### Trabajo y lotes de F0-0002

Procedencia: [F0-0002 §§11.6–11.7](F0-0002-second-slice.md).

La tabla normativa de ticks y sus orígenes está en F0-0002 §11.6. Leer un escalar existente en contexto de lectura no cobra copia; obtener propiedad desde un lugar sí cobra todas las celdas copiadas. `<` cobra dos visitas escalares y la construcción del booleano, además de nodos y operandos. `Ok`/`Err` cobran llamada y wrapper; `?` no cobra una llamada ni un retorno sintético.

El cobro de trabajo es unitario. Una implementación puede agrupar k ticks del mismo origen, pero, si no caben todos, concede conceptualmente el prefijo hasta el límite y produce R0001 con observed = limit + 1 y work_consumed = limit. No materializa la copia ni compromete efectos parcialmente. No informa consumed + k como observed de R0001.

Esto no modifica R0002: su observed sigue siendo la ocupación estructural propuesta completa, y una reserva rechazada no cambia la ocupación comprometida.

F0-0002 §11.7 publica totales de 56, 25 y 15 ticks y una familia que solicita exactamente 999.999, 1.000.000 y 1.000.001. La contabilidad no se presenta como compatible con la copia incondicional de bindings de runtime.rs de F0-0001.

R0102 señala el selector concreto que falla, incluidos corchetes, también en accesos anidados; relaciona el acceso, destino o argumento completo. R0001/R0002 por unidad final implícita señala solo la llave de cierre y relaciona el cuerpo. R0002/call_frames señala la llamada rechazada completa. Los ticks normativos son 2 para `fn main(){}` y 6 para `a[1]=5;`; las diferencias del intérprete se corrigen en I10b. Procedencia: [F0-0002 §§11.7, 13.12, 13.15 y 14.11](F0-0002-second-slice.md).

Regresiones de conciliación previstas (no reejecutadas en esta entrega):

| ID | Entrada exacta | Resultado del prototipo / destino |
|---|---|---|
| L01 | CLI check con ruta declarada inexistente por el arnés. | FAIL previsto: E0004, exit 2 → NX-E0004/source_unreadable. |
| L02 | CLI check de bytes ASCII `fn main(){}//` seguidos de byte hexadecimal FF; tamaño menor de 1 MiB. | FAIL previsto: E0004, exit 2 → NX-E0004/invalid_utf8. |
| L03 | Los mismos bytes, rellenando antes de FF hasta superar 1.048.576 bytes. | FAIL previsto: E0003, exit 1 → NX-E0003, budget_kind = source_bytes. |
| L04 | CLI check de UTF-8 válido `fn main(){}`, tamaño menor de 1 MiB. | PASS previsto, sin diagnóstico de lectura. |

### Filas de cobertura del corte heredado

Estas filas son expectativas documentales sobre F0-0001, **no reejecuciones**.
Los diagnósticos canónicos se obtienen por el mapa anterior; no se exige que
el binario del corte ya emita NX ni campos que no implementa. Cada negativo
tiene un positivo editado explícitamente. Los fallos por restricciones del
corte no se anuncian como prohibiciones del lenguaje completo.

| ID | Fuente o entrada concreta | Resultado previsto | Diagnóstico / comprobación |
|---|---|---|---|
| K01 | `fn main() { £ }` | FAIL | E0001 → NX-E0001; etapa lex, lexema £, bytes C2 A3, span [12,14), prefijo de tokens fn main ( ) { reconocido. diagnostic_namespace = F0-0001; expected_tokens = [valid_token], expected_tokens_kind = lexical_category (tokenizable por syntax::lex, no lista cerrada de terminales de G); no fallo previo de parser. |
| K01b | `fn main() { }` | PASS | —; mismo prefijo léxico válido. |
| K02 | `fn main() ;` | FAIL | E0002 → NX-E0002; etapa parse, token ;, span [10,11), diagnostic_namespace = F0-0001; parser_context = function_body, expected_tokens = ["{"], expected_tokens_kind = admissible_set, prefijo fn main() reconocido. Esta lista es exclusiva del corte F0-0001, no de Signature ampliada (véase barrido en D13). |
| K02b | `fn main() {}` | PASS | —; cuerpo válido tras la misma firma. |
| K03 | `fn main() { print(x); }` | FAIL | E0101 → NX-E0101; binding x no declarado. |
| K03b | `fn main() { let x = 1; print(x); }` | PASS | —; binding declarado. |
| K04 | `fn main() { print(true + 1); }` | FAIL | E0102 → NX-E0102/type_mismatch. |
| K04b | K04 sustituyendo true por 1. | PASS | —; suma i64. |
| K05 | `fn main() { print(1[0]); }` | FAIL | E0102 → NX-E0102/not_indexable. |
| K05b | K05 sustituyendo `1[0]` por `[1][0]`. | PASS | —; array indexable. |
| K06 | `fn main() { let x = 1; inspect(x); }` | FAIL | E0102 → NX-E0102/resource_required; nombre sí resuelto. |
| K06b | K06 sustituyendo el inicializador 1 por mock_open(1). | PASS | —; inspect presta recurso disponible. |
| K07 | `fn main() { let x = []; }` | FAIL | E0103 → NX-E0103; restricción del corte, array vacío sin anotación soportada. |
| K07b | K07 sustituyendo [] por [1]. | PASS | —; elemento inferible. |
| K08 | `fn main() { let x = 1; let x = 2; }` | FAIL | E0104 → NX-E0104. |
| K08b | K08 renombrando solo el segundo x a y. | PASS | —; nombres distintos. |
| K09 | `fn main() { let x = 9223372036854775808; }` | FAIL | E0107 → NX-E0107; literal fuera de i64. |
| K09b | K09 sustituyendo el literal por 9223372036854775807. | PASS | —; límite superior i64. |
| K10 | `fn main() { let r = mock_open(1); let a = [move r]; }` | FAIL | E0203 → NX-E0203/unsupported_move_position. |
| K10b | K10 sustituyendo [move r] por move r. | PASS | —; inicializador completo admitido por el corte. |
| K11 | `fn main() { let a = [mock_open(1)]; }` | FAIL | E0203 → NX-E0203/unsupported_resource_position. |
| K11b | K11 quitando los corchetes del inicializador. | PASS | —; recurso no anidado. |
| K12 | `fn main() { var r = mock_open(1); r = mock_open(2); }` | FAIL | E0203 → NX-E0203/unsupported_resource_rebinding. |
| K12b | K12 sustituyendo la segunda sentencia por `let s = mock_open(2);`. | PASS | —; propietarios separados. |
| K13 | `fn main() { let x = 1; let y = move x; }` | FAIL | E0102 → NX-E0203/unsupported_transfer_kind; limitación de F0-0001, no rechazo de consume sobre datos en D11. |
| K13b | K13 sustituyendo move x por x. | PASS | —; copia de i64 admitida en el corte. |
| K14 | `fn main() { print(9223372036854775807 + 1); }` | FAIL dinámico | R0101 tras comprobación estática aceptada; no es NX-E. |
| K14b | K14 cambiando el sumando final por 0. | PASS | —; sin desbordamiento. |
| K15 | `fn main() { print([1][1]); }` | FAIL dinámico | R0102 tras comprobación estática aceptada. |
| K15b | K15 cambiando el índice a 0. | PASS | —; índice válido. |
| K16 | `fn main() { assert_eq(1, 2); }` | FAIL dinámico | R0201 tras comprobación estática aceptada. |
| K16b | K16 cambiando 2 por 1. | PASS | —; datos iguales. |
| K17 | `fn main() { fail(); }` | FAIL dinámico | R0301 tras comprobación estática aceptada. |
| K17b | `fn main() {}` | PASS | —; no se solicita fallo simulado. |
| K18 | Prueba existente tests/corpus.rs, work_budget_exhaustion_closes_resources. | NO EVALUADO | Objetivo R0001 y limpieza del recurso 7; falta reejecutar el arnés del corte en esta entrega documental. No se supone probado por el mapa de códigos. |
| K19 | Siete perfiles propuestos: comptime_steps, comptime_depth, comptime_memory_bytes, type_solver_steps, effect_fixpoint_steps, trait_solver_steps, generic_instantiations. | NO EVALUADO | Objetivo NX-E0003 por cada budget_kind; faltan evaluador/solucionadores, contabilidad y arnés con límite N/N+1. No se sustituyen por source_bytes. |
| K20 | Cadena aditiva; inspección syntax.rs:22–35, 294–309. | NO EVALUADO | Emisión addition_chain_length inalcanzable con el prototipo actual: la construcción del nodo Add de la operación 128 falla por expression_tree_depth antes de que chain llegue a 129. No existe fixture fuente que aísle esta guarda manteniendo el código actual; no es una prueba meramente pendiente. |
| K21 | Cadena indexada; inspección syntax.rs:22–35, 314–329. | NO EVALUADO | Emisión index_chain_length inalcanzable con el prototipo actual: la construcción del nodo Index de la operación 128 falla por expression_tree_depth antes de que chain llegue a 129. Ni índices simples ni un índice más profundo pueden retrasar ese fallo; no hay fixture aislante posible sin cambiar el parser. |

Las fuentes L01–L04 especifican NX-E0004 y source_bytes de NX-E0003.
La inspección de tests/corpus.rs (nesting_limit_is_diagnostic,
composed_ast_depth_is_bounded_before_recursive_passes e
inferred_nesting_across_bindings_is_bounded) y del arnés adversarial
(boundaries) encuentra casos dirigidos a syntax_nesting, expression_tree_depth
y array_type_depth. No se reejecutan ni acreditan evidencia nueva aquí.
**addition_chain_length e index_chain_length son guardas inalcanzables en
el prototipo actual**, no fixtures simplemente pendientes. Expr::new asigna
profundidad mínima 1 al átomo y max(hijos) + 1 a Add/Index. Tras n operaciones
de una cadena la profundidad es al menos n + 1; al construir la operación
128, depth >= 129 y falla en syntax.rs:30. Las guardas chain > 128
(:300 y :319) solo podrían fallar al entrar en la operación 129, que nunca
se alcanza; hijos más profundos adelantan el fallo, no lo evitan. Esta es
evidencia por lectura del flujo, no ejecución ni aprobación de pruebas.

Se conservan esos dos budget_kind como correspondencias latentes de guardas
fuente, sin prometer que sean emitibles ni exigir un caso imposible. Un futuro
cambio del AST/presupuestos deberá decidir explícitamente si los hace alcanzables
o los retira; no se cambia Rust aquí. K20/K21 documentan esa limitación;
K19 distingue los siete perfiles propuestos que sí siguen pendientes de diseño
de arnés/solucionador.

| Propuesta de iteración 0 retirada | Destino; no emitir el número retirado |
|---|---|
| NX-E1103, NX-E1203 | NX-E0201, copia no admisible. |
| NX-E1106 | NX-E0202/definitely_consumed. |
| NX-E1207 para segundo await | NX-E0202/definitely_consumed. |
| NX-E1207 para reentrada interna (A09) | Sin equivalente estático: modelo pendiente NO EVALUADO. |
| NX-E1306 | NX-E0202/maybe_consumed o definitely_consumed según flujo. |
| NX-E1209 para transición interna de salida (A21) | Sin equivalente estático: modelo pendiente NO EVALUADO. No reutilizar para otro error. |
| NX-E1310/provenance_mismatch (iteración 1) | NX-E1320 para cláusula escrita insuficiente, pública o privada. La subcausa antigua no se emite ni se reutiliza. |

## Datos estructurados y aceptación

Campos mínimos: schema_version, diagnostic_namespace, code, subcause, rule,
severity, primary_span, related_spans, cause_chain, operation y repair.
Spans contienen archivo y offsets UTF-8; la traducción es presentación.
operation usa rutas como closure.call, future.cancel o branch.drop; una misma
regla no cambia de código por la envoltura. Rutas de tipos/paquetes usan
identidades canónicas. Las evidencias distinguen hecho verificado, supuesto
externo, estimación, medición y desconocido (P06).

Cada fixture negativa debe alcanzar la regla prevista; un parse error ajeno
no cuenta. Las reparaciones marcadas aplicables se vuelven a comprobar con
su positivo pareado, efectos, propiedad y observaciones. Cada defecto corregido
debe dejar esa regresión al implementar. Un caso de diseño sin fixture o sin
arnés queda NO EVALUADO y sigue pendiente, sin usarse para aprobar F0.

### Explicaciones y perfil de F0-0002

Procedencia: [F0-0002 §§12.4, 13.6–13.10 y 15](F0-0002-second-slice.md).

Todo diagnóstico emitido por F0-0002 incorpora profile_id y stage.

Las familias estáticas emitidas incorporan explanation_id, estable para el par code/subcause. La forma es `NX-Edddd` si subcause es null y `NX-Edddd/subcause` en caso contrario. El catálogo de explicaciones está versionado junto con diagnostic_namespace y se consulta sin red mediante:

```text
nexo explain NX-Edddd
nexo explain NX-Edddd --subcause nombre
```

La explicación describe regla, causa, ejemplo negativo, positivo y consecuencias de las reparaciones. `nexo explain NX-E0006` devuelve **30 entradas**, una por feature_id de la tabla cerrada, en el mismo orden por bytes, con `feature_ids = [ID]`, `subcause = null`, `explanation_id = NX-E0006`, `availability = in_profile` y `example_profile_id = f0-0002-b-v2`. Los ejemplos negativo/positivo son la primera fila B por número de F0-D15 §8.7 para ese ID y su edición completa; `unsupported_type` usa F2-F29 y `methods` F2-F23. Procedencia: [F0-0002 §13.10](F0-0002-second-slice.md), enmendado por [F0-D15 §§3.6, 6.1 y 6.4](F0-D15-sintaxis-oficial.md).

Para `implementation_limit`, la explicación enumera I08, I09, I10a, I10b e I11a como capacidades pendientes, sin atribuir una prohibición al lenguaje. El negativo es `fn Main()->Result<(),I64>{return Ok(());}` y el positivo `fn Main(){}`; el negativo exige `pending_scope = I08` y primario `Result<(),I64>` en `[11,25)`. Al implementar I08 se sustituye por una capacidad aún pendiente; cuando no quede ninguna, el ejemplo pasa a histórico (F0-D15 §§6.1 y 7.1). NX-E1301 conserva `availability = historical`, `example_profile_id = null` y ejemplos históricos, no fuentes B aceptables (F0-D15 §6.1).

Una consulta con código o subcausa inexistentes devuelve un error de uso; no fabrica documentación. Las etiquetas locales de fixtures de F0-0002 se citan fuera de ese ADR con prefijo `F2-`, sin renombrar las fixtures de D11, D13 o del catálogo (F0-0002 §13.10).

Los diagnósticos de alcance incluyen feature_id. Los de efectos incluyen effect_model, contract_origin, operation, effect_evidence y cause_chain. NX-E1105 usa exclusivamente las claves y combinaciones de permiso/acceso de la tabla por subcausa de F0-0002 §13.6, incluidas las expectativas completas de F2-P04–F2-P07; `borrow_inout` identifica la solicitud de préstamo exclusivo y no crea un préstamo si se rechaza. `callee` no se emite en `data`; las otras identidades y permisos de esa tabla están ausentes en códigos distintos de NX-E1105. Las ubicaciones sintéticas de limpieza relacionan el binding propietario y la salida; no inventan un drop escrito.

Repair distingue propuesta de edición y edición recomprobada. Una edición solo se anuncia aplicable después de comprobar de nuevo tipos, propiedad, flujo y efectos. No se introducen unsafe, debilitamiento de contratos ni cambios silenciosos de copia/transferencia.

Las pruebas comparan código, subcausa, etapa, spans, token cuando proceda y cadena causal relevante, con la excepción de `primary_span` de los cuatro presupuestos costosos de NX-E0003 definida en «Perfil de análisis de F0-0002», con referencia a F0-0002 §13.13: se comprueba su validez y no vaciedad, no un offset exacto. Mensajes traducidos y números de línea de presentación no reemplazan offsets UTF-8 ni identidad de regla.


Runtime_budget_kind es un campo independiente, cerrado a call_frames, live_value_cells y observation_bytes para R0002. No amplía ni reutiliza la enumeración budget_kind de NX-E0003.

NX-E0104 incorpora declaration_kind = binding, function o parameter y relaciona la primera declaración. NX-E0101 incorpora use_kind = value o call_target. Son datos estructurados, no subcausas; ambos conservan subcause = null.

Los perfiles de F0-0002 usan la interfaz mock y el registro ordenado de observaciones de F0-0002 §12.4. Las entradas JSON inválidas del entorno son errores de configuración del controlador con salida 2, no diagnósticos NX-E del programa. Un Err final de main tiene salida 1, outcome.kind = err y ningún diagnóstico R por ese hecho.

La comparación de observaciones exige salida, eventos, intercalación, bytes virtuales y reservas finales. Prefijos y LF cuentan; la serialización JSON del informe no cuenta. Las pruebas de IR alterada pertenecen al arnés Rust del verificador y no se atribuyen al modelo Python independiente.


### Serialización normativa de F0-0002

Procedencia: [F0-0002 §§13.6–13.10](F0-0002-second-slice.md).

Para schema_version = 1 de este perfil, se incorpora por referencia la definición completa de **F0-0002 §§13.6–13.10**: tipos literales de Diagnostic, Span, RelatedSpan, Cause, Token, Repair y Data; obligatoriedad; orden de listas; ejemplos completos; presentación textual; cardinalidad de diagnósticos y formato de explain.

Para NX-E0006, `data.pending_scope` es obligatorio y queda cerrado a la tabla siguiente; no admite `migration`, `I10` ni `I11b`. Procedencia: [F0-0002 §13.6](F0-0002-second-slice.md), enmendado por [F0-D15 §§3.6, 6.1 (fila «§13.6 feature_id y pending_scope») y 7.1](F0-D15-sintaxis-oficial.md).

| feature_id | pending_scope | Regla |
|---|---|---|
| closure, generator, option, match | F0-0003 | Diseño diferido de estas construcciones. |
| Los otros 25 identificadores de alcance | later_adr | Alcance diferido; excluye implementation_limit. |
| implementation_limit | I08 | Result, Ok/Err, `?`, TryAdd y firmas Result de entrada; primario sobre el tipo o construcción correspondiente. Main con resultado unidad puede ejecutarse; firmas incompatibles conservan NX-E0112. |
| implementation_limit | I09 | Contratos y comprobación transitiva de efectos: cláusula with escrita o primitiva que exige el manifiesto pendiente, incluida MockUnknown. No se acepta un contrato sin verificarlo. |
| implementation_limit | I10a | Perfiles reducidos y garantías completas de análisis: solicitud explícita del arnés interno de un perfil no disponible; no rechaza programas ordinarios ni sustituye fallos de cuotas/metadatos. La opción CLI se rechaza con exit 2. |
| implementation_limit | I10b | Cuotas completas de ejecución, contabilidad y reservas de observación: solicitud del arnés interno de un perfil o protocolo no disponible. El subconjunto ordinario conserva sus guardas; no sustituye discrepancias de ticks/reservas. La opción CLI se rechaza con exit 2. |
| implementation_limit | I11a | Interfaz mock de entrada y tres flujos integrados: primera llamada aún no ejecutable a MockReadRecords, MockWriteI64, MockRequest o MockReply, aunque I08 ya admita Result. I11b integra evidencia, no añade una capacidad que justifique rechazo. |

`implementation_limit` usa `stage = types`, `operation = types.check`, `rule = explanation_id = NX-E0006`, `subcause = null`, namespace Nexo-proposed-v1, severity error y perfil B vigente. El primario señala la primera construcción que necesita la capacidad o EOF vacío para una petición del arnés sin construcción fuente. `data` contiene únicamente `pending_scope`; related_spans, cause_chain, repair y expected_tokens están vacíos; unexpected_token, parser_context, recognized_prefix y expected_tokens_kind son null. El mensaje literal es `Capacidad pendiente de implementación en {pending_scope}.`, con el valor sustituido; exit 1, sin salida de programa ni eventos. Entre varias capacidades se elige el menor inicio fuente y, a igualdad de sitio, I08, I09, I10a, I10b, I11a; una barrera anterior conserva prioridad. No se reconoce por patrones de fuente ni convierte fallos internos en diagnósticos de programa (F0-D15 §7.1).

El rechazo temporal no acredita conformidad ni sustituye los esperados semánticos, de inferencia/procedencia, presupuestos/metadatos o ticks/spans/reservas de F0-0002 §17.2. Cada capacidad deja de emitir su rechazo cuando se implementa (F0-D15 §§6.1, 7.1 y 7.3).

En particular:

- Un span es `{"file":String,"start":UInt,"end":UInt}`, con offsets de bytes UTF-8 desde cero y rango `[start,end)`. No se serializa como array ni como línea/columna.
- `related_spans` es una lista de `{"role":String,"span":Span}`.
- `cause_chain` es una lista ordenada de pasos con kind, operation, span, entity y evidence.
- `repair` es una lista de propuestas con id, status, description, edits, validation y consequences; cada edición incluye span, expected_text y replacement.
- `stage`, `feature_id`, `unexpected_token`, `expected_tokens`, `expected_tokens_kind`, `parser_context` y `recognized_prefix` son claves del objeto diagnóstico, no campos implícitos del mensaje.
- Los datos especializados, incluidos budget_kind, budget_scope, incomplete_analysis, analysis_function, runtime_budget_kind, limit, observed, declaration_kind y use_kind, se serializan dentro de `data`.
- El orden de claves JSON no identifica el diagnóstico. La comparación utiliza objetos decodificados y respeta el orden normativo de arrays.
- Los diagnósticos NX-E usan Nexo-proposed-v1; los fallos R usan F0-0002-sim. No se cambia el namespace histórico F0-0001.

La primera línea textual es `CODE:LINE:COLUMN: MESSAGE`, con código canónico, línea y columna desde uno; las líneas estructuradas adicionales siguen §13.8. No se conservan prefijos E0101 o E0202 sin NX- en las expectativas migradas.

El catálogo mantiene la autoridad de códigos y subcausas. Esta referencia normativa concreta su serialización para el perfil; no crea otro registro de significados.


### Contrato observable de F0-0002

Procedencia: [F0-0002 §§13.12–13.17](F0-0002-second-slice.md).

Se incorpora por referencia como contrato del perfil F0-0002 §§13.12–13.17:

- Tabla exhaustiva de `stage` y `operation` para las emisiones incluidas.
- Spans exactos para seis presupuestos y propiedad de span válido y no vacío para los cuatro presupuestos costosos, según §13.13.
- Campos literales de los negativos sintácticos existentes y derivación gramatical para los restantes.
- Enumeraciones cerradas de `Diagnostic.operation`, `RelatedSpan.role`, `Cause.operation` y `Repair.id`, con sus reglas de emisión.
- Selección, orden, spans, entidades y evidencia de los testigos causales.
- Rutas de `unresolved_components` y dominios de `expected_type`/`found_type`.

`expected_tokens_kind` admite además `lexical_category`, usado por NX-E0001 con `expected_tokens = ["valid_token"]`. Se conserva la distinción entre categoría léxica, conjunto admisible e inserción canónica.

NX-E0005 incluye la expresión precedente con `role = preceding_expression`, conforme a D13 §2.3. En el primer ejemplo de F0-0002 §13.7, su span es `[16,17)`; el primario sigue siendo `[17,18)`. Los errores de salto tras return/value: incluyen también el salto y el tipo o destinatario requerido.

Las llamadas denunciadas por efectos y los pasos causales con kind `call` señalan el nombre del destino, según F0-0002 §8.2. Para el intrínseco `drop(r)` explícito, el primario es el nombre `drop` y la hoja `value.drop`, de kind `drop`, abarca la llamada completa sin terminador. Una limpieza implícita identifica propietario, salida y cota escrita; el destructor del manifiesto se identifica sin inventar un span del archivo del usuario.

El inventario integrado es de 70 códigos NX-E, siete R experimentales y 30 feature_id. Las reglas enmendadas son obligaciones de implementación y revisión, no evidencia de conformidad.

TD01/TD03 y el origen de tipo esperado de TD02 son divergencias declaradas con dueño I07c: en TD02, expected_type debe relacionar [16,17), no [24,25) como hace el código actual. Procedencia: [F0-0002 §§13.15, 14.10 y 17.2](F0-0002-second-slice.md).

`expected_type` relacionado identifica la anotación o restricción fuente que realmente impone el tipo, incluida la raíz anotada de una asignación indexada. Si es inferido, se relaciona la expresión que lo determina; no se inventa anotación. Never no se serializa. La cardinalidad y la selección causal incluyen el agotamiento local/global y la relación entre NX-E0105 y NX-E0202. Procedencia: [F0-0002 §§4.1.1, 13.9, 13.15, 13.17 y 14.10–14.11](F0-0002-second-slice.md).

## Trazabilidad de iteración 1

Resuelve documentalmente motivos 1/2/3/10: subcausas nombradas, propiedad común,
mapa del prototipo, numeración, error intrínseco y propietarios únicos.
La validación anterior de D11–D13 fue FAIL según la devolución del coordinador.
Aquella iteración también recibió FAIL; sus defectos pendientes motivan la
iteración 2, que recibió FAIL; la iteración 3 también recibió FAIL. La
iteración 4 recibió FAIL; la iteración 5 requiere un nuevo dictamen.

## Trazabilidad de iteración 2

Motivo 1: inventario por búsqueda real de todas las emisiones E/R; E0004 se
separa por causa de lectura/UTF-8, con prioridad de tamaño documentada y
regresiones L01–L04. Motivo 2: nuevos códigos para reglas sin cobertura,
reutilización explícita de NX-E0105/NX-E1105 y migración de la subcausa de
procedencia a NX-E1320. Motivos 3/4/6: inicializador ConstDecl, derivación
SuspendSafe y estructura WithClause con propietarios únicos. Observaciones i/iii:
NX-E1210 remite a D12 §3; NX-E1301 exige su subcausa singleton, nunca null.
La inspección documental no sustituye al validador ni al compilador futuro.

## Trazabilidad de iteración 3

B2 separa terminación de generador (NX-E1324 para return con operando) de
NX-E1317 ordinario/async. B5 fija la evidencia obligatoria de negativos
genéricos; B6 añade NX-E0005 y missing_value_operand. N1 incorpora filas
K y completa cobertura por código/subcausa junto a las tablas de ADR.
N4 cierra budget_kind y distingue los límites del prototipo de los perfiles
propuestos NO EVALUADOS. N5 aclara rangos por área sin renumerar identidades
estables. La conciliación de 13 E y cinco R permanece; no se modifica Rust.
B1/B3/B4 se trazan en D12/D13. La iteración 3 recibió FAIL según el
coordinador; no se reinterpretan sus casos previstos como aprobación.

## Cobertura que sigue solo NO EVALUADA

Estas reglas no adquieren un caso PASS/FAIL concreto por esta iteración.
Sus filas siguen pendientes por los motivos indicados; no se contabilizan
como cobertura ejecutada ni aprobación del diseño.

| Regla pendiente | Filas de referencia | Evidencia que falta |
|---|---|---|
| NX-E0413 | D11 C21; D12 A22; D13 E17 | Recurso con destructor bloqueante y contrato de efectos, con positivo pareado. |
| NX-E1108 | D11 C06 | Actor/estado compartido anidado y requisito concreto de independencia. |
| NX-E1109 | D11 C19; D12 A24/A25; D13 M18 | LocalRc/recurso afín confiado, dominios local/migrable y transferencia. |
| NX-E1110 | D11 C23 | Generador F2 que retenga una closure prestada válida al construir, sin escape previo. |
| NX-E1208 | D12 A20 | Transporte con retención externa sin protocolo de retiro. |
| NX-E1211 | D12 A30 | Destructor que suspenda, sin contrato with que confunda la violación intrínseca. |
| NX-E1307 | D13 E15 | Vista concreta que escape por una rama/error/wrapper. |
| NX-E1318 | D13 E24/E27 | Portador genérico y constructor de vista ordinaria con duración comprobable. |
| NX-E1305/guard_effect | D13 E09 | Guarda con reserva/efecto real, distinta de mutación o transferencia. |
| NX-E1310/missing_provenance | D13 M07 | Firma pública de vista válida a la que solo falte procedencia explícita. |


## Trazabilidad de iteración 4

B1 concilia NX-E1301 con la producción reconocedora y sus spans; NX-E0005
tiene recuperación explícita y el contrato de token canónico. B2 se inventaría
en D13. N1 retira la atribución de pruebas inexistentes a cadenas +/[];
K20/K21 permanecen NO EVALUADOS. N2 completa terminadores y cotas públicas
en D13/D11. N3 alinea unreachable_arm y la prioridad de operaciones;
N4 corrige formato. N5 permanece en la tabla anterior. Los 13 E y cinco R
del prototipo conservan sus mapas; esta entrega no modifica ni ejecuta Rust.

## Trazabilidad de iteración 5

B1 usa value: contextual, con value como Name; **corrige choque con EX01/EX04**.
La subcausa non_final_branch_value pertenece a NX-E1302, cuya prioridad
remite a D13 §1; permanecen 65 códigos canónicos, sin renumeración.
V01–V16b y X01/X04/X04r explicitan spans y compatibilidad. N5 aclara retención
directa/indirecta en NX-E1319. N6 agrega spans de negativos sintácticos
deliberados. N7 ordena K19–K21 y distingue en main.rs la llamada from_utf8
(:27), brazo Err (:29) y emisión (:30), leídos en el código actual.
N1–N4 se trazan en los ADRs; las revisiones 4 y 5 recibieron FAIL según el coordinador.
No se afirma ejecución nueva ni aprobación; se mantienen los NO EVALUADOS.

## Trazabilidad de iteración 6

B1: ModuleDeclStart incorpora @ y D13 documenta el contraste completo de
expected_tokens con las posiciones de EX01–EX04. K01 distingue categoría léxica
y K02 el dialecto histórico F0-0001, sin restringir las firmas nuevas.
N1: se conserva el mapa de guardas, pero addition_chain_length e
index_chain_length son inalcanzables por expression_tree_depth; K20/K21
no piden una fixture imposible ni cuentan como cobertura ejecutada.
N2 se concreta en G.1/C52–C52d y NX-E0102/field_requires_grouped_call.
N3 añade tres subcausas de NX-E0417,
con reconocimiento y filas C53–C56, y NX-E1213 con A40/A41; C56e/S56e,
C57/S57 y C58 cubren los rechazos sintácticos y el sellado Call*.
En esa iteración histórica había 66 códigos canónicos, sin reutilizar identidades; el registro integrado actual contiene 70.
N4 incluye S39/S33m en las ubicaciones y retira el duplicado E31c en D13.
La revisión 5 recibió FAIL según el coordinador; la 6 sigue sin dictamen.
No se atribuye aprobación ni ejecución a esta revisión documental.

## Trazabilidad de F0-0002

Integración vigente desde 2026-10-22. Se absorbe F0-CODIGOS-ADDENDUM-0002 y se elimina como fuente separada; las prescripciones sustituidas de contabilidad se reemplazan por la decisión de [F0-0002 §20](F0-0002-second-slice.md).

| Contenido integrado | Ubicación única en este catálogo | Procedencia normativa |
|---|---|---|
| NX-E0006, NX-E0111–NX-E0113; ampliaciones NX-E0101/NX-E0104 | Registro canónico | F0-0002 §§2.2, 4.1–4.5 y 13.17. |
| Préstamos, disponibilidad, value:, tipos, asociación y exclusiones; 30 feature_id | Precisiones, Feature IDs, asociación y operadores | F0-0002 §§3–6, 13.1, 13.9, 13.12 y 14.6. |
| Destrucción estática de ambas variantes de Result | Destrucción estática de Result | F0-0002 §8.5. |
| Cuatro budget_kind nuevos, perfiles y alcanzabilidad; unidades dispersas locales/globales | Presupuestos cerrados y Perfil de análisis | F0-0002 §§11.1, 11.4–11.5, 11.8, 13.12–13.13. |
| R0002/R0103, cuotas runtime, trabajo unitario y migración E/R | Runtime experimental y Trabajo y lotes | F0-0002 §§7, 11.2–11.7, 12.4, 13.2–13.5. |
| Explicaciones, serialización, campos, spans, reparación y testigos | Datos estructurados y sus subsecciones F0-0002 | F0-0002 §§13.6–13.17 y 15. |
| NB1, NB2, NB5 y NB8 concretados; NB3, NB4, NB6 diferidos; NB7 exige positivos | Referencia a la matriz de obligaciones | F0-0002 §16, sin eliminar pendientes históricos. |

El inventario conserva NX-E0203 como histórico, aunque no emitible en este perfil. Las cuatro adiciones elevan 66 a 70 entradas; no se cuentan subcausas ni feature_id como códigos. Los cinco R históricos más R0002/R0103 forman siete bajo F0-0002-sim, separados de NX-E. El historial de revisiones permanece como evidencia de su fecha; ninguna integración acredita aprobación de fase.
