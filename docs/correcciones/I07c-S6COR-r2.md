# I07c-S6-COR r2: propuesta de corrección (paso 4.1; solo propuesta, sin tocar código ni tests)

Base de inspección: `9e91715` (candidato, incluye r1) y `1010938` (base). Todas las afirmaciones se comprobaron con el binario debug compilado de cada una (`check`; perfiles `f0-0003-b-v1`/`f0-0005-b-v1` salvo indicación). Origen: FAIL de D' (informe `C:\nexo-work\tmp\validador-dprima-s6cor\informe.md`, motivo 1 y N1, N2) y la decisión del orquestador sobre el auditor (A) y el corpus de regresión (B).

## 1. Defecto y causa raíz comprobada

**Síntoma.** El destino agrupado de una llamada que **no es un nombre** da primario y primer paso causal vacíos en los diagnósticos de efectos. Repro de D': `fn Main() with no_io{(fn (){Print(1);})();}`:

| | operación | primario | `cause_chain` |
|---|---|---|---|
| base y candidato | `closure.call` | `[21,21)` vacío | `[closure.call [21,21), intrinsic.call [28,33)]` |
| forma sin grupo `fn (){Print(1);}()` (control) | `closure.call` | `[21,23)` (`fn`) | `[closure.call [21,23), intrinsic.call [27,32)]` |
| **esperado** (addendum: «nunca un primario vacío por empezar la llamada en `(`»; grupos exteriores fuera) | `closure.call` | **`[22,24)`** | `[closure.call [22,24), intrinsic.call [28,33)]` |

Mismo fallo, comprobado con ambos binarios (idénticos entre sí): `((fn (){Print(1);}))()` `[21,21)`; NX-E0412 `with no_alloc{(fn (){let a=[1];})();}` `[24,24)`; NX-E0413 `with no_block{(fn (){let r=MockBlockingOpen(1);})();}` `[24,24)`; guarda de `match` bajo `no_io` `(fn ()->Bool{Print(1);return true;})()` `[40,40)`. Es preexistente (la base lo hace igual); S6-COR lo declara objeto («efectos: primario y paso causal de llamada») y el addendum lo prohíbe, así que bloquea.

**Causa.** `src/ir/effects.rs`:
1. `scope_sites` (recolector de llamadas escritas) solo añade una entrada `(span de la llamada, span del nombre)` cuando `expression_primary(target)` es `Expr::Name` (`if matches!(..., Expr::Name(..))`, ~líneas 1141-1150). Un destino que no es un nombre (hoy solo el literal de closure llega a efectos) no entra en `CallSites`.
2. `CallSites::head(span, source)` hace `self.0.get(span).cloned().unwrap_or_else(|| call_head(span, source))`: sin entrada cae en `call_head`, que lee el **texto de la fuente desde `span.start`** y toma la racha ASCII alfanumérica/`_` (`take_while(is_ascii_alphanumeric || '_')`). Para la llamada `(fn (){…})()` el span de la llamada empieza en `(` (byte 21): la racha es vacía y el resultado es `[21,21)`. Sin grupo el texto empieza en `fn`: racha `fn` = `[21,23)`.
3. `head` es el selector de **todos** los pasos de llamada y del primario: intrínseco local (`function_intrinsic_effect_with_context`), `find_witness` (`FunctionCall`/`ClosureCall`), guardias y el oráculo; por eso el vacío se propaga al primario y al primer paso.

Por tanto el barrido de grupos de r0/r1 cubrió solo destinos `Name`; el resto de destinos lee texto desde el inicio de la **llamada** (que, con grupo, es un `(`) en lugar del inicio de la **expresión interior**.

## 2. Familia: barrido dirigido y sitios que leen texto

**Qué destinos de llamada alcanzan efectos.** Diferencial dirigido (`.scratch/sweep3.py`, no versionado; 924 programas = 3 contratos `no_io`/`no_alloc`/`no_block` × 11 tipos de destino × 7 contextos (sentencia, `let`, argumento, guarda de `match`, cuerpo de closure, closure anidado, `for`) × grupos 0..3, perfiles J y F5; se cuenta cualquier primario o paso causal de longitud cero en el binario candidato; los pasos `resource.drop` sin span no cuentan). Tipos: nombre de closure, función directa, literal de closure (sin parámetros, con parámetro, con captura, `->Bool`), resultado de llamada `(K())()`, índice `(arr[0])()`, `if`/`match` como destino, `()`/`(1)` como destino, intrínseco:

- **Alcanzan la etapa de efectos únicamente el nombre (local, función, intrínseco, `Iterator.Next`) y el literal de closure.** Resultado de llamada, índice, `if`/`match`, unidad y entero se rechazan antes (types/names: `NX-E0111 non_callable_target` o `NX-E0006`/`NX-E0002`/`NX-E0005`); campo y método (`a.b(1)`, `(a.b)(1)`) se rechazan con `NX-E0006` pendiente en la etapa de nombres (`names.resolve`, sobre el `.`) en J y F5 y no llegan a efectos.
- **Primarios o pasos vacíos: solo el literal de closure agrupado**, en los 3 contratos y en todos los contextos (sentencia, `let`, argumento, cuerpo de closure, closure anidado, y guarda con literal `->Bool`): 20 combinaciones tipo×contexto, todas con grupo; **cero** con la forma sin grupo y **cero** con destino nombre (r1 ya cubría `(f)()`, `((F))()`, `(Print)(1)`). El contexto `for` no llega a efectos (`NX-E0006` pendiente: sobre el token `for` en J, en `types` en F5) y no aporta casos.

**Sitios que leen texto de la fuente o dependen de él** (búsqueda exhaustiva de `source_text`, `call_head`, `next_head` en `src/`): `call_head` solo tiene tres usos: el fallback de `CallSites::head` (el defecto), el fallback de `next_head` (cuando el span de una reanudación no contiene `.Next`) y la cabeza del `yield` (`effects.rs` ~1005, el token `yield` es el primer carácter del span, no hay grupo posible). `names.rs`, `types.rs`, `lower*.rs` e `interp` no leen texto de llamadas (usan AST y tokens). Conclusión: **un solo sitio defectuoso** (el índice de llamadas) y el resto de lecturas de texto son anclas de operaciones sin llamada escrita.

## 3. Mecanismo único

**El índice de llamadas cubre toda llamada escrita**, con la posición de inicio de la **expresión interior** (destino desagrupado); `call_head` sigue siendo el único lector de texto, pero ahora siempre arranca en esa posición. Sin condicionales por código, por patrón de fuente ni por tipo de expresión.

- `scope_sites` apunta, para cada `Expr::Call`, `(span de la llamada, anclaje)` donde `anclaje = expression_primary_span(target)` (el mismo ayudante del corte). Para un destino `Name` el anclaje es el span del nombre (como hoy); para cualquier otro es el span de la expresión interior.
- `CallSites` guarda por llamada un valor `Site::Name(span)` o `Site::Anchor(span)`. `head(span, source)`: `Name(n)` -> `n` (sin cambio); `Anchor(a)` -> `call_head(&a, source)` (la racha de caracteres desde el inicio de la expresión interior, es decir `fn` en `[22,24)`); sin entrada -> `call_head(span, source)` como hoy (roles sin llamada escrita). Para una llamada sin grupo `a.start == span.start`, luego el resultado es **idéntico** al de la base para todo programa sin grupo.
- **Clave de orden de r1** (`order_start`). El mapa `starts` pasa de clave `Span` (nombre) a clave `(archivo, inicio de la cabeza)` con valor «inicio de la llamada»: para un nombre, el inicio del nombre; para un destino no nombre, el inicio del anclaje. Con insertar el **menor** inicio de llamada si dos llamadas comparten inicio de cabeza (`f()()` agrupada: `(f())()`), el orden es determinista. Así, todo paso cuyo span empieza en el inicio de la expresión interior —el paso `closure.call` y, p. ej., el `value.drop` del temporal que es el propio literal de closure, cuyo span empieza ahí— obtiene la misma clave (el inicio de la llamada) y el empate que resuelve la identidad se conserva exactamente como en la forma sin grupo. (Con clave por span exacto, el paso `closure.call` pasaría a medirse por la llamada mientras el drop conservaría su inicio crudo, el mismo desajuste que r1 corrigió para nombres.)
- Sin programas sin grupo afectados: para una llamada sin grupo, inicio de la cabeza = inicio de la llamada = `span.start`, de modo que `order_start` es la identidad y `head` devuelve lo de la base. Para programas con grupo solo cambia el span emitido (de vacío al nombre/`fn`) y lo que r1 ya fijaba.

Alternativa descartada: saltar `(` en `call_head` por lectura de texto. Es heurística de caracteres (falla con `()` unidad como destino, con `Next`, y reintroduce la lectura textual como mecanismo); el plan §3 lo excluye expresamente.

**Lectura del ADR (no es laguna).** El addendum fija «primario = span de la expresión interior» y «nunca vacío por empezar la llamada en `(`»; el control sin grupo del literal de closure usa la convención ya vigente de cabeza (`fn`, la racha desde el inicio de la expresión), que este corte no cambia. Que el primario de un literal de closure invocado deba ser el literal completo en lugar de su cabeza sería una decisión nueva para programas **sin** grupo (otro intervalo en la base): si alguien la quiere, es Flujo A y esta propuesta no la toma.

## 4. Efectos laterales

- Programas sin grupo: idénticos (mismo `head`, misma clave de orden). Se comprobará con el diferencial de r1 y con el barrido de §2 (0 diferencias fuera del span emitido).
- Orden de testigos de r1: conservado; con la clave por `(archivo, inicio)` los pasos del temporal del propio destino agrupado empatan igual que sin grupo. El desempate para llamadas anidadas agrupadas con cabeza común queda definido (inicio menor) y no tiene referencia en la base (la base daba un span vacío): no cambia ningún programa existente de la suite.
- Cadenas de `closure.call`: solo cambia el primer paso (`[22,24)` en lugar de `[21,21)`); los pasos siguientes son los mismos (`intrinsic.call [28,33)`, `value.build_array [37,40)`, `value.drop`, `resource.drop`).
- Costo: una entrada más por llamada no nombre en el mapa; sin recorrido nuevo.
- `types.rs`, pila y `drive`/`resume_non_match`: no se tocan.

## 5. Pruebas (esperados derivados del addendum y del control sin grupo)

Convención: fuentes ASCII, `check` y `run`, exactamente 1 diagnóstico, sin stdout/eventos; el primario es la cabeza `fn` desplazada por los paréntesis; el primer paso causal es igual al primario; los pasos siguientes son los del control desplazados por los paréntesis (la expresión interior y su cuerpo no se mueven respecto del control más que por ellos). Perfil J salvo indicación (comprobado el mismo resultado en F5).

| fuente | código / operación | primario = 1.er paso | pasos siguientes | control sin grupo (primario; pasos) | positivo |
|---|---|---|---|---|---|
| `fn Main() with no_io{(fn (){Print(1);})();}` | NX-E0415 `closure.call` | `[22,24)` | `intrinsic.call [28,33)` | `fn Main() with no_io{fn (){Print(1);}();}` `[21,23)`; `[27,32)` | `fn Main(){(fn (){Print(1);})();}` |
| `fn Main() with no_io{((fn (){Print(1);}))();}` | idem | `[23,25)` | `[29,34)` | idem | `fn Main(){((fn (){Print(1);}))();}` |
| `fn Main() with no_alloc{(fn (){let a=[1];})();}` | NX-E0412 `closure.call` | `[25,27)` | `value.build_array [37,40)` | `…no_alloc{fn (){let a=[1];}();}` `[24,26)`; `[36,39)` | `fn Main(){(fn (){let a=[1];})();}` |
| `fn Main() with no_block{(fn (){let r=MockBlockingOpen(1);})();}` | NX-E0413 `closure.call` | `[25,27)` | `value.drop [35,36)`, `resource.drop` sin span | `…no_block{fn (){let r=MockBlockingOpen(1);}();}` `[24,26)`; `[34,35)` | `fn Main(){(fn (){let r=MockBlockingOpen(1);})();}` |
| guarda: `fn Main() with no_io{match true{true if (fn ()->Bool{Print(1);return true;})()=>{},_=>{}}}` | NX-E0415 `closure.call` | `[41,43)` | `intrinsic.call [53,58)` | `…if fn ()->Bool{Print(1);return true;}()=>{}…` `[40,42)`; `[52,57)` | `…` sin `with no_io` |
| doble grupo de la guarda | idem | `[42,44)` | `[54,59)` | idem | idem |

Además, y como corpus de regresión (decisión B del orquestador), el barrido de §2 se incorpora como **filas de regresión** de `tests/i07c_group_primary.rs` (familia `effects`), con el mismo formato: por cada contrato (`no_io`, `no_alloc`, `no_block`) y cada destino que alcanza efectos —nombre de función, nombre de closure, intrínseco, literal de closure sin parámetros, con parámetro (`(fn (x:view I64){…})(1)`), con captura (`(fn [view v](){…})()`), `Iterator.Next` — en sentencia, `let`, argumento, guarda de `match`, cuerpo de closure y closure anidado, con 0/1/2 grupos: el esperado de cada fila se **calcula del control sin grupo** (mismos código, operación, subcausa, pasos y contrato; los spans de cabeza y de lo que va tras el grupo desplazados por `2×profundidad`; ninguno derivado del binario corregido) y se verifica que el control sin grupo es idéntico entre base y candidato. Los destinos que no alcanzan efectos (resultado de llamada, índice, `if`/`match`, unidad, entero) entran con la regla que sí aplican (`NX-E0111 non_callable_target` `[50,53)` para `(f())()`, etc.), ya cubiertos por filas existentes del corte para `non_callable_target`; campo y método quedan rechazados con `NX-E0006` pendiente (nombres) y no llegan a efectos.

**Regresión de N1** (`lower.rs`, rama `Unary Consume`, `permission_error(id, primary, "consume")`): `fn G(y:view Option<I64>){let z=move (y);}fn Main(){}` -> `NX-E1105/permission_escalation`, `ownership/ownership.access`, primario `y` `[37,38)`, paso `borrow.read [5,6)` (la declaración del parámetro); control `move y` `[36,37)` (paso igual); doble grupo `move ((y))` `[38,39)`; positivo pareado de un atributo: `fn G(y:move Option<I64>){let z=move (y);}fn Main(){}` aceptado. Mismo resultado en F5. Hoy: base `[36,39)`, candidato `[37,38)`; el mutante `value.span()` crudo da `[36,39)`.

**Mutación inversa del mecanismo nuevo** (copia desechable, `--profile default`, SHA-256 de restauración): S1 `head` de `Anchor` usa `call_head(span)` (el span de la llamada) -> mueren las filas del literal agrupado; S2 `scope_sites` vuelve a indexar solo `Name` -> mueren las mismas; S3 el anclaje usa `target.span()` (span con el grupo) en lugar de `expression_primary_span(target)` -> vacío de nuevo, mueren las filas; S4 `starts` clave por span exacto en lugar de `(archivo, inicio)` -> debe morir por una fila del literal con temporal propio (`(fn (){…})()` donde el literal es además un temporal soltado; el orden de testigos cambia respecto del control) o, si ninguna fila lo distingue, se exhibe con programa o se declara equivalente con argumento; S5 inserción con mayor en lugar de menor inicio -> no observable en programas existentes (se anotará como equivalente por no existir referencia). N1: el mutante `value.span()` de la rama `Unary Consume` debe morir por la fila nueva.

## 6. Alcance del auditor `i07c_span_audit.py` (N2 de D')

Barrido hecho con el código actual: accesos crudos `.span()` en `src/check/names.rs`: **6**; en `src/ir/effects.rs`: **1** (ningún otro equivalente: el resto de spans salen de campos `span`/`name_span` de nodos, no de `Expr::span()`). Son 7 sitios, ≤ 20, y cada uno se clasifica con una línea:

| archivo:línea (actual) | expresión | clase y motivo |
|---|---|---|
| `names.rs:191` | `value.span().start` | `boundary`: cota inferior de la búsqueda de token, no es primario |
| `names.rs:795` | `self.enter(expr.span())` | `identity`: clave de entrada/cuota del nodo, conserva el grupo |
| `names.rs:1085` | `target.span().clone()` | `relation`: relación `Context` del destino completo en `partial_transfer` |
| `names.rs:1186` | `a.value.span().clone()` | `unreachable_fallback`: primario de `duplicate_argument` sin etiqueta, inalcanzable (plan §2) |
| `names.rs:1193` | `a.value.span().clone()` | `relation`: span ocupado del slot (relación `Context` del siguiente duplicado) |
| `names.rs:1516` | `value.span().clone()` | `test`: código de pruebas del módulo |
| `effects.rs:1191` | `guard.span().clone()` | `identity`: región de la guarda (contención de testigos), no primario |

Proposición: **incluirlo en S6-COR** con el inventario fijado por expresión y contexto (mismo mecanismo `REVIEWED` por línea y contexto, `else: raise` si aparece un acceso crudo nuevo o desaparece/cambia un sitio revisado), sin redefinir el criterio del auditor; el auditor pasa a recorrer una lista de ficheros `[types.rs, names.rs, effects.rs]` y a imprimir el recuento por fichero. Las líneas de `effects.rs` se reconcilian tras el cambio de r2 (que toca ese archivo). `--self-test`: caso verde de los tres ficheros; `.span()` crudo nuevo en `names.rs` y en `effects.rs` -> exit 1; reversión de un sitio revisado de cada archivo -> exit 1; la asertión de `tests/i07c_structural_invariants.rs` cuenta ahora `reviewed: 14/14` más los 7 nuevos por fichero (`reviewed: 21/21`). Si al implementar saliera un criterio no cubierto (más de ~20 sitios o una clase nueva con interpretación), se detiene y se aplica la salida mínima indicada por el orquestador (línea base congelada que falla solo ante accesos nuevos).

## 7. Documental/andamiaje

La fila NX-E0105 del plan sin llave final es un typo documental. El diferencial dirigido de §2 (`sweep3.py`) se versionará, con semilla/recuento/SHA-256 congelados antes de ejecutar, en `docs/validation/i07c-s6cor/` en el paso 4.3, junto con la repetición del diferencial de r1 (debe dar 0 diferencias fuera del span emitido).

## 8. Orden de aplicación propuesto (4.3)

Un commit `S6COR-r2`: `scope_sites` con anclaje para toda llamada; `Site`, `head` y `starts` por `(archivo, inicio)`; filas de §5 en `i07c_group_primary.rs` (efectos, guarda, barrido y N1) y los tests unitarios mínimos en `effects.rs` (índice cubre `Name` y `Anchor`; búsqueda y oráculo coinciden con el literal de closure agrupado); auditor ampliado a `names.rs`/`effects.rs` con su `--self-test` y la asertión estructural. Después, puerta completa por el coordinador y repetición del barrido de §2 (0 primarios/pasos vacíos) y del diferencial de r1 (0 diferencias de testigo).
