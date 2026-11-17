# I07c — validación B de la enmienda de plan r7 (Flujo B, paso 2)

Alcance: registro r7 del plan (sección final), `I07c-Bloque-I-r1.md`, informes D1 (`I07c-D1.md`, `I07c-D1-r1-propuesta.md`, `I07c-D1-r1-propuesta-2.md`). Solo lectura y grep; sin compilación ni sondas.

## Veredicto: FAIL (un defecto de alcance, comprobable y de corrección mínima)

### Motivos accionables

1. **La lista de archivos de S2 no es suficiente: falta `tests/explain_and_print_output.rs`.** La prueba `explain_json_entries_are_comma_separated_in_a_single_object` (línea 152) ejecuta `nexo explain NX-E0102 --format json` y hace `assert_eq!(entries, 4)`. `CATALOG` de NX-E0102 tiene hoy exactamente 4 subcausas (`type_mismatch`, `not_indexable`, `resource_required`, `field_requires_grouped_call`; `src/diagnostic.rs` ~2811-2832). Al añadir `NX-E0102/constructor_shape` (obligado por F0-0003 §8.2 y por r7) el recuento pasa a 5 y esa prueba falla: `scripts/validate.sh` no pasa y C tendría que editar un archivo no autorizado o dejar el árbol roto. Acción: añadir a la tabla de r7 `tests/explain_and_print_output.rs` con su razón (el recuento de entradas de `explain NX-E0102` pasa de 4 a 5, sin otro cambio) y rectificar la frase «No hay otro archivo de catálogo/explain/serialización que ampliar». Las demás pruebas de ese archivo cuentan entradas dinámicamente o usan `not_indexable`/`type_mismatch`, sin conjunto cerrado.

### Comprobado (PASA)

- (1) Resto de la lista: `rg 'constructor_type|constructor_arity'` en `src tests scripts` da `src/check/types.rs`, `src/diagnostic.rs`, `tests/i01_diagnostic.rs`, `tests/j01_profile.rs`, `tests/j03_match.rs`, `tests/j04_match_semantics.rs`; `j03_match.rs` solo tiene casos concretos (125-149), correcto excluirlo. En `src/diagnostic.rs`: `validate_data` (1051-1060; NX-E0102 comparte rama con NX-E1303 y NX-E1325 solo distingue `constructor_arity`), `subcause_description` (2162-2165), `published_source` (2506-2510) y `CATALOG`. `tests/i01_diagnostic.rs` tiene listas (NX-E0102 en 452 y 680; NX-E1325 en 646 y 874) y el generador de Data (999). `EXPECTED.len()==73` cuenta códigos y no cambia. `F0-CODIGOS.md` (filas NX-E0102 línea 53, NX-E1325 línea 117) es necesario por su regla de cabecera; los tests que lo leen solo comprueban perfiles de cuota. No abre la mano más allá: `lower.rs`, `lower_match.rs`, F0-0003, F0-0002 y `docs/baseline/` siguen excluidos.
- (2) Un commit por solución: r7 mantiene S1→S2 y catálogo+emisor en el mismo commit `I07c S2`; la migración de cursores vive en `src/check/types.rs` (archivo de S1) y sus pruebas, sin colisión con la lista ampliada.
- (3) Criterio de cursores (`I07c-Bloque-I-r1.md` §3): coherente con la tabla de §4 del plan (filas `tainted` y `numeric_domain_owner` = D, «desaparece como campo lateral... transfiere sus hechos al grafo»). Exige cero campos en `Checker` e `InferenceGraph`, lectura decisoria solo en `OccurrenceFlag::Tainted`/obligación y en `PendingDeferredJoin.numeric_domain_owner`, y cierra la escapatoria del «efímero». Verificable por grep y lectura de alias; más estricto que la tabla, no más laxo.
- (4) Prohibiciones: `lower.rs` solo con traza nueva y A/B (sin cambio). La excepción sobre `docs/adr/F0-CODIGOS.md` contradice literalmente la frase de §6 «No se toca ... el ADR en la vía (a), salvo decisión de laguna por Flujo A», pero r7 la declara, la acota a `constructor_shape` y las identidades ya constan en F0-0003 §8.2 (888-893), decididas por Flujo A. Aceptable; ver nota 1.
- (5) Salidas fijadas: los esperados proceden de F0-0003 §3.2/§8.2 (offsets recalculados por D1); el diferencial de 369 fuentes más fuzz 1-5 contra `9b8e03e` limita las diferencias a `match` sobre raíz abierta.

## No bloqueante

1. Al integrar r7, retocar la frase de §6/§8 («no se toca el ADR») para que remita a la excepción de F0-CODIGOS y no queden dos reglas literalmente opuestas.
2. D1 propone congelar el SHA-256 de `D:\nexo-work\val-D1\h\fuzz.py`; recomendable.
3. `docs/validation/i07c-mutations/population.json` cita `constructor_type`; es población de mutación (andamiaje), sin cambio necesario.

Corregido el motivo 1, la enmienda es validable (PASS) sin nueva exploración.

## Ronda 2 (B conserva contexto)

Veredicto: **PASS**.

- El motivo 1 queda cerrado: la tabla de r7 incluye `tests/explain_and_print_output.rs` (plan, fila de la línea 305) con el cambio 4→5 de `explain NX-E0102`, acotado al recuento.
- Búsqueda de aserciones de recuento de entradas o subcausas de NX-E0102/NX-E1325 en `tests/` y `scripts/`: solo `explain_and_print_output.rs:152` depende del número de subcausas. `tests/cli.rs` (141-248) cuenta diagnósticos emitidos en stderr, no subcausas. `tests/i01_diagnostic.rs:891` (30) corresponde a NX-E0006 y el 910 (73) cuenta códigos. No falta ningún archivo análogo.
