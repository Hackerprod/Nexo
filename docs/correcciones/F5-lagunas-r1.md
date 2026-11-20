# F0-0005 — propuesta normativa de lagunas H1–H5 (REVERTIBLE)

## Cambios respecto a la versión anterior (r3)

- H3 fija los conjuntos admisibles incondicionales del perfil F5 para H3-A–E y aclara los resultados históricos de `for i in (0..<2){}` según el perfil seleccionado.

Ámbito: solo `f0-0005-b-v1`. Cada bloque entre «Texto para insertar» es redacción normativa lista para incorporar en `docs/adr/F0-0005-operators-control-collections.md`; esta propuesta no modifica el ADR. Los ejemplos son archivos `case.n` ASCII sin LF final, con offsets de bytes desde cero e intervalos semiabiertos. `message` es presentación, conforme a A2 §13.6; el resto del objeto JSON es contractual. En las tablas, un positivo tiene `diagnostics=[]`, `check`/`run` exit 0 y outcome unit, salvo indicación expresa. Las fixtures de perfiles anteriores conservan sus esperados propios. H1-A…H5-A son etiquetas locales de ejemplo, **no** IDs F5-xxx; el coordinador asigna los IDs definitivos al insertar las filas en §9.2.

## H1 — barrera global de L08 (REVERTIBLE)

**Ubicación:** §8.2, inmediatamente después de la fila «Cuotas nuevas aún pendientes» (línea 618); referencia aclaratoria en §10 después de la línea 1114.

**Texto para insertar:**

> En `f0-0005-b-v1`, `check` y `run` aplican las filas L01–L07 de §8.2 en sus etapas públicas `types` u `ownership` mientras falte la subtarea respectiva. La barrera global L08 de §10 se consulta **solo si la fuente supera parse, names, types, ownership y effects y llega a lower** con una capacidad nueva; entonces emite `NX-E0006`, `feature_id=implementation_limit`, `pending_scope=L08`, `lower/ir.lower`, sobre el primer nodo fuente de esa capacidad. Así, el éxito del arnés interno no autoriza ejecución fuente antes de completar efectos, limpieza y cuotas, mientras los rechazos L01–L07 de §8.2 siguen siendo públicos. En particular, L01 pendiente impide aún comprobar el dominio de `*`: `true*2` recibe L01/types sobre `*`; con L01 completo recibe el error de tipos de F5-O06 y nunca L08. Un error real de sintaxis, nombres, tipos, propiedad o efectos, o una exclusión `later_adr`, conserva su etapa y precedencia antes de la barrera lower. Entre pendientes emitibles en la misma etapa se usa §8.2: menor offset, L numérico y bytes UTF-8. Al completar L08 desaparece la barrera; las capacidades incluidas deben aceptarse o recibir su diagnóstico semántico. Los perfiles anteriores conservan sus contratos.

**Ejemplos exactos:**

| Estado y clase | Fuente | Primario / resultado |
|---|---|---|
| L01 completo, L08 pendiente; salida pública de §8.2 fila 618 y §10 | `fn Main(){let x=1*2;}` | L08/lower `[16,19)` (`1*2`), JSON H1-A. |
| L01 pendiente; salida pública de §8.2 fila 612 y §12.1 fila 1579 | `fn Main(){let x=1*2;}` | L01/types `[17,18)` (`*`), JSON H1-B. |
| L01 pendiente; la misma frontera para `!=` de F2-S28 | `fn Main(){let x=1!=2;}` | L01/types `[17,19)` (`!=`), JSON H1-H. |
| L01 pendiente; dominio todavía no comprobable | `fn Main(){let x=true*2;}` | L01/types `[20,21)` (`*`), JSON H1-D. |
| L01 completo, L08 pendiente; error semántico previo | `fn Main(){let x=true*2;}` | NX-E0102/types `[16,20)` (`true`), JSON H1-E; F5-O06. |
| L06 pendiente; salida pública de §8.2 fila 616 y §12.3 fila 1737 | `fn Main(){for x in [1]{}}` | L06/types `[10,13)` (`for`), JSON H1-F. |
| L07 pendiente; salida pública de §8.2 fila 617 | `fn Main(){var a=[1];Push(edit a,2);}` | L07/types `[20,24)` (`Push`), JSON H1-G. |
| L05 completo, L08 pendiente; exclusión antes de lower | `fn Main(){let r=0..<3;}` | `[17,20)` (`..<`), JSON H1-C; equivale a F5-D01. |
| Positivo heredado | `fn Main(){let x=1+2;}` | `diagnostics=[]`; el programa no requiere capacidad nueva. |

**Cruce literal de las cuatro filas objetadas:** §8.2 línea 612 fija `implementation_limit/L01` en `types` sobre el operador para `*` y `!=` pendientes (`fn Main(){let x=1*2;}`, H1-B; `fn Main(){let x=1!=2;}`, operador `[17,19)`, frontera F2-S28 de §12.1). §8.2 línea 617 fija L07/types sobre el destino resuelto de `Push`, `Pop` o `JoinArrays` pendientes (H1-G; la firma de §4.1 y F5-B05 identifican `Push`). §12.1 línea 1579 fija **solo** L01 para `*`/`!=` antes de L01, no L08 (H1-B y F2-S28). §12.3 línea 1737 fija L06 para `for` Array antes de L06 (H1-F; la fuente de F2-F25 en §12.1 es otra instancia). La fila L06 de §8.2 línea 616 da la misma etapa/ancla. Estas son salidas de `check`/`run`, no diagnósticos privativos del arnés. L08 de §8.2 línea 618 y §10 líneas 1108–1114 fija el rechazo posterior en lower para una fuente nueva que haya pasado esas etapas (H1-A). El §10 habla de impedir **aceptación** de fuente antes de L08; su frase general `check/run mantienen implementation_limit/L08` se lee condicionada a llegar a lower. La oración incompatible de `docs/correcciones/F5-r1.md` §«Decisiones del coordinador», H1, que reserva L01–L07 al arnés, debe alinearla el coordinador antes de usar ese plan; esta propuesta no edita el plan.

H1-A:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"lower","message":"Capacidad pendiente de implementación en L08.","operation":"ir.lower","primary_span":{"file":"case.n","start":16,"end":19},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L08"}}
```

H1-B:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en L01.","operation":"types.check","primary_span":{"file":"case.n","start":17,"end":18},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L01"}}
```

H1-C:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Recognized construct is outside the profile.","operation":"types.check","primary_span":{"file":"case.n","start":17,"end":20},"related_spans":[],"cause_chain":[],"feature_id":"range","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"later_adr"}}
```

H1-D:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en L01.","operation":"types.check","primary_span":{"file":"case.n","start":20,"end":21},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L01"}}
```

H1-E:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0102","subcause":"type_mismatch","rule":"NX-E0102/type_mismatch","explanation_id":"NX-E0102/type_mismatch","severity":"error","stage":"types","message":"Invalid name, call or type in the proposed profile.","operation":"types.check","primary_span":{"file":"case.n","start":16,"end":20},"related_spans":[{"role":"found_type","span":{"file":"case.n","start":16,"end":20}}],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"expected_type":"I64","found_type":"Bool"}}
```

H1-F:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en L06.","operation":"types.check","primary_span":{"file":"case.n","start":10,"end":13},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L06"}}
```

H1-G:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en L07.","operation":"types.check","primary_span":{"file":"case.n","start":20,"end":24},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L07"}}
```

H1-H:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en L01.","operation":"types.check","primary_span":{"file":"case.n","start":17,"end":19},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L01"}}
```

**Compatibilidad:**

| Fila vigente | Igual | Comprobación |
|---|---|---|
| A2 TD01–TD07, `+`, `<`, `==` | Sí | Diferencial sobre las fuentes históricas en su perfil y en F5 después de L08; la barrera solo afecta fuentes con capacidad nueva. |
| F2-F04 / F5-D01 | Sí | El rango almacenado conserva `range/later_adr`, primario `[17,20)` en F5-D01; no se transforma en éxito público. |
| F5-O01–O13, B01–B09 | Sí | Los errores semánticos de §9 rigen desde que su subtarea está completa y preceden a L08; los positivos de capacidad nueva solo se ejecutan después de L08. |

**Impacto medido:** en `git archive 5cf16fd`, perfil histórico J, `fn Main(){let x=1*2;}` da NX-E0002/parse `[17,18)`; `fn Main(){let r=0..<3;}` da `range/later_adr` en parse `[17,20)`. El perfil F5 aún no existe en esa base. Ninguna salida fijada cambia. **Alternativa descartada:** emitir L08 antes de comprobar `types` para toda fuente nueva; ocultaría las cuatro salidas públicas de §8.2/§12 y el NX-E0102 de F5-O06. Tampoco se acepta ejecutar fuentes nuevas mientras falte L08: §10 lo prohíbe.

## H2 — `const` antes de L09 y L10 (REVERTIBLE)

**Ubicación:** §8.2, después de la fila «Const tipos/pureza pendiente; evaluación pendiente» (línea 619), antes de las exclusiones `later_adr`.

**Texto para insertar:**

> En el perfil F5 se reconoce la declaración superior `const` de D13 y §5 desde la apertura del perfil; no se clasifica su forma escalar incluida como `comptime/later_adr`. Mientras L09 no esté completa —incluido L09 en curso— la fuente escalar de la tabla recibe exclusivamente `NX-E0006/implementation_limit`, `pending_scope=L09`, `types/types.check` sobre el token `const`. Cuando L09 está completa, se comprueban nombres, tipos, dependencias, pureza y permisos con sus diagnósticos de §9; si no hay error previo y L08 está pendiente, la barrera L08 de §8.2/§10 se aplica en `lower/ir.lower` sobre `const`, primer nodo fuente de la capacidad nueva. Si L08 está completa y L10 pendiente, se usa `implementation_limit/L10`, `lower/ir.lower` sobre la expresión inicializadora escalar. L10 solo puede completarse después de L08 y L09 (§10), y entonces la fuente escalar válida se acepta. `comptime/later_adr` queda reservado a los casos excluidos de §8.2, como `comptime` fuera de const o const puro no escalar, con sus etapas y precedencias; los perfiles históricos conservan su rechazo parse. Ninguna rama ejecuta inicialización global antes de L10.

**Ejemplos exactos:**

| Estado y clase | Fuente | Primario / resultado |
|---|---|---|
| L09 pendiente, incluso en curso; L08 cualquiera | `const k:I64=comptime 1;fn Main(){}` | L09/types, `const` `[0,5)`, JSON H2-A. |
| L09 completa, L08 pendiente, L10 necesariamente pendiente | misma fuente | L08/lower, `const` `[0,5)`, JSON H2-B. |
| L09 y L08 completas, L10 pendiente | misma fuente | L10/lower, inicializador `1` `[21,22)`, JSON H2-C. |
| L10 completa, por dependencia L09 y L08 completas | misma fuente | `diagnostics=[]`, outcome unit. |

La partición usa el estado **completada/pendiente**, no la fecha de inicio: L09 «en curso» pertenece únicamente a la primera fila. El estado L10 completa con L08 pendiente es imposible por §10. En cada fila negativa se emite un único diagnóstico público de `check`/`run`, con `data` exactamente `{"pending_scope":"L09"}`, `{"pending_scope":"L08"}` o `{"pending_scope":"L10"}`, respectivamente. Los errores anteriores, si ya son comprobables, conservan su precedencia; la tabla fija el caso escalar válido indicado.

H2-A:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en L09.","operation":"types.check","primary_span":{"file":"case.n","start":0,"end":5},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L09"}}
```

H2-B:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"lower","message":"Capacidad pendiente de implementación en L08.","operation":"ir.lower","primary_span":{"file":"case.n","start":0,"end":5},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L08"}}
```

H2-C:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"lower","message":"Capacidad pendiente de implementación en L10.","operation":"ir.lower","primary_span":{"file":"case.n","start":21,"end":22},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L10"}}
```

**Compatibilidad:**

| Fila vigente | Igual | Comprobación |
|---|---|---|
| F2-F21 histórico | Sí | `comptime/later_adr`, parse `[0,5)`, permanece en su perfil; en F5 §8.2 línea 619 rige L09/types desde el principio y §12.1 lo convierte en positivo después de L10. |
| F5-K02/K03/K04 y N02/N03/N04/N06–N09 | Sí | Impureza, ciclo, nombre, alcance y propiedad conservan su diagnóstico de §9 después de L09; L10 no los tapa. |
| A2 TD y DG | Sí | No contienen `const` de este corte; diferencial por fuente y perfil. |

**Impacto medido:** la copia de `5cf16fd`, que solo conoce el perfil histórico J, da `comptime/later_adr`, parse `[0,5)` para esta fuente. Ese resultado sigue siendo histórico, no el oráculo F5 H2-A. Ninguna salida fijada cambia. **Alternativa descartada:** conservar `comptime/later_adr` en F5 hasta iniciar L09; trataría una capacidad incluida como excluida, contradiría §8.2 línea 619 y solaparía dos estados de implementación.

## H3 — comparación y rango no asociativos (REVERTIBLE)

**Ubicación:** §2.1, después de «Comparaciones y rangos no encadenan» (línea 92); §12.1 después de la tabla de enmiendas (línea 1584); §12.2 después de sus contextos gramaticales (líneas 1668–1687); fixtures después de §9.2.

**Texto para insertar:**

> `CompareExpr = RangeExpr [("<" | "<=" | ">" | ">=" | "==" | "!=") RangeExpr]` y `RangeExpr = AddExpr [("..<" | "..=") AddExpr]` admiten a lo sumo un operador de su propio nivel sin agrupación, tal como D15 §3.5 líneas 339–340 y D14 §3.2 línea 203. Los seis comparadores están en un **único nivel**, de modo que `==` no tiene menor precedencia que `<`. Se rechaza el segundo comparador de `1<2<3`, `a<b<c`, `a==b==c` y `a<b==c`, y el segundo rango de `0..<1..<2`: todos son inválidos. El rechazo es `NX-E0002`, subcause `null`, `parse/syntax.parse`, primario y `unexpected_token` sobre el segundo operador completo. Se incorporan expresamente a §12.2 los contextos `grammar.CompareExpr` y `grammar.RangeExpr` para estas dos producciones; aún no figuran en el ADR vigente. `expected_tokens_kind=admissible_set`; en el perfil F5, H3-A–D tienen siempre `expected_tokens=[";","and","or"]` y H3-E tiene siempre `expected_tokens=["!=",";","<","<=","==",">",">=","and","or"]`, con independencia del estado de las subtareas. El orden es por bytes UTF-8. Estos conjuntos describen continuaciones admisibles del prefijo completo de la expresión en un inicializador `let`; no admiten un segundo operador del mismo nivel. `recognized_prefix` empieza en el primer byte de la expresión comprometida y termina al inicio del segundo operador. `data={}`, sin relaciones ni cadena causal. El error sintáctico precede a nombres, tipos y al rechazo de rango almacenado. Los paréntesis cierran una expresión y permiten otra comparación, como `(1<2)==true`; un rango agrupado sigue siendo iterando directo de `for`, como `for i in (0..<2){}`. Una comparación única con rango en un operando se analiza por precedencia; su uso como valor o tipo recibe la regla de alcance existente.

**Ejemplos exactos:** H3-A–E y sus JSON fijan las listas admisibles del perfil F5 en cualquier estado de las subtareas. Los nombres `a`, `b`, `c` de las filas H3-B–D son bindings I64 declarados en la propia fuente. Todas las filas negativas llevan un único diagnóstico completo, sin error derivado de terminador.

| ID | Fuente | Segundo operador / primario | JSON |
|---|---|---|---|
| H3-A | `fn Main(){let x=1<2<3;}` | `<` `[19,20)` | H3-A |
| H3-B | `fn Main(){let a=1;let b=2;let c=3;let x=a<b<c;}` | `<` `[43,44)` | H3-B |
| H3-C | `fn Main(){let a=1;let b=2;let c=3;let x=a==b==c;}` | `==` `[44,46)` | H3-C |
| H3-D | `fn Main(){let a=1;let b=2;let c=3;let x=a<b==c;}` | `==` `[43,45)` | H3-D |
| H3-E | `fn Main(){let x=0..<1..<2;}` | `..<` `[21,24)` | H3-E |
| Positivo | `fn Main(){let x=(1<2)==true;}` | `diagnostics=[]`; x es Bool verdadero. | — |
| Positivo | `fn Main(){for i in (0..<2){}}` | `diagnostics=[]`; dos iteraciones. | — |

H3-A:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0002","subcause":null,"rule":"NX-E0002","explanation_id":"NX-E0002","severity":"error","stage":"parse","message":"Syntax is not admitted by the grammar.","operation":"syntax.parse","primary_span":{"file":"case.n","start":19,"end":20},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"<","span":{"file":"case.n","start":19,"end":20}},"expected_tokens":[";","and","or"],"expected_tokens_kind":"admissible_set","parser_context":"grammar.CompareExpr","recognized_prefix":{"file":"case.n","start":16,"end":19},"repair":[],"data":{}}
```

H3-B:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0002","subcause":null,"rule":"NX-E0002","explanation_id":"NX-E0002","severity":"error","stage":"parse","message":"Syntax is not admitted by the grammar.","operation":"syntax.parse","primary_span":{"file":"case.n","start":43,"end":44},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"<","span":{"file":"case.n","start":43,"end":44}},"expected_tokens":[";","and","or"],"expected_tokens_kind":"admissible_set","parser_context":"grammar.CompareExpr","recognized_prefix":{"file":"case.n","start":40,"end":43},"repair":[],"data":{}}
```

H3-C:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0002","subcause":null,"rule":"NX-E0002","explanation_id":"NX-E0002","severity":"error","stage":"parse","message":"Syntax is not admitted by the grammar.","operation":"syntax.parse","primary_span":{"file":"case.n","start":44,"end":46},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"==","span":{"file":"case.n","start":44,"end":46}},"expected_tokens":[";","and","or"],"expected_tokens_kind":"admissible_set","parser_context":"grammar.CompareExpr","recognized_prefix":{"file":"case.n","start":40,"end":44},"repair":[],"data":{}}
```

H3-D:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0002","subcause":null,"rule":"NX-E0002","explanation_id":"NX-E0002","severity":"error","stage":"parse","message":"Syntax is not admitted by the grammar.","operation":"syntax.parse","primary_span":{"file":"case.n","start":43,"end":45},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"==","span":{"file":"case.n","start":43,"end":45}},"expected_tokens":[";","and","or"],"expected_tokens_kind":"admissible_set","parser_context":"grammar.CompareExpr","recognized_prefix":{"file":"case.n","start":40,"end":43},"repair":[],"data":{}}
```

H3-E:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0002","subcause":null,"rule":"NX-E0002","explanation_id":"NX-E0002","severity":"error","stage":"parse","message":"Syntax is not admitted by the grammar.","operation":"syntax.parse","primary_span":{"file":"case.n","start":21,"end":24},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"..<","span":{"file":"case.n","start":21,"end":24}},"expected_tokens":["!=",";","<","<=","==",">",">=","and","or"],"expected_tokens_kind":"admissible_set","parser_context":"grammar.RangeExpr","recognized_prefix":{"file":"case.n","start":16,"end":21},"repair":[],"data":{}}
```

**Compatibilidad:**

| Fila vigente | Igual | Comprobación |
|---|---|---|
| A2 TD01–TD07; DG de sintaxis | Sí | Los encadenamientos no aparecen en esas fuentes; parser histórico de comparaciones únicas se conserva. |
| F2-S27/S28/S32/S33/S34, F2-LX01/LX02 | Sí | §12.1 conserva su conversión explícita a positivos F5; el segundo operador es la condición nueva, no el primero aceptado. |
| F2-F04 y F5-D01/D02 | Sí | Un solo rango almacenado/selector conserva NX-E0006/range; solo dos operadores al mismo nivel alcanzan H3-E. |
| F5-S01–S13 | Sí | Sus FIRST, truncamientos y prefijos no cambian; `BinaryAccepted` sigue conteniendo cada operador individual. |

**Impacto medido:** en el binario release de `git archive 5cf16fd`, con `--profile f0-0003-b-v1` (perfil J), el parser emite NX-E0002/`grammar.CompareExpr` sobre el segundo operador de H3-A–D con los offsets, tokens y prefijos de la tabla y `expected_tokens=[";"]`; `(1<2)==true` se acepta. Esa lista histórica es correcta porque `and`/`or` aún no se aceptan allí. El primer `..<` de H3-E recibe `NX-E0006/range` en parse `[17,20)`. Para el archivo ASCII exacto `fn Main(){for i in (0..<2){}}`, `check` y `run` con `--profile f0-0003-b-v1` dan `NX-E0006/range/later_adr`, `parse/syntax.parse`, `[21,24)`; sin perfil, el predeterminado `f0-0002-b-v2` da `NX-E0006/for_iteration/later_adr`, `parse/syntax.parse`, `[10,13)`, pues rechaza cualquier `for`, incluido este. F5-L05 cambia la ruta de rango por §2/§3; el binario histórico no ejecuta el perfil F5, por lo que los conjuntos ampliados son esperados transcritos de la gramática y su orden, no salidas medidas. Ninguna salida fijada por el perfil F5 cambia. **Alternativa descartada:** dejar que types rechace Bool frente a I64 o que el primer rango produzca `range/later_adr`; el diagnóstico dependería de tipos/alcance y no expresaría la gramática no asociativa.

## H4 — criterio de mutación y aceptación (REVERTIBLE)

**Ubicación:** §10, sustituir únicamente las frases de mutaciones de las líneas 1138–1141; conservar las demás obligaciones de §10.

**Texto para insertar:**

> Cada regla de núcleo tiene un negativo aislado y su positivo, con esperado transcrito de este ADR. La mutación inversa de esa regla se ejecuta aislada, debe hacer fallar el par pertinente y restaura el archivo comprobando SHA-256. Al final del corte, el implementador hace una única pasada adicional sobre archivos de núcleo: repite los supervivientes y añade como máximo ocho mutaciones nuevas. El validador no crea una población nueva por ronda. Un superviviente bloquea solo cuando se acompaña de un programa fuente reproducible, salida esperada por la regla explícita y salida obtenida que exhibe incumplimiento; entonces se corrige y deja regresión. Un superviviente sin ese repro se registra nominalmente en `Pending.md`, sin contarlo como PASS ni como defecto bloqueante. «Ninguna mutación de seguridad requerida sobrevive» se interpreta con ese criterio de defecto reproducible. Se conservan las puertas mecánicas y las revisiones independientes de §10.

| Ejemplo de decisión | Fuente / offset / diagnóstico | Resultado |
|---|---|---|
| Mutación inversa aislada de `<` I64 | Par de núcleo ya fijado por §9; offsets y JSON de esa fila | Debe distinguir negativo y positivo; restauración SHA-256. |
| Superviviente sin contraejemplo | No hay fuente ni diagnóstico atribuible | Registro nominal en `Pending.md`; no bloquea. |
| Superviviente con contraejemplo | Fuente ASCII, offset y JSON esperado/obtenido de la regla incumplida | Bloquea y exige regresión. |

**Compatibilidad:**

| Filas vigentes | Igual | Comprobación |
|---|---|---|
| TD/DG, F2 y F5 §9 completos | Sí | El texto altera el umbral de evidencia de validación, no sus programas ni esperados; comparar diff de fixtures. |

**Impacto:** ningún programa cambia; no requiere sonda de ejecución. La redacción coincide con AGENTS.md «Rigor proporcional y presupuesto de validación» y su única pasada final. **Alternativa descartada:** bloquear por todo mutante superviviente aunque no exista programa que muestre defecto; contradice el criterio reproducible y multiplica campañas sin evidencia semántica.

## H5 — efectos `alloc` de JoinArrays, Pop y Len (REVERTIBLE)

**Ubicación:** §4.2 después de las líneas 297–300; añadir fixtures después de F5-B05 en §9.2. La forma de NX-E0412 procede de A2 §§8.2, 13.6 y 13.15, F0-0003 §8 y F5 §9.2 fila B05.

**Texto para insertar:**

> `JoinArrays` conserva efecto intrínseco `alloc` incluso con dos arrays vacíos; una llamada directa dentro de `with no_alloc` da NX-E0412, subcause `null`, en `effects/intrinsic.call`, con primario sobre el nombre `JoinArrays`. `data` contiene exactamente `effect_model=f0-0005-sim-v1`, `effect_evidence=known_effect`, `required_effect=alloc` y `contract_origin` sobre el token escrito `no_alloc`. `related_spans` contiene `contract` sobre ese mismo token; `cause_chain` contiene una hoja `call/intrinsic.call` sobre el nombre, Entity `intrinsic/prelude:join_arrays`, evidencia `verified`. `Pop` y `Len` no aportan `alloc` intrínseco: se permiten bajo `no_alloc` con arrays ya preparados, cuando sus argumentos, copias y limpiezas tampoco lo aportan. Construcción/copia de Array y `Push` siguen teniendo su efecto `alloc` vigente. Se comprueba el resumen también después del lowering.

**Ejemplos exactos:**

| Clase | Fuente | Offset / resultado |
|---|---|---|
| Negativo JoinArrays | `fn F(a:view Array<I64>) with no_alloc{let b=JoinArrays(a,a);}fn Main(){}` | nombre `[44,54)`, contrato `[29,37)`, JSON H5-A. |
| Positivo Pop/Len | `fn F(a:edit Array<I64>) with no_alloc{let x=Pop(edit a);let n=Len(a);}fn Main(){var a=[1];F(edit a);}` | Pop `[44,47)`, Len `[62,65)`; `diagnostics=[]`, `run` unit y sin observaciones. El Array `[1]` se crea fuera de F. |
| Positivo JoinArrays fuera de no_alloc | `fn Main(){let a=[1];let b=JoinArrays(a,a);AssertEq(b,[1,1]);}` | JoinArrays `[26,36)`; `diagnostics=[]`, `run` unit y sin observaciones. |

H5-A:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0412","subcause":null,"rule":"NX-E0412","explanation_id":"NX-E0412","severity":"error","stage":"effects","message":"La función F infringe no_alloc.","operation":"intrinsic.call","primary_span":{"file":"case.n","start":44,"end":54},"related_spans":[{"role":"contract","span":{"file":"case.n","start":29,"end":37}}],"cause_chain":[{"kind":"call","operation":"intrinsic.call","span":{"file":"case.n","start":44,"end":54},"entity":{"kind":"intrinsic","id":"prelude:join_arrays"},"evidence":"verified"}],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"effect_model":"f0-0005-sim-v1","effect_evidence":"known_effect","required_effect":"alloc","contract_origin":{"file":"case.n","start":29,"end":37}}}
```

**Compatibilidad:**

| Fila vigente | Igual | Comprobación |
|---|---|---|
| A2 E01/E02 y F5-B05 | Sí | Construcción, copia y Push retienen NX-E0412 y sus anclas; comparar JSON en sus perfiles y F5 §9. |
| F5-A/B y F5-P04 | Sí | `Pop`/`Len` y JoinArrays conservan resultados, orden, cuotas y limpieza; solo se explicita la clasificación de efectos ya en §4.2. |
| A2 TD/DG y F2 | Sí | No se modifican nombres, tipos ni parser de esas fuentes; diferencial histórico. |

**Impacto medido:** en `git archive 5cf16fd`, `JoinArrays` aún no está en el preámbulo y la fuente H5-A da NX-E0101/names `[44,54)`; Pop tampoco está disponible. Son capacidades F5 nuevas sin esperado previo de ejecución en ese blob. Ninguna salida fijada cambia. **Alternativa descartada:** clasificar JoinArrays sin `alloc` para arrays vacíos; contradice §4.2 y la clasificación conservadora de A2.
