# Validación B del plan I07c-S6 — ronda 2: **FAIL** (dos bloqueantes)

Base: HEAD `4b73f04`. Solo lectura; los repros de K1-d2 se hicieron en copias del validador.

## Bloqueantes

1. **K1-d2: la corrección mínima del auditor no basta para exit 0.**
   - Repro: copia de `scripts/i07c_span_audit.py` con aceptada solo la línea 7082 de `src/check/types.rs` (`primary_span`). El script falla de inmediato en `types.rs:8283` (`iterable.span().clone(),`) con exit 1.
   - Reproduciendo su lógica sobre todo el fichero (el script se para en el primero, así que se replicó aparte) quedan **12** accesos crudos de producción sin clasificar: 7082 y otros 11 (8283, 10957, 12072, 12301, 12874, 12984, 13195, 13200, 13375, 14613 y 14627). Esos 11 son código de F0-0003 posterior a la última actualización del script (commit `ce3f1da`).
   - El plan (§1, §3.3, fila K1-d2, §4.4) afirma que tras esa única clase E exige exit 0 y prohíbe relajar `else: raise`. Con esa autorización E fallaría desde el primer día.
   - A favor del diseño: el auditor sigue rechazando otros accesos (una `fn zz(e:&Expr)->Span{ e.span().clone() }` insertada en una copia da exit 1), y la prueba es implementable sin parametrizar el script, porque lee `parents[1]/src/check/types.rs` y basta copiarlo a un árbol temporal.
   - Acción: decidir en el plan una de dos salidas: (a) clasificar los 11 sitios restantes con clase y motivo por sitio (8283 es el primario de un NX-E0006 sobre `iterable`, así que puede ser un defecto real de grupos); o (b) no invocar el script desde E y cerrar K1-d2 solo con la prueba metamórfica de grupos.

2. **Arnés RD (§3.1.1 y §3.1.4): la clasificación «por fuente» de un D/V nuevo no es implementable con los datos que hay.**
   - El arnés no tiene AST ni etiquetas C/D/V públicas, como el propio plan admite. Reglas como «hoja incompatible adicional de la misma unión» o «depende de una expresión contaminada» exigen análisis semántico. El positivo «V nuevo de RD21» solo se clasificaría con una regla escrita para esa fuente concreta, es decir, un parche por patrón.
   - «Clasificación INCONCLUSA» no es un resultado de un test automático: el plan no dice si falla o pasa, así que C tendría que improvisar. Si falla, la presencia D/V (columna I) pasa de hecho a N.
   - Dato a favor: sobre `main` los primarios de las 152 filas coinciden con el fixture (152 de 152 en la comparación de B), así que hay base para una regla simple.
   - Acción: sustituir la clasificación semántica por un procedimiento decidible con los datos disponibles. Propuesta de B: un diagnóstico fuera del fixture pasa como delta I solo si cumple todo lo siguiente: `(code, subcause, operation)` está en el vocabulario de N(j) para esa operación; su primario es un sitio real, no vacío y único; su `data` valida la gramática de tipos serializables; no hay `expected_type` autorrelacionado; y los C del fixture se conservan exactamente. En cualquier otro caso el test falla pidiendo revisión independiente. El caso RD21 se retira como positivo clasificado por fuente o se expresa como caso sintético del arnés.

## Lo que pasa

- **P-8:** el perfil `f0-0003-b-v1` es el correcto (`src/ir/lower.rs:444,462` lo enlaza con `JNormal`). Con la fuente DV de D15, `check` y `run` dan un único NX-E1325/constructor_type, `types.check`, con `{constructor: prelude:Ok, scrutinee_type: Option<I64>}`, primario `[87,92)` y sin relaciones. El positivo `Some(n)` da checked/unit con canales vacíos. La deducción desde F0-0003 §3.2 es coherente y tiene una sola respuesta razonable.
- **N-a.3 PARCIAL y T-TD-ord INCONCLUSA:** el estado de cierre está escrito en §2, §3.3 y §4.7, y los nombres de test no prometen más de lo que fijan.
- **N-i.2:** la prueba N de relaciones (byte dentro de fuente, no vacío, `expected_type` distinto del primario) es implementable y no reclasifica I a N. La gramática de tipos del §3.1.3 cubre todos los valores de `data` de las 152 filas en `main`; ningún `expected_type` coincide con el primario.
- **No bloquea incorporados:** la tabla de mutación inversa por test (§3.5) cita funciones que existen en `src/check/types.rs` y `src/check/names.rs`; el destino de los tests que fallen en `main` está en §4.7; RD87(c) fija solo `stdout`; queda declarada la base `4b73f04` con el remoto adelantado. `scripts/i07c_span_audit.py` está en la lista de editables; `src/`, `tests/f0005_b1b.rs` y `scripts/validate*.sh` quedan fuera.

## No bloquea

- La mutación inversa de `repeated_check_and_run_keep_diagnostics_deterministic` puede resultar NO EVALUADA; el plan ya lo admite.
- Antes de C hay que revalidar blobs y renumerar referencias tras actualizar la rama remota `e479d62`.
