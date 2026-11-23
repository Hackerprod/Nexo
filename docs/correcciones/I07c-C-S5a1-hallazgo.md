# I07c-S5a.1 — hallazgo de C: la cláusula `failed` del veredicto de asignación cambia la procedencia de `expected_type`

**Estado: hallazgo de implementación sobre el plan (`I07c-S5-plan.md` §3, mecanismo V6, punto 2). El código del árbol sigue el plan al pie de la letra; C no se aparta de él.** Base: `2875943` (blob `src/check/types.rs` SHA-256 `54c9dfedb4c6909bba132e8889a791a88d6d9cfc7929d7bbc2e4bb6490cafd93`); candidato: blob `07959d3ff9ec01bb18bf04e8ff353cc062bd1b51323ffc7d314e859728e8778a`. Proceso: este corte lo implementa un agente Sonnet por desvío por presupuesto (fondo de Codex agotado).

## Qué dice el plan y qué hace el código

El plan §3 (2) instala `read_plan` solo si la pasada de planning aceptó la asignación que adelanta, y define el veredicto `rejected_assignments` como «un RHS `Err` o una asignación que dejó un `PendingDeferredJoin` nuevo». `note_assignment_verdict(place, failed)` implementa ambas ramas: `failed || join_count_grew`.

## Defecto reproducible (núcleo: procedencia de `expected_type`)

Programa P1 (113 B, perfil `f0-0002-b-v2`, `check` y `run` rechazan igual):

```
fn Main(){let ar=[1];var r=[];var q=[];let a=((if true then true else q)==ar);Print([r,q]);q=[[true]];r=[];q=ar;}
```

Los tres diagnósticos coinciden en código, primario y tipos entre `main` y el candidato (`NX-E0103 [27,29)`, `NX-E1303 [70,71)`, `NX-E0102/type_mismatch [109,111)` `ar`, `Array<Array<Bool>>` / `Array<I64>`). Difiere **solo** la relación `expected_type` del último: `main` la sitúa en `[93,101)` (`[[true]]`, la asignación `q=[[true]]`); el candidato en `[85,86)` (el `r` de `Print([r,q])`).

Esperado según el ADR: §4.1.2.2 («la restricción que fija un componente es la primera igualdad compatible que, junto con las precedentes, determina su tipo») y la fila del addendum «Ancla con componente pendiente y asignación posterior» (`expected_type` = origen de la primera igualdad compatible que fijó el componente, como `aar` en el repro A). `[r,q]` solo iguala dos incógnitas; el elemento de `q` queda fijado por `q=[[true]]` (`[93,101)`). El valor de `main` es el correcto; el del candidato es un retroceso de procedencia.

Programa P2 (reducido, 137 B):

```
fn Main(){let x=1;let p=true;let ar=[1];var r=[];var q=[];let a=(if p then [r,q] else (if (x<1) then r else [true]));q=[[3]];q=ar;r=[r];}
```

`NX-E0102 [127,129)` (`ar`, `Array<Array<I64>>` / `Array<I64>`): `main` relaciona `expected_type=[119,124)` (`[[3]]`); el candidato `[76,77)` (el `r` de `[r,q]`). Mismo mecanismo y misma derivación.

## Causa comprobada

Mutante «sin la rama `failed`» (`M2d`, `docs/validation/i07c-s5/s5a1-mutate.py`: `&& (failed || …)` sustituido por `&& before.is_some_and(|before| self.pending_join_count() > before)`): coincide con `main` en P1 y P2, es **idéntico al candidato** en las 3.674 fuentes del diferencial congelado (0 diferencias, debug), pasa las 55 pruebas del corte y se distingue del candidato solo en 2 de ~72.000 programas aleatorios de búsqueda exploratoria (los dos de arriba; semillas 1–10 de `.scratch/search.py`, no versionado). La asignación rechazada por conflicto concreto (`q=ar`) añade el sitio a `rejected_assignments`; sin `read_plan` en ese sitio, la relación pasa a salir de la capa sembrada (origen `[r,q]`) en lugar del origen de la asignación que fijó el componente.

## Alcance y qué no cambia

- Ningún repro ni regresión del plan lo exhibe (A, B, F06, M2, O01, O03, O07, O08, D-REF, protecciones, corpus 26+1): por eso la suite verde no lo detecta.
- Hay también diferencias de **solo procedencia** frente a `main` (código, primario y tipos iguales; cambia una relación) en 2 de 8.000 programas aleatorios (semilla 21). En el ejemplo reducido `fn Main(){let p=true;let ar=[1];var r=[];var q=[];let a=[q,(if p then r[0] else [r])];q=[[ar]];r=[[3]];}` el candidato relaciona el `3` con `[88,94)` (`q=[[ar]]`) frente al `[57,58)` de `main`; por la misma derivación el candidato parece más cercano al ADR ahí. No es una regresión inequívoca: se registra para que D lo derive de forma independiente.

## Decisión que corresponde a A/B (no a C)

Quitar `failed ||` del veredicto es una línea y no cambia ninguna salida medida del corte, pero contradice el texto literal del plan (§3 punto 2: «un RHS `Err`») y quedaría fuera de lo validado. C no lo aplica. Si A/B lo aprueban: añadir P1 y P2 como regresiones (esperado derivado arriba) y regenerar el diferencial. Si no, registrar P1/P2 como límite conocido con fecha (`Pending.md`).

## Otros hallazgos de la medición (no bloquean el corte; detalle en el informe)

- F11 ya no conserva los 2 `NX-E0103` del plan (§9) en esta base (`main` actual da 0); solo F10 mantiene su residuo (2 `NX-E0103`, idéntico a la base).
- Reproducción mínima del residuo FAM-E adicional (idéntico en base y candidato, 101 B): `fn Main(){let p=true;var r=[];var q=[];let a=(if p then r[0] else (if p then q else r));r=[3];q=[3];}` da dos `NX-E0103` (`[27,29)`, `[36,38)`) donde el ADR fija `α_q=I64` por `q=[3]` y no queda ningún constructor sin resolver. Es la familia de F10 (S5a.5).
