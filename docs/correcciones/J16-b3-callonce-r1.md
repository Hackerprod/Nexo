# J16, bloque 3 — propuesta r1, versión 2: segunda llamada de CallOnce

Fecha: 2026-11-17. Propuesta de C para D, paso 4.1 del Flujo B. Base de código: `181c6d9` (su `src/ir/lower.rs` coincide con `6d2f3db`), perfil `f0-0003-b-v1`. Requisito de núcleo: disponibilidad del receptor y diagnóstico de propiedad. No autoriza cambios de producción ni afirma veredicto. La tanda verde sin commit, su respaldo y el diff ajeno de `docs/adr/F0-0002-second-slice.md` quedan fuera.

## Cambios respecto a la versión anterior

1. Se incorpora la causa del modo diferido: cualquier `while` de la función activa `defer_ownership`; en emisión Live `check_available` devuelve `Ok` sin consultar disponibilidad y NX-E1107 se emite antes de que la pasada dispersa pueda ver el uso.
2. Se sustituye el adelanto local de `check_available` por una sola decisión de modo posterior a la prueba de propiedad. Se reutilizan la comprobación local y la pasada de propiedad existentes, sin otro emisor de E0202 ni otra pasada.
3. Se miden bucles, ramas, traslado, retorno hipotético, copia inválida y diagnósticos múltiples. M2 usa ahora un testigo que discrimina.

## 1. Contrato y causa raíz completa

Fuente negativa completa, ASCII sin LF:

```nexo
fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();f();}
```

F0-0003 §4.2: `move f(args)` consume el entorno, la llamada consumidora desarma `f`, y un modo insuficiente usa NX-E1107. F0-0003 §9.2, fila «Consumidor», exige **NX-E0202** si la llamada consumidora va seguida de `f()`. §9.1 C08 fija E0202/definitely_consumed sobre el receptor ya consumido; C09 fija E1107 sobre la primera llamada `f()` de CallOnce todavía disponible. §8.2 fija para E1107 `ownership/ownership.check`, `required_mode`, `requested_mode`, declaración y primario en el nombre receptor. §3.3 define unión de propiedad para caminos que continúan y una prioridad especial para operaciones prohibidas de guardas, sin dar a E1107 prioridad general sobre E0202.

En `src/ir/lower.rs::Task::OtherCallEnter`, `resolved_expr` obtiene el tipo de closure sin comprobar disponibilidad. La comparación de `mode_rank(receiver_mode)` con `mode_rank(required)` emite E1107 y hace `continue` antes de `named_closure_receiver`. Allí `check_available` detectaría `consumed_at` o `maybe_consumed_at` y produciría E0202. Sin `while`, adelantar el control local da el diagnóstico esperado.

Con cualquier `while` de la función, `Lower::defer_ownership` queda activo. En emisión Live, `check_available` devuelve `Ok` antes de mirar los mapas locales: estos no son autoridad para la unión del bucle. El E1107 temprano corta la bajada de la llamada; al quedar diagnóstico en `lower.block`, `compile_ast_with_flow_limit` descarta la función antes de `ownership_flow::analyze`. Esa pasada dispersa sí podría observar `TransferValue(f)` seguido del uso `BeginLoan(f)` y determinar E0202 y el backedge, pero nunca recibe el uso. El prototipo que quitó la deferencia de `check_available` resolvió `while` posterior y previo, pero dio E1107 tras `while{move f();}f()`, donde §3.3 exige `maybe_consumed`. **La causa son el orden de modo frente a disponibilidad y, en el modo diferido, el momento prematuro de esa decisión respecto de la CFG.**

En la base, `check`/`run`, debug/release: salida 1, E1107 [67,68), subcausa nula, `ownership/ownership.check`, declaración [32,33), `data={"requested_mode":"read","required_mode":"consume"}`, sin causa ni observaciones. El esperado de §§4.2/9.2 es E0202/definitely_consumed [67,68), `ownership/ownership.use`, declaración [32,33), consumo [58,66), `data={}`, causa `consume/value.consume` [58,66) de `binding:32`, evidencia `verified`, sin observaciones. Para bucles, spans y subcausa se derivan por bytes de cada fuente y la unión de §3.3.

## 2. Un mecanismo en la raíz y archivos

Para una closure con receptor **nombrado** y modo solicitado inferior al requerido, `OtherCallEnter` construye el objeto E1107 actual como **candidato de modo** asociado a `LocalId` y span del uso; no lo publica ni detiene la bajada. Continúa la llamada inválida solo para analizar propiedad, con lectura como modo de prueba: no consume el binding ni adquiere `edit` por una solicitud ya inválida. Esta IR queda contaminada por el candidato y no se verifica ni se entrega como ejecutable. No se atribuyen resultado ni efectos de ejecución a la llamada inválida. Los receptores no nombrados conservan su tratamiento actual.

`named_closure_receiver` mantiene su **única** llamada a `check_available` en la posición actual. En emisión no diferida o flujo hipotético `Analyze`, si el binding ya se consumió, devuelve E0202 y cancela solo el candidato de ese uso. En Live diferido, el control local se aplaza como hoy, pero la IR conserva el uso del `LocalId`; `ownership_flow::analyze` aplica la unión de la CFG y emite E0202 si procede. Después de esa pasada, un único punto de selección publica los candidatos sin E0202 del **mismo uso** (identidad de binding y span, no texto de fuente ni solo coincidencia de span). Un E0202 de otra llamada no cancela E1107. Se conserva la ordenación final existente.

Si otro error detiene la bajada antes de que haya función completa, se conservan los candidatos anteriores, salvo el del uso que acaba de recibir E0202 local. Esto preserva dos diagnósticos independientes. Tras `return`, `EmissionMode::Analyze` conserva la disponibilidad local aun sin instrucción ejecutable en la CFG, conforme a los flujos hipotéticos de F0-0003 §9.2. No se fuerza `defer_ownership=false`, no se añade pasada de propiedad y no se duplica `check_available`. La corrección es una sola: **decidir E1107 únicamente después de la prueba de disponibilidad correspondiente al estado del uso**.

Producción: **solo** `src/ir/lower.rs` (estado efímero de candidatos, continuación de la bajada inválida con lectura de prueba y selección posterior al análisis o error local). Se reutilizan `named_closure_receiver`, `check_available`, la instrucción de uso y `ownership_flow::analyze`; no se cambia `src/ir/ownership_flow.rs`, `src/ir/model.rs` ni `src/ir/verify.rs`. Si C descubre que no puede preservar el uso y los diagnósticos con este mecanismo, vuelve a propuesta antes de editar producción. El plan J16 r5 S6 permite `tests/j16_integrated.rs`, `scripts/j16_oracle.py` y `docs/validation/F0-0003-J16-matrix.json`; la ampliación de producción queda sujeta a D y al coordinador. No se tocan ADR, baseline ni scripts de puerta.

Alternativas descartadas por medición: quitar la deferencia de `check_available` pierde la unión tras bucle; adelantar solo el control local falla con cualquier `while`; suprimir E1107 acepta C09; detectar texto `while` o añadir un emisor paralelo de E0202 duplica reglas.

## 3. Matriz medida y justificación normativa

Medición offline en copia limpia de `181c6d9` bajo D:, `CARGO_TARGET_DIR` en D: y dos workers. El primer prototipo (control adelantado y deferencia desactivada) falló tras bucle. El segundo guardó candidatos, continuó la bajada con lectura de prueba y los filtró después de propiedad. El prototipo asoció candidatos por span; §2 exige además `LocalId` en la implementación para distinguir usos que coincidan en span. La tabla da `check` debug, fuentes ASCII sin LF, códigos/subcausas y spans primarios; §6 exige repetir en `check`/`run` debug/release y fijar el objeto completo. El caso principal de la v1 ya coincidía en esas cuatro combinaciones. Los negativos de `run` no deben tener observaciones.

Sea `P = fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};`; cada sufijo de la tabla se cierra con `}`, salvo fuente completa indicada.

| Caso: sufijo tras P o fuente completa | Base | Prototipo v2 | Regla del ADR |
|---|---|---|---|
| `move f();f();` | E1107 [67,68) | E0202/definitely [67,68) | §4.2 desarma `f`; §9.2 «Consumidor» exige E0202. |
| `move f();edit f();` con `var f` | E1107 [72,73) | E0202/definitely [72,73) | §4.2: `edit` no revive entorno desarmado; §9.2 «Consumidor». |
| `move f();move f();` | E0202/definitely [72,73) | Igual | §4.2 y C08: ya cruza disponibilidad. |
| `f();` | E1107 [58,59) | Igual | §4.2 y C09: `read` insuficiente sin consumo. |
| `f();f();` | E1107 [58,59), E1107 [62,63) | Igual | §4.2: llamada inválida no consume; §8.1 permite varios errores. |
| `move f();` | [] | Igual | Positivo C09 y «Consumidor». |
| `move f();f();var i=0;while i<1{i=i+1;}` | E1107 [67,68) | E0202/definitely [67,68) | El bucle posterior no revive `f`; §§4.2/9.2. |
| `fn Main(){var i=0;while i<1{i=i+1;}let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();f();}` | E1107 [92,93) | E0202/definitely [92,93) | El bucle previo no revive `f`; §§4.2/9.2. |
| `var i=0;while i<2{move f();f();i=i+1;}` | E1107 [85,86) | E0202/maybe [76,84) y E0202/definitely [85,86) | El cuerpo puede repetirse: unión de §3.3; la primera llamada puede encontrar `f` consumido en iteración anterior. La segunda sigue al consumo de esta iteración (§4.2). La base ocultaba el primer defecto. |
| `var i=0;while i<1{move f();i=i+1;}f();` | E1107 [92,93) | E0202/maybe [76,84) y E0202/maybe [92,93) | El cuerpo puede no ejecutarse o repetirse: §3.3 y §9.2 «Movimiento por brazo». El límite literal no fija el número de iteraciones en el análisis. |
| `if true{move f();}f();` | E1107 [76,77) | E0202/maybe [76,77) | §3.3 une caminos incluso con rama constante; §9.2 «Movimiento por brazo». |
| `if true{move f();}f();var i=0;while i<1{i=i+1;}` | E1107 [76,77) | E0202/maybe [76,77) | Misma unión de §3.3 en Live diferido. |
| `let g=move f;f();` | E1107 [71,72) | E0202/definitely [71,72) | §4.3: mover portador conserva dueño en destino y desarma origen; §§4.2/9.2. |
| `move f();return;f();` | E1107 [74,75) | E0202/definitely [74,75) | §9.2 «Flujos hipotéticos» conserva errores tras `return`; §4.2: `f` sigue desarmado. |
| `move f();f();f();` | E1107 [67,68), E1107 [71,72) | E0202/definitely en ambos | Dos usos independientes tras un consumo: §§4.2/9.2 y cardinalidad de §8.1. |
| `f();move f();f();` | E1107 [58,59), E1107 [71,72) | E1107 [58,59), E0202/definitely [71,72) | El primero conserva §4.2/C09; el segundo cae bajo §9.2 «Consumidor». No se pierde el primero. |
| `let g=f;g();f();` | E0201 [64,65), E1107 [66,67), E1107 [70,71) | Igual | §4.1 rechaza copia de closure con recurso; §8.1 no limita a un error por archivo. La v1 perdía E1107 en `g()`; v2 evita ese cambio y no decide aquí una política nueva de cascadas. |
| `f();var i=0;while i<1{i=i+1;}` | E1107 [58,59) | Igual | C09 también rige en Live diferido sin consumo previo. |
| `edit f();` con `let f` o `var f` | E1107 [63,64) | Igual | §4.2: `inout` insuficiente para CallOnce; la lectura de prueba no debe añadir E0105 por inmutabilidad. |

Controles sin cambio de la v1: C08 conserva E0202/definitely [35,36); closure Read con `Inspect(r)` permite dos llamadas y eventos `open, inspect, inspect, close` (§§4.2/9.2); closure mutable admite dos `edit f()`, mientras dos `f()` sobre la misma closure mutable conservan E1107 en ambos. El diferencial amplio de D sobre v1 no acredita v2; §6 exige repetirlo. La matriz identifica también cambios de **cardinalidad** en backedges, no solo sustituciones de código.

## 4. Regresiones y pruebas

1. En `tests/j16_integrated.rs`, añadir negativo literal `move f();f();` y positivo que quita solo el segundo uso. Fijar salida 1/0, cardinalidad 1/0 y el objeto completo de §1 (código, subcausa, etapa, operación, primario, relaciones, causa y datos). Ejecutar `check`/`run` debug/release. C09 y `f();f();` fijan E1107 con `required_mode`/`requested_mode` sin consumo; C08 y `move f();move f();` fijan E0202.
2. Añadir parejas para `while` previo/posterior, consumo y uso dentro de `while`, consumo en `while` y uso después, rama, `let g=move f;f()`, retorno hipotético y `edit f()` sin consumo previo. Calcular spans por bytes de fuentes literales antes de ejecutar. Fijar cardinalidad y objeto completo, incluidas subcausas `maybe_consumed`, relaciones de consumo/backedge y orden final. Probar también propiedad dispersa forzada con el gancho interno existente.
3. Añadir los dos casos con diagnósticos múltiples y la copia inválida de §3 como diferencial exacto. Fuzz acotado de secuencias read/edit/move y ubicación de `while`/rama/return: ningún pánico, `internal_error`, `InvalidIr` ni cuelgue; ninguna divergencia debug/release o `check`/`run`.
4. En `scripts/j16_oracle.py`, declarar fuentes y esperados literales antes de invocar Nexo, transcritos de F0-0003 §§3.3, 4.2, 8.1–8.2, 9.1–9.2 y A2 para causas; actualizar la fila CallOnce de `docs/validation/F0-0003-J16-matrix.json`. No generar esperados desde el código bajo prueba. Los positivos comprueban resultado y eventos; los negativos carecen de ejecución. Los nombres de pruebas describen regla y fuente.

## 5. Inversiones discriminantes

Una mutación por vez en copia offline, construir, correr el testigo y restaurar byte a byte con SHA-256 antes de la siguiente. Registrar SHA del blob inicial, mutado y restaurado; los hashes de v1 no valen para este mecanismo. Una sola pasada final de mutación de núcleo.

| Inversión | Testigo y fallo observable |
|---|---|
| M1: volver a publicar E1107 antes de disponibilidad/CFG | `move f();f();` y variante con `while` posterior: E1107 reaparece donde §9.2 exige E0202. |
| M2: omitir `check_available` del receptor nombrado en flujo local/hipotético | `move f();return;f();`: v2 da E0202/definitely [74,75); mutación aislada medida da E1107 [74,75). C08 y `move f();move f();` **no** discriminan: la pasada dispersa aún da E0202. |
| M3: decidir con `consumed_at` local o dejar cortar E1107 en Live diferido | `while{move f();}f()` y `move f();f();while...`: E1107 sustituye E0202/maybe o definitely. |
| M4: omitir modo tras aplazarlo | C09 y `f();f();` sin `move`: aceptación indebida o falta de E1107. |
| M5: cancelar todos los candidatos de la función ante un E0202 | `f();move f();f();`: se pierde E1107 previo e independiente, contrario a §8.1/C09. |

## 6. Orden, puerta y revisión

Tras veredicto de D sobre esta propuesta y decisión del coordinador sobre ampliar `src/ir/lower.rs`, C implementa §2, añade §4 y hace un commit por solución en `i07c-robustez`. Antes de entregar a D: `sh scripts/validate.sh` en clon limpio del commit (fmt, clippy `-D warnings`, tests debug/release y RD); diferencial contra `181c6d9` de §3 y corpus J16 declarado sin cambio; `check`/`run` debug/release con objeto diagnóstico completo; oráculo independiente; fuzz; medidor de programas objetivo ejecutables/total. Ejecutar §5 una vez al final. Un hallazgo fuera de §2 vuelve a propuesta, sin improvisar segunda ruta. D revisa diff, fidelidad, oráculos y repros y emite su propio veredicto. Esta versión 2 no afirma que la puerta se haya ejecutado.
