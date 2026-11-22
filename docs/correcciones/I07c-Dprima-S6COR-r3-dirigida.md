# D' revalidación DIRIGIDA sobre aa4da6835d73b432075e62fb34cddcfd99b3d9f1

**Veredicto: PASS** (el FAIL del motivo 1 sobre ddf551e queda resuelto; sin exploración fuera de lo pedido).

1. Caso `fn Main(){for x in ([]){}}` (f0-0003-b-v1, check y run): PASA. Candidato = 1 diagnóstico `NX-E0006` [20,22), igual en forma al control
   `for x in []{}` (1 diagnóstico, [19,21)). Base: 2 (E0006 [19,23) + E0103 [20,22)). La enmienda (docs/adr/F0-0002-lagunas.md, sección «Primario…», frase
   «El número de diagnósticos es el del mismo programa sin agrupación… deduplicación existente por sitio (`record_error`)») lo declara con ese ejemplo,
   y `(([]))` y trivia. La segunda sección (sitio de llamada) conserva el texto original. Con F5 el `for` es NX-E0006 sobre `for` y no cambia.
2. Filas nuevas de tests/i07c_group_primary.rs: PASA. Recalculé los offsets por bytes: `([])` [20,22); `(([]))` [21,23); `( [] )` [21,23);
   `(// c\n[])` [25,27); control [19,21); `([])()` [11,13); `(([]))()` [12,14); `(// c\n[])()` [16,18); F5 [11,13); control `[]()` [10,12). Los esperados
   son aritmética sobre el control y la enmienda, no del binario. Sin positivo pareado: aceptable, no existe programa válido con `[]` sin resolver (`let a=([]);`
   da NX-E0103) y el control rechazado por la misma regla hace de pareja; la relajación en el test está acotada a nombres `dedupe_*`.
3. Familia: PASA. Con mis corpus congelados `bd3eaecc…` (3.307) y `ed2d1670…` (4.500) y ejecutando base 1010938 contra aa4da68 (check y run, 3 perfiles, release;
   cand debug == release en todo): solo cambia recuento/códigos en 11 fuentes, todas `for x in G([])` en f0-0003-b-v1 (2 → 1). Búsqueda propia adicional
   de 4.992 programas (26 plantillas de posición x 16 expresiones, incluidas `[]`, `None`, `Ok(1)`, `Err(1)`, `Some(1)`, `[[]]`, `[None]`; grupos 1 a 3 y trivia;
   3 perfiles): solo cambian de recuento (a) el iterable de `for` y (b) el destino de llamada, ahora también con `(Ok(1))()`/`((Err(1)))(1)` (NX-E0113, no solo `[]`):
   base 2 → candidato 1. En todos el candidato iguala al control sin grupo (única excepción, `-@` con `1`: `-1` literal frente a `-(1)`, es semántica léxica
   preexistente y no recuento). Sin otras familias.
4. Mutación inversa D1 (quitar la deduplicación de `record_error`): PASA. SHA-256 original 54c9dfedb4c6909b… → mutado 5757c3ef385a8fef…, nextest --profile default: mueren
   `array_iterable_barrier_reports_the_inner_iterable` y `call_binding_diagnostics_report_the_inner_target_and_argument` (2 de 7); restauración verificada por SHA-256.
5. Nada más cambió: PASA. `git diff ddf551e aa4da68 --stat`: Pending.md, docs/adr/F0-0002-lagunas.md, docs/correcciones/I07c-S6COR-r3.md, docs/validation/i07c-s6cor/{mutate.py,
   mutation-results.json}, tests/i07c_group_primary.rs; `-- src` vacío. `gate.sh aa4da68… --verify`: GREEN; log y JSON coinciden: debug 2168/2168, release 2170/2170,
   RD 152/152. NOTA: no son 2176/2178: las filas nuevas amplían tests existentes y no añaden funciones `#[test]`; el recuento es el de ddf551e. SHA-256 de
   scripts/validate.sh = 7a1c7a19… = origin/main:scripts/validate.sh.

No bloquea: falta una fila para la variante NX-E0113 del destino (`(Ok(1))()`); misma causa y mecanismo que las filas añadidas. Los puntos A (R0001 con condición de `while`
agrupada, a decidir por el coordinador) y B (neutralidad del destino de asignación agrupado, preexistente) del informe anterior siguen abiertos y sin cambio.
Copias y targets borrados; sin procesos.
