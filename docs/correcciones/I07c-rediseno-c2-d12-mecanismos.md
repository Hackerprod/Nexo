# C2 D12 — partición de los 62 defectos del sondeo de 5.000

Estado: investigación, sin PASS. Base: `c2-fuzz-5000-pd6.json` y
`c2-d12-census-5000.json` (43 mejoras, 38 defectos con oráculo completo,
24 diferencias sin oráculo completo). La partición usa el diagnóstico nuevo
frente al oráculo y la ruta observada en la traza JSONL. Los nombres designan
el mecanismo que debe verificarse al corregir; la mera coincidencia de códigos
no prueba una causa raíz común.

| Grupo | Mecanismo a comprobar | Filas | Representantes de traza |
| --- | --- | ---: | --- |
| M1 | Conservación de la lectura proyectada cuando el oráculo deja una obligación opcional o sin modelar; el aplazamiento de la primera proyección cruza un `if` y una asignación posterior. | 24 | 158: 4 `join_resolve`, 9 `literal_resolution_entry`; 283: 6 y 10. |
| M2 | Elección de testigo y relación al materializar una igualdad diferida como NX-E0102; hay al menos un diagnóstico candidato ausente del oráculo. | 18 | 306: `select` elige `[63,70)` mientras la relación discrepante sale de otro literal; 375: `if_join_context` identifica `[144,149)` y el oráculo fija el primer operando posterior `[166,169)`. |
| M3 | Cierre de constructor o `Join` pendiente como NX-E0103, con o sin NX-E1303 acoplada. | 7 | 830: 15 `join_resolve`, 15 `constructor_close`; 1088: 9 y 18. |
| M4 | Emisión de NX-E1303 de una unión tras la selección del primer sitio incompatible. | 5 | 1124: 10 `join_resolve`, 15 `if_join_context`; 1990: 4 y 6. |
| M5 | Efecto acoplado de igualdad diferida y unión: altas candidatas tanto NX-E0102 como NX-E1303. | 8 | 520: 10 `join_resolve`, 6 `literal_resolution_entry`; 2432: 10 y 5. |

Miembros M1: 158, 283, 319, 514, 561, 578, 1365, 1705, 1798, 2156,
2474, 2487, 2850, 2910, 3012, 3200, 3269, 3467, 3680, 3767, 3794,
3838, 4080, 4117.

Miembros M2: 306, 375, 443, 842, 2017, 2144, 2674, 2921, 3338,
3368, 3442, 3489, 3535, 3740, 3800, 4234, 4316, 4749.

Miembros M3: 830, 1088, 1767, 2472, 2742, 3943, 4124.

Miembros M4: 1124, 1990, 3709, 4149, 4969.

Miembros M5: 520, 2432, 2488, 2920, 3731, 3743, 4378, 4992.

La sonda que retiró de forma global el aplazamiento de la proyección redujo
el subconjunto a 21 defectos, pero el sondeo completo encontró nuevas
diferencias y perdió aciertos de oráculo; se restauró. La variante que exigía
un fijador previo redujo el subconjunto a 34, pero el sondeo completo encontró
un defecto nuevo (semilla 221); se restauró. La selección estricta del contexto
de entrada para todo `Join` dio 40 defectos modelables y cambió siete miembros
históricos; se restauró. Una elección universal del primer operando del literal
no cambió los 62; se restauró. Ninguna de esas variantes es una solución.
