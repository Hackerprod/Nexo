# D6: 15 diferencias declaradas de match con if en brazo

Cada fuente ASCII, su SHA-256, la salida JSON completa de main, la derivacion del if aislado y la salida normativa propuesta para R6 estan en `r13-match-d6.json`. Las 15 salidas de main se ejecutaron con el binario compilado de 31d91db. El generador coteja cada diagnostico del if aislado con la propuesta en codigo, subcausa, tipos, relaciones y spans tras desplazar los bytes. Los diagnosticos adicionales y su causa se enumeran por fila en el JSON. B debe derivar de forma independiente cada salida completa propuesta antes de C3. Los intervalos siguientes son bytes `[inicio,fin)`.

| Fila | ID; bytes | main actual (codigo y primario) | R6 propuesto (codigo y primario) | Clausula |
|---:|---|---|---|---|
| 5 | `rejilla1-anidada-else-denunciada`; 181 B | E0103 [64,66), E0103 [73,75), E1303 [149,153) | E0103 [64,66), E0103 [73,75) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 7 | `rejilla1-anidada-else-denunciada2`; 179 B | E0103 [64,66), E0103 [73,75), E1303 [143,144) | E0103 [73,75), E1303 [143,144) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 37 | `B-161-anidada-else-pendiente-denunciada`; 161 B | E0103 [53,55), E1303 [129,133) | E0103 [53,55) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 57 | `dif-s7021993-78-anadido`; 270 B | E1303 [219,223) | E1303 [219,223), E1303 [260,261), E0102 [267,268) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 59 | `dif-s7021993-561-quitado`; 260 B | E0103 [93,95), E0103 [102,104), E0103 [111,113), E1303 [222,224), E1303 [254,255) | E0103 [102,104), E1303 [202,207), E1303 [231,236) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 62 | `dif-s7021993-1569-anadido`; 239 B | E0103 [93,95), E0103 [111,113) | E0103 [111,113), E1303 [201,202), E1303 [232,234) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 65 | `dif-s4417061-996-quitado`; 279 B | E1303 [238,243), E1303 [273,274) | E1303 [238,243), E1303 [251,255) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 67 | `dif-s4417061-1190-quitado`; 260 B | E1303 [208,212), E1303 [220,225), E1303 [243,246) | E0103 [93,95) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 68 | `dif-s4417061-2805-quitado`; 291 B | E1303 [183,185), E0102 [262,263), E1303 [284,286) | E1303 [183,185), E1303 [222,226) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 69 | `dif-s4417061-3243-anadido`; 235 B | E0103 [102,104), E0103 [185,186) | E1303 [185,186), E1303 [223,224) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 71 | `dif-s4417061-3389-anadido`; 239 B | E0103 [93,95), E0103 [102,104), E0103 [111,113) | E0103 [93,95), E0103 [111,113), E1303 [198,201), E1303 [231,234) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 72 | `dif-s4417061-3509-anadido`; 324 B | E1303 [226,227), E1303 [233,234), E1303 [261,262), E1303 [279,284), E0102 [312,315), E0102 [320,321) | E1303 [226,227), E1303 [261,262), E0102 [282,283), E1303 [302,305), E0102 [312,315), E0102 [320,321) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 78 | `dif-s9908255-1377-anadido`; 319 B | E1303 [191,192), E1303 [210,214), E1303 [251,253), E1303 [270,275) | E1303 [191,192), E1303 [210,214), E1303 [251,253), E1303 [312,313) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 79 | `dif-s9908255-1876-quitado`; 245 B | E0103 [111,113), E1303 [208,209), E1303 [239,240) | E0103 [111,113), E1303 [208,209), E1303 [217,221) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
| 102 | `rmm3fuera-s4417061-3751`; 221 B | E1303 [173,177) | E1303 [173,177), E0102 [216,219) | F0-0002 4.1.2.1-5, 13.9; D5/D6 |
