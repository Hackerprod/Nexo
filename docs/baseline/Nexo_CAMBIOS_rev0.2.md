# Nexo — Registro de cambios de la revisión 0.2

**Documento principal:** `Nexo_ROADMAP_rev0.2.md`
**Base:** revisión 0.1, conservada sin sobrescribir.
**Fecha:** 19 de octubre de 2026.

## Registro de revisión: 0.1 → 0.2

**Resultado de esta revisión:** se mantiene el núcleo de 0.1 y se transforma la adopción en una condición de aprobación, además de ampliar las decisiones semánticas y el alcance práctico. «Incorporado» significa incorporado al plan; no implementado ni medido.

| Solicitud de la validación | Resolución 0.2 | Integración principal |
|---|---|---|
| A01. macOS arm64 y Linux arm64 | Aceptada. Cuatro plataformas nativas obligatorias, incluida Windows x86-64; pruebas en hardware real. | Matriz 4.2; F2–F6; §11. |
| A02. Interfaces y genéricos | D08: traits nominales, coherencia por paquete, extensiones explícitas, operadores mediante traits y despacho estático/dinámico diferenciado. | F0–F3; T17. |
| A03. Python temprano | Exportación mínima a CPython en F2: escalares, texto y lectura de búferes. El acceso sin copia general sigue en F4. | F2; §6.6; T19. |
| A04. Ventaja por cohorte | H-PY, H-RS y H-CS con magnitudes útiles y reglas de análisis previas. La mera no inferioridad ya no permite aprobar F6. | §7; F0, F6; §8. |
| A05. Conteo de referencias | D09 acepta local no atómico y compartido atómico, con protocolo de confinamiento, transferencia y publicación. No se cambia un contador bajo alias activos. | F0, F2, F3, F5; T18. |
| A06. Sustituto de reflexión | Derivación tipada en `comptime`, inspeccionable y sometida a las mismas comprobaciones que el código escrito a mano. | D03; F1–F3; T20. |
| B07. Nexo en 10 minutos | Cuatro programas completos de referencia de diseño: CLI, HTTP/JSON, simulación y generador. | §0; corpus EX01–EX04. |
| B08. Ergonomía cotidiana | D10 fija texto, argumentos, iteradores, generadores, REPL y doctests; se explicitan sus interacciones con recursos. | F1–F3; T21–T23. |
| B09. Baterías esenciales | HTTP, JSON, CLI, logging y tests son compromisos 1.0, con APIs acotadas y mantenimiento oficial. | F1–F5; §6.7; T24. |
| B10. Adopción | Playground Wasm del núcleo y guías para Python, Rust y C#, con pruebas de inicio sin asistencia. | F4, F6; §6.8; T27. |
| B11. Diagnósticos | Nuevo P09, catálogo estable y pruebas de causa, ubicación y reparación. | F1–F3, F6; T25. |
| B12. Iteración en F0 | Prototipo vertical desechable permitido desde el principio. Las revisiones son puertas de promoción, no permisos para explorar. | F0. |
| B13. Compilación a escala | Corpus de 10.000 y 100.000 líneas; objetivos y puertas separados; contingencia de backend preparada desde F0–F2. | D04; §4.3; T26. |
| C14. `panic` por tarea | Investigación optativa en F5; no cambia el aborto de proceso de 1.0. | D02; F5; §10. |
| C15. Enteros | Medición del coste y operaciones recuperables, modulares, saturadas y con precondición comprobada diferenciadas. | D07; F2–F3; T07. |
| C16. Asistentes en el estudio | Grupo secundario independiente; no forma parte del lenguaje ni sustituye el estudio principal. | §7.7; F6. |
| C17. Nombre | Revisión de conflictos y decisión de nombre antes de entrar en F6. | D06; F4–F5; §11. |

**Precisiones de integración.** Las referencias compartidas físicamente entre tareas necesitan un protocolo real, no una etiqueta; un búfer «de solo lectura» no acredita ausencia de escritores externos; carecer de un recolector global no demuestra mejor latencia que C#; compilar un programa a Wasm no significa haber compilado también el compilador al navegador. Estas distinciones están resueltas en sus decisiones y pruebas, no relegadas a notas finales.

**Elementos preservados:** P01–P08; las propiedades separadas de copia, independencia y transferencia; regiones, arenas e instantáneas; los contratos de recursos; T01–T16; la comparación con antecedentes; y la independencia de `@variant` y de cualquier uso de IA.


## Alcance del fichero entregado

La revisión desarrolla P01–P09, D01–D10, F0–F7, T01–T28 y cuatro programas completos de aceptación. Conserva la sección de influencias por lenguaje y añade 16 referencias a las 42 de la base.

Las comprobaciones de edición verificaron la conservación de P01–P08 y de las filas T01–T16, la presencia de los requisitos por fase, las referencias Markdown y la estructura de tablas. No se ha compilado Nexo ni ejecutado los programas de ejemplo, benchmarks o estudios de personas: son entregables y criterios del roadmap.
