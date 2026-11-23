# F4 oráculo: hallazgos

Registro de defectos encontrados al construir y probar el oráculo, con su causa, la evidencia y el estado. Un defecto del **producto** se registra aquí con su fuente, el esperado y el obtenido, y entra en `tests/fixtures/f4_oracle_known_issues.json` (solo lo crea el coordinador o D tras cita de K y repro); no cuenta como defecto del oráculo.

## Producto

Ninguno. El producto no implementa F0-0004 y no hay binario contra el que contrastar (ver `COBERTURA.md`); `f4_oracle_known_issues.json` está vacío. El primer contraste con un binario K abrirá aquí los hallazgos, cada uno con su cita de K y su repro mínimo; nunca se ajustará un esperado para hacerlo pasar.

## K (ambigüedades y lagunas)

Las lagunas de K se registran en `LAGUNAS.md` (LF-01–LF-25) con su efecto sobre el oráculo y su estado. Las dos que el coordinador debería llevar al Flujo A antes de comparar la familia afectada son LF-03 (carácter de control crudo dentro de un literal) y LF-24 (primario de R0002 cuando falla la copia de un String).

## Oráculo

| ID | Hallazgo | Causa | Evidencia / regresión | Estado |
|---|---|---|---|---|
| O-01 | `compare` exigía un único `--profile` por corrida; las recetas Q ejecutan los cuatro perfiles de prueba de K §8.3 | diseño de O4 (un perfil por corrida) | `G9.harness.foreign_profile_refused`, `G9.harness.faithful_exit0` (registros con perfil de prueba) | corregido en O5 (desvío 23) |
| O-02 | El ficticio contestaba por SHA-256 de la fuente: no podía distinguir dos entornos para el mismo programa (K §11 tiene doce) | diseño de O4 | `G9.harness.same_source_two_envs` | corregido en O5 (desvío 25) |
| O-03 | El error del controlador (E01–E06) no tiene forma completa fijada por K; compararlo con la forma de un resultado de programa daba falsos positivos | K fija exit 2 y la razón; A2 §12.4 solo el `outcome` | `harness.observed.controller_document_is_compared_on_its_own_keys`, `harness.observed.controller_reason_is_compared` | corregido en O5 (LF-22) |
| O-04 | La diversidad estructural de la clase A (22–28 formas por semilla) quedaba bajo el mínimo del plan (30) | la forma contaba solo el conjunto de clases de línea | `corpus.A.structural_minimums` (≥ 33 en G9, ≥ 63 en G10) | corregido en O5 (forma con primera y última clase, longitud y escritura) |
| O-05 | La primera pasada de mutación dejó 15 supervivientes: huecos del `selftest`, no defectos del modelo (orden de evaluación de operandos, `IsEmpty`, `Concat`, asociación posicional, limpieza parcial, ciclo mínimo, `Format`/`PartialEq`, CR crudo, recuperación tras llave suelta, vida de las cadenas de un bucle, `Err(i64)` entero) | los esperados de esas reglas solo estaban en filas que no las separaban | `K3.*`, `K4.*`, `K5.2.*`, `K8.2.*`, `literal.*`, `harness.observed.*` (todas con esperado de K) | corregido en O6 |
| O-06 | `gen.corpus_sha` marcaba como sobrantes las entradas pinadas de las clases A, E y Q | el cotejo solo conocía V, N, L, B | `gen.corpus_sha` incluye `special_entries()` | corregido en O5 |
| O-07 | La equivalencia de B02 (dependencias de K ya implicadas por las listas de fase) la refutaba la propia comprobación `mutate.catalogue_valid` ejecutada sobre la copia mutada | el selftest completo de una copia mutada ejecutaba la campaña sobre sí misma | `F4_ORACLE_MUTANT=1` omite el grupo `MUTATE` en copias mutadas; B02 sobrevive al `selftest` completo | corregido en O6 |
| O-08 | `compare` ejecutaba las filas literales de K solo con `--with-literals` (falso PASS de un `gate` por defecto) | opt-in en O4/O5 | `cli.literals_on_by_default`, mutante H11 | corregido en O7 (D'-1) |
| O-09 | `TypeError` en `sorted(seed_matched)` al mezclar códigos `None` y cadena | clave de ordenación parcial | `harness.mixed_family_summary.*`, mutante H08 | corregido en O7 (D-1) |
| O-10 | Campo extra de un diagnóstico o clave extra de primer nivel pasaban `compare` (exit 0) | `observed` filtraba a las claves esperadas | `harness.corruption.extra_diag_field`, `extra_top_key`, mutantes H09, H10 | corregido en O7 (decisión del coordinador) |
| O-11 | Sobrevivían `%` fuera de los operadores de un Hole, `evidence` de la cadena causal y el límite `string_bytes` | casos sin esperado de K | `K11.application_literals_recognised_with_their_holes`, `K10.5.cause_evidence_is_verified`, `K8.2.*` | corregido en O7 (D-2..D-4) |
| O-12 | El modelo aceptaba `s[a..<b]`, `+=` sobre String, `Print`/Hole de recurso y `AssertEq` sin PartialEq; COBERTURA anunciaba `s[a..<b]` sin existir | reglas no modeladas | familias `n_string_slice_range`, `n_string_compound_plus`, `n_print_resource`, `n_hole_resource`, `n_asserteq_no_partial_eq`; mutantes R01–R04 | corregido en O7 (D'-2, D'-brechas) |
