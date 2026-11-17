# I07c §8.3 — verificación dirigida B, ronda 3

## Veredicto: PASS

- Bloqueante de r2 corregido: «Interinidad» limita la coincidencia con la decisión a válido, `b1a=ir1`, ir1c, ir1d, sin asignación posterior e `ir1_t` (los que la propuesta acepta). Para el negativo `r=[1]`, el gemelo `var r=[true];r=[1];` y `r=[r]` dice que el único veredicto válido es el rechazo de su fila (NX-E0102, NX-E0102, NX-E0103), nunca su aceptación.
- Sin otra frase con el mismo defecto: las demás menciones de aceptación (estado, tablas, variantes) ya distinguen aceptados y rechazados.
- Lo corregido en r2 sigue intacto: «fija», origen `expected_type=[21,35)` y gemelo `[26,27)`/`[16,22)`, sin NX-E0114, tabla y método «Igual».
- Registrable (no bloquea): la primera frase de «Interinidad» agrupa el negativo entre los programas con «solo N(f)» y la frase posterior lo matiza con rechazo obligatorio; conviene unificar la redacción al incorporar.

## Ubicación de inserción
`docs/adr/F0-0002-second-slice.md`, §4.1.2.3, después del párrafo «Cierre de dominio indeciso (precisión normativa, 2026-11-01)» y su párrafo de ejemplos, antes de la cláusula «4. Contexto y cardinalidad»: párrafo normativo y tabla «Ejemplos y diagnóstico exacto», en ese orden.
