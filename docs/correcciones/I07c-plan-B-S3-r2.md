# Validación B del plan r13+r14 (Bloque II, S3) — ronda 2

Base: `git archive origin/i07c-estructural-r2` (0beb4ac; types.rs idéntico a efe299f, solo cambia documentación).
Fuentes del §4 de r14 ejecutadas con binario de la base (debug) en D:\nexo-work\val-B-S3\r2.

## Veredicto: FAIL (2 defectos de plan comprobables)

### Motivos
1. **Oráculo erróneo en la fila "Igualdad compuesta incompatible", extensión profunda (r14 §4).**
   r14 dice que `data.expected_type` es `Array<`×126+`Bool`+`>`×126 y `data.found_type` es
   `Array<`×126+`I64`+`>`×126, citando §4.1.2.6. Pero el primario es la hoja `1` `[401,402)` y
   F0-0002 §4.1.2.6 dice: `data.found_type` es «el tipo completo de la expresión señalada como
   encontrada» y «Descomponer una igualdad ... no autoriza a serializar el tipo del elemento con el
   span del array completo». La expresión señalada es `1`: `found_type=I64`, `expected_type=Bool`,
   igual que el caso literal de la propia fila (`[26,27)`, Bool/I64, ADR §4.1.2.2 «Gemelo negativo con
   tipo adelantado»). La base da exactamente `{"expected_type":"Bool","found_type":"I64"}`,
   relaciones `expected_type=[16,272)`, `found_type=[401,402)`. Un implementador correcto fallaría el
   test tal como está escrito. Corregir los dos campos `data` a `Bool`/`I64` (conservar offsets).
2. **Identificador de perfil inexistente en la CLI (fila `type_solver_steps`).** r14 usa
   `f0-0002-test-types-32-v2`. `nexo check <fuente> --profile f0-0002-test-types-32-v2` da
   `outcome.kind=driver_error, reason=unknown_profile`. El identificador B de CLI es
   `f0-0002-b-test-types-32-v2` (F0-D15 §6.1; F0-CODIGOS). Con ese perfil, 64 bindings anotados dan
   `NX-E0003`, `types`, `analysis.limit`, `type_solver_steps`, limit 32, observed 33 (primario
   [233,236), no normativo, como dice el plan). Corregir el nombre.

### Los 4 motivos de la ronda 1
- Motivo 1 (SHA-256): RESUELTO. `c8fbb9e6…132db` es el hash del blob (comprobado en
  origin/i07c-estructural-r2 y efe299f). Nota: r14 atribuye el valor viejo a PowerShell; no lo pude
  confirmar (5dbb… no es el blob ni su variante CRLF). No bloquea.
- Motivo 2 (`require_union`): RESUELTO. La fila 3381/3387/3422–3423 y el ciclo
  `bind→require_union→bind` coinciden con el código; el criterio pasa a SCC de grafo de llamadas
  (mi análisis de SCC da exactamente {bind, require_union, union, union_inner} y {unify,
  unify_with_expected_origin}, más autollamadas).
- Motivo 3 (cursor): RESUELTO en el plan. Líneas verificadas: bind propaga `visit` en 3732/3807/3863;
  `VisitCursor::default()` en 3423, 4748/4758/4772/4788, 2831/2850, 5843, 6817. RD71 (ADR §14.10, tabla
  RD; r14 cita §4.1.2.5 solo para la razón) expandido da `NX-E0102/type_mismatch` [136,138) único
  (ADR y base). Las fuentes discriminantes de los arcos require_union/union_inner las deja a C con
  cláusula de parada explícita: límite declarado, aceptable.
- Motivo 4 (esperados): PARCIAL; ver motivo 1 de arriba. Verificado contra ADR y base:
  - Bool/Array×126 `[21,274)`, relaciones `[16,20)`/`[21,274)`, `found_type` completo (len 885); positivo acepta.
  - `None` `[16,20)` E0113 `$.some` (F0-0003 §9.1 O01 + tabla de data); positivo acepta.
  - `[true]`/`[1]` `[26,27)`, rel `[16,22)`: literal del ADR §4.1.2.2. Extensión: offsets 401/402/272 correctos.
  - `var r=[];r=[r];` NX-E0103 `[16,18)`, `data={}` (ADR §13.17 y tabla §4.1.2.2).
  - Result/Option `[106,108)`, `[66,73)`, `[90,105)`: ADR F0-0003 §3.2 (variante Result<I64,I64>). Anidado E1325 `[89,94)`: literal del ADR.
  - Generator 127 acepta/128 `value_type_depth` limit 128 observed 129 (ADR F0-0003 §10.4).

### No bloqueante
- Sigue la mención del hash viejo en la sección r13 (línea 392); r14 la sustituye expresamente.
- La fuente discriminante de cursor no está escrita en el plan (límite declarado).
