# F0-0002 I08 — notas de implementación

## Alcance y autoridad

Este corte aplica F0-0002 §§4.4, 7, 9, 11.7, 12.4, 13.15, 14.1–14.3 y 17.1–17.2 sobre la superficie oficial F0-D15. Las filas de resultado de §11.7 se comprueban por resultado; su contabilidad exacta permanece en I10b. Las puertas de F0 y la validación independiente siguen pendientes.

## Invariantes de tipos y errores

`InferTy::Tainted` es el tipo absorbente de una expresión fallida: una restricción que necesita ese tipo no obtiene evidencia y no emite una segunda causa. `Never` expresa ausencia de continuación y `Deferred` una variable monomórfica pendiente; ninguno equivale a error. El checker sigue visitando los hermanos independientes en orden fuente, incluidos ambos argumentos de `TryAdd`. No se añade estado de contaminación lateral a la representación de `Result`. Los testigos de tipo proceden del sitio que introdujo la restricción y no de asignaciones posteriores.

`Result<T,E>` es tipo completo: copia e igualdad requieren que **ambos** componentes sean copiables, aun si la variante activa es conocida. `Ok` y `Err` usan el tipo esperado para fijar el componente opuesto; sin él se conserva NX-E0113 con `unresolved_components`. `?` comprueba en orden: operando `Result`, resultado de función `Result`, igualdad de E. Los diagnósticos dependientes de un operando fallido se suprimen; los de argumentos hermanos se conservan.

Un payload fallido de `Ok`/`Err` deja el constructor con `Type::Tainted`, aunque el contexto escriba `Result<T,E>`: ese contexto no legitima una consecuencia del valor fallido. Las regresiones comprueban el error del payload, una sentencia hermana independiente y la ausencia de NX-E0113 dependiente cuando `AssertEq` rechaza un recurso dentro de `Ok`.

## IR, propiedad y ejecución

`Ty::Result` conserva T/E. `Op::BuildResult` consume un payload tipado y crea un wrapper; `Op::BranchResult` consume el wrapper y entrega T en `Ok` o retorna `Err(E)` por el canal ordinario de la función en `Err`. El último es una selección fusionada en una instrucción IR: el verificador exige el tipo `Result<T,E>`, la coincidencia de E con la función y el uso único del temporal. El intérprete abandona el frame por la misma ruta de retorno y limpieza que `return`, no por un fallo R. Este detalle de representación no altera la selección semántica de F0-0002 §9.1.

La precisión de F0-0002 §9.1 y su registro en §20 permiten esta codificación de `BuildOk`/`BuildErr` y la fusión de `BranchResult`, manteniendo las obligaciones de §§7.1 y 9.2. Es una enmienda de representación para aprobación independiente de arquitectura, sin ampliar la sintaxis. Las dos líneas de texto y la línea en blanco existentes en §9.1 y §20 permanecen intactas hasta esa decisión.

El valor runtime almacena variante y payload propietario. `cells` cuenta wrapper más payload. La destrucción descuenta el valor completo y visita solo la variante activa; un recurso en el payload conserva su reserva de cierre hasta el último propietario. Las llamadas con `view` mantienen el propietario en el frame llamador; `move` lo transfiere. `TryAdd` produce `Ok(suma)` o `Err(30)`; `+` mantiene R0101. Una `Err` final de Main no genera diagnóstico y sale con código 1.

La reserva de trabajo y de la celda del wrapper ocurre antes de consumir el temporal de `BuildResult`; el tick de `BranchResult` también precede a su consumo. Un fallo de cuota conserva el payload en el conjunto de temporales que limpia el frame.

Una familia de arrays anidados con `Result<I64,I64>` fija la frontera de `live_value_cells`: un wrapper adicional cambia aceptación por R0002. El primario de la unidad final sigue como obligación de emisión de I10b; la regresión de I08 comprueba resultado, código y datos de cuota, sin atribuir a este corte el span normativo posterior.

La misma familia verifica que `TryAdd` reserva las celdas de su `Result` una sola vez y que `?` libera la celda del wrapper entre iteraciones, sin cambiar el resultado normal por un fallo de cuota anticipado.

## Límite de la proyección por mutación

La población y las ejecuciones dirigidas están en el estado generado y en `docs/validation/i08-mutations/`. Los supervivientes se mantienen visibles por ID, sin atribuirles equivalencia general:

- M00001, M00013 y M00014 modifican la señal interna heredada de contaminación. Los casos I08 comprueban que un fallo de payload absorbe sus consecuencias y conserva hermanos independientes; no se observó cambio de aceptación ni del primer diagnóstico normativo. Una revisión independiente debe buscar un contraejemplo razonable de Result/`?` antes de descartarlos.
- M00020 y M00022 eliminan el descuento de celdas después de que `Main` ya haya entregado `Ok` o `Err`. Ese contador local no forma parte de `Execution` ni se consulta después de construir el resultado.
- M00025, M00026 y M00033 cambian `break` por `continue` después de fijar un fallo de cuota. `result_array_quota_uses_expanded_source_and_releases_payload` mata M00025 con la fuente B completa de 32768 `Ok(1)`: fuente de 196677 bytes, R0002 en `[196660,196665)`, `live_value_cells`, sin salida posterior y eventos `open:0:1`, `close:0:1`. M00026 y M00033 sobreviven a la población cerrada; el detalle de contabilidad después del fallo sigue en I10b.
- `result_build_and_branch_transfer_temporaries_once` contiene los programas k5 y k4 para la transferencia BuildResult/BranchResult: k5 verifica `open:0:1`, `close:0:1`; k4 no produce eventos. Ambos aceptan y terminan con resultado de unidad, sin diagnósticos. En la población regenerada, M00038 invierte BranchResult y muere con `result_build_and_branch_transfer_temporaries_once`. M00037 invierte BuildResult y la ejecución completa lo detecta primero con la nueva regresión `question_mark_accepts_never_operand_when_all_if_branches_return`. M00036 muere con `e0202_after_while_pins_current_behavior`.

Los mutantes que alteraban la supresión de hermanos o dependientes, la reserva doble, el tipo de IR, los préstamos CFG y el hash canónico tienen ahora regresiones dirigidas. El estado generado registra sus veredictos instrumentales, no un veredicto de fase.

M00017 agota 60 s en `i08` tanto en release como en debug; el arnés lo clasifica `detectada` porque las ejecuciones base terminaron en 1.57 s y 4.65 s, respectivamente. Se conserva literalmente esa clasificación y el vencimiento medido en `results.jsonl` y en el estado generado.

## Entrada y límite temporal

`check` admite módulos de biblioteca. Tras parseo, nombres, tipos, ownership y lowering/verificación estática, `run` exige `Main` sin parámetros y resultado `()` o `Result<(),I64>`. `check_ast` (`src/check.rs:72–78`) ya no ejecuta la puerta y conserva `Resolved` en `CompilationCheck`; la ruta real la aplica una vez en `compile_scalar_with_entry` después de lowering. `check_proposed(run_entry)` conserva la segunda ordenación de prueba inmediatamente tras tipos mediante el wrapper común. La ausencia usa NX-E0112/missing_main en EOF y la firma inválida NX-E0112/invalid_main_signature sobre `Main`, relacionada con la firma escrita. F0-D15 y F0-CODIGOS conservan la fila I08 como histórica. `implementation_limit` ya no se emite para Result, constructores, `?` ni TryAdd; el ejemplo vigente de explain es `with no_io` (I09).

`tests/i08.rs::b_order_run_matches_check_after_lowering_error` fija la prioridad: el repro de ownership da NX-E0201 `[60,61)` en check y run, y `fn f()->I64 { } fn main(){}` da NX-E1317 `[14,15)` en ambos. Los controles de entry mantienen el caso vacío, firma inválida y `fn Main(){}`.

## Límites y revisión

Las variantes anidadas, `?` en argumento/selector y bloques de selección siguen las reglas generales de evaluación única y retorno de §7.1; las pruebas de comportamiento se alojan con I08. La IR verificada usa una operación BranchResult fusionada en vez de dos bloques CFG materiales; una revisión independiente debe comprobar que conserva el plan de limpieza, todos los orígenes y las cuotas aplicables. La contabilidad completa de trabajo y los perfiles reducidos quedan en I10a/I10b. Cualquier `adr_gap` adicional detectado en la campaña se registrará aquí con su comportamiento y regresión, sin atribuir al ADR una regla que no escribe.
