# F4 oráculo: cobertura y contraste con el binario

El producto no implementa F0-0004 (K01–K12 pendientes): no existe binario contra el que contrastar ninguna familia. El contraste con el binario del último commit verde es `NO EVALUADO` en todos los tramos (motivo en cada fila); el oráculo se prueba con los literales de K, el modelo, ejecutables ficticios (`tests/fixtures/f4_oracle_mock_nexo.py`) y la campaña de mutación (`MUTACIONES.md`). Ningún esperado sale del binario.

## Cobertura por fase

Lo comparable por cada fase sale de K §10.x y K §12 (plan §2.2). «Literales» son filas transcritas de K con su fragmento verificado byte a byte; «generadas» son clases V, N, L, B, A, E y Q con esperado del modelo.

| Fase | Lenguaje y reglas | Literales de K | Generadas |
|---|---|---|---|
| G0 | gramática del literal (K §2.1), NX-E1401 con sus 6 subcausas, barreras K02–K10, manifiesto | X01–X03, L01–L06, H01–H06, N13, U01 | L (7 familias), B (12), V sin String |
| G1 | String propietario, copia, `==`, `ByteLen`, `Print`, indexación, `no_alloc`, cuotas de texto | X04, X23–X26, N09, A01 | V, N, B de K02 |
| G2 | resto de §3, `std.text`, `std.parse`, seis comparadores, Array/for de String, `s[a..<b]` | T01–T14 (10 filas y 4 adicionales, con `use std.parse`), X22, M05, O01–O04, O10, O11, O13, B01-text, A01, A02 | V, N de texto |
| G3 | struct/enum, identidad, constructores, SCC, `nominal_members`, campos, copia | X05–X10, X21, X27, M07, N03, N04, N12, P02, P05, P06, H09, O12, B01-struct/enum, K04P | V, N de nominales |
| G4 | recursos en nominales, limpieza completa y parcial, préstamos de campo | X11, M04, N05–N08, O09, A06 (y su variante fallida) | N de propiedad |
| G5 | `extend`, `use extension`, receptor temporal, NX-E0002 de prefijo | M01–M03, P01, H12, H13, H12-edit, H13-move | V, N de métodos |
| G6 | patrones nominales, aridad, cobertura, testigo residual | N01, N02, H11, A05 | N de patrones |
| G7 | derive, Format/PartialEq, NX-E1403/E1404, comparadores nominales, interpolación | X12–X16, N10, N11, M08, O05–O08, O14–O16, H07, H08, A04, A07, CMP6 | V, N de derive |
| G8 | `pub`, firmas, imports, alias, `pub use` | X17–X20, M06, P03, H10, A03 (P04 solo como par del modelo) | V, N de visibilidad |
| G9 | entorno v2, `ReadLine`/`Println`, eventos, bytes, controlador, aplicación | E01–E07 (negativo y positivo), A08 con entorno, `APP`, `APP-V01`–`APP-V11`, `APP-100`, X24–X27 y O13 con su perfil reducido | A (aplicación con entorno de 1–100 líneas), E (valida o con un defecto de 34 familias), Q (perfil normal y cuatro reducidos) |
| G10 | todo a escala, fuzz, fronteras, matriz C01–C11 | todos los de G0–G9 | recetas largas de Q con `--final`, fuzz de fuentes y de entornos, matriz C01–C11 |

## Contraste con el binario por tramo

| Tramo | Familias | Contraste con el binario |
|---|---|---|
| F4-O1 | L-NX-E1401 (G0), barrera K02–K10 y manifiesto, literales de K §§10.1–10.8 (integridad y fragmentos) | NO EVALUADO: ninguna capacidad K está implementada, el binario vigente rechaza o ignora la sintaxis F4. |
| F4-O2 | String/parseo/slicing/comparadores, nominales, SCC, propiedad/préstamo, métodos, patrones, derive/Format/PartialEq, visibilidad (pares G1–G8) | NO EVALUADO: mismo motivo; los pares comprueban el modelo contra los literales de K, no contra el CLI. |
| F4-O3 | Generadores V/N/L/B (G0–G10): 14 familias V, 71 N, 7 L, 12 B, positivos reparados, `reductions`, forma estructural y SHA de corpus | NO EVALUADO: el binario no implementa K; las filas literales de K se reproducen con elecciones guionadas del generador (32 filas). |
| F4-O4 | Arnés: `generic` (copiadas/adaptadas con pines AST), `runner` (solo JSONL), `cli` (generate, fuzz-corpus, compare, gate, fuzz, minimize, batch), `minimize`, `batch`, `known_issue`, fuzz, reanudación | NO EVALUADO contra el producto: el binario no implementa K; el arnés se prueba con ejecutables ficticios (45 corrupciones detectadas, control fiel con exit 0). |
| F4-O5 | Aplicación de K 11 (`app`: regla de negocio con enteros ilimitados), entorno v2 (lector JSON estricto y esquema 2, E01–E07, A08), once variantes y lote de cien líneas, clases A, E y Q (`charges`: String 65.535/65.536/65.537, cadena de copias, bucle de asignaciones, miembros 16.383/16.384/16.385 y los cuatro perfiles reducidos), X24–X27 y O13, grupos G9 y G10 | NO EVALUADO: el binario no implementa K10 ni los perfiles `f0-0004-b-test-*`; los ficticios responden por SHA-256 de la fuente y del entorno (control fiel exit 0, corrupciones detectadas también sobre registros con entorno). |
| F4-O6 | `mutate` (69 mutantes: 68 muertos con check etiquetado y 1 equivalente con argumento), guardas etiquetadas de K en `selftest`, `README`, consolidación de lagunas y hallazgos | NO EVALUADO: es herramienta sobre el propio oráculo; la campaña no ejecuta ningún binario de Nexo. |

## Matriz C01–C11 de K §12.1 (`compare/gate --final` la imprime)

| Criterio | Estado | Qué cubre el oráculo y qué no |
|---|---|---|
| C01 superficie | PARCIAL | Literales de §10 y clase L; offsets en bytes con texto multibyte. NO EVALUADO: parse–print–parse (la CLI no expone AST ni impresor). |
| C02 texto | PASA | `utf8ref`, T01–T14, familias de String, O01–O04/O10/O11, matriz de tamaños y escalares de K §12.2. |
| C03 nominalidad | PASA | X05–X10, X21, X27, N03/N04, P02/P05/P06, constructores y SCC. |
| C04 propiedad | PARCIAL | X11, N05–N08, M04, O09, A05, A06. NO EVALUADO: conteo de cierres por adquisición en programas generados con recursos. |
| C05 efectos | PARCIAL | `no_alloc` de literal, copia y `Concat` (X23, N09). NO EVALUADO: `no_io` de Print/ReadLine/Println y destructores transitivos (K no da fila ni diagnóstico fijo). |
| C06 selección/métodos | PASA | M01–M03, H12, H13, P01, N01/N02, H11. |
| C07 IR | NO EVALUADO | El verificador de IR no es observable por la CLI. |
| C08 simulación | PARCIAL | E01–E07, A08, clase E con colas, eventos y bytes. NO EVALUADO: igualdad text/json (solo se compara JSON) y agotamiento de cuota sin consumo de cola. |
| C09 límites | PARCIAL | Recetas Q (L−1/L/L+1) en perfiles normal y reducidos, X24–X27, O13, fuzz sin pánico. NO EVALUADO: los catorce perfiles reducidos de F5 y las pilas de 256/512 KiB. |
| C10 producto | PASA | A01–A08, aplicación y variantes, lote de cien líneas, clase A; sin detección por fuente. |
| C11 conservación | NO EVALUADO | Las regresiones de A2/F0-0003/F0-0005 son de otras suites; el oráculo exige perfil explícito. |

## Fuera del oráculo CLI (declarado en el plan)

Ticks y `work_consumed` (LF-06), bytes vivos de Array, pilas y profundidad 127/128/129, verificador de IR, P04 (par del modelo, NO EVALUADO por el CLI), los catorce perfiles reducidos trasladados de F5 (solo se usan los cuatro de texto y miembros) y `gen`/captura/SuspendSafe de K §4.2. `nominal_members` solo desde G3.

## Cifras de la última medición (clon limpio de la rama)

`selftest` completo: 762 comprobaciones, 0 fallos, 174 s (límite del plan: 10 min); `selftest --fast`: 21 comprobaciones, < 1 s (límite: 60 s); `audit`: 0 hallazgos; `mutate F4-Ok`: 69 mutantes (68 muertos, 1 equivalente), unos 4 min con 2 workers más 3 min del equivalente; corpus pinado: 376 entradas (`V N L B` por semilla y, desde G9, `A E Q`, y `G10F` con las recetas finales).
