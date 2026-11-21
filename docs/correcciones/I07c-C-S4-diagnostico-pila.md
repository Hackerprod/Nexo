# I07c S4.1 — diagnóstico medido de pila en `option_match126` y `result_match126`

Fecha: 2026-11-21. Rol: medición y análisis; no se modificó `C:\projects\Nexo-i07c`.

## Conclusión

S4 **sí aumenta marcos** de la ruta debug de `match`: con debuginfo desactivado, `drive` +352 B, `resume_non_match` +448 B, `match_advance` +128 B y `expr_inner` +96 B. La suma de las reservas de `drive` y su llamada directa a `resume_non_match` pasa de 65.496 a 66.296 B (+800 B), sin incluir llamadas, alineación ni marcos anteriores. El `Frame` de `drive` mide 88 B en ambos árboles; `SolverBind` y el marco de `solve` se reducen. El aumento proviene de temporales/código no optimizado en los dos grandes procedimientos y, en los puntos donde se invocan, de los nuevos ayudantes de hechos del grafo. No hay evidencia de una llamada recursiva nueva por cada uno de los 126 niveles de `Option`/`Result`.

**El salto de umbral 65→129 KiB no se reproduce al comparar `be8c47c` y `dd26cad` compilados igual.** Cinco ejecuciones por combinación, en TYPES para ambas filas: con `CARGO_PROFILE_TEST_DEBUG=0`/`-C debuginfo=0`, ambos árboles pasan 65 y 129 KiB (5/5); con debuginfo del perfil test por defecto, ambos desbordan a 65 KiB (5/5, salida `3221225725`) y pasan a 129 KiB (5/5). En barrido simple 60–69 KiB, ambos pasan desde 65 KiB sin debuginfo; con debuginfo ninguno pasa entre 65 y 125 KiB (barrido 65–89 de 1 KiB y 89–129 de 4 KiB). Son umbrales *candidatos* de `thread::Builder::stack_size`, no máximos exactos de consumo ni mínimos exhaustivos.

El ejecutable S4 original `C:/nexo-work/target-s4/debug/deps/nexo-ae65c584bd3eb2e6.exe` reproduce el desbordamiento a 65 KiB; su fingerprint `test-lib-nexo.json` registra `rustflags: []`, el binario tiene PDB de 90.255.360 B y corresponde al perfil con debuginfo. La medición histórica S3 de 65 KiB está en `I07c-C-S3-informe.md`, anterior a integrar B1b. El blob `src/check/types.rs` de `be8c47c` es idéntico al de `8aba06f` (Git blob `a9195bbd05051a7d6ccf04de729380db1132eb2b`); el informe S3 no identifica un ejecutable equivalente a este perfil/árbol B1b. Por ello no se puede atribuir el salto publicado de umbral a S4; la referencia histórica y/o el perfil de compilación son variables de confusión. La cláusula del plan «ningún marco S3 crece» sí falla frente a `be8c47c` por las cifras anteriores.

## Árboles, entorno y comandos

- Copias extraídas por `git archive --format=tar be8c47c` y `git archive --format=tar dd26cad` en `C:/nexo-work/val/s4-diag/{base,s4}`. SHA-256 de `src/check/types.rs`: base `aad0c50fc49c219f4d08a22e40bb36513de203644725b4a14ea405700e1079e3`; S4 `c9c0d57a84118ed6c9fa508165a6cf1b8451f8274675fc288425c7ede23b1ae6`.
- `rustc 1.99.1 (58cf8f9c3 2026-10-13)` MSVC. `PATH` incluyó `C:/Users/hackerprod/.rustup/toolchains/1.99.1-x86_64-pc-windows-msvc/bin` y `.cargo/bin`; `TEMP=TMP=C:/nexo-work/tmp`; cada target separado `C:/nexo-work/target-diag-base` / `target-diag-s4`; 4 workers. Las compilaciones se ejecutaron bajo `python -B scripts/heavy-lock.py run --session i07c-s4-diag -- powershell -NoProfile -Command '$env:CARGO_BUILD_JOBS=4; cargo test --lib --no-run'`.
- Perfil de arnés sin debuginfo: `CARGO_PROFILE_TEST_DEBUG=0`, `RUSTFLAGS='-C debuginfo=0 -C link-arg=/MAP:C:/nexo-work/val/s4-diag/<árbol>-nodebug.map'`. Perfil test por defecto: sin `CARGO_PROFILE_TEST_DEBUG`, `RUSTFLAGS='-C link-arg=/MAP:C:/nexo-work/val/s4-diag/<árbol>-test.map'`. `/MAP` solo pide el mapa del enlazador. El binario sin `/MAP` del perfil por defecto de base también falló a 65 KiB; el S4 original sin `/MAP` también falla.
- Sonda: `python scripts/i07c_stack_probe/probe.py <test-exe> --mode TYPES --kib 65|129 --case option_match126|result_match126`, repetida cinco veces con cada binario. Fuente Option SHA-256 `9d702f634956ba899926253718fce6f6e47c99c35ef48367d975996d611c7312`; Result `c15e432a269f65c625d6b17b613039ee004839a8328febd698f3442d8144b45c`.
- Para cada símbolo, dirección tomada de la línea `0001` del `.map`; `dumpbin.exe /DISASM /RANGE:0x<inicio>,0x<inicio+192> <exe>`. La reserva es el operando de `sub rsp,N`, o el `mov eax,N; call __chkstk; sub rsp,rax` del prólogo. Incluye la reserva explícita, no `push`/dirección de retorno ni el resto de la cadena. Los tamaños de tipos vienen de `RUSTC_BOOTSTRAP=1 cargo rustc --lib --profile test -- -Z print-type-sizes`, ejecutado también bajo `heavy-lock.py`.

## Reservas de marco (bytes, ordenado por delta sin debuginfo)

| Función | base sin DI | S4 sin DI | Δ | base con DI | S4 con DI | Δ |
|---|---:|---:|---:|---:|---:|---:|
| `drive::resume_non_match` | 31.592 | 32.040 | **+448** | 32.872 | 33.336 | **+464** |
| `drive` | 33.904 | 34.256 | **+352** | 34.832 | 35.264 | **+432** |
| `match_advance` | 2.808 | 2.936 | +128 | 2.952 | 3.096 | +144 |
| `expr_inner` | 11.936 | 12.032 | +96 | 12.656 | 12.752 | +96 |
| `branch_accept` | 752 | 784 | +32 | 800 | 832 | +32 |
| `check_against_context_one` | 6.336 | 6.352 | +16 | 6.512 | 6.560 | +48 |
| `branch_next` | 432 | 432 | 0 | 480 | 480 | 0 |
| `selection_type` | 2.512 | 2.512 | 0 | 2.656 | 2.656 | 0 |
| `pattern_type` | 2.640 | 2.640 | 0 | 2.800 | 2.800 | 0 |
| `finish_selection` | 784 | 784 | 0 | 832 | 832 | 0 |
| `resolve_pending_projections` | 2.624 | 2.240 | −384 | 2.672 | 2.288 | −384 |
| `require_union` | 896 | 576 | −320 | 928 | 592 | −336 |
| `bind` | 1.248 | 768 | −480 | 1.296 | 800 | −496 |
| `solve` | 70.712 | 67.496 | −3.216 | 71.640 | 68.488 | −3.152 |

En S4 sin DI también se emiten, fuera de línea, `InferenceGraph::begin_visit` (248 B), `child_fact` (328 B), `fact` (184 B), `fact_mut` (184 B) y `merge_child` (184 B). Con DI: 312, 392, 232, 232 y 248 B. No existían en base. Su reserva se suma solo mientras la llamada correspondiente esté activa; estos valores no se deben sumar todos a la vez. `resolve_pending_projections` y `solve` decrecen y no explican el crecimiento observado.

## Tamaños de tipos (bytes; `-Z print-type-sizes`)

| Tipo | base | S4 | Δ |
|---|---:|---:|---:|
| `drive::Frame` | 88 | 88 | 0 |
| `solve::Frame` | 680 | 624 | −56 |
| `SolverBind` | 248 | 136 | −112 |
| `MatchProgress` | 472 | 480 | +8 |
| `BranchAction` | 152 | 160 | +8 |
| `InferenceGraph` | 1.160 | 1.248 | +88 |
| `Checker` | 1.608 | 1.696 | +88 |
| `VisitCursor` / `FactId` | 64 | 8 | −56 (tipos distintos) |
| `VisitFact` | — | 72 | nuevo en el grafo |

`Frame` del `drive` no creció; el tamaño de `SolverBind` tampoco es la causa. El `Checker` conserva en sí mismo el aumento del grafo (+88 B), pero se pasa por referencia en esta ruta. Los +8 B de `MatchProgress` y `BranchAction` contribuyen a temporales, no bastan por sí solos para explicar los +352/+448 B.

## Ruta y llamadas por profundidad

La fuente tiene **un `match` con un comodín** y 126 constructores *de tipo* en el parámetro, no 126 `match` anidados. La ruta de control en `types.rs` es `drive` → `check_against_context_one`/`expr_inner` para el scrutinee → `selection_type` → `match_advance` → `pattern_type`/`branch_next`/`finish_selection`, y para las continuaciones no `match`, `drive` → `resume_non_match` (que puede llamar `resolve_pending_projections` al cerrar un bloque). `selection_type` en § actual usa `Vec<Frame>` y bucle `while let Some(frame) = frames.pop()`; `match_advance` itera brazos/etapas. No hay auto-llamada de estas funciones por cada nivel de `Option` o `Result`. `begin_visit` aparece a la entrada de `drive`; `child_fact` en fronteras de expresión/visita; `fact`/`fact_mut` consultan hechos de esas fronteras. Son nuevas llamadas fuera de línea, pero no una cadena de 126 llamadas. En esta fuente, con un solo comodín, `branch_accept` ni siquiera corresponde a un valor de rama; sus marcos se incluyen para completar la comparación de la ruta general.

El par `drive`→`resume_non_match` es el riesgo principal de pila: sin DI las reservas sumadas son 65.496 B (base) y 66.296 B (S4); con DI, 67.704 y 68.600 B. La diferencia de perfil reduce ese par S4 en **2.304 B**, mayor que el delta S4/base de **800 B** en el perfil sin DI. Estas sumas son indicativas; no representan un high-water medido y los tamaños solicitados al hilo en Windows se agrupan de forma discreta.

## Opciones de ajuste sin cambio semántico

1. **Fijar identidad de compilación y referencia para la puerta de pila.** Repetir S3+B1b y S4 con el mismo `rustc`, perfil, `RUSTFLAGS`, harness y árbol, y conservar hashes de ambos ejecutables. Efecto observado: 65 KiB 5/5 en ambos sin DI; 129 KiB 5/5 en ambos con DI. Esto resuelve la atribución del salto, no reduce el costo S4 ni justifica relajar el requisito de marcos.
2. **Partir los temporales grandes de `drive` y `resume_non_match` en ayudantes pequeños fuera de línea**, o acortar sus vidas antes de la llamada a la continuación. Objetivo cuantificable: recuperar hasta ~352 B y ~448 B respectivamente sin DI (hasta ~800 B en el par); con DI, ~896 B. Estimación como recuperación del exceso medido, no ahorro garantizado. Comprobar prólogos y sonda después de cada decisión de diseño.
3. **Separar la contabilidad de hechos de la ruta que mantiene vivos `Result<Type, Diagnostic>`/`MatchProgress`**, con retorno compacto o subrutina específica de `match`. Se estima un margen de cientos de bytes, acotado inicialmente por los +128 B de `match_advance` y los +8 B de cada tipo de progreso; puede solaparse con la opción 2. El `Frame` de 88 B y `SolverBind` no son objetivos útiles para esta regresión. Marcar simplemente `fact`/`fact_mut`/`child_fact` como `#[inline(never)]` no promete ahorro: ya aparecen como funciones separadas en ambos mapas S4 (estimación ~0 B para esa anotación aislada).

Ninguna de estas opciones se implementó. La evidencia no demuestra que una reducción menor de 1 KiB cambie la banda de umbral 65/129 KiB del perfil con DI, pues la base ya falla a 65 KiB bajo ese perfil.

## Limpieza pendiente
Se verificó que los destinos son directorios normales dentro de C:\nexo-work. La revisión automática rechazó el borrado recursivo con ruta literal y devolvió "blocked by policy". Permanecen C:\nexo-work\target-diag-base, C:\nexo-work\target-diag-s4 y C:\nexo-work\val\s4-diag. No se usó otro intérprete para eludir el bloqueo. El repositorio sigue sin cambios.
