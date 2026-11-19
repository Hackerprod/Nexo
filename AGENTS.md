# Nexo — Flujo de trabajo de agentes

Objetivo permanente: un lenguaje de propósito general que tome lo mejor de los demás (ver roadmap §2),
facilite el desarrollo y permita control progresivo de la máquina, con herramientas útiles para
personas y agentes (**agent friendly**, sin que ello sea el único criterio). Visión y principios en
`docs/VISION.md`. Autoridad documental: `docs/baseline/` (rev 0.2, intacta) y `docs/F0_ACUERDOS.md`.
No se crea una rev 0.3.

## Roles

- **Coordinador (Claude, juez):** no implementa. Lanza a los demás, relaciona veredictos,
  decide la siguiente fase, verifica commits y estado remoto. Su rol es que todo fluya.
- **Codex (modelo Astra):** diseña arquitectura/ADR cuando algo no está en el roadmap.
- **Codex (modelo GPT 6 Sol; Luna como opción barata):** implementa.
- **Subagente Sonnet 5.5 (esfuerzo medium):** valida, vía `.claude/agents/nexo-validator.md`. Nunca implementa ni rebaja requisitos para aprobar.

## Flujo A — Fases o mecánicas no planteadas en el roadmap

1. Codex (Astra) propone arquitectura (ADR en `docs/adr/`).
2. Subagente Sonnet 5.5 valida la propuesta contra el objetivo: el roadmap y la idea central
   (lenguaje que toma lo mejor de los demás y es agent friendly).
3. Si Sonnet no da **PASS**, se especifica por qué y se itera de nuevo con Astra.
   Se repite hasta PASS.

## Flujo B — Cuando ya no hay nada por diseñar, solo construir (decisión del propietario, 2026-11-14)

Cada corte y cada corrección de bloqueantes siguen el mismo flujo, con cuatro roles de subagente (A, B, C,
D), distintos entre sí. **CL = contexto limpio**: el subagente arranca sin memoria previa y solo recibe el
documento `.md` y los criterios. Un paso sin las siglas CL **conserva el contexto** del subagente que ya
trabajó en él.

1. **Subagente A (CL)** crea el plan en un `.md` (`docs/correcciones/<corte>-plan.md` o, para un corte
   nuevo, el plan del corte): alcance, requisitos del ADR, causa raíz y familia cuando es corrección,
   mecanismo único, efectos laterales, alternativas, pruebas, orden y dependencias.
2. **Subagente B (CL)** valida el plan contra el ADR y el código y da **PASS o FAIL con motivos**
   (lista numerada, accionable).
   - 2.1. Si FAIL, **A** (conserva contexto) propone la corrección del plan en el mismo `.md`.
   - 2.2. **B** (conserva contexto) valida el plan corregido. Se repite 2.1–2.2 hasta que B dé PASS.
3. **Subagente C (CL)** implementa el plan **desde el `.md`** validado, sin apartarse de él; si descubre
   que no basta, no improvisa: lo registra y vuelve al paso 1 (el plan se revisa por A y B).
   - Un **commit por solución del plan** en la rama remota (`i07c-robustez`, nunca `main`), con el
     identificador de la solución en el mensaje, para poder revertir solo la solución que falle.
   - **Puerta mecánica antes de entregar a D:** `sh scripts/validate.sh` en clon limpio (fmt, clippy
     `-D warnings`, tests debug y release, RD), el diferencial contra la versión anterior sobre los
     programas que el plan declara sin cambio y el medidor de objetivos. Si la puerta falla, C corrige
     (conservando el plan) antes de que D intervenga; D no valida un árbol que no pasa la puerta.
4. **Subagente D (CL)** valida la implementación contra el plan y el ADR (código, diferencial, fuzz,
   oráculos del corte) y da **PASS o FAIL con motivos**. D comprueba además la **fidelidad al plan**
   (revisión de diff): que los archivos cambiados están en el plan, que no hay cambios fuera de él, y que
   no hay parches por patrón de fuente, casos especiales, constantes hechas para pasar una prueba, rutas
   duplicadas ni código muerto; cada hallazgo es FAIL con motivo.
   - 4.1. Si FAIL, **C** (conserva contexto) escribe las correcciones **en otro `.md`**
     (`docs/correcciones/<corte>-rN.md`): causa raíz comprobada, familia, mecanismo único, efectos
     laterales y pruebas, sin tocar código.
   - 4.2. **D** (conserva contexto) valida esa propuesta de corrección hasta que dé PASS.
   - 4.3. **C** (conserva contexto) corrige el código según la propuesta validada y D repite la
     validación (paso 4) hasta que dé PASS.
5. **Confirmación final (D' CL).** Con PASS de D, una pasada corta de un subagente D' con contexto
   limpio (distinto de D) revisa el diff final y reproduce los repros del plan, para evitar el sesgo de
   confirmación de quien ya conoce las rondas anteriores. Solo con PASS de D' integra el coordinador
   (commit y push a `main`). Ningún subagente modifica código antes de que el plan o la propuesta
   correspondiente tenga PASS.
6. **Cortes grandes.** Un plan con muchas soluciones se divide en bloques coherentes (por dependencias
   y por familia); cada bloque recorre el flujo completo, para que B y D validen por partes.

### Vía rápida para correcciones simples (decisión del propietario, 2026-11-14)

El coordinador decide, y registra el motivo en el informe de cierre, cuándo una corrección es **simple y
obvia** y no necesita el ciclo completo de plan y propuesta. Criterios (todos): causa raíz de una sola línea
o función, ya comprobada leyendo el código; un único archivo o rutina; sin interacción con otros
mecanismos (propiedad, temporales, cobertura, diagnósticos, cuotas); sin cambio de comportamiento de
programas ya aceptados; sin decisión de laguna del ADR. En la vía rápida: C implementa con regresión
(que falla sin la corrección y pasa con ella) y pasa la puerta mecánica; D (CL) hace UNA verificación
dirigida con fidelidad al código y diferencial; sin propuesta ni plan previos. Ante la duda, o si aparece
una interacción, se vuelve al ciclo completo. Los mecanismos con interacciones (p. ej. estado de
temporales por arista, cobertura de `match`, orden de cierre) nunca van por la vía rápida.

Qué cuenta como FAIL lo fija «Rigor proporcional y presupuesto de validación» (solo defectos
reproducibles; núcleo vs andamiaje). Los ciclos 2, 4.1–4.3 se repiten hasta PASS; si no convergen, el
coordinador lo registra y reduce el alcance o reabre el diseño (Flujo A) en vez de iterar sin fin.
Implementador (C) y validadores (B, D) son siempre distintos; A y C también, para que C parta de un
contexto limpio.

## Reglas comunes

- Implementador y validador siempre distintos. Al validador se le da el código y los
  criterios, no la opinión previa del coordinador.
- Las observaciones de una revisión se clasifican según el contrato del corte, no según el número de
  rondas: un incumplimiento obligatorio bloquea siempre, por pequeño que sea (una cifra errónea, una
  cita inexacta, una ambigüedad que deja una decisión abierta o una contradicción entre documentos);
  los bloqueantes menores acumulados degradan el lenguaje y son difíciles de corregir después. Solo se
  registra sin bloquear lo realmente leve: una mejora de redacción o una nota que no cambia ninguna regla,
  cifra ni cita. En el Flujo B, qué cuenta como incumplimiento obligatorio lo fija «Rigor proporcional y
  presupuesto de validación».
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

- El brief del implementador incluye desde el principio el estándar que el validador exigirá para los
  requisitos de núcleo (ver «Rigor proporcional y presupuesto de validación»): negativo aislado por
  regla (rompe un solo atributo y comprueba código, subcausa, span, token y campos exactos), mutación inversa verificada por regla con restauración por SHA-256, resultados
  esperados transcritos del ADR y no generados por el código bajo prueba, casos válidos contra
  falsos positivos, fuzz sin panics, y nombres de test sin recuentos de pasadas. Esto redujo un
  corte de siete rondas de validación a tres.
- Nunca terminar con el árbol roto: si falta presupuesto, revertir a un estado verde y listar lo
  pendiente nominalmente.
- Los documentos versionables (HANDOFF.md, Pending.md, RECOVERY.md, notas) se ACTUALIZAN, no se
  reescriben: comparar con `git diff HEAD` para no perder contenido útil. No citan archivos temporales, el
  brief ni etiquetas de proceso, y no afirman PASS ni commit antes de tiempo.
- El validador compara siempre `check`/`run` contra HEAD con su propia batería (`git archive HEAD`),
  no solo con la del implementador. Las mutaciones y su efecto sobre el veredicto se rigen por
  «Rigor proporcional y presupuesto de validación».
- Las lagunas reales del ADR se resuelven como indica «Rigor proporcional y presupuesto de
  validación»; los tests no inventan contenido. Las obligaciones de emisión de pasos posteriores se
  registran como límite.
- Entorno Windows: el sandbox de Codex falla (`setup refresh had errors`); el implementador se
  lanza con `codex exec --dangerously-bypass-approvals-and-sandbox` limitado al repo por el brief
  y con git de solo lectura. Autorizado por el propietario solo para este repositorio.

## Rigor proporcional y presupuesto de validación (decisión del propietario, 2026-11-12)

Motivo y registro en `docs/F0_ACUERDOS.md` §10. Rige desde el cierre de I11a y prevalece sobre las
lecciones operativas anteriores cuando discrepen. La independencia implementador/validador y la
exigencia de evidencia no cambian; cambia dónde se gasta el rigor y cuándo termina una validación.

- **Dos clases de requisito.** Antes de lanzar un corte, el coordinador clasifica en el brief cada
  requisito como *núcleo* o *andamiaje* y entrega la misma clasificación al implementador y al validador.
  - *Núcleo*: lo que decide qué significa un programa Nexo o qué diagnóstico recibe quien lo escribe:
    léxico y sintaxis oficial, nombres, tipos e inferencia, propiedad y préstamos, efectos, enteros
    comprobados, `Result` y `?`; el resultado, el error y el orden de eventos de recursos de la
    ejecución; y el código, la subcausa y el span primario de los diagnósticos de esas reglas.
  - *Andamiaje*: lo que existe para ejercitar el prototipo, que el roadmap declara desechable (F0,
    «Prototipo permitido desde el inicio»): entorno simulado y su JSON, transporte y mensajes de la CLI,
    cifras de cuotas y de trabajo, recuentos de bytes, formato de salida, scripts y arneses.
  - En caso de duda: ¿cambiaría el significado de un programa o el diagnóstico de una regla del
    lenguaje? Si no, es andamiaje.
- **Evidencia exigida.** Núcleo: negativo aislado por regla con su positivo, esperados transcritos del
  ADR, fuzz sin pánicos y diferencial debug/release y `check`/`run`. Andamiaje: pruebas de ejemplo de
  los flujos normativos del corte y fuzz sin pánicos; sin campañas de mutación, sin oráculos
  independientes y sin fijar cifras que el ADR no enuncie.
- **Qué bloquea.** Solo un defecto reproducible con un programa fuente o una invocación de la CLI, con
  la salida esperada y la obtenida. En núcleo: aceptar lo que una regla rechaza o rechazar lo que
  admite; resultado, error u orden de eventos erróneo; código, subcausa o span primario erróneo;
  divergencia debug/release o `check`/`run`; pánico, `internal_error`, `InvalidIr` o cuelgue. En
  andamiaje: pánico, `internal_error`, cuelgue o resultado erróneo en un flujo normativo del corte. Un
  defecto de núcleo bloquea siempre, por pequeño que sea, y deja regresión.
- **Qué no bloquea** (se registra en `Pending.md`): un mutante superviviente sin programa que exhiba
  comportamiento erróneo; un hueco de cobertura; una cifra, texto o precedencia de andamiaje que el ADR
  no enuncia; una mejora de redacción.
- **Mutaciones.** Una sola pasada por corte, al final, ejecutada por el implementador y solo sobre
  archivos de núcleo. El validador no genera poblaciones nuevas en cada ronda; convierte un
  superviviente en defecto bloqueante únicamente aportando el programa que lo exhibe. La mutación
  inversa por regla que acompaña a cada negativo de núcleo no es una campaña y se mantiene.
- **Presupuesto.** Como máximo dos rondas completas de validación por corte. Después solo hay
  verificaciones dirigidas: el validador comprueba que cada defecto bloqueante listado está corregido y
  tiene regresión, sin exploración nueva. Si la segunda ronda deja defectos de núcleo con una causa
  estructural común, el coordinador no lanza una tercera: reduce el alcance del corte o reabre el
  diseño por el Flujo A, y lo registra.
- **Lagunas del ADR.** Se deciden en el mismo corte y se escriben en el ADR con ejemplo válido e
  inválido: las de andamiaje las decide el coordinador; las de núcleo con más de una respuesta
  razonable van por el Flujo A. No se añaden tests `adr_gap_*` que fijen el comportamiento actual; los
  existentes se resuelven cuando se toque su corte.
- **Progreso por programas.** `corpus/objetivo/` contiene los programas que Nexo debe llegar a
  ejecutar, escritos en la sintaxis oficial a partir de EX01–EX04 y de los ejemplos de los ADR. El
  avance se informa en `HANDOFF.md` como programas objetivo que ejecutan sobre el total. Un corte que
  no hace ejecutar ninguno nuevo lo justifica en su brief.
- **ADR nuevos.** Enuncian reglas con ejemplo válido, inválido y diagnóstico esperado; no fijan cifras
  de andamiaje y citan por sección, no por número de línea. Los ADR ya revisados no se reescriben.
- **Respaldo y nombres.** El trabajo en curso se confirma cada día en su rama como «en validación»,
  aunque el corte no tenga PASS; el mensaje no afirma PASS. Los archivos y nombres de test describen
  lo que prueban, no la ronda de validación que los originó.

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
  exige tocar la causa de raíz (representación, modelo de datos, diseño), se hace en la raíz. Un defecto
  local se corrige localmente; solo varios fallos con una causa estructural común justifican rediseñar.
- Una migración se hace en un solo corte: el código y las fixtures nuevas sustituyen a las antiguas, sin
  dejar rastro de la versión anterior salvo en documentos históricos.


## Aceleración de puertas y máquina compartida (decisión del propietario, 2026-11-19)

Origen: medición propia del atasco (puerta completa de ~27–33 min, repetida ~25 veces por bloque, 71 ejecutables de test, disco `D:` lento, dos carriles sobre la misma máquina) y un análisis independiente en `.scratch/analisis-fable.md` (informe de un agente sin acceso a nuestras propuestas). Estas reglas se aplican a TODO agente y brief; no se pierden por compactación de contexto. Principio rector: **un fallo es evidencia suficiente de fallo; solo la ejecución completa es evidencia de éxito.** Ninguna acelera ni muestrea una comprobación que hoy fundamente un PASS, un FAIL, la muerte o la supervivencia de un mutante.

### Reglas de máquina (obligatorias, en todos los briefs)
- Prohibido `taskkill /IM` y cualquier parada de procesos por nombre de imagen (incidente 2026-11-19: un D' mató `cargo.exe` y `python.exe` y con ello los 6 procesos de la campaña de mutación de la otra sesión). Cada agente mata solo los PID que lanzó y no deja procesos al terminar.
- Todo el trabajo va en `C:` (NVMe): `CARGO_TARGET_DIR=C:/nexo-work/target`, clones y copias de validadores en `C:\nexo-work\val\<nombre>` o `C:\nexo-work\tmp` (borrar clon y `target` al terminar; ~25 GB libres compartidos: ≤ 8–10 GB por agente). `D:` (HDD USB) solo para archivo o resultados ya reconstruibles; nunca para compilar. Hasta 6 workers de cargo por agente (`CARGO_BUILD_JOBS=6`); `cargo-nextest` está instalado.
- PATH de las tareas: toolchain `C:\Users\hackerprod\.rustup\toolchains\1.99.1-x86_64-pc-windows-msvc\bin` y `C:\Users\hackerprod\.cargo\bin`.

### Puerta por niveles (nextest; plan en `docs/correcciones/NX-build-r1.md`)
- `scripts/validate.sh` es la puerta COMPLETA (fmt, clippy `-D warnings`, todos los tests ejecutables en debug y release, con `--profile default`). Es la única que fundamenta PASS: la usan D, D', la integración a main, el cierre de bloque y de corte, y todo diferencial o medición de mutante «muere/sobrevive».
- Nivel rápido (`scripts/validate-fast.sh`, perfil `fast`): solo para el bucle de edición de C; excluye por nombre una lista versionada de tests pesados y NUNCA vale como evidencia de PASS ni de supervivencia (imprime el aviso). Un chequeo mecánico versionado comprueba que rápida ∪ pesados = completo y que cada nombre existe.
- C trabaja con un `target` persistente por sesión (compilación incremental) y deja el clon limpio con `validate.sh` solo para la puerta final del commit publicado o del bloque. Mismo árbol y mismo `rustc` con registro verde no se repite: se cita el hash del árbol.
- Veredicto de mutante: parada en el primer fallo (`cargo nextest run`) basta para declararlo MUERTO (se anota el test que lo mata); declarar SUPERVIVIENTE exige la suite completa en debug y release. Un timeout de test es «detectado por no terminación» solo si la suite correcta termina y el mutante no, y se repite con menos carga antes de contarlo.

### Cola de mejoras acordadas (ordenadas; cada una por Flujo B con medición antes/después)
1. Un único binario de integración (`tests/suite.rs` con `#[path]`, `autotests = false`; el `#[global_allocator]` de `i07c_mutant_projection_scale` queda aparte): medido por el análisis independiente 269 → 73 s en frío debug, 4,3 → 1,2 GB, mismos 1.979/1.981 tests; cada corte nuevo añade una línea, no un binario.
2. Selección por etapa en el nivel rápido (tests de la etapa en curso + guardas globales baratas) y release solo al cierre de bloque.
3. `scripts/gate.sh <commit>` lanzado por el coordinador, con registro (hash del árbol, versión de `rustc`, salida, recuentos por perfil, SHA-256 del log) que D reutiliza; D' ejecuta su propia puerta en frío. Caché de `nexo.exe` de cada commit con registro para los diferenciales.
4. Reglas de turno para trabajos pesados entre sesiones (bloqueo de archivo o límite de procesos de la campaña de mutación mientras haya un bloque en C, D o D').
5. Solo con prueba de que no cambian los marcos de pila medidos en debug (puertas de 204/256 KiB): `debug = "line-tables-only"` y `rust-lld`; sin esa prueba no se adoptan.
