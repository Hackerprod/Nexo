# Evidencia C0 — R6

Base de comparación: `31d91dbebd31c35b3cd09095c2d8e65b96c80548`. La clasificación y su generador se copiaron de `origin/i07c-rediseno` commit `32aadf373b3880bcab6219f8689f9cb2bea5c09d`; los inventarios congelados y scripts de corpus/gates provienen de `origin/i07c-rediseno` commit `8727a264861fdfbd3fb16d9dacaf539d48676840`. La v9 no cambia el alcance de C0.

## Salida observable

`c0-exact-gate.json` registra comparación byte a byte de `check` y `run`, en debug y release, sobre 151.188 filas/1.209.504 invocaciones de los 20 corpus/perfiles, más las 103 filas y 824 invocaciones de `match`. Resultado: cero discrepancias en ambos grupos. SHA-256 de binarios base: debug `1d000977ffb809ff6392b1e6e03d8a76c80518f5ef8745c4a9f96ae8943d1ac4`, release `01ab68a476e930ff778048dca0d0ad3cad7dac19b2db34e611b593c624c5a26d`; candidato: debug `290349762f450cbbfc80c5e34d49cc438f5ef52df3ef236e4d5afd6dbc2ad496`, release `aee8aa7e4830cd442f0eb4a4257a364ada408acdd7ca2579900d8b3b1dbbfc9c`.

## Pasadas e inventario

El inventario congelado contiene 145.189 pares únicos fuente/perfil: 151.188 filas base producen 145.049 pares únicos, `match` añade 47, dirigidas añade 48 y matriz añade 45. SHA-256 del inventario: `5378070878affcb70342427fbe9c0ad680d59827976c6f9c84344ac1a34961aa`. El medidor recorrió el inventario completo en debug y release: 358.392 registros por build, histograma de `passes` `{0: 20600, 1: 337792}`, máximo 1 (límite 3), suma `planning_calls` 609.866 por build. Cero pares ausentes y cero timeouts. Los casos que no entran al checker registran cero pasadas; `planning_calls` permanece en columna separada.

## G3 y pila

El auditor estructural verificó los 240 usos de `span_key`: Completion 1, ConstructorDuty 7, FactId 47, JoinRecord 1, OccurrenceId 17, Pending 42, ResultStatus 120 y span de salida 5. El self-test confirma que acepta desplazamientos de línea y rechaza tanto un uso crudo plantado como una reclasificación del inventario; fija sus recuentos y su SHA-256. SHA-256 del blob LF de `c0-span-key-usos.json`: `01929eeaec2d9d25278b6d408f52dda72dc4f51559ee5faa9186935e17f4c32d`.

El test estructural conserva también el self-test de `scripts/i07c_span_audit.py`, que cubre accesos `.span()` crudos en `names.rs`, `effects.rs` y lowering. El oráculo `scripts/i07c_rd_oracle.py` se importa de `origin/i07c-rediseno` con SHA-256 `afc2459da8f539b5958223f6f972e46c4aeef8b6fd266803b7079fbbeb137967` para que las herramientas de medición funcionen desde esta rama.

Con rustc 1.99.1 y perfil test con debuginfo: base, `drive` 27.536 B, `resume_non_match` 33.080 B, pareja 60.616 B y margen 5.944 B; C0, 27.680 B, 33.144 B, pareja 60.824 B y margen 5.736 B. Aumento de pareja: 208 B (menor que 500 B); margen final sobre 66.560 B: 5.736 B. `scripts/f5-stack65.py`: `option_match126` 5/5 y `result_match126` 5/5.

## Puerta completa

`sh scripts/validate.sh` terminó con exit 0 en un clon limpio en `31d91db` con el parche C0 aplicado, bajo `scripts/heavy-lock.py`, `CARGO_BUILD_JOBS=4`, `NEXTEST_TEST_THREADS=4`, `CARGO_TARGET_DIR=D:/nexo-work/target-luna-c0c1`. `cargo fmt --check` y `cargo clippy -- -D warnings` pasaron. Nextest default: debug 2.218/2.218, 73 ignorados; release 2.220/2.220, 74 ignorados. La puerta ejecutó también la sonda de pila anterior.
