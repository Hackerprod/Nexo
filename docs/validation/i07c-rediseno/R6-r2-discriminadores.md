# R6 r2 — discriminadores de atomicidad (sin PASS)

Fuente de esperados: `docs/correcciones/I07c-rediseno-restauracion-atomica-R6-r2.md` y el informe final de B ronda 2. Fuentes ASCII sin BOM ni LF final. La medición usa el binario **R5 experimental anterior a las ediciones locales de R6**, SHA-256 `9d48720efb6add5a488947e710e55201a5030b1dec5a874bb5ca58e4b47a60a4`, en `C:/nexo-work/target-r6/release/nexo.exe`; no es un prototipo R6 ni una puerta completa. Perfil `f0-0002-b-v2`, `check --format json`. El arnés versionado es `scripts/i07c_rd_r6_probes.py` en `i07c-rediseno-proto`; emite el detalle de subcausa, relaciones y `data`, aunque la comparación dirigida actual coteja código y primario.

| Fuente | Esperado derivado | R5 con `atomic0` | R5 sin `atomic0` |
|---|---|---|---|
| W1, 221 B | E0102 `[99,100)`, E1303 `[147,148)`, `[199,200)`, `[209,212)`, E0102 `[216,219)` | Coincide | Pierde E1303 `[209,212)` |
| W2, 169 B | E0103 `[77,79)`, E1303 `[140,145)`; pendiente de validación de B | E0102 `[85,86)` sustituye E0103 | Coincide |

La ablación individual muestra que copiar/restaurar `type_nodes` al entrar en `anchor_contrast` es demasiado grueso y que quitarla deja una escritura parcial. El criterio propuesto R6 es un journal **por igualdad compuesta**, con publicación o rechazo enteros, y contexto de entrada congelado por consumidor. Estas dos sondas son obligatorias antes del barrido; no autorizan declarar que R6 funciona.
