# Nexo — Flujo de trabajo de agentes

Objetivo permanente: un lenguaje que tome lo mejor de los demás (ver roadmap §2) y sea
**agent friendly**. Autoridad documental: `docs/baseline/` (rev 0.2, intacta) y
`docs/F0_ACUERDOS.md`. No se crea una rev 0.3.

## Roles

- **Coordinador (Claude, juez):** no implementa. Lanza a los demás, relaciona veredictos,
  decide la siguiente fase, verifica commits y estado remoto. Su rol es que todo fluya.
- **Codex (modelo Astra):** diseña arquitectura/ADR cuando algo no está en el roadmap.
- **Codex (modelo GPT 6 Luna):** implementa (opción barata).
- **Subagente Sonnet 5.5 (esfuerzo medium):** valida, vía `.claude/agents/nexo-validator.md`. Nunca implementa ni rebaja requisitos para aprobar.

## Flujo A — Fases o mecánicas no planteadas en el roadmap

1. Codex (Astra) propone arquitectura (ADR en `docs/adr/`).
2. Subagente Sonnet 5.5 valida la propuesta contra el objetivo: el roadmap y la idea central
   (lenguaje que toma lo mejor de los demás y es agent friendly).
3. Si Sonnet no da **PASS**, se especifica por qué y se itera de nuevo con Astra.
   Se repite hasta PASS.

## Flujo B — Cuando ya no hay nada por diseñar, solo construir

1. Codex (GPT 6 Luna) implementa.
2. Subagente Sonnet 5.5 valida robustez y detecta desvíos (respecto del ADR, el roadmap y
   la idea central).
3. Si da **PASS**, se comienza la siguiente fase. Si no, el coordinador especifica el motivo
   y ordena a Codex corregir, iterando hasta PASS.

## Reglas comunes

- Implementador y validador siempre distintos. Al validador se le da el código y los
  criterios, no la opinión previa del coordinador.
- Veredicto de cada requisito: PASA / FALLA / NO EVALUADO / INCONCLUSO, con evidencia.
  Cada fallo corregido deja una regresión. Un negativo que falla por sintaxis accidental
  no cuenta.
- Ni demos ni conteo de tests aprueban una fase. Un PASS de Sonnet vale para el corte,
  no para la fase; las fases las aprueba el propietario. Los requisitos de F0 "dos
  revisores sin discrepancias" y otras puertas del roadmap siguen pendientes y visibles.
- Cada paso terminado: commit descriptivo, push sin force, verificar HEAD remoto.
  Antes de empezar cualquier sesión: `git fetch` y comprobar que el local no está
  detrás del remoto (otro agente puede haber publicado).
- Sin secretos, sin reescribir historial, sin ocultar defectos encontrados.
- Estado y pendientes: `Pending.md`, `HANDOFF.md`, `RECOVERY.md`. Actualizarlos al cerrar
  cada corte.
- Verificación de hashes de la base: contra el blob de git
  (`git show HEAD:<ruta> | sha256sum`); en Windows `core.autocrlf` altera el fichero de
  trabajo.
