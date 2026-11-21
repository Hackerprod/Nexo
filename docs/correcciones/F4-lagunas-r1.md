# F0-0004 — propuesta normativa de laguna N3 (revertible)

## Alcance y autoridad

Ámbito: solo perfiles K de F0-0004. Esta propuesta no modifica el ADR ni el código. Los programas de las tablas son archivos ASCII sin BOM ni LF final; los intervalos son bytes, semiabiertos. `message` es presentación según F0-0004 §10.1, línea 403; los restantes campos de cada JSON son contractuales. `N3-V` es una etiqueta local, no un ID de fixture del ADR. Los perfiles A2/J/F5 conservan el token String opaco y su rechazo histórico `text` en parse (F4-r1 r3, líneas 98, 106 y 108).

## N3 — barrera de Hole antes de K08.3

**Laguna y citas.** F0-0004 §2.1, líneas 76–94, define `StringLiteral`, `Hole`, los escapes y los errores de la gramática final. §9.1, línea 389, asigna interpolación a K08; línea 393 sitúa todas las barreras K en `types/types.check`, da prioridad a una barrera sintáctica anterior y ordena elegir la primera construcción, con empate por K. §10.1, líneas 405–441 y 467, fija X01–X03, sus spans y tokens finales; §10.5, líneas 565–601, fija H01–H08. §12, líneas 1312–1313, 1319, 1325, 1337–1343, entrega String base en K02 y el parser recursivo de interpolación en K08.3, asignando X01–X03/H01–H08 a K08. El plan F4-r1 r3, «Decisiones de lagunas y límites abiertos», punto 4 (línea 188), identifica que el ADR todavía no decide la salida pública de esas fuentes entre K01 y K08.2 y condiciona B0/B7. La construcción de un AST de partes en K01 (plan, línea 98) no equivale a habilitar la semántica de Hole.

**Decisión propuesta del coordinador, reversible.** En perfiles K, desde K01 y hasta completar K08.2, el reconocedor puede conservar las partes del literal, pero no publica diagnósticos de estructura interior de interpolación. Una vez completa K02, todo literal String que contenga `{` o `}` y cuya **envoltura léxica y sintáctica exterior** se reconozca se rechaza por `NX-E0006`, `feature_id=implementation_limit`, `pending_scope=K08`, `stage=types`, `operation=types.check`, sobre el literal completo. La barrera se aplica igualmente a `"{1}"`, `"{}"`, `"}"` y `"{1"`: la validez final del interior no crea un oráculo sintáctico intermedio. Mientras K02 esté pendiente, la misma construcción recibe primero `implementation_limit/K02` por el orden K de §9.1; no se afirma K08 antes de cerrar K02. Un fallo léxico o sintáctico **exterior** anterior conserva parse y precede a cualquier barrera de types; por ejemplo, una comilla exterior sin cierre no llega a K08. Al completar K08.3 se retira la barrera K08 y se aplican la gramática y los diagnósticos finales de §2.1/§9/§10. X01–X03 y H01–H08 son pares **finales de K08.3**, sin esperado intermedio de sintaxis; no se añade ninguna prueba `adr_gap_*`. Las pruebas temporales de barrera se nombran por el comportamiento que comprueban.

**Compatibilidad y precedencia.** La lectura de «una barrera sintáctica anterior prevalece» (§9.1, línea 393) se limita a un error que el reconocedor público anterior realmente emite: el interior del Hole queda diferido mientras K08 está pendiente; el léxico y la estructura exterior siguen activos. Así se respeta el orden parse → types y la regla del primer K pendiente: K02 precede a K08 hasta su cierre. `"{1}"` es gramaticalmente correcto por §2.1, línea 88, pero no ejecutable antes de K08.3; después será positivo. `"{}"`, `"}"` y `"{"` (con cierre exterior) también llegan temporalmente a la barrera K08, y solo tras K08.3 reciben las subcausas de NX-E1401. El caso X03 literal es `"{1"`, no `"{"`; para `"{"` §2.1 fija la misma subcausa `unterminated_interpolation` pero §10.1 no da una fixture ni offsets para esa fuente: su JSON final, si se exige como par adicional, sería **derivado** de la regla, no transcrito de X03. La prioridad de un escape inválido ya validado en D15 §2.4, líneas 124–136, permanece anterior (NX-E0001). Esta propuesta no fija prioridades adicionales dentro del parser final.

**Alternativas y Flujo A.** Hay más de una respuesta razonable de núcleo: emitir NX-E1401 inmediatamente al encontrar el interior inválido, o conservar la barrera K08 hasta que el parser recursivo completo esté listo. Cambia el diagnóstico de un programa fuente; por la regla de lagunas de núcleo de AGENTS.md corresponde Flujo A antes de incorporarla al ADR o de usarla como criterio de PASS. Se recomienda la barrera uniforme descrita arriba: corresponde a la entrega K08.3 de §12 y evita presentar una parte de la gramática final como capacidad pública completa. La decisión sigue siendo revertible; el propietario puede revisar el ADR resultante. No se rebaja ni se reinterpreta el esperado final de K08.3.

## Pares literales de `check` y `run`

Estado de la tabla: K02 completa; K08.3 pendiente (incluye K08.1/K08.2 en curso o completas). Cada negativo da exactamente un diagnóstico; `check` y `run` producen el mismo objeto y exit distinto de 0, sin ejecutar Main. Para el positivo sin llaves, ambos tienen exit 0, `diagnostics=[]`, outcome unit, stdout/events vacíos. Para `N3-V`, «válido» significa **sintaxis final válida**: conserva la barrera K08 y exit distinto de 0 hasta K08.3; después `check`/`run` aceptan con las mismas observaciones vacías.

| Par / archivo | Fuente literal completa | Esperado temporal, primario | Después de K08.3 |
|---|---|---|---|
| Positivo sin Hole, `N3-P.n` | `fn Main(){let s="x";}` | `diagnostics=[]` | Igual. |
| Válido con Hole, `N3-V.n` | `fn Main(){let s="{1}";}` | K08, `[16,21)`, JSON N3-V (**derivado de la decisión**) | `diagnostics=[]`, outcome unit, stdout/events vacíos. |
| X01 inválido | `fn Main(){let s="{}";}` | K08, `[16,20)`, JSON N3-X01 (**derivado**) | NX-E1401/empty_interpolation, JSON X01 (**ADR**). |
| X02 inválido | `fn Main(){let s="}";}` | K08, `[16,19)`, JSON N3-X02 (**derivado**) | NX-E1401/unescaped_closing_brace, JSON X02 (**ADR**). |
| X03 inválido | `fn Main(){let s="{1";}` | K08, `[16,20)`, JSON N3-X03 (**derivado**) | NX-E1401/unterminated_interpolation, JSON X03 (**ADR**). |
| Llave abierta, `N3-C.n` | `fn Main(){let s="{";}` | K08, `[16,19)`, JSON N3-C (**derivado**) | NX-E1401/unterminated_interpolation, JSON N3-C-final (**derivado de §2.1/§9**). |

Los JSON de barrera siguientes concretan la decisión propuesta y §9.1, línea 393; **no son transcripción** de las fixtures finales. `subcause=null`, `data` contiene solo `pending_scope`, sin token, relaciones, cadena ni reparación. `message` se fija aquí solo para que la invocación sea verificable.

N3-V:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K08.","operation":"types.check","primary_span":{"file":"N3-V.n","start":16,"end":21},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K08"}}
```

N3-X01:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K08.","operation":"types.check","primary_span":{"file":"X01.n","start":16,"end":20},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K08"}}
```

N3-X02:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K08.","operation":"types.check","primary_span":{"file":"X02.n","start":16,"end":19},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K08"}}
```

N3-X03:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K08.","operation":"types.check","primary_span":{"file":"X03.n","start":16,"end":20},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K08"}}
```

N3-C (fuente `"{"`):

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en K08.","operation":"types.check","primary_span":{"file":"N3-C.n","start":16,"end":19},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"K08"}}
```

Los tres JSON finales siguientes **transcriben los campos contractuales** de F0-0004 §9, líneas 332–339, y §10.1, líneas 399–403, 405–409, 437–441 y 467. El texto elegido para `message` es presentación, no una cadena normativa del ADR.

X01 final:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"empty_interpolation","rule":"NX-E1401/empty_interpolation","explanation_id":"NX-E1401/empty_interpolation","severity":"error","stage":"parse","message":"Interpolación vacía.","operation":"syntax.parse","primary_span":{"file":"X01.n","start":17,"end":19},"related_spans":[{"role":"delimiter","span":{"file":"X01.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"}","span":{"file":"X01.n","start":18,"end":19}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

X02 final:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"unescaped_closing_brace","rule":"NX-E1401/unescaped_closing_brace","explanation_id":"NX-E1401/unescaped_closing_brace","severity":"error","stage":"parse","message":"Llave de cierre sin escape.","operation":"syntax.parse","primary_span":{"file":"X02.n","start":17,"end":18},"related_spans":[{"role":"delimiter","span":{"file":"X02.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"}","span":{"file":"X02.n","start":17,"end":18}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

X03 final:

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"unterminated_interpolation","rule":"NX-E1401/unterminated_interpolation","explanation_id":"NX-E1401/unterminated_interpolation","severity":"error","stage":"parse","message":"Interpolación sin cierre.","operation":"syntax.parse","primary_span":{"file":"X03.n","start":17,"end":18},"related_spans":[{"role":"delimiter","span":{"file":"X03.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"{","span":{"file":"X03.n","start":17,"end":18}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

N3-C-final (fuente `"{"`, **derivado**, no fila X03 del ADR):

```json
{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0004-b-v1","code":"NX-E1401","subcause":"unterminated_interpolation","rule":"NX-E1401/unterminated_interpolation","explanation_id":"NX-E1401/unterminated_interpolation","severity":"error","stage":"parse","message":"Interpolación sin cierre.","operation":"syntax.parse","primary_span":{"file":"N3-C.n","start":17,"end":18},"related_spans":[{"role":"delimiter","span":{"file":"N3-C.n","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"{","span":{"file":"N3-C.n","start":17,"end":18}},"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}
```

## Texto exacto para insertar en el ADR, sujeto a Flujo A

**Ubicación:** F0-0004 §9.1, inmediatamente después del párrafo de barreras K que termina «no se mantiene un frontend viejo tras una bandera» (línea 393). Un segundo párrafo va en §10.1, después de «Contrato común C» (línea 403), antes de la tabla X01–X27. Se incluyen aquí como redacción lista para incorporar, sin editar el ADR todavía.

**Texto para insertar en §9.1:**

> Bajo los perfiles K, desde K01 hasta completar K08.2, el reconocimiento de la envoltura exterior del literal String no publica todavía diagnósticos de sintaxis del interior de interpolación. Mientras K02 esté pendiente, todo literal String alcanza primero la barrera `NX-E0006/implementation_limit`, `pending_scope=K02`, en `types/types.check`. Tras completar K02 y mientras K08 esté pendiente, todo literal String que contenga `{` o `}` y cuya envoltura exterior se reconozca alcanza la barrera `NX-E0006/implementation_limit`, `pending_scope=K08`, en `types/types.check`, con primario en el literal completo; se aplica también a `"{1}"`, `"{}"`, `"}"` y `"{1"`. La barrera sintáctica anterior de este apartado sigue prevaleciendo para errores del léxico o de la envoltura exterior que se emitan antes de types. Los errores del interior del Hole se habilitan conjuntamente al completar K08.3 y retirar la barrera K08. No se crea un oráculo intermedio de sintaxis del Hole ni un pending_scope de subtarea.

**Texto para insertar en §10.1:**

> X01–X03 y H01–H08 son pares finales del cierre de K08.3. Antes de ese cierre, bajo perfiles K y después de K02, las envolturas exteriores reconocidas de X01 (`fn Main(){let s="{}";}`), X02 (`fn Main(){let s="}";}`) y X03 (`fn Main(){let s="{1";}`) reciben `NX-E0006/implementation_limit`, `pending_scope=K08`, `types/types.check`, con primarios respectivamente `[16,20)`, `[16,19)` y `[16,20)`; `data` contiene únicamente `pending_scope`, los campos de token, relaciones, cadena y reparación están vacíos/null. El positivo sintáctico `fn Main(){let s="{1}";}` también conserva esa barrera, con primario `[16,21)`. Al completar K08.3 se aplican íntegramente los JSON X01–X03 y H01–H08 de esta sección y sus positivos; no se añaden pruebas `adr_gap_*`.

## Otras fronteras del mismo tramo, registradas sin decidirlas

1. **X01–X03 fuera de las tres fuentes literales de arriba.** §2.1, líneas 88–92, comprende espacios en Hole, EOF, comilla prematura, `:` y expresión internamente inválida; §10.1, líneas 405–409 y 467, solo materializa X01–X03. La regla general propuesta cubre envolturas reconocidas, pero no fija nuevos pares ni precedencia de recuperación antes de K08.3.
2. **H01–H06 y la recuperación de salto/comentario.** §2.1, líneas 88–92, define CR/LF, `//`, pila de contextos, descarte y reanudación; §10.5, líneas 565–601, da H01–H06 y sus tokens/offsets finales. Queda por validar en Flujo A si el reconocedor exterior previo puede diferir esas fronteras sin emitir otro error parse anterior, especialmente cuando el salto corta la envoltura física; esta propuesta no les asigna nuevos JSON temporales.
3. **H07–H08, offsets y capacidad nominal.** §2.1, línea 94, exige mapa de bytes fuente; §10.5, líneas 577–601, da `missing_format` final y offsets tras UTF-8/escapes; §12, líneas 1337–1339, separa planes Format (K08.1) del parser Hole (K08.3). No se fija aquí si un diagnóstico de capacidad de K08.1 podría adelantarse a la barrera K08 para estas fuentes.
4. **Escapes de D15.** D15 §2.4, líneas 124–136, valida cinco escapes y hace prevalecer NX-E0001 para `\q` antes del rechazo `text`; F0-0004 §2.1, líneas 83 y 88, hereda esos escapes. No se define aquí un nuevo par mixto que combine escape inválido con Hole, ni otra precedencia interna.
5. **Llaves duplicadas `{{`/`}}`.** F0-0004 §2.1, líneas 80–88 y 94, las declara texto literal en la gramática final; el plan F4-r1 r3, líneas 98 y 106, las habilita como texto en K02. Su tratamiento temporal con la regla amplia «contenga `{` o `}`» requiere decisión explícita para que `"{{1}}"` no reciba simultáneamente aceptación K02 y barrera K08. Esta propuesta prioriza la barrera uniforme solo como recomendación N3; la compatibilidad con esa decisión del plan debe resolverse al revisar A/B, sin cambiar ahora el plan.
