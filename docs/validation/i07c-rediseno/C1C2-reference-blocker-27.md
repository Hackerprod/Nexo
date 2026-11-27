# C1+C2: referencia experimental detenida (continuación 27)

Estado: **bloqueo reproducible**, sin PASS ni contrato v13. Rama de diseño `i07c-rediseno-c1c2-ref`, commit `5a05dc88ccebd523c804e8695983a516b4edbc73`, descendiente de C0 `96d564496b1072d09a4a169c3be801d69aa3af96`. Binario release de ese árbol: SHA-256 `8129d748096734a168e30af16f139ff6cac970412a8c664cd6d3f81cb44dfe45`; oráculo `main` `31d91db` compilado de su blob: SHA-256 `27de2d466962ea8455d02f19dccaacb315abc1c935eeeadcbd7b816ee0b82963`. Rustc 1.99.1; `CARGO_BUILD_JOBS=4`, `CARGO_TARGET_DIR=C:/nexo-work/target`, compilación release bajo `heavy-lock --session i07c-rediseno`.

Se portaron de `i07c-rediseno-proto` `fc9600a` el registro de primera escritura por nodo, los accesos mutables de `TypeNode` y la restauración inversa de un intento rechazado. Se añadieron `EqualityId` monotónico, `EqualityRecord` mínimo y dueño en `PendingDeferredJoin`. Se conservaron los dos commits de la referencia C1 v12. **El árbol no cierra C1+C2:** `recursive_path_origin` aún consulta constructores globales, y un `Waiting` aún no restaura con seguridad sus escrituras ni reproduce su obligación por ID.

## Prueba mínima que bloquea la puerta

Fuente ASCII de 78 bytes, SHA-256 `cf0279fa3fc51b9498120c6c2c4d2f2f1f23050b5d5a1089ff7e3fc8a5a0d69f`, perfil `f0-0002-b-v2`:

```nexo
fn Main(){var v=(if true then {return;} else {return;});var a=[];a=[v];v=[a];}
```

`main` emite un único `NX-E0103` con primario `[73,76)`. La referencia `5a05dc8` emite tres: `[62,64)`, `[67,70)` y `[73,76)`. La fuente y los JSON se obtienen con `candidate_gate.execute`; el positivo de `owned_rows_gate.py --commit C1` sí da 3/74, 0/45, 0/13, F06=`main`, cero fallos. La otra prueba congelada `dirigido5:2069` (SHA de fuente `46a9d45f31f56be5314629233935cc704da00179b8f3a9c67a35a835174bd6ea`) conserva dos E0103 en `main`, `[73,75)` y `[135,136)`, pero la referencia desplaza el segundo a `[129,132)`. Es `A-unmodeled`, ajeno a C1. Rigen F0-0002 §§4.1.2.1–2, 4.1.2.4 y 13.9.

El gate completo de 20 corpus/151.188 filas **no se ejecutó** para `5a05dc8`: estos repros ya prueban que no puede quedar verde; un fallo basta para detener una puerta roja. Tampoco se ejecutó `validate.sh` ni la pila 5/5 para este commit. `cargo check --profile dev`, `cargo build --release`, `cargo fmt` y `git diff --check` terminaron sin error; son solo comprobaciones de compilación/formato. `verify_r9_contract.py` sigue verde sobre el contrato **v12**, que no acredita este árbol. No se modificaron corpus, esperados, clasificadores ni puertas.

| Commit | Gate completo de 20 corpus | Filas propias medidas |
|---|---|---|
| C1 v12 `8820551` | 151.188 filas: OK 62.755, C 16.671, U 71.750, H 2, D 7, A-unmodeled 3 (ejecución de Luna, continuación 25) | 3/74, 0/45, 0/13 |
| C1+C2 experimental `5a05dc8` | No ejecutado: repro mínimo rojo antes del barrido | 3/74, 0/45, 0/13; F06=`main` |
| C3–C6 | A verificar por Luna con el gate completo | A verificar |

## Siguiente paso necesario

La identidad debe llegar desde el constructor hasta la obligación que se activa, con dependencias de igualdad publicadas y `ReadContext` de entrada; el drenaje de `Join` debe leer esa cadena causal, no recorrer `ObligationKind::Constructor` global. La restauración de `Waiting` debe reejecutar por su ID sobre el grafo publicado. Solo después de cerrar la prueba mínima y `dirigido5:2069` se corre la puerta completa y se fijan los SHA del contrato v13. Los cortes posteriores quedan **a verificar por Luna con el gate completo**; ninguna cifra se les atribuye.
