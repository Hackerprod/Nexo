# F0-0002 I07c — Contrato de cierre del corte

Este documento fija qué significa «cerrado» para I07c. No modifica `docs/adr/F0-0002-second-slice.md`:
reparte entre cortes obligaciones que el ADR ya asigna a otro corte y delimita lo verificable de este.
Una desviación del ADR se resuelve en el ADR, no aquí.

## 1. Alcance

I07c cierra TD01–TD07 y DG07/DG08 (ADR §17.2, filas I07c): inferencia monomórfica con variables de tipo
relacionadas, `Never` separado de incógnita y de contaminación, aceptación independiente del orden del
solucionador, y origen real de `expected_type`. La tabla de aceptación enumera sus casos; «variantes
diferidas de ramas, arrays, igualdad y asignaciones» se interpreta como las formas sintácticas de
§4.1.1 que producen o consumen una variable de tipo, no como un conjunto abierto.

## 2. Fuera de alcance (asignado por el ADR a otro corte)

- Exactitud de los diagnósticos bajo perfiles de cuota reducida (spans, subcausa y testigos de NX-E0003
  más allá de «una sola NX-E0003 por ámbito con `observed = limit + 1` y el resto de diagnósticos como
  subconjunto exacto de la ejecución sin cuota»): I10a («I10a cierra perfiles reducidos local/global…
  contrato de spans de §13.13», §17 fila I10 y §17.2: «se cierra en I10»).
- Código que I07c no cambia respecto de `b506d7d` (rutas de I05, I06, I07a, I07b): un hueco de cobertura
  allí se registra en `Pending.md` como deuda del corte de origen; no bloquea I07c.

## 3. Criterios de cierre (todos obligatorios)

1. TD01–TD07, DG07/DG08 y los negativos y positivos literales de §14.10 conservan todos los campos
   protegidos por §4.1.2 N(g), en debug y release. En las filas RD, aceptación, código,
   subcausa y primario de cada C son normativos; relaciones y valores de tipo en `data` son
   instantánea I(b), incluso cuando la referencia determina un único valor.
2. `sh scripts/validate.sh` en verde sobre una copia limpia del árbol, con fmt y clippy `-D warnings`.
3. Fuzz estructurado con semillas fijas y registradas (≥ 400.000 en release y ≥ 100.000 en debug, `check`
   y `run` coherentes): 0 pánicos, 0 `InvalidIr`, 0 `internal_error`, 0 diagnósticos duplicados.
4. La aceptación no depende del orden del solucionador (permutaciones, §14.10) y el modelo
   ejecutable de la regla de contexto concreto concuerda con la implementación en una muestra de
   ≥ 100.000 programas al nivel normativo de §4.1.2: aceptación, código, subcausa y primario
   de cada C, conservación de sitios independientes, determinismo y cero errores internos. Se
   comprueban además los constructores pendientes de N(h), las relaciones reales de N(i) y los
   límites de emisión de D/V de N(j). Solo la presencia D/V permitida, la cardinalidad entre
   hojas D y el contenido de relaciones y tipos de `data` clasificado I(b) son instantánea
   revisable. Los C fuera de RD conservan los campos exigidos por §§13.15 y 13.17 según N(g).
5. Un solo punto determina el span primario de un operando (grupos transparentes) y un sitio produce como
   máximo un diagnóstico; ambas propiedades se prueban de forma estructural, no solo por muestreo.
6. Mutaciones sobre una POBLACIÓN CERRADA y reproducible de CÓDIGO DE PRODUCCIÓN: un script
   versionado (`scripts/i07c_mutaciones.py`) enumera determinísticamente sustituciones de
   comparaciones, conectores, constantes booleanas, registros, estado, operandos y límites en
   las líneas que I07c cambia respecto de `b506d7d`, en `src/check/` y `src/ir/`, excluyendo
   pruebas y arneses. Cada mutante se ejecuta contra la suite completa en copia desechable.
   Bloquea si cambia aceptación o un C normativo, pierde un sitio independiente, duplica un sitio,
   provoca pánico, `InvalidIr` o `internal_error`, o viola otra regla explícita del ADR.
   Entonces requiere una prueba que lo detecte o equivalencia verificable. Una diferencia solo
   en I(b) o en presencia D/V permitida por N(j), o bajo cuota reducida de I10a, se registra
   como informativa. Para un C fuera de RD, las relaciones y los tipos de `data` que fijan
   §§13.15 y 13.17 siguen siendo normativos; N(g) protege también todos los campos de los
   literales enumerados. La población y los resultados se versionan en
   `docs/validation/i07c-mutations/`; todo cambio de producción obliga a regenerarlos. Una
   revisión independiente reejecuta el script, coteja la población y una muestra. Una
   sustitución externa solo bloquea con un resultado observable que viole una regla explícita
   del ADR bajo el perfil normal.
7. Las cifras de pruebas, campañas y mutaciones se generan con un script (con la referencia base explícita
   `b506d7d`, no `HEAD`) y figuran en un único bloque de estado; los demás documentos remiten a él y no
   repiten cifras.
8. Instrumentos informativos, que NO son puerta de cierre: el arnés de gemelos (diferido frente a
   concreto), las campañas de cuota reducida más allá de la propiedad mínima (una sola NX-E0003 por
   ámbito con `observed = limit + 1`; el resto es de I10a) y cualquier huella hash de salidas. Las huellas
   hash de salidas no se versionan: fijan la implementación y no la especificación; los resultados que
   se quieran congelar se guardan como instantáneas legibles y revisables (la tabla RD, un corpus
   pequeño). Pueden ejecutarse y registrarse, pero una diferencia en ellos se informa, no bloquea.

## 4. Procedimiento

- Las mutaciones se aplican solo en una copia desechable del árbol con su propio directorio de destino,
  nunca en el árbol de trabajo. Quien valida copia el árbol, verifica el hash de cada fuente antes de
  empezar y mide sobre la copia.
- Los casos que el ADR no fija se registran como `adr_gap_*` con el resultado actual y la regla
  consultada; no bloquean.
- Una revisión que no termine en tiempo acotado entrega su informe parcial con la lista nominal de lo
  que no ejecutó.
