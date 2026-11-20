# J16, bloque 3 — propuesta r1, versión 3.1: préstamo del receptor de CallOnce

Fecha: 2026-11-17. Propuesta de C, paso 4.1 del Flujo B, sobre `50b78cf` en `i07c-robustez`. Requisito de núcleo: disponibilidad del receptor y diagnóstico de propiedad de F0-0003. Este documento no autoriza cambios de código ni afirma un veredicto.

## Cambios respecto a la versión anterior

1. D' encontró B1 en la implementación de la v2 (`9cb2da1`): tras `return`, `break` o `continue`, una llamada CallOnce inválida deja un préstamo de lectura de prueba y añade NX-E1105 falso al uso posterior. La v2 no cubría este cierre.
2. La traza localiza el cierre ausente en `Task::UserCallNext`, no en la rama `receiver.diverged()` de `OtherCallEnter`: el receptor devuelve `Flow::Value { temp: None }` en `Analyze` y llega al final de los argumentos, donde se omite la liberación normal. Se añade un único cierre de ese préstamo al final de la misma ruta de llamada cuando no se emite `CallClosure`.
3. Se sustituye la matriz por medición de 29 fuentes en `check`/`run`, debug/release, se incorporan los tres testigos principales de B1 y `Drop(f)`, y se documenta el NX-E1105 legítimo de la captura `edit f` que faltaba en la tabla v2.
4. D midió que el mismo préstamo queda activo en `Analyze` después de llamadas válidas: 56 de 107 fuentes cambiadas pasan de rechazo por E1105 falso a aceptación. Se incorpora un caso aceptado, se limita el cierre a `Analyze` y se retira una inversión cuyo testigo no discrimina.

## 1. Contrato, reproducción y causa raíz medida

Fuente ASCII sin LF del testigo principal:

```nexo
fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};return;f();move f();}
```

Con `--profile f0-0003-b-v1 --format json`, `50b78cf` rechaza con NX-E1107 [65,66) y NX-E1105/active_borrow_conflict [74,75); el segundo lleva `loan:65:0`, causa `borrow.read` y `requested_access=consume`. El esperado es **solo NX-E1107 [65,66)**, `ownership/ownership.check`, `related_spans=[declaration [32,33)]`, `data={"requested_mode":"read","required_mode":"consume"}`, sin causa. El archivo histórico `ccf6df1`, compilado desde `git archive` en la copia, da solo ese E1107 tanto para la fuente anterior como para `f();move f();` sin `return`. Por tanto, B1 fue introducido por la continuación de bajada inválida de la v2; no es un cambio de la regla de disponibilidad.

F0-0003 §4.2 fija que `f()` solicita read y una closure consumidora exige consume: E1107 no consume ni adquiere un permiso persistente. §9.2 exige analizar los flujos hipotéticos tras `return` y que una llamada consumidora previa desarme el receptor; §3.3 une propiedad de caminos que continúan. La lectura de prueba de la v2 sirve para consultar disponibilidad antes de decidir E1107 o E0202, no para dejar una deuda de préstamo de una llamada que no se emitió. La misma omisión de cierre afecta a llamadas válidas en `Analyze`: el préstamo del receptor tampoco debe sobrevivir al fin de la llamada hipotética. Un E1105 real por otro portador de permiso sigue siendo obligatorio.

Traza de la copia de `50b78cf`:

| Punto en `src/ir/lower.rs` | Medición de B1 |
|---|---|
| `Task::OtherCallEnter`, líneas 7477 y 7555 | Guarda E1107 como candidato de `LocalId(1)` [65,66) y degrada solo la prueba a `Read`. |
| `named_closure_receiver`, líneas 4556 y 4633 | `check_available` acepta `f` disponible; crea `LoanId(65)` en `active_loans[f]`, longitud 0→1, con `emission=Analyze`. |
| `Task::UserCallNext`, líneas 7848 y 7962 | Terminan los argumentos. La rama `Analyze` devuelve valor sin temp y hace `continue` antes del cierre del `receiver_loan` de la línea 7990; longitud sigue 1. No se entra en `receiver.diverged()` de la línea 7594. |
| `Lower::conflict`, línea 3270 | `move f()` [74,75) ve `active_loans[f]=1` y fabrica E1105 con origen [65,66). Sin `return`, el cierre normal de la línea 7990 deja longitud 0 antes del `move`. |
| Prototipo en el punto terminal de `UserCallNext` | Libera el `receiver_loan` no emitido al terminar argumentos: longitud 1→0; persiste E1107 [65,66) y desaparece únicamente E1105 [74,75). |

La función de liberación existente es `Lower::release_call_loans` (línea 3720); `end_call_loans` (línea 3728) registra el fin en Live y no emite IR en Analyze. No se toca `check_available`, ni su deferencia en bucles, ni `ownership_flow`.

## 2. Corrección mínima y efectos previstos

En **una sola ruta de `src/ir/lower.rs`**, al llegar al punto terminal de `Task::UserCallNext` (después de todos los argumentos), cerrar el `receiver_loan` de `CallKind::Closure` en la rama `self.emission == Analyze`: retirarlo de `active_loans` con `release_call_loans` y finalizarlo con `end_call_loans` antes de devolver `Flow::Value { temp: None }`. Esto usa el mismo `ReceiverLoan=(LocalId,LoanId,Span)` y los mismos métodos que el cierre normal de la llamada; no introduce otra bajada de receptor, pasada de propiedad, emisor de diagnóstico, adaptador, patrón de fuente, constante especial ni cambio de visibilidad. En Live, el préstamo sigue vivo durante argumentos y llamada y conserva su cierre actual. El prototipo cambia solo esa rama y ninguna otra función.

El candidato E1107 se conserva hasta el selector de modo de la v2: se publica si no hay E0202 del mismo `LocalId` y span, y no se borra por un E1105 ajeno. Si `f` ya se consumió, el `check_available` local o `ownership_flow` prevalecen con E0202, incluso tras `return` o en `while`. Si un portador `h` retiene `edit f`, su préstamo propio queda activo y E1105 sigue siendo legítimo; la corrección solo retira `LoanId` creado para el receptor de la llamada que termina. Las cascadas E0201/E0105 previas y los candidatos independientes conservan la política de la v2. Una cascada con error de bajada previo puede dejar E1107 en vez de E0202: está fuera de B1 y no se redecide aquí.

Alternativas descartadas: borrar todos los préstamos antes del siguiente uso ocultaría conflictos reales de §4.3; filtrar E1105 por código/span de fuente sería un parche diagnóstico; hacer que el receptor inválido consuma `f` violaría §4.2; quitar `defer_ownership` rompería uniones de §3.3; otro camino especial para `return`, `break` o `continue` duplicaría la misma regla de fin de llamada.

## 3. Matriz de efectos laterales medida

Medición offline en `D:\nexo-work\tmp\`: `git archive 50b78cf`, `CARGO_TARGET_DIR` en D:, `cargo -j 2`; prueba de concepto **solo en la copia**. La matriz inicial contiene 29 fuentes completas ASCII sin LF; la regresión aceptada añadida eleva el total a 30. En las 29 filas medidas, `check` y `run` dieron los mismos diagnósticos y salida 1 salvo el positivo (salida 0); debug y release coincidieron. La notación `E7`, `E5`, `E2!`, `E2?`, `E1` abrevia respectivamente NX-E1107, NX-E1105/active_borrow_conflict, NX-E0202/definitely_consumed, NX-E0202/maybe_consumed y NX-E0201; cada número entre corchetes es el span primario [inicio,fin). Los negativos de `run` no producen observaciones. `P` es `fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};` y cada sufijo se cierra con `}`; cuando se usa `i`, se declara en el sufijo.

| Sufijo de P o fuente indicada | `50b78cf` → prototipo | Justificación F0-0003 |
|---|---|---|
| `return;f();move f();` | E7 [65,66), E5 [74,75) → E7 [65,66) | B1: §4.2, §9.2; prueba read no retiene. |
| `return;f();Drop(f);` | E7 [65,66), E5 [74,75) → E7 [65,66) | B1 con Drop; misma disponibilidad. |
| `var i=0;while i<1{break;f();move f();}` | E7 [82,83), E5 [91,92) → E7 [82,83) | B1 tras `break` hipotético. |
| `var i=0;while i<1{continue;f();move f();}` | E7 [85,86), E5 [94,95) → E7 [85,86) | B1 tras `continue` hipotético. |
| `return;f();` | E7 [65,66) → igual | El cierre no suprime E1107. |
| `f();return;move f();` | E7 [58,59) → igual | La llamada inválida previa en Live ya se cerraba. |
| `f();move f();return;` | E7 [58,59) → igual | El movimiento posterior no retroactúa. |
| `move f();return;f();` | E2! [74,75) → igual | §9.2: estado consumido en flujo hipotético. |
| `move f();return;f();move f();` | E2! [74,75), E2! [83,84) → igual | Disponibilidad tiene prioridad local. |
| `return;move f();f();` | E2! [74,75) → igual | Consumo hipotético antes de uso. |
| `if true{return;f();move f();}` | E7 [73,74), E5 [82,83) → E7 [73,74) | B1 dentro de rama; §3.3/§9.2. |
| `if true{f();return;move f();}` | E7 [66,67) → igual | `return` después de llamada inválida. |
| `if true{return;}else{f();move f();}` | E7 [79,80) → igual | Solo el otro brazo continúa; §3.3. |
| `var i=0;while i<1{break;f();Drop(f);}` | E7 [82,83), E5 [91,92) → E7 [82,83) | B1, variante con Drop en bucle. |
| `var i=0;while i<1{f();break;move f();}` | E7 [76,77) → igual | `break` después de llamada inválida. |
| `var i=0;while i<1{f();continue;move f();}` | E7 [76,77) → igual | `continue` después de llamada inválida. |
| `var i=0;while i<1{break;}f();move f();` | E7 [83,84) → igual | El uso exterior está en Live. |
| `var i=0;while i<1{i=i+1;continue;}f();move f();` | E7 [92,93) → igual | El uso exterior está en Live. |
| `let g=f;g();f();` | E1 [64,65), E7 [66,67), E7 [70,71) → igual | Copia prohibida y dos candidatos independientes; §§4.1/8.1. |
| `return;let g=f;g();f();` | E1 [71,72), E7 [73,74), E7 [77,78) → igual | Misma cascada en Analyze. |
| `move f();f();` | E2! [67,68) → igual | Consumidor de §9.2. |
| `var i=0;while i<1{i=i+1;}move f();f();` | E2! [92,93) → igual | Bucle previo no revive `f`. |
| `var i=0;while i<1{move f();f();i=i+1;}` | E2? [76,84), E2! [85,86) → igual | Backedge y consumo en el cuerpo; §3.3. |
| `f();move f();` | E7 [58,59) → igual | Sin consumo previo, la llamada inválida no consume. |
| `var i=0;while i<1{f();move f();i=i+1;}` | E2? [76,77), E2? [80,88) → igual | Reiteración del bucle y unión de §3.3; preexistente. |
| `move f();f();move f();` | E2! [67,68), E2! [76,77) → igual | Dos usos tras consumo. |
| `var i=0;while i<1{move f();f();move f();i=i+1;}` | E2? [76,84), E2! [85,86), E2! [89,97) → igual | Backedge y usos posteriores. |
| Fuente completa: `fn Main(){let r=MockOpen(1);var f=fn [move r](){Drop(r);};let h=fn [edit f](){};f();Drop(h);}` | E5 [80,81), E7 [80,81) → igual | **E1105 legítimo**: `h` retiene `edit f` (§4.3); E1107 por modo (§4.2). Cambio de base a v2 que faltaba en tabla anterior. |
| `move f();` | []/salida 0 → igual | Positivo consumidor C09/§9.2. |
| Fuente completa añadida: `fn Main(){let r=MockOpen(1);let f=fn [move r](){Inspect(r);};return;f();move f();}` | E5 falso en la base → []/salida 0 con el cierre | Llamada válida de lectura en flujo hipotético: §4.2 y §9.2; su préstamo termina al acabar la llamada. |

En la matriz inicial cambian los seis miembros de la familia B1: los cuatro testigos `return`/`break`/`continue`/`Drop` y sus dos ubicaciones hipotéticas adicionales en rama y bucle. La nueva fila aceptada también cambia. Las otras 23 fuentes conservan código, subcausa, primario y cardinalidad. En el diferencial independiente de D sobre 4.792 fuentes, **107 fuentes cambian** (214 celdas `check`/`run`, idénticas en debug/release): 51 conservan E1107 y pierden E1105 falso; 56 fuentes con llamada válida pierden su único E1105 falso y pasan de rechazo a aceptación. Todas tienen `return`, `break` o `continue` antes de la llamada; ninguna fuente sin flujo divergente cambia y ninguna añade diagnósticos. El diferencial amplio de D' de 6.643 fuentes contra `ccf6df1` permanece como evidencia de la v2, no se atribuye al prototipo de esta propuesta.

## 4. Regresiones públicas y oráculo

Después de que D valide esta propuesta, C añadirá en `tests/j16_integrated.rs` y en `scripts/j16_oracle.py` esperados literales independientes del compilador para estas fuentes completas. Cada negativo exige salida 1, cardinalidad exacta, `code`, `subcause`, span primario, `stage`, `operation`, relación de declaración, datos y ausencia de causa para E1107, en `check`/`run` debug/release; el oráculo confirma que `run` rechazado no tiene observaciones. El positivo exige salida 0 y ausencia de diagnósticos. Sin la corrección, los B1 fallan por un segundo diagnóstico y el positivo por E1105; con la corrección de prueba, pasan.

| Regresión nueva (P + sufijo + `}`) | Esperado exacto de diagnósticos | Testigo positivo o control |
|---|---|---|
| `return;f();move f();` | solo NX-E1107/null [65,66) | `return;move f();` sin E1107; y `move f();` ejecuta. |
| `var i=0;while i<1{break;f();move f();}` | solo NX-E1107/null [82,83) | `var i=0;while i<1{break;move f();}` sin E1105. |
| `var i=0;while i<1{continue;f();move f();}` | solo NX-E1107/null [85,86) | `var i=0;while i<1{continue;move f();}` sin E1105. |
| `return;f();Drop(f);` | solo NX-E1107/null [65,66) | Control de operación `drop` [74,75), sin E1105 espurio. |
| Fuente completa `fn Main(){let r=MockOpen(1);let f=fn [move r](){Inspect(r);};return;f();move f();}` | []/salida 0 | El gemelo `f();move f();` sin divergencia también se acepta; §4.2 y §9.2. |

Conservar y ejecutar las regresiones v2 para `move f();f();` (E0202), `f();f();` (dos E1107), `f();move f();f();` (E1107 y E0202), `let g=f;g();f();` (E0201 y dos E1107) y `let h=fn [edit f](){};f();Drop(h);` con `var f` (E1105 y E1107 legítimos). El oráculo publica fuente, esperado y spans antes de invocar Nexo; nunca calcula esperados desde la salida. Se mantienen fuzz acotado sin pánico, `internal_error`, `InvalidIr` ni cuelgue, y diferencial `check`/`run` y debug/release.

## 5. Inversiones discriminantes previstas

Una sola pasada al final sobre archivos de núcleo, una mutación por vez, restauración byte a byte y SHA-256 de blob inicial, mutado y restaurado. Las filas son testigos **previstos para la implementación**, no resultados de una campaña de esta propuesta.

| Inversión | Testigo que debe matarla |
|---|---|
| Quitar el nuevo cierre en `UserCallNext` para `Analyze` | Los tres B1 públicos: reaparece E1105/active_borrow_conflict [74,75), [91,92) o [94,95) además de E1107. |
| Suprimir o publicar anticipadamente el candidato E1107 | C09/`f();` pierde E1107 o `move f();f();` vuelve a E1107 en lugar de E0202. |
| Omitir disponibilidad local hipotética | `move f();return;f();` cambia E0202/definitely [74,75) por E1107. |
| Usar `consumed_at` local en Live diferido o cancelar todos los candidatos ante cualquier E0202 | `while{move f();}f()` pierde `maybe_consumed`; `f();move f();f();` pierde el E1107 independiente. |

Se descarta la inversión «cerrar el préstamo de un portador ajeno»: D comprobó que sustituir el cierre del `ReceiverLoan` por `active_loans.remove(&root)` conserva E1105 en `let h=fn [edit f](){};f();Drop(h);`, porque ese diagnóstico procede de la retención de `h`. El testigo previsto no distingue la inversión y no se atribuye una muerte de mutante inexistente.

## 6. Archivos permitidos, orden y puerta

Producción: únicamente `src/ir/lower.rs`. Evidencia: `tests/j16_integrated.rs`, `scripts/j16_oracle.py`, `docs/validation/F0-0003-J16-matrix.json` y `docs/validation/F0-0003-J16-mutations.json` regenerados. Este documento es la propuesta. Ningún otro `src/`, test, script o ADR se edita por B1; en particular queda intacto el diff ajeno de `docs/adr/F0-0002-second-slice.md`. Si el cierre de la ruta terminal no basta, C vuelve a propuesta antes de modificar otro mecanismo.

Orden tras validación: aplicar §2; añadir §4; ejecutar la matriz §3 y las inversiones §5; pasar `sh scripts/validate.sh` en clon limpio (fmt, clippy `-D warnings`, tests debug/release, RD), diferencial contra `50b78cf` en programas declarados sin cambio, `check`/`run` debug/release con diagnósticos completos, fuzz y medidor de programas objetivo. Registrar las métricas reales y entregar a D sin declarar aprobación de fase ni de corte por anticipado.
