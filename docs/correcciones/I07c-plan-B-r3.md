# I07c — validación B del plan, ronda r3 (paso 2.2)

Plan: docs/correcciones/I07c-plan.md (subsección r3 del registro). Evidencia propia por script y lectura de types.rs.

## Veredicto global: PASS

## N1, inventario de campos de Checker: RESUELTO
- Extraje los identificadores de `struct Checker<'a>` (types.rs:1422–1508) y los comparé con la tabla del plan: 66/66, sin ausentes y sin duplicados (el diff solo marcó `errors` porque mi rango de extracción de la tabla lo cortó; está en la última fila).
- Clasificaciones D/I/Q revisadas una a una y razonables. Los de uniones y formas que hoy influyen (`deferred_join_roots`, `recursive_join_roots`, `branch_join_sites` como D; `eligible_shapes`, `nested_assignment_sites`, `preseeded_children` como I) quedan expresamente condicionados: el plan reconoce que hoy afectan formas y conflictos (usos en 1914, 2090, 3002, 3457, 5830, 5868) y exige que S1 los migre al grafo antes de alegar que son índices. D tiene una búsqueda estática precisa que declara FAIL toda rama residual que decida por un índice lateral.
- Las ocho variantes de PendingConstraint (568–578) siguen correctas.

## N2, ir1c/ir1d y autorreferencias: RESUELTO
- §2.1 retira «aceptar» de ir1c, ir1d y b1a-false (`r=[false]` en N(f)); la fila «Actual» de ir1c/ir1d (`checked`) coincide con lo medido en la base.
- §7 y §8.3: antes del ADR solo N(f); un rechazo provisional se documenta como «laguna abierta», nunca como PASS. No se afirma ninguna decisión de §8.3; la observación de que existe solución finita Array<Bool> es descriptiva y consistente con la propuesta aparte.
- El plan referencia sin duplicar `docs/correcciones/I07c-laguna-8-3.md` (existe; estado PROPUESTA REVERTIBLE, aún sin validar). Dependencia correcta: S1/S2 avanzan en fuentes independientes y el cierre de S2 exige la decisión.
- Tests adr_gap_*: 41 definiciones en src y tests, las 41 nombradas en el plan.

## Defectos nuevos: ninguno bloqueante

No bloqueantes registrables:
1. La fila 1 del registro r2 conserva el texto antiguo sobre ir1c/d; ya lleva la nota de que r3 lo sustituye.
2. La propuesta de §8.3 debe tener PASS de B y decisión del coordinador antes de fijar oráculos; el plan ya lo exige.
