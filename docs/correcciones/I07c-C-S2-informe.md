# I07c S2 — informe de implementación

Base de código: `e479d62` (S1). El trabajo permanece sin commit ni push, por instrucción del coordinador. S2 modifica solo `src/check/types.rs` y añade `tests/i07c_pattern_deferred.rs`; `src/ir/lower.rs` no requiere cambio.

## Sonda previa y mecanismo

La sonda en copia desechable confirmó la familia y los oráculos del plan antes de editar el árbol de entrega:

- B1a: la ocurrencia del array `[21,35)` se exportaba `Tainted → Unresolved`, aun cuando `(r==r)==true` `[22,34)` se exportaba `Concrete(Bool)` y el binding `r` terminaba como `Array<Bool>`. El valor vivo llegaba a IR sin tipo cerrado. B1b/c tenían la misma frontera de disponibilidad nominal; no hubo una nueva precondición de lowering.
- B3: `pattern_type` enlazaba `v` como `Array(Deferred(4))` en el mismo grafo; la anotación de su uso cerraba el componente a `I64`. La expresión `[]` se exportaba `Array<I64>`, pero el binding `v` seguía `Unresolved` porque la exportación de un compuesto sin variable raíz usaba `syntax_type` sin consultar el grafo.
- En b2g, `[DV]` daba `bottom_only=true` y disponibilidad nominal inicial; su gemelo con `[]` daba `bottom_only=false` y disponibilidad inicial falsa. Ambos rechazaban en el sitio previsto por sus fuentes. La diferencia es un estado del componente, no otra familia de obligaciones ni un cambio de oráculo.

`pending_nominal_available` consulta el dominio y el constructor del nodo del grafo: un dominio ya refutado o un constructor fallido no da testigo. Conserva la disponibilidad previa si el componente está cerrado o es solo fondo. Si queda un componente Array por inferir, permite el resultado nominal cuando un binding del mismo componente puede recibir restricciones posteriores; las obligaciones de dominio y constructor siguen activas. Esto permite que el RHS autorreferente fije `r:Array<Bool>` y evita usar como testigo un constructor desnudo e irresuelto de `[]==[]`. La primera versión demasiado amplia añadió un diagnóstico derivado de índice a una regresión causal de S1; se corrigió la condición sin cambiar el esperado y la regresión volvió a pasar.

La exportación de bindings resuelve ahora `known.ty` mediante `resolved_syntax_type`, que consulta los componentes del grafo incluso bajo Array/Option/Result. El local de patrón conserva sus identificadores `Deferred` dentro del tipo compuesto, por lo que exporta `Array<I64>` una vez cerrado. No se fabrica un tipo en IR ni se añade una ruta paralela de inferencia.

## Repros y regresiones

Esperados tomados de F0-0002 §§4.1.1–4.1.2.3, §§13.15–13.17 y §14.10; los binarios base se usaron solo para el diferencial.

| Fuente | Antes S1 | Después S2, debug y release |
|---|---|---|
| b1a=ir1, b1b, b1c | `internal_error` | `check` acepta. b1a/b1c: `run=unit`; b1b: `run=simulator_fault/R0102` en `[74,77)` por indexar el array vacío, sin error interno. |
| b1a-false y b1a sin asignación posterior | `internal_error` / NX-E0103 respectivamente | `check` acepta y `run=unit`, sin diagnósticos. |
| ir1c, ir1d, ir1_t | aceptaban | aceptan sin diagnósticos; `run=unit`. |
| `var r=[];r=[r];` | NX-E0103 | único NX-E0103, `types.infer`, `[16,18)` (`[]`), datos y relaciones vacíos. |
| `var r=[];r=[(r==r)==true];r=[1];` | `internal_error` | único NX-E0102/type_mismatch en `[39,40)` (`1`), Bool/I64, `expected_type=[21,35)`, `found_type=[39,40)`. |
| `var r=[true];r=[1];` | mismo C | único NX-E0102/type_mismatch en `[26,27)` (`1`), Bool/I64, origen `[16,22)`. |
| B3 `match Some([])` con `v:Array<I64>` | `internal_error` | `check` acepta y `run=unit`; también el caso con guarda y el binding `let x=Some([])` previo. |
| `Some([1])` y B3 sin fijar el elemento | aceptaba / NX-E0103 | el positivo acepta; el negativo conserva NX-E0103 sobre `[]` `[21,23)`. |

`tests/i07c_pattern_deferred.rs` comprueba positivos en `check` y compilación ejecutable, resultado y ausencia de observaciones; para los negativos exige exactamente un C con código, subcausa, etapa, operación, regla, explicación, primario, token, datos, relaciones y los demás campos vacíos/nulos. Incluye la matriz dirigida de Option y patrones sin pánicos ni `InvalidIr`. El fuzz dirigido de S1 (semillas 17021, 17022 y 10495373245766874321) se volvió a ejecutar por la puerta. Los repros B2 de S1 (b2a–e, b2g, q4w y los pares circulares) conservaron código, subcausa y primario en ambos perfiles y modos; B3 conflictivo siguió rechazando en `[93,94)`.

Como comprobación adicional del mismo enlace compuesto, `match Ok([])` con `Ok(v)` anotado `Array<I64>` y `Err(e)` anotado `Bool`, `Some(Some([]))` con patrón anidado y `Some([[]])` con patrón `Array<Array<I64>>` dieron `checked`/`unit` sin diagnósticos en release.

## Mutaciones inversas

SHA-256 del blob S1 `git show HEAD:src/check/types.rs`: `354cb74fae1547bcd9d51e1ce4a1f24a66162e05a1135bf818b74d8e52de749d`. SHA-256 de `types.rs` corregido: `4fe96534c6f25cc84d77d0be96d9c08c33867f9d49dd68f4790cfd207e99056a`; de la regresión nueva: `20d25f9df0fcb09740b635e466193c80a8152609c9f97bd310536c185161d118`.

| Inversión aislada en copia desechable | SHA-256 mutante | Prueba que falla con mutante y pasa restaurada |
|---|---|---|
| Invertir el enlace del binding al componente pendiente en `pending_nominal_available` | `0ea4afeb512cf195b7e24b97364e038e6e12cc05f9b6e3043b1e54eecca54726` | `self_read_assignment_exports_nominal_result`: `InvalidIr("unresolved checker array type")`. |
| Volver a `syntax_type` sin resolver el grafo al exportar el local compuesto | `7b6c56809401f3578ffb81086720135580999172a85294bbec89afde01032b73` | `pattern_local_exports_closed_array_element`: `InvalidIr("binding value flow")`. |

Después de cada inversión se restauró el SHA-256 corregido y pasó la misma prueba dirigida. La primera restauración requirió actualizar la marca de tiempo del archivo en la copia para que Cargo no reutilizara el ejecutable mutante; el hash ya coincidía antes de recompilar.

## Puerta y diferencial

`sh scripts/validate.sh` en copia limpia de `HEAD` más los dos archivos de S2: salida 0 con `CARGO_TARGET_DIR` dedicado, `TEMP/TMP` en el volumen de trabajo, Cargo `--locked --offline`, fmt, Clippy `-D warnings`, tests debug y release. `tests/i07c_pattern_deferred.rs` 3/3, `tests/i07c_causal_obligations.rs` 60/60, `tests/f0007.rs` 21/21 en cada perfil; `j14_accounting.rs` 9/9, T01–T14 y las regresiones de ticks 2/6/1 y 56/25/15 incluidas. `scripts/i07c_rd_conformidad.py --verify` dio 152/152 pares `check`/`run` y cero diferencias de instantánea tanto en debug como en release. `scripts/objetivo.py`: 15/29, ejecutan 01–15. `git diff --check`: salida 0.

El diferencial congeló 203 fuentes de `corpus/pass`, `corpus/fail`, `tests/fixtures` y objetivos 01–15 (SHA-256 del inventario de ruta y contenido: `58f692237df5071774a757deb14560b6317b8965365c0307288e51219ff02c24`). En release, `check` y `run` dieron **0/406 diferencias** contra S1 (`e479d62`) y **0/406** contra `e6e5f82` para esos programas declarados sin cambio. Las diferencias dirigidas frente a S1 son solo B1/B3 y los positivos nuevos de §8.3 enumerados arriba; frente a `e6e5f82` se añaden los C de B2 ya documentados en S1. No cambió ninguna salida fijada de RD/TD/DG ni los repros S1.

## Pendiente

S3 conserva el trabajo del solver iterativo y las pruebas de profundidad de pila; S2 no lo adelanta. La campaña final masiva de fuzz y mutación del corte sigue asignada al cierre posterior a S3. El progreso objetivo permanece en 15/29 porque estos repros corrigen inferencia y exportación, no incorporan un programa nuevo de `corpus/objetivo/`.
