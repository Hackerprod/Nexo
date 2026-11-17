# I07c — validación B del plan r8 (raíz hipotética, frontera IR)

Alcance: «Registro de correcciones r8» del plan, sección «Mecanismo único de frontera con IR (r8)» de `I07c-Bloque-I-r1.md`, hallazgo de C. Lectura de código y sondas en `D:\nexo-work\tmp\b8` con binarios ya compilados (`val-D1/target/release`, candidato `9b8e03e`; sin compilar).

## Veredicto: FAIL (un motivo de mecanismo; el resto PASA)

### Motivo accionable

1. **El marcador `ResolvedType::Never` por procedencia en el checker es innecesario y no tiene programa que lo discrimine; el plan debe justificarlo o suprimirlo.** `lower.rs::resolved_type` (1701-1710, comentario incluido) baja `Never` y `Unresolved` al mismo `Ty::Never`, y `begin_match_selection_after_selected` recibe `selected_ty` ya colapsado (`resolved_expr`, lower_match.rs:418). `EmissionMode::Analyze` solo se activa en código sin continuación normal, donde ningún valor se produce; por tanto cortar con `selected.diverged() || (selected_ty == Ty::Never && Analyze)` basta, y es justo la condición que la sonda de A ya hizo pasar para el positivo principal, anidado, `Ok`, `Some/None`, `var`, alias, `match (l0)`, `let k=x`, `let k:Bool=x`, `let k:I64=x`. La propagación de procedencia por binding/alias en el grafo y el cambio de `expr_vars`/`bindings` añaden un mecanismo en `types.rs` (zona de la migración de contaminación, motivo 2) cuyo único consumidor sería comprobar la variante cruda por span, lo cual solo distingue `Unresolved` de `Never` en una ruta (Analyze) donde esa diferencia no cambia ningún resultado. Si se mantiene, A debe aportar un programa fuente que, con solo la condición `Ty::Never && Analyze`, acepte/ejecute mal o dé `InvalidIr`; sin él es código sin efecto observable (AGENTS, Flujo B 4: código muerto/rutas duplicadas) y su «mutación inversa» solo mata por construcción circular (la condición lee la variante). Acción: simplificar r8 al cortocircuito de IR en `lower_match.rs` más el registro de forma de S2, o aportar el programa.

### Comprobado (PASA)

- (1) Causa raíz: la traza de C coincide con el código (`Unresolved`→`Ty::Never`; `Flow::Value` en lectura de Place tras inicializador divergente; `match DV{...}` directo ya cae en `selected.diverged()`). Sondas: `match DV{Some(x)=>{ move r; move r }}` da `checked` (los brazos muertos no se analizan hoy), alias/`match (l0)`/uso de `x` dan `internal_error` en `9b8e03e`.
- (2) No inventa tipo ni depende de patrón/fuente: el cortocircuito es por flujo/modo, no por `Some`/`Ok`/`DV`. No oculta valor vivo: un valor vivo con α abierto ya lo rechaza el checker (`let l0=None;match l0{..}` → `NX-E0113`), y tras un inicializador divergente todo el bloque restante es código muerto (alias, `var` con asignación posterior, `match (l0)`, uso de `x`, lectura dentro de rama: ninguno puede producir valor).
- (3) Archivos: `lower_match.rs` necesario (única función), `lower.rs` fuera correcto, `src/ir/i07c_tests.rs` solo para regresión; mismo commit S2 que catálogo/emisor, coherente con r7 y «un commit por solución». r8 declara que sustituye la exclusión de `lower_match.rs` de r7 (aceptable).
- (4) Sin cambio de salidas fijadas: hoy estas fuentes dan `internal_error`; `check`/`run` y debug/release convergen porque el código es muerto. Diferencial de 369 fuentes + fuzz exigido.
- (5) Oráculo de F0-0003 §3.2/§8.2 se cumple sin laguna de IR: el ADR fija `checked`/`unit` sin completar α. La obligación de `None` antes de `Some` sigue siendo de `types.rs` (la sonda de A lo admite).

## No bloqueante

- La obligación de procedencia debería, si se conserva, no interactuar con `tainted`; no hay programa que lo exhiba.

## Ronda 2 — plan r9 (HEAD publicado `b9dc9cb`)

Veredicto: **PASS**. Se adopta la vía (a): el motivo 1 de la ronda 1 queda cerrado.

- Mecanismo: una sola condición en `begin_match_selection_after_selected` (`selected.diverged() || (selected_ty == Ty::Never && Analyze)`) más la obligación de patrón en `types.rs`; sin marcador, `lower.rs` fuera. `Live` no puede tomar el atajo, y `Analyze` es código sin continuación normal.
- Valor vivo no enmascarado: lo garantiza el cierre del checker. En `9b8e03e`, `let l0=None;match l0{Some(x)=>{},_=>{}}` da `NX-E0113`. Tienen negativo/positivo pareado (`None` frente a `Option<I64>`), prueba interna de `InferredTypes` por modo y mutación inversa de la denuncia de componentes pendientes (NX-E0113, no `checked`/`InvalidIr`).
- Alias, `var` con asignación posterior y `match (l0)` quedan cubiertos: todo es código muerto tras el inicializador divergente, y figuran en los positivos exigidos.
- Archivos (`lower_match.rs`, `i07c_tests.rs` solo para la prueba interna) y commit S2 único coherentes.

No bloqueante: la frase de r7 «`lower_match.rs` sigue fuera» queda sustituida por r9; conviene que C la trate como histórica.

## Ronda 3 — plan r10 (HEAD publicado `3729b33`)

Veredicto: **PASS** (lectura de código; no compilé: no hay binario de r10 en `D:\nexo-work`).

- Causa raíz del FAIL de D confirmada en el código: r9 devolvía `Completed` y salteaba `MatchNext`/cuerpos; el análisis de propiedad de `Analyze` está en esa ruta (`corpus/fail/unreachable_resource_copy.n` lo exige).
- r10 conserva `if selected.diverged()` y solo relaja las dos exigencias de descriptor en `match_tests` (1038) y `bind_match_pattern` (1105) para `Ty::Never` en `Analyze`. No hay flujo duplicado ni tipo inventado. `Live` conserva `InvalidIr("pattern union")` como barrera.
- Valor vivo irresuelto: sigue siendo NX-E0113 del checker (comprobado en `9b8e03e`), con par vivo y mutación (4).
- Alias, `var` con asignación y `match (l0)` pasan por la misma ruta.
- Archivos y commit S2 coherentes; `lower.rs` fuera; el negativo y positivo de D, y la mutación (3), discriminan el atajo.
- Salidas fijadas: los repros de D conservan la base.

No bloqueante (C vuelve a A/B si falla): en `bind_match_pattern` la distinción `None` variante / binding se hace hoy por `ty`; con `Ty::Never` pasa a ser «Name sin entrada en `InferredTypes.bindings` no es local». Debe cubrirse con test de binding usado (`Some(x)=>{let k=x;}`) y de `None` primero, aún pendientes del checker.
