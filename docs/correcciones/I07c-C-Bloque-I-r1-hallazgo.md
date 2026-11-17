# I07c Bloque I — hallazgo de C en 4.3

Estado: **implementación detenida; requiere revisión A/B de la propuesta y del alcance**. No se ha modificado código, pruebas ni ADR. La sonda temporal descrita abajo se revirtió; `git diff -- src/check/types.rs` quedó vacío. No se reclama PASS ni se entrega Bloque I a D.

## Fuente y contrato

Fuente `D:\nexo-work\i07c-c0\r2\b3x.n`:

```nexo
fn Main(){let l0=(if true then {return;} else {return;});match l0{Some(x)=>{},_=>{}}}
```

F0-0003 §3.2 fija `check=checked`, `run=unit`, sin diagnósticos, salida, eventos ni observaciones: `Some(x)` establece `Option<α>`, `_` cubre `None` y `α` puede permanecer hipotético porque no se ejecuta un valor de ese componente. El binario de `9b8e03e` devuelve `driver_error/internal_error` (D1, motivo 1).

## Sonda y resultado

En una edición **temporal** de `src/check/types.rs`, justo antes de `pattern_type` del `StatementMatch`, se invocó `require_union(id, Option, span)` cuando el scrutinee era `Deferred(id)` y el patrón era `Some(x)`, y se refrescó `selected` con `selection_type`. Es una sonda de la igualdad de forma, no una implementación de la obligación completa, del anidamiento ni de los diagnósticos. `cargo build --locked --offline` en debug compiló. La misma invocación `nexo check --profile f0-0003-b-v1 --format json D:\nexo-work\i07c-c0\r2\b3x.n` produjo:

```text
probe selected after shape: Union(Option, Deferred(1), None); concrete=None
probe export (63, 65): Unresolved
outcome.kind=driver_error; reason=internal_error; diagnostics=[]
```

La traza apareció dos veces en el barrido de planificación y comprobación. La sonda se retiró completamente. Generador de fuzz de D leído sin modificar: `D:\nexo-work\val-D1\h\fuzz.py`, SHA-256 `328BA0488CC11F0FBC39184B4D9B571559B8A627BB3CB3985CA0A3FAA3D57EF2`.

## Precondición que falta en el alcance aprobado

`selection_type` puede representar internamente `Option<Deferred(1)>`, pero `concrete` exige que el payload sea concreto antes de devolver la unión (`src/check/types.rs::concrete_with_depth`). `resolved_syntax_type` exige lo mismo y la exportación registra `ResolvedType::Unresolved` para la lectura del scrutinee (`src/check/types.rs`, bucles de exportación de expresiones y `expr_vars`). `ResolvedType` solo dispone de `Concrete(SyntaxType)`, `Never` y `Unresolved`; `SyntaxType` no representa un componente hipotético. `src/ir/lower.rs::resolved_type` convierte `Unresolved` en `Ty::Never`. `src/ir/lower_match.rs::match_tests` y `bind_match_pattern` exigen `union_descriptor()` incluso para el match posterior al `return` y devuelven `InvalidIr("pattern union")` si reciben `Ty::Never`.

Por tanto, registrar la forma en el grafo, como ordena la propuesta validada, **no basta** para el positivo normativo mientras `α` permanezca hipotético. La propuesta §2 prohíbe fabricar tipo en lowering y permite tocar `src/ir/lower.rs` o `src/ir/lower_match.rs` solo tras nueva traza y revisión A/B. La sonda aporta esa traza nueva: el tipo del grafo ya tiene forma Option, pero la interfaz exportación–IR exige un tipo completamente concreto. Un placeholder concreto para `α` sería un tipo inventado y no está autorizado por el mecanismo validado.

## Retorno solicitado a A/B

Revisar la representación entre inferencia e IR y el tratamiento de un match cuyo scrutinee no tiene valor ejecutable, conservando el componente hipotético y los oráculos de F0-0003 §3.2/§8.2. Decidir el mecanismo único y el alcance exacto de `src/ir/lower.rs` y, si corresponde, `src/ir/lower_match.rs`; después revalidar la propuesta antes de que C retome código. La migración de `tainted`/`numeric_domain_owner` tampoco se ha iniciado: la orden de parada del brief impide entregar una mitad del bloque.
