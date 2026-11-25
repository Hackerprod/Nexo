# I07c — barrido completo de R5 tras FAIL de B r1

Estado: **FAIL de A; pendiente la revalidación independiente de B**. R5 por sitio de sentencia no es la representación de cinco dominios del diseño. Base `main` release SHA-256 `44bef37452e47e9076b4a22db75df6f4a2f6386ab8d9baf39bb7f4f403a4a7`; candidato release `82729b1f2b6c21300974a455614c2c12ad0d20a48c4f3da00456ae9cfc4890eb`. Flags `pendfa,retfa,retract,nojoin,atomic0,e0103rule,retfail,rmm3b,rmm3e,rmm3f,nojoinr,skipv3,r5prefix,r1t5,r5t5`. Los resúmenes JSON y las 102 fuentes A modeladas están en `docs/validation/i07c-rediseno/` de `i07c-rediseno-proto` (commit `211138a`); el arnés usa cuatro workers, comprueba SHA de cada corpus y guarda todos los lotes sin error de proceso/JSON.

A = `main` coincide con oráculo y R5 difiere; B = R5 coincide y `main` difiere; C = ambos difieren igual; D = ambos difieren de modo distinto. `core` compara código, subcausa y primario; `full` añade tipos y `related_span` de `expected_type`. Los corpus sin oráculo quedan **INCONCLUSOS**, no forman evidencia de A=0.

| Corpus/perfil | N | A/B/C/D core | A/B/C/D full | Sin oráculo; cambios |
|---|---:|---|---|---:|
| c1 | 24.000 | 0/2769/0/0 | 8/1464/12702/1540 | 0; 0 |
| c2 / 0002 | 16.000 | 0/644/7684/70 | 0/362/10804/389 | 191; 13 |
| c2 / 0003 | 16.000 | 0/1094/2369/144 | 1/636/7922/673 | 191; 13 |
| b | 4.000 | 0/551/307/0 | 2/311/2214/280 | 41; 0 |
| perm | 4.044 | 0/342/2/0 | 12/262/1164/133 | 222; 0 |
| t | 10.584 | 0/0/0/0 | 0/0/0/0 | 10.584; 4807 |
| s7021993 / 0003 | 4.500 | 0/207/0/0 | 1/112/530/116 | 3402; 550 |
| s4417061 / 0003 | 4.500 | 0/224/0/0 | 2/128/555/113 | 3362; 547 |
| s9908255 / 0003 | 4.500 | 0/179/0/0 | 2/99/559/92 | 3392; 542 |
| dirigido5 | 6.000 | 0/0/0/0 | 0/0/0/0 | 6.000; 165 |
| c1-nueva | 8.000 | 0/822/0/0 | 0/434/4500/461 | 0; 0 |
| c2-nueva / 0002 | 8.000 | 0/295/3938/32 | 1/175/5438/169 | 90; 7 |
| c2-nueva / 0003 | 8.000 | **1**/559/1184/59 | 3/311/3947/342 | 90; 7 |
| c1-nueva2 | 8.000 | 0/852/0/0 | 3/452/4454/474 | 0; 0 |
| c2-nueva2 / 0002 | 8.000 | 0/317/3849/30 | 1/188/5482/184 | 89; 2 |
| c2-nueva2 / 0003 | 8.000 | 0/520/1197/68 | 2/289/4094/336 | 89; 2 |
| B proyecciones 2026112401 | 3.000 | 0/682/206/16 | 10/295/1645/543 | 0; 0 |
| B T5 2026112402 | 3.000 | **22**/1063/162/117 | 20/399/1288/952 | 0; 0 |
| B proyecciones 2026112403 | 3.000 | 0/659/207/12 | 18/261/1664/532 | 0; 0 |
| Propio 771412 | 60 | 0/17/0/0 | 0/0/40/17 | 0; 0 |

Las 20 filas son 13 conjuntos históricos, sus tres perfiles adicionales y cuatro corpus nuevos. La colección nueva de B transcribe el generador aportado, pero las semillas originales y los archivos de las 9.000 fuentes de B no se entregaron: estos 9.000 son muestras nuevas. En las fuentes modeladas hay **23 A core** (22 en B T5, una en c2-nueva/0003) y **79 A adicionales full**; estas últimas cambian solo el `related_span` de `expected_type` y bloquean S5a.5. La fuente 7479 de c2-nueva/0003 pierde `NX-E1303` `[222,224)` en un `match` cuyo primer brazo contiene una `if` anidada. El oráculo de esa forma usa un análogo, así que la adjudicación normativa queda ligada a la laguna de `match`; la cifra A no se modifica.

`r5-match103.json` ejecuta 103 fuentes: 51/51 de conservación idénticas a `main`; R5 cambia 51/52 de las otras. La declaración histórica de dos filas corresponde a una base `d51b5645…`, no al `main` actual: `dif-s4417061-292-anadido` coincide hoy entre ambos binarios y `dif-s9908255-3958-anadido` difiere en ambos de aquella tabla. Se requiere decisión normativa del valor parcial de un brazo `match` antes de adjudicar esas 52 filas, propuesta en `I07c-rediseno-restauracion-atomica-lagunas-B-r1.md` con fuente ASCII de 181 bytes. Las 51 de conservación siguen siendo puerta obligatoria.

M1/M2 de B: 6/6 en tres perfiles. Gemelos válidos de B: 15/15; el décimo sexto es `NX-E0002` sintáctico en ambos binarios y no cuenta como negativo de tipos. Sondas dirigidas: 16/16; matriz T5/N 45/45. Esas sondas no discriminan 22 A T5 ni 79 fallos de procedencia, por lo que no fundamentan PASS. `r5-consumers-experimento.md` de la rama prototipo muestra además que sustituir el sitio de sentencia por ancestros `FactId` activos falla `planned` (15/16): la dependencia cruza visitas del driver.

Mediciones posteriores del mismo ejecutable: `i07c_rd_conformidad.py --verify` con los flags de R5 dio **152/152 pares RD `check`/`run`**, `RD snapshot differences: 0`, filtro autoverificado; `r5-rd152.log` y `r5-followup-summary.json` conservan salida y SHA. En 18 fuentes dirigidas por cada uno de los tres perfiles, **54/54** coincidieron en aceptación entre `check`/`run` y entre `main`/R5. La mediana exploratoria por lanzamiento en esa batería fue `main` 21,057 ms `check` / 21,051 ms `run`; R5 21,120 / 21,033 ms. Estas medianas están dominadas por arranque de proceso y no prueban la puerta de +10 % del solucionador. `r5-profile-acceptance.json` conserva cada invocación.

La ablación de `retract` y de `atomic0` buscó para cada uno un programa donde R5 coincidiera con el oráculo de núcleo y retirarlo no: **ninguno en 4.200 fuentes modeladas por pieza** (300 por cada una de 14 filas de corpus/perfil). Al retirar ambos a la vez, las 16 sondas y 45 filas T5/N permanecieron idénticas (16/16, 45/45), guardadas en `r5-no-retract-atomic0-sondas.json`. Esto **no demuestra equivalencia en los 13 conjuntos completos**; sí deja ambas piezas sin sonda que justifique mantenerlas en un futuro prototipo de regla única. El diseño R6 las excluye salvo que aparezca un contraejemplo normativo. Las otras siete piezas heredadas tienen testigos dirigidos en `r5-ablation-witnesses.json` de la rama prototipo, aunque RM-M3 sigue pendiente de la laguna de `match`.

La ablación adicional de `r1t5,r5t5,retract,atomic0` tampoco define un corte viable: las sondas quedan 16/16, pero la matriz baja a **44/45**; en tres corpus B de 3.000 fuentes cada uno, A/B/C/D core = `0/682/206/16`, **`53/321/702/319`** (T5) y `0/659/207/12`. En full aparecen 12, 27 y 17 A respectivamente. Los archivos `r5-noT5-*.json` del commit `8fd6ff6` de la rama prototipo guardan los resultados. La reducción de alcance que omite T5 se descarta por esos 53 contraejemplos y la fila de matriz perdida.

Los demás requisitos de la puerta — diferencial debug/release, exe/MAP y pila, tiempo del solucionador separado del arranque, copia acotada de nodos, terminación, S5a.2/.3 y disposición efectiva de los 17 tests — siguen **NO EVALUADOS** para una representación válida. La puerta completa se reserva para un candidato sin A y con `match` decidido: conforme a NX-build, un fallo reproducible basta para FAIL, solo una ejecución completa probaría éxito. No se declara PASS ni se escribe plan de implementación.
