# I07c S3/B1b/S4: umbral de pila TYPES

Fecha: 2026-11-21. Copias por git archive en C:\nexo-work\val\s3stack\{s3,b1b,s4}. Worktree C:\projects\Nexo-i07c en solo lectura.

## Veredicto

(a) Regresion real de pila entre S3 y B1b para ambas filas. Con debuginfo (perfil test por defecto), S3 6b08c07 pasa a 65 KiB 5/5; B1b be8c47c desborda a 65 KiB 5/5. S4 8cd5afe tambien desborda a 65 KiB 5/5. Los tres pasan a 129 KiB 5/5. Sin debuginfo, los tres pasan a 65 y 129 KiB 5/5. Por tanto, el dato S3 de 65 KiB no exige otro perfil para estas filas.

"Pasa" = exit 0 del probe. "Desborda" = exit 1 del probe, codigo del hijo 3221225725 y mensaje "has overflowed its stack". Cada celda expresa n/5.

| Arbol | Perfil | KiB | J16-S1C-option_match126 | J16-S1C-result_match126 |
|---|---|---:|---:|---:|
| S3 6b08c07 | sin debuginfo | 65 | 5/5 pasa | 5/5 pasa |
| S3 6b08c07 | sin debuginfo | 129 | 5/5 pasa | 5/5 pasa |
| S3 6b08c07 | con debuginfo | 65 | 5/5 pasa | 5/5 pasa |
| S3 6b08c07 | con debuginfo | 129 | 5/5 pasa | 5/5 pasa |
| B1b be8c47c | sin debuginfo | 65 | 5/5 pasa | 5/5 pasa |
| B1b be8c47c | sin debuginfo | 129 | 5/5 pasa | 5/5 pasa |
| B1b be8c47c | con debuginfo | 65 | 5/5 desborda | 5/5 desborda |
| B1b be8c47c | con debuginfo | 129 | 5/5 pasa | 5/5 pasa |
| S4 8cd5afe | sin debuginfo | 65 | 5/5 pasa | 5/5 pasa |
| S4 8cd5afe | sin debuginfo | 129 | 5/5 pasa | 5/5 pasa |
| S4 8cd5afe | con debuginfo | 65 | 5/5 desborda | 5/5 desborda |
| S4 8cd5afe | con debuginfo | 129 | 5/5 pasa | 5/5 pasa |

## Ejecutables medidos: SHA-256

Ubicacion: C:/nexo-work/target-s3stack-<arbol>/debug/deps/<test-exe>.

| Arbol | Perfil | Test-exe | SHA-256 |
|---|---|---|---|
| S3 | sin debuginfo | nexo-61fce89390bcc650.exe | 8b426dbbe5d0336099e2b5cde78f85ac098673d2be9e2e30d6039b83b2612783 |
| S3 | con debuginfo | nexo-ae65c584bd3eb2e6.exe | 7e384f8c40bb031d97c9c20ac4e446ec433c4190e0eb5446bae00e6c07a1eb13 |
| B1b | sin debuginfo | nexo-61fce89390bcc650.exe | 468e522c4f4e56e08417d8cadea98f166d572512e99a7f5b53d55e60b08926ea |
| B1b | con debuginfo | nexo-ae65c584bd3eb2e6.exe | 58b4f9931b0a5f5280b91b38fd022bb7694413e48ce2a708ebf48ede37cc724d |
| S4 | sin debuginfo | nexo-61fce89390bcc650.exe | ee662053b6d0197128d91798886984ad4bfb4ec9423e707a1b34888dfb63f953 |
| S4 | con debuginfo | nexo-ae65c584bd3eb2e6.exe | 244d4a7a2df4b40e2d25f2619cccb307e1dd98aea9be9e39e7e7fbdb8781dd34 |

El probe.py es identico en los tres arboles: SHA-256 37940070a7fa1cb0b646e14cb98c84a14d9ffbf6327b9e344fd21c4d13237f43, blob Git a0638ed868af8b2d75029d5f6f91d73d0b44c3f1. No se copio otro probe. SHA-256 de los fuentes generados: option_match126 9d702f634956ba899926253718fce6f6e47c99c35ef48367d975996d611c7312; result_match126 c15e432a269f65c625d6b17b613039ee004839a8328febd698f3442d8144b45c.

## Comandos y entorno

En cada copia: PATH antepuso C:\Users\hackerprod\.rustup\toolchains\1.99.1-x86_64-pc-windows-msvc\bin y C:\Users\hackerprod\.cargo\bin; TEMP=TMP=C:/nexo-work/tmp; CARGO_BUILD_JOBS=4; CARGO_TARGET_DIR=C:/nexo-work/target-s3stack-<arbol>; rustc 1.99.1 (58cf8f9c3 2026-10-13). No se uso /MAP.

Sin debuginfo: CARGO_PROFILE_TEST_DEBUG=0 y RUSTFLAGS=-C debuginfo=0. Con debuginfo: CARGO_PROFILE_TEST_DEBUG sin definir y RUSTFLAGS vacio. Cargo indico "test profile [unoptimized]" y "test profile [unoptimized + debuginfo]", respectivamente.

La compilacion inicial S3 sin debuginfo se lanzo desde PowerShell mediante:
python -B scripts/heavy-lock.py run --session i07c-s3stack -- powershell -NoProfile -Command '$env:CARGO_BUILD_JOBS=4; cargo test --lib --no-run'

PowerShell interrumpio la captura de stderr de Cargo despues de producir el ejecutable. Sus cuatro combinaciones se sondearon 5 veces cada una. Las cinco compilaciones restantes se lanzaron desde la copia correspondiente con:
python -B scripts/heavy-lock.py run --session i07c-s3stack -- sh -c 'export CARGO_BUILD_JOBS=4; cargo test --lib --no-run'

Cada combinacion se sondeo mediante cinco invocaciones independientes, sustituyendo los valores entre llaves:
python C:\nexo-work\val\s3stack\{arbol}\scripts\i07c_stack_probe\probe.py C:\nexo-work\target-s3stack-{arbol}\debug\deps\{test-exe} --mode TYPES --kib {65|129} --case {option_match126|result_match126}

No se borraron copias ni targets; el coordinador hara la limpieza. El cerrojo quedo liberado tras cada compilacion.
