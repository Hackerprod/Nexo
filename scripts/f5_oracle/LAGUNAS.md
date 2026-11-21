# Límites normativos del oráculo CLI F5

- F5 §§7.2, 9.5 y 10 fijan ticks y bytes lógicos de búfer, pero una ejecución
  exitosa de `nexo.exe` no publica `work_consumed`, bytes vivos ni reservas de
  Array. El diferencial CLI compara únicamente `outcome`, exit,
  `observation_bytes`, `reserved_observation_bytes`, eventos, salida y
  diagnósticos. Los cargos internos se acreditan por los arneses Rust del corte.
- F5 §8.2 fija código, etapa, operación y `pending_scope` de L08. Fuera de
  fuentes y spans literales como F5-H01 (`1*2`, `[16,19)`) y F5-H10 (`const`,
  `[0,5)`), no fija un primario único para este oráculo; se registra el campo
  observado sin usarlo para decidir el diferencial.
- Las combinaciones de errores de distintas etapas o sin precedencia total
  publicada en F5 se excluyen del generador de negativos. Cada negativo
  generado debe romper una sola regla y derivar su esperado del ADR.
- F5 §8.2 ordena barreras simultáneas por etapa, offset y número L, pero no
  fija cuántos NX-E0006/L07 se emiten si hay dos llamadas pendientes a
  `Push` o `Pop`. El arnés marca `diagnostics_cardinality=unspecified`: exige
  al menos un diagnóstico y que todos los emitidos tengan el código, etapa,
  operación y `pending_scope` L07 del ADR, sin usar el número de repeticiones
  observado en `d86cd26` como regla normativa. Las listas con cardinalidad
  publicada conservan comparación estricta de longitud y orden.
- Dos declaraciones escalares `const` pendientes de L09 presentan la misma
  laguna de cardinalidad: §8.2 fija un diagnóstico para su ejemplo de una
  sola declaración, pero no el número para dos declaraciones. El arnés exige
  al menos uno y verifica cada diagnóstico L09; mantiene el ciclo const
  `F5-K03` como negativo de núcleo cuando L09 esté completa.
- Antes de completar L04b, F5-r1 §«Alcance, clasificación y secuencia» y
  «Puertas por bloque» conserva el diagnóstico NX-E1317/ownership/flow.return
  para `fn F()->I64{while true{return 5;}}fn Main(){}` y sus pares. El
  diferencial sigue esa transición; no inventa L04 pendiente en G1b.

## F5-S08: biblioteca sin entrada

Laguna F5-S08, decisión del coordinador, revertible; pendiente de precisión en
el ADR (Flujo A). F5 §1.1 conserva D15 salvo enmienda expresa; F5 §12.2,
líneas 1847 y 1858–1862, manda reparar solo el tipo de `fn F(x:view){}` y
afirma `unit` y cero salida para los positivos. El positivo
`fn F(x:view I64){}` tiene 18 bytes ASCII sin LF y carece de `Main`. D15 §7.2,
línea 1282 (también línea 836), acepta una biblioteca sin `Main` en `check` y
exige `NX-E0112/missing_main` en `run`. Se acredita el positivo S08 solo con
`check`: outcome `checked`, perfil conservado, cero diagnósticos y cero salida.
En `run` se espera un único `NX-E0112/missing_main`, `entry/entry.check`,
primario `[18,18)`, `data={"entry":"Main"}`, outcome `rejected` y exit 1.
La edición de fuente normativa se conserva. Revisión de los otros 17 pares
F5-S01–S07 y S09–S18: todos contienen `fn Main`; ninguno comparte la laguna.
