# I07c §8.3 — validación B, ronda 2 (conserva contexto)

Verificación de la corrección de `I07c-laguna-8-3.md` frente a `I07c-laguna-8-3-B-r1.md`. Binario base e6e5f82 en `D:\nexo-work\target-i07c-b2`.

## Veredicto: FAIL (1 bloqueante menor, solo en «Interinidad»)

## Estado de lo pedido en r1
- **B1 («puede» → «fija»): CORREGIDO.** El párrafo dice «ese resultado fija» y «operación válida»; no quedan modales ambiguos en el texto normativo.
- **B2 (origen de `expected_type` y ubicación): CORREGIDO.** El texto fija `[21,35)` (array completo) y lo argumenta desde §§4.1.2.6–7 y 13.15 (procedencia hasta la expresión que fija el tipo de la asignación; correspondencia con el inicializador del gemelo), y explica por qué RD29/RD89/RD93/RD106 usan el literal (allí fija directamente el elemento). No se apoya en el binario. La ubicación queda fijada: párrafo y tabla de ejemplos forman la enmienda.
- **R1 (sin NX-E0114): CORREGIDO** (no queda ningún código nuevo).
- **R2 (`r=[r]`): CORREGIDO.** Fila nueva NX-E0103 `[16,18)`, `types.infer`; el binario base da exactamente eso.
- **R3 (gemelo y método): CORREGIDO.** Gemelo `var r=[true];r=[1];` con primario `[26,27)` y `expected_type=[16,22)`; el binario base da lo mismo. Método de la tabla «Igual» descrito y reconocido como documental.
- **R4/R5: CORREGIDO** (la propuesta declara la discrepancia con la base; «RD79 está retirado»).

## Evidencia propia
- Offsets recalculados a mano: `fn Main(){`=[0,10); `var r=[];`=[10,19), `[]`=[16,18); RHS `[21,35)`, elemento `[22,34)`, `1`=[39,40). Gemelo: `var r=[true];`=[10,23), `[true]`=[16,22), `1`=[26,27). Correctos.
- Sin cambio en filas fijadas, en el binario base: RD70 expandido da NX-E0103 `[17,19)` y NX-E0102 `[125,127)` con R `[80,85)`, Array<I64>/Array<Array<I64>>, tal como el ADR. RD79 sigue retirado (§14.10: «su identificador no se reutiliza»; no hay fila). TD01/TD03 (NX-E0102, tipos y orígenes del ADR, offsets desplazados solo por la expansión de DV), TD04 (ambos órdenes), TD05a y TD06: aceptados/rechazados según el ADR. El único otro patrón de destino leído en su propio RHS es RD70 (tipos concretos).

## Motivo del FAIL (accionable)
1. **Interinidad incoherente con la decisión para el negativo.** Dice: «Si la implementación solo cumple N(f) aceptando estos programas durante la interinidad, esa aceptación coincide con la decisión propuesta». «Estos programas» incluye el negativo `fn Main(){var r=[];r=[(r==r)==true];r=[1];}`, cuya decisión es rechazo NX-E0102 sobre `1`; aceptarlo sería aceptar lo que una regla rechaza (defecto de núcleo). Reescribir: la aceptación en interinidad solo coincide con la decisión para los programas que la propuesta acepta (válido, `b1a=ir1`, ir1c, ir1d, ir1_t, sin asignación posterior); para el negativo, N(f) y, si se emite un resultado, solo el rechazo NX-E0102 de la fila (la segunda asignación ya está regida por §4.1.2.2 «Nunca se usa una asignación rechazada...»).

## Ubicación de inserción (para el PASS)
`docs/adr/F0-0002-second-slice.md`, §4.1.2.3, inmediatamente después del párrafo «Cierre de dominio indeciso (precisión normativa, 2026-11-01)» y de su párrafo de ejemplos («Con las abreviaturas de §14.10 ...»), y antes de la cláusula «4. Contexto y cardinalidad». Se insertan en ese orden el párrafo normativo y la tabla «Ejemplos y diagnóstico exacto».
