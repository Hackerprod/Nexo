# Resumen del barrido de familia (S5a.1, límite de procedencia)

Fecha: 2026-11-22. Comando: `python -B docs/validation/i07c-s5a/barrido-familia.py --base .scratch/cand-bin/base-release.exe --cand .scratch/cand-bin/cand-release.exe --n 500 --seed 1 --extra-dir docs/validation/i07c-s5a/limites` (también con los binarios debug `base-debug.exe`/`cand-debug.exe`: resultados idénticos).

- `main` = `.scratch/cand-bin/base-release.exe`, SHA-256 `bc46facdf2ff211bedda3d038a049b4437f732bddeb42167c9d6383571699969`.
- Candidato = `.scratch/cand-bin/cand-release.exe`, SHA-256 `998e4fad060e23762d32e33f59ddd0e5e9282d0c030f0d80bb16b88268ab0a6c`.
- Corpus congelado: 510 programas (500 generados con semilla 1 + los 10 de `limites/`), perfil `f0-0002-b-v2`, SHA-256 `b8f84342ae9a42cbe17507f077b5aecac9de2a17d332359b7cbf410f93bee77b` (concatenación `nombre\0contenido\0` en orden de nombre).
- Diferencias: PROCEDENCIA = 39 (código, subcausa, primario y `data` iguales; difieren solo relaciones), DIAGNOSTICO = 72 (difiere código, primario, tipos o recuento), FALLO = 0. Sin pánicos ni salidas no JSON.
- De los 10 de `limites/`: PROCEDENCIA = P1, P2, P3, c3; DIAGNOSTICO = P4, w1, w3 (por los diagnósticos espurios de `main` que S5a.1 elimina), generated1411, generated1681; sin diferencia = P4pos.

Lectura y límite del resultado: el guion NO adjudica; lista todas las diferencias para que D y D' las deriven del ADR (l.301/l.309). Las 72 de clase DIAGNOSTICO incluyen la eliminación intencionada de E0103/E0102 espurios de `main` (propósito de S5a.1) y cambios de tipos/primarios (p. ej. `gen-1-0019`, `gen-1-0026`) sin derivar aquí: no se afirma que sean mejoras. El corpus generado es más amplio que la familia declarada (muchos programas combinan varias asignaciones incompatibles), por lo que no demuestra por sí mismo que el límite sea exactamente el declarado; D/D' deben comprobar que toda diferencia PROCEDENCIA tiene asignación aceptada previa y rechazada posterior sobre un binding, y adjudicar las de clase DIAGNOSTICO.
