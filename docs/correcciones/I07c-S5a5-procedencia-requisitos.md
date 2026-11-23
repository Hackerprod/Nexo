# I07c-S5a.5 «procedencia» — requisitos del plan (FAM-E de procedencia, con F10/F11)

**Estado: requisitos para el subagente A (Flujo B, plan nuevo); no autoriza implementar.** OBLIGATORIO antes de cerrar I07c (decisión del orquestador, 2026-11-22). Origen: límite conocido de S5a.1 declarado en `docs/validation/i07c-s5a/limites-conocidos.md` (causa, alcance y filas P1, P2, w1, w3, c3, P4, P4pos y residuos). Informes: `I07c-C-S5a1-hallazgo.md`, `I07c-S5a1-enmienda-r1.md`, `-B-r1.md`, `-B-r2.md`.

## 1. Alcance

Núcleo (procedencia de `expected_type` y diagnósticos de F10/F11). Cierra la deriva exacta: asignación aceptada previa + asignación rechazada posterior sobre un binding con entrada intrínseca/no intrínseca en `assignment_type_plan`, capas 1..4. Incluye los residuos F10 (2 `NX-E0103` de la FAM-E reducida de 101 B del hallazgo de C) y F11 si siguen presentes en la base vigente (C midió 0 en `main` para F11).

## 2. Causa raíz que el plan debe confirmar

`assignment_type_plan` solo guarda RHS `intrinsic_assignment_type`; `rejected_assignments` es binario por binding mientras que el ancla de la relación es por asignación; `seed_cap` bloquea `read_plan` en capas ≥ 3. Hay dos criterios (con entrada de plan: clave por sitio; sin entrada: por binding) que el plan debe unificar en un mecanismo único.

## 3. Reapertura del diseño: alternativas a medir

1. Veredicto por (sitio, inicio del RHS) de la enmienda r2 de A: B mostró que introduce un `NX-E0102` espurio `[166,167)` en P4 (`B-r2` motivo 1); medir si se corrige o se descarta.
2. Ancla por asignación (cada asignación aceptada registra su RHS/origen como fuente de la relación; el veredicto binario por binding desaparece o queda solo como compuerta de terminación).
3. Evidencia concreta (`concrete_evidence`) por sitio con origen aceptado: B/A midieron que «caer siempre a evidencia» reintroduce E0103 espurio en F01, F02, F04, F05, F09, O05 (35 diferencias); la evidencia puede venir de OTRO sitio.
4. Compuerta `seed_cap` y capas ≥ 3: decidir si se levanta con garantía de terminación (`seed_cap` solo decrece, tope 129) o se sustituye; medir los ~4 de 12.000 aleatorios que cambian sin derivar (r1 §5).
5. Unificar los criterios con y sin entrada de plan: B (`B-r2` punto 2) mostró que cambia P2 (E1303 primario `[108,114)` → `[101,102)`) y la relación de `[133,134)` (`[78,79)`); derivar del ADR (D-REF, l.301/l.309) antes de aceptar.

Cada alternativa se mide con binarios de `main` y del candidato, diferencial 3.674 fuentes × 3 perfiles × `check`/`run`, y las permutaciones de orden (49 por programa). Terminación e independencia de orden se demuestran (veredicto solo en planning, `read_plan` solo en la pasada final).

## 4. Laguna y Flujo A

Si la derivación de P2 `[108,114)`/`[101,102)`, `[133,134)`, P4 (relaciones de E1303), `generated:1411` o `generated:1681` exige una decisión con más de una respuesta razonable, va por Flujo A (ADR/addendum con ejemplo válido e inválido); no se fija con tests `adr_gap_*`. Los residuos P5 y las 4 aleatorias (fuentes no conservadas) se regeneran con `docs/validation/i07c-s5a/barrido-familia.py` y se adjudican.

## 5. Criterios de aceptación

1. Los 7 repros con esperado derivado: P1 `[93,101)`, P2 `[119,124)`, w1 `[93,100)` sin E0102 `[87,88)`, w3 `[119,126)`, c3 `[93,99)`, P4 `[165,168)` y su positivo con el E0102 `[166,167)` previo corregido o declarado límite de `main` (decisión escrita), cada uno con positivo pareado (`check` y `run`, debug y release).
2. Ningún diagnóstico espurio nuevo: en P4, sin E0102 `[166,167)` adicional; ningún E0103/E0102 que `main` y el candidato no emitan.
3. Diferencial de procedencia contra `main` que enumere TODAS las diferencias de relaciones `expected_type`/`found_type` (no solo código/primario), clasificadas y derivadas; ninguna sin adjudicar (directriz 2026-11-21 §2).
4. `barrido-familia.py` (semilla fija, corpus congelado por SHA-256) sin diferencias PROCEDENCIA fuera de las adjudicadas y sin DIAGNOSTICO nuevo.
5. Mutantes del mecanismo muertos con programas (sin `failed`, sin rama por sitio, clave por binding); reserva de pila ≥ +1.000 B (`drive`/`resume_non_match`) y puerta completa verde; límite de `limites-conocidos.md` cerrado y retirado de `Pending.md`.
