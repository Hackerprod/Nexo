# Nexo — Roadmap del lenguaje

**Documento:** NEXO-ROADMAP-001 · **Revisión:** 0.2 · **Fecha:** 19 de octubre de 2026  
**Estado:** propuesta de diseño y validación; no es una especificación 1.0 aprobada ni un informe de resultados.  
**Destino:** revisión técnica del núcleo, planificación de implementación y evaluación con programadores.
**Base:** revisión 0.1; se conservan sus principios, contratos y familias T01–T16, salvo las precisiones identificadas en el registro siguiente.

> **Nexo es un lenguaje de propósito general donde los datos se copian, los recursos se transfieren y el estado compartido se declara. Los algoritmos se escriben sobre vistas, los costes se pueden observar y restringir, y un programa sencillo puede ganar control sobre la máquina sin desechar su lógica.**

En este documento, «se copian» describe independencia observable, no una copia física obligatoria. «Se transfieren» no impide prestar temporalmente un recurso. Conservar la lógica es un objetivo que se medirá: no se promete que cualquier programa alcance cualquier presupuesto añadiendo anotaciones.

Todas las fases están **pendientes de ejecución**. Los umbrales son criterios propuestos para registrar antes de medir, no resultados obtenidos. Los ejemplos utilizan sintaxis ilustrativa de Nexo y no deben presentarse como programas ejecutables de un compilador existente.

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

## Índice

0. Nexo en 10 minutos: qué gana el programador y cuatro programas completos.
1. Principios fundacionales y reglas no negociables.
2. Qué toma Nexo de cada lenguaje.
3. Registro de decisiones y respuesta a las posiciones de Claude.
4. Fases, dependencias y criterios verificables.
5. Batería transversal de pruebas y evidencia.
6. Diseño integrado: memoria, puente Python, biblioteca esencial y adopción.
7. Validación con programadores de Python, Rust y C#.
8. Riesgos y criterios de abandono o pivote.
9. Comparativa con Swift, Hylo, Rust, Zig y Mojo.
10. Extensiones tardías y opcionales.
11. Alcance comprometido y exclusiones de la versión 1.0.
12. Fuentes primarias.
13. Matriz de trazabilidad de la revisión 0.2.

---

## 0. Nexo en 10 minutos: qué gana el programador

### 0.1. La razón para probarlo

**Empezar con datos sencillos y conservar el algoritmo cuando haga falta controlar memoria, representación y concurrencia.** Esa es la propuesta de producto. Se añade una vía de adopción por módulos: una persona puede probar una función desde Python antes de trasladar una aplicación.

| Situación del programador | Experiencia que se quiere ofrecer | Prueba de valor |
|---|---|---|
| Un script resuelve el problema, pero la lógica local se vuelve lenta. | Tipos inferidos, ejecución inmediata, biblioteca esencial y compilación nativa del mismo código. | Resolver una tarea correcta con esfuerzo comparable y acelerar el trabajo útil, incluyendo adaptación. |
| Una simulación necesita otro almacenamiento. | El algoritmo recorre una vista; la colección elige AoS o SoA y prepara su almacenamiento exclusivo. | Cambiar disposición sin mantener otra implementación y con menos tiempo de modificación. |
| Una ruta crítica introduce reservas inesperadas. | El editor localiza la operación y su cadena de llamadas; el contrato impide reintroducirla. | Encontrar y corregir la regresión más rápido, sin desplazar el coste fuera de la medición. |
| Un proyecto combina tipos propios, JSON y bibliotecas nativas. | Derivación estática, herramientas integradas y adaptadores con contratos reutilizables. | Completar una aplicación sin escribir pegamento repetitivo para lo esencial. |

Nexo no exige que todas las personas migren. F6 comprobará qué tareas y cohortes obtienen una ventaja suficiente para justificarlo.

### 0.2. Convenciones de estos ejemplos

Los siguientes archivos son **programas completos de la sintaxis y APIs propuestas**, no fragmentos con funciones auxiliares omitidas. Todavía no existe un compilador Nexo validado que permita afirmar que se han ejecutado. Cada archivo pasa a ser un caso de aceptación EX01–EX04: al llegar su fase, debe comprobarse y ejecutarse sin sustituir su lógica por pseudocódigo.

`Result`, `Option`, `Ok`, tipos primitivos y constructores básicos pertenecen al preámbulo. `use std.modulo` importa un módulo. `std.app.Error` es una unión documentada de errores estándar con conversiones declaradas; no una excepción universal. Una biblioteca reutilizable puede declarar una unión más pequeña. Los recursos tienen limpieza de ámbito; las funciones de E/S de los ejemplos devuelven `Result`. Los nombres y contratos de estas APIs se congelan junto con los ejemplos en F1–F3.

### EX01. Una herramienta que suma registros de un archivo — F2

**Archivo `total.nx`.** La entrada contiene un entero por línea; las líneas vacías se ignoran. La ruta es el primer argumento o `ventas.txt` por defecto.

```nexo
use std.app
use std.cli
use std.console
use std.fs
use std.parse

fn main() -> Result<(), app.Error> {
    let path = cli.positional(0, default: "ventas.txt")?
    let text = fs.read_text(path)?
    var total: i64 = 0

    for line in text.lines() {
        let field = line.trim()
        if field.is_empty() {
            continue
        }
        let value = parse.i64(field)?
        total = total.try_add(value)?
    }

    console.println("Total: {total}")?
    return Ok(())
}
```

**Uso previsto:** `nexo run total.nx -- ventas.txt`; `nexo build total.nx --release` crea su ejecutable nativo. Con las líneas `12`, `8` y `5`, la salida esperada es `Total: 25`. Un entero mal formado o un total fuera de rango termina con un error tipado y código de salida no nulo.

**Qué gana:** no escribe un ciclo de vida manual para el archivo ni un sistema de tipos completo para cada variable. El error de conversión y el desbordamiento siguen siendo visibles. El ejecutable de este ejemplo no necesita Python; sus dependencias de sistema se declaran en el paquete de distribución.

### EX02. Un servidor HTTP pequeño con JSON — F5

**Archivo `salud.nx`.** El servidor se limita a la interfaz local. La función de petición es síncrona y no guarda la vista recibida; el servidor administra las conexiones dentro de su ámbito.

```nexo
use std.app
use std.http
use std.json

@derive(JsonEncode)
struct Salud {
    estado: String
}

fn responder(req: http.RequestView) -> Result<http.Response, app.Error> {
    if req.method != http.Method.GET {
        return Ok(http.Response.text(405, "Método no permitido"))
    }
    if req.path != "/salud" {
        return Ok(http.Response.text(404, "Ruta no encontrada"))
    }

    let body = json.encode(Salud(estado: "ok"))?
    return Ok(http.Response.json(200, consume body))
}

fn main() -> Result<(), app.Error> {
    let server = http.bind(
        "127.0.0.1:8080",
        max_connections: 64,
        max_header_bytes: 16384,
        max_body_bytes: 1048576,
        idle_timeout_ms: 5000
    )?

    http.serve(consume server, handler: responder)?
    return Ok(())
}
```

**Resultado esperado:** una petición `GET /salud` recibe estado 200 y un objeto JSON con `estado` igual a `ok`. El cuerpo de la respuesta es propietario y permanece vivo mientras el transporte lo necesita. `http.serve` consume el servidor, atiende hasta la señal de cierre documentada y reúne sus tareas antes de retornar; los tests proporcionan un controlador de cierre sin depender de señales manuales.

**Qué gana:** servidor, serialización y límites básicos disponibles sin elegir paquetes para cada pieza. Este ejemplo no configura TLS ni una política de despliegue público; la biblioteca y su perímetro están definidos en §6.7.

### EX03. Una simulación con almacenamiento intercambiable — F3

**Archivo `particulas.nx`.** `TableLayout` es un trait sellado para los layouts soportados. `UniqueTableView<T,L>` es una vista mutable exclusiva con procedencia de ámbito; `L` se especializa en compilación. Una proyección de fila puede exponer campos separados sin fingir que existe una estructura contigua.

```nexo
use std.app
use std.console
use std.table.{Table, TableLayout, UniqueTableView, SoA, AoS}

struct Particula {
    x: f32
    vx: f32
}

fn avanzar<L: TableLayout>(inout ps: UniqueTableView<Particula, L>, dt: f32)
    with no_alloc, no_block, no_suspend, no_io {
    for inout p in ps {
        p.x += p.vx * dt
    }
}

fn main() -> Result<(), app.Error> {
    var ps = Table<Particula, layout: SoA>.from_values([
        Particula(x: 0.0, vx: 1.0),
        Particula(x: 2.0, vx: -1.0)
    ])

    with ps.prepare_unique() as view {
        avanzar(inout view, dt: 0.5)
    }

    console.println("Primera posición: {ps.row(0).x}")?
    return Ok(())
}
```

**Resultado esperado:** la primera posición es `0.5`. Cambiar `layout: SoA` por `layout: AoS` conserva el cuerpo de `avanzar`; las pruebas comparan también la segunda partícula y los límites del recorrido. La preparación y la salida por consola están fuera del contrato del núcleo, pero dentro del tiempo y memoria totales de la aplicación.

`from_values` y `prepare_unique` son APIs cómodas con la política de agotamiento de memoria del proceso de D02. Sus variantes fallables y la comprobación sin reserva tienen contratos separados. La vista evita que una nueva copia incompatible aparezca durante la actualización.

**Qué gana:** puede experimentar con el almacenamiento sin duplicar la fórmula de la simulación. El contrato impide que una dependencia añada una reserva al núcleo sin que el compilador lo señale.

### EX04. Una secuencia perezosa sin construir un array — F2

**Archivo `cuadrados.nx`.** Un generador síncrono conserva su estado y entrega un valor por iteración; no crea una tarea ni un hilo. El rango es semiabierto.

```nexo
use std.app
use std.console

// Para un u32, el cuadrado cabe en u64.
gen fn cuadrados(limite: u32 = 4) -> u64 {
    for n in 0..limite {
        let wide = u64(n)
        yield wide * wide
    }
}

fn main() -> Result<(), app.Error> {
    for value in cuadrados(limite: 4) {
        console.println("{value}")?
    }
    return Ok(())
}
```

**Resultado esperado:** cuatro líneas: `0`, `1`, `4`, `9`. La conversión `u64(n)` es un ensanchamiento explícito exacto. El estado del generador se destruye al terminar, al salir antes del bucle o al propagar un error de salida; no requiere materializar los cuatro resultados.

**Qué gana:** la comodidad de producir elementos bajo demanda con tipos y limpieza definidos. Los costes de crear, avanzar y destruir el generador se muestran por separado.

### 0.3. Del ejemplo a la evidencia

F0 puede interpretar subconjuntos de estos programas y usar una biblioteca simulada para explorar interfaces, identificando los reemplazos. Para aprobar la fase asignada a cada ejemplo se utilizará la biblioteca real y el archivo completo. EX01 y EX04 se prueban además en la REPL y como doctests; EX03 en ambos layouts; EX02 con límites, errores y cierre automatizado.

Los cuatro ejemplos se publicarán con entradas, salidas esperadas, versión mínima del compilador y estado de ejecución. El playground de F6 ejecutará EX01 con un archivo virtual, EX03 y EX04; mostrará EX02 como código descargable porque el navegador no ofrece el mismo servidor TCP.

---

## 1. Principios fundacionales y reglas no negociables

### P01. Un solo lenguaje, una sola semántica

El modo de ejecución, el nivel de optimización y el grado de control no cambian el significado del programa. El intérprete y el compilador comparten las reglas de tipos, acceso, errores y evaluación.

No habrá un «modo cómodo» que permita accesos inválidos ni un «modo rápido» que los convierta silenciosamente en comportamiento indefinido. Un contrato adicional puede rechazar un programa seguro porque utiliza demasiados recursos; no modifica sus resultados para hacerlo encajar.

### P02. Datos independientes, identidad explícita

El núcleo distingue propiedades que se propagan por los campos, las colecciones, los genéricos y las capturas:

| Categoría | Regla | Ejemplo |
|---|---|---|
| Datos con semántica de valor | Una copia lógica no permite observar las modificaciones privadas de la otra. | Una posición, un documento sin estado compartido, una colección de números. |
| Recursos | No admiten duplicación implícita; se prestan o se transfiere su propiedad. | Un archivo abierto, una conexión, una escena viva, un registro de callback. |
| Estado compartido declarado | Su identidad y reglas de acceso son parte de su tipo. | Datos inmutables compartidos, un actor, un estado protegido por un mutex. |

Estas categorías no se implementarán como tres etiquetas excluyentes pegadas a cualquier objeto. Por ejemplo, un identificador de actor puede ser copiable sin ser una instantánea independiente del actor. El compilador distinguirá, como mínimo, capacidad de copia, independencia transitiva y capacidad de transferencia entre tareas.

Una estructura que contiene estado mutable compartido no adquirirá independencia simplemente por estar envuelta en un `struct`. Los constructores de copia con efectos externos arbitrarios quedan fuera del mecanismo de copia implícita.

La semántica de valores mutables tiene antecedentes directos en Hylo y Swift; Nexo adopta esa dirección, no reclama haberla inventado. [R01] [R02]

### P03. Préstamos locales e inferidos; exclusividad real

Los accesos ordinarios de lectura podrán prestarse sin sintaxis adicional. `inout` indicará modificación visible para el llamador y requerirá acceso exclusivo durante el préstamo. Un préstamo no podrá sobrevivir al objeto o región que lo respalda.

`inout` **no prueba** que el almacenamiento interno sea físicamente único: una colección puede compartir un búfer con otra copia lógica. Las APIs que necesiten exclusividad física la prepararán o comprobarán expresamente.

Se priorizará inferencia local. Las interfaces públicas que devuelvan vistas o dependan de regiones podrán necesitar contratos explícitos de procedencia. No se promete inferir cualquier relación de duración. La exclusividad de accesos mutables ya es una parte importante del modelo de Swift. [R02]

### P04. Las optimizaciones no cambian el comportamiento observable

Se preservarán los resultados definidos, los errores del programa, las modificaciones autorizadas, los efectos externos y el orden que exija la especificación. Una optimización tampoco podrá introducir un acceso inválido ni incumplir un contrato declarado.

No se exige conservar instrucciones, direcciones internas de valores, tiempos ni número de reservas eliminables. El contrato deberá separar esos detalles de las observaciones semánticas. El coste de la variante compilada se reportará aparte.

Las liberaciones de recursos con efectos observables respetarán su orden especificado; no se tratarán como simples reclamaciones de bytes. Los fallos ambientales, como agotamiento de memoria del proceso, tendrán una política documentada y no una promesa de reproducción idéntica entre implementaciones.

En concurrencia se preservarán los comportamientos admitidos por el modelo de memoria y sus obligaciones de progreso, no una intercalación particular de instrucciones. Los tests diferenciales de programas concurrentes usarán escenarios controlados y especificaciones de resultados permitidos.

### P05. Separar tipos, acceso, efectos, plataforma y tiempo

El tipo describe datos; la propiedad y los préstamos, acceso y duración; los efectos, operaciones permitidas; el backend, representación y destino. Una garantía temporal exige además un modelo de trabajo y de plataforma.

`no_alloc`, `no_block`, `no_suspend` y `no_io` serán contratos distintos. «Nativo», «gestionado» y «tiempo real» no serán tres efectos intercambiables. Existen antecedentes de comprobación de restricciones de reserva y bloqueo en Clang. [R03]

### P06. Costes observables sin inventar precisión

Los diagnósticos distinguirán **propiedad verificada**, **suposición externa**, **estimación**, **medición** y **desconocido**. El editor mostrará el alcance, la configuración y la causa de cada dato; no una cifra aparentemente exacta por línea cuando no pueda justificarla.

Los costes incluirán temporales, adaptación de interfaces, cierres, conteo de referencias y limpieza. La atribución al código fuente podrá abarcar un bloque o cadena de llamadas, porque una transformación puede fusionar varias expresiones. Los informes de optimización de LLVM constituyen infraestructura previa útil. [R04]

### P07. Seguridad con una frontera de confianza declarada

La garantía de código seguro es condicional a la corrección del compilador, el runtime, las primitivas auditadas y los contratos de componentes externos. Usar Rust para implementar el compilador no demuestra por sí solo la seguridad de Nexo.

Las operaciones de bajo nivel que no pueda verificar el sistema se confinarán a módulos `unsafe` pequeños, identificables y revisables. Su encapsulación no demuestra automáticamente que una biblioteca C cumpla lo declarado. [R05]

### P08. Valor sin extensiones experimentales

Nexo debe servir para construir y mantener programas ordinarios sin `@variant`, modelos de IA, conexión a servicios remotos ni prueba formal de equivalencia entre funciones completas. Su identidad es el lenguaje y la experiencia de programación.

---

### P09. Los diagnósticos son parte del producto

Un error debe ayudar a corregir el programa, no limitarse a nombrar una regla. Toda familia de errores estable tendrá un código, ubicación principal, causa, ubicaciones relacionadas y una explicación consultable. Cuando exista una modificación útil, indicará sus consecuencias: copiar, transferir, cambiar de ámbito o admitir otro efecto no son soluciones equivalentes.

El compilador, la REPL y el editor compartirán el mismo diagnóstico estructurado. Las sugerencias automáticas no añadirán `unsafe`, debilitarán un contrato ni cambiarán silenciosamente la semántica. Los fallos de genéricos, derivación y costes mostrarán la cadena de requisitos o llamadas hasta el origen pertinente. El diseño de diagnósticos de Rust es un antecedente de ingeniería, no un estándar que Nexo haya alcanzado ya. [R49]

**Puerta de producto:** F1 introduce el catálogo; F2 comprueba exactitud, equivalencia entre interfaces y reparaciones; F3 exige explicar costes transitivos. El estudio de F6 comprueba que las personas los pueden utilizar. Un mensaje técnicamente correcto pero sistemáticamente incomprensible requiere revisión.

---

## 2. Qué toma Nexo de cada lenguaje

Esta sección identifica **influencias concretas**, no una copia de sintaxis ni una atribución de invención exclusiva. Varias características pertenecen a tradiciones compartidas. Las fases F0–F6 forman el camino hacia 1.0; F7 es posterior y opcional.

### 2.1. Núcleo semántico y control de la máquina

| Lenguaje o tradición | Qué se toma | Cómo se integra y qué se descarta | Fase |
|---|---|---|---|
| **Python** | Lectura directa, colecciones de alto nivel, exploración interactiva, generadores y ejemplos ejecutables en documentación. [R06] [R47] [R48] | Inferencia estática, `nexo run`, REPL y `yield` síncrono; no compatibilidad de código ni adopción del aliasing mutable, las excepciones o todo su dinamismo. | F1–F2 |
| **Hylo** | Semántica de valores mutables, programación genérica y acceso mediante proyecciones. [R01] | Base de valores y vistas; se evalúa expresamente cómo añadir regiones, recursos y costes sin destruir esa sencillez. | F0–F3 |
| **Swift** | Valores, exclusividad de acceso mutable, recursos no copiables y aislamiento concurrente. [R02] [R07] [R08] | Se mantienen las distinciones entre valor, recurso y estado compartido; no se presenta ARC como una solución gratuita para todos los casos. | F1–F5 |
| **Rust** | Propiedad, transferencias, préstamos, traits nominales y derivación estructural. [R05] [R09] [R43] [R45] | Inferencia local, coherencia de implementaciones y derivación mediante `comptime`; las restricciones siguen existiendo y se explican. No se importan todas las reglas de especialización ni macros procedurales arbitrarias. | F0–F5 |
| **C** | Representaciones de bajo nivel e interoperabilidad mediante sus interfaces y convenciones binarias. [R10] [R05] | Tipos compatibles con C y módulos de frontera. No se importa aritmética de punteros sin control al código ordinario ni se promete una ABI universal entre plataformas. | F2 mínima; F4 completa |
| **C++** | Genéricos especializados, restricciones sobre parámetros y abstracciones que admiten implementación de bajo coste. [R11] | Genéricos comprobados contra sus requisitos; especialización medida. No se reproduce toda la maquinaria de plantillas, sus reglas históricas o el preprocesador textual. | F1–F3 |
| **Zig** | Asignadores explícitos, evaluación en compilación y herramientas de interoperabilidad con C. [R12] | Contextos de asignación y `comptime` restringido; se añaden comprobaciones de duración. No se supone que importar una cabecera aporte propiedad o efectos. | F1–F4 |
| **Koka** | Inferencia y composición de efectos; optimización del conteo de referencias como línea de estudio. [R13] | Efectos transitivos y evaluación de costes del conteo. D09 define un protocolo propio de confinamiento y publicación; no se atribuye a Koka una conversión universal local→atómica ni se exigen manejadores generales. | F0 y F3 |
| **F# y la tradición ML** | Inferencia de tipos y modelado explícito de alternativas. [R14] [R42] | Tipos algebraicos, `Option`, `Result` y patrones exhaustivos; las firmas públicas dan límites al trabajo del inferidor. | F1 |

### 2.2. Productividad, concurrencia y aplicaciones

| Lenguaje | Qué se toma | Cómo se integra y qué se descarta | Fase |
|---|---|---|---|
| **Kotlin** | Tratamiento explícito de ausencia de valor, argumentos con nombre y valores por defecto. [R15] [R46] | `Option<T>`, etiquetas de argumentos comprobadas y valores por defecto sin efectos. El puente futuro con Kotlin no obliga a adoptar la JVM como runtime de Nexo. | F1; puente F7 |
| **Go** | Herramientas coherentes, formateo, concurrencia por tareas y una biblioteca HTTP accesible. [R16] [R52] | Comandos unificados, tareas con ámbito y baterías esenciales mantenidas conjuntamente. Los mensajes o canales no conceden por sí solos ausencia de carreras. | F2 y F5 |
| **Erlang** | Procesos lógicos aislados y comunicación por mensajes. [R17] | Actores y supervisión de errores recuperables. No se promete reproducir el aislamiento de su máquina virtual ni sobrevivir a corrupción del proceso nativo. | F5 |
| **C#** | Modelo semántico reutilizable por editor, diagnósticos y refactorizaciones, como en Roslyn. [R18] | Servidor incremental para compilador/LSP y métodos de extensión con resolución definida. La derivación estática cubre formato y serialización; no se exige CLR ni reflexión dinámica. | F2 |
| **Java** | Interfaces de compilador, diagnósticos estructurados y organización de herramientas para proyectos. [R19] | APIs estables para herramientas y módulos. No se hereda su modelo de objetos ni se exige una máquina virtual al ejecutar. | F2–F4 |
| **JavaScript** | Composición de operaciones asíncronas. [R20] | `await` y tareas con efectos y cancelación definidos. No se copian coerciones implícitas ni se impone un único hilo a todo el lenguaje. | F5 |
| **TypeScript** | Adopción gradual y utilidad del análisis estático en el editor. [R21] | Integración por módulos y fronteras tipadas. Su influencia no es «compilar a Wasm»: Nexo diseñará ese backend por separado y no borrará las garantías de memoria al compilar. | F2–F4 |

### 2.3. Datos, matemáticas e historial

| Lenguaje o sistema | Qué se toma | Cómo se integra y qué se descarta | Fase |
|---|---|---|---|
| **Julia** | Operaciones numéricas expresivas y composición de recorridos vectoriales. [R22] | Genéricos y operaciones sobre vistas; fusión y SIMD solo cuando preserven la semántica. No se exige reproducir su despacho dinámico o JIT. | F3 |
| **SQL, con PostgreSQL como referencia** | Separar la consulta lógica del plan y explicar cómo se ejecuta. [R23] | Operadores de colecciones y planes visibles. Reordenar exige respetar efectos, errores y orden observable; no todo se decide en `comptime`. | F3; consultas avanzadas F7 |
| **Clojure** | Colecciones persistentes e historial basado en versiones que comparten estructura. [R24] | Biblioteca de instantáneas con límites de retención y elementos apropiados. No se promete que todo array sea persistente ni que conservar versiones no consuma memoria. | F3–F5 |
| **Mojo** | Conexión entre valores, control de bajo nivel, metaprogramación e interoperabilidad con Python y C. [R25] [R26] | Se estudia esa continuidad como precedente y competidor. No se adopta una identidad centrada en un dominio ni se afirma que Nexo sea el primero en unir ergonomía y propiedad. | F2–F4; hardware adicional F7 |
| **Halide y Exo** | Separación entre algoritmo y decisiones de ejecución; transformaciones dirigidas con condiciones de corrección. [R27] [R28] | Referencias para especialización y variantes futuras, no dependencia del núcleo 1.0. | F7 |

**Regla de integración:** una característica no entra por ser atractiva en su lenguaje de origen. Debe tener una función concreta en Nexo, una interacción definida con memoria y efectos, pruebas positivas y negativas y un coste de implementación asumible. Esta tabla no obliga a incorporar todas las influencias a la versión 1.0.

---

## 3. Registro de decisiones

Cada decisión tendrá un identificador, motivación, alternativas, ejemplos aceptados y rechazados, responsable y pruebas asociadas. Modificar una decisión después de medir exige nueva revisión y nueva evaluación; no se reinterpreta retrospectivamente un resultado fallido.

### D01. Sintaxis: aceptación con condiciones

**Posición de Claude:** llaves y punto y coma opcional.

**Decisión:** aceptar llaves, `let`/`var`, `fn`, `struct`, patrones y terminadores opcionales mediante una regla léxica pequeña y documentada. No se acepta el argumento de que las llaves, por sí solas, demuestren mayor robustez del analizador.

La gramática debe resolver continuación de expresiones, `return`, llamadas multilínea y acceso encadenado sin depender de inferencia de tipos. Dos sentencias en una línea necesitarán separación explícita. El formateador no podrá cambiar el árbol sintáctico ni los efectos. Los archivos utilizarán provisionalmente `.nx` y el manifiesto `nexo.toml`. Los ejemplos adoptan comentarios `//`, rutas de importación `std.modulo`, literales UTF-8 e interpolación con `{expresion}`; `{{` y `}}` escapan llaves. D10 fija argumentos y generadores. Las funciones usan `return` explícito para valores; la sintaxis de expresión final implícita queda fuera de 1.0.

**Validación:** corpus de ambigüedades y propiedad de ida y vuelta entre análisis y formato en F1–F2. Si la omisión de terminadores produce confusiones recurrentes, se endurece la regla antes de congelar la sintaxis.

### D02. Errores: aceptación parcial y rechazo de una garantía de aislamiento

**Posición de Claude:** `Result`, `?`, errores tipados, sin excepciones; un fallo irrecuperable aborta la tarea y no el proceso.

**Decisión:** aceptar `Result<T,E>`, propagación con `?`, conversiones de error explícitas o declaradas y tratamiento separado de cancelación. No habrá excepciones generales como canal ordinario de control.

**Se rechaza la última promesa como regla universal.** En 1.0, un error recuperable puede terminar una tarea mediante su resultado y ejecutar su limpieza. Un `panic` o fallo interno no recuperable será, por defecto, fallo del proceso. Aislarlo requeriría otro modelo de ejecución o una frontera demostrablemente adecuada. No se continuará tras corrupción de memoria de una biblioteca externa suponiendo que el resto del proceso está intacto.

Las comprobaciones fallables esperadas ofrecerán APIs con `Result`/`Option`. Los destructores no propagarán errores mediante `?`: los recursos cuyo cierre pueda fallar tendrán cierre explícito fallable y una política de limpieza residual documentada. No se permitirá que una excepción de un lenguaje extranjero atraviese una frontera C sin adaptación.

**Adición 0.2:** F5 abre un experimento separado de aislamiento de `panic` en tareas controladas. Debe incluir limpieza, invariantes de estado compartido, código generado y runtime, no solo ausencia textual de `unsafe` o FFI. Su fracaso no bloquea 1.0: la política estable sigue siendo abortar el proceso. Una sesión REPL puede reiniciarse tras ese aborto, pero no conservar recursos o efectos externos como si nada hubiese ocurrido.

### D03. `comptime`: aceptar un subconjunto puro y acotado

**Posición de Claude:** puro, determinista y sin entrada/salida.

**Decisión:** aceptar. El evaluador podrá usar constantes, tipos y metadatos autorizados; no leerá archivos, reloj, red, variables de entorno o propiedades accidentales del anfitrión. El objetivo de compilación será una entrada explícita. Habrá límites reproducibles de pasos, profundidad y memoria, con diagnósticos cuando se excedan.

La generación externa de código será una etapa separada, con entradas y salidas declaradas. La pureza de `comptime` no basta para reproducir el binario: también deben fijarse dependencias, herramientas, opciones, rutas relevantes y etapas de enlace. [R29]

Para 1.0 se excluyen macros textuales, reflexión dinámica obligatoria y manipulación arbitraria del árbol del programa. Los genéricos básicos no dependerán de tener metaprogramación ilimitada.

**Derivación estática comprometida:** se habilita `@derive(...)` sobre metadatos tipados y autorizados, no sobre texto arbitrario. F1 incorpora `Format`, `PartialEq` y, cuando los campos lo permitan, `Eq` y `Hash`; F2 añade `JsonEncode`/`JsonDecode` básicos; F3 completa nombres de campo, exclusiones explícitas, límites y derivaciones genéricas restringidas. La implementación generada vuelve a pasar el comprobador normal de tipos, propiedad y efectos. El código expandido y su origen son inspeccionables.

Una derivación solo accede a campos privados con la autorización del módulo que declara el tipo. No puede fabricar `Send`, independencia, exclusividad ni una copia de un recurso. `Eq` no se deriva automáticamente para campos de igualdad parcial; serializar no significa copiar bits de memoria. `Decode` devuelve `Result`, valida rangos y límites y no construye tipos con invariantes sin su constructor autorizado. Omitir un campo exige una política definida para reconstruirlo.

La biblioteca define un formato etiquetado para enums y una política explícita de campos JSON desconocidos; no deduce esquemas SQL ni un ORM. Los descriptores serializables son valores estáticos opcionales: no hay un registro universal de tipos en ejecución. La derivación de Rust y la introspección en compilación de Zig son antecedentes distintos que se adaptan a esta decisión. [R45] [R12]

### D04. Implementación: aceptar Rust y una IR propia; escalonar backends

**Posición de Claude:** compilador en Rust, representación intermedia propia, backend rápido de depuración y LLVM para producción.

**Decisión:** aceptar Rust y una representación intermedia tipada de Nexo. La IR conservará procedencia de préstamos, transferencias, efectos, limpieza, vistas y relación con el código fuente hasta completar sus comprobaciones.

F0 permite un prototipo vertical desechable y F1 entrega el intérprete del núcleo. F2 añade **LLVM como backend nativo principal**, con configuraciones de desarrollo y optimización.

**Contingencia preparada, no una segunda implementación completa por adelantado:** F0 especifica la interfaz de backend, ABI interna y artefactos de depuración; F2 construye un pequeño adaptador experimental a Cranelift para escalares, llamadas y limpieza, con pruebas diferenciales. Su cometido es medir integración y viabilidad, no anunciar soporte completo. Cranelift documenta un equilibrio propio entre compilación y optimización. [R30]

La contingencia se activa con los umbrales y desglose de F2. Si falla la puerta de edición y la generación/enlace es el cuello de botella, se amplía el backend rápido o se presenta una solución equivalente medida antes de F6. Si el coste está en inferencia o monomorfización, cambiar backend no sustituye corregirlo. Ningún backend evita las comprobaciones del frontend. La producción conserva LLVM; un backend de desarrollo promovido debe pasar el corpus completo en los destinos donde se distribuya.

`nexo run` comprobará estáticamente el programa antes de ejecutarlo. La elección de intérprete o código nativo no cambiará su lenguaje. El compilador como servidor incremental compartirá consultas con el LSP. No se exigirá MLIR además de LLVM; su uso requerirá una necesidad y evaluación separadas.

### D05. Arenas y regiones: aceptar, sin promesas de tiempo gratuito

**Posición de Claude:** eliminación explícita inicial; regiones por ámbito; posible recolección posterior.

**Decisión:** aceptar arenas generacionales de larga duración y regiones de duración léxica. El acceso mediante identificadores comprobará contenedor, posición y generación; las comprobaciones redundantes podrán eliminarse si se justifican. Las arenas generacionales son un mecanismo previo para evitar que un identificador antiguo acceda a otro objeto al reutilizar una posición. [R31]

La región liberará su almacenamiento al terminar, pero el coste depende de bloques, destructores y acciones pendientes. No se prometerán «reservas casi gratuitas» ni «liberación instantánea». Las llamadas usarán un contexto de asignación declarado: entrar en una región no redirige mágicamente asignaciones de código extranjero o bibliotecas no compatibles.

La reclamación automática de ciclos arbitrarios dentro de una arena queda fuera de 1.0. No habrá recolector global obligatorio. Para grafos con identidad se utilizarán contenedores propietarios e identificadores; para ciclos en estado compartido se exigirán políticas explícitas, referencias débiles o ruptura de enlaces, sin prometer ausencia universal de fugas.

### D06. Licencia y gobernanza: aceptar y concretar el proceso

**Posición de Claude:** Apache 2.0 desde el primer commit y propuestas públicas.

**Decisión:** adoptar Apache 2.0 para el código original del compilador y sus herramientas, conservando las obligaciones de los componentes reutilizados. Esta entrega no crea ni publica un repositorio. La licencia de referencia es el texto de Apache Software Foundation. [R32]

El repositorio deberá incluir `LICENSE`, instrucciones de contribución, responsables, registro de decisiones y proceso público de propuestas. Las contribuciones externas podrán presentarse desde su apertura y estarán sujetas a las mismas pruebas y revisión técnica. Los cambios semánticos necesitarán decisión registrada y explicación de compatibilidad.

Publicar propuestas no equivale por sí solo a una fundación independiente o gobierno comunitario. La autoridad para aprobar cambios, nombrar responsables y resolver desacuerdos deberá constar en la gobernanza. «Nexo» continúa siendo un nombre de trabajo. F0 abre el registro de nombres alternativos y F4–F5 realizan búsqueda de conflictos en marcas, proyectos, paquetes y dominios. Antes de entrar en F6 se exige un informe de disponibilidad o una decisión de cambio de nombre; no basta con que esté libre un dominio. Una búsqueda de registro, como la de USPTO, es una entrada del proceso y no una conclusión jurídica universal. Esta revisión no afirma haber completado una autorización de marca. [R55]

### D07. Semántica adicional que debe congelarse en F0

Se adoptan evaluación secuencial definida, inicialización obligatoria, ausencia de nulos universales y aritmética entera comprobada por defecto, con operaciones modulares explícitas. La optimización no cambiará estas reglas entre depuración y producción.

Las operaciones flotantes tendrán un contrato estricto de referencia, sin reasociación o aproximación silenciosa. F0 especificará redondeo, valores especiales, conversiones y observaciones de bits; las bibliotecas matemáticas documentarán las dependencias del destino. No se confundirá SIMD con autorización para cambiar resultados.

El alcance léxico fijará la liberación de recursos salvo transferencia o cierre explícito. El compilador podrá acortar la vida de almacenamiento interno únicamente cuando no altere observaciones ni contratos. Los tipos públicos y efectos prometidos formarán parte de las interfaces de paquetes.

**Enteros en 0.2:** se conservan operaciones comprobadas en todas las compilaciones. `try_add`/`try_mul` devuelven errores recuperables; `wrapping_add` y equivalentes realizan aritmética modular definida; las operaciones saturadas tienen nombres separados. Para eliminar una comprobación sin cambiar la semántica, el compilador debe justificar que el rango lo permite. Una API `add_proven` exige evidencia verificable de su precondición, obtenida estáticamente o por una comprobación previa cuya validez se conserva. Una afirmación textual del programador no es evidencia.

Las primitivas realmente no comprobadas quedan en `unsafe`, con obligación de rango documentada, y no se añaden como sugerencia automática. F2 registra el coste de comprobación en casos simples; F3 y F6 lo miden también tras vectorización y en aplicaciones completas. No se comparan variantes modulares y comprobadas como si fueran intercambiables fuera del dominio sin desbordamientos.

---

### D08. Interfaces, genéricos, coherencia y operadores

**Decisión F0:** traits **nominales** con `trait` e implementaciones explícitas mediante `impl`. La conformidad no se deduce por encontrar métodos del mismo nombre. Los traits expresan requisitos de operaciones, tipos asociados y efectos; permiten implementaciones de métodos por defecto. Se toma la comprobación por requisitos de Rust y Swift, no se mezclan dos sistemas de resolución. [R43] [R56]

```nexo
trait Medida {
    fn valor(self) -> f64 with no_alloc, no_block, no_suspend, no_io
}

struct Distancia {
    metros: f64
}

impl Medida for Distancia {
    fn valor(self) -> f64 with no_alloc, no_block, no_suspend, no_io {
        return self.metros
    }
}

fn doble<T: Medida>(dato: T) -> f64
    with no_alloc, no_block, no_suspend, no_io {
    return dato.valor() * 2.0
}
```

`self` y los parámetros ordinarios se prestan para lectura mientras no haga falta conservar una copia; `inout self` declara modificación y `consume self` transferencia. Un cuerpo genérico se comprueba contra sus requisitos antes de instanciarse. Usar un `T` no autoriza duplicarlo, enviarlo a otra tarea ni reservar memoria: esas operaciones necesitan requisitos o efectos correspondientes.

**Coherencia.** Cada par trait/tipo concreto tiene una implementación canónica en el grafo de paquetes. Un paquete puede implementar un trait que declara o implementarlo para un tipo nominal exterior que declara. Un alias o envolver un tipo local dentro de una colección extranjera no lo convierte en tipo propio. Implementar un trait extranjero para un tipo extranjero exige un nuevo tipo envoltorio local, no una excepción por importación. Es una regla inicial deliberadamente más sencilla que todas las excepciones de Rust.

Las implementaciones genéricas condicionales requieren requisitos explícitos y comprobación de solapamiento. Se rechaza una pareja potencialmente solapada cuando no se puede justificar su disjunción; no se selecciona por orden de enlace. No hay especialización semántica por prioridad, conformidades negativas arbitrarias ni resolución mediante ejecución ilimitada de `comptime` en 1.0. La especialización de código conserva la implementación ya resuelta. El límite y estrategia del solucionador se fijan en F0 y se prueban entre paquetes en T17.

**Genéricos.** Restricciones `T: Trait`, cláusulas `where`, tipos asociados y parámetros constantes de tipos admitidos. La monomorfización es la estrategia inicial para llamadas estáticas; presupuestos de instanciaciones, caché y deduplicación previenen crecimiento accidental. No se incluyen tipos de orden superior generales ni tipos asociados parametrizados arbitrarios. Las proyecciones prestadas requieren una relación de procedencia expresa en el trait de vista: devolver una referencia no es una escapatoria al modelo de duración.

**Despacho dinámico.** `dyn Trait` es explícito. Solo admite el subconjunto compatible: operaciones con representación y efectos conocidos, tipos asociados fijados y sin métodos genéricos abiertos que no puedan figurar en la tabla de despacho. Una vista prestada dinámica no implica reserva; un contenedor propietario con borrado de tipo declara su reserva y limpieza. F1 entrega despacho estático; F3 cierra el subconjunto dinámico y sus contratos. No hay herencia de clases como mecanismo adicional.

**Extensiones.** `extend Tipo` añade métodos de conveniencia sin campos ni nueva identidad. Se activan mediante importación explícita; no fabrican una conformidad ni alteran los métodos originales. Los métodos propios del tipo tienen prioridad. Entre extensiones o traits igualmente aplicables se produce ambigüedad diagnosticada y se exige calificación; nunca manda el orden de importación. Las funciones libres no tienen sobrecarga arbitraria por el tipo de retorno. Las etiquetas de argumentos forman parte de la API y de su caché.

**Operadores.** Un conjunto fijo de símbolos y precedencias se resuelve mediante traits como `Add`, `Mul`, `PartialEq` e `Index`. No se crean símbolos ni precedencias por paquete. `+=` implica modificación exclusiva y evalúa la dirección lógica una sola vez. `&&`/`||`, asignación, préstamo, `?` y control de flujo no son sobrecargables en 1.0. Un operador definido por el usuario conserva sus efectos: usar `+` no acredita una operación numérica pura o sin reservas. No se asumen leyes algebraicas para reordenar código por el mero nombre del trait.

Las propiedades de seguridad —copia admisible, independencia transitiva, transferencia, proyección disjunta— permanecen selladas o derivadas estructuralmente por el compilador. Una implementación de trait ordinaria no puede declararlas verdaderas sin satisfacer sus reglas.

**Pruebas y motivo:** T17 debe cubrir conformidad ausente, requisitos implícitos indebidos, solapamiento entre dependencias, extensión ambigua, efectos incompatibles, operadores con efectos y crecimiento de instanciaciones. La ventaja buscada es genéricos legibles y refactorizables sin que añadir un paquete cambie silenciosamente qué función se ejecuta.

### D09. Conteo de referencias, CoW y publicación concurrente

**Decisión F0:** eliminar conteo donde basta propiedad única o almacenamiento regional; usar conteo no atómico cuando todas las operaciones sobre una asignación están confinadas a un único dominio serial; utilizar conteo atómico para la propiedad compartida entre dominios concurrentes. La distinción entre conteo local y atómico tiene antecedentes explícitos en `Rc`/`Arc` de Rust. El conteo atómico protege el contador, no convierte por sí solo el contenido en mutable concurrentemente. [R44]

No toda copia diferida exige un contador convencional: puede apoyarse en pruebas de unicidad, regiones u otros metadatos. D09 elige una implementación inicial, no convierte ese detalle en la semántica del lenguaje.

| Estado de almacenamiento | Operaciones admitidas | Regla de salida |
|---|---|---|
| Único, sin observadores incompatibles | Lectura, modificación y transferencia del propietario. | Transferencia sin copia cuando todo su contenido transferido respete el contrato. |
| CoW local | Contador no atómico; alias físicos solo dentro del dominio confinado. | Separar almacenamiento antes de escribir cuando proceda; preparar publicación antes de cruzar dominios. |
| Compartido publicado | Contador atómico y contenido inmutable o con acceso sincronizado explícito. | Copiar/adquirir exclusividad conforme al contrato; no «degradar» a contador local con observadores vivos. |
| Regional prestado | Acceso dentro de la duración y aislamiento permitidos, sin RC obligatorio por nodo. | Reunir préstamos/tareas antes de destruir región; promoción explícita para escapar. |

**Confinamiento y tareas.** En el primer runtime, el dominio local es un ejecutor ligado a un hilo; `LocalRc` y recursos de afinidad no son transferibles. Una tarea que pueda migrar entre trabajadores no puede llevar contadores locales accesibles desde otro trabajador. Puede permanecer fijada al ejecutor, preparar sus datos transferibles o demostrar que todo el grafo se mueve sin acceso simultáneo. `await` no prueba ninguna de estas condiciones. Las capturas y los tipos compuestos propagan esta restricción.

**Publicación segura inicial.** El primer camino general materializa una instantánea independiente con control atómico, conservando intactos los alias locales existentes. Puede copiar estructura y reservar; el coste se atribuye a la frontera. No se modifica «en sitio» un campo de contador mientras puedan acceder a él usuarios no atómicos. Si se demuestra exclusividad de toda la parte transferida —no solo de la raíz— y no existen préstamos, referencias débiles ni retención extranjera incompatibles, se permite una conversión optimizada con estado quiescente y publicación sincronizada. Ese camino comienza por búferes planos exclusivos; no es requisito optimizar todos los grafos para F5.

El modelo de memoria define publicación *happens-before*, adquisición, decremento final y destrucción. La implementación inicial usa órdenes conservadores; cualquier relajación requiere revisión y pruebas del modelo. La disponibilidad de atomics del destino es parte del contrato del runtime. Un contador con accesos atómicos y no atómicos concurrentes sobre la misma representación está prohibido.

**Mutación y débiles.** Observar contador uno no es por sí solo un permiso de escritura. Se necesita exclusividad del handle y ausencia o tratamiento correcto de mejoras desde referencias débiles, publicación concurrente y alias anidados. La API de comprobación entrega una guarda válida durante el acceso, no un booleano que pueda caducar. Para empezar, un dato compartido atómico se trata como inmutable y una edición obtiene un almacenamiento privado salvo prueba suficiente de unicidad.

**CoW, costes y compatibilidad.** El usuario observa valores independientes en todas las rutas. `prepare_unique()` puede separar el búfer publicado y los componentes necesarios; no se transforma un `shared` lógico en un valor independiente sin operación explícita. Los rayos X muestran incrementos/decrementos, atomics, bytes duplicados, preparación, publicación y destrucción. La publicidad no presentará «conteo local» como coste cero.

**Ejecución por fases:** F0 fija invariantes y una máquina de estados; F2 implementa CoW local y mide contra una compilación diagnóstica con conteo siempre atómico; F3 integra unicidad y regiones; F5 habilita publicación y tareas. T18 incluye grafos con hijos compartidos, referencias débiles, cambios de ejecutor, callbacks, cancelación y liberación final. La comparativa registra rendimiento, memoria y coste de publicación: no basta ganar en un bucle monohilo ignorando el traslado a otra tarea.

### D10. Ergonomía cotidiana con semántica definida

**Texto.** `String` representa secuencias válidas de escalares Unicode codificadas en UTF-8; `Bytes` representa bytes arbitrarios. La conversión fallable devuelve `Result`. `byte_len`, iteración por escalares y recorrido de grafemas son operaciones distintas; no se permite indexar una cadena con un entero fingiendo que cada carácter ocupa un byte. Las fronteras de slicing se comprueban. La versión de datos Unicode se fija por edición de biblioteca; las reglas de grafemas tienen un estándar propio. [R54]

Interpolación desde F1, con evaluación izquierda→derecha una sola vez y requisitos `Format` comprobados. Formatear texto puede reservar; escribir directamente a un destino proporcionado declara sus efectos. No se normaliza silenciosamente el contenido. Las rutas de archivos usan un tipo de sistema independiente cuando no sean representables como texto UTF-8.

**Argumentos.** Nombres de parámetros disponibles como etiquetas en llamadas; los obligatorios pueden ser posicionales. Una vez utilizado un argumento con nombre, el resto son nombrados. No se admite duplicar un argumento. El orden de evaluación es el escrito en la llamada, antes de asociarlo a los parámetros. Las expresiones por defecto son valores puros evaluables en `comptime`, sin recursos ni estado mutable compartido; cada llamada obtiene el valor lógico independiente. Cambiar una etiqueta pública es cambio de API. No se replica la sobrecarga entera de Kotlin para ofrecer esta ergonomía. [R46]

**Iteración.** F1 incluye el protocolo síncrono `Iterator` con tipo asociado `Item` y `next(inout self) -> Option<Item>`. El iterador es normalmente un recurso no copiable, no un valor que se duplica implícitamente. Los efectos de creación, avance y limpieza se resumen por separado. Los iteradores que prestan una colección mantienen su duración; la variante mutable expone accesos disjuntos con ámbito, no permite guardar vistas inválidas.

**Generadores.** F2 incorpora `gen fn` y `yield`, inspirados en la producción perezosa de Python. [R48] Devuelven un recurso de estado concreto; el cuerpo comienza al avanzar, no al crear el generador. La implementación es una máquina de estados síncrona; almacenamiento embebido cuando su tamaño y uso lo permitan, reserva explícita al borrar el tipo o cuando la representación elegida lo requiera.

En F2 solo se entregan elementos propietarios: copiar valores admisibles o transferir recursos declarados. Las capturas retenidas tienen que ser propietarias; no se devuelve un generador con préstamos locales escapados. F3 puede admitir capturas prestadas con procedencia comprobada. No se entregan préstamos mutables que permanezcan activos tras un nuevo avance. Descartar o abandonar el generador ejecuta una sola limpieza de su estado.

`yield` cede al llamador de `next`, no al planificador, y no activa `async`. Para conservar el significado de `no_suspend` de 0.1, la suspensión local del generador se incluye en su resumen y en el de las operaciones que lo avanzan: no se permite prometer `no_suspend` mientras ese efecto esté presente. Un iterador ordinario que calcula y devuelve un elemento sin suspensión sí puede cumplirlo. Esta primera regla es conservadora; cualquier refinamiento posterior exige una decisión semántica, no ocultar la operación durante el lowering. Los efectos de E/S, bloqueo, reserva y callbacks también se conservan al avanzar. Un generador fallable entrega `Result<Item,E>`; no existe una excepción oculta al iterar. Generadores asíncronos y préstamos generadores avanzados no son compromiso 1.0.

**REPL.** Desde F2: `nexo repl` con evaluación estática por celda, `:type`, `:effects`, `:cost`, `:load`, `:reset` y salida. Misma semántica que `nexo run`. Las declaraciones se versionan; no se modifica el layout de objetos vivos al redefinir un tipo. Los recursos abiertos siguen siendo recursos: redefinir un nombre no oculta préstamos activos. Un error de compilación no ejecuta la celda; un error durante ejecución puede haber realizado E/S y no se promete revertirla.

**Documentación ejecutable.** Desde F2, `nexo test --doc` ejecuta bloques positivos y comprueba ejemplos `compile_fail` con código de diagnóstico esperado. Una guía no depende del texto exacto traducido de un error. Las pruebas están aisladas y no usan red por defecto; tiempo, memoria y dependencias se declaran. Los doctests de Python son un antecedente directo. [R47]

**Pruebas:** T21–T23 cubren UTF-8 inválido, Unicode, orden de evaluación, defaults, generación perezosa, finalización temprana, REPL y documentación. La comodidad forma parte de las puertas de fase, no de un plan posterior de pulido.

---

## 4. Fases, dependencias y criterios verificables

### 4.1. Cómo se aprueba una fase

Una fase termina con un informe reproducible. Cada requisito obtiene **PASA**, **FALLA**, **NO EVALUADO** o **INCONCLUSO**, con evidencia y responsable. Ningún requisito obligatorio de esa fase puede quedar pendiente al aprobarla. Una prueba automática no equivale a demostrar ausencia universal de errores; un defecto conocido que viola una garantía anunciada bloquea la promoción.

**Iteración permitida:** se puede prototipar una fase posterior para responder una pregunta de diseño. Ese experimento no permite anunciarla como aprobada ni omitir sus dependencias. Los protocolos se registran antes de los datos confirmatorios, no antes de poder escribir la primera línea de un prototipo. Un cambio semántico reabre las pruebas afectadas, con registro de qué se invalida.

No se estima duración ni se presupone un tamaño de equipo. F0 asignará responsables a núcleo/tipos, runtime/memoria, herramientas, interoperabilidad/biblioteca y validación/plataformas. Una persona puede cubrir varias áreas; una revisión crítica requiere otra persona competente. La indisponibilidad de esa revisión no prohíbe explorar, pero impide aprobar la garantía correspondiente.

| Fase | Resultado principal y ganancia visible | Dependencia de aprobación | Puerta de salida |
|---|---|---|---|
| F0 | Decisiones contrastadas con un prototipo vertical. | Acuerdo fundacional. | D01–D10 coherentes, invariantes y criterios registrados. |
| F1 | Escribir datos, funciones, genéricos y errores con buen feedback. | F0. | Núcleo e intérprete correctos en el corpus; diagnósticos útiles. |
| F2 | Ejecutar, compilar, explorar en REPL y llamar desde Python. | F1. | Semántica nativa, puente mínimo y ciclo de edición medido. |
| F3 | Mantener la lógica al controlar almacenamiento y costes. | F2. | Contratos, regiones, unicidad, vistas y derivación integrada. |
| F4 | Incorporar bibliotecas y distribuir módulos. | F3; puentes mínimos desde F2. | Perfiles C, Python completo acotado, paquetes y núcleo Wasm. |
| F5 | Construir servidores y simulaciones concurrentes completos. | F3–F4. | Publicación, tareas, HTTP, cancelación y cuatro plataformas probadas. |
| F6 | Justificar adopción y publicar 1.0. | F1–F5. | Superioridad registrada, usabilidad, distribución y mantenimiento. |
| F7 | Ampliar sin convertir experimentos en requisitos del núcleo. | Núcleo estabilizado. | Evaluación independiente por extensión. |

### 4.2. Plataformas obligatorias y niveles de evidencia

**1.0 incluye como anfitrión de las herramientas y destino de ejecución: Linux x86-64, Windows x86-64, macOS arm64 y Linux arm64.** La selección es una decisión de alcance: no depende de una cifra no verificada de cuota de mercado. Compilación cruzada se incorpora por pares documentados; soportar cuatro anfitriones no promete todos los cruces posibles entre ellos.

| Etapa | Linux x86-64 | macOS arm64 | Windows x86-64 y Linux arm64 |
|---|---|---|---|
| F0 | Máquina de referencia y herramientas fijadas. | Responsable, máquina nativa y cadena de herramientas identificados. | Responsables, hardware y plan de CI identificados. |
| F1 | Intérprete y corpus completo. | Construcción de herramientas y corpus reducido del intérprete. | Pruebas de construcción del frontend. |
| F2 | Backend, REPL y puente Python mínimo completos. | Backend, REPL y puente Python mínimo completos. | Generación, enlace y ejecución nativa de corpus de arranque: escalares, llamadas, texto y limpieza. |
| F3 | Núcleo, memoria y layouts completos. | Igual. | Núcleo, memoria, REPL y puente mínimo completos. |
| F4 | ABI, paquetes y perfiles C/Python. | Igual. | Igual para cada plataforma. |
| F5–F6 | Biblioteca, concurrencia y aplicaciones completas. | Igual. | Igual para cada plataforma. |

F0 fija versiones mínimas de sistema, libc/CRT, SDK, objetivos de CPU y versiones de Python/LLVM/Rust. Las actualizaciones quedan registradas y revalidan las partes afectadas. Las pruebas de ABI, atomics, cancelación y rendimiento de ARM se ejecutan en ARM real; emulación y compilación cruzada complementan, pero no sustituyen, esa evidencia. Los binarios no utilizan instrucciones opcionales sin comprobación o destino explícito.

No conseguir un destino obligatorio mantiene el proyecto pre-1.0 bajo esta revisión. Una reducción de plataformas requiere otra decisión de alcance, no una omisión silenciosa en CI.

### 4.3. Compilación y contingencia de backend

Se conservan los objetivos rápidos de 0.1 y se separan de puertas máximas iniciales para no convertir un deseo en una predicción. Se añaden proyectos de 100.000 líneas y un corpus de estrés de genéricos. Los valores se registran definitivamente en F0, antes de medir candidatos.

| Carga y operación | Meta de experiencia | Puerta máxima propuesta F6 |
|---|---:|---:|
| 10.000 líneas: diagnóstico incremental, p95. | 500 ms | 750 ms |
| 10.000 líneas: cambio local hasta ejecutable enlazado de desarrollo, p95. | 2 s | 4 s |
| 10.000 líneas: compilación limpia del proyecto. | 30 s | 60 s |
| 100.000 líneas: diagnóstico incremental, p95. | 1,5 s | 2 s |
| 100.000 líneas: cambio local hasta ejecutable enlazado, p95. | 8 s | 15 s |
| 100.000 líneas: compilación limpia del proyecto. | 180 s | 300 s |

Estas puertas se aplican a máquinas de referencia x86-64 y arm64 fijadas en F0 —clase inicial propuesta: al menos cuatro núcleos físicos, 16 GB de RAM y SSD local—, no a cualquier ordenador. Se documentan CPU exacta, energía, memoria, sistema, hilos y dependencias; las cuatro plataformas deben además pasar sus pruebas funcionales y medir sus propios tiempos. La condición de «al menos» no autoriza cambiar después a una máquina mayor para salvar un resultado.

La medición distingue construcción de dependencias, caché de compilación, caché de archivos, enlace y arranque. «Limpia del proyecto» mantiene herramientas y dependencias ya instaladas, pero elimina sus objetos de proyecto; la construcción total desde cero se reporta aparte. Los tiempos incrementales incluyen el análisis necesario y el enlace: no se anuncia el tiempo de comprobación como si fuese tiempo hasta ejecutar. Se añaden cambios de firma pública, tipo compartido, layout y metadatos de derivación, aunque las puertas rápidas se refieran al cambio local.

T26 usa al menos 100 ediciones registradas por categoría y 10 compilaciones limpias, con distribuciones y memoria máxima. El corpus de 100.000 líneas se estratifica en módulos, funciones, genéricos, vistas y código derivado; no se completa duplicando texto inerte. F2 puede utilizar un corpus sintético publicado para capacidad, identificado como tal. F6 añade un proyecto integrado de esa escala con funciones utilizadas y tests; el resultado sintético no sustituye la experiencia diaria.

**Activación de contingencia:** si una puerta se incumple en dos ejecuciones reproducibles de la misma revisión y generación/enlace representa al menos el 60% del tiempo pertinente, se compara el adaptador rápido preparado en F2 y se decide su ampliación. Primero se conserva el desglose de frontend/instanciación/optimización/enlace; un fallo dominado por tipos activa trabajo en el frontend. Si un backend rápido se promociona, pasa T01–T28 aplicables y equivalencia con intérprete/LLVM en sus destinos. Si ninguna solución cierra la puerta, F6 sigue pendiente; no se omiten comprobaciones para reducir latencia.

### F0. Núcleo contrastado y prototipo vertical desechable

**Entrada.** Acuerdo sobre P01–P09 y responsable de decisiones identificado.

**Entregables.** Gramática inicial; D01–D10; modelos de valores, recursos, préstamos, efectos, coherencia de traits, derivación, CoW y publicación; política de errores y números; interfaces de IR/backend; catálogo de primitivas confiadas. Plan de cuatro plataformas, biblioteca esencial y revisión del nombre. Protocolos de T01–T28, H-PY/H-RS/H-CS y T26 preparados y registrados antes de sus mediciones confirmatorias.

**Prototipo permitido desde el inicio.** Analizar, comprobar e interpretar una herramienta corta, una actualización de datos y un flujo de petición con E/S simulada. Puede emplear copias físicas y reducir traits a casos explícitos. Las simplificaciones se identifican. El código es desechable y no se convierte automáticamente en runtime de producción; permite descubrir problemas de sintaxis, propiedad y llamadas antes de congelarlos.

**Pruebas y salida.** Cada regla central dispone de ejemplo válido e inválido con respuesta esperada. Dos revisores resuelven el corpus sin discrepancias semánticas abiertas. El modelo reducido comprueba transferencias, aliasing, salidas de ámbito, coherencia y los estados de D09. No se exige una prueba formal completa del lenguaje. El piloto formativo de comprensión puede usar programas del prototipo; sus datos no se reutilizan como confirmación.

La fase sale con una versión semántica revisable y un listado de invariantes, no con una especificación inmóvil. Se fijan resultados esperados del tour y de los tests; cambios posteriores reabren únicamente las dependencias afectadas.

**Fuera de alcance.** Backend de producción, runtime concurrente completo, ecosistema y resultados competitivos de rendimiento.

**Riesgo y pivote.** Si traits, valores y recursos generan reglas incompatibles, se simplifica el núcleo. Si no se identifica ninguna tarea donde una mejora sería valiosa, se revisa el producto antes de ampliar implementación. Falta de revisión independiente bloquea la promoción, no el prototipado.

### F1. Núcleo ejecutable, genéricos y diagnósticos

**Entrada.** F0 aprobado y corpus semántico versionado.

**Entregables.** Analizador, tipos, IR e intérprete; valores, recursos, préstamos locales, `inout`, limpieza; funciones, módulos, patrones, `Option`/`Result`; traits y genéricos estáticos de D08. F1 implementa extensiones sin ambigüedad, operadores básicos y casos de coherencia entre paquetes locales. `comptime` puro y acotado; derivación `Format`/`PartialEq` y `Eq`/`Hash` donde sean válidas.

Texto UTF-8, interpolación, argumentos con nombre y por defecto, iteración síncrona; biblioteca inicial de consola, archivos, parsing y tests. Primer catálogo P09 con errores estructurados, ubicaciones relacionadas y códigos estables. Se proporciona una CLI mínima del intérprete; las herramientas unificadas se completan en F2.

**Pruebas y salida.** T01, T02, T07, T08, T17, T20, T21 y la parte inicial de T25. Al menos 100 casos positivos y 100 negativos con motivos diferenciados o variaciones justificadas; el recuento no reemplaza la cobertura. Se rechazan doble préstamo mutable, uso tras transferencia, escape de préstamo y anidación que oculte un recurso. Cada limpieza ocurre una vez en retornos y errores. Se prueban implementaciones de traits solapadas y derivaciones que intentan serializar o copiar un recurso sin contrato.

Cada diagnóstico del catálogo identifica la regla correcta y sus ubicaciones; un caso negativo no cuenta si falla antes por sintaxis accidental. Las sugerencias que se anuncien como aplicables se vuelven a comprobar; no añaden permisos ni copias sin indicarlo. La salida de `comptime` es reproducible con entradas iguales y falla claramente al agotar su presupuesto.

**Fuera de alcance.** Hilos, regiones de usuario, FFI general, layouts alternativos y generación de código nativo. El intérprete puede usar copias físicas sencillas para representar la semántica.

**Riesgo y pivote.** Ante inferencia global inestable, exigir firmas públicas y acotar el solucionador. Ante un contraejemplo de seguridad, reabrir D08 o el modelo de acceso y sus tests antes de continuar. No retrasar todos los diagnósticos a una fase de pulido.

### F2. Compilación, REPL y entrada temprana desde Python

**Entrada.** F1 aprobado y corpus ejecutable disponible.

**Entregables.** LLVM, modos de desarrollo/producción, servidor incremental, LSP, formato, tests, documentación y diagnósticos JSON. Comandos `run`, `check`, `build`, `repl`, `test`, `fmt`, `doc`, `lsp`; manifiesto y lockfile iniciales para paquetes locales. Adaptador experimental de backend rápido según D04, con presupuesto acotado y sin anunciar soporte completo.

CoW local según D09, con instrumentación y una configuración diagnóstica de conteo siempre atómico para comparar costes. Generadores síncronos propietarios, REPL y doctests de D10. Derivación JSON para escalares, structs y colecciones básicas. CLI con ayuda y argumentos, formato y logging básico a consola; no se espera a un registro público de paquetes para usarlos.

**Puente Python mínimo obligatorio.** `nexo build --python` produce un módulo local para las versiones y configuraciones de CPython fijadas en F0. Exportación explícita de funciones síncronas: booleanos, enteros de anchura declarada, flotantes, cadenas y búferes de lectura del conjunto soportado. Texto y datos se copian a valores propietarios por defecto; se permite préstamo sin copia de exportadores inmutables auditados. La dirección obligatoria es Python→Nexo. El perfil exacto, las conversiones y los rechazos se fijan en §6.6. El generador general de adaptadores, callbacks y Arrow no son prerrequisito.

**Pruebas y salida.** T09, T18 local, T19, T20–T23, T25 y T26. EX01 y EX04 completos. El corpus de F1 y al menos 10.000 programas pequeños generados dentro del subconjunto registrado producen observaciones compatibles en intérprete y nativo, optimizados y sin optimizar. El formateo es idempotente y conserva el árbol. REPL y archivo comparten tipos y resultados. La caché se invalida con cambios de traits, efectos, valores por defecto, metadatos y objetivo.

La exportación Python debe instalarse y llamarse sin modificar el compilador. Se prueban conversiones de rango, UTF-8, longitud cero, error Nexo adaptado a Python y liberación de búfer una sola vez. No hay vistas que escapen de la llamada ni suposiciones de exclusividad basadas solo en `readonly`. T26 registra ambos tamaños de proyecto, muestra el desglose y activa la contingencia cuando corresponda. La matriz de F2 en §4.2 es obligatoria.

**Fuera de alcance.** Backend rápido completo por defecto, hot reload avanzado, compatibilidad Python arbitraria, callbacks extranjeros, modificación in situ de arrays Python y gestor público de paquetes. No se exige que todas las rutas del puente sean cero copia.

**Riesgo y pivote.** Reducir formatos del puente antes que aceptarlos con aliasing no controlado; mantener una vía mínima útil en F2. Si LLVM domina el tiempo, activar la contingencia; si domina la inferencia o derivación, corregir ese componente. Fallos de semántica bloquean la fase, aunque la nueva ruta sea más rápida.

### F3. Control de memoria, vistas y derivación integrada

**Entrada.** F2 aprobado; procedencia, limpieza, efectos y metadatos representados en la IR.

**Entregables.** `prepare_unique()` y su alternativa sin reserva; regiones por ámbito, arenas generacionales y acceso comprobado a dos nodos; clonación de escenas; contratos transitivos y rayos X de costes. `Table<T, layout: AoS|SoA>`, vistas lógicas y SIMD admitido. Despacho dinámico restringido de D08 con efectos y almacenamiento explícitos.

Derivación `comptime` completa para el alcance 1.0: esquema JSON limitado, formato, comparación, límites, nombres de campo y configuraciones declaradas. El código derivado utiliza las mismas APIs de asignación y vistas que el resto de Nexo. Se incluyen una secuencia persistente, un mapa persistente y un historial con política de retención. La biblioteca publica costes de actualización, versión y liberación, incluidos casos desfavorables.

**Pruebas y salida.** T03, T04, T10–T14, T17–T18, T20, T22 y T25; regresiones previas en las cuatro plataformas. EX03 completo sobre ambos layouts. Los contratos detectan reservas en genéricos, operadores, derivaciones, temporales y destructores. Las regiones rechazan escapes anidados, cierres y errores con vistas. Modelos de generación/identidad de pocos bits prueban agotamiento seguro y retirada de posiciones.

La ruta de propietario único del observador ejecuta cero duplicaciones de búfer, comprobadas por IR e instrumentación. La preparación de unicidad no se valida solo sobre la raíz de una estructura anidada. La prueba de SIMD incluye vacíos, alineación, colas, números especiales y desbordamientos. Las operaciones enteras sin comprobación requieren prueba o módulo `unsafe`, nunca un cambio implícito de modo de compilación.

La salida incluye la batería básica de biblioteca y REPL/Python mínimo en las cuatro plataformas, de acuerdo con §4.2. Las rutas restringidas generadas por JSON y logging, cuando existan, tienen límites y efectos comprobados, no excepciones al sistema.

**Fuera de alcance.** Plazos duros, layout arbitrario, prueba automática de cualquier rango, reflexión dinámica universal y recolector de ciclos.

**Riesgo y pivote.** Si CoW dificulta cumplir costes, separar colecciones cómodas y búferes únicos bajo vistas comunes. Si SoA exige demasiadas excepciones, acotar tipos admitidos de forma explícita. Una derivación que consume demasiado tiempo se simplifica o cachea; no se permite saltar su comprobación.

### F4. Ecosistema, perfiles C, puente ampliado y Wasm

**Entrada.** F3 aprobado; contratos y representaciones listos para fronteras entre módulos.

**Entregables.** Perfiles C versionados, adaptadores inspeccionables, publicación de paquetes, lockfile con integridad y revocación. Perfiles de subconjuntos de SQLite y zlib, con versiones/ABI/configuraciones precisas y contratos basados en sus APIs. [R33] [R34]

Se amplía el puente mínimo: transferencia de búferes, lectura estable sin copia, acceso mutable exclusivo cuando pueda acreditarse y un subconjunto explícito de Arrow C Data Interface. Interoperabilidad en sentido Nexo→Python se evalúa como módulo con dependencia explícita del intérprete; no es requisito adoptar todo su modelo de objetos. [R35] [R36]

Wasm32 ejecuta el núcleo, texto, colecciones y pruebas sin sistema nativo. Se construye también **la ruta del compilador al navegador**: frontend, comprobador e intérprete del núcleo compilados a Wasm en un Worker, sin depender de llevar LLVM al navegador. El backend Wasm para programas y la distribución Wasm de herramientas son entregables distintos. Se prueba el prototipo de playground sin publicación obligatoria todavía. [R37]

Perfiles C y paquetes se prueban en las cuatro plataformas. JSON, CLI y logging adquieren su API candidata. HTTP utiliza primitivas de red y un parser probado desde esta fase; el servidor integrado termina en F5. El proceso de revisión de nombre y dominios empieza aquí y debe cerrar en F5.

**Pruebas y salida.** T05, T08, T09, T15, T19, T20, T24 y el prototipo de T27. Se demuestran una transferencia real sin copia y una lectura con inmutabilidad justificada; también copia o rechazo en casos incompatibles. Los tiempos incluyen adaptación y liberación. Cada símbolo de perfil tiene pruebas de retención, errores, callbacks, reentrada y afinidad aplicables.

El playground prototipo abre un archivo virtual, ejecuta EX04 y detiene un bucle mediante límite de trabajo/Worker; no recibe red ni archivos del anfitrión por defecto. Se comprueba que el código del usuario no pasa a un servidor externo sin acción explícita. Un resultado de Wasm no acredita ABI nativa ni concurrencia arm64.

**Fuera de alcance.** Importar con seguridad cualquier cabecera, C++ completo, Arrow completo, todos los exportadores Python, navegador con acceso directo al sistema y distribución universal de Python incluida.

**Riesgo y pivote.** Una API C demasiado compleja obtiene adaptador manual acotado. Si la ruta del compilador Wasm resulta demasiado grande, se reduce el subconjunto interactivo explícitamente y se mide la alternativa; no se anuncia «playground local» si ejecuta el código en un servicio remoto. Si ninguna ruta útil cumple T27, F6 queda pendiente.

### F5. Concurrencia, biblioteca esencial y aplicaciones completas

**Entrada.** F3–F4 aprobados; contratos de transferencia, publicación y retención versionados.

**Entregables.** Tareas estructuradas, actores, canales acotados, cancelación cooperativa y runtime. Se implementa D09 completo para publicación entre ejecutores, con el camino seguro de copia y el caso optimizado de búfer exclusivo. Las capturas expresan aislamiento y afinidad; migrar una tarea no evade el confinamiento de sus datos.

Biblioteca HTTP cliente/servidor, JSON, CLI, logging y tests con alcance §6.7. Regiones de petición/fotograma e instantáneas coherentes. Los grupos de tareas reúnen a sus dependientes antes de destruir región o recurso. El servidor usa límites de datos y tiempo, cancelación y cierre definidos. EX02 y las tres aplicaciones de validación funcionan con la biblioteca real, no con infraestructura privada que luego el usuario deba reconstruir.

**Investigación opcional de panic aislado.** Un experimento con tareas de dominio controlado puede evaluar limpieza y protección de invariantes sin FFI ni `unsafe` no auditados en su ejecución transitiva. Se registran casos de estado compartido parcialmente actualizado y callbacks, no solo frames de pila. La política de compilación estable sigue siendo aborto de proceso. No se continúa después de corrupción ni se entrega aislamiento a medias como propiedad de 1.0.

**Pruebas y salida.** T06, T10, T13, T16, T18 y T24; todas las plataformas. Cancelación inyectada en puntos enumerados, limpieza exactamente una vez, saturación de canales y cierre sin tareas huérfanas. Pruebas pequeñas de intercalaciones y pruebas de estrés con alcance registrado. Rechazo de contador local que cruza un ejecutor, alias anidados que invalidan unicidad y guardas mutables mantenidas en suspensiones incompatibles.

EX02 responde, rechaza entradas fuera de límite y cierra con peticiones en curso. Se comprueba memoria retenida y latencia de liberación bajo historial y RC. Las tres aplicaciones completan éxito, error, sobrecarga y cierre. Un error de coordinación sembrado se reproduce aunque no exista carrera de memoria. Se entrega informe de nombre antes de F6.

**Fuera de alcance.** Actores distribuidos, aislamiento general de fallos nativos, pausa temporal garantizada, planificador arbitrario y HTTP/2/3 o plataforma web completa.

**Riesgo y pivote.** Si publicar datos sin copia es demasiado complejo, conservar la copia explícita con costes medidos; no compartir contadores incorrectamente. Si el runtime se reduce, el alcance mínimo de HTTP y las tareas declaradas sigue sujeto a pruebas. Desactivar el experimento de panic no invalida el núcleo; no cerrar memoria concurrente sí bloquea promoción.

### F6. Ventaja confirmada, adopción y publicación 1.0

**Entrada.** F1–F5 aprobados; cuatro plataformas; revisión del nombre resuelta; APIs candidatas y protocolos confirmatorios registrados antes de sus datos.

**Entregables.** Compilador, REPL, herramientas, biblioteca esencial y documentación distribuibles en los cuatro anfitriones. Playground web público del subconjunto documentado; guías de migración Python/Rust/C#, ejemplos ejecutados y ruta de instalación sin árbol de desarrollo. Declaración de garantías, base de confianza, límites y dependencias. Revisión independiente de préstamos, regiones, D08/D09 y adaptadores.

**Pruebas y salida.** T01–T28 obligatorias según su alcance; estudios de §7; puertas de compilación; instalación desde distribuciones limpias y tres proyectos de personas externas sin parches privados. Reconstrucción reproducible en dos entornos limpios para cada cadena fijada, comparando artefactos no firmados cuando firma/notarización introduzca metadatos externos; éstos se registran aparte. No se exige identidad de binarios entre sistemas operativos distintos.

**Puerta de adopción:** la propuesta completa para las tres cohortes exige H-PY, H-RS y H-CS confirmadas conforme a §7, además de las puertas de seguridad y usabilidad. Si ninguna se cumple, se activa obligatoriamente «valor insuficiente». Si solo algunas se cumplen, no se anuncia el acuerdo completo como validado: puede mantenerse pre-1.0 o formularse un pivote público de público/alcance, con protocolo y confirmación nueva para ese alcance. Un dato inconcluso no cuenta como victoria.

Cero defectos conocidos abiertos que violen una garantía de seguridad anunciada o cambien semántica por optimización. La estabilidad 1.0 se aplica a sintaxis/APIs documentadas, no a una ABI interna universal de genéricos. El grupo secundario de asistentes es informativo; no puede salvar un fallo del grupo principal.

**Fuera de alcance.** Lo excluido en §11 y cualquier dependencia de F7.

**Riesgo y pivote.** Buenas pruebas sin ventaja de producto no bastan. Si los costes de mantenimiento multiplataforma o de biblioteca exceden la capacidad asignada, se conserva pre-1.0 o se cambia explícitamente la forma de producto. No se publica por «ganar en tres de cinco».

### F7. Extensiones tardías y opcionales

**Entrada.** Núcleo estabilizado, caso de uso y responsable; ninguna extensión reduce recursos necesarios para requisitos comprometidos.

**Entregables.** Experimentos separados para `@variant`, recetas, GPU, móvil, JVM/CLR, generación concurrente avanzada y simulación determinista. Otros backends se evalúan aquí cuando no formen parte de la contingencia ya activable en F2. Cualquier herramienta externa de IA sigue siendo opcional y no altera las reglas del lenguaje.

**Pruebas y salida.** Dominio, criterios y coste total registrados por extensión. Para variantes, separación de demostrado, probado, no resuelto y refutado. Android incluye JNI y ciclo de vida, no solo una exportación C. [R38] Ninguna extensión modifica silenciosamente comportamiento, efectos o evidencias del núcleo.

**Fuera de alcance.** Equivalencia universal, aceleración asegurada o garantías automáticas de aislamiento.

**Riesgo y pivote.** Una extensión que no aporta valor o rompe garantías se mantiene externa, experimental o se retira.

---

## 5. Batería transversal de pruebas y evidencia

### 5.1. Los seis casos mínimos del acuerdo

Se conserva la batería de la discusión anterior y se asignan identificadores para enlazarla con las fases.

| ID | Prueba | Resultado exigido | Fase de introducción |
|---|---|---|---|
| **T01** | Copiar datos anidados y modificar la copia. | El original conserva su contenido independiente; un campo compartido explícito no se confunde con una copia profunda. | F1 |
| **T02** | Intentar duplicar implícitamente un recurso. | Error claro, con posibilidad de préstamo o transferencia; ningún uso válido del propietario anterior tras consumirlo. | F1 |
| **T03** | Eliminar un nodo y reutilizar su posición. | El identificador antiguo no accede al nodo nuevo ni a un nodo de otro contenedor. | F3 |
| **T04** | Modificar almacenamiento posiblemente compartido dentro de `no_alloc`. | Rechazo, salvo que se justifique una ejecución sin reservas; no vale insertar una copia oculta. | F3 |
| **T05** | Entregar a C un puntero que conservará. | Exigir un recurso con duración y estabilidad suficientes; liberar antes de terminar la retención es inválido. | F4 |
| **T06** | Capturar el mismo estado mutable desde tareas incompatibles. | Rechazo o acceso mediante un mecanismo de aislamiento explícito; una captura indirecta no evade la regla. | F5 |

### 5.2. Pruebas adicionales obligatorias

| ID | Familia | Casos que debe incluir |
|---|---|---|
| T07 | Semántica básica y optimización | Orden de evaluación; enteros en los límites; datos vacíos; valores flotantes especiales; copias eliminadas sin cambiar observaciones. |
| T08 | Errores y limpieza | Retorno temprano con `?`; cierre explícito; inicialización parcial; destructor con efectos; política de `panic`; adaptación de errores extranjeros en F4. |
| T09 | Interfaces e invalidación | Cambio de efecto en dependencia; especialización genérica; caché antigua; cambio de perfil C o de configuración de destino. |
| T10 | Regiones | Escape directo, campo anidado, cierre, error que contiene una vista, callback y tarea; regiones anidadas; capacidad agotada. |
| T11 | Unicidad | Propietario único; dos copias vivas; datos anidados; intento de crear observador durante la vista exclusiva; comprobación sin reserva. |
| T12 | Vistas, disposición y SIMD | AoS/SoA; disjunción de proyecciones; alineación; colas de bucle; campos no admitidos; identidad de resultados con el contrato numérico. |
| T13 | Instantáneas | Deshacer/rehacer; ramas de historial; límite de versiones; retención de nodos; elementos compartidos; destrucción de la última versión. |
| T14 | Efectos transitivos | Reserva en cierre, destructor o genérico; llamada indirecta; bloqueo en limpieza; suspensión; función extranjera de efectos desconocidos. |
| T15 | Datos extranjeros | Búfer no contiguo, mutable pese a préstamo de lectura, retención, error de formato, liberación, cambios de versión y callbacks reentrantes. |
| T16 | Concurrencia | Publicación de datos; cancelación; saturación de canales; cierre; error de coordinación sin carrera; deuda de reclamación de memoria. |

### 5.3. Familias incorporadas en 0.2

T01–T16 siguen siendo obligatorias. Las nuevas familias se introducen en sus fases y se reejecutan cuando cambian compilador, runtime, dependencias o destino.

| ID | Familia y fase inicial | Casos obligatorios y resultado exigido |
|---|---|---|
| **T17** | Traits, coherencia y operadores — F1. | Implementación ausente, requisitos de copia no declarados, dos paquetes con impl solapada, tipo extranjero, alias que pretende eludir coherencia, extensión ambigua, operador que reserva y llamada dinámica con efectos excesivos. Rechazar por la regla pertinente; añadir o reordenar imports no cambia silenciosamente la resolución. |
| **T18** | CoW y RC local/atómico — F2/F5. | Contador local, siempre atómico y RC eliminado comparados; publicación con alias raíz o hijos; weak upgrade; callback; cambio de ejecutor; cancelación y último decremento. Ningún acceso concurrente mezcla protocolos; toda copia/publicación se contabiliza; unicidad entrega una garantía vigente. |
| **T19** | Puente mínimo Python — F2; ampliación F4. | Enteros fuera de rango, bool distinto de int, valores flotantes especiales, texto con surrogate no representable, búfer vacío/no contiguo, exportador mutable detrás de vista readonly, fallo al exportar y liberación tras error. Conversión definida, rechazo o copia segura; ningún puntero escapa por accidente. |
| **T20** | Derivación en `comptime` — F1–F3. | Tipos anidados, restricción ausente, recurso no serializable, campos privados, nombres JSON duplicados, campos omitidos sin reconstrucción, Eq impropia, metadato modificado y presupuesto agotado. Código generado sujeto a tipos/efectos y caché invalidada por metadatos. |
| **T21** | Texto y argumentos — F1. | UTF-8 inválido, bytes/grafemas distintos, slicing fuera de frontera, llaves escapadas, interpolación con efectos, orden de argumentos nombrados, duplicación, default mutable o con E/S y cambio de etiqueta pública. Semántica y diagnóstico estables. |
| **T22** | Iteradores y generadores — F1/F2. | Construcción sin ejecución del cuerpo, avance perezoso, salida temprana, generador sin empezar, error en consumidor, recurso capturado, escape de préstamo y préstamo inválido tras avance. Estado y limpieza una vez; no hay tarea oculta ni materialización obligatoria. |
| **T23** | REPL y doctests — F2. | Igualdad archivo/celda, redefinición con valores vivos, préstamo entre celdas, fallo de compilación, error con E/S previa, reset y compile_fail. No se afirma rollback de E/S; el negativo coincide con el código de error correcto. |
| **T24** | Biblioteca esencial — F2–F5. | JSON con tamaños/profundidad/rangos inválidos; CLI/help; logging con secretos configurados y destino fallido; HTTP mal formado, límites, tiempos, respuesta parcial y cierre. Las funciones documentadas se usan sin paquetes privados y sus efectos incluyen limpieza. |
| **T25** | Calidad de diagnósticos — F1–F3/F6. | Causa y ubicaciones correctas, cadena transitiva, campos anidados, código derivado, Unicode, sugerencias y paridad CLI/LSP/REPL. Cada reparación anunciada como aplicable vuelve a comprobarse y explica cambios de coste; comprensión evaluada en personas. |
| **T26** | Compilación y escala — F2/F6. | 10.000/100.000 líneas, cambios locales y públicos, instanciaciones, derivación y casos patológicos con límites. Medir desglose, p95, memoria e invalidación. Activar contingencia por los criterios de §4.3, no por preferencia del implementador. |
| **T27** | Playground y migración — F4/F6. | Arranque sin instalar, archivo virtual, Worker cancelable, límites de compilación/ejecución, cero red por defecto, ejemplos exportables y guías de tres cohortes. Las tareas públicas se completan sin intervención privada; el subconjunto excluido se identifica antes de ejecutar. |
| **T28** | Plataformas y distribución — F2–F6. | Construcción, ejecución, ABI, atomics, Python, REPL, biblioteca, empaquetado limpio y actualizaciones en cuatro plataformas; reconstrucción en entornos independientes. No sustituir ejecución arm64 por mera generación de código; registrar SDK y artefactos de firma por separado. |

EX01–EX04 son programas de aceptación que reutilizan estas familias: no sustituyen casos negativos ni permiten marcar una fase como aprobada porque funciona una demostración. Los tests de seguridad, formatos y red se ejecutan en entornos locales controlados.

### 5.4. Qué se conserva como evidencia

Cada ejecución publicará revisión del código, configuración, versiones, plataforma, entradas, semillas cuando existan y resultados íntegros. Los fallos nuevos se reducen a casos de regresión. Los programas negativos deben rechazarse por la regla pertinente: un error sintáctico accidental no demuestra que funciona el comprobador de propiedad.

Las comprobaciones dinámicas y los sanitizadores aportan evidencia sobre ejecuciones, no pruebas universales. Los efectos «verificados» necesitan una justificación en el análisis y un conjunto de primitivas con contratos revisados. La instrumentación comprueba que la implementación no contradice ese análisis en los casos ejecutados.

Los contratos se comprueban también después de introducir temporales, adaptadores y llamadas auxiliares del runtime. Toda operación añadida necesita un efecto conocido o una suposición identificada en la base de confianza. Las compilaciones instrumentadas se distinguen de las de producción: la instrumentación no puede alterar un presupuesto y conservar su misma etiqueta sin explicación.

Los contadores internos y la IR distinguen operaciones semánticas de detalles físicos. Una ausencia de llamadas a `malloc` no demuestra por sí sola ausencia de reservas regionales, temporales de otra biblioteca o reclamación costosa.

---

## 6. Diseño integrado: memoria, puente Python, biblioteca esencial y adopción

### 6.1. Contratos de recursos con significado preciso

La primera versión del análisis utilizará operaciones básicas y conjuntos de efectos transitivos. La inferencia local calcula un resumen; una interfaz pública puede declarar un límite que sus implementaciones deben respetar. Las llamadas indirectas necesitan un límite de efectos en su tipo. «Desconocido» no se tratará como conjunto vacío.

| Contrato o dato | Significado propuesto | No implica |
|---|---|---|
| `no_alloc` | No solicitar almacenamiento dinámico nuevo en montículo ni región durante la llamada y su limpieza. | Cero copias, cero pila, ausencia de liberaciones o tiempo acotado. |
| `no_heap_alloc` | No solicitar almacenamiento dinámico al asignador de montículo; puede utilizar capacidad regional bajo su propio contrato. | Que una región pueda crecer sin reservar al sistema. |
| `no_block` | No ejecutar operaciones clasificadas como potencialmente bloqueantes, incluidas las transitivas. | Ausencia de desalojo del hilo, fallos de página, espera por hardware o bucles muy largos. |
| `no_suspend` | No ceder la ejecución a través del mecanismo de suspensión del lenguaje. | Que el sistema operativo no pueda interrumpir el hilo. |
| `no_io` | No ejecutar entrada/salida externa clasificada, ni mediante una dependencia. | Pureza total o ausencia de modificación del estado recibido. |
| Firma de coste | Reservas, liberaciones, copias y cotas que pueda justificar el análisis, con su dominio. | Una fórmula exacta para cualquier función. |

La pila ordinaria de tamaño determinado por compilación no cuenta como reserva dinámica de `no_alloc`; su tamaño se informa aparte. La reserva de pila de tamaño arbitrario no se incorpora al código seguro de 1.0. La recursión puede consumir pila y no queda acotada por `no_alloc`.

`prepare_unique()` puede copiar antes de entrar en la ruta restringida y devuelve una vista exclusiva con ámbito. La alternativa sin reserva devuelve esa misma clase de garantía o un resultado de no disponibilidad: no basta devolver un booleano cuya validez pueda perderse antes de modificar. Mientras viva la vista, no se crean observadores incompatibles ni se invalida su almacenamiento. La unicidad de una colección exterior no acredita automáticamente la de sus elementos anidados.

Las cotas numéricas de memoria o iteraciones se admitirán inicialmente solo donde el análisis las justifique. Los presupuestos de tiempo medidos en integración continua llevan plataforma, carga y distribución observada, y nunca se etiquetan como plazos demostrados.

### 6.2. Regiones por ámbito: duración comprobada, coste declarado

El esquema propuesto hace visible el contexto de asignación:

```text
region peticion(capacity: capacidad) {
    let datos = parsear(cuerpo, using: peticion)?
    let salida = render(datos, using: peticion)?
    await responder_prestado(salida)?
}
```

El contrato de `responder_prestado` debe garantizar que, al completar su espera, el transporte ya no conserva la vista. Que una operación haya encolado la respuesta no basta. Alternativamente, la respuesta recibe un recurso propietario independiente de la región.

Una función solo puede reservar en la región si su interfaz y representación de resultados lo permiten. No se cambia el asignador de una biblioteca arbitraria por estar situada textualmente dentro del bloque. Las funciones reutilizables podrán parametrizar ese contexto; los detalles se inferirán cuando sean locales.

Copiar fuera exige independizar todas las partes que dependan del almacenamiento regional. Mover una vista o envolverla en otro objeto no prolonga la vida de sus bytes. Los valores escalares independientes pueden salir sin una operación especial de promoción. Las regiones anidadas respetan relaciones de duración y los errores no pueden transportar vistas ya inválidas.

La salida ejecuta limpieza de recursos y después recupera almacenamiento. Un reset de un bloque preasignado sin destructores puede ser muy sencillo; cerrar muchos archivos o devolver muchos bloques no lo es. Se medirán adquisición de capacidad, crecimiento, agotamiento y cierre. Las regiones de tareas solo se destruyen después de terminar o cancelar y reunir todas sus tareas dependientes.

### 6.3. Instantáneas e historial: una biblioteca con política de retención

La semántica de valores permite razonar sobre versiones independientes, pero no selecciona automáticamente una estructura persistente. Se ofrecerán tipos distintos para almacenamiento contiguo, búfer único y colecciones persistentes. Clojure es un precedente de colecciones inmutables que comparten estructura entre versiones. [R24]

```text
var historial = History<Documento>(max_versions: 100)
historial.push(documento)
documento = documento.con_parrafo_actualizado(indice, texto)
```

La API comprobará que el contenido admite instantáneas independientes. Copiar un handle de actor o un recurso no captura su estado. Una instantánea de un conjunto de objetos relacionados requiere una raíz coherente que abarque ese conjunto; tomar varias copias en momentos distintos no crea una transacción.

Se publicarán costes de conservar una raíz, copiar caminos o páginas, editar, descartar versiones y liberar la última referencia. El historial tendrá políticas de capacidad y contabilidad documentadas; contar cien raíces no equivale a cien copias completas, pero tampoco a consumo constante.

La destrucción de la última versión puede provocar muchas liberaciones. Trasladarlas a otra tarea es una política explícita con retención adicional y efectos propios, no una forma de borrar el coste. Ninguna colección persistente convierte automáticamente un bucle de edición en `no_alloc`.

### 6.4. Perfiles C compartidos: reutilizar una auditoría, no universalizarla

Un perfil declarativo debe registrar biblioteca, símbolos, versión, configuración de compilación, ABI, plataforma y evidencia de correspondencia con el artefacto utilizado. Para cada función indicará propiedad, retención, nulabilidad, límites de búferes, asignador de liberación, callbacks, reentrada, afinidad de hilo y efectos conocidos o desconocidos.

Los adaptadores generados serán inspeccionables y publicarán sus suposiciones. Una firma del paquete identifica procedencia e integridad; no prueba la verdad del contrato. Dos revisores podrán reutilizar la misma investigación y pruebas, pero una nueva versión o configuración exige revisar su aplicabilidad. Las bibliotecas cargadas dinámicamente necesitan una política explícita de compatibilidad, no solo confianza en el nombre del archivo.

El gestor de paquetes bloqueará versiones y perfiles y permitirá retirar revisiones defectuosas. Una retirada no reescribe silenciosamente binarios ya distribuidos: se emite diagnóstico y se exige una actualización comprobada. Los contratos ausentes mantienen la frontera como `unsafe` o impiden su uso bajo restricciones incompatibles.

Para Python y Arrow se separa **mantener vivo el almacenamiento** de **mantener estable su contenido**. Un préstamo de lectura no puede transformarse en un valor independiente si otro participante puede modificar el mismo búfer. Las interfaces oficiales de búferes y Arrow exigen además respetar descriptores y liberaciones. [R35] [R36]

### 6.5. Concurrencia y reclamación

D09 fija el protocolo: sin conteo cuando no hace falta, conteo local no atómico dentro de un dominio confinado y conteo atómico para almacenamiento publicado entre dominios. Una tarea migrable no se equipara a un hilo confinado. Publicar no permite cambiar el modo del contador mientras existan accesos locales activos; T18 cubre esa transición y sus costes.

La instantánea enviada a otra tarea conserva sus dependencias vivas. La transferencia de un recurso invalida el acceso anterior. Los IDs no propietarios permiten consultar una arena viva, pero no impedirán por sí solos que esta desaparezca: toda consulta necesita un propietario o préstamo válido del contenedor.

Clonar una escena viva crea un nuevo espacio de identificadores y reconstruye sus enlaces internos; cuando sea necesario, devuelve una correspondencia entre IDs originales y nuevos. Copiar un ID no copia el nodo. La identidad de un contenedor tampoco se reutiliza de una forma que pueda validar identificadores antiguos; el agotamiento de identificadores necesita una salida segura y comprobada.

El protocolo de cierre debe distinguir falta de carreras, terminación y ausencia de fugas. Ninguna de esas propiedades se deduce automáticamente de las otras.

---

### 6.6. Puente Python por etapas: adoptar una función, no reescribir todo

**F2, contrato mínimo.** Las exportaciones se declaran en Nexo y el generador produce un módulo de extensión CPython y sus firmas. La documentación de extensiones y el protocolo de búferes sirven de base para la frontera, no para atribuir a objetos extranjeros las garantías internas de Nexo. [R57] [R35]

| Entrada/salida | Política mínima F2 | Ampliación F4 |
|---|---|---|
| Booleanos y enteros | Tipos explícitos; rechazo de rango; bool no se convierte por sorpresa en cualquier entero. | Más anchuras y estructuras con esquema. |
| Flotantes | Conversión declarada y tratamiento de NaN/infinito/ceros de acuerdo con D07; sin tolerancias implícitas. | Vistas tipadas de arrays con contrato numérico. |
| Texto | Copia a `String` válido; error para texto extranjero no representable bajo el contrato UTF-8. | Préstamos de texto con vida limitada y fuente estable donde proceda. |
| Búfer de lectura | Préstamo de exportadores inmutables auditados o instantánea obtenida bajo una operación/contrato de estabilidad válido. El resto se rechaza o requiere adaptación explícita. | Cero copia para más formatos, retención y transferencia verificadas por el perfil. |
| Modificación compartida | No ofrecida en F2. | Vista mutable solo con exclusividad y duración suficientes; copia o rechazo en caso contrario. |
| Errores | `Result` se adapta a un error Python documentado; los errores Python durante adaptación se manejan antes de volver a Nexo. | Callbacks y propagación ampliada, sin atravesar ABI con excepciones. |

**Estabilidad de lectura.** Un `memoryview` readonly sobre almacenamiento mutable no crea una instantánea. Copiar bytes tampoco soluciona por sí solo una carrera con un escritor externo durante la copia. Para F2 se acota a exportadores conocidos: `bytes`, vistas sobre esos bytes y conversiones seguras bajo las reglas documentadas de la configuración CPython. Otros exportadores necesitan perfil o una instantánea proporcionada por una API que garantice estabilidad. No se asume que mantener el GIL coordina cualquier hilo nativo externo.

F0 elige al menos dos versiones menores de CPython para la distribución 1.0 y declara si sus variantes sin GIL y subintérpretes están soportados. El puente mínimo puede limitarse a la configuración con GIL e intérprete principal; otros modos se rechazan explícitamente hasta cerrar su prueba. La versión de Python no cambia el sistema de efectos de Nexo. El módulo nativo respeta afinidad y estado del intérprete y no conserva referencias Python sin un recurso propietario válido.

La primera experiencia de adopción debe ser: compilar un módulo, importarlo, llamar una función y diagnosticar una conversión incorrecta. T19 mide el recorrido completo. F4 añade Arrow y cero copia general acotado; su desarrollo no reabre la necesidad de esperar para probar Nexo desde Python.

### 6.7. Biblioteca estándar con baterías esenciales

**Compromiso 1.0:** una distribución oficial contiene HTTP cliente/servidor, JSON, CLI, logging y tests, además del núcleo de texto, colecciones, E/S, tiempo y tareas necesario para utilizarlos. Los módulos se enlazan bajo demanda: una herramienta numérica no incorpora un servidor HTTP si no lo usa.

«Incluido» significa disponible mediante `std` y probado con la versión del lenguaje; no que el equipo deba escribir desde cero cada parser, backend TLS o primitiva de sistema. Las dependencias reutilizadas se identifican, fijan y mantienen. Go documenta un módulo HTTP en su biblioteca estándar; es una referencia de experiencia, no una demostración de que todas las APIs deban copiarse. [R52]

| Módulo | Mínimo comprometido | Introducción y pruebas |
|---|---|---|
| `std.cli` | Argumentos, opciones, validación, ayuda, rutas y códigos de salida. | F1 mínimo; F2 API utilizable; casos inválidos y Unicode T21/T24. |
| `std.json` | Codificador/decodificador tipados, derivación, límites de tamaño/profundidad y errores de ruta de campo. | F2 básico; F3 completo acotado; T20/T24. |
| `std.log` | Niveles, campos estructurados, destinos consola/archivo, política de fallo y exclusión de campos sensibles. | F2 básico; F3 efectos y destinos de capacidad fija; F5 concurrencia. |
| `std.test` | Aserciones, fixtures, ejemplos, negativos de compilación, aislamiento y salida estructurada. | F1 runner; F2 doctests; F5 tests concurrentes con límites. |
| `std.http` | Cliente y servidor HTTP/1.1, routing pequeño, límites, tiempos de espera, streaming con backpressure y cierre ordenado. | Parser/primitivas en F4; integración y auditoría en F5; T24. |
| Núcleo auxiliar | Archivos, consola, bytes/texto, colecciones, clocks de sistema explícitos y tareas. | F1–F5 según efecto y plataforma. |

**JSON:** rechazo de claves duplicadas en la configuración segura inicial; rangos, profundidad y tamaño máximos explícitos. No se admiten NaN/infinito como números JSON sin un codec alternativo elegido. Los datos de entrada no generan tipos ni ejecutan código. `JsonDecode` para tipos con invariantes llama al constructor validado o falla la derivación.

**Logging:** una llamada de logging no se considera `no_alloc` ni `no_block` por su nombre. Las variantes de destino preasignado declaran qué sucede al agotarse la capacidad: error, descarte contado o reserva externa explícita. No se permiten pérdidas silenciosas anunciadas como «cero coste». Los secretos marcados no se imprimen mediante derivación de formato general sin autorización.

**HTTP:** gramática y delimitación de mensajes se contrastan con HTTP/1.1 y se prueban con entradas mal formadas y desacuerdos de tamaño. [R53] Los límites de cabecera, cuerpo, conexión y tiempo están presentes desde el primer servidor. El cliente HTTPS usa un proveedor TLS mantenido con validación de certificados y hostname por defecto; no se implementa criptografía propia para cerrar la fase. El servidor admite TLS mediante adaptador oficial cuando se configura identidad/certificado; la gestión automática de certificados y una plataforma completa de despliegue quedan fuera de 1.0. HTTP/2, HTTP/3, WebSockets, autenticación de aplicación y ORM no forman parte del mínimo.

Cada módulo tiene responsable, pruebas por plataforma, política de actualización y datos de compatibilidad. F6 no puede publicarse con estas baterías como simples enlaces a paquetes no mantenidos. Una auditoría o actualización de un componente reutilizado conserva su papel dentro de la base de confianza.

### 6.8. Adopción y playground sin instalación

**Camino de adopción:** explorar un ejemplo, modificarlo, exportar un proyecto reproducible y, cuando corresponda, introducir una función en un proyecto existente. No se mide adopción por visitas o descargas únicamente.

El playground de F6 utiliza el frontend y el intérprete compilados a Wasm dentro de un Worker del navegador. Ofrece edición, diagnóstico, ejecución del núcleo y archivos virtuales. El backend que compila programas Nexo a Wasm es otra ruta: disponer de ese backend no basta para ofrecer compilación local en la web. La misma semántica y corpus del núcleo se ejecutan en las dos rutas soportadas.

**Perímetro inicial:** sin sockets de servidor, FFI nativa, dependencias arbitrarias ni acceso al disco real por defecto. Los límites de pasos de `comptime`, pasos de ejecución, memoria y salida protegen la sesión; un Worker puede finalizarse y reiniciarse sin bloquear la página. La versión de herramientas se muestra, y cualquier operación de compartir contenido requiere una acción expresa. No se envía código a un backend de ejecución como sustitución oculta.

EX01 utiliza un archivo virtual; EX03 y EX04 se ejecutan localmente; EX02 se presenta con su restricción y se exporta como proyecto nativo. La documentación no simula haber abierto un puerto TCP real en el navegador. F4 valida el prototipo, F6 publica el servicio y prueba instalación/exportación/ejecución.

**Guías obligatorias:** «Nexo para Python», «Nexo para Rust» y «Nexo para C#». Cada una incluye valores frente a alias, errores, genéricos, biblioteca, pruebas, interop y un recorrido de migración verificable. No equivalen a traductores automáticos. La guía de C# trata recursos y estado compartido sin identificar todo objeto de referencia con un error; la de Rust muestra dónde desaparecen anotaciones y dónde sigue siendo necesario un contrato.

**Puerta propuesta de incorporación:** en un grupo nuevo de 12 personas —cuatro por cohorte, distinto del estudio principal—, al menos 10 ejecutan y modifican un ejemplo en menos de 10 minutos sin ayuda privada, y al menos 9 exportan e instalan un proyecto en un máximo de 20 minutos. Se registran descargas, conexión, versión de navegador, plataforma y todos los fallos; la muestra prueba ese recorrido, no estima por sí sola adopción de mercado. El tutorial es autocontenido y no requiere una cuenta para ejecutar localmente.

### 6.9. Diagnóstico de costes como interacción diaria

Este resultado propuesto pertenece al catálogo de P09:

```text
NX-E0412: no se puede verificar el contrato no_alloc
  actualizar → normalizar → crear_temporal

Causa:
  normalizar reserva un búfer cuyo tamaño depende de la entrada.

Origen:
  normalizar.nx:18 — Buffer.new(datos.len)

Alternativa comprobable:
  recibir espacio temporal preparado por el llamador.

Consecuencia:
  cambia la interfaz; la preparación sigue teniendo coste fuera de esta llamada.
```

El diagnóstico incorpora ubicaciones estructuradas, versión de la dependencia y origen del efecto; el texto no afirma haber encontrado una corrección universal. En una derivación señala también el campo y la regla generada. El editor distingue operación observada, estimada, supuesta y desconocida, y permite navegar al origen.

T25 comprueba que el mensaje corresponde a la IR y que el código JSON del diagnóstico coincide en CLI, LSP y REPL. La usabilidad no se resuelve contando mensajes: H-CS y las tareas de §7 miden si ayudan a localizar y reparar el problema.

---

## 7. Validación con programadores de Python, Rust y C#

### 7.1. Preguntas y ventajas registradas antes del estudio

Se conserva la evaluación de comprensión, corrección y mantenimiento de 0.1. Se añaden hipótesis de **ventaja**, porque alcanzar una no inferioridad aceptable no justifica por sí solo introducir otro lenguaje.

La especificación del estudio identifica antes de recoger datos confirmatorios: tareas, participantes, variantes, comparadores, bibliotecas, entrenamiento, tiempo máximo, magnitud útil, variables primarias, tratamiento de fallos, incertidumbre y regla de decisión. Los cambios descubiertos en el piloto se registran antes de cerrar el protocolo. No se selecciona retrospectivamente la cohorte o el benchmark favorable.

| Hipótesis primaria | Ventaja útil propuesta | Condiciones que deben acompañarla |
|---|---|---|
| **H-PY — Python** | En tres tareas registradas de lógica local de CPU, cociente geométrico de tiempos Nexo/Python ≤ 1/3, con límite superior del intervalo de confianza ≤ 1/2. | Solución correcta y tiempo de desarrollo Nexo no superior a 1,10 veces el habitual con evidencia de no inferioridad; adaptación y memoria incluidas. No proclamar aceleración de espera de red ni de bibliotecas nativas ya optimizadas. |
| **H-RS — Rust** | En cambios de representación y acceso sobre las mismas tareas, tiempo hasta solución correcta Nexo/Rust ≤ 0,80 y límite superior de incertidumbre < 1. | Objetivo adicional registrado de episodios de reparación por acceso/propiedad ≤ 0,70 del comparador, sin más fallos funcionales ni copias que incumplan presupuesto. Se exige mantener un solo cuerpo lógico. |
| **H-CS — C#** | Localizar y corregir una regresión de reservas introducida en una dependencia en ≤ 0,75 del tiempo de C#, con límite superior < 1. | La ruta completa registrada satisface su contrato de memoria y las regresiones sembradas se detectan. Latencia p99 del procesamiento completo ≤ 1,10 veces la referencia C# configurada de forma competente; se incluyen preparación, publicación y limpieza. |

Las magnitudes son decisiones propuestas, no resultados. H-PY exige una estimación de mejora de tres veces y evidencia compatible con al menos dos veces para esas cargas, no una promesa de tres veces en cualquier programa. H-RS no confunde un rechazo correcto del compilador con un fallo de seguridad: se cuentan episodios de reparación del programador y su tiempo. H-CS busca previsibilidad y localización útil, no ganar comparando con una aplicación deliberadamente mal ajustada.

**Referencia C# justa.** Se permiten `Span`, pooling, reutilización y opciones de latencia apropiadas a la tarea. .NET documenta modos de latencia y un mecanismo de región sin recolección sujeto a condiciones; no se presenta la ausencia de GC global de Nexo como una ventaja empírica suficiente. [R50] [R51] Nexo puede sufrir liberaciones en cascada, asignación o esperas: también se miden. La ausencia de eventos de un recolector no acredita un plazo temporal ni p99 inferior.

**Regla F6.** La propuesta completa para las tres cohortes necesita las tres hipótesis confirmadas y las puertas mínimas. Si ninguna confirma una ventaja, se activa «valor insuficiente» aunque todo el compilador pase sus tests. Si una o dos confirman, existe evidencia para un público más estrecho, no para anunciar toda la propuesta como validada. Un pivote requiere alcance y criterios nuevos, con confirmación nueva; no se rebautiza la comparación fallida como éxito.

### 7.2. Participantes, preparación y diseño

Se conserva el piloto formativo de **9 participantes** —tres por cohorte— y el plan confirmatorio inicial de **36 participantes nuevos**, doce con experiencia principal en Python, doce en Rust y doce en C#. No se afirma que doce por grupo basten para todos los efectos propuestos. Tras el piloto y antes del registro confirmatorio se estima la precisión/potencia requerida y se fija el tamaño final y presupuesto; una ampliación posterior solo puede seguir una regla secuencial previamente registrada o un estudio nuevo.

La experiencia se comprueba con una tarea breve en el lenguaje habitual. Se imparten 90 minutos de introducción a Nexo, una hoja de referencia y práctica diferente de los ejercicios evaluados. No se exige aprender sin manual. Desarrolladores del compilador y participantes del piloto no forman parte de la muestra confirmatoria.

Cada persona resuelve variantes equivalentes en Nexo y en su lenguaje habitual. Las seis variantes se reparten en tres sesiones con pausas y límite de 45 minutos por variante. Orden y asignación se contrabalancean. Los comparadores idiomáticos y esqueletos son revisados por personas competentes en cada lenguaje. Las librerías y herramientas se igualan por capacidad, no necesariamente por número de líneas de código entregado; toda asimetría se publica.

El grupo principal utiliza editor, compilador y documentación, sin asistentes de generación de código. La ayuda humana sigue un guion y queda registrada. Las grabaciones o trazas se obtienen con consentimiento, se minimizan y se publican anonimizadas cuando corresponda; ningún ejercicio exige credenciales reales o datos privados.

### 7.3. Proyectos, tareas y mantenimiento

| Proyecto | Versión sencilla | Cambio solicitado | Observaciones |
|---|---|---|---|
| CLI | Leer registros, validar, agrupar y emitir un informe. | Reutilizar temporales, añadir un error y exportar un núcleo a Python. | Corrección, esfuerzo, llamadas de puente, reservas y tiempo completo. |
| Servidor | Atender peticiones sobre un estado. | Región de petición, cancelación, límites e instantánea de lectura. | Vida de datos, cierre, coordinación, memoria y colas de latencia. |
| Simulación | Actualizar partículas y un grafo pequeño. | AoS→SoA, IDs seguros y bucle `no_alloc`. | Conservación de la lógica, errores de acceso, publicación y costes. |

Los esqueletos evitan confundir «construir una red completa» con comprender propiedad. Las tareas se prueban por separado y luego se modifica una aplicación integrada. Una necesidad de cambiar interfaces se cuenta como trabajo, aunque el algoritmo permanezca intacto.

Para H-PY se registran tres cargas de CPU local con lógica personalizada, no tres variaciones de un mismo microbucle. Se incluyen entradas pequeñas y grandes y una referencia Python idiomática. Cuando exista una implementación vectorizada o de biblioteca apropiada, se incorpora como comparador adicional; no se oculta que pueda igualar o superar a Nexo. La superioridad se limita a los comparadores efectivamente ensayados.

Para H-RS, los errores se codifican por episodio causal: un problema de acceso que produce veinte mensajes es un episodio, no veinte. Las tareas que comparan reparación por préstamos deben producir oportunidades reales en el piloto; una línea base sin ningún episodio no se fuerza a un cociente artificial. Esa definición se cierra antes del estudio.

Para H-CS, el cambio sembrado introduce una reserva en una dependencia, una conversión o una limpieza; ambas herramientas tienen acceso a sus mecanismos normales de observación. Se comprueban causa localizada, corrección y ausencia de desplazamiento oculto del coste fuera del intervalo. Los presupuestos de la aplicación se fijan antes de ver datos.

### 7.4. Puertas mínimas de usabilidad preservadas

| Métrica | Umbral propuesto antes de medir | Interpretación |
|---|---|---|
| Completar correctamente cada familia | Al menos 10 de 12 por cohorte inicial; si el tamaño cambia antes del registro, proporción mínima 5/6 con redondeo hacia arriba. | Puerta de la muestra; no afirmación universal. |
| Predecir copia, modificación y transferencia | ≥80% correcto por cohorte; ninguna regla central <70%. | Modelo mental compatible con el diseño. |
| Localizar copia o reserva transitiva | Al menos 10 de 12 por cohorte inicial, o proporción 5/6 si cambia el tamaño. | Utilidad mínima de costes visibles. |
| Tiempo de mantenimiento general | Mediana Nexo ≤1,25 veces el lenguaje habitual, con análisis de incertidumbre registrado. | No inferioridad mínima; no sustituye H-PY/H-RS/H-CS. |
| Conservar el algoritmo al cambiar almacenamiento | Un cuerpo lógico en cada aplicación y registro de toda duplicación. | El pegamento adicional también se cuenta. |

Las tareas no completadas son fallos y tiempos censurados, no observaciones que se eliminan del cálculo para mejorar el promedio. Los errores de sintaxis, tipos, préstamo, recurso, región, efectos, FFI y coordinación se codifican por separado. Otra persona revisa una muestra de la clasificación. Se compara confianza declarada con corrección real.

Si las herramientas cambian después de observar fallos confirmatorios, la nueva versión se confirma con una muestra independiente. Volver a entrenar a las mismas personas no proporciona evidencia independiente de la mejora.

### 7.5. Análisis, incertidumbre y decisiones

Se registran métodos adecuados para tiempos pareados y censurados, resultados binarios y episodios de reparación. La unidad de repetición humana es la persona, no cada mensaje del compilador. Cien ejecuciones del mismo programa no se tratan como cien programadores. Las repeticiones de rendimiento caracterizan variación de máquina; no amplían por sí solas el dominio de programas cubierto.

Se publican estimaciones, distribuciones e intervalos por cohorte. Los contrastes/intervalos primarios se ajustan por la familia de tres hipótesis de superioridad mediante el procedimiento fijado en el registro; los intervalos de la tabla son los ajustados, no tres oportunidades independientes de declarar victoria por azar. Los criterios conjuntos de cada hipótesis —por ejemplo, velocidad con esfuerzo comparable— deben satisfacerse a la vez.

Un estudio se clasifica como **confirmado**, **refutado en el dominio probado** o **inconcluso**. No alcanzar significación no demuestra equivalencia; un intervalo ancho tampoco demuestra superioridad. Las medidas secundarias se identifican como exploratorias y no reemplazan una primaria desfavorable. La decisión de F6 conserva seguridad como requisito independiente.

El tamaño y presupuesto se revisan usando el piloto, no resultados intermedios para seguir reclutando hasta conseguir un valor favorable. Si el presupuesto no permite la precisión prevista, se reporta la limitación y se mantiene pendiente la afirmación correspondiente.

### 7.6. Rendimiento, costes y programas completos

Se conservan las referencias idiomáticas y de bajo nivel de 0.1, elegidas antes de medir. Algoritmo, dominio numérico, errores, límites y trabajo útil comparables. No se compara una variante comprobada con otra que omite validaciones y se atribuye después toda la diferencia al lenguaje.

Se registran compilador, flags, CPU, RAM, sistema, bibliotecas, afinidad, calentamiento, tamaños y variación; orden alternado y datos brutos publicados. El tiempo total incluye adaptadores, conversión, unicidad, publicación entre tareas, reclamación y cierre. La memoria incluye retención por versiones y buffers, no solo asignación dentro del núcleo optimizado.

**Metas preservadas:** en núcleos CPU, mediana Nexo/referencia de bajo nivel ≤1,5 y ningún caso obligatorio >2 salvo restricción aceptada antes de confirmación. En aplicaciones completas, tiempo y memoria máxima ≤1,5 de la referencia elegida. Las condiciones de superioridad de §7.1 se añaden a estas puertas, no se deducen de ellas. Las cargas de Python no sustituyen comparadores de bajo nivel cuando se afirme rendimiento de sistemas.

**Matriz D09:** conteo local, siempre atómico de diagnóstico, RC eliminado por unicidad y publicación entre ejecutores. Medir incrementos/decrementos, atomics, bytes duplicados, latencia de publicación, memoria máxima y destrucción. La comparación mantiene la misma semántica; una ruta de publicación por copia no se presenta como si fuese gratuita. El resultado negativo local o concurrente se conserva.

**Enteros:** comparar comportamiento comprobado con comprobaciones eliminadas por prueba en entradas del mismo dominio. Las variantes modulares/saturadas se prueban funcionalmente, pero no sustituyen al comparador estricto para declarar equivalencia. Se incluyen sobrecarga y vectorización de código derivado.

El mantenimiento se mide tras cambiar el comportamiento, no solo tras escribirlo una vez: interfaces modificadas, ubicaciones del algoritmo, tiempo, errores y reconstrucción. Reducir líneas es secundario si el resultado es menos comprensible.

### 7.7. Estudio secundario con asistentes, separado del diseño

Se añade una muestra exploratoria propuesta de **18 personas nuevas**, seis por cohorte, con asistencia de código. Se fijan antes modelo/versión, herramientas, documentación de Nexo suministrada, acceso a red y presupuesto de interacción. No se utilizan soluciones del estudio principal ni una configuración diferente para cada lenguaje sin declararlo.

Se miden tiempo hasta programa correcto, errores introducidos/aceptados, uso de diagnósticos y verificación por la persona. El resultado se publica separado: evalúa el flujo de herramientas disponible, no una propiedad intrínseca de la sintaxis. No puede compensar una hipótesis principal fallida ni justificar añadir IA al compilador. Si no puede ejecutarse por disponibilidad de herramientas, su estado se declara y no se inventan conclusiones de adopción asistida; la decisión de valor del lenguaje sigue en el estudio principal.

---

## 8. Riesgos y criterios de abandono o pivote

«Abandonar» se refiere a una característica, una promesa o la forma de producto evaluada, no a concluir que investigar el problema carezca de sentido. Cada revisión fijará presupuesto de trabajo y alcance antes de empezar. Si se agota sin cerrar la evidencia, el estado es inconcluso y se decide explícitamente la continuación.

| Fase | Riesgo dominante | Señal verificable | Acción antes de continuar |
|---|---|---|---|
| F0 | Núcleo contradictorio, coherencia de traits insuficiente o proceso paralizante. | Revisores discrepan; el prototipo vertical no resuelve casos centrales; no existe tarea discriminante. | Iterar con el prototipo, simplificar D08/D09 o pivotar a herramientas. Revisar para promover, no para permitir empezar a explorar. |
| F1 | Inferencia frágil o acceso inseguro. | Un programa admitido viola exclusividad o duración; cambios locales alteran contratos de forma inexplicable. | Bloquear promoción, corregir reglas y exigir firmas donde la inferencia no sea adecuada. |
| F2 | Compilación lenta, REPL incoherente o puente Python que no permite adopción incremental. | Fallan puertas de 10.000/100.000 líneas, diagnósticos o round-trip; memoria extranjera queda retenida. | Corregir el componente medido; activar la contingencia preparada de backend cuando corresponda; limitar tipos del puente sin aplazarlo entero a F4. |
| F3 | Copias, limpieza o layouts destruyen la previsibilidad. | Contratos aceptados ejecutan operaciones prohibidas; muestras no explican el coste real. | Hacer preparación explícita, reducir layouts y rechazar contratos no demostrables. |
| F4 | Perfiles demasiado confiados o mantenimiento inasumible. | Una biblioteca cambia retención/efectos sin invalidar adaptadores; cero copia depende de aliasing no controlado. | Invalidar perfil, copiar o limitar acceso; adaptadores manuales y versiones acotadas. |
| F5 | Cancelación, publicación de RC, HTTP o plataformas no cierran. | Tareas huérfanas; contador con accesos atómicos y no atómicos concurrentes; límite de petición evadido; fallo exclusivo de ARM. | Bloquear promoción; corregir confinamiento/ABI/runtime. Mantener experimental el aislamiento de panic. No retirar un destino obligatorio sin revisar explícitamente el alcance. |
| F6 | Valor insuficiente pese a un lenguaje técnicamente correcto. | No hay superioridad confirmada, o falla una cohorte de la propuesta completa; instalar o migrar depende de ayuda privada. | No aprobar por mera no inferioridad. Mantener pre-1.0, registrar un alcance más estrecho y confirmarlo con datos nuevos, o publicar piezas útiles por separado. |
| F7 | Complejidad experimental contamina el núcleo. | Una extensión exige romper garantías estables o no mejora su caso de uso. | Mantenerla externa, experimental o retirarla. |

Riesgos transversales: crecimiento de binarios por especialización; retención excesiva por historial; recolección y liberación en momentos inesperados; dependencias del compilador; cobertura insuficiente de plataformas; seguridad del gestor de paquetes; discrepancia entre garantías publicadas y su base de confianza. Cada uno necesita responsable y datos, no solo una advertencia en la documentación. La ampliación a cuatro plataformas, HTTP y tooling tiene presupuesto propio: se asigna responsable de mantenimiento por componente y destino en F0. La falta de equipo o hardware bloquea esa puerta; no se compensa marcando tests como omitidos. Se registran asimismo conflictos de marca, dependencia del proveedor TLS y crecimiento de código derivado.

Un resultado positivo en rendimiento no compensa un fallo de seguridad. Un resultado positivo de usabilidad tampoco convierte una suposición sobre código C en un hecho demostrado.

---

## 9. Comparativa honesta con Swift, Hylo, Rust, Zig y Mojo

### 9.1. Qué existe y qué tendría que demostrar Nexo

La comparación conserva los antecedentes de 0.1 y contrasta las incorporaciones de 0.2 con documentación primaria el 19 de octubre de 2026. Describe capacidades documentadas; no es un ensayo ejecutado de todos los compiladores ni un inventario exhaustivo de sus bibliotecas.

| Lenguaje | Capacidades relevantes que ya documenta | Qué debe demostrar Nexo frente a él |
|---|---|---|
| **Swift** | Valores y CoW, exclusividad, protocolos, tipos no copiables, concurrencia con aislamiento y vistas no escapables como `Span`. [R02] [R07] [R08] [R39] [R40] [R56] | Que regiones, preparación de unicidad, layouts de colecciones y costes transitivos formen una experiencia más eficaz para las tareas elegidas. No basta con añadir `inout` o valores. |
| **Hylo** | Semántica de valores mutables, genéricos y proyecciones como ejes de diseño. Su introducción pública señala que sigue en desarrollo. [R01] | Que la ampliación a recursos, regiones, FFI y herramientas no destruya la sencillez que motiva ese núcleo. Es uno de los comparadores semánticos más cercanos, no una influencia marginal. |
| **Rust** | Propiedad, préstamos, recursos, traits coherentes, derivación, `Rc`/`Arc` y hilos con ámbito. [R05] [R09] [R41] [R43] [R44] [R45] | Reducir decisiones y errores de uso en tareas de valores, vistas y presupuestos sin debilitar seguridad. No afirmar que Rust carece de arenas seguras o exige anotar todas las duraciones. |
| **Zig** | Asignadores explícitos, `comptime`, interoperabilidad C y control de representación. Su documentación describe la responsabilidad sobre memoria y duración. [R12] | Añadir comprobación integrada de préstamos/regiones y herramientas de costes conservando una parte relevante de ese control. Debe medirse el coste en complejidad y compilación. |
| **Mojo** | Modelo de propiedad, semántica de valores, acceso mutable exclusivo, traits, metaprogramación y puentes Python/C. [R25] [R26] [R58] | Demostrar diferencias concretas en contratos, experiencia de edición, bibliotecas y mantenimiento general. «Sintaxis sencilla más control nativo» no es por sí solo una diferenciación nueva. |

No se utiliza como argumento el estado de propiedad empresarial de un competidor, su popularidad supuesta o afirmaciones antiguas sobre quién acepta contribuciones. La licencia de Nexo es una decisión del proyecto, no una prueba de superioridad técnica.

### 9.2. La integración que se propone, sin inventar exclusividad

La hipótesis distintiva es ofrecer en una misma experiencia: valores independientes por defecto y recursos explícitos; regiones con escapes comprobados; algoritmos sobre vistas con AoS/SoA; costes transitivos visibles y restringibles; e interoperabilidad mediante perfiles versionados cuyo grado de confianza sea visible.

**No está demostrado que ningún otro lenguaje o conjunto de herramientas pueda integrar esa combinación.** La ausencia de una característica en una página consultada no demuestra que no exista en su ecosistema. Tampoco la integración, por sí sola, constituye novedad científica.

Para validar una aportación específica se construirán las mismas tareas en los cinco comparadores cuando su disponibilidad y alcance lo permitan. Se registrará qué proporciona el lenguaje, qué requiere una biblioteca y qué debe mantener manualmente la aplicación. Una alternativa no ejecutable se marcará como no evaluada, no como derrota.

La afirmación publicable deberá ser concreta: por ejemplo, que en determinadas tareas Nexo evita duplicación de algoritmos y reduce intervenciones manuales para controlar recursos, con resultados y límites. Solo después de esa comparación tendría sentido afirmar una ventaja de integración.

---

## 10. Extensiones tardías y opcionales

### 10.1. `@variant` y recetas de optimización

Las variantes relacionarán una referencia con una implementación dentro de un dominio y contrato explícitos. Distinguirán **demostrado en el dominio**, **probado mediante tests**, **no resuelto** y **refutado**. Solo la evidencia apropiada autorizará una sustitución que prometa equivalencia; el respaldo debe respetar también los presupuestos del llamador.

Cambiar referencia, variante, dependencias, semántica numérica o condiciones relevantes invalidará la evidencia. Mutación, aliasing, errores y terminación forman parte del contrato; no basta comparar el vector de salida. La aproximación numérica es una autorización distinta de la equivalencia exacta.

Se priorizarán recetas que deriven variantes del mismo algoritmo y reduzcan mantenimiento. Halide y Exo sirven como antecedentes, no como prueba de que ese mecanismo ya esté resuelto para todos los programas. [R27] [R28]

### 10.2. Otros experimentos

GPU, backends adicionales fuera de la contingencia de D04/F2, reflexión adicional, recarga en caliente y compilación Android/JVM tendrán propuestas separadas. El aislamiento de panic por tarea se investiga opcionalmente en F5 y solo se amplía aquí tras su evaluación, sin cambiar la política estable de 1.0. El puente Android deberá tratar JNI, callbacks y duración de objetos además de la ABI nativa. [R38]

La simulación concurrente determinista solo prometerá reproducción dentro del entorno que controle: planificación, reloj, aleatoriedad y operaciones externas modeladas. Una semilla no reproduce por sí sola cualquier ejecución externa.

Cualquier uso de IA será una herramienta externa y opcional. No podrá modificar unilateralmente las reglas semánticas, los criterios de aceptación o la evidencia exigida. Ninguna prueba de valor de Nexo 1.0 dependerá de ella.

---

## 11. Alcance comprometido y exclusiones de la versión 1.0

### 11.1. Qué significa publicar 1.0 bajo la revisión 0.2

**Plataformas obligatorias:** Linux x86-64, Windows x86-64, macOS arm64 y Linux arm64 como anfitriones de herramientas y destinos nativos, con versiones mínimas, SDK y pruebas reales documentadas. El subconjunto Wasm del núcleo y su playground se distribuyen con su perímetro propio; no se equiparan a toda la biblioteca nativa.

**Lenguaje:** valores, recursos y compartición explícita; préstamos locales e interfaces de duración; tipos algebraicos, errores tipados, traits coherentes, genéricos y operadores; inferencia, texto UTF-8, argumentos nombrados/defaults, iteradores/generadores síncronos y derivación `comptime` restringida.

**Control:** regiones por ámbito, arenas generacionales, unicidad, colecciones AoS/SoA sobre vistas, SIMD admitido, contratos transitivos, CoW local y publicación atómica segura. Colecciones persistentes e historial con políticas de retención. No hay un recolector global obligatorio; la reclamación conserva sus costes y reglas.

**Herramientas:** intérprete, LLVM, servidor incremental, REPL, pruebas/doctests, formateador, LSP, diagnósticos estructurados y rayos X de costes. Contingencia de backend rápido preparada y promovida solo cuando cierre sus pruebas; no es una excusa para incumplir las puertas de edición.

**Baterías esenciales:** HTTP/1.1 cliente/servidor con TLS mediante proveedor configurado según §6.7; JSON tipado y derivado; CLI; logging; tests; y los módulos básicos de sistema, texto, colecciones y tareas que necesitan. Distribución modular y mantenimiento oficial, con dependencias explícitas.

**Integración y adopción:** ABI C delimitada, perfiles compartidos y auditables, puente Python mínimo y ampliado acotado, subconjunto Arrow, paquetes con integridad e invalidación. Cuatro ejemplos completos, playground, tres guías de migración, instalación y proyectos externos sin parches privados. Nombre público decidido antes de F6; licencia Apache 2.0 y proceso de contribuciones documentado.

**Evidencia:** T01–T28, protocolos registrados, muestras e incertidumbre, puertas de compilación y superioridad por cohorte. Cada afirmación publicada distingue propiedad del lenguaje, suposición externa, medición y alcance. Todas las fases siguen pendientes en este documento: publicar un roadmap no ejecuta sus pruebas.

### 11.2. Qué queda expresamente fuera

| Área | Fuera de 1.0 |
|---|---|
| Compatibilidad de lenguajes | Ejecutar sin cambios cualquier programa Python, C++, Java, C# o JavaScript; sustituir todos sus frameworks. |
| Sistema de tipos | Herencia de clases, resolución estructural universal, especialización semántica solapada, sobrecarga por retorno y tipos de orden superior arbitrarios. |
| Reflexión | Reflexión dinámica universal, macros textuales, ejecución con E/S en `comptime` y generación que eluda tipos/efectos. La derivación tipada sí está incluida. |
| Memoria | Recolector global obligatorio, recolección universal de ciclos, promoción local→atómica con alias incompatibles, copias gratuitas y ausencia universal de fugas. |
| Tiempo | Plazos duros, cotas para cualquier bucle, liberación instantánea y ausencia garantizada de pausas del sistema o del runtime. |
| Datos | Layout arbitrario transparente, conversión gratuita, Arrow completo e instantáneas automáticas de recursos externos. |
| Interoperabilidad | Adaptadores seguros solo a partir de cabeceras, todo C++/Python, exclusividad deducida de readonly y cero copia universal. |
| Concurrencia | Ausencia de errores de coordinación, actores distribuidos, detención inmediata de cualquier tarea, generadores async avanzados y recuperación general de corrupción. El aislamiento de panic de F5 no es garantía estable. |
| Biblioteca | ORM completo, plataforma gráfica, navegador, HTTP/2/3, WebSockets, gestión automática de certificados y framework de autenticación. No excluye las baterías esenciales de §6.7. |
| Plataformas | Frameworks Android/iOS, JVM/CLR completos, GPU, kernels de sistema operativo, todas las arquitecturas embebidas y todos los pares de cross-compilation. |
| Compilador | Backend propio desde cero, dos backends nativos completos obligatorios desde F2 y hot reload universal de objetos vivos. La contingencia rápida ya no se pospone obligatoriamente a F7. |
| Evidencia | Equivalencia de programas arbitrarios, demostración completa de toda la cadena de compilación o seguridad incondicional de una biblioteca externa. |
| Extensiones | `@variant` como requisito, optimización dependiente de IA o servicios remotos obligatorios para programar. |

### 11.3. Criterio final de identidad y adopción

Nexo conserva su propósito si una persona puede empezar con una solución comprensible y controlar después su almacenamiento y ejecución sin desechar una parte sustancial de la lógica. Para publicar la propuesta completa también debe demostrarse una ventaja útil en las tareas y cohortes registradas: seguridad y no inferioridad, por sí solas, no bastan.

**Los datos se copian, los recursos se transfieren y el estado compartido se declara. Las optimizaciones no cambian el comportamiento observable. La ventaja para el programador se demuestra en programas y modificaciones reales.**

---

## 12. Fuentes primarias

Las referencias respaldan antecedentes y características de otros sistemas, no validan una implementación de Nexo. Las decisiones y umbrales del documento son propuestas propias. Las páginas de documentación continua pueden cambiar; al ejecutar F0 deberán registrarse versiones o revisiones concretas para las comparaciones.

| Referencia | Fuente | Alcance utilizado |
|---|---|---|
| [R01] | Hylo — Introduction to Hylo | Valores mutables, genéricos y estado del proyecto. |
| [R02] | Swift — Memory Safety | Exclusividad y accesos inout. |
| [R03] | Clang — Function Effect Analysis | Restricciones transitivas de reservas y bloqueo. |
| [R04] | LLVM — Remarks | Informes de optimización y relación con el código fuente. |
| [R05] | Rust — The Rustonomicon: FFI | Fronteras extranjeras y adaptadores seguros. |
| [R06] | Python — The Python Tutorial | Sintaxis, estructuras y experiencia de uso. |
| [R07] | Swift Evolution — SE-0390: Noncopyable structs and enums | Recursos no copiables y convenciones de propiedad. |
| [R08] | Swift — Concurrency | Tareas, actores y transferencia entre dominios de concurrencia. |
| [R09] | Rust — What Is Ownership? | Propiedad, transferencia y duración. |
| [R10] | ISO/IEC JTC1/SC22/WG14 — C | Referencia al proceso y documentos del lenguaje C. |
| [R11] | C++ — Borrador de trabajo, Templates | Plantillas, parámetros y restricciones. |
| [R12] | Zig — Language Reference, master | Comptime, memoria, representación e interoperabilidad C. |
| [R13] | Koka — The Koka Programming Language | Inferencia y composición de efectos. |
| [R14] | Microsoft — F#: Type Inference | Inferencia estática. |
| [R15] | Kotlin — Null Safety | Tratamiento de tipos anulables. |
| [R16] | Go — Effective Go | Formateo, terminadores y fundamentos de concurrencia; no inventario de todas las novedades. |
| [R17] | Erlang — Processes | Procesos y comunicación. |
| [R18] | Microsoft — Roslyn Workspace Model | Modelo reutilizable por herramientas. |
| [R19] | Oracle — JavaCompiler, Java SE 25 | API de compilador y diagnósticos. |
| [R20] | TC39 — ECMAScript, Control Abstraction Objects | Promesas y mecanismos asíncronos; especificación continua. |
| [R21] | TypeScript — The Basics | Análisis estático y relación con JavaScript. |
| [R22] | Julia — Functions | Funciones y operaciones vectoriales. |
| [R23] | PostgreSQL — Using EXPLAIN | Planes, estimaciones y mediciones de consultas. |
| [R24] | Clojure — Data Structures | Colecciones persistentes y compartición estructural. |
| [R25] | Mojo — Manual | Áreas documentadas del lenguaje y sus herramientas. |
| [R26] | Mojo — Ownership | Convenciones de propiedad y exclusividad. |
| [R27] | Halide — Proyecto oficial | Separación entre algoritmo y organización de la ejecución. |
| [R28] | Exo — Proyecto oficial | Optimización dirigida y comprobación de transformaciones. |
| [R29] | Reproducible Builds — Definitions | Entradas y condiciones de reproducibilidad. |
| [R30] | Cranelift — Proyecto oficial | Backend con objetivos propios de compilación. |
| [R31] | generational-arena — Documentación de la biblioteca | Identificadores generacionales y reutilización de posiciones. |
| [R32] | Apache Software Foundation — Apache License 2.0 | Texto de la licencia propuesta. |
| [R33] | SQLite — List of SQLite Functions | Punto de entrada a contratos de su API C. |
| [R34] | zlib — Manual | API, asignación y ciclo de vida de sus operaciones. |
| [R35] | Python — Buffer Protocol | Formato, acceso y liberación de búferes. |
| [R36] | Apache Arrow — C Data Interface | Representación y propiedad de datos compartidos. |
| [R37] | WebAssembly — Core Specification: Introduction | Máquina y relación con el entorno anfitrión. |
| [R38] | Android Developers — JNI Tips | Integración nativa/JVM, referencias y callbacks. |
| [R39] | Apple — Improve Memory Usage and Performance with Swift, WWDC25 | Span, tipos no escapables y control de costes. |
| [R40] | Swift — Optimization Tips | Copia diferida y posibles costes de optimización. |
| [R41] | Rust — std::thread::scope | Hilos con ámbito y préstamos de datos. |
| [R42] | Microsoft — F#: Discriminated Unions | Alternativas tipadas, Option y patrones. |

[R01]: https://hylo-lang.org/introduction/ "Hylo — Introduction to Hylo"
[R02]: https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/MemorySafety.md "Swift — Memory Safety"
[R03]: https://clang.llvm.org/docs/FunctionEffectAnalysis.html "Clang — Function Effect Analysis"
[R04]: https://llvm.org/docs/Remarks.html "LLVM — Remarks"
[R05]: https://doc.rust-lang.org/nomicon/ffi.html "Rust — The Rustonomicon: FFI"
[R06]: https://docs.python.org/3/tutorial/ "Python — The Python Tutorial"
[R07]: https://raw.githubusercontent.com/swiftlang/swift-evolution/main/proposals/0390-noncopyable-structs-and-enums.md "Swift Evolution — SE-0390: Noncopyable structs and enums"
[R08]: https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/Concurrency.md "Swift — Concurrency"
[R09]: https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html "Rust — What Is Ownership?"
[R10]: https://www.open-std.org/jtc1/sc22/wg14/ "ISO/IEC JTC1/SC22/WG14 — C"
[R11]: https://eel.is/c%2B%2Bdraft/temp "C++ — Borrador de trabajo, Templates"
[R12]: https://ziglang.org/documentation/master/ "Zig — Language Reference, master"
[R13]: https://koka-lang.github.io/koka/doc/book.html "Koka — The Koka Programming Language"
[R14]: https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/type-inference "Microsoft — F#: Type Inference"
[R15]: https://kotlinlang.org/docs/null-safety.html "Kotlin — Null Safety"
[R16]: https://go.dev/doc/effective_go "Go — Effective Go"
[R17]: https://www.erlang.org/doc/system/ref_man_processes.html "Erlang — Processes"
[R18]: https://learn.microsoft.com/en-us/dotnet/csharp/roslyn-sdk/work-with-workspace "Microsoft — Roslyn Workspace Model"
[R19]: https://docs.oracle.com/en/java/javase/25/docs/api/java.compiler/javax/tools/JavaCompiler.html "Oracle — JavaCompiler, Java SE 25"
[R20]: https://tc39.es/ecma262/multipage/control-abstraction-objects.html#sec-promise-objects "TC39 — ECMAScript, Control Abstraction Objects"
[R21]: https://www.typescriptlang.org/docs/handbook/2/basic-types.html "TypeScript — The Basics"
[R22]: https://docs.julialang.org/en/v1/manual/functions/ "Julia — Functions"
[R23]: https://www.postgresql.org/docs/current/using-explain.html "PostgreSQL — Using EXPLAIN"
[R24]: https://clojure.org/reference/data_structures "Clojure — Data Structures"
[R25]: https://mojolang.org/docs/manual/ "Mojo — Manual"
[R26]: https://mojolang.org/docs/manual/values/ownership/ "Mojo — Ownership"
[R27]: https://halide-lang.org/ "Halide — Proyecto oficial"
[R28]: https://exo-lang.dev/ "Exo — Proyecto oficial"
[R29]: https://reproducible-builds.org/docs/definition/ "Reproducible Builds — Definitions"
[R30]: https://cranelift.dev/ "Cranelift — Proyecto oficial"
[R31]: https://docs.rs/generational-arena/latest/generational_arena/ "generational-arena — Documentación de la biblioteca"
[R32]: https://www.apache.org/licenses/LICENSE-2.0 "Apache Software Foundation — Apache License 2.0"
[R33]: https://www.sqlite.org/c3ref/funclist.html "SQLite — List of SQLite Functions"
[R34]: https://www.zlib.net/manual.html "zlib — Manual"
[R35]: https://docs.python.org/3/c-api/buffer.html "Python — Buffer Protocol"
[R36]: https://arrow.apache.org/docs/format/CDataInterface.html "Apache Arrow — C Data Interface"
[R37]: https://webassembly.github.io/spec/core/intro/introduction.html "WebAssembly — Core Specification: Introduction"
[R38]: https://developer.android.com/ndk/guides/jni-tips "Android Developers — JNI Tips"
[R39]: https://developer.apple.com/videos/play/wwdc2025/312/ "Apple — Improve Memory Usage and Performance with Swift, WWDC25"
[R40]: https://raw.githubusercontent.com/swiftlang/swift/main/docs/OptimizationTips.rst "Swift — Optimization Tips"
[R41]: https://doc.rust-lang.org/std/thread/fn.scope.html "Rust — std::thread::scope"
[R42]: https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/discriminated-unions "Microsoft — F#: Discriminated Unions"


### Referencias incorporadas o contrastadas para 0.2

Las siguientes fuentes sustentan los antecedentes añadidos; las decisiones de D08–D10, los perfiles de biblioteca y las hipótesis son propuestas de Nexo, no propiedades atribuidas a esos proyectos. Consulta de revisión: 19 de octubre de 2026. Para las ejecuciones confirmatorias se congelarán revisiones de documentación y herramientas.

| Referencia | Fuente primaria | Alcance |
|---|---|---|
| [R43] | The Rust Reference — Implementations | Coherencia, solapamiento y reglas de implementaciones. |
| [R44] | Rust std — Arc | Diferencia Rc/Arc, contenido compartido y referencias débiles. |
| [R45] | The Rust Reference — Derive | Derivación de implementaciones a partir de un tipo. |
| [R46] | Kotlin — Functions | Argumentos por defecto y con nombre. |
| [R47] | Python — doctest | Ejecución de ejemplos documentados. |
| [R48] | Python Language Reference — Expressions | Generadores y expresiones yield. |
| [R49] | Rust Compiler Development Guide — Errors and lints | Ingeniería de diagnósticos, ubicaciones y sugerencias. |
| [R50] | Microsoft — Latency Modes | Opciones de latencia de GC y sus límites. |
| [R51] | Microsoft — GC.TryStartNoGCRegion | API condicionada de región sin recolección. |
| [R52] | Go — net/http | Biblioteca oficial HTTP cliente/servidor. |
| [R53] | IETF/RFC Editor — RFC 9112 | Mensajes y conexiones HTTP/1.1. |
| [R54] | Unicode — UAX #29 | Segmentación y grafemas. |
| [R55] | USPTO — Search our trademark database | Herramienta de búsqueda de marcas; no autorización del nombre. |
| [R56] | The Swift Programming Language — Protocols | Requisitos y extensiones de protocolos. |
| [R57] | CPython — Defining extension modules | Frontera y ciclo de vida de módulos de extensión. |
| [R58] | Mojo — Traits | Precedente de traits en el comparador, no característica exclusiva de Nexo. |

[R43]: https://doc.rust-lang.org/reference/items/implementations.html "Rust — Implementations"
[R44]: https://doc.rust-lang.org/std/sync/struct.Arc.html "Rust — Arc"
[R45]: https://doc.rust-lang.org/reference/attributes/derive.html "Rust — Derive"
[R46]: https://kotlinlang.org/docs/functions.html "Kotlin — Functions"
[R47]: https://docs.python.org/3/library/doctest.html "Python — doctest"
[R48]: https://docs.python.org/3/reference/expressions.html#yield-expressions "Python — Yield expressions"
[R49]: https://rustc-dev-guide.rust-lang.org/diagnostics.html "Rust — Errors and lints"
[R50]: https://learn.microsoft.com/en-us/dotnet/standard/garbage-collection/latency "Microsoft — Latency Modes"
[R51]: https://learn.microsoft.com/en-us/dotnet/api/system.gc.trystartnogcregion "Microsoft — GC.TryStartNoGCRegion"
[R52]: https://pkg.go.dev/net/http "Go — net/http"
[R53]: https://www.rfc-editor.org/rfc/rfc9112 "RFC 9112 — HTTP/1.1"
[R54]: https://www.unicode.org/reports/tr29/ "Unicode — Text Segmentation"
[R55]: https://www.uspto.gov/trademarks/search "USPTO — Trademark search"
[R56]: https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/Protocols.md "Swift — Protocols"
[R57]: https://docs.python.org/3/c-api/extension-modules.html "CPython — Defining extension modules"
[R58]: https://mojolang.org/docs/manual/traits/ "Mojo — Traits"

---

## 13. Matriz de trazabilidad de la revisión 0.2

Esta matriz localiza decisiones y pruebas. Las fases anteriores contienen los entregables y puertas: la matriz no sustituye su integración.

| Petición | Decisión/principio | Fases responsables | Evidencia de cierre |
|---|---|---|---|
| A01. Cuatro plataformas | §4.2; §11 | F0 plan; F2 arranque; F3 núcleo; F4–F6 integración. | T28, ejecución nativa y distribución completa en cada destino. |
| A02. Genéricos e interfaces | D08 | F0 reglas; F1 estáticos; F3 dinámicos/vistas. | T17, dependencias incompatibles y diagnóstico correcto. |
| A03. Python en F2 | §6.6 | F2 mínimo; F3 cuatro plataformas; F4 ampliado. | T19 y módulo importable sin parche privado. |
| A04. Ventaja por cohorte | §7.1 y §7.5 | F0 protocolo; F6 confirmación. | H-PY, H-RS, H-CS y resolución explícita de valor insuficiente. |
| A05. RC local/atómico | D09 | F0 modelo; F2 local; F3 unicidad; F5 publicación. | T18, invariantes y comparación de coste total. |
| A06. Derivación | D03/D08 | F1 formato/comparación; F2 JSON; F3 alcance completo. | T20 y efectos transitivos del código generado. |
| B07. Programas completos | §0 | F2 EX01/04; F3 EX03; F5 EX02. | Archivos completos, entradas/salidas y ejecución de aceptación. |
| B08. Ergonomía | D01/D10 | F1 texto/argumentos/iteradores; F2 generadores/REPL/doctests. | T21–T23. |
| B09. Baterías esenciales | §6.7 | F1–F5 por módulo. | T24 y tres aplicaciones sobre APIs oficiales. |
| B10. Playground y guías | §6.8 | F4 prototipo; F6 publicación. | T27 y estudio nuevo de incorporación. |
| B11. Diagnósticos | P09; §6.9 | F1–F3; F6 personas. | T25, causa pertinente y reparación medible. |
| B12. Prototipo vertical | F0 | F0; reabrir reglas cuando proceda. | Tres programas reducidos identificados; revisiones previas a promoción. |
| B13. Compilación 100k | D04; §4.3 | F0 interfaz; F2 medida/contingencia; F6 puerta. | T26, desglose y corpus real/sintético distinguidos. |
| C14. Panic aislado | D02 | F5 experimental, eventual F7. | Informe separado; no modifica garantía estable. |
| C15. Enteros | D07 | F2 medida; F3 integración; F6 aplicación. | T07, prueba de rango y comparadores semánticamente compatibles. |
| C16. Grupo asistido | §7.7 | F6 estudio secundario. | Datos separados, modelo/configuración registrados; no sustituye el principal. |
| C17. Nombre | D06 | F0 alternativas; F4–F5 revisión; F6 nombre resuelto. | Informe de búsqueda y decisión documentada, no solo dominio disponible. |

**Estado global al emitir 0.2:** diseño revisado, no ejecución aprobada. Ninguna cifra de rendimiento, usabilidad, seguridad del compilador o disponibilidad de marca se presenta como resultado obtenido.
