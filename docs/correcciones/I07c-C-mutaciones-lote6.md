# I07c - lote 6: regresiones de mutantes con programa testigo

Rama de partida: i07c-estructural en a6918ca. El único cambio de código está en tests/i07c_solver_depth.rs. La población congelada es docs/validation/i07c-mutations-ccf/population.json (SHA-256 a6f21ab42c6bc0436f7b12f9053519824e819ae762f67f12f3bb8cb741a103f4). Las fuentes están transcritas literalmente en los tests; el índice indica su posición en el corpus de caza de 32.631 programas.

Cada test llama compile_scalar_with_profile con ReducedProfile::from_id("f0-0003-b-v1"), run_entry=false (check), y compara en orden todos los diagnósticos: código, subcausa, regla, etapa/operación, span/token, relaciones y data, además de los campos comunes del esquema. Los esperados provienen de las salidas de referencia indicadas.

| ID | Programa | Esperado | Resultado | SHA-256 blob mutante |
|---|---:|---|---|---|
| M00436 | 0 | base ccf6df1 | matado por survivor_m00436_check_diagnostics | 80fc1c72c70967019db107035ad8b883927edb428fe4fdd575b6d3c4a68003c8 |
| M00466 | 305 | base ccf6df1 | matado por survivor_m00466_check_diagnostics | 3cf25a1a15c1fe0a6974cfd269f4c1cafbcfe9f96cf3cc3bdb2f9b5f7b99707d |
| M00474 | 4941 | Bloque I efe299f | matado por survivor_m00474_check_diagnostics | beae5e975e3ab50a5dd03d71cc9c867dc48dea4411f29808939b08281ac4c7c1 |
| M00475 | 4941 | Bloque I efe299f | matado por survivor_m00475_check_diagnostics | e089b450a46355bb438d6cabaa872cfc019438c8ff310fc2593fd1a23b30bd79 |
| M00494 | 12092 | base ccf6df1 | matado por survivor_m00494_check_diagnostics | d23e9e007f90f3da1cc9c8a739e2bdc0500d918fe820e4a44e33e8d5997b1ced |
| M00588 | 6040 | base ccf6df1 | matado por survivor_m00588_check_diagnostics | 75ea482068a44e5f92d42c67c5e6241703fd9e7fd615888969bdf3dc5fa7ebd2 |
| M00655 | 4667 | Bloque I efe299f | matado por survivor_m00655_check_diagnostics | b0e2d27701f0bc4033197bea434a0f9e72f150a4e09d53339f8032373de6aae3 |
| M00701 | 6203 | base ccf6df1 | matado por survivor_m00701_check_diagnostics | 45822e87dac6a3f059901a3ac1f03c6151d879af645cd9b15004c55875f1b201 |
| M00702 | 6203 | base ccf6df1 | matado por survivor_m00702_check_diagnostics | 5271c14c3346cd230f23405aab75c3f8449eff8f1c96ad8c75cad4c8ff4696f0 |
| M00703 | 6203 | base ccf6df1 | matado por survivor_m00703_check_diagnostics | c3f1f9d853aa6e093eead2893c07754bf2b1fa479b0f69ad3dcf85d10ed60bf5 |

El blob original src/check/types.rs tiene SHA-256 572dca78c597a56b9f0cf10b676f658ca69afb1e6ed7c057c73b97f22c1fab1e; se exige ese mismo SHA-256 tras cada restauración. Cada blob mutante se calcula aplicando una única vez el literal y reemplazo exactos del population.json congelado al blob original.

## Justificación de los esperados del Bloque I

- **M00474 y M00475 (programa 4941):** F0-0002 §4.1.2.5 clasifica como derivado el diagnóstico cuya evaluación exige el tipo de una expresión contaminada; §4.1.2.2 mantiene NX-E0103 en los constructores Array con obligación propia. l1[Some([])] no proporciona elemento válido al array externo [l1[Some([])]]: el NX-E0103 de [136,150) de la base es derivado y se omite. El [] interno de [145,147) conserva su obligación independiente.
- **M00655 (programa 4667):** El Len(Some([1])) inválido produce NX-E0102/not_indexable y contamina el scrutinee del match (F0-0002 §4.1.2.5). F0-0003 §§3.2 y 8.2 exigen un tipo concreto válido del scrutinee para NX-E1325/constructor_type. Por ello se omite el diagnóstico derivado en Some(v) [168,175), preservando los NX-E0103 independientes.

## Verificación

- Debug, copia desechable: timeout 900 cargo test --locked --offline --test i07c_solver_depth survivor_ -- --nocapture -> exit 0, 10 passed.
- Release, copia desechable: timeout 900 cargo test --release --locked --offline --test i07c_solver_depth survivor_ -- --nocapture -> exit 0, 10 passed.
- Por cada ID: aplicación del literal/replacement de la población congelada en copia desechable; timeout 900 cargo test --release --locked --offline --test i07c_solver_depth survivor_mXXXXX_check_diagnostics -- --exact --nocapture -> exit 101, ese test FAILED (10/10). La ejecución dirigida verificó después de cada mutante el SHA-256 restaurado 572dca78c597a56b9f0cf10b676f658ca69afb1e6ed7c057c73b97f22c1fab1e. Ninguna muerte se atribuyó a error de compilación o plazo agotado.
- Puerta completa en clon limpio y con cerrojo heavy-lock: timeout 900 sh scripts/validate.sh -> exit 0; cargo fmt --check y cargo clippy -D warnings correctos; nextest --profile default: debug 2080/2080 passed, release 2082/2082 passed.
