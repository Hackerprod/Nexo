# Validación B del plan r13 (Bloque II, S3) — ronda 1

Base: `git archive HEAD` (da14dc8; types.rs idéntico al de efe299f) en D:\nexo-work\val-B-S3\src; worktree no modificado.

## Veredicto: FAIL (3 motivos de plan, comprobables)

### Motivos
1. **Cifra errónea: SHA-256 del blob base.** El plan (§1 r13, "Blob base") da `5dbb551f…cb5c4`.
   `git show HEAD:src/check/types.rs | sha256sum` y `git show efe299f:src/check/types.rs | sha256sum`
   dan `c8fbb9e66c2bfe00299eff5a744f2172affb11e2f523dcd1279d836d3fd132db`
   (también el de D:\nexo-work\i07c-a-s3). `5dbb551f…` no coincide con ningún blob del historial de types.rs.
   Esta cifra es el ancla de las mutaciones inversas con restauración por SHA-256. Corregirla.
2. **Inventario de recursiones incompleto: falta `require_union` (types.rs:3381).**
   `bind` llama a `require_union` (3729) y `require_union` llama a `bind` por payload (3422): ciclo
   dependiente de profundidad `bind -> require_union -> bind`. El análisis de grafo de llamadas de
   types.rs (no tests) da la SCC {bind, require_union, union, union_inner}; la tabla solo cubre
   bind/union_inner/union. La prueba estructural ("rg de llamadas propias en las funciones de la tabla")
   no detectaría esa arista. Añadir la fila (3381/3422, atomicidad, cargo `charge_solver` 3387, orden
   payload Ok→Err) al inventario, al mecanismo (marco "par bind") y al criterio rg/indirecto.
3. **Contradicción sobre el cursor de visita (motivo 2).** El plan afirma que los marcos S3 "transmiten"
   `VisitCursor`, pero los `bind` anidados nacidos en `union_inner` (4747/4757/4771/4787) y en
   `require_union` (3423) usan `&VisitCursor::default()`, no el `visit` del llamador; solo la recursión
   directa de `bind` (3732/3807/3863) lo propaga. Como `defer_shape_conflict` estampa
   `numeric_domain_owner` desde ese cursor (4165, 4325…), un marco único que herede `visit` en todos
   los casos cambia el owner. El plan debe fijar qué cursor lleva cada marco de origen (default para los
   que hoy lo usan; el de la visita para los demás) y una regresión que lo distinga.
4. **Esperados no transcritos del ADR (hueco en pruebas).** El plan dice "B coteja los esperados
   literales con F0-0002 §§4.1.1–4.1.2.3, 13.15–13.17 antes de C", pero de los negativos de núcleo solo
   aparecen valores que son salidas de la base (`NX-E0102/type_mismatch [21,274)`, `NX-E0113 [16,20)`),
   que el propio plan dice que no son oráculos. Para igualdad profunda incompatible, componente recursivo
   sin solución finita y forma Result/Option conflictiva no hay fuente ni esperado (código, subcausa,
   primario, token, data). B no puede cotejar nada. Incluir en el plan fuente + esperado + sección del
   ADR por cada negativo/positivo.

### Comprobaciones que PASAN
- Medición reproducida (debug, TYPES, hilo de 1 KiB de resolución): Some×84 mínimo 321; Some×126 449;
  Array×126 193; `let x:Bool=` Array 257; `None[` 449; match Option/Result/patrón 193; todos pasan a 204
  los de 193 y los cuatro (84/126/Bool/None) fallan a 204 y 256. Release: Some×126 129, Array 65,
  None 129. CHECKER por CLI con `NEXO_STACK_MINIMUM_CASE`+`NEXO_PROBE_CHECKER_KIB=256`: desborda en
  Some84, Some126, Array-Bool, None; pasa Array126 y match. Coincide con la tabla.
- Frontera CLI debug y release: Array/Some 126 aceptan; 127 `NX-E0003/parse/syntax_nesting`
  limit 128 observed 129; firma 127 acepta, 128 rechaza; Generator §10.4 con 127 Some acepta y 128
  rechaza `types/value_type_depth` 128/129. Conforme a F0-0003 §10.4 y §13.2.
- Números de línea citados (inventario y llamadores) existen y se corresponden. Las recursiones
  directas (unresolved_union_paths, type_shape_hint, shape_hints_conflict, require_copyable,
  concrete_with_depth, resolved_syntax_type, selection_type, result_shape_conflict,
  resolved_join_side, syntax_type, from, bind, unify/union_inner) están todas; las iterativas
  (inferred_array_depth, check_depth_now, compound_depth, pretty) lo son.
- Clasificación núcleo/andamiaje, 204 operativo vs 256 normativo (F0-0003 §13.2), archivos permitidos,
  orden en un commit, diferencial 369+4.800 (cifras coinciden con el informe m2), puertas, mutación
  inversa y no uso del override <256: coherentes con AGENTS.md y §§1/7 del plan. Ninguna parte exige
  decisión de laguna ni cambia la promesa de Nexo.

### No bloqueante
- El generador de fuzz congelado y las 369 fuentes viven en D:\nexo-work\val-D1; el plan no dice dónde.
- `PartialEq/Clone/Drop` derivados de InferTy/Type quedan "si la sonda lo revela": criterio no mecánico.
- `src/check/i07c_quota_campaign.rs` (incluido en types.rs) no figura en archivos permitidos.
- La fila S3 del Bloque II lista como "a sustituir" funciones que r13 declara ya iterativas.
