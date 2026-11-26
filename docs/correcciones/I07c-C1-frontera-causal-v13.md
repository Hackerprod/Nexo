# Frontera causal de C1 (propuesta para R6 v13)

Estado: **en validación**, sin veredicto de C1. Base `main` `31d91db`; contraejemplo ejecutable en `docs/validation/i07c-rediseno/C1-v12-full-gate-counterexample.md` y reporte íntegro `C:/nexo-work/evidence/luna-c1-corpus.json`.

## Decisión sobre el límite C1/C2

La frontera v12 «C1 sin `EqualityRecord` ni journal» queda retirada. C1 debe crear un `EqualityId` monotónico al comenzar cada igualdad fuente y conservarlo en toda igualdad estructural hija, en cada `PendingDeferredJoin`, en sus aristas tentativas y en las obligaciones recursivas alcanzadas. El ID identifica el intento causal, no el span, `FactId`, representante de variable ni constructor. Reintentar o completar una obligación conserva ese ID; otra igualdad sobre el mismo componente recibe otro. Un diagnóstico E0103 recursivo se reserva en la igualdad que cierra el ciclo y solo usa el origen de un deber de constructor alcanzado por sus operandos y registrado como dependencia de esa igualdad. La reserva no reescribe la obligación ni el origen de otra igualdad.

El mínimo de C1 incluye `EqualityRecord { id, operandos fuente, ordinal, dependencias, estado, joins, diagnostic_reservation }` y journal de primeras escrituras de aristas, forma y procedencia por `(node, campo)` durante el intento. Una igualdad incompatible revierte su journal y publica solo su reserva; una suspendida conserva su obligación por ID sin publicar aristas tentativas; una compatible publica las aristas de ese ID una vez. Las cuotas y los IDs monotónicos no se revierten. El drenaje procesa cada obligación con su dueño; deduplicar por span o raíz no puede sustituir la identidad causal. Esto implementa F0-0002 §§4.1.2.1–2 y 13.9 en las tres recursiones propias sin desplazar los E0103 de las diez filas no propias.

`ReadContext` por entrada/epoch, `OriginFact` general, `Completion` para asignaciones posteriores y migración de las vistas de planificación permanecen en C2, **solo si** el gate completo de C1 pasa. `ResultStatus`, `JoinRecord` de `if` y sus testigos pasan por C3, `ConstructorDuty` general y reservas C/D/V por C4, activación/replay y cuota por C5, retirada de flujo heredado y dos H por C6. Los deberes de constructor que C1 necesita para su igualdad recursiva son una vista mínima de propiedad en su `EqualityRecord`; C4 consolida el dominio general sin duplicar autoridad. Si el mínimo C1 no conserva las 151.188 filas, C1 y C2 se fusionan y **no se publica un commit C1 intermedio**.

La puerta sigue intacta: 20 corpus, 151.188 filas, 0 A/D/A-unmodeled/H nuevos, tres dirigidas propias exactas, F06 igual a `main`, 103 `match` iguales a `main`, `validate.sh`, pila y medidores. C2–C6 requieren cada uno el gate cruzado completo y sus filas propias en un prototipo **por commit real**; una lista de propietarios o una prueba parcial no acredita un corte. Ninguna cifra de esta tabla se declara medida para un commit aún inexistente.

| commit | gate completo OK/C/U/H; A/D/A-unmodeled | filas propias |
|---|---|---|
| C1 v12 `8820551` (falsado) | 62.755/16.671/71.750/2; 0/7/3 | 3/74, 0/45, 0/13 |
| C1 viable | **NO EVALUADO** | 3/74, 0/45, 0/13 requeridas |
| C2 | **NO EVALUADO** | 10/74, 0/45, 3/13 requeridas |
| C3 | **NO EVALUADO** | 27/74, 9/45, 9/13 requeridas |
| C4 | **NO EVALUADO** | 32/74, 13/45, 11/13 requeridas |
| C5 | **NO EVALUADO** | 74/74, 45/45, 13/13 requeridas |
| C6 | **NO EVALUADO** | 74/74, 45/45, 13/13 requeridas |

Esta propuesta no modifica `r9-commit-gates.json`, `owned_rows_gate.py`, corpus ni oráculos hasta que exista una referencia ejecutable con cifras completas por frontera. El inventario de propietarios de R6-final §6 se interpreta con esta precedencia: el `EqualityRecord` mínimo y el journal de C1 son autoridad de las igualdades recursivas; las demás migraciones conservan su corte declarado, condicionadas al gate ejecutable.
