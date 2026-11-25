# I07c R6 r2 — dictamen de puerta dirigida

Estado: **FALLA; no se entrega a B como contrato validado ni a C para implementar**. La conclusión es sobre el **prototipo parcial actual**, no sobre la imposibilidad de una representación R6 ni sobre el espacio de representaciones del brief. No se declara PASS.

## Evidencia reproducible

Base de la medición: `i07c-rediseno-proto` `da7da19`, binario release SHA-256 `1d65f3294cd4191cff6b253f3ef39fffbad55ebd295ff99720738a2a7c9d3b0f`, compilado con rustc 1.99.1 y `CARGO_BUILD_JOBS=4` bajo `heavy-lock`. Arnés: `scripts/i07c_rd_r6_probes.py`, perfil `f0-0002-b-v2`, `check --format json`, fuentes ASCII sin BOM ni LF final, flags `pendfa,retfa,nojoin,atomic0,e0103rule,retfail,nojoinr,skipv3,r5prefix,r1t5,r5t5`. Resultado: 22 coincidencias, **3 diferencias** y ST1 equivalente sin oráculo exacto. La matriz de 45 coincide 45/45 en código, subcausa y primario; su caso recursivo histórico espera el span de `main` que D2 sustituyó y no se usa como oráculo R6.

| Fuente | Esperado del contrato | Obtenido | Mecanismo que falta |
|---|---|---|---|
| `planned` 115 B | E0102 `[52,53)`, E1303 `[69,70)`, E0102 `[76,77)`, `[106,112)` | E0102 `[52,53)`, `[69,70)`, `[76,77)`, `[83,84)`, `[90,91)`, `[106,112)` | `ConsumerId` estable y `ReadContext` de entrada: el código usa `FactId` de visita y recalcula desde la forma planeada posterior. |
| SB 100 B | solo E0102 `[72,74)` | E0103 `[47,49)` y E0102 `[72,74)` | `ResultStatus` independiente de las igualdades válidas de las ramas; `r` queda irresuelto pese a su contraste interno compatible. |
| W2 169 B | E0103 `[77,79)` y E1303 `[140,145)` (adjudicación de A, pendiente de B) | E0102 `[85,86)` y E1303 `[140,145)` | La forma de `r[0]` completada en `a1` se distribuye retrospectivamente a la `if` de `a0`; §4.1.2.4 «Disponibilidad por nodo» lo impide. |

ST1 solo figura como fragmento de fuente en el informe de B; sus originales X-03..X-08 y las otras 14+8 fuentes tampoco quedaron en `b-repro/` ni en el informe final. El arnés distingue expresamente los equivalentes construidos por A. No se afirma que esos equivalentes sean las fuentes de B. Los originales son necesarios para la medición exacta solicitada.

## Alcance del prototipo

El cambio de código sustituye una copia global de `type_nodes` en el contraste `atomic0` por un journal de nodos; contiene además la distribución de contexto T5 y el primario recursivo de D2. `NodeJournal` no es aún `EqualityRecord`: no posee la totalidad de escrituras de una igualdad, ni `ReadContext`, `ConsumerId`, `ResultStatus`, `ConstructorDuty` y reservas de emisión. Conserva interruptores de R5. Por ello no cumple la condición de «solo representación nueva» ni demuestra la atomicidad general de Array/Option/Result con independencia de las rutas heredadas.

La puerta impuesta por el encargo exige todas las sondas dirigidas y las 45 filas correctas **antes** del barrido. Por las tres diferencias anteriores no se ejecutaron los 13 conjuntos, fuzz de B, 103 filas de `match`, aceptación, RD, pila, pasadas ni coste. Tampoco se ejecutó `validate.sh` ni se fundamenta PASS mecánico. El código de la rama desechable compila en release y `cargo fmt --check` terminó bien; el árbol está limpio. No se preparó plan para C.

## Decisión de este intento

Se detiene el intento parcial R6 en su puerta dirigida. Corregir cada salida mediante un filtro o una flag reproduciría el mecanismo que B rechazó en R5. La próxima implementación tendría que sustituir conjuntamente la identidad de visita, la planificación retrospectiva y la contaminación de resultados por los dominios y transiciones del contrato `I07c-rediseno-restauracion-atomica-R6-r2.md`, y repetir esta puerta desde un binario nuevo. La especificación sigue siendo **propuesta de A**; B decidirá su validez semántica, incluida la adjudicación W2. Este dictamen **no** afirma que cinco representaciones completas hayan agotado el espacio de diseño.
