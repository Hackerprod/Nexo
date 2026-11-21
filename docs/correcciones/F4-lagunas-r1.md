# F0-0004 — propuesta normativa de laguna N3, r2 (revertible)

## Autoridad y decisión

Solo perfiles K. Esta propuesta no edita el ADR ni el plan. F0-0004 §2.1, líneas 81–94, fija la gramática final; §9.1, línea 393, da prioridad a la barrera sintáctica anterior; §10.1, líneas 401–403, 405–409, 437–441 y 467, y §10.5, líneas 567–601, fijan los oráculos. §12, líneas 1312–1313, 1319, 1325, 1339 y 1343, asigna las parejas completas a K08.3. El plan `F4-r1.md` r3 instala en K01 el parser final del literal con partes Hole (§«Decisión L1», línea 98), pero aplaza el oráculo X/H (líneas 104–108 y 188). N3 resuelve ese intervalo.

**Decisión corregida del coordinador, reversible:** K01 analiza el interior con el parser final bajo K. Cada literal **mal formado** emite desde K01 su diagnóstico final de sintaxis, con prioridad parse sobre types: `"{}"`, `"}"`, `"{1"`, `"{"`, X01–X03 y H01–H06. En `"{"`, la comilla [18,19) aparece cuando se espera operando y no cierra una cadena anidada antes de EOF; §2.1, línea 92, exige `NX-E1401/incomplete_interpolation_expression`, no `unterminated_interpolation`. Sin analizar el interior no hay una envoltura exterior reconocible para esa fuente. X03, en cambio, tiene Expr completa y señala `{` [17,18). La interpretación anterior N3-C/N3-C-final queda retirada.

Un literal **bien formado con al menos un Hole** conserva `NX-E0006/implementation_limit` en `types/types.check`, primario literal completo, `pending_scope=K08`, desde que K02 se completa hasta K08.3; antes alcanza K02, primer K pendiente. Un literal sin Hole —incluidos `"{{"`, `"}}"` y `"a{{b}}c"`— alcanza K02 y, al cerrarlo, sigue la ruta normal. `{{` y `}}` son Text, no Hole (§2.1, línea 81; §12, línea 1313). K08.3 retira la barrera, habilita la semántica del Hole y repite X01–X03/H01–H08 con sus positivos; no cambia el JSON sintáctico ya emitido desde K01. K08.1/K08.2 no publican una interpolación parcial (§12, líneas 1325, 1337–1339).

La selección de gramática por manifiesto es solo K. A2/J/F5 conservan el token String opaco y `text` en parse, según D15 §2.4, líneas 124–136, y el plan F4-r1 r3, líneas 98, 104 y 106. El primer diagnóstico se decide por el primer error o construcción pendiente en orden fuente, con empate por K (§9.1, línea 393); una barrera de K04 anterior puede preceder al literal de H07/H08. Un escape inválido conserva NX-E0001 (D15 §2.4; K §2.1, línea 88). No se inventan prioridades internas nuevas.

Contando bytes de las cuatro fuentes ASCII, los rangos de literal son `"{1}"` [16,21), `"{}"` [16,20), `"}"` [16,19) y `"{1"` [16,20). Solo el primero es un Hole bien formado que alcanza la barrera K08; los tres últimos emiten parse. En `"{"` los bytes [16,19) no constituyen una envoltura reconocida sin analizar el interior. Estos rangos no sustituyen los primarios X01–X03 (§10.1, líneas 437–441).

## Tabla de programas y resultados

Cada fuente es un archivo UTF-8 sin BOM ni LF final; los casos ASCII son todos salvo N3-U y H01–H08. En H01–H08 la fuente se escribe como **cadena JSON**, decodificada una vez antes de grabarla, según §10.5, línea 567; `\r` en H01 representa CR real, mientras `\\n` en H07 representa barra y n del fuente Nexo. Los intervalos son bytes UTF-8 semiabiertos. Cada celda `D-ID` o `B-ID-Knn` remite al **objeto JSON completo** de abajo; `[]` es el JSON completo de `diagnostics` aceptado. El objeto de diagnóstico se inserta como único elemento de `diagnostics`. En ambos comandos `check` y `run` un diagnóstico da `exit 1`, outcome rejected, sin salida de programa ni eventos y sin ejecutar Main; aceptación da `exit 0`, `diagnostics=[]`, y `run` outcome unit con stdout/events vacíos. El transporte de `check` satisfactorio en formato text imprime `nexo-check:ok` más LF (D15 §7.1, líneas 760–762; K §10.1, líneas 401–403; K §10.7, línea 637). `message` es presentación (§10.1, línea 403), no identidad normativa. `kind:"symbol"` para X01–X03 se **deriva** de §9, línea 339; no figura literalmente en sus filas §10.1.

| Archivo | Fuente completa (JSON de cadena) | K01: check/run | K02 completo, K08 pendiente: check/run | K08.3: check/run | Cita del esperado |
|---|---|---|---|---|---|
| `N3-V.n` | `"fn Main(){let s=\"{1}\";}"` | `B-N3-V-K02`; exit 1 | `B-N3-V-K08`; exit 1 | `[]`; exit 0 | K §2.1, 81–88; §9.1, 393 |
| `X01.n` | `"fn Main(){let s=\"{}\";}"` | `D-X01`; exit 1 | `D-X01`; exit 1 | `D-X01`; exit 1 | K §10.1, 407, 439, 467 |
| `X02.n` | `"fn Main(){let s=\"}\";}"` | `D-X02`; exit 1 | `D-X02`; exit 1 | `D-X02`; exit 1 | K §10.1, 408, 440, 467 |
| `N3-C.n` | `"fn Main(){let s=\"{\";}"` | `D-N3-C`; exit 1 | `D-N3-C`; exit 1 | `D-N3-C`; exit 1 | K §2.1, 92; §9, 334, 339; §10.5, 576, 592 (derivado) |
| `X03.n` | `"fn Main(){let s=\"{1\";}"` | `D-X03`; exit 1 | `D-X03`; exit 1 | `D-X03`; exit 1 | K §10.1, 409, 441, 467 |
| `N3-T1.n` | `"fn Main(){let s=\"{{\";}"` | `B-N3-T1-K02`; exit 1 | `[]`; exit 0 | `[]`; exit 0 | K §2.1, 81; §12, 1313 |
| `N3-T2.n` | `"fn Main(){let s=\"}}\";}"` | `B-N3-T2-K02`; exit 1 | `[]`; exit 0 | `[]`; exit 0 | K §2.1, 81; §12, 1313 |
| `N3-T3.n` | `"fn Main(){let s=\"a{{b}}c\";}"` | `B-N3-T3-K02`; exit 1 | `[]`; exit 0 | `[]`; exit 0 | K §2.1, 81; §12, 1313 |
| `N3-U.n` | `"fn Main(){let s=\"é{}\";}"` | `D-N3-U`; exit 1 | `D-N3-U`; exit 1 | `D-N3-U`; exit 1 | K §2.1, 81, 88, 94; §9, 334, 339 (derivado) |
| `N3-Q.n` | `"fn Main(){let s=\"{\"x\"}\";}"` | `B-N3-Q-K02`; exit 1 | `B-N3-Q-K08`; exit 1 | `[]`; exit 0 | K §2.1, 86, 88, 94 (derivado) |
| `N3-Q2.n` | `"fn Main(){let s=\"{1 + \"2\"}\";}"` | `B-N3-Q2-K02`; exit 1 | `B-N3-Q2-K08`; exit 1 | `[]`; exit 0 | K §2.1, 92, 94 (derivado) |
| `H01.n` | `"fn Main(){let s=\"é{1\r\n}\";}"` | `D-H01`; exit 1 | `D-H01`; exit 1 | `D-H01`; exit 1 | K §10.5, 567, 571, 587, 601 |
| `H02.n` | `"fn Main(){let s=\"é{1\r}\";}"` | `D-H02`; exit 1 | `D-H02`; exit 1 | `D-H02`; exit 1 | K §10.5, 567, 572, 588, 601 |
| `H03.n` | `"fn Main(){let s=\"é{1\n}\";}"` | `D-H03`; exit 1 | `D-H03`; exit 1 | `D-H03`; exit 1 | K §10.5, 567, 573, 589, 601 |
| `H04.n` | `"fn Main(){let s=\"é{1//}\";}"` | `D-H04`; exit 1 | `D-H04`; exit 1 | `D-H04`; exit 1 | K §10.5, 567, 574, 590, 601 |
| `H05.n` | `"fn Main(){let s=\"é{ \t }\";}"` | `D-H05`; exit 1 | `D-H05`; exit 1 | `D-H05`; exit 1 | K §10.5, 567, 575, 591, 601 |
| `H06.n` | `"fn Main(){let s=\"é{1 + \";}"` | `D-H06`; exit 1 | `D-H06`; exit 1 | `D-H06`; exit 1 | K §10.5, 567, 576, 592, 601 |
| `H07.n` | `"struct S{}\r\nfn Main(){let s=S();let t=\"é\\n{s}\";}"` | K04 anterior; sin oráculo K01 | K04/Format anteriores; sin oráculo intermedio | `D-H07`; exit 1 | K §10.5, 567, 577, 593, 601 |
| `H08.n` | `"struct S{}fn Main(){let s=S();let t=\"é\\\"\\\\{s}\";}"` | K04 anterior; sin oráculo K01 | K04/Format anteriores; sin oráculo intermedio | `D-H08`; exit 1 | K §10.5, 567, 578, 594, 601 |

Los negativos X01–X03/H01–H06 se ejecutan en K01 **por primera vez** como pares de sintaxis. Su edición positiva elimina ese error de parse; en K01 el literal corregido alcanza la barrera K02, y luego K08 si contiene Hole. Se repiten en K08.3, donde sus positivos ya se aceptan con semántica, efectos y cuotas. H07/H08 tienen sintaxis válida, pero requieren nominal K04, Format K08.1 e interpolación K08.3: sus JSON finales de `missing_format` se ejecutan como pares completos en K08.3. La celda «K04 anterior» no pretende fijar un primario de capacidad que el ADR no da para esas fuentes. N3-U y N3-C son derivados explícitos de la regla de §2.1/§9, no nuevas fixtures transcritas.

## JSON completos de diagnóstico

Los objetos siguientes expanden el contrato común C (§10.1, líneas 399–403) y los campos específicos de §9, líneas 332–339, §10.1, líneas 437–441 y 467, y §10.5, líneas 585–601. Los B se derivan de §9.1, línea 393, y D15 §7.1, línea 762; no son fixtures finales del ADR. Los D X/H reproducen las filas del ADR, con `message` de presentación.

### B-N3-V-K02

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K02.","operation":"types.check","primary_span":{"file":"N3-V.n","start":16,"end":21},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K02"}}
```

### B-N3-V-K08

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K08.","operation":"types.check","primary_span":{"file":"N3-V.n","start":16,"end":21},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K08"}}
```

### D-X01

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"empty_interpolation","rule":"NX-E1401/empty_interpolation","explanation_id":"NX-E1401/empty_interpolation","severity":"error","stage":"parse","message":"Interpolación vacía.","operation":"syntax.parse","primary_span":{"file":"X01.n","start":17,"end":19},"related_spans":[{"role":"delimiter","span":{"file":"X01.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"}","span":{"file":"X01.n","start":18,"end":19}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### D-X02

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"unescaped_closing_brace","rule":"NX-E1401/unescaped_closing_brace","explanation_id":"NX-E1401/unescaped_closing_brace","severity":"error","stage":"parse","message":"Llave de cierre sin escape.","operation":"syntax.parse","primary_span":{"file":"X02.n","start":17,"end":18},"related_spans":[{"role":"delimiter","span":{"file":"X02.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"}","span":{"file":"X02.n","start":17,"end":18}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### D-N3-C

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"incomplete_interpolation_expression","rule":"NX-E1401/incomplete_interpolation_expression","explanation_id":"NX-E1401/incomplete_interpolation_expression","severity":"error","stage":"parse","message":"Expresión de interpolación incompleta.","operation":"syntax.parse","primary_span":{"file":"N3-C.n","start":18,"end":19},"related_spans":[{"role":"delimiter","span":{"file":"N3-C.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"\"","span":{"file":"N3-C.n","start":18,"end":19}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### D-X03

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"unterminated_interpolation","rule":"NX-E1401/unterminated_interpolation","explanation_id":"NX-E1401/unterminated_interpolation","severity":"error","stage":"parse","message":"Interpolación sin cierre.","operation":"syntax.parse","primary_span":{"file":"X03.n","start":17,"end":18},"related_spans":[{"role":"delimiter","span":{"file":"X03.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"{","span":{"file":"X03.n","start":17,"end":18}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### B-N3-T1-K02

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K02.","operation":"types.check","primary_span":{"file":"N3-T1.n","start":16,"end":20},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K02"}}
```

### B-N3-T2-K02

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K02.","operation":"types.check","primary_span":{"file":"N3-T2.n","start":16,"end":20},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K02"}}
```

### B-N3-T3-K02

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K02.","operation":"types.check","primary_span":{"file":"N3-T3.n","start":16,"end":25},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K02"}}
```

### D-N3-U

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"empty_interpolation","rule":"NX-E1401/empty_interpolation","explanation_id":"NX-E1401/empty_interpolation","severity":"error","stage":"parse","message":"Interpolación vacía.","operation":"syntax.parse","primary_span":{"file":"N3-U.n","start":19,"end":21},"related_spans":[{"role":"delimiter","span":{"file":"N3-U.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"}","span":{"file":"N3-U.n","start":20,"end":21}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### B-N3-Q-K02

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K02.","operation":"types.check","primary_span":{"file":"N3-Q.n","start":16,"end":23},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K02"}}
```

### B-N3-Q-K08

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K08.","operation":"types.check","primary_span":{"file":"N3-Q.n","start":16,"end":23},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K08"}}
```

### B-N3-Q2-K02

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K02.","operation":"types.check","primary_span":{"file":"N3-Q2.n","start":16,"end":27},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K02"}}
```

### B-N3-Q2-K08

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K08.","operation":"types.check","primary_span":{"file":"N3-Q2.n","start":16,"end":27},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K08"}}
```

### D-H01

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"raw_line_break_in_interpolation","rule":"NX-E1401/raw_line_break_in_interpolation","explanation_id":"NX-E1401/raw_line_break_in_interpolation","severity":"error","stage":"parse","message":"Salto crudo en interpolación.","operation":"syntax.parse","primary_span":{"file":"H01.n","start":21,"end":23},"related_spans":[{"role":"delimiter","span":{"file":"H01.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"invalid","text":"\r\n","span":{"file":"H01.n","start":21,"end":23}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### D-H02

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"raw_line_break_in_interpolation","rule":"NX-E1401/raw_line_break_in_interpolation","explanation_id":"NX-E1401/raw_line_break_in_interpolation","severity":"error","stage":"parse","message":"Salto crudo en interpolación.","operation":"syntax.parse","primary_span":{"file":"H02.n","start":21,"end":22},"related_spans":[{"role":"delimiter","span":{"file":"H02.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"invalid","text":"\r","span":{"file":"H02.n","start":21,"end":22}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### D-H03

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"raw_line_break_in_interpolation","rule":"NX-E1401/raw_line_break_in_interpolation","explanation_id":"NX-E1401/raw_line_break_in_interpolation","severity":"error","stage":"parse","message":"Salto crudo en interpolación.","operation":"syntax.parse","primary_span":{"file":"H03.n","start":21,"end":22},"related_spans":[{"role":"delimiter","span":{"file":"H03.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"invalid","text":"\n","span":{"file":"H03.n","start":21,"end":22}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### D-H04

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"comment_in_interpolation","rule":"NX-E1401/comment_in_interpolation","explanation_id":"NX-E1401/comment_in_interpolation","severity":"error","stage":"parse","message":"Comentario en interpolación.","operation":"syntax.parse","primary_span":{"file":"H04.n","start":21,"end":23},"related_spans":[{"role":"delimiter","span":{"file":"H04.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"//","span":{"file":"H04.n","start":21,"end":23}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### D-H05

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"empty_interpolation","rule":"NX-E1401/empty_interpolation","explanation_id":"NX-E1401/empty_interpolation","severity":"error","stage":"parse","message":"Interpolación vacía.","operation":"syntax.parse","primary_span":{"file":"H05.n","start":19,"end":24},"related_spans":[{"role":"delimiter","span":{"file":"H05.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"}","span":{"file":"H05.n","start":23,"end":24}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### D-H06

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"incomplete_interpolation_expression","rule":"NX-E1401/incomplete_interpolation_expression","explanation_id":"NX-E1401/incomplete_interpolation_expression","severity":"error","stage":"parse","message":"Expresión de interpolación incompleta.","operation":"syntax.parse","primary_span":{"file":"H06.n","start":24,"end":25},"related_spans":[{"role":"delimiter","span":{"file":"H06.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"\"","span":{"file":"H06.n","start":24,"end":25}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

### D-H07

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1403","subcause":"missing_format","rule":"NX-E1403/missing_format","explanation_id":"NX-E1403/missing_format","severity":"error","stage":"types","message":"Falta capacidad Format.","operation":"types.check","primary_span":{"file":"H07.n","start":44,"end":45},"related_spans":[{"role":"declaration","span":{"file":"H07.n","start":7,"end":8}}],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"required_capability":"Format"}}
```

### D-H08

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1403","subcause":"missing_format","rule":"NX-E1403/missing_format","explanation_id":"NX-E1403/missing_format","severity":"error","stage":"types","message":"Falta capacidad Format.","operation":"types.check","primary_span":{"file":"H08.n","start":44,"end":45},"related_spans":[{"role":"declaration","span":{"file":"H08.n","start":7,"end":8}}],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"required_capability":"Format"}}
```


## Enmienda exacta al plan F4-r1.md r3 (propuesta; no editar aún)

Las sustituciones siguientes se aplican **después de PASS de B** y mantienen la asignación de parejas completas de K §12, línea 1343:

1. En la fila **K01**, línea 79, añadir al final de la celda de alcance: «El parser final del literal String analiza desde K01 la estructura interior del Hole bajo K. X01–X03 y H01–H06 ejecutan desde G0 sus negativos sintácticos finales y positivos aislados, con JSON de K §§2.1, 9 y 10; un Hole bien formado sigue la barrera K02 y después K08. Las llaves duplicadas son Text sin Hole.» En su celda de pruebas, añadir: «G0 ejecuta X01–X03/H01–H06, `"{"`, `"é{}"`, `"{{"`, `"}}"`, `"a{{b}}c"` y comillas anidadas según F4-lagunas-r1 r2; diferencial A2/J/F5 y debug/release.»
2. En **K08.3**, línea 92, sustituir «Usar el parser final de contextos/Hole instalado en K01» por «Usar el parser final de contextos/Hole ya activo desde K01 para la sintaxis, y habilitar aquí su semántica». Añadir a la celda de pruebas: «Repetir X01–X03/H01–H06 y sus positivos, idéntico JSON sintáctico a G0; ejecutar H07/H08 completos y los positivos semánticos. Retirar K08 solo para los Holes bien formados.»
3. En **«Decisión L1»**, línea 98, sustituir desde «K02 retira la barrera del literal sin Hole» hasta «La laguna temporal X01–X03/H01–H08 se registra abajo y condiciona B0/B7» por: «K01 emite los errores sintácticos finales de Hole mal formado bajo K, incluidos X01–X03/H01–H06; K02 retira la barrera del literal sin Hole, incluidas `{{`/`}}` como Text. `"{1}"` bien formado conserva la barrera K08 hasta K08.3. Bajo A2/J/F5 los interiores siguen opacos y prevalece el rechazo `text`. H07/H08 son pares semánticos finales de K08.3. N3 queda resuelta por F4-lagunas-r1 r2 y condiciona G0/B0 y G7/B7.» Mantener el resto del párrafo.
4. En **«Pruebas diagnósticas de barrera K01»**, línea 100, añadir: «Los X01–X03/H01–H06 inválidos son parse finales desde K01 y no pasan por K02/K08; los positivos que contienen Hole válido siguen las barreras K02→K08.»
5. En **«Regresión de gramática por perfil»**, línea 104, sustituir la última oración que comienza «Los pares K usan N13» por: «Los pares K usan N13/positivo `0` antes de K02 y X01–X03/H01–H06 desde K01 con diagnóstico parse final; después de K02, `"{{"`/`"}}"` se aceptan y `"{1}"` queda en K08; K08.3 repite los pares sintácticos y habilita H07/H08 semánticos.»
6. Sustituir completas las líneas 106 y 108 por: «Los pares de barrera K01 anteriores se ejecutan solo bajo K; A2/J/F5 conservan parse y precedencia histórica. Las formas inválidas X01–X03/H01–H06 emiten su JSON final en K01; las formas Hole válidas pasan por K02 y luego K08; las llaves duplicadas son texto K02.» y «K08.3 por perfil: se retira la barrera de Hole bien formado y se habilitan BuildString y capacidades únicamente bajo K. X01–X03/H01–H06 repiten el JSON parse ya exigido en K01; H07/H08 se validan completos aquí. A2/J/F5 mantienen `text` opaco, incluido `"…{x}"`; la inversión de gramática muere por el diferencial histórico.»
7. En la fila **K08.3 (H02/H03)**, línea 126, añadir: «Estos negativos y sus positivos ya se exigen en K01; aquí se repiten con la semántica del bloque.» En la fila de obligaciones K08.3, línea 125, añadir: «H07/H08 son pares finales de capacidad Format, no negativos sintácticos de G0.» En G0/G7, línea 162, añadir: «G0 incluye X01–X03/H01–H06 sintácticos; G7 los repite y añade H07/H08 semánticos.»
8. Sustituir entero el punto **4 de «Decisiones de lagunas y límites abiertos»**, línea 188, por: «4. **N3 resuelta, decisión reversible:** el parser final del literal activo desde K01 emite los diagnósticos sintácticos definitivos de X01–X03/H01–H06 y de las variantes derivables de K §§2.1/9. K02 habilita String sin Holes, incluido texto `{{`/`}}`; un Hole bien formado recibe primero K02 y luego K08 hasta K08.3. H07/H08 son semánticos finales de K08.3. G0 ejecuta los pares de sintaxis; G7 los repite y ejecuta los pares de semántica. Oráculos, citas y texto normativo propuesto: F4-lagunas-r1 r2. No hay `adr_gap_*` ni expected generado por el binario.»
9. En **«Cambios respecto a la versión anterior»**, línea 5, sustituir «se declara la laguna X01–X03/H01–H08 entre K01 y K08 sin decidirla» por «se resuelve N3: la sintaxis X01–X03/H01–H06 se exige desde K01, y H07/H08 completos al cerrar K08.3». En **«Abierto para validación B»**, línea 190, sustituir «confirmar L1/L3 y resolver N3 por la autoridad aplicable antes de C en el bloque condicionado» por «confirmar L1/L3 y validar la decisión N3 de F4-lagunas-r1 r2 antes de C en B0».

## Texto exacto para insertar en el ADR (sin editarlo)

**F0-0004 §9.1, inmediatamente después del párrafo de barreras K que termina «no se mantiene un frontend viejo tras una bandera» (línea 393):**

> Bajo perfiles K, K01 instala el parser final del literal String con fragmentos Text y partes Hole; desde K01 emite los diagnósticos sintácticos definitivos de §2.1 y §9 para literales mal formados, antes de cualquier barrera de types. `{{` y `}}` son Text, no Hole. Mientras K02 esté pendiente, los literales bien formados encuentran la barrera `NX-E0006/implementation_limit`, `pending_scope=K02`; al cerrar K02, los literales sin Hole siguen la ruta String ordinaria. Un literal bien formado con al menos un Hole encuentra `NX-E0006/implementation_limit`, `pending_scope=K08`, `types/types.check`, con primario en el literal completo, hasta cerrar K08.3; entonces se habilita su semántica y se retira la barrera. La selección del primer error o capacidad pendiente respeta el orden fuente y la prioridad sintáctica de este apartado. No hay pending_scope de subtarea ni capacidad parcial de K08.1/K08.2.

**F0-0004 §10.1, después de «Contrato común C» (línea 403):**

> X01–X03 y H01–H06 son negativos de sintaxis final que se ejecutan por primera vez en K01; sus ediciones positivas eliminan el error de parse y alcanzan la barrera de capacidad que corresponda. Se repiten en K08.3 con sus positivos plenamente aceptados y con los campos de §2.1, §9 y esta sección; se repiten en K08.3. En particular, `fn Main(){let s="{";}` emite NX-E1401/incomplete_interpolation_expression sobre la comilla [18,19), con unexpected_token de comilla y delimiter [16,17), por §2.1; `fn Main(){let s="{1";}` conserva NX-E1401/unterminated_interpolation sobre `{` [17,18). Los literales `fn Main(){let s="{{";}`, `fn Main(){let s="}}";}` y `fn Main(){let s="a{{b}}c";}` no tienen Hole y quedan disponibles al completar K02. `fn Main(){let s="{1}";}` sí tiene Hole: después de K02 y hasta K08.3 recibe NX-E0006/implementation_limit, pending_scope=K08, types/types.check, primario [16,21). H07/H08 son pares de capacidad Format completos de K08.3 y conservan sus JSON finales. No se añaden pruebas `adr_gap_*`.

**F0-0004 §12, después de la asignación adicional que termina «las parejas completas se ejecutan al cerrar la K que aporta la capacidad» (línea 1343):**

> La asignación de X01–X03/H01–H08 a K08.3 conserva los pares completos y la semántica de interpolación. Los negativos de sintaxis X01–X03/H01–H06 se comprueban ya en K01 con los mismos diagnósticos finales; sus ediciones positivas comprueban la ausencia del error de parse y pueden alcanzar K02/K08. Los pares completos se repiten en K08.3. H07/H08 requieren Format e interpolación y se ejecutan como pares completos en K08.3.

## Fronteras restantes

- H01–H04: la recuperación de CR/LF y `//` sigue exactamente §2.1, línea 90, y §10.5, líneas 567–601. La tabla fija solo los JSON que esas filas dan; no fija un nuevo resultado para una combinación de salto, comentario y cadena anidada sin fixture.
- H05/H06 se rigen por §2.1, líneas 90–92, y §10.5, líneas 575–592. `"{"` se deriva de esa regla, no de X03. Una expresión internamente inválida conserva NX-E0002 ordinario (§2.1, líneas 88 y 92); no se asigna aquí JSON a una fuente distinta que el ADR no materialice.
- H07/H08 incluyen multibyte y escapes antes del Hole; el primario [44,45) cuenta bytes originales (§2.1, línea 94; §10.5, líneas 577–601). Las comillas anidadas válidas extienden el literal exterior hasta su cierre real: `"{"x"}"` ocupa [16,23) y `"{1 + "2"}"` [16,27), derivados de §2.1, líneas 86–94.
- Un Hole inválido después de otra construcción pendiente no desplaza el primer diagnóstico por orden fuente (§9.1, línea 393). No se fija un primario temporal no enunciado para H07/H08 cuando K04 está pendiente. Los cinco escapes de D15 conservan la prioridad de NX-E0001 (§2.1, línea 88; §9, línea 339).
- Las variantes adicionales de CR/LF/`//` combinadas con comillas anidadas que dependan de la recuperación no reciben un JSON inventado: §2.1, línea 90, define el algoritmo, mientras §10.5, líneas 571–594, solo materializa H01–H08. No queda aquí una alternativa razonable que cambie un resultado **final** del núcleo; la decisión N3 solo precisa cuándo se emite un resultado ya fijado por el ADR.
