# Nexo — Acuerdos posteriores a la base 0.2

Fecha: 2026-10-19, America/Los_Angeles (2026-10-20 UTC).
Estado: registro de trabajo para F0; no revisión 0.3 del roadmap.

## 1. Base y autoridad

Se conserva la revisión 0.2 como línea base. La recomendación de Claude sobre
cantidad de personas o tamaño del equipo no se adopta ni se añade como requisito.
El propietario indicó que el asistente implementa y un subagente independiente
valida para impedir atajos. No se delega la implementación al validador.

El propietario pidió incorporar lo pendiente en Markdown y respaldar cada paso en
Hackerprod/Nexo. No basta con que algo permanezca en el contexto de una conversación.

## 2. D11 — Closures y capturas: decisión pendiente en F0

Redactar un ADR que decida y pruebe:
- Sintaxis de closures largas y cortas.
- Captura por defecto y reglas explícitas de copia lógica, préstamo y transferencia.
- Propagación transitiva de capacidad de copia, independencia, afinidad y efectos.
- Escape de préstamos locales, captura de recursos y destrucción una sola vez.
- Capturas compartidas y mutables: no eludir exclusividad por anidación.

Debe ser coherente con P02/P03, D09, D10 y T06/T10/T14/T22. Incluir alternativas,
contraejemplos, casos pass/fail y la revisión del validador. No se afirma decidido.

## 3. D12 — Modelo asíncrono: decisión pendiente en F0

Decidir entre declaración async y suspensión inferida como efecto, dejando claras
las interfaces públicas y la inferencia local. Precisar creación frente a avance,
cancelación, recursos retenidos y limpieza. Decidirlo en F0 no adelanta el runtime
asíncrono completo de F5.

D10 ya establece que yield síncrono no crea tarea ni hilo, pero sí es suspensión en
el resumen de next y no puede ocultarse bajo no_suspend durante el lowering.
No se debe reabrir ese acuerdo accidentalmente al decidir async.

## 4. D13 — Expresiones, módulos y visibilidad: decisión pendiente en F0

Precisar if/match como expresiones que producen valores, unificación de tipos,
exhaustividad, efectos y movimientos por rama; visibilidad pub, módulos, paquetes
y reglas de exportación/importación. Evitar conflictos entre paquetes y traits.

D01 exige return explícito y excluye la expresión final implícita en 1.0. Una
closure corta no puede reintroducir esa regla indirectamente sin documentar una
modificación semántica y revisar sus dependencias.

## 5. H-PY — Ajuste de ambición que debe formalizarse antes de medir

Se registra íntegra la propuesta de Claude para que no se pierda:
- Objetivo propuesto: al menos 10× frente a Python idiomático en CPU local.
- Mantener 3× únicamente para comparaciones frente a soluciones vectorizadas
  apropiadas, por ejemplo NumPy.

**Diferencia con la base:** §7.1 de 0.2 exige cociente geométrico Nexo/Python ≤1/3
con límite superior del intervalo ajustado ≤1/2. §7.3 añade referencias vectorizadas
cuando proceda, sin exigir vencerlas 3×. No se ha editado retrospectivamente esa base.

Antes de una medición confirmatoria, un ADR de protocolo debe resolver alcance de
las dos cohortes de cargas, comparadores, intervalos de confianza, tamaños de entrada,
hardware, costes de conversión y memoria, corrección y tiempo de desarrollo. Conservar
las salvaguardas de §7; no elegir cargas después de ver resultados ni anunciar 10×
como logrado. El objetivo nuevo queda registrado como propuesta de cambio pendiente
de revisión, no como resultado ni puerta estadística ya especificada.

F0 no incluye resultados competitivos de rendimiento: su intérprete experimental
no permite confirmar ni refutar la ventaja del futuro backend nativo.

## 6. Inicio de implementación

Rust e IR propia siguen D04. Organización propuesta: frontend (léxico/sintaxis),
comprobación de tipos/propiedad/efectos, IR, intérprete, CLI y corpus pass/fail con
código de diagnóstico esperado. La distribución en crates puede crecer con evidencia;
no es por sí misma una garantía del lenguaje.

Secuencia propuesta por Claude: EX04 reducido y EX01 con E/S simulada; después valores
y recursos T01/T02, mientras se redactan D11–D13. El implementador puede contrastar
primero copia, transferencia y limpieza para reducir riesgo. Identificar todas las
simplificaciones y las primitivas de pruebas; no sustituir el lenguaje por respuestas
precocinadas para las demos.

EX01/EX04 completos con biblioteca real son puertas F2. F0 permite subconjuntos y E/S
simulada, y también exige explorar actualización de datos y flujo de petición simulado.
Dos ejemplos exitosos no sustituyen el corpus ni aprueban F0.

## 7. Revisión y límites de aprobación

Para cada corte: invariantes y alcance → implementación → pruebas reproducibles →
revisión independiente → correcciones y regresiones → commit de respaldo verificado.
El validador no modifica la implementación ni rebaja las expectativas para aprobarla.
Registrar PASS, FAIL y NOT EVALUATED; un parse error accidental no prueba propiedad.

La revisión por un subagente requerida por el propietario no demuestra automáticamente
el requisito documental de dos revisores sin discrepancias, ni las futuras revisiones
externas. Esos requisitos se conservan visibles y pendientes hasta que exista evidencia
o una decisión explícita del propietario que cambie el proceso. No se necesita una
plantilla de personal para comenzar a explorar.

## 8. Integridad de la base

SHA-256:
- Nexo_ROADMAP_rev0.2.md: 9a7ed7336a920f82bfcbd92030761320e4e7b334991a95acdc1ee5252144d62d
- Nexo_CAMBIOS_rev0.2.md: 118c00274155f0a5a8534b6230ce2e8f079bd9829598aa28336cfa1ff3d38a8c
