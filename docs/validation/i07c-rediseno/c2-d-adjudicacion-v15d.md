# Adjudicación de las 381 sondas D de B r22 (v15d)

Se ejecutaron de nuevo las 381 fuentes con la referencia C2 corregida en
`f0-0003-b-v1`. `c2-d-adjudicacion-v15d.json` contiene cada fuente completa,
SHA-256, firma de main, oráculo ADR y candidato, clase resultante, familia,
cláusula y commit dueño (SHA-256 del manifiesto:
`d326df34fec0b6cb01985f295ba67357d30521f07802ef2e6b5e029ca440cdf3`).

| Resultado de las 381 | Filas | Regla aplicada |
| --- | ---: | --- |
| C2 coincide con oráculo | 6 | Origen/lectura por componente, §4.1.2.1 y §4.1.2.6 |
| C2 conserva main | 1 | Relación de testigo aún pendiente de JoinRecord, §4.1.2.5–6 |
| D: campos de una firma con mismos códigos y primarios | 40 | JoinRecord y origen de primera continuación, §4.1.2.5–6; C3 |
| D: E0103 de constructor difiere | 54 | ConstructorDuty y cierre de deber, §4.1.2.1–2 y §13.9; C4 |
| D: restante, en `if` anidado | 280 | ResultStatus/JoinRecord y propagación de rechazo, §4.1.2.4–6; C3 |

La asignación de D compara las firmas completas del oráculo y del candidato:
primero discrepancias de E0103 por código y primario, luego diferencias solo
de campos con la misma secuencia de códigos y primarios, y finalmente las
reservas/propagación de error de `if`. Cada una de las 374 D restantes
contiene `if`; ninguna relación `expected_type` del candidato apunta a un
span posterior a su propio primario. La tabla es la adjudicación de estas
fuentes, no una aprobación anticipada de C3/C4: sus gates volverán a
comprobar las firmas completas.

La repetición de estas 381 fuentes con el binario release produjo las mismas
firmas que debug en 381/381; las 381 firmas de main (binario propio SHA
`422f68f956608cad7d92b13d975e803c7920456408a0a6192d14efedc76543ac`)
y del oráculo también se reprodujeron sin discrepancias.

La única `not_indexable` que el candidato añade frente a *ambos* main y
oráculo está en la fila 260 (SHA de fuente
`bea739c56e20a1b1436835d332cc85bb25b43762221c43798b78eaad6f0dbe88`,
primario `[237,241)`). Aparece dentro de la continuación de `if s then ...`
cuya condición `s` ya falla en `[230,231)`; la supresión corresponde al
`ResultStatus` contaminado de C3, §4.1.2.4–6. Es distinta de la sonda corta
de B: allí la proyección anterior recibía forma de una igualdad posterior;
esa `not_indexable` ya se eliminó en C2.
