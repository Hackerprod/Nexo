# F4 oráculo: lagunas de K (no se comparan)

Todo lo que K no fija va aquí y NO se compara; nunca se rellena con valores observados del binario.
Fuente de cada laguna: plan del oráculo r3 §3 (iniciales) o hallazgo de implementación (marcado «C»).

| ID | Laguna | Efecto en el oráculo |
|---|---|---|
| LF-01 | Primario de barrera para construcciones K03–K07, K09, K10 distintas de un literal String (K §9.1 solo fija el literal completo, N13 y `"{1}"`). | `barrier.compares_primary` solo es verdadero para `string_literal`/`string_hole`. |
| LF-02 | K reportada y cardinalidad cuando hay ≥2 construcciones pendientes de K distintas; `Print`/`==` de un nominal (K04 o K08), `Array<S>` con recurso (K04 o K05), `ReadLine` con `use` (K09 o K10). | Excluidas de la clase B (`barrier.AMBIGUOUS_KINDS`, `class_b_eligible`). |
| LF-03 | Carácter de control/NUL **crudo** en un literal fuente (K solo trata CR/LF dentro de un Hole). | `literal.analyze` devuelve `other=raw_control_in_text`; no se compara. NUL se prueba por `console_read_line`. |
| LF-04 | Orden de diagnósticos de etapas distintas en una misma fuente. | Los negativos del oráculo tienen exactamente un diagnóstico. |
| LF-05 | H07/H08 con K04 pendiente (la barrera K04 puede preceder al NX-E1403). | Solo se comparan con K04 cerrada. |
| LF-06 | Ticks y `work_consumed` (K no los fija salvo tres oráculos de §8.2). | No se comparan. |
| LF-07 | P04 (API interna de resolución de interfaces). | NO EVALUADO contra el CLI; solo par del modelo (`cli=false` en la fila). |
| LF-08 C | NX-E0001 por escape desconocido: `recognized_prefix`/datos exactos dependen de D15 §6.3, no transcrito en este tramo. | `literal.analyze` devuelve `other=invalid_escape`; familia excluida hasta transcribir D15 §6.3. |
| LF-09 C | NX-E0002 interior a un Hole (`"{1 + }"`, `:` tras Expr): K lo remite al contexto ordinario del parser. | `other=expr_error`; no se compara. |
| LF-10 C | Forma JSON exacta de `related_spans`, `unexpected_token`, `recognized_prefix` y `cause_chain` (K los escribe en notación abreviada; solo P04 muestra `{role,span:{file,start,end}}`). | Se asume `{role,span}`, `{kind,text,span}`, span y `{kind,operation,span,entity,evidence}`; si el binario usa otra forma, el contraste lo registrará como hallazgo de forma y no como defecto de valor. |
| LF-11 C | Primario/cardinalidad de NX-E0006/user_types para un nominal recursivo solo a través de Array (K §4.2 lo difiere sin fijar primario). | Solo se predice el código y el alcance; el primario no se compara. |
| LF-12 C | Orden de ejecución de `Drop` de un campo dentro de A06 con fallo en un argumento (K da eventos pero no ticks). | Solo se comparan eventos (§11.1). |
| LF-13 C | Primario, `data` y código de salida de R0301 (`Fail`) en la variante fallida de A06: K fija solo los eventos `open:0:2` y `close:0:2` y el código R0301. | Solo se comparan código y eventos (`selftest` G4). |
| LF-14 C | Posición del `fn Fallar()` dentro de la fuente de la variante fallida de A06. | La variante se prueba como programa del modelo, sin fuente literal comparada con el CLI. |
| LF-15 C | Expresiones `if`/`match` dentro de un Hole más allá del caso de A08 y de `"{1 + "2"}"`. | El reconocedor de `literal.py` clasifica el resto como `expr_error` (no se compara). |
| LF-16 C | Forma de `outcome.error` para `Err(i64)`: A2 §12.4 enumera `"error":I64` pero su ejemplo literal escribe `"error":"40"`. | Resuelta en O5 por tolerancia: el esperado escribe el error como cadena decimal (`"70"`) y `runner.observed` compara el entero del obtenido como su cadena decimal; ambas formas pasan. Las variantes de K 11 y los programas de la clase A devuelven `Err` y se comparan así. |
| LF-17 C | Primario de la barrera cuando la primera construcción pendiente de la K elegida no es el literal (p. ej. `Print("a")`: la llamada empieza antes que su literal). | Solo se compara el primario si la construcción elegida (primera por posición) es el literal String (`barrier.compares_primary`). |
| LF-18 C | Orden relativo de eventos y líneas de salida de un mismo programa (el modelo O2 lleva las dos listas por separado). | Las V con eventos y salida a la vez se descartan (`unordered`); las V con recursos solo emiten eventos. |
| LF-19 C | Un miembro desconocido de un receptor String es K03 (NX-E1402) o K06 (método de usuario) según K; K §9.1 asigna «resto de §3» a K03. | `generate.constructs_of` lo cuenta como K03 cuando el receptor es una cadena. |
| LF-20 C | Si `check` exige, admite o rechaza `--mock-env` (A2/J/F5 lo definen solo para `run`; los negativos E01–E06 fallan en el controlador). | `generic.invoke` pasa `--mock-env` solo en `run`; un contraste real que lo contradiga se registrará como hallazgo de forma. |
| LF-21 C | Igualdad debug/release y `check`/`run` de diagnósticos para fuentes de `fuzz` sin esperado semántico. | `fuzz` solo exige ausencia de pánico, `internal_error`, `InvalidIr` y timeout confirmado, más la igualdad completa debug/release (diferencial); no compara contra el modelo. |
| LF-22 C | Forma JSON del error del controlador (E01–E06): K 10.6 fija exit 2 y la razón; A2 §12.4 da `{"kind":"driver_error","reason":RAZON}` pero no fija el resto del documento (`stdout`, `events`, `diagnostics`). | El esperado es el documento parcial `{"exit":2,"outcome":{"kind":"driver_error","reason":R}}` y `runner.observed` compara solo esas claves (documento sin `schema_version`). |
| LF-23 C | Entorno v2 fuera de lo que K 6.1 fija: profundidad JSON mayor de 16, claves duplicadas, carácter de control crudo dentro de una cadena JSON, frontera de 1 MiB (`mock_env_too_large`), elementos de las cuatro colas heredadas y orden de validación cuando hay dos defectos a la vez. | `app.parse_json`/`validate` lanzan `ModelGap`; ningún entorno generado lo contiene y no se compara (cada fuente E tiene exactamente un defecto). |
| LF-24 C | Primario de R0002 cuando la reserva que falla es la copia de un String (`let b=a;`): K 8.2 dice «copiar-lugar completo» sin ejemplo ni fila (¿el lugar copiado o el inicializador?). | Las recetas Q de copia (`live_chain`) comparan los 20 campos restantes y omiten `primary_span`; los literales (X24–X26, O13 y recetas de literal/bucle) sí lo comparan. |
| LF-25 C | Ocupación de trabajo, frames, celdas y observaciones en las recetas Q: K 8.3 pide registrar todas las cuotas, pero K no fija ticks (LF-06) ni cómo cuenta un rango `1..=N` las celdas de su iterador. | `charges.occupancy` registra las cuatro cuotas de texto y miembros; el resto se acota (las recetas no imprimen, no tienen frames anidados y sus ticks son órdenes de magnitud inferiores a 16.000.000). No se compara nada de eso. |

## Revisión de O6: estado de cada laguna y decisiones que pide

«Coordinador» = decisión de andamiaje (no cambia el significado de un programa); «Flujo A» = núcleo con más de una respuesta razonable: ejemplo válido e inválido en K antes de fijar un fixture (AGENTS.md, rigor proporcional). Ninguna laguna se rellena con valores observados del binario.

| ID | Estado | Decisión que pide |
|---|---|---|
| LF-01 | abierta | Coordinador: fijar el primario de la barrera NX-E0006 para K03–K07, K09 y K10 antes del primer contraste de esas unidades. |
| LF-02 | abierta | Coordinador: qué K se reporta (y cuántos diagnósticos) con ≥ 2 construcciones pendientes distintas; hasta entonces se excluyen de la clase B. |
| LF-03 | abierta | **Flujo A**: K no dice qué ocurre con CR, LF o NUL crudos dentro de un literal (léxico). Ejemplo válido e inválido en K antes de fijar fixture; NUL ya se prueba por `console_read_line`. |
| LF-04 | abierta | Ninguna por ahora (el oráculo exige un solo diagnóstico); Flujo A si el producto debe emitir varios de etapas distintas. |
| LF-05 | abierta | Ninguna: se cierra al implementar K04 (la barrera K04 deja de preceder a NX-E1403). |
| LF-06 | abierta por diseño | Ninguna: K no calibra ticks salvo tres oráculos de §8.2. |
| LF-07 | abierta | Ninguna: P04 es API interna, NO EVALUADO por la CLI y verificado como par del modelo. |
| LF-08 | abierta | Ninguna del coordinador: trabajo pendiente del oráculo (transcribir D15 §6.3 y §7.1 y habilitar la familia de escape inválido). |
| LF-09 | abierta | Ninguna hasta que el parser F4 implemente expresiones en Holes; después, ejemplo en K (Flujo A) o transcripción del contexto ordinario. |
| LF-10 | abierta | Coordinador, solo si el producto elige una forma JSON distinta de la asumida: se registrará como hallazgo de forma en el primer contraste. |
| LF-11 | abierta | Ninguna: K difiere el primario del nominal recursivo por Array. |
| LF-12 | abierta | Ninguna: solo se comparan eventos. |
| LF-13 | abierta | Ninguna: solo se comparan código y eventos de A06 fallida. |
| LF-14 | cerrada sin acción | La posición de `fn Fallar()` no cambia el resultado; el programa se prueba en el modelo. |
| LF-15 | abierta | Ninguna: ampliar el reconocedor de Holes con `if`/`match` cuando se transcriba LF-09. |
| LF-16 | resuelta en O5 por tolerancia | Ninguna: se aceptan `"error":"70"` y `"error":70`; si el producto usa una tercera forma se registrará como hallazgo de forma. |
| LF-17 | abierta | Coordinador, junto con LF-01. |
| LF-18 | abierta | Ninguna: las V que mezclan eventos y salida se descartan (K §6.1 solo ordena Println: evento y luego stdout, que la clase A y las filas de K §11 sí cubren). |
| LF-19 | abierta | Coordinador: ratificar que un miembro desconocido de un String es K03 (K §9.1 asigna «resto de §3» a K03). |
| LF-20 | abierta | Coordinador, solo si el producto exige o rechaza `--mock-env` en `check`: el primer contraste lo dirá. |
| LF-21 | abierta | Ninguna: `fuzz` solo exige ausencia de pánico, `internal_error`, `InvalidIr` y timeout, más la igualdad debug/release. |
| LF-22 | abierta (nueva, O5) | Ninguna: se compara `exit` y `outcome` del controlador. |
| LF-23 | abierta (nueva, O5) | Ninguna: nada de lo no fijado se genera ni se compara (profundidad > 16, claves duplicadas, control crudo en JSON, frontera de 1 MiB, colas heredadas). |
| LF-24 | abierta (nueva, O5) | **Flujo A**: primario de R0002 cuando falla la copia de un String («copiar-lugar completo»: ¿el lugar copiado o el inicializador?). Ejemplo en K antes de comparar `primary_span` en esas recetas. |
| LF-25 | abierta (nueva, O5) | Ninguna: ocupación de trabajo, frames y celdas de las recetas Q solo acotada. |

Resumen: piden decisión del coordinador LF-01, LF-02, LF-17, LF-19 y, solo ante un contraste adverso, LF-10 y LF-20; piden Flujo A LF-03 y LF-24; ninguna otra necesita decisión.
