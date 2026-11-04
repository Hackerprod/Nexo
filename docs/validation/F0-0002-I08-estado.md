# F0-0002 I08 — estado medido

Resumen de mediciones registradas; revisión independiente y aprobación de F0 pendientes.

| Comprobación | Resultado |
|---|---|
| `cargo test --locked --offline` | verde; 1303 pruebas activas |
| `cargo test --locked --offline --release` | verde; 1305 pruebas activas |
| RD normativo check/run | 152/152, 152/152 |
| `sh scripts/validate.sh` en clon limpio | verde |
| `cargo clippy --all-targets -- -D warnings` | verde |
| `cargo fmt --all -- --check` | verde |
| Campaña tipada debug | verde; semilla 9306578997655527858; 100000 programas; 9005 negativos |
| Campaña tipada release | verde; semilla 9306578997655527858; 300000 programas; 26941 negativos |
| Fuzz de solidez Result | verde |
| Pilas 128/256/512 KiB | verde |
| Mutaciones de producción | 87 enumeradas; 87 ejecutadas; 80 detectadas/no compilan; supervivientes: M00001, M00013, M00014, M00020, M00022, M00026, M00033; inconclusas: ninguna; plazo agotado: M00017 (release: detectada (i08: no termina en 60 s; base 1.57 s); debug: detectada (i08: no termina en 60 s; base 4.65 s)) |

Los resultados y límites de los requisitos S09, S20–S21, P10–P11, PP05, R01–R03, F15 y NX-E0112 se explican en las notas de implementación. Este estado no constituye un veredicto de revisión independiente.
