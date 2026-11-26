# Frontera causal de C1 (propuesta para R6 v13)

Estado: **en validación**, sin veredicto de C1. Base `main` `31d91db`; contraejemplo ejecutable en `docs/validation/i07c-rediseno/C1-v12-full-gate-counterexample.md` y reporte íntegro `C:/nexo-work/evidence/luna-c1-corpus.json`.

**Continuación 27:** referencia experimental `i07c-rediseno-c1c2-ref` `5a05dc88ccebd523c804e8695983a516b4edbc73`, con `EqualityId`, `EqualityRecord` mínimo y journal portado de `fc9600a`, **bloqueada** por el repro ASCII de 78 bytes y `dirigido5:2069`. La guía y la evidencia exacta están en `docs/validation/i07c-rediseno/C1C2-reference-blocker-27.md`. No se publica el contrato v13 ni se cambia el manifiesto v12: no existe un commit C1+C2 con puerta cruzada completa verde. En la tabla siguiente, la fila del corte fusionado permanece NO EVALUADO para el gate completo; los cortes posteriores quedan a verificar por Luna con el gate completo.

## Decisión sobre el límite C1/C2

La frontera v12 «C1 sin `EqualityRecord` ni journal» queda retirada. C1 debe crear un `EqualityId` monotónico al comenzar cada igualdad fuente y conservarlo en toda igualdad estructural hija, en cada `PendingDeferredJoin`, en sus aristas tentativas y en las obligaciones recursivas alcanzadas. El ID identifica el intento causal, no el span, `FactId`, representante de variable ni constructor. Reintentar o completar una obligación conserva ese ID; otra igualdad sobre el mismo componente recibe otro. Un diagnóstico E0103 recursivo se reserva en la igualdad que cierra el ciclo y solo usa el origen de un deber de constructor alcanzado por sus operandos y registrado como dependencia de esa igualdad. La reserva no reescribe la obligación ni el origen de otra igualdad.

El mínimo de C1 incluye `EqualityRecord { id, operandos fuente, ordinal, dependencias, estado, joins, diagnostic_reservation }` y journal de primeras escrituras de aristas, forma y procedencia por `(node, campo)` durante el intento. Una igualdad incompatible revierte su journal y publica solo su reserva; una suspendida conserva su obligación por ID sin publicar aristas tentativas; una compatible publica las aristas de ese ID una vez. Las cuotas y los IDs monotónicos no se revierten. El drenaje procesa cada obligación con su dueño; deduplicar por span o raíz no puede sustituir la identidad causal. Esto implementa F0-0002 §§4.1.2.1–2 y 13.9 en las tres recursiones propias sin desplazar los E0103 de las diez filas no propias.

`ReadContext` por entrada/epoch, `OriginFact` general, `Completion` para asignaciones posteriores y migración de las vistas de planificación pertenecían a C2 en v12. La ablación de abajo exige probarlos **junto con** `EqualityId` en el primer corte: el candidato de frontera es **C1+C2 fusionado**, sin publicar el C1 de tres recursiones como commit intermedio. `ResultStatus`, `JoinRecord` de `if` y sus testigos siguen en el corte siguiente, `ConstructorDuty` general y reservas C/D/V en el posterior, activación/replay y cuota después, y retirada del flujo heredado y dos H al final. Los deberes de constructor que el primer corte necesita son una vista de propiedad de su `EqualityRecord`; su consolidación posterior no crea autoridad duplicada. Esta distribución sigue en prueba: si el corte fusionado falla la puerta, debe absorber el siguiente mecanismo causal antes de existir como commit.

La puerta sigue intacta: 20 corpus, 151.188 filas, 0 A/D/A-unmodeled/H nuevos, tres dirigidas propias exactas, F06 igual a `main`, 103 `match` iguales a `main`, `validate.sh`, pila y medidores. C2–C6 requieren cada uno el gate cruzado completo y sus filas propias en un prototipo **por commit real**; una lista de propietarios o una prueba parcial no acredita un corte. Ninguna cifra de esta tabla se declara medida para un commit aún inexistente.

| commit | gate completo OK/C/U/H; A/D/A-unmodeled | filas propias |
|---|---|---|
| C1 v12 `8820551` (falsado) | 62.755/16.671/71.750/2; 0/7/3 | 3/74, 0/45, 0/13 |
| C1+C2 fusionado propuesto | **NO EVALUADO** | 10/74, 0/45, 3/13 requeridas |
| C3 propuesto | **NO EVALUADO** | 27/74, 9/45, 9/13 requeridas |
| C4 propuesto | **NO EVALUADO** | 32/74, 13/45, 11/13 requeridas |
| C5 propuesto | **NO EVALUADO** | 74/74, 45/45, 13/13 requeridas |
| C6 propuesto | **NO EVALUADO** | 74/74, 45/45, 13/13 requeridas |

Esta propuesta no modifica `r9-commit-gates.json`, `owned_rows_gate.py`, corpus ni oráculos hasta que exista una referencia ejecutable con cifras completas por frontera. El inventario de propietarios de R6-final §6 se interpreta con esta precedencia: el `EqualityRecord` mínimo y el journal de C1 son autoridad de las igualdades recursivas; las demás migraciones conservan su corte declarado, condicionadas al gate ejecutable.

## Falsación ejecutable de la frontera estrecha

La ablación `docs/validation/i07c-rediseno/c1-unwitnessed-ablation.patch` se aplica a `i07c-rediseno-c1-ref` `8820551` en una copia desechable; solo deja usar el origen previo cuando el camino del ciclo no atraviesa un constructor sin testigo. Incluye trazas condicionadas por `NEXO_EQ_TRACE`. Se compiló release con rustc 1.99.1, `CARGO_BUILD_JOBS=4` y `heavy-lock --session i07c-rediseno`. Binario base de `main` 31d91db: SHA-256 `27de2d466962ea8455d02f19dccaacb315abc1c935eeeadcbd7b816ee0b82963`; ablación: `938f62700879b6ea5db77c228b93a1fefa76b69c10162f68d88012491c1c9627`. El binario de `main` se compiló en `C:/nexo-work/tmp/main-c1` desde el commit fijado; el SHA difiere del usado por Luna porque es otra compilación reproducible por fuente/commit, no otro oráculo.

`owned_rows_gate.py --commit C1` da 3/74, 0/45, 0/13, F06=`main`, cero fallos. Las diez filas que falsaron v12 se ejecutaron con el mismo oráculo y clasificador: nueve vuelven a C/U; **`dirigido5:2069` sigue `A-unmodeled`**. Programa ASCII (SHA-256 `46a9d45f31f56be5314629233935cc704da00179b8f3a9c67a35a835174bd6ea`):

```nexo
fn Main(){let p=true;var v=(if true then {return;} else {return;});var w=[];var a=(if true then {return;} else {return;});if p{v=[a];a=v;}}
```

`main` emite E0103 `[73,75)` y `[135,136)`; la ablación emite `[73,75)` y `[129,132)`. La traza del grafo muestra **un único** arco de constructor con testigo, `[129,132)`, en el camino del ciclo. Por eso un filtro basado en número de constructores, testigo o raíz no distingue esta igualdad de las tres recursiones propias: la igualdad de `a=v` ocurre dentro de la rama `if p`, cuyo contexto y publicación de aristas no son los de la igualdad anterior `v=[a]`. `repro_c1_conditional.py` verifica el SHA de la fuente, ejecuta ambos binarios y compara la salida JSON completa; se ejecutó y reprodujo la diferencia. Rigen F0-0002 §§4.1.2.1–2, 4.1.2.4 y 13.9. La puerta completa se inició bajo heavy-lock y terminó cuatro corpus sin fallos; se detuvo **solo el PID de esa ejecución** después de la falsación dirigida, conforme a «un fallo es evidencia suficiente de fallo». No hay reporte completo ni se atribuye cero a los otros dieciséis corpus.

**Bloqueo de publicación de v13 y de la referencia:** en `8820551` no existen `EqualityId`, `EqualityRecord`, journal ni `ReadContext` por entrada/epoch; la ablación demuestra que la separación por constructor no reemplaza esas piezas. No existe un prototipo por commits C2–C6 con el que ejecutar la tabla completa. Publicar SHA de C1 corregido, cifras C1–C6 o `verify_r9_contract.py` como «v13 verde» requeriría inventar resultados. La v12 conserva su gate y propietarios congelados mientras se construye el corte fusionado. `verify_r9_contract.py` de v12 sigue verde, lo que solo comprueba estructura de v12.
