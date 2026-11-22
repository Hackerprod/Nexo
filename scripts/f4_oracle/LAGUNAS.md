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
