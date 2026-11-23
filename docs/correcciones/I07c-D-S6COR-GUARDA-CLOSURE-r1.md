# I07c-D-S6COR-GUARDA-CLOSURE r1 — validación D (CL) de 5534709

Veredicto: **PASS** (sin defectos bloqueantes reproducibles).

## Evidencia
1. Fidelidad al plan: el diff contra origin/main toca solo src/ir/effects.rs, scripts/i07c_span_audit.py (clave 1223 a 1232), scripts/i07c_guarda_closure_diff.py (C0), tests/i07c_group_primary.rs, tests/j05_match_guards.rs y docs/correcciones (plan e informe). Sin cambios en src/check, ADR ni baseline. Lo único retirado de tests existentes son los campos de las 12 filas head_*_guard (reubicadas a GUARD_CONTRACT_CASES, aprobado). Sin parches por patrón: la arista de llamada replica la de find_witness (CallDirect y CallClosure) y las guardas de closure se indexan por el `site` de `$closure:<site>`. Sin código muerto detectado.
2. Puerta: `sh scripts/gate.sh 5534709... --verify` imprime «registro GREEN verificado»; SHA-256 de scripts/validate.sh del commit = origin/main (7a1c7a19...130df); selección nextest exit 0; 2213 ejecutables en el log.
3. Los 12 primarios derivados por bytes con un script propio (inicio de guarda tras `true if ` + nº de agrupaciones externas; token = racha de identificador) y comparados con la salida real de check y run (f0-0003-b-v1) del binario del commit, con los dos diagnósticos del mismo primario (contrato E0415/E0414 y NX-E1305/guard_effect):

| Fila | Primario | Token | Códigos observados |
|---|---|---|---|
| no_io_name_1 | [83,84) | f | E0415 + E1305/guard_effect |
| no_io_name_2 | [84,85) | f | idem |
| no_io_literal_1 | [41,43) | fn | idem |
| no_io_literal_2 | [42,44) | fn | idem |
| no_io_if_1 | [83,85) | if | idem |
| no_io_if_2 | [84,86) | if | idem |
| no_suspend_name_1 | [93,94) | f | E0414 + E1305/guard_effect |
| no_suspend_name_2 | [94,95) | f | idem |
| no_suspend_literal_1 | [46,48) | fn | idem |
| no_suspend_literal_2 | [47,49) | fn | idem |
| no_suspend_if_1 | [93,95) | if | idem |
| no_suspend_if_2 | [94,96) | if | idem |

   Coinciden con el ADR F0-0002-lagunas (fila de guarda de match: dos diagnósticos del mismo sitio, orden por código, mismos pasos, sin relaciones en el E1305). La cadena del primer paso es `closure.call` con entidad `function:<offset del fn>`; el segundo es `intrinsic.call`.
4. Diferencial propio base (git archive origin/main) vs candidato, check y run, perfiles f0-0003-b-v1, f0-0005-b-v1 y f0-0002-b-v2, debug y release: 118 programas propios (closures con y sin efecto, literal, anidadas, por nombre, con parámetro, captura, enlace de patrón, varias llamadas, guardas dentro de cuerpos de closure, gemelos `fn`). 0 anomalías, debug==release. Paridad closure == literal == `fn` en las 24 combinaciones (perfil x verbo x efecto) y en los gemelos con contrato (no_io, no_suspend, no_alloc, no_block); orden y cadenas iguales al gemelo `fn`. C0: `--self-test` PASS; `--corpus` (840) PASS en debug y release, debug==release en base y candidato; `--fuzz 3000` PASS (0 fallos, 6136 de 12000 salidas cambian, igual que el informe de C). RD `--verify`: 152/152 pares, 0 diferencias. Auditor de spans y su self-test PASS.
5. Mutación inversa (copia desechable de HEAD, SHA-256 inicial f0a53089...a193 restaurado y comprobado tras cada mutante): M1, M2, M3, M4, M5 y M6 MUEREN (M1-M3 por 5 tests, M4/M5 por `guard_inside_a_closure_body_is_checked` y la regresión del intrínseco directo, M6 por `guard_calls_report_the_contract_and_the_guard_effect_at_one_site` y la prueba CLI). Supervivientes R000/R006/R015/R016: intenté exhibirlos (closure con Print y MockUnknown, orden de efectos, destinos no fuente, entidades) y no hallé programa distinto del gemelo `fn`; se mantienen como no bloqueantes.
6. Pila: este corte no toca src/check (drive/resume_non_match sin cambios); f5-stack65 forma parte de validate.sh, cuyo registro es GREEN. No remedí aparte los marcos con f5-frame-sizes (innecesario sin cambio en types.rs).

## Hallazgos
Ninguno bloqueante. No bloqueante: la ejecución de la suite completa por D se apoya en el registro GREEN verificado (no repetida en frío); clave 1232 del auditor es la medida por C (el plan decía 1226 y delegaba el recálculo).
