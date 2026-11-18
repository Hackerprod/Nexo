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

## Nueva parada en S3, posterior al PASS de B-r5 (2026-11-17)

La enmienda r17 ya retira la antigua exigencia de discriminante CLI para los cinco sitios de cursor vacío; la parada anterior queda como historia. En la base limpia `815424c`, antes de convertir el solver, apareció un choque **entre el oráculo obligatorio de r14 §4 y F0-0002 §4.1.2.6** para la extensión profunda del «Gemelo negativo con tipo adelantado». No se cambió el ADR ni se fijó un test con la salida de la base.

**Fuente exacta:** ASCII sin BOM ni LF final, concatenando `fn Main(){var r=` + `[` repetido 126 veces + `true` + `]` repetido 126 veces + `;r=` + `[` repetido 126 veces + `1` + `]` repetido 126 veces + `;}`. Son 530 bytes; SHA-256 `633e200006cc34e349f3e0a13907afd58453cd1d548e5e88ac974ce9947b2654`. El token denunciado es `1` en `[401,402)`; el primer array ocupa `[16,272)`.

**CLI reproducible**, con esos bytes en `<fuente.n>`:

```text
nexo check <fuente.n> --profile f0-0003-b-v1 --format json
nexo run <fuente.n> --profile f0-0003-b-v1 --format json
```

**Esperado por el plan r14 §4:** salida 1, `outcome.kind=rejected`, único `NX-E0102/type_mismatch`, `stage=types`, `operation=types.check`, primario y relación `found_type=[401,402)`, relación `expected_type=[16,272)`, `data.expected_type=Array<`×126 + `Bool` + `>`×126 y `data.found_type=Array<`×126 + `I64` + `>`×126. El plan atribuye esos campos a F0-0002 §§4.1.2.1–2, 4.1.2.6 y 13.15–17.

**Regla del ADR que choca:** F0-0002 §4.1.2.6 dice que `data.expected_type` describe el contexto **de ese sitio**, que `data.found_type` describe el tipo completo **de la expresión señalada** y que descomponer una igualdad no autoriza a serializar el tipo del elemento con el span del array completo. En el mismo §4.1.2.3, el gemelo literal `fn Main(){var r=[true];r=[1];}` fija `Bool`/`I64` con primario en la hoja, y la precisión de `r=[(r==r)==true];r=[1];` reserva también `Bool`/`I64` para la hoja incompatible. Aplicadas a esta fuente, esas reglas dan `data={"expected_type":"Bool","found_type":"I64"}` en `[401,402)`, con el origen del array completo conservado como relación.

**Obtenido en base limpia `815424c`:** tanto `check` como `run`, en debug y release, dan salida 1, `rejected` y exactamente un `NX-E0102/type_mismatch` en `[401,402)`, `data={"expected_type":"Bool","found_type":"I64"}`, `expected_type=[16,272)` y `found_type=[401,402)`. No hay divergencia entre perfiles ni modos. El blob de `src/check/types.rs` en HEAD, medido en shell de bytes, es `c8fbb9e66c2bfe00299eff5a744f2172affb11e2f523dcd1279d836d3fd132db` como exige r14.

**Razón de parada:** convertir la continuación para emitir los arrays completos sobre el primario de la hoja cumpliría r14 y contradiría la regla citada del ADR; conservar `Bool`/`I64` cumpliría el ADR y haría fallar el negativo aislado obligatorio de r14. C no elige un oráculo nuevo ni introduce una excepción por profundidad. A/B deben corregir el esperado del plan o identificar una regla del ADR que resuelva la contradicción antes de reanudar S3.

Se habían convertido recorridos puros de `types.rs` como trabajo preliminar. Compilaron y pasaron Clippy `-D warnings`; la suite de biblioteca dio 1132 PASS/0 FAIL/53 ignorados, y el diferencial debug `check`/`run` de las 369 fuentes protegidas dio cero diferencias. **Se retiraron íntegramente esos cambios** al activar esta parada; `src/check/types.rs` volvió al blob anterior y `git status --short` quedó limpio antes de añadir este hallazgo. No se hicieron la conversión del solver, la puerta mecánica final, las mutaciones ni la campaña de fuzz. S3 sigue pendiente, sin commit de solución ni informe de cierre.

### Resolución (coordinador del carril)

Sin contradicción real: la «Nueva parada» cita el esperado de r14 §4 (tipos Array compuestos), pero r15 (PASS de B ronda 3 y ronda 5) lo sustituye por `data={"expected_type":"Bool","found_type":"I64"}` con primario `[401,402)`, que es exactamente lo obtenido en la base y lo que exige F0-0002 §4.1.2.6. El plan se lee como un único documento enmendado en orden r13 → r17; ante conflicto, prevalece el registro posterior. No hace falta nueva ronda A/B; se relanza a C con esa instrucción explícita.

## Parada operativa posterior a r17 — campaña de mutación global (2026-11-18)

La resolución anterior permitió implementar S3 según r13–r17. El candidato de `src/check/types.rs` tenía SHA-256 `1c1f9edda28e8377b9f9fbd9dcb845b169f00163daeac161d7b8f5af86596b36`; `sh scripts/validate.sh` pasó en clon limpio, RD dio 152/152 por perfil, objetivos 15/29, las ocho filas J16-S1C pasaron a 204 KiB debug en TYPES y CHECKER y a 256 KiB debug/release. El diferencial protegido dio 0 cambios en 369 fuentes y 4.800 fuentes del fuzz de D; la campaña tipada completó 1.000.000 por perfil, el fuzz genérico 1.000.000 por perfil y el modelo independiente 100.000. Las diez mutaciones inversas dirigidas discriminaron y restauraron el hash del candidato. No apareció una diferencia de salida Nexo que exigiera otro mecanismo.

**Invocación de la puerta que no se pudo cerrar:** `python scripts/i07c_mutaciones.py --base b506d7d --run --limit 10 --full-suite --mutation-profile both --debug-sample 100 --only production --target-dir D:/nexo-work/target-i07c-c --results-copy docs/validation/i07c-mutations/results-s3.jsonl`, en el candidato. El enumerador produjo 4.026 sustituciones de producción respecto de `b506d7d` (población SHA-256 `fdc967c51d5e7d1f893194df2336111d5b5baf3c76e4266e6e6b835baa2237eb`). **Esperado por el plan §7:** clasificación de las 4.026 en copia desechable, con suite completa para cada superviviente, antes de entregar S3 a D. **Obtenido:** diez filas completas M00001–M00010; nueve detectadas y M00005 superviviente tras ambas suites completas. M00003 requirió la suite integral para ser detectado; M00005 sobrevivió a ambas. Cada uno de esos casos consumió varios minutos de compilación y ejecución por perfil. Restan 4.016 candidatos bajo la restricción de una sola carga pesada a la vez y poca RAM. Ejecutar solo pruebas dirigidas, omitir las integraciones o reducir la población violaría §7.

No se atribuye PASS ni se entrega S3. La población y el mecanismo ya están definidos; falta una estrategia operativa explícita y un presupuesto de tiempo para completar esta puerta de varios días sin relajar su evidencia, o una enmienda A/B del alcance. C no modifica esa puerta por su cuenta. El candidato y los resultados de la muestra se conservaron fuera del árbol; todos los archivos de implementación y evidencia de S3 se retiraron del espacio de trabajo. `src/check/types.rs` volvió al blob de HEAD SHA-256 `c8fbb9e66c2bfe00299eff5a744f2172affb11e2f523dcd1279d836d3fd132db`; `git status --short` estaba limpio antes de añadir este hallazgo.
