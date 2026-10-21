Lo diseñaría alrededor de una identidad: nombres visualmente claros, expresiones compactas y operaciones importantes explícitas. Tomaría las mayúsculas de C#, la ligereza de las funciones modernas, la composición funcional y el control de recursos de los lenguajes de sistemas, pero los sometería a unas mismas reglas.

Hay una distinción importante: reunir características buenas no produce automáticamente un lenguaje mejor. Algunas se contradicen. El trabajo consiste en elegir qué combinaciones conservar, qué simplificar y qué rechazar.

Para Nexo, trataría lo siguiente como una propuesta de sintaxis para evaluar, no como una modificación de sus decisiones ya acordadas. Los ejemplos describen un lenguaje propuesto; no son código que un compilador existente acepte hoy. Los nombres de biblioteca también son ilustrativos.

# 1. La apariencia general que buscaría

Comenzaría con algo así:

fn SumarCuadradosPares(numeros: view List<Int>) -> Int {
    return numeros.Iter()
        |> Filter(n => n % 2 == 0)
        |> Map(n => n * n)
        |> Sum()
}

fn Main() {
    let resultado = SumarCuadradosPares([1, 2, 3, 4, 5, 6])

    Console.WriteLine("Resultado: {resultado}")
}
El resultado sería 56.

Aquí ya aparecen varias decisiones:

**SumarCuadradosPares, List, Filter y Console.WriteLine** aportan la presencia visual que te gusta de C#. Las variables locales permanecen en minúscula inicial: numeros, resultado, n.

**fn, let, view y return** forman el esqueleto del programa. No compiten visualmente con los nombres importantes.

**-> indica el tipo producido**, mientras que **|> indica hacia dónde pasan los datos**. No les asignaría significados intercambiables.

Y conservaría return en los bloques: la última línea de una función no devolvería un valor accidentalmente.

La identidad no estaría en inventar símbolos extravagantcada elemento tenga un trabajo reconocible y estableible y estable**.

---

# 2. Mayúsculas: sí, pero con una jerarquía

Tomaría de C# la convención de usar PascalCase para tipos, métodos y miembros públicos. Microsoft distingue expresamente estas convenciones de las reglas que exige el compilador. Mantendría esa separación. [Microsoft Learn](https://learn.microsoft.com/en-us/dotnet/csharp/fundamentals/coding-style/identifier-names)

Mi convención sería:

| Elemento | Estilo | Ejemplo |
|---|---|---|
| Tipos y contratos | PascalCase | Jugador, Renderizable |
| Funciones y métodos declarados | PascalCase | CalcularDistancia, Guardar |
| Campos y propiedades del modelo | PascalCase | Nombre, VelocidadMaxima |
| Variantes y constantes | PascalCase | Estado.Activo, MaxIntentos |
| Variables y parámetros | camelCase | jugadorActual, deltaTime |
| Módulos | minúsculas con puntos | juego.fisica |

Por ejemplo:

let jugadorActual = Jugador(
    Nombre: "Ana",
    Puntos: 150,
)

let distanciaTotal = CalcularDistancia(origen, destino)

Console.WriteLine(jugadorActual.Nombre)
La djugadorActual*jugadorActual** y **Jugador** comunica instancia frente a tipo. distanciaTotale **distanciaTotal** y **CalcularDistancia** comunica dato frente a operación.

No introduciría prefijos obligatorios como I, C, m_ o str. Preferiría:

Renderizable
Conexion
Nombre
En lugar de:

IRenderizable
CConexion
strNombre
**Las mayúsculas orientarían al lector, pero no cambiarían la semántica.** El compilador no debería decidir que algo es un tipo únicamente porque comienza con mayúscula.

---

# 3. Pocas reglas de puntuación y un formato oficial

Usaría llaves para delimitar bloques, sin exigir paréntesis alrededor de condiciones:

if temperatura > limite {
    ActivarVentilador()
}
Los puntos y coma no serían necesarios al final de una línea. Sí servirían para separar expresamente dos instrucciones en la misma línea, aunque el formateador normalmente las separaría.

let ancho = 800
let alto = 600
Las llamadas y colecciones multilínea admitirían coma final:CrearVentana(
    titulo: "Editor",
    ancho: 1280,
    alto: 720,
)
Pero esto necesita reglas léxicas precisas: no basta con decir «los puntos y coma son opcionales». Por ejemplo, una línea que continúa con |> o . seguiría perteneciendo a la expresión anterior; un return no tomaría silenciosamente una expresión de la línea siguiente.

También habría un formateador oficial desde el principio. Tomaría como referencia la decisión de Go de proporcionar una herramienta estándar para formatear automáticamente el código. [Go](https://go.dev/blog/gofmt)

No ofrecería simultáneamente una variante con llaves y otra con indentación significativa dentro del mismo código fuente.

La variedad de estilos no sería una característica del lenguaje.

---

# 4. Variables: distinguir estabilidad, mutación y constantes

Usaría tres declaraciones:

let nombre = "Ana"
var puntos = 100
const MaxIntentos = 3

puntos += 25
**`let** declara un valor que no puedes modificar a través de esa variable.

**var** permite modificarlo.

**const** exige un valor conocido durante la compilación.

Eso evita que const signifique unas veces «no reasignable» y otras «evaluado al compilar».

Para datos ordinarios, propondría:

``text
let numeros = [1, 2, 3]

// Rechazado: numeros no ofrece acceso mutable.
numeros.Add(4)

Cuando se quiera mutación:

text
var numeros = [1, 2, 3]

Más adelante explicaré cómo marcaría las operaciones que modifican esos datos.

Hay una excepción importante que debe quedar visible: **un contenedor de estado compartido no se convierte en profundamente inmutable por guardarlo en un `let`**. Tipos como `Shared<Mutex<Estado>>` tendrían reglas propias y explícitas; no escondería esa diferencia.

---

# 5. Tipos: inferencia local, contratos visibles

La inferencia serviría para evitar repetir lo obvio:

text
let intentos = 3
let activo = true
let nombre = "Ana"

Pero permitiría declarar el tipo cuando aporta información:

text
let identificador: U64 = 42
let velocidad: F32 = 2.5
let temperatura: F64 = 21.75
let datos: List<U8> = []

Mi selección inicial incluiría:

text
Bool
Int
I8, I16, I32, I64
U8, U16, U32, U64
USize
F32, F64
String
Unit
Never

Fijaría `Int` como un entero de tamaño definido —por ejemplo, `I64`— y reservaría `USize` para tamaños dependientes de la plataforma.

**No dejaría que el mismo nombre cambiara silenciosamente de anchura según el destino.**

Las conversiones que puedan perder información serían explícitas:

text
let amplio: I64 = 500
let pequeno = try U8.From(amplio)

Aquí `U8.From` devolvería un resultado con error por estar fuera de rango.

También distinguiría alias de tipos nuevos:

text
type Usuarios = List<Usuario>

newtype UsuarioId = U64
newtype ProductoId = U64

El primero solo abrevia un nombre. Los otros crean identidades diferentes:

text
fn BuscarUsuario(id: UsuarioId) -> Usuario?

Un `ProductoId` no sería aceptado por accidente.

**Elegiría esto porque una buena sintaxis también debe permitir expresar diferencias del dominio, no solamente tipos de máquina.**

---

# 6. Funciones: una forma completa y una abreviatura inequívoca

La forma completa sería:

text
fn Sumar(a: Int, b: Int) -> Int {
    return a + b
}

Para una sola expresión:

text
fn Sumar(a: Int, b: Int) -> Int => a + b

La regla sería muy concreta:

> **`=> expresión` equivale a un cuerpo que devuelve esa expresión. Un bloque entre llaves necesita `return` para devolver un valor.**

Así no hay retorno implícito por posición.

Una función sin resultado útil podría omitir `-> Unit`:

text
fn MostrarMensaje(mensaje: view String) {
    Console.WriteLine(mensaje)
}
`

### Argumentos nombrados y valores predeterminados

Tomaría de Kotlin la utilidad de los argumentos nombrados y los valores por defecto, sin copiar necesariamente su puntuación. Ambas características están documentadas como parte de sus funciones. [Kotlin](https://kotlinlang.org/docs/functions.html)fn Escalar(valor: F64, factor: F64 = 1.0) -> F64 {
    return valor * factor
}

let original = Escalar(12.5)
let duplicado = Escalar(12.5, factor: 2.0)
Mi regla sería que los argumentos opcionales, cuando se proporcionen, se escriban por nombre. Tras el primer argumento nombrado, los siguientes también serían nombrados.

Los argumentos se evaluarían de izquierda a derecha, en el orden escrito.

Y los nombres de parámetros utilizados públicamente serían parte del contrato de la API: renombrarlos podría romper llamadas.

Menos sobrecargas, llamadas más legibles y ninguna ambigüedad sobre qué representa un booleano o un número.

---

# 7. Condiciones y bucles: palabras para la lógica, símbolos para las matemáticas

Usaría:

and
or
not
Para operaciones booleanas:

if usuario.Activo and not usuario.Bloqueado {
    PermitirAcceso()
}
Reservaría &, | y ^ para operaciones sobre bits.

Las condiciones tendrían que ser Bool: no aceptaría que una lista vacía, un número o una cadena se interpreten automáticamente como verdadero o falso.

### Condiciones que producen valores

Adoptaría una forma explícita:

let etiqueta = if activo then "Activo" else "Pausado"
Scala ofrece un precedente de if como expresión y del uso de then. Aquí lo combinaría con bloques de instrucciones delimitados por llaves, sin introducir retornos implícitos dentro de ellos. [Scala Documentation](https://docs.scala-lang.org/scala3/book/control-structures.html)

La separación sería:

// Selección de un valor.
let descuento = if esMiembro then 10 else 0

// Ejecución de instrucciones.
if esMiembro {
    MostrarBeneficios()
    RegistrarAcceso()
}
No añadiría además el operador ternario ?:.

### Bucles y rangos

for numero in numeros {
    Console.WriteLine(numero)
}

for indice in 0..<10 {
    Console.WriteLine(indice)
}

while intentos < MaxIntentos {
    intentos += 1
}
Definiría dos rangos:

0..<10    // Incluye 0, excluye 10.
1..=10    // Incluye ambos extremos.
No ofrecería un tercer .. cuyo extremo haya que memorizar.

break y continue conservarían sus funciones habituales. No añadiría variantes sintácticas para ahorrar una sola palabra.

---

# 8. Datos y objetos: un modelo principal, no cinco formas equivalentes

Usaría struct como construcción principal para agrupar datos y comportamiento:

struct Rectangulo {
    Ancho: F64
    Alto: F64

    prop Area: F64 => self.Ancho * self.Alto

    fn Escalado(factor: F64) -> Rectangulo {
        return Rectangulo(
            Ancho: self.Ancho * factor,
            Alto: self.Alto * factor,
        )
    }
}
Uso:

let panel = Rectangulo(Ancho: 20.0, Alto: 10.0)
let ampliado = panel.Escalado(2.0)

Console.WriteLine(panel.Area)
Console.WriteLine(ampliado.Area)
Aquí elegiría varias cosas deliberadamente.

Los campos serían inmutables por defecto. Un campo modificable llevaría var.

**self sería explícito al acceder a miembros desde el interior.** Evita confundir parámetros con campos.

**prop sería una propiedad calculada**, no almacenamiento oculto. Restringiría sus accesores a lectura sin E/S ni mutación externa. Una operación costosa o con efectos debería presentarse como método.

**Los métodos leerían self por defecto.** Los métodos modificadores usarían un modificador explícito, explicado en la sección de memoria.

No introduciría inicialmente class, record, data class y struct con diferencias pequeñas y parcialmente superpuestas.

### Copiar modificando algunos campos

let nuevoPanel = copy panel with {
    Ancho: 30.0,
}
El `copy` importa: indica que se está creando otro valor, no modificando el original.

La operación solo existiría cuando los campos puedan copiarse conforme a sus La concisión no debería esconder una copia potencialmente costosa.e costosa.**

---

# 9. Estados y patrones: modelar posibilidades, no combinaciones accidentales

Usaría enumeraciones con datos asociados:enum EstadoTrabajo {
    EnCola
    Ejecutando(Progreso: Int)
    Terminado
    Fallido(Codigo: Int)
}
Y un match que pueda producir un valor:

let descripcion = match estado {
    EstadoTrabajo.EnCola => "Esperando"
    EstadoTrabajo.Ejecutando(progreso) => "Progreso: {progreso}%"
    EstadoTrabajo.Terminado => "Completado"
    EstadoTrabajo.Fallido(codigo) => "Error {codigo}"
}
Tomaría de Rust la capacidad de combinar variantes, desestructuración y patrones, pero exigiría una presentación uniforme. Su documentación muestra cómo los patrones pueden reconocer y extraer partes de valores. [Documentación de Rust](https://doc.rust-lang.org/book/ch19-03-pattern-syntax.html)

El compilador exigiría cubrir todas las variantes.

Esto sería preferible a representar un trabajo así:

EstaEjecutando: Bool
EstaTerminado: Bool
TieneError: Bool
Porque esa representación permite combinaciones contradictorias.

Para enumeraciones propias, favorecería escribir todos los casos en vez de cerrar siempre con _: así, añadir una variante nueva obliga a revisar los lugares pertinentes.

También reutilizaría los patrones para desestructurar:

let (ancho, alto) = ObtenerResolucion()
Una sola idea —reconocer la forma de un valor— serviría en distintos lugares.

---

# 10. Ausencia de valores: opcionales, no un null universal

Un tipo normal no admitiría ausencia:

let puntos: Int = 10
Un tipo opcional sí:

let puntos: Int? = Some(10)
let sinPuntos: Int? = none
`T?` sería una abreviatura de Option<T>.

Para dar un valor alternativo:

let puntosFinales = puntos ?? 0
Para acceder a través de un valor opcional:

let puntuacion = jugador?.Puntos ?? 0
Y para tratar ambos casos:

let mensaje = match puntos {
    Some(valor) => "Puntos: {valor}"
    none => "Sin puntuación"
}
No introduciría un operador corto para «confía en mí y extrae el valor aunque no exista».

Preferiría que una extracción que pueda fallar tenga un nombre visible o requiera comprobar el caso.

Además, ausencia y error serían conceptos distintos. Buscar un usuario inexistente puede producir Usuario?; no poder consultar la base de datos debe producir un error identificable.

---

# 11. Errores: legibles en la firma y valores en la semántica

Este sería uno de los rasgos más importantes de la propuesta:

enum ErrorDivision {
    DivisionPorCero
}

fn Dividir(a: F64, b: F64) -> F64 fails ErrorDivision {
    if b == 0.0 {
        fail ErrorDivision.DivisionPorCero
    }

    return a / b
}
La firma se lee:

> Dividir recibe dos números y produce un número, pero puede fallar con ErrorDivision.

Debajo de esa sintaxis, el resultado sería:

Result<F64, ErrorDivision>
La idea de representar errores recuperables mediante Result<T, E> tiene un precedente directo en Rust. Aquí propondría una superficie más verbal para las declaraciones y la propagación. [Documentación de Rust](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)

### Tratar el error

let mensaje = match Dividir(10.0, 0.0) {
    Ok(valor) => "Resultado: {valor}"
    Err(ErrorDivision.DivisionPorCero) => "No se puede dividir por cero"
}
### Propagarlo

fn CalcularPromedio(total: F64, cantidad: F64)
    -> F64 fails ErrorDivision
{
    let promedio = try Dividir(total, cantidad)

    return promedio
}
Las reglas serían:

**fail error** produce el resultado fallido.

**try expresion** extrae el valor correcto o devuelve el error desde la función aUna llamada sin tryin try** conserva el Result, que puede guardarse, examinarse o transformarse.

No habría un segundo mecanismo paralelo de excepciones recuperables invisibles en las firmas.

Cuando el error de una función no coincida con el de la función que la llama, exigiría una conversión explícita, por ejemplo mediante MapError.También distinguiría fallos recuperables de errores de programación: fails no prometería que el hardware nunca falla ni que un índice inválido es imposible. Los accesos fuera de límites y desbordamientos tendrían una política comprobada y definida; no comportamiento indefinido silencioso.

---

# 12. Colecciones: composición clara, evaluación explícita

Las colecciones comunes serían sencillas:

let numeros = [1, 2, 3, 4]

let coordenada = (10, 20)

let edades = Map([
    ("Ana", 20),
    ("Luis", 24),
])

let identificadores = Set([1, 2, 2, 3])
El literal [...] representaría una lista dinámica. Un arreglo de tamaño fijo se declararía como tal:

let canales: Array<U8, 4> = [255, 128, 0, 255]
### Flujo de transformaciones

Adoptaría |> con una única regla: introducir el valor de la izquierda como primer argumento de la llamada de la derecha. Esa es la función documentada del operador en Elixir. [Hexdocs](https://hexdocs.pm/elixir/Kernel.html)

let resultado = numeros.Iter()
    |> Filter(n => n > 0)
    |> Map(n => n * 2)
    |> Take(10)
    |> ToList()
La equivalencia básica sería:

valor |> Transformar(argumento)
Con:

Transformar(valor, argumento)
La expresión de la izquierda se evaluaría una sola vez.

**.Iter() indicaría un recorrido diferido. ToList() materializaría una colección.**

Esto evita que la sintaxis aparente garantizar algo que en realidad depende de la biblioteca. Una tubería no significaría automáticamente «sin asignaciones», «paralelo» o «fusionado en un solo bucle».

Reservaría el punto para operaciones asociadas a un objeto:

archivo.Position
imagen.Width
Y favorecería la tubería para transformar un flujo de datos.

No añadiría inicialmente una segunda sintaxis completa de consultas y una tercera de comprensiones para realizar lo mismo.

---

# 13. Lambdas: compactas, pero sin variables misteriosas

Usaría parámetros con nombre:

n => n * n

(a, b) => a + b

jugador => jugador.Activo
Cuando se necesiten tipos:

let duplicar = (valor: Int) => valor * 2
Para varios pasos:

let ajustar = (valor: Int) => {
    let limitado = Clamp(valor, 0, 100)

    return limitado * 2
}
Tomaría la facilidad de pasar funciones que ofrecen lenguajes como Kotlin, pero no sus parámetros implícitos como norma del nuevo lenguaje. Kotlin documenta tanto los tipos de función como las lambdas y sus formas de uso. [Kotlin](https://kotlinlang.org/docs/lambdas.html)

No adoptaría simultáneamente:

it
$0
_
Como maneras alternativas de referirse al argumento.

Ahorrar una letra no me parece suficiente beneficio para introducir tres convenciones.

### Capturas

En cierres que necesiten conservar datos, permitiría expresar la intención:

let factor = 3

let multiplicar = capture [copy factor] (valor: Int) => {
    return valor * factor
}
Los cierres locales que solo leen podrían inferir préstamos. Pun cierre que escape de su ámbito no podría conservar una vista inválida a una variable localal**.

Las capturas por copia, edición o transferencia formarían parte de su comprobación de tipos. No serían decoraciones que el compilador ignore.

---

# 14. Genéricos, contratos y polimorfismo

Los genéricos tendrían nombres normales y restricciones visibles:

fn Mayor<T>(a: T, b: T) -> T
    where T: Order and Copy
{
    return if a >= b then a else b
}
Aquí Order proporciona comparación y Copy permite las copias implícitas necesarias.

Para contratos de comportamiento:

trait Describible {
    fn Describir() -> String
}
Implementación:

impl Describible for Rectangulo {
    fn Describir() -> String {
        return "Rectángulo de {self.Ancho} × {self.Alto}"
    }
}
Uso genérico:

fn Mostrar<T>(valor: view T)
    where T: Describible
{
    Console.WriteLine(valor.Describir())
}
**El contrato sería nominal y explícito:** tener un método con el mismo nombre no convertiría automáticamente un tipo en implementación de cualquier contrato.### Despacho dinámico explícito

Cuando el tipo concreto deba resolverse en ejecución:

fn MostrarDinamico(valor: view dyn Describible) {
    Console.WriteLine(valor.Describir())
}
`dyn` mostraría esa elección. No mezclaría silenciosamente el modelo genérico con el dinámico.

### Extensiones

Permitiría añadir operaciones sin modificar el tipo original:

extend String {
    fn EstaVacio() -> Bool {
        return self.ByteLength == 0
    }
}
Pero una extensión no podría añadir almacenamiento, acceder a detalles privados ni reemplazar métodos existentes. Dos extensiones igualmente aplicables y ambiguas producirían un error.

Para sobrecargar operadores, usaría una lista cerrada de contratos. No permitiría inventar operadores arbitrarios ni modificar su precedencia.

---

# 15. Memoria y mutación: aquí no escondería complejidad

Tomaría en serio la propiedad y los préstamos, no solamente la apariencia de Rust. Su modelo de propiedad se basa en reglas que el compilador comprueba para gestionar memoria. Cambiar los símbolos no elimina esas obligaciones. [Documentación de Rust](https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html)

Propondría este vocabulario:

| Forma | Significado |
|---|---|
| view T | Préstamo de lectura |
| edit T | Préstamo mutable exclusivo |
| take T | La función recibe la propiedad |
| move valor | Transferencia explícita |
| copy valor | Copia lógica explícita |

### Leer sin transferir

fn Contar(datos: view List<Int>) -> USize {
    return datos.Count
}

let cantidad = Contar(numeros)
La lectura no exigiría símbolos adicionales en cada llamada.

### Modificar explícitamente

struct Punto {
    var X: F64
    var Y: F64
}

fn Reiniciar(punto: edit Punto) {
    punto.X = 0.0
    punto.Y = 0.0
}

var posicion = Punto(X: 10.0, Y: 20.0)

Reiniciar(edit posicion)
La firma y la llamada muestran que la función puede modificar el argumento.

Para métodos, la regla equivalente sería:

struct Contador {
    var Valor: Int = 0

    edit fn Incrementar() {
        self.Valor += 1
    }
}

var contador = Contador()

edit contador.Incrementar()
Ese `edit adicional tiene un coste visual. Lo aceptaría porque permite localizar modificaciones sin inspeccionar todas las implementaciones.

### Transferir propiedad

Una función consumidora se declararía así:

``text
fn Enviar(datos: take Buffer<U8>) -> Envio {
    return Envio(Datos: move datos)
}

La llamada:

text
let envio = Enviar(move buffer)

Después, `buffer` dejaría de estar disponible.

### Copias independientes

text
var original = [1, 2, 3]
var copia = copy original

copia[0] = 99

La modificación no afectaría al valor lógico de `original`.

Permitiría compartir internamente almacenamiento inmutable como optimización, pero no cambiar esa independencia observable.

Las copias implícitas quedarían reservadas a tipos admitidos por `Copy`. Una asignación de un valor no copiable no desencadenaría una clonación grande por sorpresa.

### Vistas que salen de una función

La relación con su origen también debería poder expresarse:

text
fn Primero<T>(items: view List<T>)
    -> (view T)? from items
{
    return items.First()
}

`from items` indicaría que la vista devuelta no puede sobrevivir al origen.

**No prometería eliminar los tiempos de vida; intentaría expresarlos con menos ruido donde sean necesarios.**

---

# 16. Recursos: limpieza automática y finalización comprobable

Los archivos, conexiones y otros recursos no se copiarían como números.

Un ejemplo sería:

text
fn LeerArchivo(ruta: view String) -> Bytes fails IoError {
    var archivo = try File.Open(ruta)

    return try File.ReadAll(edit archivo)
}
`

Al abandonar el ámbito normalmente o mediante un error recuperable, el recurso se liberaría según su contrato.

Pero no escondería una distinción importantLiberar un recurso no es lo mismo que confirmar que una operación terminó correctamente.e.**Un archivo puede necesitar un Flush comprobable. Una transacción puede necesitar Commit. Esas operaciones devolverían errores y no se relegarían a un destructor incapaz de informarlos adecuadamente.

También incluiría defer para acciones de salida explícitas, ejecutadas en orden inverso al registro:

defer Metrics.RecordOperationEnd()
Su cuerpo no podría propagar errores recuperables ni suspenderse de manera oculta. La referencia de Zig es un precedente útil tanto para defer como para hacer visibles operaciones de compilación. [Zig](https://ziglang.org/documentation/master/)

No garantizaría que esa limpieza ocurra ante una terminación forzada del proceso o un fallo del sistema.

---

# 17. Concurrencia: tareas con límites, no trabajo abandonado

Separaría asincronía, creación de tareas y paralelismo.

Una función asíncrona sería:

async fn ObtenerUsuario(id: UsuarioId)
    -> Usuario fails ApiError
{
    let respuesta = try await Api.GetUsuario(id)

    return try respuesta.Decode<Usuario>()
}
En esta propuesta, llamarla produciría una operación pendiente; await la ejecutaría o esperaría dentro de la tarea actual. spawn la programaría como tarea hija.

No asumiría que escribir async crea un hilo.

### Concurrencia estructurada

Tomaría como referencia los grupos de tareas de Swift: delimitan la vida de las tareas hijas y esperan su finalización antes de abandonar el grupo. La cancelación es cooperativa, no una garantía de detener instantáneamente cualquier cálculo. [GitHub](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0304-structured-concurrency.md)

Mi superficie sería:

async fn CargarFicha(id: UsuarioId)
    -> Ficha fails ApiError
{
    await taskgroup {
        let usuarioTask = spawn Api.ObtenerUsuario(id)
        let ajustesTask = spawn Api.ObtenerAjustes(id)

        let usuario = try await usuarioTask
        let ajustes = try await ajustesTask

        return Ficha(
            Usuario: move usuario,
            Ajustes: move ajustes,
        )
    }
}
Elegiría await taskgroup para hacer visible que salir del grupo también puede requerir esperar.

Si una propagación de error abandona el grupo, se solicitaría cancelar las tareas pendientes y se esperaría su limpieza. Cada resultado tendría que observarse o descartarse mediante una operación explícita.

El sistema de tipos comprobaría qué datos pueden atravesar tareas y qué préstamos siguen siendo válidos.

Para comunicación y estado compartido, preferiría bibliotecas como Channel<T> y Actor<T>, apoyadas por esas comprobaciones, antes que incorporar muchas palabras clave adicionales.

---

# 18. Generadores, texto y construcción de interfaces

Son tres áreas distintas, pero deberían aprovechar mecanismos existentes.

### Generadores

iter fn Contar(hasta: Int) -> Int {
    for numero in 0..<hasta {
        yield numero
    }
}
La firma declararía el tipo de cada elemento; llamar a la función produciría una Sequence<Int>.

for numero in Contar(3) {
    Console.WriteLine(numero)
}
`yield produciría un elemento. return terminaría el generador. No usaría la misma palabra para ambas cosas.

### Texto

``text
let nombre = "Ana"
let mensaje = "Hola, {nombre}"

let ruta = r"C:\Proyectos\Juego"

let consulta = """
    SELECT Nombre, Puntos
    FROM Jugadores
    WHERE Activo = true
    """

Las llaves literales se escribirían como `{{` y `}}`.

Definiría `String` como texto UTF-8 y distinguiría operaciones sobre bytes, valores Unicode y grafemas. No daría a `texto[0]` un significado ambiguo de «primer carácter».

Por ejemplo:

text
texto.Bytes()
texto.Runes()
texto.Graphemes()

### Interfaces de usuario

No crearía otro lenguaje incrustado obligatoriamente:

text
let panel = Ui.Column(
    gap: 12,
    children: [
        Ui.Text("Perfil"),
        Ui.Button(
            "Guardar",
            onClick: () => Console.WriteLine("Guardado"),
        ),
    ],
)
`Aquí Ui.Text, Ui.Button y Ui.Column serían funciones de biblioteca que producen nodos compatibles.

Una biblioteca debería poder resultar expresiva sin alterar la gramática del lenguaje.

---

# 19. Bajo nivel y efectos: garantías comprobables, no etiquetas decorativas

Para cubrir software de sistemas, introduciría restricciones verificables.

@NoAlloc
fn Sumar(datos: view List<F32>) -> F32 {
    var total: F32 = 0.0

    for dato in datos {
        total += dato
    }

    return total
}
`@NoAlloc significaría que el compilador debe comprobar la ausencia de asignaciones de memoria dinámica, incluyendo las operaciones llamadas.

No sería una sugerencia al optimizador.

De manera similar:

``text
pure fn Cuadrado(valor: Int) -> Int => valor * valor

`pure` prohibiría E/S, mutación externa y lectura de estado mutable oculto. No prometería terminación ni ausencia de errores de programación.

Estas capacidades requieren un sistema de efectos real. **No se obtienen por añadir dos palabras a la gramática.**

### Fronteras externas

La interoperabilidad tendría una frontera visible:

text
extern "C" {
    unsafe fn Procesar(
        datos: Ptr<U8>,
        cantidad: C.Size,
    ) -> C.Int
}

Usaría tipos de ABI explícitos: `C.Int` no se asumiría equivalente a nuestro `Int`.

Las operaciones inseguras requerirían un bloque `unsafe` y contratos revisables. Ese bloque no desactivaría indiscriminadamente todas las comprobaciones del lenguaje.

El diseño permitiría trabajar sin un recolector de basura obligatorio, pero no confundiría ese objetivo semántico con una garantía de rendimiento ya demostrada.

---

# 20. Módulos, pruebas y metaprogramación

### Módulos

text
module juego.geometria

use std.math as Math

pub fn Hipotenusa(a: F64, b: F64) -> F64 {
    return Math.Sqrt(a * a + b * b)
}

Las declaraciones serían internas al módulo por defecto; `pub` las expondría. Los tipos podrían restringir miembros con `private`.

Evitaría importaciones globales implícitas y resolución de nombres que cambie según el orden de los archivos.

### Pruebas y documentación

text
/// Suma los cuadrados de los números pares.
fn SumarCuadradosPares(numeros: view List<Int>) -> Int {
    return numeros.Iter()
        |> Filter(n => n % 2 == 0)
        |> Map(n => n * n)
        |> Sum()
}

test "Los pares de 1 a 6 producen 56" {
    let resultado = SumarCuadradosPares([1, 2, 3, 4, 5, 6])

    Assert.Equal(resultado, 56)
}

El lenguaje y sus herramientas reconocerían pruebas y ejemplos de documentación.

### Generación comprobada

text
@derive(Eq, Hash, Debug)
struct Version {
    Major: U16
    Minor: U16
    Patch: U16
}

La derivación solo existiría cuando sus campos satisfagan los contratos correspondientes.

Para cálculo durante compilación:

text
const BlockSize: USize = 64

const Tabla = comptime ConstruirTabla(BlockSize)

Zig documenta `comptime` como evaluación durante el análisis del programa; lo tomaría como referencia para una capacidad delimitada, no para copiar todo su modelo. [Zig](https://ziglang.org/documentation/master/)

No empezaría con macros capaces de reescribir arbitrariamente cualquier parte del lenguaje. Primero introduciría derivaciones controladas, reflexión de tipos y evaluación con límites de recursos, sin acceso ambiental irrestricto.

---

# 21. Un ejemplo integrado

Este ejemplo muestra cómo deberían encajar datos, argumentos nombrados, préstamos, errores, patrones e interpolación.

text
module ejemplo.ranking

use std.io.Console

struct Jugador {
    Nombre: String
    Puntos: Int
    Activo: Bool = true
}

struct Resumen {
    Cantidad: USize
    Total: Int
}

enum RankingError {
    MinimoNegativo
}

fn Resumir(
    jugadores: view List<Jugador>,
    minimo: Int = 0,
) -> Resumen fails RankingError {
    if minimo < 0 {
        fail RankingError.MinimoNegativo
    }

    var cantidad: USize = 0
    var total: Int = 0for jugador in jugadores {
        if jugador.Activo and jugador.Puntos >= minimo {
            cantidad += 1
            total += jugador.Puntos
        }
    }

    return Resumen(
        Cantidad: cantidad,
        Total: total,
    )
}

fn Main() {
    let jugadores = [
        Jugador(Nombre: "Ana", Puntos: 150),
        Jugador(Nombre: "Luis", Puntos: 900, Activo: false),
        Jugador(Nombre: "Eva", Puntos: 200),
    ]

    let mensaje = match Resumir(jugadores, minimo: 100) {
        Ok(resumen) =>
            "{resumen.Cantidad} jugadores, {resumen.Total} puntos"

        Err(RankingError.MinimoNegativo) =>
            "El mínimo no puede ser negativo"
    }

    Console.WriteLine(mensaje)
}

Con esos datos, imprimiría:

text
2 jugadores, 350 puntos
`

Lo importante no es solamente que se vea ordenado.

La función declara que lee la lista, que puede devolver un error concreto, que sus acumuladores son mutables y que sus resultados tienen campos nombrados. No hay que descubrir esas decisiones inspeccionando convenciones externas.

---

# 22. Cómo convertir esto en un diseño utilizable

No implementaría cada ejemplo como una excepción independiente. Definiría un núcleo semántico y traduciría la sintaxis hacia él.

| Construcción superficial | Representación o significado central |
|---|---|
| `T?` | `Option<T>` |
| `-> T fails E` | Resultado `Result<T, E>` |
| `=> expresion` | Cuerpo con retorno explícito |
| `x |> F(a)` | Llamada `F(x, a)` |
| `prop Area` | Accesor de lectura con restricciones |
| `async fn ... -> T` | Operación asíncrona que produce `T` |
| `iter fn ... -> T` | Secuencia que produce elementos `T` |

Eso permitiría cambiar la presentación sin inventar otra semántica.

Para tu idea de varias sintaxis sobre una base común, mantendría esta como la orientada a desarrolladores. Una representación compacta para herramientas podría llegar después, sobre la misma estructura interna y con equivalencia comprobada. No sacrificaría ahora la lectura humana para ahorrar tokens sin mediciones.

Antes de aprobarla, faltaría formalizar y probar cuestiones como la precedencia completa, la separación entre genéricos y comparaciones, los saltos de línea, las capturas, los préstamos devueltos y la cancelación.

Los ejemplos son una propuesta bastante amplia de diseño, no una especificación formal cerrada.

## Qué rechazaría deliberadamente

No permitiría tres palabras para declarar funciones, cinco maneras equivalentes de escribir lambdas, operadores definidos por cada biblioteca, conversiones potencialmente destructivas implícitas ni excepciones recuperables ocultas.

Tampoco añadiría una característica solo porque otro lenguaje la tiene. Su incorporación tendría que mejorar un caso concreto sin deteriorar demasiado los demás.

## Dónde intentaría que fuese notablemente mejor

| Objetivo | Decisión | Coste aceptado |
|---|---|---|
| Leer modificaciones sin perseguir implementaciones | `edit` visible | Más palabras en operaciones mutables |
| Distinguir ausencia de fallo | `Option` separado de `Result` | Modelar correctamente ambos casos |
| Evitar copias grandes inadvertidas | `copy` y `move` explícitos | Algunas asignaciones más verbosas |
| Comprender operaciones asíncronas | `await`, `spawn`, grupos delimitados | Un modelo de tareas que aprender |
| Mantener una apariencia uniforme | Convenciones y formateador oficial | Menos libertad estilística |
| Permitir control de bajo nivel | Vistas, recursos, efectos y fronteras inseguras | Mayor complejidad del compilador |

La superioridad tendría que comprobarse implementando los mismos programas: una herramienta de consola, un servicio asíncrono, una interfaz, un pequeño sistema de juego y un núcleo numérico. Mediría comprensión, errores, mantenimiento, tiempos de compilación y costes de ejecución, no únicamente cantidad de caracteres.Mi elección final sería esta: mayúsculas que den estructura, palabras que revelen intención y símbolos que tengan un único trabajo. No buscaría que nadie reconozca ninguna influencia; buscaría que ninguna influencia domine y que el conjunto tenga reglas propias, consistentes y justificables.