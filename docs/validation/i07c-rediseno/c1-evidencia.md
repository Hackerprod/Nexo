# I07c R6 C1 — evidencia de implementación

Especificación/artefactos normativos importados de `origin/i07c-rediseno` en `1e84543df9321e380869d18ade1e5b8dbfd7e178` (R6-final v15b). `verify_r9_contract.py` y `verify_corpora.py` completaron; `i07c_rd_oracle.py --matrix` coincidió en 45/45. SHA-256: `r9-commit-gates.json` AC9FCFF5279E37A199B3D3F0DA63583B7703860452940BED7B6D53908BB73DFC; `candidate_gate.py` 48DF7FB62C839A165FD99B40AF2E626385E70476B2DEF09D7422CDA600F1281C; `owned_rows_gate.py` 1820CE8BB26374C9630D309A9ABC509CF4E9452D0E9CCE25E76DF80959FB3E78; `verify_r9_contract.py` 12488AE91E93CD4527EC98960BADA423D68FEC112B1FD4285F0B7C149BBCF9E.

## Gate cruzado y filas propias

Contra la base ejecutable `31d91db`, `candidate_gate.py --commit C1` completó los 20 corpus y 151.188 filas en debug y release. Ambos informes dieron OK 62.755, C 16.678, U 71.753, H 2; fallos 0, A/D/A-unmodeled/H-divergent 0. `class_by_row` coincidió fila por fila entre perfiles (151.188/151.188). C1 conserva las 103 filas de `match` de `main`.

SHA-256 de binarios (main debug/release; candidato debug/release): `65E6CC75FC3BCE3CAC0F4772F744E913C60A0EDBA11B666BDDFD9DDD86946E7B`, `FF0A35A4F1D0210AE3E196FFA0050E2DDDC013770B700AD8A9A41473A6338AD8`, `602EDC6C9AD3A622D973912ADB691BA45F5B49E888009C4B989446123DC6C2DF`, `793601E4E46091E1623EEE9E93C06E3CC8D64FC677EB927A755D1D92831EEF97`. SHA-256 de informes candidate debug/release: `a080a16682f1cbb99193a929a2c882116b5ad841f6d97deb488c34bf61e984be`, `36927266f758593d4d2d5aa07bd74f4656999cd9b48d5aed8b2dedb3d3660b7f`.

`owned_rows_gate.py --commit C1` en debug y release: dirigidas 3/74, matriz 0/45, frozen13 0/13; F06=`main`; fallos 0. SHA-256 de informes debug/release: `864238da2ba53d171f62aced6fe9a5da2e8fbe7004b225802026496bda6f13b6`, `e99a3df63371dc2030cff10bfafd96605afbce9742f8b304d75b7cbcea06a710`.

## Puerta mecánica

`sh scripts/validate.sh` completó con `cargo fmt --all -- --check`, `cargo clippy --locked --offline --all-targets -- -D warnings`, nextest default debug 2.220/2.220 y release 2.222/2.222, y `f5-stack65.py`: `option_match126` 5/5 y `result_match126` 5/5 a 65 KiB. Los conteos son los 2.219/2.221 tests base más la regresión del rollback y el alias registrado por la suite.

Con rustc 1.99.1, perfil test por defecto con debuginfo, el par vigilado midió `drive` 27.584 B + `resume_non_match` 32.936 B = 60.520 B; margen frente a 66.560 B: **6.040 B** (mínimo requerido: 1.000 B). SHA-256 ejecutable `/MAP`: `6a6050a0735efdcff12f53ceb535c6b4c784e5a3399c1b9e7bfaa6fc3489d827`; mapa: `f3ccea5513d50598dab1728307113c1c84a39edb82ae3b11d3f4606ac8b4fe9f`. `f5-frame-sizes.py --selftest` pasó.

El primer commit porta la referencia causal y protege la comprobación de profundidad tras rollback; el segundo adjudica las dos expectativas aprobadas de proyección y agrega la regresión de tres variantes. Se mantuvieron las demás aserciones existentes. Evidencia de implementación para revisión D/D'; no es un veredicto de validación independiente.
