# I07c-S6-COR r1: propuesta de corrección (paso 4.1; solo propuesta, sin tocar código ni tests)

Base de inspección: `1552e81` (candidato), `1010938` (base). Todas las afirmaciones se comprobaron con el binario debug compilado de cada una (`check`, perfil `f0-0003-b-v1` salvo indicación). Origen: FAIL de D, motivo 1 (informe de D, `C:\nexo-work\tmp\validador-d-s6cor\informe.md`), más los ítems de tests N1 y N2 y la retirada de los `.pyc`.

## 1. Defecto y causa raíz comprobada

**Síntoma.** `fn Main() with no_io{(MockOpen)(1);}`:

| | operación | primario | `cause_chain` |
|---|---|---|---|
| sin grupo `MockOpen(1);` (base = candidato) | `intrinsic.call` | `[21,29)` | `[intrinsic.call [21,29)]` |
| con grupo, base | `intrinsic.call` | `[21,21)` vacío | `[intrinsic.call [21,21)]` |
| con grupo, candidato | `value.drop` | `[22,30)` | `[value.drop [21,34)]` |
| **esperado** (addendum: nombre del destino; «no cambia … el orden causal») | `intrinsic.call` | `[22,30)` | `[intrinsic.call [22,30)]` |

El candidato cambia de testigo (otra operación y otro paso) y deja primario y paso incoherentes entre sí.

**Causa.** Para esa función hay dos testigos locales del mismo efecto `io` (`function_intrinsic_effect_with_context`): `W_i` (la llamada intrínseca `MockOpen`, cadena de un paso `intrinsic.call`, entidad `prelude:mock_open`) y `W_d` (el `DropValue` del temporal que produce, cadena `value.drop`, entidad `temporary:21:0`). `find_witness` elige entre ellos con `compare_paths` (`src/ir/effects.rs`, función anidada de `find_witness`): primero `(unknown, nº de causas)` (empate: ambos conocidos, 1 causa); luego, si ambos caminos tienen arista, el inicio del sitio de la llamada; si no, el vector `a_offsets` = `cause.span.start` de cada paso de la cadena (líneas ~4084-4089); y solo después la identidad de entidad (`a_ids`, ~4090-4098). Es la regla de la base, `F0-0002-second-slice.md` línea 2633: «los empates se resuelven por la secuencia de offsets de sitios fuente y después por identidad».

Clave de orden comprobada (offsets de la cadena; entre paréntesis la identidad usada solo si empatan los offsets):

| programa | `W_i` | `W_d` | ganador |
|---|---|---|---|
| `MockOpen(1);` (base = cand.) | `[21]` (`prelude:mock_open`) | `[21]` (`temporary:21:0`) | empate -> identidad `p` < `t`: **`W_i`** |
| `(MockOpen)(1);` base | `[21]` (paso `[21,21)`) | `[21]` | empate -> identidad: **`W_i`** |
| `(MockOpen)(1);` candidato | **`[22]`** (paso `[22,30)`, el nombre) | `[21]` (paso `[21,34)`) | `[21] < [22]`: **`W_d`** |

Es decir: en S6-COR el paso de llamada pasó a medirse por el inicio del **nombre** (22, exigido por F0-0002 §8.2 para el span emitido), mientras que el paso `value.drop` conserva el inicio de la **llamada** (21). Se mezclan dos anclas en una misma clave de orden y desaparece el empate que resolvía la identidad. La base usaba siempre el inicio de la llamada (`call_head` empieza en `Instruction.span.start`).

**Lectura del ADR (no es laguna).** Lectura A (adoptada por D y por el texto del addendum: «No cambia … el orden causal»): «offsets de sitios fuente» son los **sitios** (inicio de la llamada/operación), independientes de qué intervalo se emita como primario; agrupar no cambia el orden. Lectura B (literal, sobre los spans emitidos de cada paso): daría `value.drop`, y entonces el addendum se contradiría («agrupar no cambia el orden») y el mismo programa sin grupo daría otro testigo que con grupo. Con el texto adoptado solo A es coherente, y la corrección que se propone la implementa. **No se decide ninguna laguna**; si el coordinador o D prefieren B, es Flujo A (el addendum tendría que enunciar que el temporal soltado puede ganar) y esta propuesta no procede.

## 2. Familia: barrido dirigido y sitios de código

**Sitios de `effects.rs`/`lower*.rs` cuyo orden depende de un offset que ahora toma el nombre:**

1. `compare_paths`, atajo de aristas (`left.start.cmp(&right.start)`, ~4077): compara el inicio del paso de llamada de dos caminos. Entre llamadas hermanas el orden por nombre y por llamada coincide (una llamada solo contiene a otra en su destino, que entonces no tiene sitio de nombre, o en sus argumentos, que empiezan más tarde): sin cambio de resultado, pero depende de la misma ancla -> se unifica.
2. `compare_paths`, `a_offsets`/`b_offsets` (~4084-4089): **el que causa el defecto** (mezcla pasos de llamada por nombre con pasos de drop/yield/otros por inicio de operación).
3. `find_witness_enumerated` (oráculo de test, `candidates.sort_by` ~4336, clave `offsets`): misma clave, debe compartirla.
4. Guardias, `candidates.sort_by_key(|(site, rank, ..)| (site, rank))` (~1806, `site = witness.primary.start`) y `errors.sort_by` por primario (~1949): ordenan **diagnósticos por su primario emitido** (F0-0002 §13.9: sitios independientes por `primary_span.start`); el primario es el nombre por el addendum. No son desempate de testigos de una misma causa; el barrido no encontró cambio de orden y se dejan.
5. `lower.rs`/`lower_match.rs`: no ordenan testigos; `first_f5_capability` ya guarda la clave cruda y emite el normalizado (§1 del plan). Sin dependencia nueva.
6. `ownership_flow.rs:537-539` / `sparse.rs:608-610` (orden y dedup por primario): no se tocaron; el barrido no mostró diferencias de orden en E0202/E1105/E0105.

**Diferencial dirigido (generador propio, `.scratch/sweep.py`, no versionado; semilla 5150, 1.500 programas, 3 contratos `no_io`/`no_alloc`/`no_suspend`, 1-3 sentencias de 24 plantillas con 0-3 grupos en llamadas intrínsecas, directas, closures, `Next`, guardias y temporales; perfiles J y F5; compara por programa los diagnósticos `(rule, operation, cadena (operación, entidad))`, sin spans):** 154 diferencias de testigo, **todas** de los dos tipos siguientes y todas con un constructor de recurso agrupado `(Mock…Open)(…)` en `Main` (0 diferencias en programas sin él):

- 126: `intrinsic.call -> value.drop` (primario de función, como el de D; incluye `F((MockOpen)(1))`, `Drop((MockOpen)(1))`, `let r=((MockOpen))(1);`, `[(MockOpen)(1)]`, y combinaciones con guardias y `Next`).
- 28: `closure.call` cuyo último paso pasa de `intrinsic.call` a `value.drop`, p. ej. `fn Main() with no_io{let f=fn (){(MockOpen)(1);};(f)();}`: base `[closure.call [49,50), intrinsic.call [33,33)]`, candidato `[closure.call [49,50), value.drop [33,46)]`. **El corpus de C no cubría este caso** (el cuerpo de un closure con temporal soltado).
Los diagnósticos de guardia (`NX-E1305`) y los pasos `function.call` con destino agrupado no cambian de testigo (`F((G)())`, `(K)(1)` en guardia: 0 diferencias).

## 3. Mecanismo único

La clave de desempate de testigos **no depende de los grupos**: usa el **inicio de la llamada escrita**, que es el que usaba la base; el span emitido (primario y paso) sigue siendo el nombre.

- `CallSites` (ya construido una vez en `analyze`, `effects.rs`) añade un segundo mapa `starts: HashMap<Span, u64>`: `span del nombre del destino -> inicio del span de la llamada`, poblado en la misma pasada de `scope_sites` (`calls.push((call_span, name_span))` ya los tiene a ambos). Método `order_start(&self, step: &Span) -> u64`: `starts.get(step).copied().unwrap_or(step.start)`. Un span de paso que no es el nombre de una llamada escrita (drop, yield, intrínsecos sintéticos, `Next`, declaraciones) conserva su inicio, que ya era el de la base.
- Se usa en los tres sitios de §2 (1, 2, 3) y solo ahí: `compare_paths` recibe `sites` (ya disponible en `find_witness`) y sustituye `span.start` por `sites.order_start(span)` en el atajo y en `a_offsets`/`b_offsets`; el oráculo hace lo mismo. Sin condicionales por código ni por patrón de fuente, sin tocar `Cause`, `Witness`, `Op` ni el JSON.
- Equivalencia con la base: para toda llamada escrita con destino `Name`, `order_start(nombre)` = inicio de la llamada = `call_head.start` de la base; para el resto de pasos no cambia nada. Por tanto el desempate coincide con el de la base en todos los programas, agrupados o no, y el único cambio observable de S6-COR en efectos vuelve a ser el span emitido.
- Seguridad de la clave: un `Cause.span` igual al span de un `Name` destino solo puede ser el paso de esa llamada (un `Name` expresión no coincide con un token de declaración).

## 4. Efectos laterales

- Solo cambia qué testigo gana en los programas de §2 (con grupo, `value.drop` vs intrinsic): vuelve al de la base; no cambia código, subcausa, número de diagnósticos ni el orden publicado por primario. Los 14.700 movimientos de primario del diferencial de C no se ven afectados (esas fuentes no tenían empate intrínseco/drop).
- Costo: un `HashMap` más por compilación (mismo recorrido, tamaño = nº de llamadas escritas) y una búsqueda por comparación de testigos (O(1)); las cuotas de trabajo `count_cfg_work` no se tocan.
- `types.rs`, `drive` y `resume_non_match` no se tocan: la pila no cambia (reserva por `f5-frame-sizes.py` igual que en el candidato).
- El auditor no se afecta (`effects.rs` no está en su inventario).

## 5. Pruebas

**Filas corregidas de `tests/i07c_group_primary.rs`** (esperados derivados de la regla del addendum aplicada al control sin grupo: se conserva la operación, la identidad de entidad y el número de pasos del control; los spans son el nombre, desplazado por los paréntesis; nunca del binario candidato). Todas `check` y `run`, 1 diagnóstico, `contract`/`related` iguales a los del control:

| fuente (perfil J) | operación | primario = paso | control sin grupo (primario = paso) | positivo |
|---|---|---|---|---|
| `fn Main() with no_io{(MockOpen)(1);}` | `intrinsic.call` | `[22,30)` | `MockOpen(1);` `[21,29)` | `fn Main(){(MockOpen)(1);}` |
| `fn Main() with no_io{((MockOpen))(1);}` | `intrinsic.call` | `[23,31)` | idem `[21,29)` | `…((MockOpen))(1);` |
| `fn Main() with no_io{(MockBlockingOpen)(1);}` | `intrinsic.call` | `[22,38)` | `MockBlockingOpen(1);` `[21,37)` | `fn Main(){(MockBlockingOpen)(1);}` |
| `fn F(r:view MockResource){}fn Main() with no_io{F((MockOpen)(1));}` | `intrinsic.call` | `[51,59)` | `F(MockOpen(1));` `[50,58)` | `fn F(r:view MockResource){}fn Main(){F((MockOpen)(1));}` |
| `…F(((MockOpen))(1));}` | `intrinsic.call` | `[52,60)` | idem `[50,58)` | `…F(((MockOpen))(1));}` sin `with no_io` |
| **nuevas (closure)** `fn Main() with no_io{let f=fn (){(MockOpen)(1);};f();}` | `closure.call` | `[49,50)` | `…fn (){MockOpen(1);};f();` primario `[47,48)`, cadena `[closure.call [47,48), intrinsic.call [33,41)]` | `…let f=fn (){(MockOpen)(1);};f();}` sin `with no_io` |

Cadena esperada de la fila closure: `[closure.call [49,50), intrinsic.call [34,42)]` (el nombre `MockOpen` dentro de `(MockOpen)` empieza en 34); con `((MockOpen))` y `((f))()`: `[closure.call [53,54), intrinsic.call [35,43)]`. Las cuatro filas de la familia que hoy fijan `value.drop` en `i07c_group_primary.rs` (~1019-1070 y su doble grupo) se sustituyen por las de la tabla (operación y cadena de un paso `intrinsic.call`, no `value.drop`); en el generador de filas ya no hay sobrescrituras (`OV`).

**Regresión estructural** (tests unitarios en `effects.rs`, módulo `witness_differential_tests`): (a) `order_start` devuelve el inicio de la llamada para destino bare, agrupado y de closure, y el propio inicio para un paso que no es nombre de llamada; (b) `witness_search_and_enumerator_agree_on_grouped_and_closure_calls` se amplía con `fn Main(){let r=(MockOpen)(1);}`/cuerpo de closure con temporal agrupado: búsqueda y enumerador eligen el mismo testigo (`intrinsic.call`), que además coincide con el de la fuente sin grupo.

**Mutación inversa del mecanismo** (copia desechable, `--profile default`, parada en el primer fallo, restauración por SHA-256): R1 `order_start` devuelve `step.start` (identidad); R2 `compare_paths` sin `order_start` solo en `a_offsets`/`b_offsets` (dejando el atajo); R3 el oráculo `find_witness_enumerated` sin `order_start`; R4 `starts` vacío al construir `CallSites`. Deben morir respectivamente por las filas de la tabla (R1, R2, R4) y por el test (b) (R3). R2 con el atajo conservado no es equivalente: se exhibe con las filas `(MockOpen)(1);` (empate solo en el vector de offsets).

**Ítems de tests solamente (no bloquean; hay programas), esperados derivados del addendum (grupos fuera; `move` prefijo; control bare):**

- N1 (perfil J, `lower.rs` ~10531, pierna `conflict` de `move (a)`): `fn Main(){var a=Some(1);match edit a{Some(x)=>{let b=move (a);},None=>{}}}`: `NX-E1105/active_borrow_conflict`, `ownership/ownership.access`, primario `a` `[59,60)`, paso `borrow.inout [35,36)`; control `let b=move a;` `[58,59)` (base y candidato iguales); `move ((a))` primario `[60,61)`; positivo: `fn Main(){var a=Some(1);let b=move (a);}`. Hoy: base `[58,61)`, candidato `[59,60)`.
- N2 (perfil F5, `names.rs` ~1208, brazo intrínseco `Inout`): `fn Main(){var a=[1];Push((a),2);}` -> `NX-E0111/argument_mode_mismatch` `call.bind` primario `[26,27)`, control `Push(a,2);` `[25,26)`; `fn Main(){var a=[1];let z=Pop(((a)));}` -> `[32,33)`, control `Pop(a)` `[30,31)`; `gen fn G()->I64{yield 1;}fn Main(){var g=G();Iterator.Next((g));}` -> `[60,61)`, control `Iterator.Next(g);` `[59,60)`, **positivo** `Iterator.Next(edit g);` aceptado. Para `Push`/`Pop` no existe positivo aceptado en F5 (`Push(edit a,2)` da `NX-E0006` pendiente de arrays): el emparejamiento es el control bare con la misma regla, y el positivo lo aporta `Next`. Hoy base `[25,28)`, `[30,35)` y `[59,62)`.

**Documental/andamiaje:** los tres `.pyc` de `docs/validation/i07c-s6cor/__pycache__/` ya se eliminaron del árbol de trabajo (`git status` los muestra borrados; el `git rm` lo hace el coordinador); el `.gitignore` ya cubre `scripts/**/__pycache__/` pero no este directorio: valdría añadir `docs/validation/**/__pycache__/` (decisión del coordinador). La fila NX-E0105 de la tabla del plan sin la llave final es un typo documental del plan, no de código.

## 6. Orden de aplicación propuesto (4.3)

Un solo commit `S6COR-r1`: `CallSites.starts` + `order_start` + los tres sitios de §3; reemplazo de las filas de §5 y nuevas de closure/N1/N2 en `i07c_group_primary.rs`; test unitario de `effects.rs`; sin tocar el generador de diferencial. Después: puerta completa por el coordinador, y repetir el barrido de §2 (debe dar 0 diferencias de testigo) y el diferencial de C (14.700 movimientos de primario sin cambio; ya no habrá fuentes no explicadas).
