# I07c R6 r4 — medición del prototipo de testigo parcial

Estado: medición de A para revisión de B. No constituye veredicto de validación ni autoriza implementar C0–C6. La especificación de texto es `I07c-rediseno-restauracion-atomica-R6-r4-precision.md`; el prototipo desechable y los JSON están en `i07c-rediseno-proto` hasta `913a8fe`, `docs/validation/i07c-rediseno/`. No se modificó el prototipo después de abrir los corpus de validación. Binarios release: `main` SHA-256 `44bef37452e47e9076b4a22db75df6f4a2f6386ab8d9baf39bb7f4f403a4a4a7`, R6 SHA-256 `66c2cf310e7a2f846798f5d893d013c79998bc0ad57b0dce6db7fdeda2934ccd`. Flags R6: `pendfa,retfa,nojoin,atomic0,e0103rule,retfail,nojoinr,skipv3,r5prefix,r1t5,r5t5,noshape,r6witness`. El código sigue siendo prototipo con interruptores, no la representación C0–C6.

## Puerta dirigida previa

35/35 sondas B r3, W1a 215 B, M2 161 B y 10/10 sondas nuevas B r4 coinciden en código y primario por bytes; matriz normativa 45/45. La sonda auxiliar `recursive` de `i07c_rd_cheap.py` espera el primario histórico `[66,67)` en vez del normativo `[60,63)` y no integra la matriz. `r6-r4-directed.json`, `r6-r4-matrix.json`; repetición debug de la dirigida en `r6-r4-directed-debug.json`. W1 tiene los cinco diagnósticos, incluido E1303 `[209,212)`; W1a conserva los cuatro primeros sin `q=aar`.

## Barrido de diagnósticos

La clasificación A/B/C/D compara salida de `check` con oráculo congelado: A = `main` acierta y R6 discrepa; B = R6 acierta y `main` discrepa; C = ambos discrepan igual; D = discrepan distinto. «Core» compara código, subcausa y primario; «full» añade tipos y relación salvo las formas c2 cuyo oráculo solo fija hasta tipo. `OK` y «sin oráculo» no se incluyen en A/B/C/D. Las salidas de `driver_error` se auditan aparte: el arnés de firmas solo leía `diagnostics` y no detectaba el `outcome` anómalo.

| Corpus / perfil | N | Sin oráculo | Core A/B/C/D | Full A/B/C/D |
|---|---:|---:|---:|---:|
| c1 | 24.000 | 0 | 0/2769/0/0 | 17/1463/12698/1545 |
| c2 | 16.000 | 191 | 0/644/7684/70 | 3/360/10804/391 |
| c2@f0-0003-b-v1 | 16.000 | 191 | 7/1088/2375/144 | 12/627/7928/676 |
| b | 4.000 | 41 | 0/551/307/0 | 3/319/2204/282 |
| perm | 4.044 | 222 | 0/342/2/0 | 18/303/1126/130 |
| t | 10.584 | 10.584 | 0/0/0/0 | 0/0/0/0 |
| B 7021993 | 4.500 | 3.402 | 0/207/0/0 | 2/115/526/117 |
| B 4417061 | 4.500 | 3.362 | 0/224/0/0 | 2/129/554/113 |
| B 9908255 | 4.500 | 3.392 | 0/179/0/0 | 1/101/557/92 |
| dirigido5 | 6.000 | 6.000 | 0/0/0/0 | 0/0/0/0 |
| c1-nueva | 8.000 | 0 | 0/822/0/0 | 0/436/4499/460 |
| c2-nueva | 8.000 | 90 | 0/295/3938/32 | 2/175/5435/172 |
| c2-nueva@f0-0003-b-v1 | 8.000 | 90 | 3/553/1190/59 | 6/305/3950/345 |
| c1-nueva2 | 8.000 | 0 | 0/852/0/0 | 4/453/4453/474 |
| c2-nueva2 | 8.000 | 89 | 0/317/3849/30 | 1/189/5481/184 |
| c2-nueva2@f0-0003-b-v1 | 8.000 | 89 | 5/518/1200/67 | 7/286/4096/337 |
| B proyecciones 2026112401 | 3.000 | 0 | 0/682/206/16 | 16/305/1594/584 |
| B proyecciones T5 2026112402 | 3.000 | 0 | 5/1071/223/48 | 15/411/1317/911 |
| B proyecciones 2026112403 | 3.000 | 0 | 0/659/207/12 | 21/276/1610/571 |
| Propietario proyecciones 771412 | 60 | 0 | 0/17/0/0 | 0/0/40/17 |
| **Total de filas/perfiles** | **151.188** | **27.743** | **20/11790/21181/478** | **130/6253/68872/7401** |

Los 20 archivos `r6-summary-*.json` contienen conteos, SHA y flags; `r6-regression-audit.json` enumera las 20 A de núcleo y las 130 A full. Cuatro de las cinco A de núcleo T5 ya discrepaban del oráculo en full para `main`; por ello solo 16 A full coinciden con A core y las otras 114 A full cambian únicamente `related_span`, nunca el tipo esperado o encontrado. En `r6-modeladas-73.json` se reconstruyen exactamente las 73 filas congeladas desde el blob `f4c47df`: 50 A (5 de núcleo, 45 solo relación) y 23 OK.

## Por pieza de diseño, en la batería dirigida

Esta tabla usa solo B3 35, W1a, M2 y las diez nuevas de B r4, con solapamiento de fuentes entre piezas. La firma comparada es código y primario por bytes; los casos y el reparto están en `r6-piece-directed.json`. No extrapola estos conteos a los corpus.

| Pieza | Fuentes | A/B/C/D | OK |
|---|---:|---:|---:|
| `EqualityRecord` | 5 | 0/1/0/0 | 4 |
| `ReadContext` | 9 | 0/3/0/0 | 6 |
| `ResultStatus` / `JoinRecord` / T5 | 20 | 0/8/0/0 | 12 |
| `ConstructorDuty` | 8 | 0/3/0/0 | 5 |
| `Waiting` / `Suspended` | 9 | 0/0/0/0 | 9 |

## Familias reproducidas

El JSON `r6-regression-minima.json` guarda fuente ASCII exacta, longitud y salidas completas de ambos binarios. Cada familia afecta a núcleo o a la procedencia normativa de la firma completa:

1. `match` con `if` anidada en el primer brazo (150 B, `f0-0003-b-v1`): `fn Main(){let o:Option<I64>=Some(1);var q=[];let a=match move o{Some(n)=>{value:(if true then (if true then 1 else q) else q);},None=>{value:true;}};}`. `main`: E0103 `[42,44)` y E1303 `[141,145)`; R6: solo E0103. Las 15 A core de los tres c2 en perfil 0003 son de la familia `matchexpr`; las siete del c2 original también aparecen en la tabla de conservación. Quitar `r6witness` no devuelve E1303; los antiguos interruptores RM-M3 sí lo hacen, sin que ello justifique restaurar ese mecanismo retirado.
2. T5 y constructor pendiente (87 B): `fn Main(){var r=[];var s=[];let a=[(if true then (if 1 then r[0] else [1]) else s),r];}`. `main`: E0102 `[53,54)`; R6 añade E0103 `[25,27)` en `r=[]`. Otra forma de la misma interacción pierde una igualdad posterior (77 B): `fn Main(){var q=[];(if true then (if 1 then [[1]] else [[1]]) else q);q=[1];}`. `main`: E0102 `[37,38)` y `[73,74)`; R6 solo la primera. Las cinco A core del corpus B T5 alternan E0103 adicional y pérdida de contraste posterior.
3. Procedencia `expected_type` (75 B): `fn Main(){let ar=[1];var q=[];var r=[];(if true then [q] else r);r=q;q=ar;}`. Ambos producen E0102/type_mismatch `[67,68)` con tipos iguales; `main` relaciona `[53,56)` y R6 `[65,66)`. Las 114 A full sin A core del barrido tienen esta clase de diferencia de campo, aunque el repro no prueba que compartan una sola causa de implementación.
4. Resultado interno en `match`+`if`+asignación posterior (131 B): `fn Main(){let o:Option<I64>=Some(1);var q=[];let a=match move o{Some(n)=>{value:(if true then q else q);},None=>{value:q;}};q=[1];}`. `main` acepta (`check=checked`, `run=unit`); R6 da `driver_error/internal_error` en ambos. Aparece en cuatro fuentes de c2@0003, seis de c2-nueva@0003, tres de c2-nueva2@0003 y seis fuentes de las semillas de B: **19 regresiones**. Otras dos fuentes de semillas dan `driver_error` tanto en `main` como en R6. Las 21 se reprodujeron con `check` y `run`; debug/release R6 coinciden en `driver_error` para las 21. El error persiste sin `r6witness` y con RM-M3, desaparece sin `noshape`; `noshape` solo lo reproduce. `r6-driver-error-audit.json`, `r6-pass-errors-reproduced.json`, `r6-internal-error-minima.json`, `r6-internal-error-ablation.json`. El barrido de diagnósticos no contó las 19 regresiones en las 20 A: **39 filas/perfiles distintas de núcleo regresan** al incluir el resultado de ejecución.

## `match`, ejecución, RD, pila y pasadas

`r6-match103.json`: 103 filas, 51 de conservación; 42/51 idénticas a `main`, 9 distintas. En las otras 52 de diferencias declaradas, 26 igualan el esperado declarado y 26 no. De las nueve de conservación, dos son `rejilla2-anidada-primera-pendiente-s-q` y `B-178-anidada-pendiente-externa-q`; las otras siete son `X-matchexpr-*` del primer perfil c2@0003.

`r6-profile-acceptance.json`: 54 fuentes/perfiles, aceptación `main`/R6 igual en 54/54 y `check`/`run` igual en 54/54; mediana de lanzamiento release `main` check/run 21,977/22,069 ms y R6 21,961/22,023 ms. `r6-debug-release-acceptance.json`: 54/54 paridades de resultado y 54/54 de aceptación `check`/`run`; la batería dirigida se repitió en debug. Estas 54 fuentes no incluían los 21 `driver_error`; su contraste aislado se documenta arriba. RD: 152/152 pares normativos `check`/`run`, cero diferencias de snapshot (`r6-rd-log.txt`).

Pila de test con rustc 1.99.1 y `/MAP`: `Checker::drive` 27.536 B, `resume_non_match` 33.080 B, pareja 60.616 B, margen 5.944 B frente a 66.560 B (`r6-stack-map.json`, SHA de exe/MAP). Una repetición posterior dio 5/5 `option_match126` y 5/5 `result_match126` a 65 KiB, pero la primera tuvo desbordamiento en la tercera repetición de `option_match126`. El estrés posterior sobre **el mismo exe** dio `option_match126` 97/100 con flags R6 y 97/100 sin flags, `result_match126` 30/30 con flags (`r6-stack-stress.json`); los seis fallos son desbordamientos. La puerta de 65 KiB es inestable.

Las pasadas de los 20 corpus/perfiles se registran en `r6-pasadas-*.json`: 151.188 ejecuciones, máximo observado **2**, histograma `0:15.558`, `1:1.280`, `2:134.329`, más 21 `driver_error` fuera del histograma; cero inconclusas. No se cuenta un `driver_error` como pasada válida. Los `driver_error` por corpus/perfil son c2@0003 **4**, c2-nueva@0003 **6**, c2-nueva2@0003 **3**, B 7021993 **5**, B 4417061 **2**, B 9908255 **1**. Los cuatro corpus de proyecciones y los demás conjuntos tuvieron cero. El valor máximo 2 no resuelve los fallos de diagnóstico, resultado ni pila.
