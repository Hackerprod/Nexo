# I07c: marcos S3 → B1b en TYPES

Fecha: 2026-11-21. Rol: solo medición y análisis. No se modificó `C:\projects\Nexo-i07c` ni se implementó corrección.

## Veredicto

B1b agrandó **dos marcos que coexisten** en la ruta de `drive`: `drive` 32.432 → 34.832 B (**+2.400**) y `drive::resume_non_match` 31.416 → 32.872 B (**+1.456**). La reserva explícita de la pareja pasa de 63.848 a 67.704 B (**+3.856 B**). El tamaño pedido al hilo de 65 KiB es 66.560 B: la pareja B1b por sí sola ya lo supera en 1.144 B, antes de retornos, `push`, alineación, arnés u otras llamadas. La pareja S3 deja 2.712 B antes de esos costes. Esto explica el cambio de umbral observado sin atribuirlo a 126 llamadas recursivas: la sonda tiene un solo `match`, con 126 constructores en el *tipo*.

Los marcos de `match_advance`, `selection_type`, `pattern_type`, `branch_*`, `finish_selection`, `resolve_pending_projections`, `solve`, `bind` y `require_union` no crecieron. `expr_inner` sí aumenta 576 B; `check_against_context_one` baja 32 B. La causa de fuente es la ampliación F5 de los dos procedimientos grandes y no un aumento del `Frame` persistente del driver: B1b añadió continuaciones lógicas, ramas de expresiones y diagnósticos F5 y asignaciones compuestas dentro de `drive`/`resume_non_match`. Bajo el perfil test sin optimización, las ramas mutuamente excluyentes reservan más espacio para temporales y caminos de limpieza de valores poseídos. La medición no separa cuántos bytes aporta cada brazo nuevo individual.

## Reproducción y método

Copias creadas mediante `git archive 6b08c07` y `git archive be8c47c` en `C:/nexo-work/val/b1bframes/{s3,b1b}`. `rustc 1.99.1 (58cf8f9c3 2026-10-13)` MSVC; `PATH` antepone toolchain y Cargo indicados en el encargo; `TEMP=TMP=C:/nexo-work/tmp`; `CARGO_BUILD_JOBS=4`; targets `C:/nexo-work/target-b1bframes-{s3,b1b}` (1,1 GB cada uno). Perfil `test` por defecto con debuginfo, sin `CARGO_PROFILE_TEST_DEBUG` ni `RUSTC_BOOTSTRAP`; `RUSTFLAGS` contiene solo `-C link-arg=/MAP:C:/nexo-work/val/b1bframes/<árbol>-test.map`. Cada `cargo test --lib --no-run` se ejecutó desde Git Bash/sh bajo `python -B scripts/heavy-lock.py run --session i07c-b1bframes -- sh -c 'sh /c/nexo-work/val/b1bframes/test_final.sh'`; los comandos y salidas están en `test_final.sh` y `{s3,b1b}-test-build.txt`. No se ejecutó desde PowerShell.

| Árbol | Ejecutable de test | SHA-256 ejecutable | `.map` SHA-256 |
|---|---|---|---|
| S3 | `target-b1bframes-s3/debug/deps/nexo-5c578626d119c004.exe` | `d3f4b4624d9550259a4f65d71f7bf09139126f83f75670a266aa090ab63e1fe7` | `9d0160e64e228623eb0406caa04587d7024f9178ac30eb74def432f25e39f474` |
| B1b | `target-b1bframes-b1b/debug/deps/nexo-69b5769b2ae599d6.exe` | `164f960f34c5b0703e03acb1ab567c0a692a621e2cc68ee8e813866d8668560a` | `aa97f8693fe5e3eceee19e2cdfa66e4b8c88590c72b11fe93c149845d2e9320a` |

Para cada función se tomó el símbolo exacto de la sección `0001` del `.map` y se inspeccionó el prólogo con `dumpbin.exe /DISASM /RANGE:0x<inicio>,0x<inicio+192> <exe>`. La cifra es `sub rsp,N` o `mov eax,N; call __chkstk; sub rsp,rax`. El listado de símbolos, direcciones y reservas queda en `{s3,b1b}-test-frames.tsv`; el extractor está en `measure_frames.py`. El `size_of` se obtuvo con `RUSTC_BOOTSTRAP=1 cargo rustc --lib --profile test -- -Z print-type-sizes` bajo el mismo cerrojo y con el mismo perfil; salidas en `{s3,b1b}-types.txt`. Esa compilación auxiliar se hizo antes de los enlaces finales para que los `.map` publicados correspondan a los ejecutables sondeados.

La sonda `scripts/i07c_stack_probe/probe.py <exe> --mode TYPES --kib 65 --case <caso>` dio, para `option_match126` y `result_match126`, S3 **5/5 exit 0** y B1b **5/5 desbordamiento** (código hijo `3221225725`, mensaje `has overflowed its stack`). Los dos casos B1b pasan a 129 KiB. Los SHA-256 de fuente generada son, respectivamente, `9d702f634956ba899926253718fce6f6e47c99c35ef48367d975996d611c7312` y `c15e432a269f65c625d6b17b613039ee004839a8328febd698f3442d8144b45c`. La medición previa sin `/MAP` (`I07c-medicion-pila-s3-b1b-s4.md`) obtuvo los mismos veredictos 5/5 a 65 y 129 KiB; `/MAP` no cambió la banda observada.

## Reservas de marco del binario con debuginfo (bytes)

| Función | S3 | B1b | Δ |
|---|---:|---:|---:|
| `drive` | 32.432 | 34.832 | **+2.400** |
| `drive::resume_non_match` | 31.416 | 32.872 | **+1.456** |
| `expr_inner` | 12.080 | 12.656 | +576 |
| `check_against_context_one` | 6.544 | 6.512 | −32 |
| `selection_type` | 2.656 | 2.656 | 0 |
| `match_advance` | 2.952 | 2.952 | 0 |
| `pattern_type` | 2.800 | 2.800 | 0 |
| `branch_next` | 480 | 480 | 0 |
| `branch_accept` | 800 | 800 | 0 |
| `finish_selection` | 832 | 832 | 0 |
| `resolve_pending_projections` | 2.672 | 2.672 | 0 |
| `solve` | 71.640 | 71.640 | 0 |
| `bind` | 1.296 | 1.296 | 0 |
| `require_union` | 928 | 928 | 0 |

Sumas de **reservas** de rutas, no picos de pila ni sumas por 126 niveles: `drive → resume_non_match`: 63.848 → 67.704 B (+3.856); `drive → check_against_context_one → expr_inner`: 51.056 → 54.000 B (+2.944); `drive → match_advance → selection_type`: 38.040 → 40.440 B (+2.400); con `pattern_type` en vez de `selection_type`: 38.184 → 40.584 B (+2.400); con `finish_selection`: 36.216 → 38.616 B (+2.400). Las llamadas de los últimos tres caminos se alternan durante el `match`; sus tres marcos hoja no coexisten. `solve` también tiene un marco grande, pero su reserva no cambió y no se suma a la pareja `drive → resume_non_match` sin traza que demuestre coexistencia.

## Tamaños de tipos relevantes (`-Z print-type-sizes`, bytes)

| Tipo | S3 | B1b | Δ |
|---|---:|---:|---:|
| `Checker::drive::Frame` | 88 | 88 | 0 |
| `MatchProgress` | 472 | 472 | 0 |
| `BranchAction` | 152 | 152 | 0 |
| `MatchAction` / `DriveStart` | 144 / 144 | 144 / 144 | 0 |
| `OwnedCheckContext` | 136 | 136 | 0 |
| `Result<InferTy, Box<Diagnostic>>` | 24 | 24 | 0 |
| `InferTy` / `parse::Type` | 24 / 24 | 24 / 24 | 0 |
| `parse::Expr` | 160 | 160 | 0 |
| `parse::Statement` | 368 | 376 | +8 |
| `Checker` | 1.608 | 1.608 | 0 |
| `solve::Frame` / `SolverBind` | 680 / 248 | 680 / 248 | 0 |
| `LogicalRightProgress` | — | 168 | nuevo; en `Box` |

El `AssignOp` añadido a `Statement::Assign` coincide con el aumento de 8 B de `Statement`, pero el `Frame` del driver conserva referencias a `Statement`; ese cambio de tamaño no explica por sí solo los +3.856 B. Las dos variantes nuevas de `Frame` contienen `Box` y mantienen el enum en 88 B. El valor de 168 B de `LogicalRightProgress` se crea/desestructura dentro de `resume_non_match`; su almacenamiento duradero está en heap, aunque su construcción y destrucción requieren temporales del procedimiento.

## Diff de fuente que toca la ruta

`git diff 6b08c07 be8c47c -- src/check/types.rs` (copia `types.diff`, para inspección) acota los cambios funcionales pertinentes:

- `types.rs` § `drive` (`be8c47c:8473`): añade `LogicalRightProgress` y `Frame::LogicalLeft/LogicalRight`; en `resume_non_match` (`8542`, brazos desde `8751`) construye y desestructura un contexto poseído y un resultado para el operador lógico, y vuelve a `check_against_context_one`. La sustitución de `matches!(op, "+" | "-")` por `operators::is_arithmetic(op, checker.f5)` afecta varias ramas binarias de esa misma función.
- En el bucle de `drive` (`9713`–`10115`) se añaden `compound_operand_type` en asignaciones, diagnósticos de rango en `for`/índice/binario, ramas `Expr::Neg` y `Expr::Not` con contextos poseídos, y `LogicalLeft` para `and`/`or`. `compound_operand_type` es nueva en `types.rs:1248`.
- `expr_inner` (`11006` y siguientes) añade los resultados de `Neg`/`Not` y el caso de constante. Las comprobaciones binarias cambian a los ayudantes de `operators.rs`. En `check_against_context_one`, el único cambio directo relevante sustituye la clasificación de comparación; su marco baja 32 B.
- Las definiciones de `MatchProgress` y `BranchAction`, y los cuerpos de `selection_type`, `pattern_type`, `match_advance`, `branch_*`, `finish_selection`, `resolve_pending_projections`, `solve`, `bind` y `require_union` no reciben cambios directos en este diff; sus reservas lo corroboran. La ampliación de `parse::Expr` (`Neg`, `Not`) no aumenta su tamaño. `parse::Statement::Assign` sí recibe `AssignOp`.

El listado `.map` de B1b contiene funciones nuevas emitidas **fuera de línea** (no presentes en S3):

| Ayudante nuevo | Reserva B1b |
|---|---:|
| `types::compound_operand_type` | 136 |
| `operators::binary` | 216 |
| `operators::is_arithmetic` / `is_comparison` | 120 / 120 |
| `operators::is_relational` / `needs_f5` | 104 / 128 |
| `pending_f5::diagnostic` / `range_outside_for` | 464 / 352 |
| `pending_f5::range_for` / `const_pending` / `intrinsic_scope` | 88 / 88 / 56 |

También aparecen cierres y *drop glue* asociados a esos ayudantes y a `LogicalRightProgress`; no son un marco adicional permanente del `drive`. El `.map` permite confirmar símbolos separados, pero no cuantifica por sí solo qué expresiones fueron inlineadas en cada call site. Por eso la atribución de +2.400/+1.456 B es al conjunto de cambios de fuente en esos procedimientos, no a un helper individual. Poner `#[inline(never)]` solo en `operators` o `pending_f5` no tiene ahorro demostrado: sus funciones principales ya tienen símbolo fuera de línea.

## Mecanismos propuestos y comprobación

1. Extraer las **continuaciones lógicas nuevas y la adaptación de asignación compuesta** de `resume_non_match` a uno o varios ayudantes pequeños `#[inline(never)]`, pasando referencias o un estado compacto y devolviendo `Result`/acción corta. Meta de medición: recuperar hasta los **1.456 B** de exceso de `resume_non_match` en este perfil; la ganancia neta puede ser menor porque el ayudante tiene su propio marco durante una llamada lógica. Para este repro sin operadores lógicos nuevos, la extracción evita reservar sus temporales en la continuación común. Los 1.456 B aislados probablemente dejan margen demasiado estrecho a 65 KiB; comprobar con la pareja completa.
2. Extraer del **bucle de `drive`** el despacho F5 de `Neg`/`Not`, rango, operadores lógicos y asignación compuesta en ayudantes `#[inline(never)]`, evitando materializar contextos y diagnósticos F5 en el marco común. Meta: recuperar hasta **2.400 B** de exceso de `drive`; combinada con (1), recuperar aproximadamente **3.856 B** y volver a una pareja cercana a 63.848 B. No mover `Frame` ni `MatchProgress` a `Box` como primer paso: sus tamaños no crecieron y ya se almacenan en `Vec`/`Box` donde corresponde. Reducir solo `Result<InferTy, Box<Diagnostic>>` (24 B) tampoco explica kilobytes.

Para verificar cualquiera: compilar S3, B1b y la propuesta con el mismo `rustc` y perfil test con debuginfo, obtener `/MAP` y los prólogos, comparar `drive`/`resume_non_match` y los otros marcos, ejecutar `probe.py` en ambos casos a 65 y 129 KiB repetido cinco veces y confirmar el umbral también en ejecutables sin `/MAP`. Ejecutar después las pruebas funcionales del corte para detectar cambios semánticos. Ninguno de estos mecanismos se implementó aquí.

## Límites

El prólogo mide la reserva explícita por función, no el máximo real del hilo: excluye retornos, `push`, alineación, pila del arnés y marcos de llamadas anidadas; una función que no se invoca en la ruta no aporta a su pico. Los umbrales de `thread::Builder::stack_size` en Windows son discretos. El mapa no asigna bytes de una reserva a una línea de fuente ni acredita que los 126 constructores originen 126 marcos. La correlación causal fuerte es entre el conjunto de cambios B1b en `drive`/`resume_non_match`, los deltas de sus prólogos y el umbral reproducido; para atribuir bytes a un brazo individual haría falta una compilación ablativa controlada fuera de esta medición. No se realizó campaña sin debuginfo: el documento anterior ya registra que ambos árboles pasan a 65 KiB bajo ese perfil. El worktree quedó con los mismos tres archivos no rastreados que al inicio; las copias, targets y archivos auxiliares quedan para que los limpie el coordinador.
