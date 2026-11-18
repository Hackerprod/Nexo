# I07c S3 — parada previa al cambio del solver

Base de trabajo: `fc82cb6`, rama `i07c-estructural`, limpia y al día con su remoto tras `git fetch` (`HEAD...@{upstream}` = `0 0`). El blob `git show HEAD:src/check/types.rs | sha256sum` es `c8fbb9e66c2bfe00299eff5a744f2172affb11e2f523dcd1279d836d3fd132db`, como exige r14 §1. No se editó ningún archivo del producto ni de pruebas en el árbol de entrega.

## Condición de parada

R14 §3 exige, **antes de editar el mecanismo**, fuentes CLI reducidas que recorran `bind → require_union → bind` y `bind → union → union_inner → bind` durante un operador numérico activo. Para **cada** arco, la mutación inversa que hereda `VisitCursor` debe cambiar el JSON CLI. El propio plan ordena volver a A/B si no se obtiene un discriminante CLI; una prueba interna del cursor no lo reemplaza. Se encontraron fuentes que recorren ambos arcos, pero la mutación dirigida del arco `require_union` **no cambió** el JSON en los candidatos probados, incluido uno con dos conflictos diferidos que comparten el owner. No se obtuvo el discriminante obligatorio de ese arco. No puedo declarar cumplida esa regresión ni iniciar el cambio del solver según el orden del plan.

## Fuente, CLI, esperado y obtenido

Fuente ASCII sin BOM ni LF:

```nexo
fn Main(){var a=(if true then {return;} else {return;});let z=a+1;}
```

SHA-256 de la fuente: `000dbe12f911a3724a0b742c00b201a7a7ef59b893ad2fd684a999723b46ba9f`.

Invocaciones, sustituyendo `<fuente.n>` por un archivo con los bytes anteriores:

```text
nexo check <fuente.n> --profile f0-0003-b-v1 --format json
nexo run <fuente.n> --profile f0-0003-b-v1 --format json
```

Esperado para este control: `check=checked`, `run=unit`, diagnósticos vacíos. Obtenido con el binario de base no instrumentado: exactamente esos resultados. En una copia desechable de la misma base, una traza añadida a `bind` confirmó `numeric_domain_owner` activo para el operando `a` (`[62,63)`) en ambas visitas del checker; no apareció la entrada a `require_union` ni la de `union` desde `bind` en esa visita. Este control permitió comprobar la instrumentación, pero no satisface el testigo exigido por r14.

Fuente que sí recorre `bind → require_union → bind` con owner activo, ASCII sin BOM ni LF:

```nexo
fn Main(){var x=[];var y=[];var a=Some([x]);var c=Some([y]);let z=(if true then {let b:Option<Array<I64>>=a;let d:Option<Array<I64>>=c;value:1;} else 1)+1;x=[1];y=[1];}
```

SHA-256: `5c200ae11a6aa66d098d217f96bceda43e067b9721ccea48c804629e7282b399`. En la traza de base, ambos `bind` de payload se ejecutan bajo `require_union` a `[106,107)` y `[133,134)`, con owner del operador `[66,154)`. Una copia desechable del mismo blob se mutó para pasar ese owner del `bind` padre a ambos `bind` de payload, sustituyendo los cursores vacíos solo en `require_union`. La traza confirmó dos `PendingDeferredJoin` con ese owner. **Esperado por la prueba de mutación de r14:** que el JSON CLI difiera. **Obtenido:** `check` y `run` conservaron `rejected` y los mismos dos `NX-E0102/type_mismatch` en `[106,107)` y `[133,134)`; la comparación de los objetos `diagnostics` completos tampoco mostró diferencia. Por tanto, la fuente recorre el arco, pero no lo discrimina.

Fuente que recorre `bind → union → union_inner → bind` con owner activo:

```nexo
fn Main(){let z=(if true then {var a=[[]];var b=(if true then {return;} else {return;});a=[[b]];b=[1];value:1;} else 1)+1;}
```

SHA-256: `32292988640e56c02b74552b80b8b3b8bd332a6c0b1bc147`. La traza de base registra `bind` con `Deferred` y el paso a `union` en `[90,95)` durante el operador numérico. Esperado y obtenido en la base: `check=checked`, `run=unit`, diagnósticos vacíos. No se hizo la mutación de este segundo arco una vez falló el requisito independiente del primero; la traza por sí sola no acredita el discriminante exigido.

Control normativo independiente RD71 (F0-0002 §14.10), fuente ASCII:

```nexo
fn Main(){let x=1;let p=true;var ar=[1];var br=[true];var aar=[[1]];AssertEq([p,p],(if p then ar[ar[x]] else {value:aar[x-(if true then ar else 279)];}));}
```

SHA-256: `5db331ae6fde0fc6c137b915a61369cd87ec4ded11282d7f15f50fcfaab81da3`. Esperado y obtenido en `check` y `run`: `rejected`, un solo `NX-E0102/type_mismatch`, primario `[136,138)` (`ar`). Es control de salida, no discriminante de los dos arcos.

## Búsqueda previa

En la copia desechable del blob base se instrumentaron las entradas a `bind`, `bind → require_union`, `bind → union`, `unify` y `defer_shape_conflict`, registrando si el cursor portaba `numeric_domain_owner`; la instrumentación no está en el árbol de entrega. Una primera batería de 1.122 fuentes ASCII usó el producto determinista de 11 declaraciones iniciales (raíz diferida, Array, Option/Result, concreta y dos raíces), 17 expresiones candidatas (alias, `+`, `Len`, índice, Array, `Some`, `Ok`, ramas con uno o dos operandos) y 6 sufijos de fijación/comparación. Se invocó `check` con `f0-0003-b-v1` para cada fuente. Esa batería dio 132 entradas a `bind` con owner activo y ningún arco buscado. La búsqueda dirigida posterior colocó declaraciones y anotaciones dentro del bloque de un `if` que es operando de `+`; así encontró los dos arcos y probó la mutación de `require_union` descrita arriba. El resultado negativo de esa mutación no demuestra que un discriminante sea imposible, pero activa la parada expresa de r14 §3 sin sustituirla por una comprobación interna.

La lectura de `drive` muestra que Array (`types.rs` alrededor de 8449), llamadas `Some`/`Ok`/`Err` (alrededor de 8627) y otros argumentos de llamada retiran temporalmente `numeric_domain_owner` antes de visitar sus hijos; un bloque de `if` puede conservar el owner al visitar declaraciones internas. Hace falta una fuente CLI cuya salida sí cambie al heredar el cursor en `require_union`, además de una para `union_inner`, o una revisión A/B del requisito. No se ejecutaron la conversión, la puerta mecánica, el diferencial final ni la campaña final; S3 permanece pendiente.
