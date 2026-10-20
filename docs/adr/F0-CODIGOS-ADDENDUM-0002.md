# Addendum de F0-0002

Estado: propuesta; sin evidencia de implementación ni validación.

## 1. Filas a añadir en «Registro canónico»

Añadir las cuatro filas siguientes a la tabla existente, conservando el orden numérico. No renumerar entradas.

| Código | Propietario | Regla o subcausas cerradas |
|---|---|---|
| NX-E0006 | F0-0002/alcance del prototipo | Construcción reconocida pero fuera del perfil experimental solicitado. Subcause = null. Exige profile_id y feature_id registrado para ese perfil; no acredita una prohibición del lenguaje completo ni una regla semántica todavía no implementada. |
| NX-E0111 | F0-0002/llamadas | Asociación o destino de llamada inválidos: `missing_argument`, `excess_positional_argument`, `unknown_argument_label`, `duplicate_argument`, `positional_after_named`, `argument_mode_mismatch`, `non_callable_target`. Una incompatibilidad de tipos con asociación válida conserva NX-E0102/type_mismatch; copiar un argumento no copiable conserva NX-E0201. |
| NX-E0112 | F0-0002/entrada ejecutable | Entrada solicitada no disponible: `missing_main`, `invalid_main_signature`. No se emite por ausencia de main al comprobar un módulo de biblioteca. |
| NX-E0113 | F0-0002/inferencia local | No existe una determinación única de los tipos requeridos por un constructor de valor del subconjunto, después de aplicar el contexto local disponible. Subcause = null. No se inventa un tipo comodín. El array vacío conserva NX-E0103 y la firma de closure conserva NX-E1111. |

Además, ampliar expresamente las dos filas existentes siguientes del mismo «Registro canónico», sustituyéndolas por estas filas completas. Se conservan sus códigos, causas anteriores y `subcause = null`; no se introduce una subcausa implícita.

| Código | Propietario | Regla o subcausas cerradas |
|---|---|---|
| NX-E0101 | F0-0001/tipos; ampliación de resolución F0-0002 | Nombre de valor no declarado, incluido un binding usado como expresión o un destino de llamada no declarado. Subcause = null. En F0-0002, use_kind distingue value y call_target; un nombre resuelto a valor no invocable usa NX-E0111/non_callable_target. |
| NX-E0104 | F0-0001/nombres; ampliación de declaraciones F0-0002 | Declaración duplicada en el mismo ámbito: binding, función o parámetro. Subcause = null. En F0-0002 exige declaration_kind = binding, function o parameter, span de la segunda declaración y primera declaración relacionada. |

Estas ampliaciones no cambian las emisiones ni los resultados históricos de F0-0001.
## 2. Subsección a añadir después de las prioridades del «Registro canónico», antes de «Presupuestos cerrados de NX-E0003»

### Precisiones de F0-0002

Las subcausas de NX-E1105 se reutilizan para préstamos de argumentos y représtamos directos: active_borrow_conflict, permission_escalation y overlapping_mutable_borrows. No se crean códigos distintos por no intervenir una closure.

NX-E0202 conserva definitely_consumed y maybe_consumed en ramas y retroaristas. El diagnóstico de bucle identifica el consumo y la retroarista causante; no supone una única iteración.

Para una misma sentencia ValueStmt reconocida sin operando, la prioridad es invalid_value_target, missing_value_operand, non_final_branch_value y errores de flujo derivados. En `{ value: }`, el diagnóstico principal es NX-E1302/missing_value_operand sobre value:, y no se duplica NX-E0005 por el terminador ausente de esa misma sentencia vacía. En `{ value: 1 }` corresponde NX-E0005 sobre }. La prioridad no suprime errores sintácticos independientes ni permite aceptar una recuperación.

Sin tipo esperado, la discrepancia entre tipos sintetizados de continuaciones de un if usa NX-E1303. Con un tipo esperado propagado desde retorno, anotación, asignación o parámetro, el valor incompatible usa NX-E0102/type_mismatch; no se duplica NX-E1303 por esa misma causa. Los defectos estructurales de ramas preceden a incompatibilidades derivadas de un valor inexistente.

En NX-E0111, la asociación se comprueba antes de tipos y efectos. Se informa primero el defecto encontrado en el orden escrito de argumentos; los parámetros ausentes se informan después de recorrer los argumentos presentes. Missing_argument señala el callee y relaciona el parámetro ausente; excess_positional_argument señala el argumento excedente; unknown_argument_label y duplicate_argument señalan la etiqueta; positional_after_named señala el argumento posicional; argument_mode_mismatch señala el argumento; non_callable_target señala el destino. Un destino que no resuelve nombre usa NX-E0101.

NX-E0112/missing_main usa el rango vacío de EOF y la entrada solicitada. Invalid_main_signature señala el nombre main y relaciona su firma.

NX-E0113 señala el constructor cuyo tipo permanece indeterminado y enumera los componentes no resueltos. Una anotación candidata solo se anuncia aplicable después de recomprobar el archivo completo.

Para comparaciones de F0-0002, tipos de operandos distintos producen NX-E0102/type_mismatch sobre el operando derecho. Si los tipos coinciden pero el operador no los admite, el span es el operador. No se intenta copiar el operando para producir antes NX-E0201. El dominio exacto se fija en F0-0002 §4.6.

### Feature IDs de NX-E0006 para f0-0002-v1

Los feature_id siguientes son identificadores cerrados del perfil, no subcausas de NX-E0006. Los perfiles reducidos publicados en F0-0002 §11 heredan esta misma lista.

| feature_id | Construcción diferida o migración |
|---|---|
| legacy_move | Operador provisional move de F0-0001; consume es la migración candidata, sujeta a recomprobación. |
| inferred_non_unit_result | Función sin anotación de resultado que requiere inferir un resultado distinto de unidad. |
| float_literal | Literal decimal reconocido; semántica flotante no implementada. |
| range | Rango reconocido mediante .., sin ejecución de rangos en el perfil. |
| tuple | Tipo, expresión o patrón de producto distinto de unidad/agrupación. |
| attributes | Atributos, incluido @derive. |
| default_argument | Valor por defecto de parámetro. |
| closure | Literal de closure y sus capturas. |
| function_value | Uso almacenado, retornado o pasado de una función como valor. |
| receiver_mode | Prefijo inout/consume aplicado al receptor de una llamada completa. |
| partial_transfer | Transferencia consume de una proyección, en vez de un binding completo. |
| resource_array | Array cuyo elemento contiene transitivamente un recurso. |
| no_heap_alloc | Contrato reconocido, fuera del conjunto implementado por este perfil. |
| match | Selección por patrones; `pending_scope = F0-0003` identifica el diseño de la selección necesaria para `next`/`Option`, sin prometer toda la selección por patrones de la base en ese corte. |
| option | Tipo o constructores intrínsecos de Option. |
| user_types | Struct, enum o alias declarado por el usuario. |
| traits_generics | Traits, impl, genéricos, where o despacho dinámico. |
| modules_visibility | Mod, use, pub o rutas de módulos adicionales fuera del contexto de tipo descrito abajo. |
| extensions | Extend, use extension o llamada cualificada de extensión. |
| provenance_clauses | From, result_from o retains escritos. |
| comptime | Evaluación o constantes de módulo mediante comptime. |
| text | Cadenas e interpolación. |
| methods | Selección o invocación de métodos/campos fuera de las proyecciones indexadas del perfil; `self` como expresión de receptor reconocida, con span de ese token y etapa `parse`. En posición que exige Name, `self` conserva NX-E0002. |
| compound_assignment | Asignación compuesta, incluido +=. |
| for_iteration | For y su protocolo de iteración. |
| loop_control | Break o continue. |
| generator | Gen fn, yield y operaciones de generador. |
| async | Async, await y tipos u operaciones reconocidos de tareas/futuros. |
| region | Regiones y su grupo de tareas. |
| unsupported_type | Ruta de tipo reconocida que no pertenece a la lista aceptada ni tiene una exclusión más específica; incluye u32, u64, f32, String y app.Error. |

`indirect_call` se retira de la lista emitible propuesta para este perfil. No existe un valor invocable aceptado que permita aislar esa causa: la introducción de ese valor falla antes por closure o function_value. Un valor aceptado no invocable usa NX-E0111/non_callable_target. No se exige una fixture imposible ni se acredita soporte indirecto.

El span señala el introductor reconocido, con estas precisiones: inferred_non_unit_result señala el nombre de la función y relaciona el retorno no unitario; resource_array señala la construcción completa; partial_transfer señala la proyección; unsupported_type señala la ruta de tipo completa. En contexto de tipo, app.Error usa unsupported_type, no modules_visibility ni NX-E0101.

La forma documental es `code = NX-E0006, subcause = null, feature_id = nombre`. No se escribe `NX-E0006/nombre`, pues la barra queda reservada a subcausas del catálogo. Explanation_id permanece NX-E0006; feature_id selecciona el detalle de alcance.

Una fila de alcance necesita un positivo que elimine o sustituya explícitamente la característica. F0-0002 §14.6 publica esos pares con spans. Acreditan el diagnóstico de perfil, no la semántica de la característica diferida.

### Destrucción estática de Result

El resumen estático de drop sobre Result<T,E> une los efectos de destrucción de ambas variantes, conocidos y desconocidos, salvo evidencia explícita de variante única. La ejecución destruye solo el payload activo. Esa evidencia no altera la copiabilidad del tipo completo.


### Precisión de asociación y etapas de F0-0002, iteración 2

En una llamada se emite únicamente el primer defecto de asociación encontrado al recorrer los argumentos escritos. Después de ese defecto no se emiten parámetros ausentes derivados ni errores de tipos de la misma llamada. Solo una asociación válida de todos los argumentos presentes permite buscar el primer parámetro ausente por orden de declaración.

Por tanto, F2-S03 contiene únicamente unknown_argument_label y F2-S05 únicamente positional_after_named. Llamadas independientes pueden producir varios diagnósticos de la etapa arguments.

La lista y el orden entre etapas se fijan en F0-0002 §13.9: una etapa con errores impide las posteriores; los errores independientes de una etapa semántica se conservan, evitando cascadas. Los formatos y nombres JSON exactos se fijan en §§13.6–13.8.

Los tokens `*` y `!=` en posición de operador binario usan NX-E0002, no NX-E0006. Exigen parser_context = unsupported_binary_operator, span del operador y expected_tokens = ["+", "-", "<", "=="]. Esa causa precede al terminador ausente derivado. La lista de feature_id permanece en 30 entradas.


### Precisiones de F0-0002, iteración 3

La exclusión binaria mediante NX-E0002 incluye exactamente `*`, `!=`, `>`, `<=` y `>=`, con `parser_context = unsupported_binary_operator` y `expected_tokens = ["+","-","<","=="]`. Precede a NX-E0005 derivado y no emite una reparación de terminador. El cierre de tipos seguido de `=` conserva la separación contextual de `>=` descrita en F0-0002 §3.6.

La descripción de `partial_transfer` se amplía a **transferencia o destrucción de una proyección de un propietario existente**. `consume a[0]` conserva etapa `parse`; `drop(a[0])`, una vez resuelto el intrínseco, usa etapa `arguments`. Ambos señalan la proyección y relacionan el prefijo o destino correspondiente. No se añade un feature_id.

Un prefijo `consume` o `inout` cuyo operando no es Place usa NX-E0002 con `parser_context = consume_place` o `inout_place` y `expected_tokens = ["Place"]`. Los receptores prefijados de llamada conservan primero su exclusión `receiver_mode`.

La tabla normativa de etapas para todos los códigos, subcausas y características emitibles es F0-0002 §13.12. Las demás entradas del catálogo no se convierten en emisiones del perfil.
## 3. Filas a añadir en «Presupuestos cerrados de NX-E0003»

Añadir a la tabla existente:

| budget_kind | Contabilidad y procedencia | Umbral del corte / estado |
|---|---|---|
| value_type_depth | Máximo número de envoltorios Array/Result en un camino de tipo, incluidas inferencias a través de bindings y firmas. | F0-0002 propuesto: 128. |
| function_count | Declaraciones de función reconocidas en el archivo. | F0-0002 propuesto: 1.024. |
| ownership_flow_steps | Una unidad por instrucción transferida por el análisis de propiedad y por componente de estado unida. | F0-0002 propuesto: 1.000.000. |
| ir_items | Registros de función, bloque, slot, instrucción, operando, préstamo y limpieza insertados. | F0-0002 propuesto: 262.144. |

Añadir después de la tabla:

Para profile_id = f0-0002-v1, type_solver_steps tiene límite 1.000.000 y cobra cada restricción procesada y cada par de nodos de tipo comparado. Effect_fixpoint_steps tiene límite 1.000.000 y cobra cada visita a operación y propagación por arista de llamada. Su recorrido es determinista. Source_bytes, syntax_nesting, expression_tree_depth y array_type_depth conservan los límites numéricos publicados en F0-0002.

Addition_chain_length e index_chain_length permanecen en el registro histórico; F0-0002 no promete emisiones independientes de expression_tree_depth.

Todo perfil reducido de pruebas se identifica y publica antes de ejecutar sus casos. Alcanzar un presupuesto distinto del esperado no acredita la guarda objetivo.


Los siguientes perfiles reducidos de F0-0002 quedan publicados junto con sus generadores fuente de §11.4. Cada uno hereda `f0-0002-v1` y cambia únicamente la cuota indicada:

| profile_id | Único cambio |
|---|---|
| f0-0002-test-types-32-v1 | type_solver_steps = 32. |
| f0-0002-test-flow-32-v1 | ownership_flow_steps = 32. |
| f0-0002-test-effects-32-v1 | effect_fixpoint_steps = 32. |
| f0-0002-test-ir-32-v1 | ir_items = 32. |

Las recetas son, respectivamente, 64 bindings escalares anotados, 64 inicializaciones de bindings, 64 llamadas directas a una función vacía y 64 bindings que requieren slots de IR. La fuente `fn main(){}` es el positivo bajo el límite. Se exige alcanzar el cargo 33 de la guarda objetivo sin exceder otra cuota.

Las fronteras de contador 31/32/33 y la alcanzabilidad desde fuente se informan por separado. No basta una prueba del contador aislado ni un rechazo por source_bytes.

Si una construcción excede simultáneamente array_type_depth y value_type_depth, se informa array_type_depth. Para aislar value_type_depth se parte de Result<i64,i64> y se añaden 128 envoltorios Array mediante bindings distintos.


### Contabilidad y alcanzabilidad normal de F0-0002, iteración 2

Para el perfil normal, F0-0002 §11.8 concreta cuatro familias dentro de 1 MiB:

| budget_kind | Cota constructiva que excede la cuota | Bytes de la fuente publicada |
|---|---:|---:|
| type_solver_steps | 15.400 comparaciones de 65 pares de nodos = 1.001.000, sin contar restricciones adicionales. | 93.307 |
| ownership_flow_steps | 1.024 joins de al menos 1.025 componentes = 1.049.600. | 23.491 |
| effect_fixpoint_steps | 1.022 sitios de un anillo, durante al menos 1.021 rondas = 1.043.462 propagaciones. | 18.240 |
| ir_items | 65.537 bindings por un mínimo de cuatro registros lógicos = 262.148. | 840.882 |

La comparación de tipos cobra pares lógicos incluso con interning; una unión de flujo cobra componentes aunque coincidan; efectos usa rondas síncronas por SCC y cuenta sitios de llamada distintos; IR cuenta ocurrencias de operandos, incluidos destino y literal. Los generadores, orden, aislamiento y cotas de las etapas anteriores son los de §11.8.

No se reinician los contadores entre funciones ni para recomprobaciones posteriores. Los primeros cargos de análisis excedidos usan observed = limit + 1. Los perfiles reducidos continúan siendo pruebas complementarias, no sustitutos de estas familias normales.

Son expectativas de diseño pendientes de ejecución, no evidencia de que el compilador actual alcance esas guardas.


### Etapas y ubicaciones de presupuestos de F0-0002

F0-0002 §§13.12–13.13 fijan la etapa de los diez `budget_kind` emitibles. Para `source_bytes`, `syntax_nesting`, `function_count`, `array_type_depth`, `value_type_depth` y `expression_tree_depth` se conserva la comparación exacta de `primary_span`. Para `type_solver_steps`, `effect_fixpoint_steps`, `ir_items` y `ownership_flow_steps`, el modelo independiente compara exactamente código, etapa, `budget_kind`, `limit` y `observed`, pero no `primary_span`: verifica por propiedad que exista, identifique el archivo de entrada y cumpla `0 <= start < end <= longitud_utf8`, con extremos en fronteras de caracteres. La excepción rige en perfiles normales y reducidos y frente a debug/release; no exige igualdad de esos offsets entre builds. No se prescribe una tabla de cargos internos por construcción.

En particular, `source_bytes` usa `[limit,limit+1)` y `observed = limit+1` sobre la lectura acotada anterior a decodificación. `syntax_nesting` señala el token que iniciaría la activación lógica excedida; las activaciones se definen por Block, Expr y Type, sin contar auxiliares de implementación.

La excepción de ubicación no modifica cuotas, contabilidad, etapas ni familias de alcanzabilidad. Los cuatro presupuestos costosos no tienen un offset esperado en filas ni pruebas. Los presupuestos históricos no emitibles siguen registrados; no se les atribuyen fixtures nuevas.

## 4. Subsección a añadir después de la tabla de códigos R en «Conciliación con F0-0001 y números retirados»

### Runtime experimental de F0-0002

No se modifican las condiciones ni los resultados históricos de F0-0001.

Diagnostic_namespace = F0-0002-sim identifica los fallos experimentales del nuevo intérprete. Se conservan R0001, R0101, R0102, R0201 y R0301 con su regla: agotamiento de trabajo, desbordamiento de suma, índice inválido, aserción fallida y fail simulado. El perfil determina la nueva contabilidad de trabajo.

Se añaden:

| Código | Propietario | Condición exacta |
|---|---|---|
| R0002 | F0-0002/interp | Se excede una cuota estructural de ejecución. runtime_budget_kind pertenece exclusivamente a call_frames, live_value_cells u observation_bytes. Exige limit, observed, profile_id y span de la operación desencadenante. No utiliza budget_kind. |
| R0103 | F0-0002/interp | La resta comprobada de dos i64 no es representable como i64. No se realiza wrapping. |

Los límites de `f0-0002-v1` son 1.000.000 unidades de trabajo, 256 frames, 65.536 celdas vivas y 65.536 bytes de observaciones comprometidos. Se cuenta salida más eventos, incluidos prefijos y LF, mediante la codificación virtual de F0-0002 §12.4; no se cuenta el envoltorio JSON de transporte.

La ocupación de observaciones suma bytes emitidos y reservas de cierre. Antes de adquirir un recurso se reserva el tamaño exacto de su cierre. La limpieza no se interrumpe por agotamiento de trabajo ni omite los eventos reservados.

Observed es el cargo u ocupación propuestos que excederían el límite. Una reserva rechazada no modifica la ocupación comprometida. El primer cargo excedido de trabajo tiene observed = limit + 1; una reserva estructural agregada puede excederlo en más de uno.

Se publican estos perfiles adicionales, que heredan todo lo demás de `f0-0002-v1`:

| profile_id | Único cambio |
|---|---|
| f0-0002-test-frames-8-v1 | call_frames = 8. |
| f0-0002-test-cells-64-v1 | live_value_cells = 64. |
| f0-0002-test-observations-64-v1 | observation_bytes = 64. |

F0-0002 §11 publica recetas aislantes para cada cuota normal y fronteras en los perfiles reducidos. La duplicación histórica de arrays migra a R0002 con runtime_budget_kind = live_value_cells. Una prueba separada de bucle sin crecimiento conserva el objetivo R0001 y limpieza del recurso.

Un Err(E) producido o propagado por el programa no es un diagnóstico R ni NX-E. Main devolviendo Err(i64) termina con salida 1 y outcome.kind = err. Try_add devuelve Err(30) por desbordamiento y no R0101. El operador ? no captura fallos del simulador.

Una IR inválida o transición imposible se informa como fallo interno de implementación, sin asignarle un NX-E o R de programa.

### Limitaciones históricas que deja de emitir F0-0002

NX-E0203 permanece en el catálogo con sus cuatro subcausas históricas, pero no se emite bajo los perfiles de F0-0002:

| Subcausa histórica | Destino en F0-0002 |
|---|---|
| unsupported_move_position | La grafía move usa NX-E0006, feature_id = legacy_move. Tras migrar a consume, los bindings completos se transfieren en contextos propietarios; proyecciones y receptores prefijados conservan sus exclusiones específicas partial_transfer y receiver_mode. |
| unsupported_resource_position | Temporales, argumentos, retornos y Result con recursos pasan a estar admitidos. Arrays que contienen recursos usan NX-E0006, feature_id = resource_array. |
| unsupported_resource_rebinding | Retirada: se permite reasignación completa de var con limpieza. Un binding inmutable conserva NX-E0105. |
| unsupported_transfer_kind | Retirada: consume y drop de datos propietarios están permitidos. Move continúa rechazado como grafía legada, independientemente del tipo. |

La restricción E0002 «única fn main» también se retira del perfil nuevo. Varias funciones son válidas; check admite biblioteca sin main; run usa NX-E0112 para su entrada. La sintaxis inválida conserva NX-E0002, los terminadores reconocidos NX-E0005 y las características reconocidas diferidas NX-E0006.

F0-0002 §13 conserva una tabla de los 13 E y cinco R y la migración de los 15 archivos del corpus, incluidos los pasivos con move, nested_resource.nx y use_after_move.nx. No se modifica retrospectivamente ninguna expectativa de F0-0001.


### Precisión de trabajo y lotes de F0-0002, iteración 2

La tabla normativa de ticks y sus orígenes está en F0-0002 §11.6. Leer un escalar existente en contexto de lectura no cobra copia; obtener propiedad desde un lugar sí cobra todas las celdas copiadas. `<` cobra dos visitas escalares y la construcción del booleano, además de nodos y operandos. `Ok`/`Err` cobran llamada y wrapper; `?` no cobra una llamada ni un retorno sintético.

El cobro de trabajo es unitario. Una implementación puede agrupar k ticks del mismo origen, pero, si no caben todos, concede conceptualmente el prefijo hasta el límite y produce R0001 con observed = limit + 1 y work_consumed = limit. No materializa la copia ni compromete efectos parcialmente. No informa consumed + k como observed de R0001.

Esto no modifica R0002: su observed sigue siendo la ocupación estructural propuesta completa, y una reserva rechazada no cambia la ocupación comprometida.

F0-0002 §11.7 publica totales de 56, 25 y 15 ticks y una familia que solicita exactamente 999.999, 1.000.000 y 1.000.001. La contabilidad no se presenta como compatible con la copia incondicional de bindings de runtime.rs de F0-0001.
## 5. Texto a añadir al final de «Datos estructurados y aceptación»

### Explicaciones y perfil de F0-0002

Todo diagnóstico emitido por F0-0002 incorpora profile_id y stage.

Las familias estáticas emitidas incorporan explanation_id, estable para el par code/subcause. La forma es `NX-Edddd` si subcause es null y `NX-Edddd/subcause` en caso contrario. El catálogo de explicaciones está versionado junto con diagnostic_namespace y se consulta sin red mediante:

```text
nexo explain NX-Edddd
nexo explain NX-Edddd --subcause nombre
```

La explicación describe regla, causa, ejemplo negativo, positivo y consecuencias de las reparaciones. `nexo explain NX-E0006` devuelve 30 entradas, una por feature_id, con lista `feature_ids` unitaria, `subcause = null` y ejemplos propios de la primera fila F2-F01–F2-F33 correspondiente, según F0-0002 §13.10; no devuelve una única fuente para las 30 exclusiones. Una consulta con código o subcausa inexistentes devuelve un error de uso; no fabrica documentación. Las etiquetas locales de fixtures de F0-0002 se citan fuera de ese ADR con prefijo `F2-`, sin renombrar las fixtures de D11, D13 o del catálogo.

Los diagnósticos de alcance incluyen feature_id. Los de efectos incluyen effect_model, contract_origin, operation, effect_evidence y cause_chain. NX-E1105 usa exclusivamente las claves y combinaciones de permiso/acceso de la tabla por subcausa de F0-0002 §13.6, incluidas las expectativas completas de F2-P04–F2-P07; `borrow_inout` identifica la solicitud de préstamo exclusivo y no crea un préstamo si se rechaza. `callee` no se emite en `data`; las otras identidades y permisos de esa tabla están ausentes en códigos distintos de NX-E1105. Las ubicaciones sintéticas de limpieza relacionan el binding propietario y la salida; no inventan un drop escrito.

Repair distingue propuesta de edición y edición recomprobada. Una edición solo se anuncia aplicable después de comprobar de nuevo tipos, propiedad, flujo y efectos. No se introducen unsafe, debilitamiento de contratos ni cambios silenciosos de copia/transferencia.

Las pruebas comparan código, subcausa, etapa, spans, token cuando proceda y cadena causal relevante, con la excepción de `primary_span` de los cuatro presupuestos costosos de NX-E0003 definida en «Etapas y ubicaciones de presupuestos de F0-0002»: se comprueba su validez y no vaciedad, no un offset exacto. Mensajes traducidos y números de línea de presentación no reemplazan offsets UTF-8 ni identidad de regla.


Runtime_budget_kind es un campo independiente, cerrado a call_frames, live_value_cells y observation_bytes para R0002. No amplía ni reutiliza la enumeración budget_kind de NX-E0003.

NX-E0104 incorpora declaration_kind = binding, function o parameter y relaciona la primera declaración. NX-E0101 incorpora use_kind = value o call_target. Son datos estructurados, no subcausas; ambos conservan subcause = null.

Los perfiles de F0-0002 usan la interfaz mock y el registro ordenado de observaciones de F0-0002 §12.4. Las entradas JSON inválidas del entorno son errores de configuración del controlador con salida 2, no diagnósticos NX-E del programa. Un Err final de main tiene salida 1, outcome.kind = err y ningún diagnóstico R por ese hecho.

La comparación de observaciones exige salida, eventos, intercalación, bytes virtuales y reservas finales. Prefijos y LF cuentan; la serialización JSON del informe no cuenta. Las pruebas de IR alterada pertenecen al arnés Rust del verificador y no se atribuyen al modelo Python independiente.


### Serialización normativa de F0-0002, iteración 2

Para schema_version = 1 de este perfil, incorporar como contrato la definición completa de **F0-0002 §§13.6–13.10**: tipos literales de Diagnostic, Span, RelatedSpan, Cause, Token, Repair y Data; obligatoriedad; orden de listas; ejemplos completos; presentación textual; cardinalidad de diagnósticos y formato de explain.

En particular:

- Un span es `{"file":String,"start":UInt,"end":UInt}`, con offsets de bytes UTF-8 desde cero y rango `[start,end)`. No se serializa como array ni como línea/columna.
- `related_spans` es una lista de `{"role":String,"span":Span}`.
- `cause_chain` es una lista ordenada de pasos con kind, operation, span, entity y evidence.
- `repair` es una lista de propuestas con id, status, description, edits, validation y consequences; cada edición incluye span, expected_text y replacement.
- `stage`, `feature_id`, `unexpected_token`, `expected_tokens`, `expected_tokens_kind`, `parser_context` y `recognized_prefix` son claves del objeto diagnóstico, no campos implícitos del mensaje.
- Los datos especializados, incluidos budget_kind, runtime_budget_kind, limit, observed, declaration_kind y use_kind, se serializan dentro de `data`.
- El orden de claves JSON no identifica el diagnóstico. La comparación utiliza objetos decodificados y respeta el orden normativo de arrays.
- Los diagnósticos NX-E usan Nexo-proposed-v1; los fallos R usan F0-0002-sim. No se cambia el namespace histórico F0-0001.

La primera línea textual es `CODE:LINE:COLUMN: MESSAGE`, con código canónico, línea y columna desde uno; las líneas estructuradas adicionales siguen §13.8. No se conservan prefijos E0101 o E0202 sin NX- en las expectativas migradas.

El catálogo mantiene la autoridad de códigos y subcausas. Esta referencia normativa concreta su serialización para el perfil; no crea otro registro de significados.


### Cierre del contrato observable de F0-0002, iteración 3

Incorporar también como contrato del perfil F0-0002 §§13.12–13.17:

- Tabla exhaustiva de `stage` y `operation` para las emisiones incluidas.
- Spans exactos para seis presupuestos y propiedad de span válido y no vacío para los cuatro presupuestos costosos, según §13.13.
- Campos literales de los negativos sintácticos existentes y derivación gramatical para los restantes.
- Enumeraciones cerradas de `Diagnostic.operation`, `RelatedSpan.role`, `Cause.operation` y `Repair.id`, con sus reglas de emisión.
- Selección, orden, spans, entidades y evidencia de los testigos causales.
- Rutas de `unresolved_components` y dominios de `expected_type`/`found_type`.

`expected_tokens_kind` admite además `lexical_category`, usado por NX-E0001 con `expected_tokens = ["valid_token"]`. Se conserva la distinción entre categoría léxica, conjunto admisible e inserción canónica.

NX-E0005 incluye la expresión precedente con `role = preceding_expression`, conforme a D13 §2.3. En el primer ejemplo de F0-0002 §13.7, su span es `[16,17)`; el primario sigue siendo `[17,18)`. Los errores de salto tras return/value: incluyen también el salto y el tipo o destinatario requerido.

Las llamadas denunciadas por efectos y los pasos causales con kind `call` señalan el nombre del destino, según F0-0002 §8.2. Para el intrínseco `drop(r)` explícito, el primario es el nombre `drop` y la hoja `value.drop`, de kind `drop`, abarca la llamada completa sin terminador. Una limpieza implícita identifica propietario, salida y cota escrita; el destructor del manifiesto se identifica sin inventar un span del archivo del usuario.

No cambian los 70 códigos canónicos integrados, los siete R experimentales ni los 30 feature_id. Estas precisiones siguen siendo diseño pendiente de implementación y revisión independiente.

## 6. Sección a añadir al final del documento

## Trazabilidad de F0-0002

La propuesta añade NX-E0006, NX-E0111, NX-E0112 y NX-E0113; cuatro budget_kind de análisis; R0002 y R0103 bajo el namespace experimental F0-0002-sim; explicación consultable y prioridad local para ValueStmt vacío.

Reutiliza NX-E1105 para préstamos directos, NX-E0202 para disponibilidad por caminos, NX-E0412–NX-E0415 para contratos y NX-E1302/NX-E1317 para valores y retornos.

NB1 se declara como ampliación experimental de drop; NB2 fija la frontera léxica decimal/rango; NB5 y NB8 se concretan aquí. NB3, NB4 y NB6 siguen diferidos expresamente en F0-0002. NB7 exige positivos pareados para las reglas implementadas y no elimina los pendientes históricos.

Estas adiciones son expectativas de diseño. No acreditan compilador, ejecución, revisión independiente ni aprobación de fase.


### Inventario integrado e iteración 2

Al integrar este addendum, el inventario vigente pasa de **66 a 70 entradas canónicas NX-E**: se añaden NX-E0006, NX-E0111, NX-E0112 y NX-E0113. Las ampliaciones de NX-E0101 y NX-E0104 no añaden códigos. NX-E0203 permanece registrado históricamente aunque no se emita en este perfil, por lo que no se resta del inventario.

La cifra 66 se conserva únicamente como inventario histórico de la iteración 6 de D11–D13. La cifra integrada de presentación es 70. No se cuentan subcausas ni feature_id como códigos. Los 30 feature_id de F0-0002 permanecen sin ampliación en esta ronda.

La familia experimental F0-0002-sim contiene los cinco códigos R conciliados y las dos adiciones R0002/R0103: siete códigos R, separados del inventario NX-E.

La iteración 1 de F0-0002 recibió FAIL según el coordinador. Esta iteración concreta serialización y trabajo, sin atribuirse ejecución ni un nuevo PASS.
