# Mutación del oráculo, segunda corrección

`python -B -m scripts.f5_oracle.mutate F5-O1` … `F5-O6` ejecutó 48
inversiones aisladas: **48 muertas, 0 supervivientes**. Cada parche exige una
coincidencia textual única, se aplica en una copia privada en C: y el script
comprueba el SHA-256 del archivo original al salir. O6 mata específicamente
orden índice/RHS, orden lo/hi, igualdad sin longitud, longitud de listas del
comparador, exit de fallo y tres inversiones de proyección.

El informe de D describió 24 variantes, pero no entregó los parches exactos.
`F5-D-described` reconstruye 24 cambios de esas clases: **20 muertos y cuatro
no observables en el subconjunto comprobado**. Los primeros 20 reutilizan
inversiones ya contadas en O1/O2/O3/O6; por ello no se suman como 20 mutantes
distintos. No se atribuye identidad byte a byte con la campaña de D. Su M24
original no se aplicó por falta de coincidencia textual, y no se cuenta como
mutante ejecutado de D.

- M08, JoinArrays sin copia adicional: `self.eval` ya devuelve una copia de
  los argumentos leídos; quitar la segunda copia no cambia los valores
  observables de los programas del dominio. Un ensayo de identidad que supere
  esa copia previa sería necesario para distinguirlo.
- M13, crecimiento de capacidad +1 en vez de duplicación: la capacidad no
  aparece en stdout, y las fuentes del modelo no activan cuotas de Array.
  F5 §9.4 sí distingue ambas políticas con storage-96; queda cubierto por
  `check_charge_literals` y la receta de cuota, no por esta mutación de VM.
- M21, no cortar explícitamente el inclusivo al llegar a `hi`: Python usa
  enteros sin límite, incrementa el cursor y el siguiente test termina el
  bucle sin otra iteración visible. Incluso con MAX no hay efecto público
  en este modelo; el ADR exige evitar overflow en la implementación I64.
- M22, no comprobar en la VM un literal fuera de I64: `SourceChecker` lo
  rechaza antes de ejecutar. Los negativos dirigidos siguen exigiendo
  NX-E0107; la rama mutada de VM es inalcanzable por la CLI normal.

Las cuatro supervivencias no acreditan corrección de la regla mutada en una
implementación de Nexo: son equivalencia o falta de observación del arnés en
esta superficie. M07, M10, M16, M17 y M23 sí tienen repro discriminante y
mueren en O6. Las respuestas esperadas vienen de F5 §§2.1, 3.1, 3.3, 4.1 y
9.3, no de `nexo.exe`.
