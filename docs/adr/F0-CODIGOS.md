# F0-CODIGOS — Catálogo único de diagnósticos de D11–D13

Estado: propuesta de la iteración 6; pendiente de nueva validación.
Este archivo es el único registro de significado, propietario y subcausas.
Las tablas de los ADRs referencian entradas; no crean catálogos paralelos.
No modifica los diagnósticos emitidos por F0-0001 ni la base 0.2.

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
| NX-E0101 | F0-0001/tipos | Binding no declarado. |
| NX-E0102 | F0-0001/tipos | `type_mismatch`: tipo incompatible; `not_indexable`: indexación no aplicable; `resource_required`: operación que requiere recurso; `field_requires_grouped_call`: miembro campo usado en forma de llamada de método x.m(); la invocación del campo se escribe (x.m)() (D13 G.1, C52d/C52c). |
| NX-E0103 | F0-0001/tipos | Elemento de array vacío no inferible sin anotación. |
| NX-E0104 | F0-0001/nombres | Binding duplicado en el mismo ámbito. |
| NX-E0105 | F0-0001/acceso | Acceso de modificación solicitado sobre binding inmutable, incluida asignación, receptor inout y match inout. |
| NX-E0107 | F0-0001/números | Literal fuera del rango i64 del corte. |
| NX-E0110 | Propiedades selladas, mantenimiento D11 | Una impl o derivación de usuario intenta afirmar una propiedad/protocolo reservado al compilador (incluidos Call* y SuspendSafe). |
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

Limit, observed y profile_id son datos obligatorios; sus valores concretos
no crean subcausas. Se debe publicar el perfil antes de ejecutar el caso.
Los siete presupuestos propuestos no acreditan un evaluador/solver existente;
su contabilidad reproducible sigue NO EVALUADA (fila K19). Registrar otro
budget_kind requiere modificar este catálogo. El presupuesto de ejecución
del simulador **no** se mezcla aquí: mantiene R0001. Los tipos desconocidos
no se aceptan como seguros al agotar ningún presupuesto.

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
Hay 66 códigos canónicos vigentes, sin reutilizar identidades.
N4 incluye S39/S33m en las ubicaciones y retira el duplicado E31c en D13.
La revisión 5 recibió FAIL según el coordinador; la 6 sigue sin dictamen.
No se atribuye aprobación ni ejecución a esta revisión documental.
