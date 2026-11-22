# I07c-S6-COR r2: propuesta de corrección, revisión 3 (paso 4.1; solo propuesta, sin tocar código ni tests)

Base de inspección: `cd8609e`/`9e91715` (candidato, incluye r1) y `1010938` (base). Todas las afirmaciones se comprobaron con el binario debug compilado de cada una (`check` y, donde se indica, `run`; perfiles `f0-0003-b-v1`, `f0-0005-b-v1`). Origen: FAIL de D' (informe `C:\nexo-work\tmp\validador-dprima-s6cor\informe.md`, motivo 1 y N1, N2), decisiones del orquestador sobre el auditor (A) y el corpus de regresión (B), FAIL de D en la ronda 2 (`informe-r3.md`: la familia incluye `if` y `move`; la convención de anclaje debe declararse), **ratificación por el orquestador de la regla (a)** («cabeza = primer token de la expresión interior desagrupada», aquí ya no pendiente) y FAIL de B al texto del addendum (faltaba NX-E0414; «el resto de pasos no cambia» era falso con llamadas anidadas; NX-E1305 no es de §8.2). Esta revisión cubre los **cuatro** contratos de F0-0002 §8.2 (`no_io`, `no_alloc`, `no_block`, `no_suspend`).

## 1. Defecto y causa raíz comprobada

**Síntoma.** El destino agrupado de una llamada que **no es un nombre** da primario y primer paso causal vacíos en los diagnósticos de efectos. Repro de D': `fn Main() with no_io{(fn (){Print(1);})();}`:

| | operación | primario | `cause_chain` |
|---|---|---|---|
| base y candidato | `closure.call` | `[21,21)` vacío | `[closure.call [21,21), intrinsic.call [28,33)]` |
| control sin grupo `fn (){Print(1);}()` | `closure.call` | `[21,23)` (`fn`) | `[closure.call [21,23), intrinsic.call [27,32)]` |
| **esperado** | `closure.call` | **`[22,24)`** | `[closure.call [22,24), intrinsic.call [28,33)]` |

Idéntico para `((fn …))()` (`[21,21)`), NX-E0412 `with no_alloc{(fn (){let a=[1];})();}` (`[24,24)`), NX-E0413 `with no_block{(fn (){let r=MockBlockingOpen(1);})();}` (`[24,24)`), **NX-E0414** `with no_suspend{(fn (){MockUnknown();})();}` (`[26,26)`, chain `[closure.call [26,26), intrinsic.call [33,44)]`; control sin grupo `[26,28)`) y la guarda de `match` bajo `no_io` `[40,40)`. Es preexistente (la base lo hace igual); el addendum lo prohíbe («nunca un primario vacío por empezar la llamada en `(`»), y S6-COR declara objeto el primario y el paso causal de llamada de efectos, luego bloquea.

**Causa.** `src/ir/effects.rs`:
1. `scope_sites` solo añade una entrada `(span de la llamada, span del nombre)` cuando `expression_primary(target)` es `Expr::Name` (`if matches!(..., Expr::Name(..))`, ~líneas 1141-1150). Otro destino no entra en `CallSites`.
2. `CallSites::head(span, source)` hace `self.0.get(span).cloned().unwrap_or_else(|| call_head(span, source))`: sin entrada cae en `call_head`, que lee el **texto de la fuente desde `span.start`** (racha `[A-Za-z0-9_]`). Para la llamada `(fn (){…})()` el span de la llamada empieza en `(` (byte 21) y la racha es vacía.
3. `head` es el selector de todos los pasos de llamada y del primario (intrínseco local, `find_witness` `FunctionCall`/`ClosureCall`, guardias, oráculo), de ahí que el vacío llegue al primario y al primer paso.

## 2. Familia (reescrita): qué destinos de llamada llegan a efectos y cuáles se rechazan antes

**Barrido ampliado** (`.scratch/sweep3.py`, no versionado; **2.784 programas**: los **4 contratos** `no_io`/`no_alloc`/`no_block`/`no_suspend` × **29 formas de destino** × 6 contextos (sentencia, `let`, argumento de `Drop`, guarda de `match`, cuerpo de closure, closure anidado) × grupos 0..3, perfiles J y F5, SHA-256 de la lista `4e0d2072…9f59`; se cuenta cualquier primario o paso causal de longitud cero de la etapa de efectos en el candidato, sin contar `resource.drop`). Formas: nombre de closure, función directa, literal de closure (sin parámetros, con parámetro, con captura, `->Bool`, anidado, que llama a un nombre), `if`, prefijo `move`, `edit`, `view`, bloque `{f}`, bloque con valor `{value:f;}`, `match`, `f.x`, `-f`, `!f`, `f+f`, `Some(f)`, `None`, `[f]`, `[f][0]`, `f()` (llamada anidada), `arr[0]`, `K()` (resultado de llamada), `()`, `1`.

**Llegan a efectos** (verificado con ambos binarios): nombre (local, función, intrínseco, `Iterator.Next`), **literal de closure**, **`if` agrupado** y **destino con prefijo `move`** agrupado. Los tres últimos producen el defecto (única fuente de primarios/pasos vacíos en el barrido: `literal*`, `if` y `move`, en NX-E0412, NX-E0413, NX-E0414 y NX-E0415, también dentro de una guarda de `match`), p. ej.:
- `fn Main() with no_io{let f=fn (){Print(1);};(if true then f else f)();}` y `…(move f)();}`: NX-E0415 `closure.call`, primario `[44,44)` vacío (base y candidato idénticos; sin grupo `if …()` no parsea y `move f()` significa `move (f())`).
- `fn Main() with no_suspend{let f=fn (){MockUnknown();};(if true then f else f)();}`: NX-E0414 `closure.call`, `[54,54)` vacío (regla: `[55,57)`).
- Con el nombre `(f)()` y `((f))()` r1 ya da el nombre (`[45,46)`, `[46,47)`); el literal `(fn…)()` `[21,21)`.
- **Guardas de `match`:** una llamada con destino agrupado no nombre dentro de una guarda produce, si hay incumplimiento de contrato, NX-E0412..E0415 con el mismo defecto (guarda de `no_io` `[40,40)`, de `no_suspend` `[45,45)`). `NX-E1305/guard_effect` **no** se produce con destino agrupado no nombre (un literal `fn` que llama a `MockUnknown()` en la guarda se acepta con salida 0 sin contrato) y `(move f)()` en guarda da `NX-E1305/guard_transfer` en ownership antes de efectos: ese comportamiento es previo, está fuera de este corte y se propone registrarlo en `Pending.md`.

**Se rechazan antes de efectos** (verificado; el barrido no encuentra ningún vacío en ellos):

| destino agrupado | diagnóstico (base = candidato salvo que el primario se normaliza) |
|---|---|
| `(f())()` llamada anidada, `(Some(f))()`, `(None)()`, `(())()`, `(1)()`, `K()` | `NX-E0111/non_callable_target` `call.bind` (primario interior, p. ej. `[45,48)`) |
| `(f.x)()`, método/campo `(a.b)(x)`, `(g.Next)()` | `NX-E0006` `names.resolve` (sobre el `.`/campo, `[46,47)`) |
| `([f])()`, `([f][0])()`, `(arr[0])()` | `NX-E0006` `types.check` (arrays de closures pendientes, `[45,48)`) |
| `({f})()`, `({value:f;})()`, `(match …)()`, `(edit f)()`, `(-f)()` | `NX-E0002` `syntax.parse` (no es una expresión en estos perfiles) |
| `(view f)()` | `NX-E0002`/`NX-E0005` `syntax.parse` |
| `(!f)()` | `NX-E0001` `syntax.lex` |
| `(f+f)()` | `NX-E0102/type_mismatch` |
| `move f` dentro de una guarda `if (move f)()` | `NX-E1305/guard_transfer` `ownership.check` (el `move` en guarda se rechaza antes de efectos) |

**Sitios que leen texto de la fuente** (búsqueda exhaustiva de `source_text`, `call_head`, `next_head` en `src/`): `call_head` solo tiene tres usos: el fallback de `CallSites::head` (el defecto), el fallback de `next_head` y la cabeza del `yield` (token `yield` al inicio del span; sin grupo posible). `names.rs`, `types.rs`, `lower*.rs`, `interp` no leen texto de llamadas. Un solo sitio defectuoso.

## 3. Mecanismo único

**El índice de llamadas cubre toda llamada escrita**, con la posición de inicio de la expresión **interior** desagrupada; `call_head` sigue siendo el único lector de texto, pero siempre arranca en esa posición. Sin condicionales por código, por patrón de fuente ni por tipo de expresión.

- `scope_sites` apunta, para cada `Expr::Call`, `(span de la llamada, anclaje)`, con `anclaje = expression_primary_span(target)`: el nombre si el destino desagrupado es un `Name`, la expresión interior en otro caso.
- `CallSites` guarda `Site::Name(span)` / `Site::Anchor(span)`. `head(span, source)`: `Name(n)` -> `n` (sin cambio); `Anchor(a)` -> `call_head(&a, source)` (la cabeza de la expresión interior); sin entrada -> `call_head(span, source)` como hoy (roles sin llamada escrita). Para una llamada sin grupo el anclaje empieza en `span.start`, luego el resultado es **idéntico** al de la base para todo programa sin grupo.
- **Clave de orden de r1** (`order_start`): el mapa `starts` pasa a clave `(archivo, inicio de la cabeza)` con valor «inicio de la llamada» (menor inicio si dos llamadas comparten cabeza, p. ej. `(f())()` agrupada). Todo paso cuyo span empieza en el inicio de la expresión interior —el paso `closure.call` y, p. ej., el `value.drop` del temporal que es el propio literal— obtiene la misma clave (el inicio de la llamada): el empate de r1 se conserva igual que en la forma sin grupo. Con clave por span exacto el paso de llamada se mediría por la llamada y el drop conservaría su inicio crudo (el desajuste que r1 corrigió para nombres). Para una llamada sin grupo, `order_start` es la identidad.

Alternativa descartada: saltar `(` en `call_head` por lectura de texto (heurística de caracteres; falla con `()` unidad y con `Next`; el plan §3 la excluye).

## 4. Regla (a), ya ratificada: texto exacto del addendum nuevo (para `docs/adr/F0-0002-lagunas.md`; este fichero NO se edita aquí)

Se propone como sección propia, a continuación de «Primario de una expresión bajo agrupaciones»:

> ### Sitio de la llamada con destino agrupado que no es un nombre (relleno de laguna, revertible)
>
> Cuando una regla de F0-0002 §8.2 (`NX-E0412`, `NX-E0413`, `NX-E0414` y `NX-E0415`, los cuatro contratos de efectos) pide el sitio de una llamada cuyo destino es una expresión agrupada que **no es un nombre**, se retiran **todas y solo las agrupaciones exteriores** `Expr::Group` del destino y el primario y **todo paso de llamada** de ese destino (el primero de `cause_chain` y cada paso de llamada posterior, p. ej. el de un closure invocado dentro de otro closure agrupado) son la **cabeza** de la expresión interior: su **primer token**, esto es, la racha de caracteres de identificador (`A-Z`, `a-z`, `0-9`, `_`) que empieza donde empieza esa expresión; el espacio en blanco y los comentarios entre el paréntesis y ese primer token no son tokens (`(  f)()` tiene por primer token `f`). La regla rige también para las llamadas situadas en una guarda de `match`. El intervalo nunca es vacío: una expresión interior que no empiece por un carácter de identificador no alcanza la etapa de efectos (se rechaza antes por la regla que le corresponda). La cabeza de una expresión no depende de las agrupaciones que la rodean: `fn (){…}()` y `(fn (){…})()` dan `fn` en `[21,23)` y `[22,24)` respectivamente. Un destino que es un nombre sigue regido por «el nombre del destino» (sin cambio). `move` y `edit` son prefijos, no agrupaciones: en `(move f)()` la expresión interior empieza en `move` y su cabeza es `move`; se elige esa lectura de forma explícita en lugar de la lectura literal de §8.2 «el nombre del destino» (`f` tras el prefijo). La regla no cambia el código, la subcausa, la barrera, la aceptación, el orden causal ni el número de diagnósticos, y no afecta a ningún programa sin agrupación.
>
> Ejemplos (perfil `f0-0003-b-v1`; el primer paso de `cause_chain` coincide con el primario y cada paso de llamada con destino agrupado no nombre lleva su propia cabeza; los pasos que no son de llamada, p. ej. `intrinsic.call`, `value.build_array`, `value.drop`, no cambian). Offsets de bytes `[inicio,fin)`.
>
> | destino | programa inválido | diagnóstico esperado | válido |
> |---|---|---|---|
> | nombre | `fn Main() with no_io{let f=fn (){Print(1);};(f)();}` | `NX-E0415`, `effects`, `closure.call`, primario `[45,46)` (`f`); pasos `closure.call [45,46)`, `intrinsic.call [33,38)` | `fn Main(){let f=fn (){Print(1);};(f)();}` |
> | literal `fn` | `fn Main() with no_io{(fn (){Print(1);})();}` | `NX-E0415`, `closure.call`, `[22,24)` (`fn`); pasos `[22,24)`, `intrinsic.call [28,33)` | `fn Main(){(fn (){Print(1);})();}` |
> | `if` | `fn Main() with no_io{let f=fn (){Print(1);};(if true then f else f)();}` | `NX-E0415`, `closure.call`, `[45,47)` (`if`); pasos `[45,47)`, `[33,38)` | `fn Main(){let f=fn (){Print(1);};(if true then f else f)();}` |
> | `move` | `fn Main() with no_io{let f=fn (){Print(1);};(move f)();}` | `NX-E0415`, `closure.call`, `[45,49)` (`move`); pasos `[45,49)`, `[33,38)` | `fn Main(){let f=fn (){Print(1);};(move f)();}` |
> | `no_suspend` | `fn Main() with no_suspend{(fn (){MockUnknown();})();}` | `NX-E0414`, `effects`, `closure.call`, primario `[27,29)` (`fn`); pasos `[27,29)`, `intrinsic.call [33,44)` | `fn Main(){(fn (){MockUnknown();})();}` |
> | closures anidados | `fn Main() with no_io{(fn (){(fn (){Print(1);})();})();}` | `NX-E0415`, `closure.call`, `[22,24)`; pasos `closure.call [22,24)`, `closure.call [29,31)`, `intrinsic.call [35,40)` | la misma fuente sin `with no_io` |
> | trivia | `fn Main() with no_io{let f=fn (){Print(1);};(  f)();}` | `NX-E0415`, `closure.call`, `[47,48)` (`f`); pasos `[47,48)`, `[33,38)` | `(  f)()` sin contrato |
> | paréntesis dobles | `fn Main() with no_io{((fn (){Print(1);}))();}` | `NX-E0415`, `closure.call`, `[23,25)` (`fn`); pasos `[23,25)`, `intrinsic.call [29,34)` | `fn Main(){((fn (){Print(1);}))();}` |
> | bloque con valor | `fn Main() with no_io{let f=fn (){Print(1);};({f})();}` | no llega a efectos: `NX-E0002` `syntax.parse` `[45,46)` | `fn Main(){let f=fn (){Print(1);};(f)();}` |
> | campo / método | `fn Main() with no_io{let f=fn (){Print(1);};(f.x)();}` | no llega a efectos: `NX-E0006` `names.resolve` `[46,47)` | `(f)()` como arriba |
> | índice | `fn Main() with no_io{let f=fn (){Print(1);};([f][0])();}` | no llega a efectos: `NX-E0006` `types.check` `[45,48)` (arrays de closures pendientes) | `(f)()` como arriba |
> | llamada anidada | `fn Main() with no_io{let f=fn (){Print(1);};(f())();}` | no llega a efectos: `NX-E0111/non_callable_target` `call.bind` `[45,48)` | `(f)()` como arriba |
> | guarda de `match` | `fn Main() with no_io{match true{true if (fn ()->Bool{Print(1);return true;})()=>{},_=>{}}}` | `NX-E0415`, `closure.call`, `[41,43)` (`fn`); pasos `[41,43)`, `intrinsic.call [53,58)` | la misma fuente sin `with no_io` |
>
> *Alternativas no adoptadas:* el intervalo de la expresión interior completa y, para `move`, el nombre `f` tras el prefijo (lectura literal de §8.2). Ninguna rige ningún programa sin agrupación; cambiar la regla es revertible sin afectar a programas sin grupo.

**Qué filas dependen de la regla (a) y cuáles no.** *No dependen:* el destino nombre (`[45,46)` y dobles: ya rige «nombre del destino»); el literal `fn` con control sin grupo `[21,23)` (la regla se reduce a «la misma cabeza contada desde la expresión interior»). *Dependen de (a) y de su ratificación:* `if` (`[45,47)`, `[46,48)` doble) y `move` (`[45,49)`, `[46,50)` doble): no tienen control sin grupo (`if …()` no parsea; `move f()` es `move (f())`), sus esperados son la regla, no el binario.

## 5. Efectos laterales

- Programas sin grupo: idénticos (mismo `head`, misma clave de orden). Programas con grupo: solo cambia el span emitido de vacío a la cabeza (primario y **todo** paso de llamada con destino agrupado no nombre, que en llamadas anidadas son varios) y, por la clave de orden, nada más que lo fijado en r1.
- **Salidas protegidas, con binarios:** el candidato (que ya da las dos únicas formas conocidas de vacío) se ejecutó sobre `corpus/` (`.n`), `tests/fixtures/` (`.n` y todas las cadenas de fuente de los `.json`/`.jsonl`, incluidas las 152 filas RD de `i07c_rd.json`): 523 fuentes únicas × 3 perfiles × `check`/`run` = **3.138 ejecuciones, 0 diagnósticos de efectos con primario o paso de longitud cero** (SHA-256 de la lista `7049b39f…`). Como la regla solo cambia el span emitido en llamadas con destino agrupado no nombre (que hoy son vacíos) y la clave de orden solo difiere con grupos, **ninguna salida protegida puede cambiar**; TD/DG y la suite se vuelven a ejecutar completas en el paso 4.3 (`cargo nextest … --no-fail-fast`) y el RD (`i07c_rd_conformidad.py --verify`, 152/152) se repite.
- Cadenas de `closure.call`: cambia cada paso de llamada con destino agrupado no nombre (en `(fn (){(fn (){Print(1);})();})()` de `[closure.call [21,21), closure.call [28,28), intrinsic.call [35,40)]` a `[22,24), [29,31), [35,40)`); no cambian los pasos que no son de llamada (`intrinsic.call`, `value.build_array`, `value.drop`, `resource.drop`) ni los pasos de llamada con destino nombre (`[51,52)`, etc.).
- Costo: una entrada por llamada no nombre; sin recorrido nuevo. `types.rs`, la pila y `drive`/`resume_non_match` no se tocan.

## 6. Pruebas (esperados derivados del addendum/regla y del control, no del binario corregido)

Convención: fuentes ASCII, `check` y `run`, exactamente 1 diagnóstico, sin stdout/eventos (en `run` el programa se rechaza antes de ejecutar); el primario es la cabeza desplazada por los paréntesis; el primer paso causal es igual al primario; los pasos siguientes son los del control o, donde no hay control, los de la misma fuente con el destino sin cambiar (derivados de la posición del cuerpo del closure, anterior al destino y por tanto inmóvil). Para el esperado «nombre» el control `f()` y la fila de r1.

**Filas que no dependen de la regla (a)** (control sin grupo, mismos código/operación/pasos):

| fuente | contrato / código | primario = 1.er paso | pasos siguientes | control (primario; pasos) | positivo |
|---|---|---|---|---|---|
| `fn Main() with no_io{(fn (){Print(1);})();}` | NX-E0415 `closure.call` | `[22,24)` | `intrinsic.call [28,33)` | `fn Main() with no_io{fn (){Print(1);}();}` `[21,23)`; `[27,32)` | `fn Main(){(fn (){Print(1);})();}` |
| `…{((fn (){Print(1);}))();}` | idem | `[23,25)` | `[29,34)` | idem | idem con `((…))` |
| `fn Main() with no_alloc{(fn (){let a=[1];})();}` | NX-E0412 | `[25,27)` | `value.build_array [37,40)` | `…{fn (){let a=[1];}();}` `[24,26)`; `[36,39)` | `fn Main(){(fn (){let a=[1];})();}` |
| `…no_alloc{((fn (){let a=[1];}))();}` | NX-E0412 | `[26,28)` | `[38,41)` | idem | idem con `((…))` |
| `fn Main() with no_block{(fn (){let r=MockBlockingOpen(1);})();}` | NX-E0413 | `[25,27)` | `value.drop [35,36)`, `resource.drop` sin span | `…{fn (){let r=MockBlockingOpen(1);}();}` `[24,26)`; `[34,35)` | `fn Main(){(fn (){let r=MockBlockingOpen(1);})();}` |
| `…no_block{((fn (){let r=MockBlockingOpen(1);}))();}` | NX-E0413 | `[26,28)` | `[36,37)` | idem | idem |
| guarda `fn Main() with no_io{match true{true if (fn ()->Bool{Print(1);return true;})()=>{},_=>{}}}` | NX-E0415 | `[41,43)` | `intrinsic.call [53,58)` | `…if fn ()->Bool{Print(1);return true;}()=>{}…` `[40,42)`; `[52,57)` | misma fuente sin `with no_io` |
| doble grupo de la guarda | idem | `[42,44)` | `[54,59)` | idem | idem |
| literal anidado `fn Main() with no_io{(fn (){(fn (){Print(1);})();})();}` | NX-E0415 | `[22,24)` | `closure.call [29,31)`, `intrinsic.call [35,40)` | cada literal sin grupo da su `fn` | sin `with no_io` |
| literal que llama a un nombre `fn Main() with no_io{let f=fn (){Print(1);};(fn (){f();})();}` | NX-E0415 | `[45,47)` | `closure.call [51,52)`, `intrinsic.call [33,38)` | `…fn (){f();}()` `[44,46)`; `[50,51)`, `[33,38)` | sin `with no_io` |
| nombre (ya en r1, se añaden los contratos): `fn Main() with no_alloc{let f=fn (){let a=[1];};(f)();}` | NX-E0412 | `[49,50)` | `value.build_array [42,45)` | `f()` `[48,49)` | sin contrato |
| literal bajo `no_suspend` `fn Main() with no_suspend{(fn (){MockUnknown();})();}` | NX-E0414 `closure.call` | `[27,29)` | `intrinsic.call [33,44)` | `…{fn (){MockUnknown();}();}` `[26,28)`; `[32,43)` | `fn Main(){(fn (){MockUnknown();})();}` |
| `…no_suspend{((fn (){MockUnknown();}))();}` | NX-E0414 | `[28,30)` | `[34,45)` | idem | idem con `((…))` |
| guarda bajo `no_suspend` `fn Main() with no_suspend{match true{true if (fn ()->Bool{MockUnknown();return true;})()=>{},_=>{}}}` | NX-E0414 | `[46,48)` | `intrinsic.call [58,69)` | `…if fn ()->Bool{MockUnknown();return true;}()=>{}…` `[45,47)`; `[57,68)` | la misma fuente sin `with no_suspend` |
| nombre bajo `no_suspend`: `fn Main() with no_suspend{let f=fn (){MockUnknown();};(f)();}` | NX-E0414 | `[55,56)` | `intrinsic.call [38,49)` | `f()` `[54,55)` | sin contrato |
| trivia `fn Main() with no_io{let f=fn (){Print(1);};(  f)();}` | NX-E0415 | `[47,48)` | `[33,38)` | `f()` `[44,45)`, `[33,38)`: el espacio no es token | sin contrato |
| nombre bajo `no_block`: `…no_block{let f=fn (){let r=MockBlockingOpen(1);};(f)();}` | NX-E0413 | `[65,66)` | `value.drop [40,41)`, `resource.drop` | `f()` `[64,65)` | sin contrato |

**Filas que dependen de la regla (a)** (destino `if` y `move`; sin control sin grupo: los esperados son la regla):

| fuente | primario = 1.er paso | pasos siguientes | positivo |
|---|---|---|---|
| `fn Main() with no_io{let f=fn (){Print(1);};(if true then f else f)();}` (NX-E0415) | `[45,47)` | `intrinsic.call [33,38)` | `fn Main(){let f=fn (){Print(1);};(if true then f else f)();}` |
| `…((if true then f else f))();}` | `[46,48)` | `[33,38)` | idem con `((…))` |
| `fn Main() with no_io{let f=fn (){Print(1);};(move f)();}` | `[45,49)` | `[33,38)` | `fn Main(){let f=fn (){Print(1);};(move f)();}` |
| `…((move f))();}` | `[46,50)` | `[33,38)` | idem con `((…))` |
| `fn Main() with no_alloc{let f=fn (){let a=[1];};(if true then f else f)();}` (NX-E0412) | `[49,51)` | `value.build_array [42,45)` | sin contrato |
| `…no_alloc{…(move f)();}` | `[49,53)` | `[42,45)` | sin contrato |
| `fn Main() with no_block{let f=fn (){let r=MockBlockingOpen(1);};(if true then f else f)();}` (NX-E0413) | `[65,67)` | `value.drop [40,41)`, `resource.drop` | sin contrato |
| `fn Main() with no_suspend{let f=fn (){MockUnknown();};(if true then f else f)();}` (NX-E0414) | `[55,57)` | `intrinsic.call [38,49)` | sin contrato |
| `…no_suspend{…((if true then f else f))();}` | `[56,58)` | `[38,49)` | idem con `((…))` |
| `…no_suspend{let f=fn (){MockUnknown();};(move f)();}` | `[55,59)` | `[38,49)` | sin contrato |
| `…no_suspend{…((move f))();}` | `[56,60)` | `[38,49)` | idem con `((…))` |
| guarda `no_suspend` con `if`: `fn Main() with no_suspend{let f=fn ()->Bool{MockUnknown(); return true;};match true{true if (if true then f else f)()=>{},_=>{}}}` | `[93,95)` | `intrinsic.call [44,55)` | sin contrato |
| `…no_block{…(move f)();}` | `[65,69)` | `[40,41)` | sin contrato |
| guarda con `if`: `fn Main() with no_io{let f=fn ()->Bool{Print(1); return true;};match true{true if (if true then f else f)()=>{},_=>{}}}` | `[83,85)` | `intrinsic.call [39,44)` | sin `with no_io` |

(En guarda, `move f` se rechaza antes por `NX-E1305/guard_transfer`; no hay fila de efectos.) Los números anteriores se obtuvieron como «cabeza desde el inicio de la expresión interior» de la fuente, con el cuerpo del closure y los pasos siguientes tomados de la base (que solo yerra los pasos de llamada con destino agrupado no nombre).

**Corpus de regresión de todos los tipos de destino** (decisión B del orquestador): el barrido de §2 se incorpora como filas en `tests/i07c_group_primary.rs`: por cada uno de los **cuatro contratos** (`no_io`, `no_alloc`, `no_block`, `no_suspend`) y cada forma que alcanza efectos (nombre de función y de closure, intrínseco, `Iterator.Next`, literal sin parámetros / con parámetro / con captura / `->Bool` / anidado, `if`, `move`) en sentencia, `let`, argumento, guarda y cuerpo de closure, con 0/1/2 grupos, con esperado derivado de la regla (cabeza desplazada por `2×profundidad`) y control sin grupo cuando existe; y por cada forma que se rechaza antes, una fila con su diagnóstico y primario interior (tabla de §2; las de `non_callable_target` ya existen en el corte). Ninguna con esperado leído del binario corregido.

**Regresión de N1** (`lower.rs`, rama `Unary Consume`, `permission_error(id, primary, "consume")`): `fn G(y:view Option<I64>){let z=move (y);}fn Main(){}` -> `NX-E1105/permission_escalation`, `ownership/ownership.access`, primario `y` `[37,38)`, paso `borrow.read [5,6)`; control `move y` `[36,37)`; `move ((y))` `[38,39)`; positivo de un atributo `fn G(y:move Option<I64>){let z=move (y);}fn Main(){}` aceptado. Igual en F5. Hoy: base `[36,39)`, candidato `[37,38)`; el mutante `value.span()` crudo da `[36,39)`.

**Mutación inversa del mecanismo** (copia desechable, `--profile default`, SHA-256 de restauración): S1 `Site::Anchor` usa `call_head(span)` (span de la llamada) -> mueren las filas de literal/`if`/`move`; S2 `scope_sites` vuelve a indexar solo `Name` -> idem; S3 el anclaje usa `target.span()` (con el grupo) en lugar de `expression_primary_span(target)` -> vacío de nuevo, mueren las filas; S4 `starts` por span exacto en lugar de `(archivo, inicio)` -> debe morir por la fila del literal anidado / del literal con temporal propio (si ninguna lo distingue se exhibe con programa o se declara equivalente con argumento); S5 `starts` con mayor en lugar de menor inicio -> sin referencia observable (equivalente, se declara). N1: el mutante `value.span()` de la rama `Unary Consume` debe morir por la fila nueva.

## 7. Alcance del auditor `i07c_span_audit.py` (N2 de D')

Barrido con el código actual: accesos crudos `.span()` en `src/check/names.rs`: **6**; en `src/ir/effects.rs`: **1** (los demás spans salen de campos `span`/`name_span`, no de `Expr::span()`). **7 sitios, ≤ 20**, cada uno clasificado en una línea (validado por D):

| archivo:línea (actual) | expresión | clase y motivo |
|---|---|---|
| `names.rs:191` | `value.span().start` | `boundary`: cota inferior de la búsqueda de token |
| `names.rs:795` | `self.enter(expr.span())` | `identity`: clave de entrada/cuota del nodo |
| `names.rs:1085` | `target.span().clone()` | `relation`: `Context` del destino completo en `partial_transfer` |
| `names.rs:1186` | `a.value.span().clone()` | `unreachable_fallback`: primario de `duplicate_argument` sin etiqueta (inalcanzable, plan §2) |
| `names.rs:1193` | `a.value.span().clone()` | `relation`: span ocupado del slot |
| `names.rs:1516` | `value.span().clone()` | `test`: código de pruebas |
| `effects.rs:1191` | `guard.span().clone()` | `identity`: región de la guarda (contención de testigos) |

Se **incluye en S6-COR** con el inventario fijado por expresión y contexto (mismo mecanismo `REVIEWED`, `else: raise` ante un acceso crudo nuevo o un sitio revisado que cambia/desaparece), sin redefinir el criterio; el auditor recorre `[types.rs, names.rs, effects.rs]`, imprime el recuento por fichero y la aserción de `tests/i07c_structural_invariants.rs` pasa a `reviewed: 21/21`; las líneas de `effects.rs` se reconcilian tras r2. `--self-test`: caso verde de los tres ficheros; `.span()` crudo nuevo en `names.rs` y en `effects.rs` -> exit 1; reversión de un sitio revisado en cada archivo -> exit 1. Si al implementar saliera un criterio no cubierto, se aplica la salida mínima del orquestador (línea base congelada que solo falla ante accesos nuevos).

## 8. Documental/andamiaje y orden de aplicación (4.3, no empieza hasta que D valide esta propuesta y B el texto del addendum)

El diferencial dirigido (`sweep3.py`) y las filas se versionan en `docs/validation/i07c-s6cor/` con semilla/recuento/SHA-256 congelados antes de ejecutar. Un commit `S6COR-r2`: `scope_sites` con anclaje para toda llamada; `Site`, `head` y `starts` por `(archivo, inicio)`; filas de §6 en `i07c_group_primary.rs` y los tests unitarios mínimos en `effects.rs` (el índice cubre `Name` y `Anchor`; búsqueda y oráculo coinciden con el literal agrupado); el auditor ampliado con su `--self-test`. Después: puerta completa por el coordinador, repetición del barrido de §2 (0 primarios/pasos vacíos, 0 diferencias en programas sin grupo) y del diferencial de r1 (0 diferencias de testigo), y el RD 152/152.
