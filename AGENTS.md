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

## Lecciones operativas (aplicar a todo corte nuevo)

- El brief del implementador incluye desde el principio el estándar que el validador exigirá:
  negativo aislado por regla (rompe un solo atributo y comprueba código, subcausa, span, token y
  campos exactos), mutación inversa verificada por regla con restauración por SHA-256, resultados
  esperados transcritos del ADR y no generados por el código bajo prueba, casos válidos contra
  falsos positivos, fuzz sin panics, y nombres de test sin recuentos de pasadas. Esto redujo un
  corte de siete rondas de validación a tres.
- Nunca terminar con el árbol roto: si falta presupuesto, revertir a un estado verde y listar lo
  pendiente nominalmente.
- Los documentos versionables (HANDOFF.md, Pending.md, RECOVERY.md, notas) se ACTUALIZAN, no se
  reescriben: comparar con `git diff HEAD` para no perder contenido útil. No citan .scratch, el
  brief ni etiquetas de proceso, y no afirman PASS ni commit antes de tiempo.
- El validador compara siempre `check`/`run` contra HEAD con su propia batería (`git archive HEAD`),
  no solo con la del implementador. Criterio acotado de mutaciones: re-ejecutar las supervivientes
  de la ronda previa más un máximo de mutaciones nuevas, bloqueantes solo si caen en una regla que
  el ADR enuncia de forma explícita.
- Las lagunas reales del ADR se registran y se fijan con tests de comportamiento actual; no se
  inventa contenido. Las obligaciones de emisión de pasos posteriores se registran como límite.
- Entorno Windows: el sandbox de Codex falla (`setup refresh had errors`); el implementador se
  lanza con `codex exec --dangerously-bypass-approvals-and-sandbox` limitado al repo por el brief
  y con git de solo lectura. Autorizado por el propietario solo para este repositorio.

## Autoridad delegada (2026-10-20)

El propietario delegó en el coordinador la toma de decisiones del proyecto sin consulta previa.
El coordinador decide diseño, alcance, orden y proceso, y registra cada decisión por escrito (ADR o
`docs/F0_ACUERDOS.md`) para que el propietario pueda revisarla o revertirla. La independencia
implementador/validador y los requisitos de evidencia no se relajan.

Límites que NO se cruzan sin autorización expresa: hacer público el repositorio (es privado); force
push o reescritura del historial; secretos y credenciales; gasto de dinero o infraestructura de pago;
conceder a agentes más acceso del autorizado (Codex sin sandbox solo para este repositorio); modificar
`docs/baseline/`; actuar fuera del proyecto; y declarar aprobada una fase (la aprobación de F0..F7 es
del propietario). Las directivas de visión del propietario (`docs/F0_ACUERDOS.md` §9) son el marco de
las decisiones.

## Principios de ingeniería (decisión del propietario, 2026-10-22)
- Una sola superficie de sintaxis: la oficial es la candidata B de `docs/adr/F0-D14-sintaxis-superficie.md`,
  especificada de forma normativa en `docs/adr/F0-D15-sintaxis-oficial.md`. No hay competencia entre
  candidatas, ni modo de compatibilidad, ni dos sintaxis vivas en el repositorio.
- Sin adaptadores permanentes, sin parches por patrón de fuente y sin flujos duplicados. Si una corrección
  exige tocar la causa de raíz (representación, modelo de datos, diseño), se hace en la raíz.
- Una migración se hace en un solo corte: el código y las fixtures nuevas sustituyen a las antiguas, sin
  dejar rastro de la versión anterior salvo en documentos históricos.
