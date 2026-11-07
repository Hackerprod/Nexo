# I10b — notas de implementación

Base local: `bdd0db2` (`i07c-robustez`). Estas notas registran el corte implementado y no aprueban una fase.

## Modelo de cargos

Cada `Instruction` y terminador (`Return`, `Jump`, `BranchBool`) contiene cargos de trabajo con ticks y span de origen. `BranchResult` es una operación de instrucción, no un terminador, y no hay un terminador `Fail`. `BasicBlock` no contiene cargos. El hash canónico incorpora los cargos de instrucciones y terminadores, incluidos sus spans relativos al origen de la función. Los prefijos sintácticos pendientes se adjuntan a la siguiente instrucción semántica o al terminador; las instrucciones administrativas de limpieza no reciben cargos.

La unidad implícita final conserva el origen en la última llave de cierre; su tick estático y la celda dinámica se cobran una sola vez. El tick de propagación `?` usa el span completo de `operando?` y se emite como cargo prefijo. `src/ir/work.rs` declara una sola regla por evento estático con unidades, política de origen, fase y orden; hoy `charge()` solo consume las unidades, y el origen, la fase y el orden son metadatos que leen los tests y el oráculo, no el cobro; `WorkPlanner` materializa el cargo. Se conservan 56/25/15, 2/6/1, `Print=10` y `AssertEq(1<2,true)=19`.

## Límites y verificación

Los presupuestos de trabajo, frames, celdas y observaciones usan una puerta común antes del efecto o reserva correspondiente. No existe un perfil de trabajo reducido. Las pruebas de fronteras usan la familia normativa de bucle acotado y bloques vacíos de relleno. `profile_id` permanece en nivel superior y R0002 conserva su conjunto cerrado de claves.

El verificador comprueba la misma representación ejecutada. Cada cargo presente debe tener ticks positivos y un span no vacío del archivo fuente, dentro de rango y alineado a UTF-8. `EndLoan`, `DropLocal`, `DropValue` y `DropBinding` no pueden llevar cargos; en las demás instrucciones se permite que la lista esté vacía, porque un cargo prefijo puede pertenecer a otra instrucción. Los cargos de Return, Jump y BranchBool se verifican de forma independiente en funciones de uno y varios bloques. El hash canónico vincula cada cargo y su origen; tras recalcularlo se rechaza la estructura inválida.

## Referencias del ADR

Los ticks y sus fronteras se fijan en §11.6, filas de la tabla de construcciones, y §11.7, reglas de límite y agotamiento. §11.8 (l.1207–1389) fija cuotas de análisis; la medición B-G1 está en l.1293 y su resumen de corte es la fila I10 de §17. Los campos y códigos de diagnóstico de ejecución están en §13 (l.1699, 1783–1791); las obligaciones de verificación de robustez, en §18 (l.3602). El registro de decisiones de §20 está en l.3644, 3656 y 3664. No se modificaron §9.1 ni las líneas preexistentes de §20.

## Verificación realizada

- `cargo test --offline`: suite debug, 1088 pasaron, 53 ignoradas, 0 fallos; RD 152/152.
- `cargo test --offline --release`: suite release, 1090 pasaron, 54 ignoradas, 0 fallos; RD 152/152.
- `cargo fmt --all -- --check` y `cargo clippy --offline --all-targets -- -D warnings` concluyeron sin errores.
- `sh scripts/validate.sh` concluyó en un clon limpio sin `.git` ni `target` inicial; se limpió el target y se eliminó la copia.
- Val43 detectó B1–B4. Se corrigieron los cargos de selectores `edit`, `ValueStmt` explícito e implícito, comparación de arrays de distinta longitud, orígenes R0001 de asignación y `return;`, además de G6. Las regresiones constan en `src/ir/i10b_tests.rs`.
- El oráculo independiente de `src/ir/work_oracle_tests.rs` genera árboles de fuente propios y registra spans al imprimir. Comprueba los 16 eventos alcanzables del catálogo 50 veces por campaña; `EmptyValue` permanece inalcanzable porque el parser rechaza su fuente antes del lowering. También verifica comparaciones de longitud y tag distintos en arrays y `Result`.
- `scripts/i10b_campanas.py` regeneró 100.000 fuentes debug (`0x008311061106c0de`) y 300.000 release (`0x008411061106c0de`); las semillas son nuevas y distintas. La regeneración posterior produjo el mismo SHA-256 byte a byte. El JSON registra semilla, contadores, cobertura, versión y hash de generador, sin tiempo ni rutas.
- `scripts/i10b_mutaciones.py` ejecutó las 90 mutaciones reales en serie dentro de una copia temporal. Resultado: 85 detectadas, cinco supervivientes equivalentes justificadas y cero supervivientes no equivalentes; los 15 mutantes bloqueantes murieron y cada sustitución restauró el SHA-256 de producción. Se conservan las seis justificaciones de equivalencia de la población; una de esas mutaciones también queda detectada por la integridad directa del catálogo.
- `scripts/i10b_poblacion.py` regenera el JSON canónico de población byte a byte. Estado, campañas y resultados de mutación se reproducen mediante sus generadores versionados.

El test de escala I07a detectó un recorrido cuadrático al clonar el span de sentencia durante cada búsqueda de token. Calcular el inicio de sentencia una vez eliminó ese coste; la misma prueba pasó en 4,45 s. Los totales de biblioteca, detalles de mutaciones y hashes de inventario están en el estado generado.

La validación diferencial mantiene 152/152 pares RD en debug y release. No se alteraron las cifras normativas I05–I10a. La revisión independiente y la aprobación de fase del propietario siguen pendientes.
