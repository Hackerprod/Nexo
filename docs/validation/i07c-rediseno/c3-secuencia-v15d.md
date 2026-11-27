# Secuencia ejecutable C3 → C4 (v15d)

Se ejecutó `owned_rows_gate.py --commit C3` sobre la referencia C2 corregida,
con main `efc801d2f76b8ae972d1133ab9118b67db5f4bfacaa4d0873fdd4c3dc021e40a`.
El gate original dio 27/9/9 filas y 15 fallos (7 dirigidas, 4 de matriz,
4 congeladas duplicadas). Las filas ya conformes en C2 no requieren mecanismo
adicional de C3. La tabla completa de las 45 filas, con SHA del fuente,
perfil, salida C2, esperado, mecanismo y dueño, está en
`c3-secuencia-v15d-filas.json` (SHA-256
`f2eab88c56d01d3880a11525bd28435aebcdf1536cd0ade6bacfb817096d4744`).
Treinta filas ya coinciden exactamente en C2; las otras quince corresponden
a ocho fallos de C3 (cuatro mecanismos repetidos en la batería congelada) y
siete filas que requieren C4.

| Fila | Familia | Evidencia C2 | Mecanismo mínimo | Dueño |
| --- | --- | --- | --- | --- |
| SB_equivalent | dirigida | falta E0103 [47,49) | ConstructorDuty reserva el constructor independiente | C4 |
| W2_no_retroactive_context | dirigida | falta E0103 [77,79); E0103 tardío [151,154) | ReadContext + ConstructorDuty en sitio de entrada | C4 |
| atomic_78 | dirigida | E0103 sobrante [16,18) | ConstructorDuty cancela deber inválido | C4 |
| F06 | matriz | E0103 prematuro [27,29), conflicto desplazado | ConstructorDuty + JoinRecord conserva y resuelve el deber | C4 |
| M2 | matriz | E0103 prematuro [30,32), conflicto desplazado | ConstructorDuty + JoinRecord conserva y resuelve el deber | C4 |
| W0 | matriz | E0103 sobrante [27,29) | ConstructorDuty cancela deber inválido | C4 |
| w1 | matriz | E0103 sobrante [27,29), más conflictos | ConstructorDuty + ResultStatus/JoinRecord | C4 |

Las filas `M2_partial_witness_chain`, `RS1_condition_contaminated`,
`W1_atomic_compound` y `W1a_without_later_assignment` (incluidas sus
repeticiones congeladas) siguen en C3: sus diferencias observadas son
respectivamente emisión de un conflicto adicional o contaminación del
resultado, sin E0103 de constructor que exija ConstructorDuty. El gate
corregido debe dar 24/5/9 y dejar solo estos ocho fallos en el árbol C2.
El informe íntegro de `owned_rows_gate.py --commit C3` con el binario C2 está
en `c3-secuencia-v15d-owned.json` (SHA de blob
`094c3d1050108c7e97971cdb9e88de1454ace03c1dd514100ed8d657f193801e`);
se conserva como prueba ejecutable de los ocho fallos que C3 debe resolver,
no como veredicto verde anticipado de C3.

`additional_named_probes` contiene once entradas adicionales, auditadas en
`c3-secuencia-v15d-named.json` (SHA de blob
`d66a799bce781667dc3f7410304123d79bdd3036e9a8f417e0a2c50e5a258f47`):
tres repiten filas propias anteriores; las siete fuentes M1/match restantes
conservan exactamente su esperado de main en C2 (7/7, salida completa en el
manifiesto). `match_88_byte_checked` es una puerta agregada de conservación
D5/D6, no una fuente adicional. Ninguna de las once exige introducir
ConstructorDuty en C3.
