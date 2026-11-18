# I07c Bloque I, motivo 1 — informe de C

## Alcance y estado

Esta entrega implementa únicamente el motivo 1 de la propuesta r10 validada por D: patrones `Option`/`Result` sobre raíz monomórfica abierta y la frontera de análisis de `match` con scrutinee sin valor ejecutable. El motivo 2 (`tainted` y `numeric_domain_owner`) sigue **pendiente y suspendido** tras la detención documentada en `I07c-C-Bloque-I-r1-hallazgo.md`; no se migraron sus campos, cursores ni lecturas. El criterio comprobable de cursores de la propuesta no se reclama como satisfecho. S3, integración y validación D/D' quedan pendientes. C no hizo commit ni push: git es de solo lectura en esta entrega.

## Cambios por archivo y función

- `src/check/types.rs`: `ObligationKind::PatternConstructor`, clase y ocurrencias del grafo; `require_match_pattern_shapes` enlaza raíz y componentes de cada constructor de patrón en orden fuente antes de resolver los brazos, incluso `None` antes de `Some`; `pattern_type` usa la forma compartida y exporta bindings; `pattern_constructor_error` y `pattern_shape_mismatch` emiten las subcausas de forma en el sitio correspondiente. Las igualdades posteriores detectan el primer conflicto interior, conservan origen y no completan componentes hipotéticos por defecto.
- `src/ir/lower_match.rs`: `match_tests` omite descriptores y pruebas ejecutables exclusivamente para `Analyze` con `Ty::Never`; `bind_match_pattern` visita los patrones y bindings inferidos por la ruta existente sin exigir payload ejecutable ni inventar un binding para la variante `None`. Se conserva el recorrido de brazos, propiedad y guardas; `Live` mantiene sus descriptores.
- `src/diagnostic.rs` y `docs/adr/F0-CODIGOS.md`: Data cerrado, mensajes, fuente publicada, catálogo y filas sincronizadas para `NX-E1325/constructor_shape` y `NX-E0102/constructor_shape`. La edición del ADR se limita a las dos filas de catálogo autorizadas por r7; no se modificaron F0-0002, F0-0003 ni `docs/baseline/`.
- `tests/i07c_pattern_deferred.rs`: positivos de B3, `Some`/`Ok`, orden de `None`, anidamiento, bindings y usos; negativos de familia/cobertura; los dos repros de propiedad de D con su reparación, recurso dentro de `Some` y `unreachable_resource_copy.n`.
- `tests/i07c_deferred_constraints.rs`: igualdad posterior compatible/incompatible, conflicto interior anidado, valor vivo cerrado/irresuelto y límite N(f) de `Len(l0)`.
- `tests/i01_diagnostic.rs`, `tests/j01_profile.rs`, `tests/j04_match_semantics.rs` y `tests/explain_and_print_output.rs`: serialización, catálogo, `explain`, parejas negativas/positivas y cifra de entradas NX-E0102.

## Repros y oráculos

Con `DV=(if true then {return;} else {return;})`, `fn Main(){let l0=DV;match l0{Some(x)=>{},_=>{}}}` pasaba en `9b8e03e` a `driver_error/internal_error`; ahora `check=checked` y `run=unit`, sin diagnósticos, salida, eventos ni observaciones, como F0-0003 §3.2. Las variantes `let k:Bool=x`, `let k:I64=x`, `let k=x`, `Ok(x)`, `Some(x)`/`None`, `None` primero, `var l0`, anidamiento y control directo se cubren con los oráculos de la propuesta. `None` desnudo conserva `NX-E1304/unreachable_arm`.

La familia incompatible da `NX-E1325/constructor_shape`, primario `[78,83)`, `data={"constructor":"prelude:Ok","scrutinee_constructor":"Option"}`. La igualdad posterior con `Bool` da `NX-E0102/constructor_shape`, primario `[95,97)`, `data={"expected_type":"Bool","found_constructor":"Option"}`, relaciones `origin [66,73)`, `expected_type [90,94)` y `found_type [95,97)`. El conflicto interior anidado señala `Option` en su primer origen interior. Los tests fijan también etapa, operación, regla, token, mensaje, campos comunes y causa, según el ADR.

Los dos repros de D conservan los errores de propiedad: `NX-E0201` con subcausa `null`, primario `[94,95)`, y `NX-E0202/definitely_consumed`, primario `[109,110)`, con sus relaciones y cadena causal. Sus positivos aceptan en `check` y `run`. El constructor de **valor** irresuelto conserva `NX-E0113`; `Len(l0)` sobre forma parcial no llega a pánico, `internal_error`, `InvalidIr` ni cuelgue y no se fija diagnóstico.

## Mutaciones inversas y restauración

En copia desechable, cada mutante compiló y volvió rojo un testigo CLI; después de cada uno se restauró el archivo y se verificó SHA-256:

| Mutante | SHA-256 mutante | Testigo que falla |
|---|---|---|
| Quitar obligación de patrón | `5194d583a4d3e100e7db3702e600e317f30feac9279ee1d06c1abd18c0ae2b95` | B3 positivo vuelve a `driver_error` |
| Exigir descriptor en `match_tests` | `41acbe6eb0e5a9f52b4c702b929a110a6ecbd4ab64cbb75e99b16d4d63b3976e` | B3 positivo vuelve a `driver_error` |
| Exigir descriptor en `bind_match_pattern` | `f1acec978a05e3e2edcde51b1457b0c95fe96de3a8ccd06665e537c4c22585e0` | B3 positivo vuelve a `driver_error` |
| Omitir brazos mediante atajo r9 | `00f4411342e8534cca07bf6cf8962b0123bb2c882dd6caead7654c427ba94fc5` | Ambos negativos de D pasan indebidamente a `checked` |
| Omitir NX-E0113 en valor vivo | `33215e4d7c6b33136925129300b0aa3783357ad70aeb030c2413760ccab7d90d` | Valor vivo irresuelto vuelve a `driver_error` |

SHA-256 restaurados: `src/check/types.rs` `cee556bdb250c283dd357b8b9a98e79f08800b24bd93e6682be668dde408bdc1`; `src/ir/lower_match.rs` `29ed46fe5512dfbe6b333fa256873e93701aa10412b0436ea6f96b62a8895f7e`. El artefacto Cargo compartido se limpió después de los mutantes; las pruebas dirigidas se recompilaron desde el árbol corregido.

## Puerta y diferencial

Generador congelado de D `D:\nexo-work\val-D1\h\fuzz.py`: SHA-256 `328ba0488cc11f0fbc39184b4d9b571559b8a627bb3cb3985ca0a3faa3d57ef2`. Las 369 fuentes de corpus, fixtures y RD152 se compararon contra `9b8e03e` en `check` y `run`, debug y release: 0 diferencias y 0 fallos internos. En el fuzz final congelado, semillas 1–5 (1000 fuentes por semilla, por perfil), hubo 6 diferencias por perfil, las mismas en debug y release: `check` y `run` de exactamente 3 fuentes con patrones sobre raíz abierta. Semilla 3 índice 329 acepta B3 antes interno; semilla 4 índice 908 reemplaza un C secundario NX-E0113 por NX-E0102/not_indexable después de que el patrón fija `Option<Bool>`; semilla 5 índice 390 añade NX-E0102/not_indexable en `Len(b1)` por la misma forma, bajo el límite N(f). Los seis registros completos de ambos perfiles son idénticos. No hubo pánicos, `InvalidIr` ni errores internos de la candidata. El diferencial de 369 fuentes se repitió después de la puerta final.

La puerta final se ejecutó en un clon limpio construido de `git archive HEAD` más los diez archivos editados autorizados, con Cargo offline y un trabajo de compilación a la vez. Los SHA-256 de esos diez archivos coincidieron entre clon y árbol de entrega. `sh scripts/validate.sh` salió con código 0: `cargo fmt --check`, Clippy `-D warnings`, 1131 pruebas de biblioteca debug y 1133 release, además de las integraciones, incluida RD **152/152 en ambos perfiles** (cero diferencias informativas), T01–T14, `tests/f0007.rs` y los tests `i07c_*`. Las regresiones protegidas de A2 §11.7 conservaron los ticks 2/6/1 y 56/25/15. `python scripts/objetivo.py` dio 15/29, exactamente programas 01–15 ejecutables. El diferencial **final**, después de esta puerta, contra `9b8e03e` dio 0 diferencias y 0 fallos en las 369 fuentes para `check`/`run` debug y release.

## Criterio de cursores y siguiente paso

El grep estático de `src/check/types.rs` sigue encontrando `InferenceGraph.numeric_domain_owner` en la línea 1511, `InferenceGraph.tainted` en la 1530, `PendingDeferredJoin.numeric_domain_owner` en la 480 y transporte por `previous_owner`/`prior_taint` desde las líneas 691/698; esto es intencional en esta entrega limitada al motivo 1. Por ello el criterio de cero campos/booleanos transportados del motivo 2 **no se cumple** y requiere la revisión A/B separada. S3 y D/D' deben evaluarse después de esa decisión, sin reinterpretar este informe como PASS del Bloque I completo.
