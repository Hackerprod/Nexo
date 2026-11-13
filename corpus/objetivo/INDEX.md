# Programas objetivo

Corpus progresivo de programas objetivo; tres ejemplos del roadmap conservan grafías de sintaxis A, documentadas en las traducciones pendientes.

<!-- tabla-programas -->
| ID | Archivo | Procedencia | Capacidad | Estado hoy | Motivo |
|---|---|---|---|---|---|
| 01 | 01_integer_limits.n | F0-0002 sec. 11; límites I64 | límites I64 comprobados | EJECUTA | — |
| 02 | 02_nested_copy.n | F0-0002 sec. 11; copia profunda de arrays | arrays anidados y copia | EJECUTA | — |
| 03 | 03_resource_transfer.n | F0-0002 sec. 11; transferencia de recursos | MockResource y move | EJECUTA | — |
| 04 | 04_return_cleanup.n | F0-0002 sec. 11; limpieza al retornar | limpieza de recursos | EJECUTA | — |
| 05 | 05_shadow_scope.n | F0-0002 sec. 11; sombreado de ámbito | ámbitos y shadowing | EJECUTA | — |
| 06 | 06_result_ok.n | F0-0002 sec. 11.8; Result | Result y propagación ? | EJECUTA | — |
| 07 | 07_result_error.n | F0-0002 sec. 11.8; Result | propagación tipada de Err | EJECUTA | — |
| 08 | 08_array_update.n | F0-0002 sec. 11.4; arrays | copia y escritura indexada | EJECUTA | — |
| 09 | 09_mock_request.n | F0-0002 sec. 11.6; MockRequest | petición simulada y Result | EJECUTA | — |
| 10 | 10_mock_write.n | F0-0002 sec. 11.6; MockWriteI64 | escritura simulada | EJECUTA | — |
| 11 | 11_ex04_generador.n | EX04 reducido; F0-0003 sec. 11 | generador, Result, recursos y escritura simulada | NO EJECUTA | NX-E0006/generator |
| 12 | 12_option_match.n | F0-0003 sec. 3 y 6; Option | Option y selección exhaustiva | NO EJECUTA | NX-E0006/option |
| 13 | 13_option_none.n | F0-0003 sec. 3; Option | None y match | NO EJECUTA | NX-E0006/option |
| 14 | 14_closure_capture.n | F0-0003 sec. 4; closure síncrona | captura y llamada de closure | NO EJECUTA | NX-E0006/closure |
| 15 | 15_generator_sum.n | F0-0003 sec. 5 y 9; generador | yield, Next y recorrido | NO EJECUTA | NX-E0006/generator |
| 16 | 16_primos.n | F0-0005 sec. 11 (F5-P01) | operadores, while y funciones | NO EJECUTA | NX-E0006/comptime |
| 17 | 17_criba.n | F0-0005 sec. 11 (F5-P02) | arrays y control de flujo | NO EJECUTA | NX-E0006/comptime |
| 18 | 18_collatz.n | F0-0005 sec. 11 (F5-P03) | operadores y bucles | NO EJECUTA | NX-E0006/comptime |
| 19 | 19_ordenacion.n | F0-0005 sec. 11 (F5-P04) | arrays y actualización | NO EJECUTA | NX-E0006/for_iteration |
| 20 | 20_busqueda_binaria.n | F0-0005 sec. 11 (F5-P05) | selección y arrays | NO EJECUTA | NX-E0002 |
| 21 | 21_aplicacion_registros.n | F0-0004 sec. 11; aplicación de referencia | líneas, registros nominales, Result, acumulación y texto | NO EJECUTA | NX-E0006/modules_visibility |
| 22 | 22_texto_bytes.n | F0-0004 sec. 11.1; A01 | texto y bytes | NO EJECUTA | NX-E0006/text |
| 23 | 23_texto_parseo.n | F0-0004 sec. 11.1; A02 | texto y parseo | NO EJECUTA | NX-E0006/text |
| 24 | 24_texto_arrays.n | F0-0004 sec. 11.1; A03 | String y arrays | NO EJECUTA | NX-E0006/modules_visibility |
| 25 | 25_tipo_nominal.n | F0-0004 sec. 11.1; A04 | struct y enum nominales | NO EJECUTA | NX-E0006/attributes |
| 26 | 26_ex01_herramienta.n | EX01 del roadmap; sec. EX01, CLI/archivo no simulados en los ADR | programa completo EX01 | NO EJECUTA | NX-E0006/modules_visibility |
| 27 | 27_ex02_http_json.n | EX02 del roadmap; sec. EX02, HTTP/JSON sin interfaz simulada | programa completo EX02 | NO EJECUTA | NX-E0006/modules_visibility |
| 28 | 28_ex03_simulacion.n | EX03 del roadmap; sec. EX03, Table/f32/genéricos sin reemplazo simulado | programa completo EX03 | NO EJECUTA | NX-E0006/modules_visibility |
| 29 | 29_ex04_generador_roadmap.n | EX04 del roadmap; sec. EX04, u32/u64 y consola sin equivalente completo | programa completo EX04 | NO EJECUTA | NX-E0006/modules_visibility |
<!-- /tabla-programas -->

## Traducciones pendientes de APIs simuladas

- EX01 conserva `std.cli`, `std.fs`, `std.parse` y `std.console` del roadmap porque los ADR no definen reemplazos equivalentes para argumentos, archivos ni texto. La lógica completa se transcribe sin inventar esas operaciones.
- EX02 conserva las APIs `std.http` y `std.json`: no hay una interfaz simulada de servidor, petición o serialización que permita traducirlas sin añadir semántica.
- EX03 conserva `Table`, `TableLayout`, `UniqueTableView`, `SoA`, `AoS` y `f32`; los ADR no definen equivalentes simulados para esas APIs ni ese tipo numérico.
- EX04 conserva `u32`, `u64`, el argumento por defecto y las APIs de texto/consola del roadmap. F0-0003 sec. 11 tiene un objetivo EX04 reducido separado, escrito con las APIs simuladas que sí especifica.
