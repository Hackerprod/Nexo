# F4 oráculo: campaña de mutación sobre el propio oráculo

`python -B -m scripts.f4_oracle.mutate F4-Ok`: una sustitución exacta por mutante sobre una copia privada por worker (`C:/nexo-work/tmp/f4-oracle-mut-<PID>-<worker>/`, cerrojo de copia, 2 workers), restauración del archivo y comprobación de su SHA-256 tras cada mutante, parada en el primer check que falla (`selftest --fail-fast` sobre los grupos que ejercitan el archivo; un equivalente se ejecuta con el `selftest` completo). Resultado de la pasada final (HEAD de O7): **85 mutantes, 84 muertos, 1 equivalente, 0 supervivientes abiertos**; muertos por excepción (`crash`): ninguno. Cada mutante muerto cita el check etiquetado que lo mata; las etiquetas `K…` nombran la sección de K que fundamenta el caso.

Historia: la pasada de O6 (69 mutantes) dejó 15 supervivientes que se cerraron con casos de K. La validación D de O6 añadió 19 mutantes propios (`d-mutantes-nuevos`): sobrevivían `%` fuera de `OPERATORS`, `evidence` de la cadena causal y el límite `string_bytes` del evaluador; en O7 se añadieron casos con esperado de K (literales de K 11, operadores de un Hole, escapes de D15 2.4, `evidence=verified` de K 10.5 y A2 13.6, límites de K 8.2 en la frontera, una sola K pendiente en la clase B) y los mutantes L06, L07, D01, Q05, Q06, B04, B05, H08–H11 (más R01–R04 y S01 de las familias nuevas); todos mueren. N07 y N16 de D se reprodujeron: N07 (escape `\t`) es L07 y N16 (`>= 1` en la clase B) es B05.

| ID | Familia | Archivo | Mutación | Veredicto | Check que la mata |
|---|---|---|---|---|---|
| T01 | text | `model.py` | ByteLen como ScalarLen | killed | `A01.outcome` |
| T02 | text | `model.py` | ScalarLen como ByteLen | killed | `A01.outcome` |
| T03 | text | `model.py` | normalizacion NFC en Scalars | killed | `A02.outcome` |
| T04 | text | `utf8ref.py` | orden con signo | killed | `utf8ref.compare_unsigned` |
| T05 | text | `utf8ref.py` | prefijo igual mal ordenado | killed | `utf8ref.compare_unsigned` |
| T06 | text | `model.py` | derecha primero en operadores binarios | killed | `K3.operands_evaluated_left_to_right.<` |
| T07 | text | `model.py` | != igual a == | killed | `model.compare_prefix_and_signed_bytes` |
| T08 | text | `model.py` | SliceBytes acepta end=len+1 | killed | `model.slice_bytes` |
| T09 | text | `utf8ref.py` | SliceBytes con start=end en EOF | killed | `utf8ref.boundaries` |
| T10 | text | `model.py` | frontera antes que rango | killed | `model.slice_bytes` |
| T11 | text | `model.py` | parse acepta + | killed | `model.parse_i64` |
| T12 | text | `model.py` | parse recorta espacios | killed | `model.parse_i64` |
| T13 | text | `model.py` | overflow antes de validar el resto | killed | `model.parse_i64` |
| T14 | text | `model.py` | parse rechaza -0 | killed | `model.parse_i64` |
| T15 | text | `model.py` | digitos Unicode | killed | `model.parse_i64` |
| T16 | text | `model.py` | vacio vale 0 | killed | `model.parse_i64` |
| T17 | text | `model.py` | MIN falla con -9102 | killed | `model.parse_i64` |
| T18 | text | `model.py` | FromI64 en MIN sin signo | killed | `model.from_i64` |
| T19 | text | `model.py` | Scalars por bytes | killed | `A02.outcome` |
| T20 | text | `model.py` | IsEmpty | killed | `K3.IsEmpty_is_ByteLen_equal_zero` |
| T21 | text | `model.py` | Concat invertido | killed | `K3.Concat_self_then_other` |
| N01 | nominal | `model.py` | asociacion posicional invertida | killed | `K5.2.derived_format_shapes` |
| N02 | nominal | `model.py` | evaluacion en orden de campo | killed | `A06.outcome` |
| N03 | nominal | `model.py` | limpieza por orden de declaracion | killed | `A06.outcome` |
| N04 | nominal | `model.py` | limpieza parcial en orden de evaluacion | killed | `K4.4.incomplete_constructor_cleans_in_reverse_evaluation_order` |
| N05 | nominal | `model.py` | copiable si lo es alguna alternativa | killed | `N08.diagnostic` |
| N06 | nominal | `model.py` | ciclo mas largo | killed | `K4.2.shortest_cycle_primary_is_last_edge` |
| N07 | nominal | `model.py` | primario en la primera arista | killed | `H09.diagnostic` |
| N08 | nominal | `model.py` | derive concede orden | killed | `O03-<.diagnostic` |
| N09 | nominal | `model.py` | Array vacio sin capacidad | killed | `K5.2.AssertEq_requires_PartialEq.option_of_nominal` |
| N10 | nominal | `model.py` | Format implica PartialEq | killed | `K5.2.Format_does_not_imply_PartialEq` |
| N11 | nominal | `model.py` | capacidad antes que tipos distintos | killed | `K5.2.different_types_fail_by_type_before_capability` |
| N12 | nominal | `model.py` | Format con package. | killed | `K5.2.derived_format_shapes` |
| N13 | nominal | `model.py` | separador de payload | killed | `K5.2.derived_format_shapes` |
| P01 | permisos | `model.py` | view escribe | killed | `N06.diagnostic` |
| P02 | permisos | `model.py` | asignacion a let | killed | `X10.diagnostic` |
| P03 | permisos | `model.py` | temporal sin NX-E1107 | killed | `H12.diagnostic` |
| P04 | permisos | `model.py` | modo igual al requerido rechazado | killed | `M01.positive` |
| P05 | permisos | `model.py` | metodo sin use extension | killed | `M02.diagnostic` |
| L01 | literal | `literal.py` | offsets en escalares | killed | `G0.literal.H01` |
| L02 | literal | `literal.py` | {{ como Hole | killed | `G0.literal.L03.no_syntax_error` |
| L03 | literal | `literal.py` | }} sin escapar | killed | `G0.literal.X02.positive` |
| L04 | literal | `literal.py` | CR crudo aceptado | killed | `literal.raw_cr_is_excluded_LF03` |
| L05 | literal | `literal.py` | recuperacion tras llave suelta | killed | `literal.recovery_yields_one_diagnostic_after_a_stray_brace` |
| B01 | barrera | `barrier.py` | K08 antes que K02 en un empate | killed | `G0.barrier.PB-TIE` |
| B02 | barrera | `barrier.py` | dependencias K sin validar | equivalent | `(sobrevive al selftest completo)` |
| B03 | barrera | `barrier.py` | primario de literal omitido | killed | `G0.barrier.N13` |
| C01 | consola | `app.py` | cola vacia como EOF | killed | `APP-V11.run` |
| C02 | consola | `app.py` | Println Err con stdout | killed | `APP-V09.run` |
| C03 | consola | `app.py` | bytes sin mock-io: | killed | `APP.run` |
| C04 | consola | `app.py` | CR aceptado | killed | `env.rows_E01_E06` |
| C05 | consola | `app.py` | surrogate suelto reemplazado por U+FFFD | killed | `env.rows_E01_E06` |
| C06 | consola | `app.py` | frontera del descuento | killed | `app.case.boundary_discount` |
| C07 | consola | `app.py` | IVA redondeado hacia arriba | killed | `APP.run` |
| C08 | consola | `app.py` | lee la linea 101 | killed | `APP-100.run` |
| C09 | consola | `app.py` | fin insensible a mayusculas | killed | `app.case.fin_is_case_sensitive` |
| C10 | consola | `app.py` | media redondeada | killed | `APP.run` |
| C11 | consola | `app.py` | dominio de importe | killed | `app.case.max_total_fits_i64` |
| Q01 | cuotas | `charges.py` | string_bytes en la frontera | killed | `charges.matches_model_random` |
| Q02 | cuotas | `charges.py` | text_allocations en la frontera | killed | `charges.matches_model_random` |
| Q03 | cuotas | `charges.py` | las cadenas de un bucle no mueren | killed | `K8.2.strings_of_a_loop_body_die_at_the_end_of_each_iteration` |
| Q04 | cuotas | `charges.py` | observed de miembros | killed | `charges.members_matches_model` |
| H01 | arnes | `runner.py` | exit 0 con divergencia | killed | `harness.corruption.diag:schema_version` |
| H02 | arnes | `runner.py` | known_issue sin caducidad | killed | `harness.known_issue.expires_after_max_phase` |
| H03 | arnes | `runner.py` | timeout cuenta como PASS | killed | `harness.corruption.timeout` |
| H04 | arnes | `runner.py` | expected_sha256 corrupto aceptado | killed | `harness.record_wrong_expected_sha256_rejected` |
| H05 | arnes | `runner.py` | debug y release no se comparan | killed | `harness.corruption.debug_release` |
| H06 | arnes | `generic.py` | tipos JSON 1/true/"1" equivalentes (pin AST y arnes) | killed | `audit.static` |
| H07 | arnes | `runner.py` | Err(i64) solo como cadena | killed | `harness.observed.err_as_integer_equals_decimal_string` |
| H08 | arnes | `runner.py` | orden parcial de claves con codigo None | killed | `harness.mixed_family_summary.stdout_extra_line` |
| H09 | arnes | `runner.py` | campo extra de un diagnostico ignorado | killed | `harness.corruption.extra_diag_field` |
| H10 | arnes | `runner.py` | clave extra de primer nivel ignorada | killed | `harness.corruption.extra_top_key` |
| H11 | arnes | `cli.py` | literales de K desactivados por defecto | killed | `cli.literals_on_by_default` |
| L06 | literal | `literal.py` | % fuera de los operadores de un Hole | killed | `K11.application_literals_recognised_with_their_holes` |
| L07 | literal | `literal.py` | escape \t decodificado como espacio | killed | `D15.2.4.escape_decodes.116` |
| D01 | diagnostico | `diagnostics.py` | evidence de la cadena causal | killed | `K10.5.cause_evidence_is_verified` |
| Q05 | cuotas | `model.py` | limite string_bytes del evaluador | killed | `K8.2.string_bytes_65536_passes_65537_faults` |
| Q06 | cuotas | `model.py` | limite live_text_bytes del evaluador | killed | `K8.2.live_text_bytes_262144_passes_262145_faults` |
| R01 | texto | `model.py` | s[a..<b] sin range/later_adr | killed | `K3.string_range_selector_is_range_later_adr_alone` |
| R02 | texto | `model.py` | compuesta += sobre String aceptada | killed | `K3.compound_plus_on_String_is_type_mismatch_I64` |
| R03 | nominal | `model.py` | Print de un recurso aceptado | killed | `K5.2.Print_of_a_resource_keeps_type_mismatch_not_E1403` |
| R04 | nominal | `model.py` | AssertEq sin exigir PartialEq | killed | `K5.2.AssertEq_requires_PartialEq.nominal` |
| S01 | literal | `model.py` | Holes evaluados de derecha a izquierda | killed | `A07.outcome` |
| B04 | barrera | `barrier.py` | barrera elige la ultima construccion | killed | `G0.barrier.PB-TIE` |
| B05 | barrera | `barrier.py` | clase B admite varias K pendientes | killed | `LF-02.class_B_needs_exactly_one_pending_K` |

## Equivalentes

- **B02** (dependencias K sin validar): Las dependencias de K 12 (K02<-K01 ... K10<-K06+K07+K09) ya las imponen las listas de cada fase (debe contener / puede contener de plan 2.1): todo manifiesto que las viola incumple tambien una de esas listas y falla con la misma ManifestError (exit 2); solo cambia el mensaje, que ningun literal compara. Comprobado: el `selftest` completo sobre la copia mutada pasa (148.3 s).

## Brechas conocidas (sin programa citable)

- `not` como operando de un Hole (`KEYWORD_OPERAND`) y `string_copy` en `compares_primary` (primario de barrera, LF-01): sobreviven los mutantes N09 y N15 de D porque ni K ni A2 ni F0-0005 dan un programa o un esperado (`and/or/not` están pendientes con L02). Ver LF-30.
- `severity`, el `stage` de `NX-E1105` y el namespace de `Format` solo los matan el SHA de corpus (`gen.corpus_sha`) y las filas de K de forma indirecta.
- Préstamo de Hole tardío: el modelo no lleva préstamos en ejecución y no hay sustitución textual limpia (LF-29); el orden de Holes lo mata S01 (A07).
