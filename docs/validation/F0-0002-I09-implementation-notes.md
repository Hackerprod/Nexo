# F0-0002 I09 — notas de implementación

## Alcance e invariantes

I09 analiza efectos una sola vez sobre la IR baja después de tipos y ownership. La declaración `with` se conserva desde el AST con spans de cada token. `check_proposed` continúa siendo API de solo tipos. Se retiró la emisión del límite `implementation_limit` para I09; el valor sigue admitido por comprobaciones de otras fases.

Cada resumen guarda dos conjuntos monotónicos: `known` y `unknown` para `alloc`, `block`, `suspend` e `io`. Los resúmenes reales se conservan aunque la función incumpla una cota; no hay Poison. El solucionador obtiene componentes fuertemente conexas del grafo de llamadas y procesa primero las de callees. Cada componente resuelve sus resúmenes mediante iteraciones síncronas sobre la iteración anterior hasta alcanzar la estabilidad del retículo finito. No introduce contador de pasos de I10a.

El manifiesto asigna `io` a Print, Inspect, MockOpen y MockBlockingOpen; los cuatro bits desconocidos a MockUnknown; `alloc` a construcción y copia de arrays; `io` a MockResource.drop; y `block,io` a MockBlockingResource.drop. Len, TryAdd, AssertEq, Fail y DropData no tienen efecto propio: DropData obtiene el efecto del valor destruido. Los efectos de llamadas directas se propagan por identidad de función.

## Contratos y testigos

Se emite un diagnóstico por función y restricción incumplida: NX-E0412 para `no_alloc`, NX-E0413 para `no_block`, NX-E0414 para `no_suspend` y NX-E0415 para `no_io`. Una restricción falla si su bit aparece en `known` o `unknown`. Las restricciones escritas se procesan por el orden canónico del código. Cada diagnóstico lleva `contract_origin`, `required_effect`, `effect_evidence`, `cause_chain` y spans validados.

La selección de testigo usa esta precedencia: evidencia conocida, menor número de aristas, offsets de las aristas y finalmente identidad. La prueba `known_effect_precedes_shorter_unknown_witness` compara la llamada conocida a Hoja con el testigo desconocido más corto. La cadena de destrucción es `value.drop` seguida de `resource.drop`; los destructores se incluyen en el resumen de la función.

## Salidas y propiedad en el CFG

`ExitInfo` identifica un sitio `block,index` del CFG y conserva clase, span, locales posiblemente poseídos y locales definitivamente destruidos. El verificador comprueba unicidad, cobertura bidireccional con los sitios IR esperados, ancla, clase de span y rangos de locales. Return, BranchResult (`?`), Fail y grupos de DropLocal tienen sitio explícito. `?` y Fail apuntan a sus operaciones porque no son terminadores en la IR actual.

`scoped_block` y los bloques de valor emiten `Op::DropLocal` con el `close_span` real de `}`. Un grupo de DropLocal del mismo cierre registra una sola salida Fallthrough. La salida implícita de función se ancla al terminador Return y usa `close_span` cuando es fallthrough. El parser guarda ese token en todos los constructores de Block.

La liveness recorre el CFG: `maybe_owned` se une en los predecesores; `gone` se intersecta. La escritura añade propiedad, TransferValue la quita sin marcar destrucción y DropBinding/DropLocal la quitan y añaden a `gone` solo para caminos que dominan la salida. La limpieza implícita por salida usa `maybe_owned − gone`. `ownership_flow` valida antes la propiedad y los consumos; esta fase no es un análisis local puro.

Reglas de salida comprobadas con fuentes:

1. `fn Main() with no_block{if true{let r=MockBlockingOpen(1);}}`: NX-E0413, `r` `[36,37)`, cierre `}` `[58,59)`.
2. `fn Main() with no_block{let r=MockBlockingOpen(1);Drop(r);}`: `r` `[28,29)`, `Drop` `[50,54)`, `Drop(r)` `[50,57)`. El testigo explícito es `value.drop` → `resource.drop`; no se vuelve a contar como DropLocal implícito.
3. `fn Main() with no_block{let r=MockBlockingOpen(1);if true{Drop(r);}}`: el Drop condicional no domina la salida de función; la salida mantiene el recurso en `maybe_owned` y falla NX-E0413.
4. En `fn F()->Result<I64,I64>{return Err(1);} fn Main()->Result<I64,I64> with no_block{let r=MockBlockingOpen(1);let n=F()?;return Ok(n);}`, el diagnóstico es NX-E0413, `r` `[85,86)`, contrato `[72,80)`, salida `F()?` `[113,117)`.

## Variantes, restricciones múltiples y fixtures

Los bindings Result siempre usan la unión de variantes, por lo que `Result<MockBlockingResource,I64>=Err(1)` y el espejo `Result<I64,MockBlockingResource>=Ok(1)` incumplen `no_block`. Solo se acepta evidencia de variante para temporales Ok/Err consumidos por Drop, para temporales prestados a una función de usuario y para el borde Err de `?`. Dado que la sintaxis B no puede aportar un tipo a `Drop(Ok(...))` sin dejar el otro parámetro de Result irresuelto, `typed_result_temporary_drop_uses_its_ir_variant_evidence` fabrica IR desde un binding tipado y cambia su consumo a DropData. El tipo `Result<MockBlockingResource,MockResource>` distingue el destructor activo del inactivo y mata la mutación que invierte Ok/Err. El fixture de `?` que retorna un payload de recurso desde `F()?` y `return Ok(move n)` se acepta, sin falso E0413.

`fn Main() with no_io,no_alloc{MockUnknown();}` produce E0412 y luego E0415 con primario `[30,41)` y `unknown_effect`. `fn Main() with no_io,no_alloc{Print(1);let a=[1];}` produce E0415 `[30,35)` y luego E0412 `[45,48)`; Print solo aporta `io`. También se fija que una primera restricción satisfecha (`no_alloc`) no impida emitir el incumplimiento posterior (`no_io`) para Print. `fn Main(){MockRequest();}` continúa como ejemplo I11a `implementation_limit`, primario `[10,21)`.

## Lagunas ADR y límites

- `adr_gap_exit_span`: el ADR no decide el span de Return/`?`/Fail; se usa la construcción causante. Para un temporal descartado, el primario es el nombre de la llamada y `value.drop` abarca la expresión sin terminador. El paso `resource.drop` se registra para bindings y destrucción explícita cuando corresponda, no para la destrucción implícita de un temporal.
- `adr_gap_drop_leaf_choice`: no se especifica qué hoja gana entre Drop explícito en una rama y limpieza implícita en otra; se conserva el testigo canónico implementado.
- `adr_gap_known_effect_precedence_redaction`: solo queda la ambigüedad de redacción de §13.15; se aplica evidencia conocida antes de distancia, offsets e identidad.
- `adr_gap_call_restriction_fixture`: `with {call(no_io)}` usa el resumen de llamada y tiene regresión local `call_restrictions_use_transitive_effect_summaries`; el ADR no publica un fixture normativo para esa forma.
- `adr_gap_dead_tail`: el lowering omite código posterior a divergencia incondicional; esas operaciones ausentes no aportan efectos. Se fija el comportamiento actual con regresión.
- PP06 conserva el resultado 3 de D15. Salidas por cuota y `effect_fixpoint_steps` pertenecen a I10a.

No se tocó `src/check/types.rs`, el punto A de I08, `docs/baseline/` ni las líneas pendientes de la enmienda ADR §9.1/§20. La población y el estado se regeneran con `scripts/i09_mutaciones.py` y `scripts/i09_estado.py`; el estado consume la evidencia versionada y la población.

## Reglas de evidencia y población de SCC

El verificador exige `maybe_owned ∩ gone = ∅` en cada salida; la regresión `verifier_rejects_an_exit_local_as_both_maybe_owned_and_gone` inyecta la intersección en un `ExitInfo` y comprueba el rechazo. Por esta invariante, el mutante que cambia `continue` por `break` para locales ya destruidos es equivalente en todos los estados verificados y queda fuera de la población. También se excluyó un cambio `&& let` a `|| let`, que produce Rust sintácticamente inválido.

La población también omite quitar `finish.push(node)`: es la única inserción que fija el tipo de elemento de `finish`, así que el compilador rechaza la mutación antes de probar el comportamiento. La regresión `call_components_are_strong_and_order_callee_before_caller` cubre SCC separados con callee antes que caller y un diamante que vuelve a visitar un nodo. Se omiten además los cambios `&& let` a `|| let`, que hacen inválidos los let-chains en Rust.

También se omite quitar `local.push(function_intrinsic_effect(f))`, pues dejaría `local` sin un tipo inferible y el compilador detiene el programa antes de ejecutar el solver.

Las ramas de omision de nodos ya vistos en ambos recorridos de SCC no se mutan de `continue` a `break`: aristas deduplicadas y recorrido LIFO impiden duplicados en la pila, y la lista de finalizacion contiene cada nodo una sola vez, por lo que ambas ramas son inalcanzables con un grafo IR valido.

Omitir la asignación `summaries = next` dentro de la rama estable tampoco cambia el resultado: esa rama solo se ejecuta cuando `next == old`; se excluye como equivalente.

Se excluye el `continue` a `break` del fallback de búsqueda AST: cada función verificada del IR se baja desde una función del mismo AST, por lo que ese fallback no se alcanza con entradas válidas.

También es inalcanzable cambiar a `break` el salto de `unrestricted`: la gramática rechaza `unrestricted` mezclado con otra restricción antes de bajar a IR (`NX-E0417/unrestricted_mixed`).

Se omite cambiar a `break` el fallback de nombre de efecto desconocido: el parser valida las restricciones contra el manifiesto de efectos, así que el fallback no se alcanza desde una fuente aceptada.

Para mutación release, el harness mide una vez el módulo base `ir::effects::tests` y usa un límite de 30 s para ese módulo; una prueba base de 0,06 s que no termina bajo mutación se registra como detectada por no terminación.

`verifier_rejects_missing_known_or_unknown_local_effect_bits` forja por separado un resumen sin el efecto conocido de Print y otro sin los efectos desconocidos de MockUnknown; ambos deben fallar inclusion local inductiva.

`verifier_rejects_missing_known_or_unknown_callee_effect_bits` forja resúmenes de caller que omiten por separado efectos conocidos y desconocidos del callee; el verificador inductivo debe rechazar ambos.

El test `implicit_droplocal_violates_no_block_at_owner_and_records_cfg_exit` antepone un array vivo, cuyo destructor no aporta efecto, al recurso bloqueante: saltar ese local de efecto cero no debe cortar el análisis de los demás locales de salida.

La misma regresión de salida con DropLocal verifica que varios locales del bloque conserven el mismo sitio de salida y generen un solo diagnóstico por función y restricción.

`multiple_droplocals_share_one_verified_block_exit` verifica directamente dos instrucciones `DropLocal` con el mismo span y una unica salida anclada al primer indice del grupo.

`verifier_rejects_an_exit_anchored_to_another_closing_brace` falsifica el span sin cambiar clase ni el sitio, y comprueba que la ancla se valida contra la instruccion correspondiente.

`return_exit_may_start_at_the_return_terminator` verifica el límite inclusivo del span de un terminador Return en el ancla de ExitInfo.

La regresión del span Return excluye el `;`; la salida de `return;` ocupa solo el token `return` y se rechaza un rango que exceda su final.

`try_and_fail_exit_classes_require_operation_anchors` muta el tipo de salida conservando un terminador Return y verifica el rechazo: Try y Fail deben apuntar a sus operaciones IR.

`verifier_rejects_a_return_exit_on_a_non_return_terminator` añade un sitio forjado en una salida de bloque que termina con Branch/Jump; Return no puede anclarse ahí.

El verificador rechaza ahora un span Fallthrough cuyo inicio cae al final del texto; `verifier_rejects_a_fallthrough_span_at_source_end` muta tanto ancla como instruccion para aislar el limite de cierre.

El mutante `>=` a `>` de ese limite se omite porque la consulta `source_text.get(index)` incluida en la misma condicion ya rechaza `index == len`; el cambio no altera la aceptacion.

La prueba de cierre también alinea una ancla e instrucción con un byte interior distinto de `}` para comprobar la segunda rama de validación, además del EOF.

`exit_info_uses_the_matching_ast_function_for_implicit_returns` comprueba dos funciones con cierres distintos para fijar la correspondencia AST/IR al calcular salidas implicitas.

`scalar_droplocals_share_one_exit_without_ownership_liveness` cubre el camino de ExitInfo cuando no hay locales posibles y confirma el mismo agrupamiento de cierres.

Se excluye el mutante de borde `span.start <= return.start` a `<`: el span del terminador incluye la llave de apertura del bloque y toda sentencia Return empieza despues de ella.

`explicit_return_exit_spans_match_their_own_ast_returns` comprueba retornos explícitos en ambas ramas y conserva el span de cada construcción original.

`exit_population_ignores_out_of_range_cfg_predecessors_without_panicking` forja una arista cuyo destino es igual al total de bloques y fija el límite estricto de IDs durante la liveness.

`ownership_liveness_propagates_across_cfg_predecessors` conserva un recurso poseído al cruzar una arista de rama hasta su salida, comprobando la unión `maybe_owned` del análisis de propiedad y salidas del CFG.


## Cierre de liveness y evidencia de mutación

El análisis `maybe_owned`/`gone` usa una cola de trabajo sobre el CFG: recalcula un bloque cuando cambia la salida de un predecesor y propaga solo a sus sucesores. Los conjuntos se representan en bitsets y el punto fijo es finito por el dominio de locales. Las iteraciones síncronas se conservan para los resúmenes de efectos; el verificador vuelve a comprobar que cada salida conserva conjuntos disjuntos y anclas biunívocas.

`resource_parameters_are_owned_at_entry_and_dropped_on_exit` fija el estado inicial de parámetros recurso. `definite_drop_propagates_to_nested_cfg_exits` fija la propagación de `gone` después de un Drop explícito hasta salidas en bloques CFG posteriores. Para probar el recorrido de efectos con un elemento cuyo destructor no tiene efecto, la regresión antepone un array vivo al recurso bloqueante y ordena los IDs locales para que el caso de efecto cero preceda al destructor; el mutante de terminar el recorrido ahí no puede ocultarlo.

El generador excluye mutaciones con sintaxis Rust inválida y eliminaciones del único `push` que solo impiden inferir el tipo de un `Vec`. Las equivalencias que se conservan en la población quedan marcadas y justificadas en `results.jsonl`. Para temporales, `temp_count` es el máximo TempId definido más uno: cada ID válido tiene una definición, la inicialización de `owned_temps` se sobrescribe y `id.0 == temp_count` no ocurre. El runner separa la compilación del mutante de la medición de pruebas, ejecuta cada sustitución en una copia de trabajo y verifica la restauración SHA-256. Las sustituciones de producción se midieron con pruebas release; las suites completas debug y release se registran por separado.


La población excluye las mutaciones de la marca de pertenencia a la cola (`queued[*successor] = true` omitido o sustituido por `false`): solo permiten encolar IDs duplicados. Cada cambio de valor sigue encolando sus sucesores, cada visita aplica la misma transferencia monótona y el retículo finito converge al mismo punto fijo; el cambio afecta el coste de planificación. La regresión `ownership_liveness_propagates_across_cfg_predecessors` comprueba que la arista sucesora no se omita: el recurso adquirido solo en el cuerpo del bucle tiene que propagarse por la retroarista hasta la salida.

`definite_drop_state_propagates_across_cfg_predecessors` conserva `gone` al cruzar una arista desde un `Drop` explícito hasta la salida de un bloque anidado con otro recurso vivo. La mutación que elimina `gone_in[bi] = next_gone` hace fallar esta aserción en una copia independiente del proyecto; el caso evita depender de que la unión final de efectos oculte una limpieza duplicada.

`transitive_witness_preserves_call_chain_and_diagnostic_operation` comprueba dos llamadas anidadas, el orden completo de la cadena canónica y el span primario del callsite. Mata la omisión de `witness.chain = chain`. La prueba verifica ahora que una cadena transitiva emita `Diagnostic.operation = function.call`; la mutación que borra esta asignación se incluye y debe fallar la regresión. También se excluye la mutación `move ||` a `move &&`, que no compila por sintaxis accidental.

El mismo test incorpora un ciclo recursivo antes de la llamada a `Leaf`; romper el recorrido al ver la arista ya visitada ocultaría el efecto posterior y hace fallar la cadena. Se excluye el cambio `continue` a `break` en la búsqueda de un callee que no figura en la tabla, porque `verify_program` rechaza cualquier `CallDirect` a un callee desconocido antes del análisis de efectos.


`published_effect_fixture_fields_match_the_literal_contract` fija el kind e ID del parámetro como `parameter` y `parameter:9`. La mutación que omite la asignación del span de token se excluye como equivalente: el span del parámetro empieza en su identificador por gramática, así que ambos spans tienen el mismo offset. `implicit_droplocals_do_not_mark_non_io_cleanup_as_io` recorre los dos emisores de limpieza (`scoped_block` y `scoped_value_block`) y fija `effect_io=false` para cada `DropLocal`; ambos cambios se detectaron al ejecutar esa regresión exacta.


## Mediciones

Las cifras y resultados de ejecución se mantienen exclusivamente en el estado generado `docs/validation/F0-0002-I09-estado.md`.

## Correcciones del informe independiente I09

Las regresiones de fuente fijan la operación y cada campo de la cadena literal F2-E04 de D15 §8.12, además de los spans de hoja de E05/E06, identidad `parameter:9`, identidad `binding:28`, `prelude:print`, y el tipo de `resource.drop` de un binding `Result`. Las llamadas a funciones de usuario conservan `Diagnostic.operation = function.call`; la hoja intrínseca señala solo su nombre.

Los temporales propietarios prestados a una función de usuario o a una intrínseca de lectura se destruyen en el llamador después de la llamada (§§4.3, 6.4 puntos 3 y 5, 8.1 y 13.15). El análisis atribuye el efecto a `value.drop`, con primario en el nombre de la construcción y paso temporal sobre la expresión completa; no inventa `resource.drop` para un temporal implícito (§§8.1–8.2 y 13.15). El código posterior a un retorno incondicional se comprueba hipotéticamente y sus errores independientes se conservan, pero no contamina las continuaciones alcanzables (§6.2). La destrucción explícita usa el span completo de `Drop(...)` para `value.drop`, sin terminador, y mantiene el primario sobre el nombre `Drop` (§§8.2 y 13.15). Copiar `ReadIndex` y `Result<Array<...>,...>` aporta `alloc`; una lectura de elemento escalar prestado sigue siendo positiva.

La siembra de recursos de parámetros en el análisis de salida cuenta como propietario solo el modo `move`; `view` y `edit` siguen siendo préstamos (§§4.3, 6.4 puntos 3 y 5, 8.1). Las regresiones cubren ambos modos prestados con tipos de recurso bloqueante y no bloqueante, y confirman el efecto de salida de `move`.

Un temporal propietario prestado a una llamada de usuario se destruye en el llamador al terminar la llamada. El diagnóstico conserva NX-E0413, el contrato del llamador, el primario en `MockBlockingOpen` y un único paso `value.drop` con la expresión temporal completa y entidad temporal (§§8.1–8.2, 13.15); el gemelo `Inspect` fija el mismo contrato. Los temporales transferidos a parámetros `move` se limpian en la función que los recibe.

`populate_exit_info` usa una única ruta de construcción, incluso sin locales poseídos. Los spans de Return excluyen el `;`. El caso de salida relacionada para un local anidado tras `return` o `Fail` aún puede elegir la llave de cierre; queda como deuda secundaria de evidencia. La búsqueda de testigos enumera caminos simples; su coste exponencial queda registrado para I10a, con una escalera pequeña como regresión del resultado actual.

La población mutacional conserva los operadores de efectos de I09 y añade cuatro sustituciones dirigidas a esta corrección: propiedad de parámetros solo en `move`, retención del span de temporales, span completo del paso de destrucción de un temporal explícito y span completo del paso de destrucción de un binding explícito. Las cuatro se detectan en release con regresiones concretas. La mutación de `Fail` mantiene su caso `no_block`; las variantes de orden de testigos y salida `Fail` permanecen en la población. El resultado medido y las equivalencias del dominio verificado se generan desde `scripts/i09_mutaciones.py` y `scripts/i09_evidencia_mutaciones.py`.
