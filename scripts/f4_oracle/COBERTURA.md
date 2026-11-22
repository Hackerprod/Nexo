# F4 oráculo: cobertura y contraste con el binario

El producto no implementa F0-0004 (K01–K12 pendientes): no existe binario contra el que contrastar ninguna familia de los tramos O1 y O2.

| Tramo | Familias | Contraste con el binario |
|---|---|---|
| F4-O1 | L-NX-E1401 (G0), barrera K02–K10 y manifiesto, literales de K §§10.1–10.8 (integridad y fragmentos) | NO EVALUADO: ninguna capacidad K está implementada, el binario vigente rechaza o ignora la sintaxis F4. |
| F4-O2 | String/parseo/slicing/comparadores, nominales, SCC, propiedad/préstamo, métodos, patrones, derive/Format/PartialEq, visibilidad (pares G1–G8) | NO EVALUADO: mismo motivo; los pares comprueban el modelo contra los literales de K, no contra el CLI. |
| F4-O3 | Generadores V/N/L/B (G0–G10): 14 familias V, 71 N, 7 L, 12 B, positivos reparados, `reductions`, forma estructural y SHA de corpus | NO EVALUADO: el binario no implementa K; las filas literales de K se reproducen con elecciones guionadas del generador (32 filas). |
| F4-O4 | Arnés: `generic` (copiadas/adaptadas con pines AST), `runner` (solo JSONL), `cli` (generate, fuzz-corpus, compare, gate, fuzz, minimize, batch), `minimize`, `batch`, `known_issue`, fuzz, reanudación | NO EVALUADO contra el producto: el binario no implementa K; el arnés se prueba con ejecutables ficticios (45 corrupciones detectadas, control fiel con exit 0). |
| F4-O5 | Aplicación de K 11 (`app`: regla de negocio con enteros ilimitados), entorno v2 (lector JSON estricto y validación de esquema 2, E01–E07, A08 con entorno), once variantes y lote de cien líneas, clase A (entornos de 1–100 líneas), clase E (valida o con un defecto de 34 familias), recetas Q (`charges`: String 65.535/65.536/65.537, cadena de copias, bucle de asignaciones, miembros 16.383/16.384/16.385 y los cuatro perfiles reducidos), filas X24–X27 y O13, fases G9 y G10 del `selftest`, matriz C01–C11 | NO EVALUADO: el binario no implementa K10 ni los perfiles `f0-0004-b-test-*`; los ejecutables ficticios aceptan el entorno por SHA-256 de la fuente y del entorno (control fiel exit 0 y corrupciones detectadas). |
