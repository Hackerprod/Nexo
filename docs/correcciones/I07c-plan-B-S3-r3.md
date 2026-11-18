# Validación B del plan r13–r15 (S3) — ronda 3 (verificación dirigida)

Base: `git archive origin/i07c-estructural-r2` (8982fcb). Diff 0beb4ac..HEAD: solo se añade la sección r15
(7 líneas) y el informe de B ronda 2; ningún otro archivo, sin cambios en r13/r14.

## Veredicto: PASS

1. `data` de la extensión profunda: r15 fija `{"expected_type":"Bool","found_type":"I64"}`, primario
   `[401,402)`, relaciones `[16,272)` / `[401,402)`. Coincide con F0-0002 §4.1.2.6 («tipo completo de la
   expresión señalada», no serializar el elemento con el span del array) y con el caso literal del ADR
   (§4.1.2.2, «Gemelo negativo con tipo adelantado»). Offsets recalculados (16 + 256 = 272; +3 = 275;
   275 + 126 = 401) y ejecutados en la base en la ronda 2: salida idéntica. PASA.
2. Perfil: `f0-0002-b-test-types-32-v2` aceptado por la CLI (ronda 2: NX-E0003 type_solver_steps limit 32
   observed 33 con 64 bindings). En r13–r15 los identificadores `f0-000N-*` son solo `f0-0003-b-v1`,
   `f0-0002-b-test-types-32-v2` y el viejo `f0-0002-test-types-32-v2`, que permanece únicamente en el
   texto histórico de r14 y en la cita de r15 que lo declara inválido. PASA.
3. r15 no cambia nada más. PASA.
