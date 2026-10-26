# Nexo — Ideas de benchmarks

Este documento reúne dónde y cómo poner a prueba el rendimiento de Nexo. No es normativo y no
contiene mediciones: hoy Nexo solo tiene un intérprete escalar, y `docs/VISION.md` fija que ninguna
afirmación de velocidad se presenta sin medida. Cada entrada indica si su descripción se pudo verificar
en la fuente; lo no verificado está marcado y debe comprobarse antes de adoptarlo.

## 1. Servidores web: HttpArena

Referencia del propietario: <https://http-arena.com/#scope=h2&mem=1&type=experimental&tuned=0>.
Repositorio: <https://github.com/MDA2AV/HttpArena> (MIT, escrito sobre todo en C#).

- **Qué mide.** Familias HTTP/1.1, HTTP/2, HTTP/3, pasarela, gRPC y WebSocket, con unas 27–30 pruebas
  (peticiones simples y encadenadas, JSON, compresión, TLS, subidas, estáticos, Postgres, plantillas y
  mixtas). Métricas: peticiones por segundo (la mejor de tres ejecuciones), latencia, CPU y memoria. La CPU
  se lee del cgroup del contenedor. La puntuación compuesta da 100 puntos al líder de cada prueba.
- **Hardware.** AMD Threadripper PRO 3995WX de 64 núcleos; cada implementación corre en un contenedor
  aislado con núcleos dedicados.
- **Filtros de la URL.** `scope=h2` es la familia HTTP/2. `type=experimental` es la categoría
  «Experimental», que puntúa solo contra otras experimentales (las categorías son Flagship, Emerging,
  Experimental, Engine e Infrastructure). `tuned=0` corresponde a configuraciones estándar sin afinar; que el
  valor 0 signifique exactamente eso es una inferencia. `mem=1`: significado no verificado.
- **Cómo se participa.** Un PR al repositorio con un directorio `frameworks/<nombre>/` con su Dockerfile y
  un `meta.json`, y las órdenes de validación y de benchmark de su documentación (no se pudo leer la página
  de alta). Las funciones no implementadas penalizan un 2,5 % cada una.
- **Qué exige a Nexo.** Un servidor HTTP/2 completo con TLS, concurrencia, E/S real y asignación eficiente.
  Es un objetivo tardío: requiere biblioteca estándar madura, E/S real y backend nativo. Mide E/S,
  concurrencia y memoria; no mide préstamos ni propiedad de forma directa.
- **Contexto.** TechEmpower FrameworkBenchmarks está archivado (24 de marzo de 2026, según su
  repositorio) y HttpArena se presenta como su continuación visible (esa afirmación procede de un resumen
  de búsqueda no verificado).
- **No verificado.** Herramienta de carga fuera de los perfiles de eficiencia (se cita `zrk`), reglas
  específicas para lenguajes nuevos y el documento de alta.

## 2. Benchmarks oficiales o de referencia

| Nombre | Qué mide | URL | ¿Admite lenguajes nuevos? | Utilidad para Nexo | Esfuerzo | Verificado |
|---|---|---|---|---|---|---|
| The Computer Language Benchmarks Game | 11 programas: fannkuch-redux, n-body, spectral-norm, mandelbrot, fasta, k-nucleotide, reverse-complement, binary-trees, pidigits, regex-redux | <https://benchmarksgame-team.pages.debian.net/benchmarksgame/> | No documentado | binary-trees ejercita la asignación sin recolector; el resto es cómputo puro | Bajo | Sí |
| Are We Fast Yet | 9 micro (Bounce, List, Mandelbrot, NBody, Permute, Queens, Sieve, Storage, Towers) y 5 macro (CD, DeltaBlue, Havlak, Json, Richards) | <https://github.com/smarr/are-we-fast-yet> | Sí (ports de 300 a 1.000 líneas con las mismas abstracciones) | Muy alta: mide el coste de las abstracciones y sirve también para intérpretes | Medio | Sí |
| HttpArena | Servidores HTTP (sección 1) | <https://http-arena.com/> | Sí, por PR | Concurrencia, E/S y memoria | Muy alto | Sí |
| TechEmpower FrameworkBenchmarks | Frameworks web | <https://github.com/TechEmpower/FrameworkBenchmarks> | Archivado | Solo histórico | n/a | Sí |
| simdjson (corpus `jsonexamples/`, p. ej. `twitter.json`) | Datos de prueba de JSON | <https://github.com/simdjson/simdjson> | Es solo datos | Un parser JSON propio con asignación intensiva | Medio | Sí |
| rebar | Motores de expresiones regulares | <https://github.com/BurntSushi/rebar> | Sí (se escribe un ejecutor) | Solo si Nexo escribe un motor de regex | Medio | Sí |
| LLVM test-suite | Compilación, tamaño de binario y ejecución (`lit`) | <https://github.com/llvm/llvm-test-suite> | Solo mediante LLVM | Útil si el backend emite LLVM IR | Alto | Sí |
| rustc-perf | Seguimiento por commit del rendimiento del compilador de Rust | <https://github.com/rust-lang/rustc-perf> | Pensado para rustc | Plantilla del seguimiento de compilación | Bajo (copiar la idea) | Parcial |
| Embench | 19 pruebas de sistemas embebidos sin SO | <https://github.com/embench/embench-iot> | Código en C | Solo si Nexo llega a embebidos | Medio | Parcial |
| SPEC CPU2017 | 43 pruebas de cómputo | <https://www.spec.org/cpu2017/> | No | Bajo | Inviable por licencia | Sí |
| pyperformance | Aplicaciones Python reales | <https://pyperformance.readthedocs.io/> | No (específico de Python) | Inspiración de diseño | n/a | Sí |
| JetStream 3.0 / Speedometer 3.1 | JavaScript, WebAssembly y aplicaciones web | <https://browserbench.org/JetStream/>, <https://browserbench.org/Speedometer3.1/> | No | Solo si Nexo apunta a WebAssembly o al navegador | n/a | Sí |
| CoreMark, SQLite speedtest1 | Núcleo de CPU; base de datos | — | — | Bajo; speedtest1 exige portar SQLite | — | No |

Ninguna de estas suites mide propiedad ni préstamos de forma directa. Para asignación sin recolector lo más
cercano son binary-trees, Storage, List y Json de Are We Fast Yet y un parser JSON propio. Para efectos y
recarga en caliente no hay ningún benchmark estándar.

## 3. Benchmarks comunitarios o no oficiales

| Nombre | Qué mide | URL | Verificado |
|---|---|---|---|
| plb2 (attractivechaos) | nqueen, matmul, sudoku y bedcov; unos 25 lenguajes; acepta PR | <https://github.com/attractivechaos/plb2> | Sí (fecha de la última actualización no verificada) |
| Programming Language Benchmarks (hanabi1224) | 18 problemas (binarytrees, fasta, json-serde, http-server, coro-prime-sieve, etc.), unos 28 lenguajes en una única ejecución de integración continua | <https://programming-language-benchmarks.vercel.app/>, <https://github.com/hanabi1224/Programming-Language-Benchmarks> | Sí |
| kostya/benchmarks | Brainfuck, Base64, JSON, Matmul y primos; más de 50 lenguajes; tiempo, memoria y energía | <https://github.com/kostya/benchmarks> | Sí |
| martinus/map_benchmark | Mapas hash de C++ | <https://github.com/martinus/map_benchmark> | Parcial |

No se localizaron benchmarks comunitarios de ordenación, de arranque en frío, de latencia de servidor de
lenguaje entre lenguajes ni de ciclo de edición con recarga en caliente; habrá que construirlos.

## 4. Compilador y herramientas

- **Compilación.** Tiempo limpio e incremental con entradas de 10.000 y 100.000 líneas (los proyectos T26 del
  roadmap), tamaño de binario y memoria máxima del compilador, con seguimiento por commit al estilo de
  rustc-perf.
- **Servidor de lenguaje.** Latencia desde cero, sin cambios y tras un cambio trivial, al estilo de
  `analysis-bench` y de las métricas publicadas de rust-analyzer
  (<https://rust-analyzer.github.io/metrics/>).
- **Formateador.** Rendimiento sobre el corpus propio y sobre fuentes sintéticas grandes. Ruff publica
  comparativas de linter y formateador (<https://github.com/astral-sh/ruff>); su metodología detallada no se
  verificó.
- **Recarga en caliente.** Latencia p50/p95/p99 del ciclo de edición hasta ver el cambio, por nivel H1–H3,
  según `docs/F0_ACUERDOS.md` §9.8 y `docs/adr/F0-D17-autoalojamiento-y-backend.md`.
- **Arranque y memoria.** Arranque en frío del proceso, RSS máximo y tamaño de imagen.

## 5. Metodología

- Usar los tamaños oficiales de cada benchmark para poder comparar con tablas existentes, y un tamaño pequeño
  como prueba de humo.
- Descartar el calentamiento en el intérprete y declarar iteraciones y semillas.
- Informar mediana, p95, p99 y dispersión; HttpArena toma el mejor de tres ejecuciones, por lo que conviene
  mostrar también la mediana.
- Separar siempre configuración estándar de configuración afinada y usar las mismas banderas y datos para
  todos los lenguajes.
- Con solo intérprete, comparar únicamente con otros intérpretes y medir razones, no tiempos absolutos; la
  comparación con lenguajes compilados espera al backend nativo.
- Registrar hardware, versión de cada herramienta y comandos exactos para que las medidas sean reproducibles.

## 6. Orden sugerido

1. Con el intérprete actual: programas de Benchmarks Game y plb2 (bajo coste y comparables entre sí), y
   Are We Fast Yet como suite principal de abstracciones.
2. Con asignación dinámica y texto: parser JSON sobre `twitter.json` y binary-trees.
3. Con servidor de lenguaje y formateador: métricas de latencia propias.
4. Con recarga en caliente: ciclo de edición por nivel.
5. Con backend nativo: comparación con lenguajes compilados y, solo con HTTP/2 y E/S real, HttpArena.

## 7. Lo que falta por verificar

El significado de `mem=1` y la regla literal de `tuned=0` en HttpArena; sus herramientas de carga, reglas
de alta y documento `#doc=add-framework`; si el Benchmarks Game admite lenguajes nuevos; CoreMark, SQLite
speedtest1 y las métricas concretas de rustc-perf; benchmarks de Zig, Go, clang y swc; y la metodología de
Ruff.
