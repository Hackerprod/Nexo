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

## 9. Directivas del propietario posteriores a la base (2026-10-20)

Estado de este apartado: **directivas del propietario, sin ADR ni validación**. Registran la
intención de largo plazo del proyecto. No modifican `docs/baseline/` ni crean una revisión 0.3;
cada una abre una decisión que debe tomarse con un ADR (Flujo A) antes de implementarse. Donde una
directiva amplía o contradice el roadmap 0.2 se indica expresamente. Ninguna se ha medido ni
validado.

### 9.1. Independencia total y self-hosting

**Directiva.** Nexo debe llegar a ser un lenguaje completamente independiente: su compilador debe
poder escribirse en Nexo (self-hosting) y poder construirse a partir de una versión anterior
(bootstrapping), sin depender de Rust en la cadena final.

**Relación con el roadmap 0.2.** El roadmap no contiene este objetivo (ni como entregable ni como
fase). D04 fija Rust y una IR propia como implementación inicial, con LLVM como backend nativo
principal. Esta directiva es una ampliación de alcance.

**Decisiones que abre.** (a) Cadena de arranque: qué versión del compilador en Rust se conserva como
etapa 0 y cuándo se retira. (b) Qué capacidades del lenguaje son prerrequisito para escribir un
compilador en Nexo (genéricos, traits, closures, colecciones, memoria y E/S estables). (c) La
frontera con las dependencias que no son Nexo (ver 9.2 y 9.3). Debe decidirse **antes de F2**, porque
condiciona el diseño del backend, de la ABI interna y de la biblioteca estándar.

### 9.2. Backend propio en lugar de LLVM

**Directiva.** Tras validar Nexo de extremo a extremo, la siguiente mejora es sustituir LLVM por un
backend propio.

**Relación con el roadmap 0.2.** D04 fija LLVM como backend de producción, con un adaptador
experimental a Cranelift como contingencia de tiempos de compilación. Sustituir LLVM contradice
D04 en el plano de producción y exige una decisión nueva.

**Relación con 9.1.** Son objetivos distintos pero convergentes. El self-hosting no obliga a tener un
backend propio, pero un lenguaje «completamente independiente» sí exige no depender de LLVM
(escrito en C++). La IR propia de D04, que conserva procedencia de préstamos, efectos y limpieza, es la
base natural de ese backend.

**Decisiones que abre.** (a) Criterio exacto de «validado de extremo a extremo» que habilita el
cambio (puertas de F2/F6 cumplidas). (b) Alcance por etapas: un backend propio sin optimización
agresiva para arrancar (x86-64 y arm64, las plataformas de §4.2 del roadmap), con LLVM conservado
como backend de optimización opcional mientras no se igualen las puertas de rendimiento (H-PY,
H-RS, H-CS). (c) Riesgo declarado: un backend optimizador competitivo es un esfuerzo mayor que el
resto del compilador, y las puertas de rendimiento del roadmap se formularon suponiendo la calidad
de optimización de LLVM.

### 9.3. Bibliotecas 100 % escritas en Nexo

**Directiva.** Las bibliotecas deben estar escritas íntegramente en Nexo. Se tomarán como
referencia las bibliotecas de código abierto de otros lenguajes (por ejemplo, red, archivos y demás
de C#/.NET).

**Relación con el roadmap 0.2.** Compatible con §6.7 (baterías esenciales: HTTP, JSON, CLI, logging,
tests) y con P07 (las operaciones no verificables se confinan a módulos `unsafe` pequeños y
auditados). Amplía el roadmap en que excluye depender de bibliotecas escritas en otros lenguajes y
en que toma la biblioteca de otros ecosistemas como referencia de superficie.

**Decisiones que abre.** (a) La capa irreductible: qué parte no puede escribirse en Nexo (las
intrínsecas que genera el backend y las llamadas al sistema operativo). La vía coherente con la
independencia es que Nexo declare y llame directamente a las interfaces del sistema (llamadas al
sistema o API nativas de cada plataforma) en módulos `unsafe` mínimos, sin pasar por libc ni por
bibliotecas en C. (b) Qué sucede con bibliotecas de terceros en C: los perfiles C de F4 siguen
siendo la vía de interoperabilidad, pero no forman parte de la biblioteca estándar. (c) Licencias
y procedencia al tomar referencias de código abierto: la referencia es de diseño y de API, no una
copia de código sin respetar su licencia. (d) Criptografía y TLS, que el roadmap deja fuera de EX02.

### 9.4. Dos capas: bajo nivel y alto nivel

**Directiva.** Nexo tendrá dos capas, una de bajo nivel y una de alto nivel. La de alto nivel debe
dar al programador lo más fácil posible. Analogía del propietario: un `array[]` frente a `List` en
C#, donde `List` incorpora muchas funcionalidades implícitas que aceleran el desarrollo.

**Relación con el roadmap 0.2.** Coincide con la propuesta de producto de §0.1 («empezar con datos
sencillos y conservar el algoritmo cuando haga falta controlar memoria, representación y
concurrencia») y con P01 (un solo lenguaje, una sola semántica). Se interpreta así: **no son dos
lenguajes ni dos dialectos**, sino dos niveles de biblioteca y de contrato sobre la misma semántica.
- Nivel bajo: arreglos y vistas, regiones, layouts (AoS/SoA), contratos `no_alloc`/`no_block`,
  control de representación y de memoria.
- Nivel alto: colecciones crecientes (la `List`), mapas, cadenas y flujos con operaciones ya
  hechas, iteradores y consultas, que pueden reservar memoria y ocultar detalles.
Las construcciones de alto nivel se escriben **con** las de bajo nivel (la `List` se construye sobre un
arreglo y un asignador), y un contrato de bajo nivel puede rechazar una función que las use. Así el
programador desciende de capa sin reescribir su lógica.

**Decisiones que abre.** Frontera y nombres de las capas. Qué colecciones y operaciones entran en
la capa alta. Cómo se declara en el código qué capa se está usando. Cómo interactúa con los contratos
de costes (P06) para que la comodidad del alto nivel no oculte reservas.

### 9.5. Biblioteca de interfaz de usuario basada en HTML/CSS

**Directiva.** Existirá una biblioteca de UI basada en HTML/CSS, similar al proyecto del propietario
AetherUI (https://github.com/Hackerprod/AetherUI) pero más optimizada, escrita en Nexo, con salida
multiplataforma (Windows, Android, Linux, etc.).

**Relación con el roadmap 0.2.** Fuera del alcance de 1.0: no figura entre los compromisos de §11.1 y
requiere capacidades que el roadmap sitúa más tarde (tareas y concurrencia en F5, Wasm y puentes en
F4, plataformas móviles no incluidas en las cuatro de §4.2). Se registra como objetivo de ecosistema
posterior al núcleo.

**Decisiones que abre.** Subconjunto de HTML/CSS y modelo de composición. Motor de maquetación y
de dibujo. Accesibilidad. Destinos (escritorio nativo, Android, navegador vía Wasm). Dependencia de
9.3 (biblioteca íntegramente en Nexo) y de un backend con las plataformas objetivo. Antes de diseñar
conviene estudiar AetherUI y fijar qué se conserva y qué se rediseña.

### 9.6. Sintaxis no arraigada a ningún lenguaje

**Directiva.** La sintaxis no debe estar demasiado arraigada a ningún lenguaje existente, para evitar
malentendidos. Los ejemplos del roadmap se parecen a C++ y Rust. El propietario considera que, si se
construye bien, corregir o modificar la sintaxis más tarde no será un trabajo pesado; para validar,
la sintaxis concreta no importa.

**Relación con el roadmap 0.2.** D01 acepta llaves, `let`/`var`, `fn`, `struct`, patrones y
terminadores opcionales: una superficie de la familia C/Rust. El roadmap prevé congelar la sintaxis
en F0–F1. La decisión del propietario del 2026-10-22 fijó B como única superficie oficial;
véanse F0-D14 §5 y F0-D15. La decisión del propietario del 2026-10-24 fija `.n` como extensión oficial de los archivos fuente.

**Estado del código y coste real de cambiar.** La arquitectura separa la superficie de la
semántica: el lexer (`frontend/lex.rs`), el parser (`frontend/parse.rs`) y el impresor canónico
(`frontend/print.rs`) producen y consumen un AST normalizado, y la comprobación de nombres, tipos
y las etapas posteriores operan sobre el AST, no sobre el texto. Cambiar la sintaxis supone
reescribir lexer, parser e impresor y migrar las fuentes de prueba. **Lo que más pesa hoy son los
fixtures**: cientos de fuentes con spans, tokens y conjuntos esperados transcritos del ADR están
ligados a la sintaxis oficial B. La sustitución local migró las fixtures existentes y espera
revisión independiente; la deuda de oráculos textuales sigue creciendo con cada corte.

**Decisión de superficie y proceso.** La decisión del propietario del 2026-10-22 adopta B de
F0-D14 §5 como única sintaxis oficial, especificada normativamente en F0-D15. No se mantiene
compatibilidad ni un segundo frontend. Limitar fixtures dependientes del texto a las necesarias
por corte; los esperados publicados de tokens, spans y diagnósticos se transcriben de forma
independiente del impresor. La IR, los efectos y la propiedad conservan sus contratos semánticos.

**Aclaración del propietario (2026-10-20, posterior a la directiva).** El criterio no es evitar toda
influencia reconocible, sino que **ningún lenguaje domine** y que se tome **lo mejor de cada uno**,
incluidas las mayúsculas de C# (PascalCase), que aportan identidad visual y belleza al código. La
sintaxis no debe favorecer a quienes implementan en Rust o C++ ni incomodar al resto.

**Criterios de la decisión de superficie y de su medición posterior:**
1. **Mérito propio, no procedencia.** Cada construcción se justifica por legibilidad, regularidad de
   parseo, utilidad para agentes y coste visual, nunca por «viene de» un lenguaje concreto. Una
   influencia reconocible no es un defecto ni una virtud.
2. **Sin ventaja sistemática para una cohorte.** El roadmap ya define cohortes de Python, Rust y C#
   (§7). Cada construcción debe poder leerse y escribirse sin que una cohorte parta con ventaja
   clara; se comprueba con una prueba de comprensión cruzada, no por opinión.
3. **Coherencia del conjunto.** Tomar lo mejor de cada lenguaje exige reglas comunes: una sola forma
   por idea, símbolos con un único trabajo y convenciones uniformes. El riesgo del collage no es la
   variedad de orígenes sino las formas duplicadas y las reglas que se contradicen.
4. **Mayúsculas como convención con identidad visual.** El PascalCase para tipos, funciones y
   miembros públicos y el camelCase para variables se adoptan como convención de la superficie. No
   cambian la semántica (el compilador no decide que algo es un tipo por empezar en mayúscula) y las
   hace cumplir el formateador oficial.
5. **Medición.** B se evalúa después de implementarse con tareas de agente (generación y reparación
   de programas, con recuento de errores) y comprensión por cohortes según F0-D15 §9.3. La medición
   no reabre la elección de superficie ni sustituye las puertas independientes del roadmap.

**Antecedente histórico de la propuesta externa recibida** (documento de sintaxis de 22 secciones,
2026-10-20): se evaluó como candidata antes de la decisión del propietario. Ideas con mérito propio
identificadas entonces:
vocabulario de modos `view`/`edit`/`take`/`move`/`copy` con marca visible, `if c then a else b`
como expresión de un solo valor, operadores lógicos en palabras (`and`/`or`/`not`), rangos
`0..<10` y `1..=10`, y la tabla de traducción de superficie a núcleo. Puntos a resolver antes de
adoptarla: dos maneras de componer (método con punto y tubería `|>`) y la pérdida del modo en la
tubería; `fails` frente a `Result` explícito; `prop` y `defer` (características nuevas); comas
finales, rechazadas hoy por decisión del ADR; y choques con D01, D11 y D13 (`=> expr` sin `return`,
`capture [...]`, `value:`). F0-D14 §5 resolvió esta comparación y F0-D15 fija la superficie elegida.

### 9.7. Orden propuesto de las decisiones

Orden sugerido para abrir ADR, sin implicar aprobación: 9.6 (sintaxis, punto de decisión) y 9.4
(capas) antes de congelar F1; 9.1 y 9.3 (self-hosting y bibliotecas en Nexo) antes de F2; 9.2
(backend propio) tras la validación de extremo a extremo; 9.5 (UI) después del núcleo y la
concurrencia. Los ganchos de diseño de 9.8 (identidades de función estables y hash por función en la IR)
se fijan al diseñar la IR, ya en I04; sus niveles H1 a H3 siguen a F2. Todo lo anterior queda registrado como intención del propietario hasta que exista
una decisión revisada y registrada.

### 9.8. Recarga en caliente (hot reload)

**Directiva.** Uno de los problemas de los desarrolladores es que cada cambio exige recompilar para
verlo; C# tiene recarga en caliente. El propietario pide que Nexo la tenga y pregunta si puede
hacerse de forma completamente nativa en Nexo.

**Relación con el roadmap 0.2.** El roadmap ya contiene los cimientos: el REPL con declaraciones
versionadas (D10: «no se modifica el layout de objetos vivos al redefinir un tipo»), la misma
semántica en intérprete y en código nativo (P01, D04) y el servidor de compilación incremental
compartido con el LSP (F2), con puertas de ciclo de edición (§4.3). También lo excluye expresamente:
F2 declara fuera de alcance el «hot reload avanzado». Esta directiva amplía el alcance y fija la
intención; no hay ADR ni medición.

**Niveles propuestos** (la recarga en caliente no es una sola capacidad):
- H0. Reconstrucción incremental rápida y reinicio. Ya cubierta por las puertas de F2.
- H1. Sustitución del cuerpo de una función en un proceso en ejecución, con firma y efectos
  compatibles. En el intérprete se reduce a cambiar una entrada de la tabla de funciones.
- H2. Lo mismo en código nativo de desarrollo, mediante llamadas indirectas (con su coste, solo en
  modo desarrollo) o carga de código nuevo en memoria ejecutable.
- H3. Cambio de la forma de los datos (campos, layout) con objetos vivos. Es la parte difícil; requiere
  una función de migración de estado o reiniciar. C# tampoco la admite (ediciones que fuerzan reinicio).

**Reglas que se derivan de la semántica de Nexo (a decidir en un ADR).**
1. Una edición se clasifica como «aplicable en caliente» o «requiere reiniciar», con un diagnóstico
   estructurado de código estable que explica la causa. No hay modo silencioso.
2. Una edición que cambie firma, efectos o contratos (`no_alloc`, `no_block`, `no_suspend`, `no_io`) de
   una función revalida a todos sus llamadores con el compilador incremental; si alguno deja de cumplir
   su contrato, la edición se rechaza como «requiere reiniciar».
3. La sustitución se aplica solo en puntos seguros (por ejemplo, entre iteraciones del bucle de
   eventos). Los marcos de llamada ya activos terminan con el código anterior.
4. Los generadores y las tareas suspendidas conservan los locales del cuerpo que los creó: modificar ese
   cuerpo es un cambio de layout, se rechaza o se reinicia esa tarea.
5. Propiedad: no se sustituye código mientras existan préstamos o regiones activas que dependan de la
   representación anterior; los recursos conservan su contrato de limpieza (P02).
6. Separación entre estado y código: la recarga de código no modifica el estado; el estado solo cambia
   con una migración explícita.
7. Equivalencia (P01): la ejecución tras una recarga aplicable no puede observar nada que una ejecución
   limpia con el mismo estado y el código nuevo no pudiera observar. Hace falta un arnés
   diferencial para comprobarlo, en la línea de T23.

**¿Es posible de forma completamente nativa?** Sí, con matices. La maquinaria vive en el runtime y
necesita interfaces del sistema operativo (memoria con permiso de ejecución, carga de bibliotecas
dinámicas) en módulos `unsafe` mínimos. Hoy ese runtime está en Rust. Podrá escribirse en Nexo
cuando se cumplan 9.1 (self-hosting) y, para el código nativo propio, 9.2 (backend propio).

**Ganchos de diseño que conviene fijar ahora** (baratos ahora, caros después): identidades de función
estables y un hash por función en la IR (I04), de modo que la IR sea la unidad de recompilación
incremental; llamadas del intérprete a través de una tabla de funciones sustituible en modo desarrollo;
y clasificación de ediciones integrada en el servidor incremental. Con esos ganchos, H1 queda casi
gratis en el intérprete y H2 queda abierto.

**Relación con 9.5 (UI).** Es la función de mayor valor para una biblioteca de interfaz: si los
componentes son funciones del estado, se recargan sin perder el estado.

**Fases propuestas** (sin aprobar): H0 en F2 (ya planificado); H1 en el intérprete de desarrollo
poco después de F2; H2 al cerrar el backend nativo de desarrollo; H3 solo con ADR propio.

**Decisión del propietario (2026-10-25).** Nexo debe tener recarga en caliente NATIVA de primera
clase, no solo en el intérprete. Consecuencias registradas:
- La regla 3 se enmienda: los marcos de llamada ya activos terminan con el código anterior, SALVO los
  bucles permanentes (por ejemplo, el bucle de eventos o el cuerpo de una aplicación interactiva) que el
  compilador pueda transformar de forma comprobable para trasladar su ejecución al código nuevo (nivel
  H1b, posterior a H1). El contrato de elegibilidad, estado transportado, equivalencia y salidas se
  fija en un ADR propio; sin elegibilidad demostrada, la regla 3 sigue vigente.
- Las ideas de la arquitectura externa del compilador (`docs/proposals/compilador-maestro-recarga-r3-1.md`)
  se evalúan por su MÉRITO para Nexo. No se descartan por ser difíciles ni por contradecir una
  decisión previa: si benefician al lenguaje, se portan mediante una enmienda explícita y razonada a
  la regla o al ADR que contradicen; el registro de qué se adopta, adapta o difiere está en
  `docs/proposals/compilador-maestro-recarga-evaluacion.md`.
- Las reglas 4 (generadores y tareas suspendidas) y 7 (equivalencia con ejecución limpia) se revisan
  en el mismo ADR con ese criterio, con el arnés diferencial como mínimo obligatorio.
