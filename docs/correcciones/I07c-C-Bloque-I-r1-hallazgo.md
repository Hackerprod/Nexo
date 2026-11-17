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

## Nueva detención en S1 (17 de noviembre de 2026, HEAD `6e853ff`)

Tras el PASS documental de r10, C ensayó en el mismo árbol una migración de S1: retiró los dos campos globales de `InferenceGraph`, trasladó contaminación y procedencia numérica a ocurrencias, y sustituyó booleanos transportados por consultas de sitio. La sonda compiló con Cargo offline; se descartó íntegra al observar el diferencial. `src/check/types.rs` volvió al blob de HEAD, SHA-256 `4fe96534c6f25cc84d77d0be96d9c08c33867f9d49dd68f4790cfd207e99056a`, comprobado con `git show HEAD:src/check/types.rs | sha256sum`. No quedaron cambios de código ni tests, no se empezó S2 y no se reclama puerta mecánica ni PASS.

**Salida RD71 protegida que detuvo el ensayo.** Fuente exacta de F0-0002 §4.1.2.5, RD71:

```nexo
fn Main(){let x=1;let p=true;var ar=[1];var br=[true];var aar=[[1]];AssertEq([p,p],(if p then ar[ar[x]] else {value:aar[x-(if true then ar else 279)];}));}
```

Con `check` y `run` debug, la base `9b8e03e` devolvió `rejected`, solo `NX-E0102/type_mismatch`, primario `[136,138)` (`ar`), `data={"expected_type":"I64","found_type":"Array<I64>"}`, relación `found_type` en el primario y `cause_chain=[]`. La sonda de S1 devolvió además `NX-E1303` sin subcausa, primario `[116,149)` (rama `else`), el mismo Data, relaciones `expected_type` `[94,103)` y `found_type` `[116,149)`, `cause_chain=[]`. RD71 fija C1; su omisión V anotada es la comparación **exterior** de `AssertEq`, no este C de unión de ramas. La aceptación siguió `rejected`, pero cambió la secuencia de diagnósticos, prohibido por la propuesta §3/§5.

En el diferencial debug de 369 fuentes contra `9b8e03e` cambiaron exactamente tres fuentes (ambos comandos en cada una): RD71 y su fixture duplicada `tests/fixtures/i07c_repros/forms/branch-assert-equality-array-index-mutable-5db331ae6f-sample1.n` ganaron el C descrito; `tests/fixtures/i07c/independent_unresolved_constructor_in_branch.n` sustituyó `NX-E0103` en `[115,117)` por `NX-E0102/type_mismatch` en `[111,113)` sobre `q3`. Ninguna es un patrón de raíz abierta B3. Los tests de biblioteca de la sonda pasaron 1129 y fallaron 2 en la última pasada completa; ambos fallos se reprodujeron verdes de forma dirigida tras una última corrección, pero no se repitió la pasada completa porque RD71 obligó a detener el código. Release, fuzz, RD 152/152 y `validate.sh` quedaron sin ejecutar para la sonda descartada.

La diferencia de RD71 apareció en la propagación por ocurrencia del resultado inválido de un selector hacia la unión de ramas y su consumidor `AssertEq`; la traza externa no demuestra aún cuál arista o obligación concreta quedó sin contaminar. Para continuar 4.3, A/B deben revisar un mecanismo verificable de dependencia causal para ese camino y la supresión del C derivado, manteniendo el C independiente del selector, y decidir cómo preservar también el cierre de `q3` sin una autoridad booleana entre visitas. C no fijará un esperado nuevo ni añadirá una excepción por fuente.

Generador de D congelado sin modificar: `D:\nexo-work\val-D1\h\fuzz.py`, SHA-256 `328ba0488cc11f0fbc39184b4d9b571559b8a627bb3cb3985ca0a3faa3d57ef2`.
