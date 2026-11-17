# J16, bloque 3 — propuesta r1: segunda llamada de CallOnce

Fecha: 2026-11-17. Propuesta del implementador C para revisión de D conforme al paso 4.1 del Flujo B. No autoriza aún cambios de código ni declara un veredicto. Base: `6d2f3dba223999cb835fea70d0a5dc3d0ae008c2`, perfil `f0-0003-b-v1`. Requisito de **núcleo**: disponibilidad de un receptor consumido y diagnóstico de propiedad. La tanda verde sin commit del bloque 3 y el diff ajeno de `docs/adr/F0-0002-second-slice.md` quedan fuera de esta propuesta.

## 1. Causa raíz comprobada y contrato

Fuente negativa de 72 bytes, sin LF:

```nexo
fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();f();}
```

F0-0003 §4.2 establece que `move f()` consume el entorno de la closure y que una llamada consumidora desarma `f`; §9.2, fila «Consumidor», exige NX-E0202 cuando se llama de nuevo. §9.1, C08, fija `NX-E0202/definitely_consumed` sobre el nombre de la segunda llamada, y C09 fija NX-E1107 para una llamada CallOnce **sin** `move` mientras el receptor aún está disponible. §3.3 solo establece prioridad causal para operaciones prohibidas dentro de guardas; no enuncia una prioridad general entre E0202 y E1107 en llamadas ordinarias. Aquí la precedencia se deduce del estado de propiedad exigido por §§4.2 y 9.2 y del contraste C08/C09: si el receptor ya fue consumido, el uso es la causa; si sigue disponible pero el modo solicitado es insuficiente, lo es el modo.

En `src/ir/lower.rs::Lower::expr` (brazo `Task::OtherCallEnter`, líneas 7436–7563 de la base), `resolved_expr` obtiene el tipo de closure sin consultar su disponibilidad (líneas 7447–7463). Luego la comparación `mode_rank(receiver_mode) < mode_rank(required)` emite NX-E1107 y ejecuta `continue` (7487–7513). Por ello nunca se llega a `named_closure_receiver` (4515–4523), donde `lower.check_available` detecta `consumed_at` y construye NX-E0202/definitely_consumed (3483–3505). C08 sí llega a ese control porque su closure vacía requiere `read` y no entra en el rechazo de modo. El problema es el orden de dos verificaciones existentes, no el análisis de CallOnce ni la representación de propiedad.

La base entrega en `check` y `run`, debug y release, salida 1 con un NX-E1107, subcausa `null`, etapa `ownership`, operación `ownership.check`, primario `[67,68)`, relación `declaration:[32,33)`, `data={"requested_mode":"read","required_mode":"consume"}`, sin cadena causal. El esperado del ADR es salida 1 con un NX-E0202/definitely_consumed, etapa `ownership`, operación `ownership.use`, primario `[67,68)`, relaciones `declaration:[32,33)` y `consumption:[58,66)`, `data={}`, y causa `consume/value.consume` en `[58,66)` para `binding:32`, evidencia `verified`; sin observaciones de ejecución. El prototipo aislado produjo exactamente estos campos en las cuatro combinaciones.

## 2. Corrección mínima, orden y archivos

Tras resolver `Ty::Closure { site, .. }` en `Task::OtherCallEnter` y antes de comparar los modos, si el destino es un binding nombrado, resolverlo mediante el `get` existente y llamar **una vez** a `check_available(local, target.span())`; propagar el mismo `CompileError` y detener esa llamada si no está disponible. Mover allí la llamada actual de `named_closure_receiver`, suprimiéndola en ese auxiliar para no duplicar la ruta. Si está disponible, continuar con la comprobación de modo y luego con `named_closure_receiver` tal como hoy. No inspeccionar el texto fuente ni fabricar diagnósticos nuevos. Los destinos no nombrados conservan su ruta actual. El estado de préstamos, permisos, transferencia y emisión de la llamada no cambia.

Archivo de producción adicional solicitado para esta corrección: **solo** `src/ir/lower.rs` en esos dos puntos. El plan J16 r5, bloque 3, §S6 permite `tests/j16_integrated.rs`, `scripts/j16_oracle.py` y la matriz `docs/validation/F0-0003-J16-matrix.json`; allí van las regresiones y la evidencia. El plan no permitía producción en S6 salvo repro y vuelta a revisión; esta ampliación concreta de `src/ir/lower.rs` depende de la aprobación del coordinador tras el veredicto de D sobre esta propuesta. No cambiar visibilidades, `src/ir/verify.rs` de producción, ADR, baseline, scripts de puerta ni otros módulos. La alternativa de alterar la inferencia de capacidad o insertar una excepción en NX-E1107 se descarta porque deja disponible el binding consumido en otros accesos o introduce rutas diagnósticas duplicadas.

## 3. Matriz de efectos laterales medida

Medición en `D:\nexo-work\tmp\j16b3-callonce-r1`, extraído con `git archive 6d2f3db`; binarios debug/release construidos offline con `CARGO_BUILD_JOBS=2` y `CARGO_TARGET_DIR=D:\nexo-work\tmp\j16b3-callonce-target`. Fuentes ASCII sin LF; los diez casos se ejecutaron antes y después como `check` y `run` en debug y release con `--profile f0-0003-b-v1 --format json`. Cada celda «antes»/«prototipo» resume las cuatro combinaciones, que coincidieron en código, subcausa, primario y cardinalidad. `[]` significa salida 0 sin diagnósticos. En `run`, los negativos no tuvieron observaciones.

| Caso | Fuente completa | Antes | Prototipo | Regla conservada o cambio |
|---|---|---|---|---|
| Consumidor, segundo `f()` | `fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();f();}` | 1 × E1107 `[67,68)` | 1 × E0202/definitely_consumed `[67,68)` | Cambio exigido por §9.2. |
| Consumidor, segundo `edit f()` | `fn Main(){let r=MockOpen(1);var f=fn [move r](){Drop(r);};move f();edit f();}` | 1 × E1107 `[72,73)` | 1 × E0202/definitely_consumed `[72,73)` | El receptor consumido tampoco admite `edit`. |
| Consumidor, segundo `move f()` | `fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();move f();}` | 1 × E0202/definitely_consumed `[72,73)` | Igual | Ya cruzaba `check_available`. |
| Sin `move` en la **primera** llamada | `fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};f();f();}` | 2 × E1107 `[58,59)`, `[62,63)` | Igual | Ambas llamadas tienen receptor disponible, modo `read` insuficiente; ninguna consume. |
| Positivo aislado | `fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();}` | `[]` | `[]` | `run`: `open:0:1`, `close:0:1`, unidad. |
| C08 | `fn Main(){let f=fn [](){};move f();f();}` | 1 × E0202/definitely_consumed `[35,36)` | Igual | §9.1; relación de consumo `[26,34)`. |
| C09 | `fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};f();}` | 1 × E1107 `[58,59)` | Igual | `read`/`consume`, sin consumo previo. |
| Read con recurso propio | `fn Main(){let r=MockOpen(1);let f=fn [move r](){Inspect(r);};f();f();}` | `[]` | `[]` | `run`: `open`, dos `inspect`, `close`, en ese orden. |
| Dos Read tras captura `edit` | `fn Main(){var x=1;var f=fn [edit x](){x=2;};f();f();}` | 2 × E1107 `[44,45)`, `[48,49)` | Igual | `read`/`inout`, sin consumo. |
| Dos llamadas `edit` | `fn Main(){var x=1;var f=fn [edit x](){x=2;};edit f();edit f();}` | `[]` | `[]` | La capacidad mutable reutilizable sigue disponible. |

En las 40 celdas medidas antes y después, 32 conservan diagnóstico, resultado y eventos completos; cambian las cuatro del consumidor con segundo `f()` y las cuatro del consumidor con segundo `edit f()`. La clase que puede cambiar es todo destino **nombrado ya consumido** cuya nueva llamada solicite un modo inferior al requerido: E1107 se reemplaza por el E0202 de disponibilidad con su subcausa real (`definitely_consumed` o `maybe_consumed`). No se debe generalizar el cambio a una llamada sin consumo previo ni a destinos anónimos. La comparación de diagnósticos completos normaliza solo la ruta del archivo temporal.

## 4. Regresiones públicas y pruebas

1. Añadir a `tests/j16_integrated.rs` una pareja literal para el consumidor: el negativo de §1 y el positivo que quita **solo** la segunda llamada. Fijar salida 1 frente a 0, cardinalidad 1 frente a 0 y diagnóstico exacto del negativo: `NX-E0202/definitely_consumed`, `ownership/ownership.use`, primario `[67,68)`, `data={}`, relaciones `declaration:[32,33)` y `consumption:[58,66)`, causa `consume/value.consume` `[58,66)` sobre `binding:32`. Comprobar `check`/`run` y debug/release, no únicamente `compile_scalar_with_profile`.
2. Añadir la variante sin `move` en la primera llamada como control de prioridad: dos NX-E1107, subcausa `null`, `ownership/ownership.check`, primarios `[58,59)` y `[62,63)`, `data={"requested_mode":"read","required_mode":"consume"}` en ambos, relación de declaración `[32,33)`, sin cadena. También C09, C08 y la segunda llamada `edit` de §3. El negativo principal **falla en la base y pasa con la corrección**; los controles fallan ante una corrección que suprima E1107 o deje de comprobar consumo.
3. Incorporar en `scripts/j16_oracle.py` las fuentes y los esperados literales derivados de F0-0003 §§4.2, 9.1 y 9.2, antes de invocar Nexo; actualizar la fila CallOnce de la matriz J16. En los positivos, comprobar resultado, orden de eventos y ausencia de observaciones impropias; en los negativos, ausencia de ejecución. Añadir fuzz acotado de secuencias `read`/`edit`/`move` sobre closure Read, mutable y CallOnce, con y sin consumo previo, sin pánicos ni cuelgues. No generar esperados desde el binario bajo prueba ni usar nombres de test basados en rondas.

## 5. Inversiones de la corrección

Aplicar cada mutación sola sobre `src/ir/lower.rs` en una copia, construir offline, correr su testigo y restaurar byte a byte antes de la siguiente, comprobando SHA-256. SHA-256 del blob base y restaurado en esta prueba de concepto: `987ede71bdb924b870dab0979017ff642081b3ad8453adf15440ae8746790f74`; SHA-256 del prototipo temporal: `c125ba3dcace6011777642ab3d14ba095acc751aabb6506ea117b46aa6f4c738`. La primera inversión equivale a la base y está medida; las otras son inversiones que C debe ejecutar al implementar, no resultados anticipados.

| Mutación inversa | Testigo discriminante | Resultado que la delata |
|---|---|---|
| M1: devolver `check_available` después de la comparación de modo | Consumidor con segundo `f()` de §1 | E1107 `[67,68)` en lugar de E0202; medido en la base. |
| M2: omitir por completo el control de disponibilidad del receptor nombrado | C08 y segundo `move f()` del consumidor | Aceptación incorrecta o error posterior en lugar de E0202. |
| M3: comprobar disponibilidad solo cuando el modo solicitado sea `consume` | Consumidor con segundo `f()` y con segundo `edit f()` | E1107 reaparece en los modos `read` e `inout`. |
| M4: omitir la comparación de modo tras adelantar disponibilidad | C09 y dos Read de closure `edit` | Aceptación indebida donde corresponde E1107. |

## 6. Puerta antes de entregar a D

Tras la validación de esta propuesta y la autorización de la ampliación de archivos, C implementa solo el mecanismo de §2, añade las regresiones de §4 y hace un commit de la solución en `i07c-robustez`. En clon limpio del nuevo commit: `sh scripts/validate.sh` (fmt, clippy `-D warnings`, tests debug/release y RD), diferencial contra `6d2f3db` por programa del §3 y corpus J16 declarado sin cambios, `check`/`run` debug/release con diagnóstico completo, oráculo independiente y medidor de programas objetivo ejecutables/total. La única diferencia admitida es la disponibilidad del binding consumido antes de juzgar un modo insuficiente; cualquier otra divergencia requiere volver a propuesta. Una sola pasada final de mutación de núcleo conforme a J16 y F0_ACUERDOS §10, con SHA inicial, mutado y restaurado; no se cuentan mutantes que no compilan. D revisa diff y fidelidad al plan, reproduce los negativos/positivos, comprueba fuzz y oráculos, y emite su propio veredicto. Esta propuesta no modifica código de producción ni afirma que esa puerta ya se haya ejecutado.
