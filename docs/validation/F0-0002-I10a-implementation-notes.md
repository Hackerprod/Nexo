# I10a: notas de implementación

## Alcance y decisiones

Este corte implementa las cuotas de análisis y las correcciones de escala de F0-0002 §§11.4, 11.8, 13.9 y 13.13. Se mantiene el rastreador vivo de lowering que define §11.8.B y se agrega un diferencial entre ese rastreador y el analizador disperso para funciones sin bucle. La ruta dispersa normaliza los mensajes de NX-E0105 al catálogo.

Se retiró el atajo disperso que cobraba una unidad por instrucción sin ejecutar el punto fijo. Los contadores cargan las unidades de análisis y las escalas de ownership y verify se miden por trabajo contado. `find_witness` hace BFS por distancia y desempata con (desconocido, total de causas = aristas + cadena local de la hoja, offsets, identidad); no consume `effect_fixpoint_steps`. El código de HEAD es normativo para esta entrega porque las cifras de referencia se midieron con esa clave; la redacción de ADR §13.15 sobre menor número de aristas queda anotada como laguna de redacción en Pending.md. La búsqueda se ejecuta una vez por restricción violada.

El límite de ownership se selecciona por scope: función o compilación. Los perfiles reducidos `f0-0002-b-*` se seleccionan por API Rust interna. `flow-global` exige scope de compilación y la vista del diagnóstico aplica el límite correspondiente al scope.

El límite de `ir_items` se informa después de resolver propiedad y efectos. Si no se pudo construir una función por el límite, no se analiza ni se contrata una función cuya clausura de llamadas la incluya. Así se conserva el orden de errores exigido por §13.9.

La profundidad interna de nombres ya no emite una cuota independiente; el worker checker informa un fallo interno controlado si no puede crearse. Un AST sintético con 512 grupos anidados se resuelve sin una cuota secundaria. Los límites de profundidad 127/128 se prueban con la expresión de F0-D15 §7.1, y los if anidados de nombres cubren aceptación y rechazo en el parser.

## Robustez de escala

`effects.rs` usa índices por BlockId para exits, búsquedas de bloques, posiciones de instrucciones y spans de returns. `verify.rs` conserva liveness en conjuntos persistentes, cuenta las operaciones de sus consumidores y evita comparar listas completas en cada arista. La regresión de dominancia incluye un temporal que se define en una rama y se consume en el bloque de unión.

La ruta de `src/check/types.rs` para proyecciones diferidas quedó fuera del corte. El test actual `tests/i07c_mutant_projection_scale.rs::resolved_projection_does_not_repartition_unrelated_pending_constraints` registra el comportamiento medido existente; el análisis y la corrección de esa escala quedan pendientes.

## Evidencia

La población y sus resultados por mutante están en `F0-0002-I10a-mutations.json` y `F0-0002-I10a-mutation-results.jsonl`. Los resultados de verificación ejecutados se conservan en `F0-0002-I10a-verification-results.json`; el estado generado concentra hashes, conteos, campañas y resultados: `F0-0002-I10a-state.json`. La campaña tipada se hizo con `scripts/i07c_typed_campaign/main.rs` en modo `twin`; su semilla, volumen, profundidad, reparto por hilo, casos señalados y diferencias adicionales están en `F0-0002-I10a-typed-campaign-results.json`.

El generador `twin` no encontró bugs tipados, pero las ejecuciones recientes señalaron 19 firmas distintas en release y 10 en debug, además de 8/3 diferencias no clasificadas. No se adjudican como resultado de I10a: el generador compara obligaciones diferidas de tipos del alcance I07c y queda registrado como límite de la muestra.

Inventario de bucles productivos adicional de `src/ir/lower.rs`: funciones y parámetros (`302–369`), entradas y canonicalización (`570–626`), errores y restauración de undo/args (`918–985`), ramas/estados (`1316–1373`), loans, undo y drops (`1601–1835`), índices, argumentos y préstamos (`2191–2457`), limpieza de scopes/locals (`2545–2797`), items e indexación (`3245–3330`) y el bucle de trabajo (`3516`). Los seis puntos de `count_cfg_work` están en `1603`, `1607`, `1709`, `2070`, `2094` y `2581`; el inventario incluye también los recorridos vecinos sin contador. Las pruebas de time/mutación cubren los recorridos de exits y su radio, no sustituyen esta contabilidad.

`sh scripts/validate.sh` se ejecutó en una copia limpia sin archivos temporales, metadatos de control de versiones ni `target`; la copia y su target se eliminaron al terminar. Los resultados de las suites, RD y clippy se consultan en el estado generado. La validación independiente del corte y la aprobación de fase del propietario siguen abiertas.

## Correcciones tras la revisión independiente

Se corrigió el cargo rechazado de ownership para conservar sin cambios ambos saldos local/global, incluido el rechazo simultáneo de ambos límites. `charge_effect` comprueba `>=` antes de sumar y tiene regresión de unidad para límite exacto y primer exceso. El fallo de creación del checker worker se conserva como error interno controlado y se propaga desde lowering al driver con salida 2, sin pánico.

El generador actual enumera 91 reescrituras únicas, incluidas las reposiciones cuadráticas de escaneos y contadores, la clave del testigo, el cargo denegado y el límite de efectos. La población y `docs/validation/F0-0002-I10a-mutation-results.jsonl` deben regenerarse y ejecutarse juntas; los resultados históricos no acreditan esta población. `docs/validation/F0-0002-I10a-state.json` informa si resultados y población coinciden.

## Cobertura recuperada

`lowering_worker_preserves_configured_stack_margin` quedó restituida como `src/ir/i10a_tests.rs::lowering_worker_preserves_configured_stack_margin`, con lowering desde una pila llamadora de 512 KiB. La prueba antigua `capability_limit_returns_before_lowering_an_uninferred_ast` ejercitaba el gate de `implementation_limit`, que permanece en `src/ir/lower.rs` antes de lowering. I10a no modifica esa ruta de límites; no se duplicó el caso con un selector de cuota de tipos reducido que no forma parte de los perfiles del ADR.

## Comportamiento corregido y límites medidos

El desempate del testigo compara el total de causas y conserva el BFS por distancia. El diferencial determinista compara al menos 2400 casos con violación, cinco semillas y hojas intrínsecas de cadena 1/2/3, hojas desconocidas y SCC; la reproducción de los grafos de las semillas cuenta 1969 ciclos entre 2500 casos. Cada testigo se compara con `find_witness_enumerated` en resumen, primario, operación, cadena y relaciones; el caso `Make`/`B` frente a `A` fija la prioridad del total. El worker checker propaga fallo como `internal_error`; `check_proposed` devuelve error si la API interna informa fallo de arranque. El driver emite `nexo-driver:internal_error` sin inspeccionar cadenas de causa. La prueba de exit/stderr del proceso queda limitada porque no hay hook productivo de spawn. El test de escala por tiempo usa tiempo release para `if b{return;}` con tamaños 3000 y 24000, tres ejecuciones por tamaño, umbral de razón 12; el test de campaña registra la mediana.

`find_witness` corre una vez por cada restricción violada. La referencia de escala de rombos usa cotas absolutas para tamaños 512 y 4000, no una comparación con HEAD. El inventario de bucles de Lower se revisó junto a effects y verify.

La campaña release de testigos midió la cadena 800 en 0.000775 s y los grafos sintéticos de rombos n=512/4000 en 0.001/0.005 s, ambos bajo la cota absoluta de 10 s. El control lineal de la razón de tiempo medido localmente queda por debajo del umbral de 12.


## Evidencia adicional de mutaciones y razón de tiempo

La población actual contiene 91 reescrituras. El barrido release produjo 82 mutaciones detectadas, siete supervivientes y dos no compilables. M00014 (reposición del escaneo de retornos con DropLocal) se reejecutó contra `exit_scan_work_scales_with_droppable_returns`, que verifica DropLocal y mide unidades `[24194, 48454, 97458]`; con factor 2.1 el mutante falla. M00088 (temporales de préstamo en la verificación) se reejecutó contra `verifier_work_scales_with_sparse_loan_temporaries`, con unidades base `[26248, 61264, 158944]` y umbral 2.65; el mutante falla. La prueba anterior de ese caso, antes de añadirlo, no generaba temporales de préstamo y dejaba sobrevivir el mutante.

La campaña de razón de tiempo permanece ignorada en release y exige tres muestras por tamaño; su umbral de 12 está calibrado entre el control lineal y las reposiciones cuadráticas de los escaneos por salida y por bloque. Una reposición marginal queda en el borde del umbral y se registra como riesgo residual de rendimiento; las regresiones de trabajo contado cubren la reposición del escaneo con DropLocal y la de temporales de préstamo.

`i10a_mutaciones.py` ahora escribe la población y los resultados en sus rutas I10a explícitas, evitando que resultados genéricos históricos oculten una diferencia de población. El estado generado debe recalcularse después de completar las reejecuciones y validar el clon limpio.

Umbral de la campaña de razón de tiempo: la prueba `verifier_effect_release_time_campaign_records_branch_ratios` (release, `#[ignore]`) compara el tiempo de la secuencia de retornos condicionales entre dos tamaños. El umbral está calibrado entre el control lineal medido, que incluye efectos de caché, y las reposiciones cuadráticas de los bucles sobre bloques y salidas, que lo superan con holgura. Una reposición marginal del escaneo de bloques por salida queda en el borde del umbral y se trata como riesgo residual registrado: sus unidades sí cuentan en los contadores de trabajo, que son el criterio normativo de escala.
