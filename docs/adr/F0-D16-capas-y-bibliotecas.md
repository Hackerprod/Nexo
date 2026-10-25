# F0-D16 — Dos capas y bibliotecas escritas en Nexo

Estado: **decisiones de arquitectura revisadas de forma independiente; sin implementación ni aprobación de fase**.
Fecha: 2026-10-25.

## 1. Autoridad y alcance

**R01.** Rigen [AGENTS.md](../../AGENTS.md), [acuerdos §9](../F0_ACUERDOS.md),
especialmente acuerdos §§9.1–9.4, 9.7 y 9.8, y la base inmutable
[roadmap 0.2](../baseline/Nexo_ROADMAP_rev0.2.md). Este ADR propone las
enmiendas de D16 §12; no modifica esa base ni crea una revisión 0.3. Las decisiones
de capas, biblioteca, confianza e interfaz interna se fijan antes de congelar
F1. La ejecución de sus entregables no aprueba F0, F1 ni otra fase.

**R02.** La superficie única es [D15](F0-D15-sintaxis-oficial.md), B.
[D14](F0-D14-sintaxis-superficie.md) conserva motivaciones históricas.
[D11](F0-D11-closures.md), [D12](F0-D12-async.md) y
[D13](F0-D13-expresiones-modulos.md) gobiernan capturas, suspensión,
procedencias, módulos y coherencia, con las sustituciones de superficie D15.
Los cortes son [0001](F0-0001-first-slice.md),
[0002](F0-0002-second-slice.md), [0003](F0-0003-third-slice.md),
[0005](F0-0005-operators-control-collections.md) y
[0004](F0-0004-text-and-user-types.md), en ese orden. En adelante A2, J, F5 y
K designan los últimos cuatro contratos, no modos de ejecución alternativos.

**R03.** «Biblioteca en Nexo» significa que cada algoritmo, política de
almacenamiento, validación de texto, formato, protocolo y gestión de recursos
de la distribución se entrega como fuente `.n`, comprobable por el frontend
público. No significa que el compilador inicial, las instrucciones de máquina,
el kernel o el cargador del sistema estén escritos en Nexo. Se enumeran esas
excepciones; no se permite convertir cualquier dependencia en una «primitiva».
El intérprete inicial Rust ejecuta operaciones irreductibles y código Nexo
verificado. Self-hosting y backend propio son objetivos diferentes.

## 2. Dos capas sobre una semántica

**R04.** La capa baja expone propiedad de almacenamiento, extensión, alineación,
layout, asignador, vistas y operaciones de máquina. La capa alta ofrece valores,
recursos encapsulados, colecciones crecientes, texto, formato e iteración. Las
dos usan los mismos tipos nominales, modos `view/edit/move`, inferencia,
préstamos, efectos, Result, CFG y verificador. No hay bandera de compilación
que convierta accesos incorrectos en válidos ni asignaciones en movimientos.

**R05.** Los módulos organizan las capas; no conceden permiso por llamarse
`low`, `core` o `std`. Se adoptan `std.core`, `std.mem`, `std.alloc`,
`std.collections`, `std.text`, `std.io` y `std.platform`. K ya publica
`std.text`, `std.console` y `std.parse`; conservan sus responsabilidades.
`std.fs` se introduce en M11; no es una fachada publicada por F0. Una
fachada solo reexporta o compone funciones Nexo; no contiene una segunda
implementación. Importar una capa baja segura no exige `unsafe`. Ejecutar
una operación de confianza sí exige el permiso léxico de R08.

**R06.** Se decide la distinción de F5 §4.1: **List<T> es la colección dinámica;
Array<T, length: N> es el arreglo inline de extensión constante**. N es un
entero no negativo conocido en compilación, expresado por el argumento
constante nombrado de B. Array tiene layout contiguo determinado por T y N,
no capacidad sobrante, Push, Pop ni crecimiento. Su construcción exige N
elementos inicializados; para N=0 no se evalúa un elemento ficticio. Copia,
independencia y limpieza se derivan de T; puede contener recursos y entonces
no se copia. No se permite mover un elemento de un lugar superviviente sin
una operación de reemplazo que conserve inicialización.

List posee almacenamiento dinámico y mantiene `0 <= length <= capacity`.
Su literal no contextual `[a,b]` construye List; un tipo esperado Array exige
extensión exactamente coincidente. Un literal vacío sin contexto suficiente
no elige I64. `Array<T>` sin length deja de ser una forma válida en la
migración M09; no designa List ni un arreglo de extensión desconocida.
`Len`, `Push`, `Pop` y `JoinArrays` de F5 migran respectivamente a las
funciones Nexo `Len`, `Push`, `Pop` y **JoinLists** del preámbulo público.
No queda alias JoinArrays ni Array dinámico. Las APIs de String que producen
colecciones, como Scalars, pasan a List. Una vista de extensión dinámica es
`Slice<T>`, con procedencia, nunca un Array de tamaño inferido en ejecución.

**R07.** List, String y Array conservan independencia lógica de sus elementos
independientes. List con recurso es un contenedor propietario no copiable;
la eliminación final devuelve propiedad mediante Pop, sin hueco observable.
List con préstamos retiene la unión de sus orígenes y exige contratos
`retains/from`; moverla no prolonga esos orígenes. String contiene UTF-8
válido; Bytes contiene bytes arbitrarios. String no ofrece índice entero
de «carácter», modificación cruda pública ni normalización implícita.
La interfaz cómoda usa asignador de montículo explícito en su definición;
una región no redirige ese asignador por contexto textual. Las variantes
con asignador reciben y conservan su procedencia. En F1 se usan copias
profundas; CoW local entra en F2 conforme D09, con un cambio de modelo de
costes explícito, nunca junto a la migración que debe contrastar equivalencia.

### 2.1. Permiso de confianza y superficie nueva

**R08.** Se adopta `unsafe mod nombre;`, `unsafe fn` y el bloque sentencia
`unsafe { ... }`. El módulo marcado delimita la unidad auditable; una
función unsafe declara obligaciones adicionales del llamador; el bloque
identifica cada sitio que las asume. Ninguna de las tres marcas sustituye
a las otras. Solo un módulo declarado unsafe puede contener funciones
unsafe, bloques unsafe o representaciones R12. Un módulo unsafe puede
exportar funciones seguras que restablezcan sus invariantes para todo
argumento admitido. El cuerpo de una función unsafe **no** es un bloque
unsafe implícito. Los submódulos no heredan permiso: cada declaración de
subarchivo que lo necesita se marca. La raíz no admite permiso implícito;
los detalles confiados se sitúan en un submódulo.

**R09.** Enmienda exacta a D15 §§2.2 y 3.5, conservando sus otras alternativas:

```ebnf
ModDecl       = ["pub"] ["unsafe"] "mod" Name Terminator ;
FnDecl        = ["pub"] ["unsafe"] ["async"] ["gen"] "fn" Name
                [GenericParams] Signature [WhereClause] Block ;
MethodDecl    = ["pub"] ["unsafe"] "fn" Name [GenericParams]
                Signature [WhereClause] Block ;
UnsafeStmt    = "unsafe" Block ;
Statement     = Binding | Assignment | ReturnStmt | ValueStmt | YieldStmt
              | ExprStmt | IfStmt | WhileStmt | ForStmt | LoopControl
              | MatchStmt | Block | RegionStmt | GuardStmt | UnsafeStmt ;
StorageAttribute = "@" "storage" "(" "element" ":" Type ","
                   "copy" ":" Path "," "drop" ":" Path ")" ;
Attribute     = BaseAttribute | StorageAttribute ;
ModuleDecl    = {Attribute} BareModuleDecl ;
Variant       = Name ["(" VariantPayload {"," VariantPayload} ")"] ;
VariantPayload = [Name ":"] Type ;
GenericParam  = Name [":" Bound {"and" Bound}] | "const" Name ":" Type ;
```

ModuleDecl admite Attribute donde antes admitía BaseAttribute. `unsafe` se
añade como palabra reservada de B completa mediante la sustitución literal
de 0003 §10.1 registrada en R55; `storage` y `element` son Name
contextuales exclusivamente en este atributo. No se añade expresión unsafe,
puntero `*T`, operador de dirección, cast, literal de dirección ni asm.
`unsafe async fn` y `unsafe gen fn` se reconocen pero se rechazan por
R10: la frontera es síncrona. No se cambia la gramática de `with`.
Cada variante elige payloads todos nombrados o todos sin nombre. La forma
mixta `V(a: I64, I64)` se reconoce pero se rechaza con
NX-E1603/mixed_variant_payloads, types/types.check, sobre el primer tipo
sin nombre que mezcla formas (o el primer nombre si precedía uno sin nombre),
data={}, sin relaciones. No se inventan nombres para posiciones anónimas.
Nombres repetidos usan NX-E0104, subcause=null, names/names.declare,
declaration_kind=field, primario segundo nombre y previous_declaration del
primero, como campos K. Los payloads nombrados aceptan argumentos posicionales
seguidos de etiquetas; cada posición se asocia una vez, con NX-E0111 y sus
subcausas existentes para omisiones, duplicación o etiquetas desconocidas.
Los payloads sin nombre siguen rechazando etiquetas. Option declara
`Some(v: T)` y Result `Ok(v: T), Err(v: E)`; se conservan `Some(v:1)` y
las etiquetas J/A2 sin excepciones por nombre de enum. Los patrones siguen
siendo posicionales. M08 integra esta enmienda a Variant y K §4.1.

`const` ya es reservada. `const length: I64` se instancia con `length: 2`
o una constante de ese tipo; TypeArg/ConstTypeArg de D15 §3.5 no cambian.
Array exige length no negativo y tamaño representable. Después de M09,
`Array<I64>` da NX-E0111/missing_argument, data={}, sobre el
tipo aplicado completo, relacionado declaration del parámetro; conserva
las demás claves del catálogo para missing_argument, sin tokens sintácticos.
No infiere List. La aridad histórica de Array solo permanece antes de M09.

Las funciones externas no se declaran con otra sintaxis: son entradas
tipadas del catálogo público de plataforma de R22.

**R10.** El permiso no es uno de los efectos alloc/block/suspend/io. Es una
obligación estática distinta, visible en la firma y en el grafo de confianza.
No borra tipos, disponibilidad, exclusividad de préstamos Nexo, procedencia,
efectos, cuotas ni limpieza. Una llamada unsafe exige bloque unsafe y
contrato de destino resuelto. El bloque no transmite permiso a una closure
o función anidada; cada cuerpo se comprueba desde permiso vacío. No se
permite suspensión en una función unsafe ni en un bloque unsafe, aun por
llamada transitiva o efecto desconocido. Un puntero derivado de un préstamo
conserva origen y duración; para memoria extranjera solo existe una suposición
externa explícita. Declarar confianza no convierte esa suposición en prueba.
En esta primera frontera las funciones unsafe solo admiten llamada directa:
usarlas como valor, callback o argumento Call* se rechaza. Una closure segura
puede encapsular una operación mediante su propio bloque y contrato auditado;
no se borra un bit de confianza al convertir una función unsafe en invocable.

**R11.** La capa alta garantiza inicialización, límites de acceso, unicidad
de propietario, exclusividad, ausencia de préstamos colgantes y preservación
de UTF-8 conforme sus tipos, condicionalmente a P07. La frontera confiada
debe validar datos extranjeros antes de producir un valor seguro. Un módulo
que devuelve una vista sobre memoria ya liberada incumple su contrato aunque
su bloque unsafe sea sintácticamente correcto. La auditoría es necesaria;
el comprobador no decide la verdad de obligaciones arbitrarias de punteros.
Los informes distinguen `verified`, `external_assumption` y `unknown`.
No se ofrece reparación automática que añada unsafe o amplíe una cota.

### 2.2. Copia y destrucción de representaciones propias

**R12.** Una biblioteca de valores necesita expresar una representación que
posee memoria sin copiar direcciones ni duplicar un recurso. Se introduce el
contrato confiado `@storage(element: T, copy: C, drop: D)` sobre un struct
definido en un módulo unsafe. Es la única excepción nueva a la derivación
por campos físicos. No es `impl Copy`, `impl Send`, `impl SuspendSafe` ni
una excepción reservada a std. El autor debe poseer el tipo nominal exterior.

El atributo declara un contenedor de cero o más elementos **homogéneos T**.
Sus otros campos son metadatos escalares independientes, RawAllocation<T>
o RawHandle de un descriptor de almacenamiento cuyos datos lógicos son T.
Ese descriptor solo contiene T, escalares y enlaces de almacenamiento de la
misma familia; un ciclo debe atravesar una indirección. No puede esconder
handles extranjeros, permisos de otros tipos, callbacks,
otro recurso o identidad mutable compartida. El verificador comprueba esa
forma, tipos de hooks y efectos; la auditoría comprueba correspondencia exacta
de los elementos vivos con la representación. Este contrato no basta para
un mutex, actor, archivo, Rc compartido o contenedor heterogéneo.

**R13.** C es función Nexo segura ordinaria `(x: view S)->S` para el mismo S
instanciado. En `struct S<T>` las rutas C/D se resuelven en el ámbito de
S a funciones genéricas; sus parámetros se sustituyen por los argumentos
de S en orden de declaración, incluidos constantes. La firma instanciada
coincide exactamente y sus restricciones se prueban al instanciar S; no
hay búsqueda en el llamador ni inferencia independiente del hook. Una ruta
incompatible da invalid_storage_hook. C debe tener `no_io, no_suspend`; se permiten alloc y block,
siempre visibles en el resumen, y
fallo irrecuperable D02, no un resultado Err en copia implícita. D es función
Nexo segura ordinaria `(x: edit S)->()` con no_suspend. Los dos cuerpos usan
bloques unsafe solo en los sitios necesarios. C no puede invocar la copia
de S sobre el mismo objeto para implementarse. D destruye únicamente los
elementos vivos debidos por el propietario y deja vacías sus asignaciones;
para CoW libera su participación y solo la última destruye los elementos.
Después el plan estructural
destruye campos y libera bytes. No se llama recursivamente a Drop(S) desde D.
La inicialización parcial registra elementos instalados antes de cualquier
salida fallable; no espera a construir el objeto completo para tener dueño.

**R14.** El compilador deriva copiabilidad de S exactamente de T y de la
validez de C; independencia de S exactamente de T y del contrato auditado;
procedencias, afinidad y SuspendSafe se propagan desde todos los elementos
vivos/posibles. Para T no copiable no se instancia C y S no se copia. La
interfaz incluye las condiciones de instanciación, no un C universal que
autorice copiar recursos. El descriptor público distingue «derivado
estructuralmente» de «representación confiada R12». El contrato incorpora
orden, número de elementos, origen, limpieza, efectos y layout; cambiarlo
invalida consumidores. Los hooks no pueden introducir efectos externos
arbitrarios en copia. La independencia de sus algoritmos sigue siendo una
suposición auditada, no una propiedad demostrada por la firma.

**R15.** Antes de distribuir un módulo confiado se publica un inventario
externo al paquete por símbolo: paquete/revisión/package_digest, módulo, span de permiso, precondiciones,
postcondiciones, operación irreductible alcanzada, contrato de procedencia,
efectos por operación, layout, plataformas, evidencia y responsable. El
inventario externo contiene aristas hasta cada hoja y cada hook; ningún permiso queda
oculto por inlining, monomorfización o reexportación. El informe se obtiene
de IR e interfaces, no de buscar la palabra unsafe en texto. La revisión de
un módulo no acredita módulos nuevos ni una revisión nueva del mismo cuerpo.
El inventario y el registro del valor esperado de package_digest quedan fuera
del contenido cubierto del paquete; consumen el cálculo único de D17 R10.
Se generan tras fijar ese contenido y no se incorporan a ningún archivo cubierto.

## 3. Construcción sin ciclos

**R16.** El orden de construcción siguiente es obligatorio. Una dependencia
de ejecución no puede retroceder hacia una operación que necesita el tipo
que está construyendo. Recursión ordinaria de funciones es admisible; ciclos
de tamaño inline no lo son. Las definiciones de tipos genéricos no necesitan
ejecutar el algoritmo que posteriormente los utiliza.

| Nivel de construcción | Construcción alta | Implementación sobre el nivel anterior |
|---|---|---|
| 0 | Escalares, unidad, agregados, lugares, llamadas, control | Núcleo de tipos/IR; enteros comprobados, registros, tags, ramas, permisos y planes de vida. No depende de String ni List. |
| 1 | Option y Result | Enums genéricos `.n`, tags None=0/Some=1 y Ok=0/Err=1; match usa el descriptor común. El propagador `?` es lowering a selección, extracción y retorno, no algoritmo de biblioteca. |
| 1 | Array inline, RawAllocation, RawView | Extensión/layout y permisos del núcleo; asignación cruda mediante C04 del backend. Ningún elemento inicializado se inventa al reservar bytes. |
| 2 | Asignador, búfer propietario, List, Bytes | Políticas `.n` sobre asignación y lugares crudos; crecimiento, copia y limpieza R12. List usa longitud/capacidad y no necesita String ni Format para manejar un error. |
| 3 | String, parsing y formato escalar | Bytes, enteros y bucles `.n`; validación UTF-8, slicing, conversión decimal. Error básico nominal sin String ni formato obligatorio. |
| 4 | Mapas, conjuntos, árboles y persistencia | List/Array, nodos propietarios, IDs y traits; hashing y colisiones en Nexo. RC/CoW local en Nexo sobre memoria; compartición publicada necesita atómicos de nivel 1. |
| 4 | Format y derivaciones | Trait y recetas comptime en Nexo sobre metadatos tipados públicos; expansión tipada sometida al checker común. Formato a un sink no crea String obligatoriamente. |
| 4 | Archivos, consola, E/S, CLI y JSON | Recursos y bucles Nexo sobre entradas de sistema acotadas; decodificación y políticas fuera del compilador. |
| Transformación estructural | Closure | Registro de capturas, funciones de cuerpo/copia/drop y llamada con entorno explícito; construcción sobre agregados/permisos de niveles 0–2. Call* sigue sellado y generado. |
| Transformación estructural | Generador | Registro propietario con tag de continuación, slots vivos, Resume/Drop y selección; retorno de Option nivel 1. Iterator y adaptadores son Nexo. No hay array precalculado. |
| Transformación estructural posterior | Async | Estado de la misma familia de representación, pero protocolo D12 separado; planificador, colas y grupos Nexo en F5 sobre sistema/atómicos. No se convierte yield en tarea. |

**R17.** El compilador conserva la gramática, comprobaciones, expansión
estructural y metadatos de las últimas tres filas. «Escrito con la capa baja»
no significa que una biblioteca pueda inventar nuevas reglas de captura o
suspensión. El cuerpo resultante usa las mismas operaciones que un registro
y funciones Nexo equivalentes. Ningún backend implementa por separado un
formateador, una colección, UTF-8, Option o el protocolo de iteración. El
compilador self-hosted escribirá también esas transformaciones en Nexo;
antes de ello se reconoce que el compilador inicial está en Rust.

## 4. Catálogo cerrado de confianza

**R18.** El catálogo objetivo se llama `nexo-core-v1`. Contiene las familias
cerradas de la tabla; los parámetros finitos de una familia no autorizan
añadir opcodes. Las firmas de esta tabla son contratos de IR, no nuevas
producciones de fuente. Las funciones fuente se publican en `std.core`
mediante una interfaz tipada suministrada con la herramienta, visible para
cualquier paquete. Un usuario no puede fabricar una entrada por nombre,
atributo o ruta. Un homónimo es una declaración ordinaria. Todas las hojas
tienen identidad, versión, firma, efectos, precondiciones, procedencias,
layout de destino y semántica de fallo. No existe `ExecuteHost`, función
intrínseca arbitraria, syscall por número calculado ni escape de IR sin verificar.

| ID y operaciones cerradas | Contrato, efectos y límite |
|---|---|
| C01 `IntAdd, IntSub, IntMul, IntDiv, IntRem, IntNeg, IntCompare, IntConvert, BitAnd, BitOr, BitXor, BitNot, ShiftLeft, ShiftRight, RotateRight` | Enteros I/U8/16/32/64; M05 activa I64 y U8/U32/U64 con los modos y recetas de esta sección; las demás anchuras requieren sus vectores antes de activarse. Comportamiento entero D07/F5, incluido MIN%-1. Efectos propios vacíos. No variante unchecked pública. Operadores y SHA-256 usan estas hojas, sin opcode criptográfico. |
| C02 `BoolNot, ScalarBits, FloatOp, IntToFloat, FloatToInt` | Bool separado; observación/conversión de bits por tipo autorizado, nunca dirección↔entero. FloatOp limitado a add/sub/mul/div/neg/compare/sqrt para F32/F64, redondeo ties-to-even, sin contracción ni fast-math; preservar ceros y clases IEEE, NaN canónico quiet al producir resultado NaN, observación de bits de entrada intacta. Vacío. IntToFloat redondea ties-to-even; FloatToInt trunca hacia cero y devuelve Err para NaN, infinito o fuera de rango. Cubren I/U8/16/32/64 y F32/F64, sin cast UB. Funciones trascendentes son biblioteca. Esta concreción numérica exige su corpus antes de activar F32/F64. |
| C03 `LayoutOf` | Tamaño, alineación, offsets/tag de un tipo instanciado para target explícito. Puro comptime, sin leer layout del anfitrión. Error de análisis si no hay layout finito o tamaño representable. |
| C04 `Alloc, Free` | Hojas del backend declaradas en la TCB; reserva alineada comprobada. Alloc devuelve bloque sin T vivos o Err y tiene solo alloc; Free consume bloque vacío y tiene efectos vacíos. No block/io/suspend, como la asignación F0: latencia física no es el efecto block. El backend implementa reserva/liberación directa mediante la API VM del target, sin allocator Rust/libc en productos nativos, callbacks ni política de colección. El intérprete stage0 puede representar bloques con su allocator anfitrión Rust, identificado como infraestructura en R26; conserva exactamente flags, cuotas y fallos del modelo, sin trasladar esa dependencia al código nativo. Allocate aborta por OOM; TryAllocate devuelve Err. Alloc establece páginas de datos RW y guardas sin acceso al reservar; Free libera ambas. No expone cambio de protección posterior ni páginas ejecutables. macOS usa mmap/munmap (reserva PROT_NONE y mapping RW solo dentro de la reserva propia); Windows VirtualAlloc reserva y compromete únicamente los datos, dejando guardas sin compromiso. No requiere mprotect ni VirtualProtect para datos/guardas. |
| C05 `RawCapacity, RawOffset, RawBorrow, RawInit, RawTake, RawIsLive, RawDisown, RawAdopt` | Solo unsafe; índice/offset en la misma asignación viva y alineada, tamaño y origen conservados; Init requiere slot desarmado y toma T, Take requiere inicializado y lo desarma. Borrow devuelve RawView<T> con from del dueño; no fabrica independencia. Capacity devuelve el número inmutable de slots, incluidos los no vivos, sin leer T. IsLive consulta inicialización de un slot, no sus bytes. Disown/Adopt intercambian propiedad con un contrato externo de almacenamiento según R20, sin reservar, copiar ni cambiar bytes. Efectos propios vacíos, además de operaciones sobre T solicitadas por el llamador. Sin copia de bytes de T como sustituto de copia lógica. |
| C06 sin operaciones | RawReadByte/RawWriteByte no son hojas: RawAllocation<U8>/RawView<U8> usan C05 y lugares ordinarios con los mismos flags. No se reinterpreta T arbitrario como bytes. Copia masiva es bucle Nexo optimizable tras verificación. No se reutiliza C06. |
| C07 `AtomicLoad, AtomicStore, AtomicCas, AtomicFetchAdd, AtomicFetchSub, AtomicFence` | Solo enteros 32/64 y estado atomic explícito, alineación y soporte del target obligatorios. Orden único inicial seq_cst; CAS devuelve valor previo y éxito, sin fallo espurio. Sin alloc/block/suspend/io propios; no promete latencia ni progreso lock-free universal. Destino sin soporte se rechaza, no emula con mutex oculto. Saturación/desborde de contadores se maneja en biblioteca antes de reutilizar identidad. |
| C08 `ForeignCall` | ABI y símbolo de una entrada exacta de manifiesto, no dirección arbitraria. Solo unsafe; unión de efectos declarados y desconocidos, retención/afinidad explícitas; prohibido unwinding extranjero. Sin perfil no se autoriza retorno seguro ni ausencia de efectos. Perfiles C/Python son dependencias optativas externas, nunca std. |
| C09 `SystemCall` | Selector estático de R22, firma/target/perfil exactos. Solo unsafe, sin callbacks Nexo en v1. Retención de buffers de entrada limitada a la llamada síncrona. Los recursos devueltos tienen propiedad explícita; las entradas ejecutables prestadas de la ampliación H2 D17 R45 retienen únicamente su imagen/mapping dueño, nunca buffers ni rutas del llamador. Devuelve estado y datos crudos; validación/Result público en Nexo. Efectos conservadores por entrada. |
| C10 `Abort` | Termina el proceso; no retorno, no garantía de limpieza. No es Err ni Fail de F0. Causa estructurada ya preparada, sin asignación obligatoria para informar. Efectos propios vacíos; no implica recuperación. |

Los IDs concretos son `Cdd.Nombre[k=v,...]`, claves ASCII ordenadas y sin
espacios; los no parametrizados omiten corchetes. El manifiesto enumera cada
combinación válida: type=I8/I16/I32/I64/U8/U16/U32/U64, mode=checked/try/wrapping/saturating,
op para FloatOp, from/to para conversiones y type=F32/F64 cuando procede.
Las hojas de memoria/layout C03–C05 añaden `type=K`, con K codificado
según R19, para tipos nominales/instanciados; los escalares conservan sus
nombres ASCII. Ejemplo: `C01.IntAdd[mode=wrapping,type=U32]`. IntCompare exige op y no admite mode; no se
aceptan combinaciones no registradas. Cdd identifica familia, nunca llamada.

Las hojas y variantes siguientes son el catálogo numérico de M05, propiedad
de D16; D17 R13 las consume. Una variante tiene un ID completo distinto,
sin alias `C01.IntAdd` para una llamada parametrizada.

| Hoja / parámetros canónicos | Firma y semántica | Efectos y receta lógica |
|---|---|---|
| C01.IntAdd/IntSub/IntMul/IntNeg, `mode=checked/try/wrapping/saturating,type=I64/U8/U32/U64` | Mismo tipo de operandos/resultado; try devuelve Result<T,I64>. Checked conserva fallos aritméticos; wrapping reduce módulo 2^w, saturating limita al extremo representable. Neg unsigned aplica la misma regla a 0−x. | Vacíos; operación 1, resultado escalar 1; try añade wrapper 1. |
| C01.IntDiv/IntRem, `mode=checked/try,type=I64/U8/U32/U64`; C01.IntCompare, `op=eq/ne/lt/le/gt/ge,type=I64/U8/U32/U64` | División/resto truncados hacia cero; cero divisor falla o devuelve Err; MIN/−1 y MIN%−1 conservan F5. Compare devuelve Bool. Sin wrapping de división. | Vacíos; operación 1, resultado 1; try añade wrapper 1; comparación añade dos visitas escalares como A2. |
| C01.IntConvert, `from=I64/U8/U32/U64,mode=try/wrapping/saturating,to=I64/U8/U32/U64` | Try comprueba el rango destino y devuelve Ok(valor) o Err(1) fuera de rango; wrapping conserva w bits bajos (complemento a dos si destino firmado); saturating limita al rango destino. No hay variante IntConvert checked que aborte: la conversión comprobada es fallable y usa mode=try. | Vacíos; operación 1 y escalar 1; try añade wrapper 1 en éxito y error. |
| C01.BitAnd/BitOr/BitXor, `type=U8/U32/U64`; C01.BitNot, mismo parámetro | Dos operandos del mismo tipo, uno en Not; resultado bit a bit en exactamente w bits. | Vacíos; operación 1 y celda de resultado 1. |
| C01.ShiftLeft/ShiftRight, `type=U8/U32/U64` | (T,U32)→Result<T,I64>; cantidad < w, Err(1) en otro caso. Relleno con ceros; Left descarta bits altos, sin overflow aritmético. | Vacíos; operación 1, escalar de éxito/error 1 y wrapper 1. |
| C01.RotateRight, `type=U8/U32/U64` | (T,U32)→T; rotación por cantidad módulo w, incluido cero. | Vacíos; operación 1 y resultado 1. |
| C02.BoolNot; C02.ScalarBits, `type=U8/U32/U64` | Negación Bool; ScalarBits observa el patrón sin cambiar tipo, no convierte dirección en entero. Float y conversiones de flotantes siguen la fila C02 y no son prerrequisito B1. | Vacíos; operación 1 y resultado 1. |

Cada receta añade evaluación de argumentos y cargos ordinarios de llamada
A2; no vuelve a cobrar una operación que el operador fuente ya pagó.
El manifiesto enumera el producto finito permitido por cada fila en orden
ASCII de claves (por ejemplo `C01.IntConvert[from=I64,mode=try,to=U32]`);
no aplica `mode` a bits, rotaciones ni Compare. La representación de
FloatOp usa `op` antes de `type`, como exige ese mismo esquema.

D15 conserva exclusivamente literales decimales. M05 publica las funciones
de conversión `std.core.U8/U32/U64(n: view I64)->Result<Uw,I64>` sobre
IntConvert/try, sin constructor sintáctico especial. En una función que
devuelve Result con error I64, `let k = std.core.U32(1779033703)?;` crea
la constante U32 de SHA-256; las restantes constantes U32 caben en I64.
La evaluación comptime de esa misma conversión comprobada produce constantes
tipadas sin notación hexadecimal ni sufijos nuevos. Un U64 mayor que MAX_I64
se construye desde partes U32 convertidas y desplazadas, comprobando Result;
no se amplía subrepticiamente el rango léxico de IntLiteral.

**R19.** Constantes, registros, variantes, llamadas, permisos, transferencias,
selección, tags y planes de limpieza son operaciones **del núcleo IR**, no
funciones secretas de biblioteca. Array inline se baja a esas operaciones.
El núcleo puede insertar copia/destrucción estructural tipada y llamar hooks
Nexo; no implementa políticas de crecimiento o conversión de texto. Una
primitiva irreductible no puede llamar código Nexo salvo mediante una llamada
IR explícita comprobada; no se ocultan callbacks en C04/C09.

Firmas mínimas de memoria (notación de contrato: T y U son tipos del target;
los modos son los de B, no prefijos de tipo): Alloc recibe cantidad I64 y
alineación I64 comprobadas y devuelve Result<RawAllocation<T>,I64>; Free
recibe `move RawAllocation<T>` y devuelve unidad. RawInit recibe
`edit RawAllocation<T>, view I64, move T`; RawTake recibe
`edit RawAllocation<T>, view I64` y devuelve T; RawIsLive recibe
`view RawAllocation<T>, view I64` y devuelve Bool. RawCapacity recibe
`view RawAllocation<T>` y devuelve I64 no negativo, igual al n original de
Alloc, con ID `C05.RawCapacity[type=K]`, K identidad canónica de instancia
de T codificada como hex minúsculo de sus bytes canónicos; efecto vacío, una operación y una celda resultado. RawBorrow recibe dueño
prestado, offset y extensión comprobables y devuelve RawView<T> from dueño;
el permiso resultante nunca supera el solicitado al dueño. RawOffset
preserva el mismo dueño/rango, no permite formar un puntero fuera de la
asignación salvo la posición final no desreferenciable. RawDisown toma
RawAllocation<T> y entrega RawHandle<T>; RawAdopt recibe RawHandle<T> y
entrega RawAllocation<T>, con la obligación de unicidad R20. No hay
conversión genérica T→U por estas operaciones. RawView ofrece solo proyección
tipada a lugar dentro de un bloque unsafe, mediante el índice B `v[i]`:
0<=i<extensión, lectura o edición según el permiso conservado, sin extracción
que deje hueco mediante move. ReadPlace/CopyValue del núcleo realizan su
lectura, por lo que leer T conserva copia y efectos de T. La marca unsafe no
dispensa el permiso del dueño ni la vigencia de la vista. `v[i]` requiere
slot vivo incluso como destino: reemplaza y destruye un T vivo. Sobre slot
muerto solo RawInit instala y marca vivo después del compromiso; RawTake
transfiere y desarma atómicamente. Nunca se destruyen bytes no inicializados.

Los contadores y estados de sistema son enteros de anchura publicada;
ABI v1 no exporta tuples implícitas: los resultados múltiples, como CAS,
usan registros de campos ordenados publicados en la interfaz. El registrador
de declaraciones carga primero enums/structs de arranque (incluido Result)
sin ejecutar código y después resuelve las firmas del catálogo contra esas
identidades públicas. No necesita una std precompilada para resolver Result
ni permite que el backend fabrique una segunda unión homónima.

**R20.** RawAllocation<T> es un recurso único sin copia implícita. Conserva
base, tamaño, alineación, asignador/identidad de liberación y estado de slots.
El tamaño cero tiene representación válida sin dirección desreferenciable.
No permite comparar direcciones para observar identidad de un valor alto.
Free solo recibe una asignación sin T vivos; el descriptor de inicialización
es estado de ejecución obligatorio también en nativo optimizado: bitmap de
ceil(capacity/8) bytes inicialmente cero, propiedad de la asignación. IsLive
lo consulta y Init/Take lo actualizan. Bytes y accesos administrativos se
miden aparte del almacenamiento lógico. Su eliminación requiere prueba de
equivalencia de todas las consultas/salidas; no se presupone optimización.
La auditoría debe cubrir incluso caminos que el checker de préstamos no
pueda probar. Se prohíbe liberar mediante asignador distinto.

RawHandle<T> conserva identidad de asignación, generación, rango y alineación;
es transportable como dato crudo, pero ningún uso de sus bytes es seguro por
sí solo. RawDisown consume RawAllocation y transfiere su obligación de
liberación a la representación auditada, devolviendo RawHandle. RawAdopt
recupera un propietario único únicamente cuando el contrato demuestra que
no queda otro propietario, participación, préstamo o acceso incompatible.
No se puede adoptar memoria ajena ni inventar un handle a partir de I64.
El modelo instrumentado detecta doble adopción/liberación y generación
caducada. Una copia física de RawHandle no crea participación ni prolonga
vida: esa operación corresponde al código Nexo que mantiene el contador.
No tiene independencia, transferencia entre tareas ni SuspendSafe inferidas
por su dirección; las aporta, cuando proceda, el descriptor auditado del
contenedor. Usarlo fuera de ese contrato mantiene evidencia desconocida.

Esta es la representación elegida para CoW D09 en M13a: cabecera de conteo y
búfer en Nexo, participación mediante RawHandle; incremento/decremento local
en Nexo, atomics C07 solo en estado publicado. Antes de escribir se separa
el búfer si existen participaciones incompatibles. La última participación
recupera propiedad con RawAdopt y destruye; nunca cambia un contador local
a atómico mientras existen aliases activos. D09 gobierna publicación y
débiles; no se implementa RC dentro de RawDisown/RawAdopt ni en el compilador.

**R21.** `std.mem.Allocate<T>(n)`, `Read`, `Init` y `Take` usados en D16 §8 son
envoltorios Nexo del catálogo: Allocate llama Alloc y aplica la política OOM;
Read presta mediante RawBorrow y copia T por la operación normal, exigiendo
copiabilidad; Init/Take trasladan propiedad. Se declaran unsafe, síncronos,
con capacidad n no negativa y aritmética tamaño/alineación comprobada. Read
no proporciona un memcpy genérico. Las variantes fallables retornan el
Result ya definido en nivel 1, sin que construir un tag necesite asignador.
Liberación residual del recurso RawAllocation ejecuta la receta pública Nexo
`std.mem.DestroyRaw<T>(x: move RawAllocation<T>)`: primero ejecuta
`var y = move x;`. Dentro de un bloque unsafe obtiene
`var i = RawCapacity(y);`; mientras i>0 decrementa i y, si `RawIsLive(y, i)`, ejecuta
`let value = RawTake(edit y, i);` y `Drop(value);`. Al terminar llama
`Free(move y);`. El parámetro move es inmutable (A2 §4.3); solo y admite
edit. Capacity no inspecciona slots muertos y el bitmap R20 sigue disponible
en código optimizado salvo prueba que elimine también todas sus consultas. Esa receta se enlaza
por identidad de interfaz como los otros hooks, no por ruta privada. Así una
reserva parcialmente construida sigue teniendo limpieza aunque todavía no
exista un List completo. El bucle pertenece a Nexo; la semántica de flags
y traslado es del núcleo. La receta no solicita Drop de su propio receptor:
consume su asignación directamente mediante Free, desarmando la obligación.
Para cerrar el arranque se compila esta función antes de ejecutar cualquier
programa que reserve; su cuerpo solo necesita núcleo, C04/C05 y drop de T.

**R22.** Selectores de sistema v1: `FileOpen`,
`FileRead`, `FileWrite`, `FileClose`, `FileMakeDirectory`, `FileRename`, `FileKind`, `FileRemove`, `ConsoleRead`, `ConsoleWrite`, `Args`,
`ClockMonotonic`, `ClockWall`, `Entropy`, `SocketOpen`, `SocketBind`, `SocketListen`,
`SocketAccept`, `SocketConnect`, `SocketRecv`, `SocketSend`, `SocketClose`.
FileOpen/Read/Write/Close, FileMakeDirectory/FileRename/FileKind/FileRemove, ConsoleRead/Write,
Entropy y Socket* tienen io y
block conservador, sin suspend. Ambos relojes y la lectura de argumentos
ya capturados tienen solo io. Args
consulta cantidad/tamaño y copia a buffer U8 del llamador, sin reservar ni
llamar Nexo. Incluye el directorio absoluto inicial y entorno de lanzamiento
para el manifiesto del driver; no cambia el directorio global. Capturar ese
directorio añade block conservador, sin alloc/suspend; leer los argumentos
ya capturados conserva io. ClockWall devuelve segundos Unix y nanosegundos o Err, sin
monotonía, para validar vigencia TLS. VmReserve/VmRelease son mecanismos
privados de C04, no selectores; no hay dos rutas públicas de asignación. Las llamadas
de lectura/escritura reciben buffers y longitud explícitos; resultados de
error no autorizan leer bytes no inicializados. Éxitos parciales devuelven
cantidad exacta, EOF es distinto de error, y las interrupciones no se
reintentan en el compilador. Entropy falla explícitamente, nunca inventa bytes.

El manifiesto por target fija entrada del kernel o API **nativa del sistema**,
convención, anchuras, codificación de rutas, códigos de estado y límites;
Linux usa su interfaz de sistema y Windows sus APIs del sistema sin CRT/libc.
macOS usa la lista permitida de D17 §6 «Backend propio y puerta de extremo a extremo», R40, incluida su frontera libSystem; que un proveedor sea
parte del sistema no autoriza usar su parser JSON, TLS, colecciones o libc
como biblioteca estándar encubierta. Los manifiestos exactos por arquitectura
son entregables M11/M15: sin uno verificado, la entrada no es utilizable.
No se asegura una ABI universal ni se eligen números de syscall portables.
FileMakeDirectory y FileRename reciben rutas prestadas validadas y devuelven
estado nativo I64, sin reserva propia; los wrappers Nexo lo convierten en
Result<(),I64>. M11 implementa proveedores y wrappers. NativePath en Linux
conserva bytes arbitrarios sin NUL, sin exigir UTF-8; macOS sigue la
codificación de su manifiesto y Windows UTF-16LE sin NUL. Convertir una
String a ruta es una operación Nexo distinta, fallable cuando corresponda.
La creación recursiva de directorios es bucle Nexo; rename exige mismo
volumen y no copia silenciosamente. FileRename reemplaza atómicamente un
archivo destino ordinario en el mismo volumen; rechaza destino directorio,
y no promete durabilidad. Windows usa MOVEFILE_REPLACE_EXISTING, nunca
MOVEFILE_COPY_ALLOWED. FileMakeDirectory acepta un directorio existente
solo tras FileKind=Directory; un enlace, archivo u otra clase da Err. No
promete atomicidad entre esa consulta y operaciones posteriores ante cambios
externos del sistema de archivos. R40 de D17 fija mkdir/rename y
CreateDirectoryW/MoveFileExW como proveedores.

`C09.SystemCall[selector=FileKind]` recibe NativePath prestado y consulta
solo la clase de la entrada final sin seguir enlaces: devuelve registro
crudo {status: I64, kind: I64}, con códigos 0=Regular, 1=Directory, 2=Link,
3=Other únicamente con status=0. El wrapper Nexo devuelve
Result<FileEntryKind,I64>. Tiene io/block, sin alloc/suspend. Proveedores:
Linux newfstatat/fstatat según ISA con AT_SYMLINK_NOFOLLOW, macOS lstat,
Windows GetFileAttributesW (todo reparse point se clasifica Link). Solo se
expone la clase; tamaño, seek y flush no quedan habilitados por esta hoja.
Receta: argumentos/llamada, una operación y dos celdas de registro crudo;
wrapper enum y Result cobran una celda cada uno, sin doble cargo de estado.
M11 verifica archivo, directorio, enlace y ausencia por destino; Other se
prueba con entrada especial en Linux/macOS. Windows no produce Other: limita
FileKind a rutas del sistema de archivos y rechaza espacios de dispositivos
antes de consultar, según D17 R40; su negativo especial verifica ese rechazo.

`C09.SystemCall[selector=FileRemove]` recibe ruta nativa prestada validada
y kind de enum FileRemoveKind (File/EmptyDirectory); devuelve estado nativo
I64, que el wrapper Nexo transforma en Result<(),I64>, con io/block
y sin alloc/suspend propios. Elimina una sola entrada, nunca recorre hijos
ni sigue un enlace final para borrar su destino; EmptyDirectory exige un
directorio vacío. El wrapper consulta FileKind y rechaza Link antes de
intentar EmptyDirectory; la hoja nunca elimina el destino de un enlace.
Ante sustitución externa entre consulta y borrado, solo puede eliminarse la
entrada final, sin garantía de identidad entre operaciones de ruta. Ausencia, permisos, clase incompatible
y directorio no vacío son Err con estado nativo, sin reintento ni éxito
ficticio. No exige cerrar handles ajenos ni promete borrado físico inmediato.
La receta lógica cobra argumentos y llamada ordinarios, una operación,
una celda de estado; el wrapper cobra unidad y Result o error y Result; validación y
conversión de ruta en Nexo cobran sus recorridos, sin trabajo gratuito.
M11 entrega proveedores unlink/rmdir (Linux por syscall, macOS por libSystem)
y DeleteFileW/RemoveDirectoryW (Windows), wrappers y casos archivo, vacío,
no vacío, ausente, enlace, permisos y error sin eliminar otra entrada. El
driver aplica FileRemove únicamente a sus temporales enumerados, en orden
inverso de creación, sin limpieza recursiva de rutas ajenas. FileKind se
limita a clasificación; metadatos generales, seek y flush no se habilitan: lectura/escritura secuencial, longitudes verificadas
por lectura y publicación por rename bastan; no se promete durabilidad ante
caída del sistema. Los proveedores de ConsoleRead/Write en Windows reciben
buffers U8 alineados a dos bytes para unidades UTF-16LE de consola y buffers
U8 para redirección; no exige activar U16. La firma por target recibe ambos
buffers del llamador y devuelve estado, clase de buffer y longitud cruda;
esta longitud cuenta unidades UTF-16 en consola, bytes en redirección. La biblioteca Nexo
convierte UTF-8/UTF-16 y conserva longitudes/parciales sin depender de la
página de códigos. GetConsoleMode/ReadConsoleW/WriteConsoleW se rigen por
D17 R40. La obtención inicial del directorio usa getcwd/GetCurrentDirectoryW
en buffers del llamador; no usa FileKind para obtener el directorio actual.
TLS, DNS, framing HTTP, buffering y texto son algoritmos Nexo. Ampliar los
selectores, por ejemplo para tareas o carga ejecutable, exige otra revisión
del catálogo antes de F5/H2; no hay selector «otros».
Los símbolos pthread/CreateThread/SRW/TLS de hilo/WSA y los símbolos BSD
socket/bind/listen/accept/connect/recv/send/shutdown/getsockopt/setsockopt
de macOS marcados F5 en D17 R40
solo se habilitan tras la revisión de catálogo de M15 que enumere sus
selectores, firmas, efectos, costes y proveedores; su lista no concede una
capacidad ejecutable anticipada.

**R23.** El catálogo de F0 sigue separado por propósito: A2 §8.4, J §6,
F5 §§4/6/7 y K §§3/6/7 son **implementaciones provisionales**, no entradas
adicionales permanentes de nexo-core-v1. MockOpen, MockBlockingOpen, Inspect,
sus cierres, las cuatro colas A2, MockUnknown, Print/Fail/AssertEq y consola
K solo pertenecen al entorno de pruebas. Sus nombres, cuotas y eventos
siguen vigentes durante F0. MockUnknown mantiene efectos desconocidos aunque
no haga nada. Nunca se enlazan mocks en std de producción.

## 5. Distribución, identidad y arranque

**R24.** std se publica primero como paquete local inmutable dentro de la
distribución privada: fuente `.n`, manifiesto, interfaz derivada, lockfile,
catálogo requerido, target y edición Unicode. El inventario R15 y el valor
esperado de `package_digest` de D17 R10 son registros externos al paquete;
el resolvedor/lockfile del consumidor guarda ese valor esperado. El lockfile
distribuido no contiene el digest de su propio paquete.
Los bytes cubiertos y las exclusiones son exclusivamente los de esa tabla.
Una herramienta selecciona exactamente una revisión std por grafo conforme
D13 §11. Las revisiones incompatibles fallan; no se carga «la más reciente».
SemVer describe compatibilidad de API, mientras que package_digest fija los bytes.
Un cambio de etiquetas, variantes, efectos, procedencias o semántica observable
es cambio de API aunque conserve el nombre. F4 añade publicación pública
de paquetes únicamente cuando esté autorizada; este ADR no la autoriza.

**R25.** `std` es un alias reservado resuelto por el manifiesto de herramienta,
no un directorio privado del compilador. El preámbulo exporta identidades de
Option/Result/List/Iterator y traits seleccionados de ese paquete. La
selección usa IDs públicos de definición de biblioteca, firma y versión;
no la grafía, rutas del disco, índices de interning ni una whitelist de
archivos fuente. Compilar un paquete sin preámbulo implícito es una operación
de arranque del constructor de std, con las mismas reglas y catálogo público;
no deshabilita comprobaciones ni añade un dialecto. Los elementos del
catálogo se suministran como interfaz de herramienta, no como paquete Nexo
que dependa de std, evitando un ciclo de paquetes.
El enum Nexo Option/Result conserva constructor, aridad y tags publicados;
`?` requiere la identidad Result exportada por el paquete resuelto. Un enum
de usuario llamado Result no obtiene propagación por homonimia. La comprobación
de esa identidad procede de la interfaz, no de un tipo Rust especial.

**R26.** La construcción reproducible de std desde un directorio vacío y
aislado solo recibe compilador stage0, fuente std, catálogo, manifiestos de
target y herramientas explícitas de enlace. No lee src del compilador,
cabeceras privadas Rust, archivos de otro checkout ni binarios de std
preexistentes. La prueba debe poder renombrar el checkout del compilador y
seguir construyendo. Otra biblioteca de usuario puede invocar todas las APIs
bajas públicas con los mismos permisos; no necesita parchear la herramienta.
El inventario de símbolos/imports de un binario nativo debe contener solo
programa Nexo, std Nexo, código de máquina generado y la lista de APIs del
sistema autorizadas por D17 R40, incluida libSystem en macOS.
No se admite enlazar libc fuera de esa lista, CRT, libgcc,
compiler-rt o el allocator Rust para completar una operación de biblioteca.
Una expansión escalar que requiera un helper usa fuente Nexo o instrucciones
del backend; si falta soporte, el destino se rechaza. Rust/std del intérprete
stage0 permanece identificado como infraestructura anfitriona, no como
dependencia nativa de std.

**R27.** D17 §4 «Semilla y cadena de tres generaciones» y D17 §10 «Subtareas y
dependencias» poseen el calendario: N04/N05 antes de cerrar F1 y N13 antes
de cerrar F3, sujetos a sus pivotes explícitos. D16 entrega biblioteca e
intrínsecas; no fija otra semilla ni cadena stage1–3. M12 entrega std a N06/N07c;
M16 remite a N03–N13 y M17 a N14–N23, incluida recarga, según D17 R04.

**R28.** D17 §6 «Backend propio y puerta de extremo a extremo» gobierna
LLVM inicial, backend propio y retirada. Toda cadena usa la misma biblioteca
Nexo y catálogo. D17 es único dueño de las enmiendas a D04 y A2 §9.4;
D16 no establece otra ABI ni calendario nativo.

## 6. Migración única de los cortes

**R29.** Durante F0 terminan A2 → J → F5 → K con sus intrínsecas previstas.
D16 añade en F0 únicamente diseño e inventario M01. N01 concilia ese
contrato antes de N02; N02 implementa IR/ABI al inicio de F1 y M02/M03
consumen la entrega después, antes de congelar el núcleo F1. No interrumpe esos cortes para implementar un allocator, paquetes
o genéricos incompletos. Las migraciones se ejecutan en F1, antes de congelar
su núcleo; generadores mantienen su alcance reducido hasta F2.

| Corte de sustitución | Prerrequisitos completos | Sustitución y retirada en el mismo cambio |
|---|---|---|
| M08 | M04, genéricos/módulos de F1; antes de M05 | Option/Result fuente y descriptor nominal único; se retiran constructores/tipos especiales del checker salvo identidades públicas del preámbulo. `?` conserva lowering de núcleo sobre Result resuelto. Match no se reimplementa. |
| M09 | M05–M07 y M08 | Array dinámico→List Nexo, Array inline, Bytes y asignadores. Se retiran Push/Pop/JoinArrays/copia de array dinámico del intérprete. Literales y fixtures migran; no alias ni selector de perfil antiguo. |
| M09a | M09; traits F1 | Operadores por traits Nexo sobre C01/C02; se retira despacho numérico alto, sin cambiar cortocircuito del núcleo. |
| M09b | M09a | HashMap, búsqueda e internado Nexo por IDs; único propietario de algoritmos y política de colisiones. |
| M09c | M09b | Mergesort estable y serialización ordenada; N06 solo integra estas bibliotecas. |
| M10 | M09c | String, UTF-8, parseo, formato, derivaciones e Iterator/adaptadores en Nexo. Se retiran handlers String/Format/PartialEq e Iterator.Next especial. Closure/gen se normalizan a registros y funciones; se retiran sus dispatchers especiales cuando las mismas rutas ordinarias cubran el corte. |
| M11 | M10 | Consola/archivos reales Nexo sobre sistema. Las colas simuladas pasan a un proveedor de pruebas explícito de la misma interfaz de E/S; se retiran handlers de API alta. El proveedor solo entrega datos/estados crudos y eventos, nunca interpreta texto/colecciones. |
| M11a | M11 | Cierre fallable explícito y consumo del handle también en Err; se elimina cualquier segundo cierre residual. |

**R30.** Una sustitución tiene un único punto de activación: fuente nueva,
preámbulo, descriptores, lowering, fixtures, cuotas e informes cambian juntos.
El ejecutable resultante no incorpora el handler retirado, fallback al AST,
tabla privada equivalente o bandera de compatibilidad. Los binarios anteriores
solo son oráculos archivados externos a la distribución para la revisión.
No se valida una implementación nueva porque llame por debajo a la antigua.
Un defecto bloquea la sustitución; no se publican ambos caminos.

**R31.** Dominio común: programas y entradas del perfil A2/J/F5/K anterior
traducibles por identidad de símbolos a la API sustituta con el mismo perfil
lógico de cuotas y proveedor controlado. Incluye rechazos, cuotas y cotas de
efectos; excluye las capacidades nuevas (Array inline, memoria cruda, E/S
real), probadas aparte. Equivalencia exige en ese dominio:
misma aceptación/rechazo, código y subcausa, ubicación causal, orden de
evaluación, valores, error tipado, permisos, eventos y orden de limpieza;
mismos efectos conocidos/desconocidos y mismas cotas cumplidas o rechazadas.
Las diferencias autorizadas son únicamente las siguientes, transcritas antes
de ejecutar comparaciones:

1. Renombrado Array dinámico→List y JoinArrays→JoinLists, incluido texto de
   tipos y offsets afectados, mediante mapa por símbolos, nunca regex.
2. Nueva identidad nominal de biblioteca, versión de perfil y modelo; un
   constructor conserva su tag, orden y ruta diagnóstica `.some/.ok/.err`.
3. Causa hoja `intrinsic.call` de una operación retirada pasa a
   `function.call` con su identidad Nexo; se añade la cadena al origen
   irreductible. El primario del usuario no se desplaza a std. Esto es una
   enmienda de representación diagnóstica, no igualdad byte a byte ficticia.
4. E/S real sustituye colas solo en M11; equivalencia previa se comprueba
   con el mismo proveedor controlado, sin equiparar un dispositivo real
   no determinista a una plantilla JSON.
5. Solo en M11 ReadLine pasa de `{alloc,io}` a `{alloc,io,block}` y
   Print/Println reales añaden block por ConsoleWrite: filas de consola
   de K §6. El proveedor controlado conserva los efectos F0 antes de M11.
   C04/Allocate conservan alloc; Free/DestroyRaw solo añaden drop de T.
   Push, copia y destrucción de List/String no adquieren block por memoria.
   `fn F(a: edit List<I64>) with no_block { Push(edit a, 1); }` acepta.

No se permite normalizar desaparecidos errores, efectos, campos de propiedad
o eventos para obtener igualdad. La representación de diagnósticos históricos
solo se traduce en el comparador externo, nunca mediante adaptador del producto.

**R32.** Costes visibles durante M08–M10: copia profunda, ausencia de copia al
mover, crecimiento `max(1,2*C)`, capacidad tras Pop, pico de coexistencia de
búferes, longitud UTF-8 y número de búferes String conservan F5/K. Se conserva
la atribución lógica de trabajo y cuotas del corpus migrado mediante un
`CostOrigin` por operación semántica pública. El origen tiene receta versionada
de cargos de A2/J/F5/K; las instrucciones administrativas de expansión no
vuelven a cobrar la misma operación. Cambiar de cuerpo no permite cambiar
esa receta sin enmienda de modelo y corpus. No se equiparan ticks con tiempo.

Las APIs nuevas de reserva cruda/capacidad explícita, fuera del dominio
heredado R31, cobran un tick por slot nuevo solicitado, además de los cargos
ordinarios de llamada y cabecera, antes de reservar. La petición cero no
inventa slots ni ticks por slot. Si forman parte de una expansión registrada
de List/Bytes, consume la receta pública correspondiente y su reserva W,
sin volver a cobrar el mismo hecho como una llamada cruda de usuario.
La receta incluye el barrido de slots vacíos y la destrucción futura; su
crédito se adquiere con el propietario y se transfiere al moverlo. D17 R50
consume esta regla al derivar los acumulados de bootstrap-v1, sin modificar
los ticks A2/J/F5/K del dominio de equivalencia.

Los contadores de usuario son trabajo, call_frames, live_value_cells y
observation_bytes de A2 §11.2, con ampliaciones J/F5/K. Marcos, locales,
temporales y bitmap administrativos de biblioteca se excluyen de frames y
celdas de usuario; resultados y copias lógicas se cobran una sola vez. Un
callback de usuario vuelve a contadores de usuario. Clasificar expansión
requiere identidad y receta verificada, no el nombre std ni un atributo libre.

Cada receta r publica cotas W_r(q), F_r(q), C_r(q) de instrucciones, profundidad
adicional y celdas administrativas según tamaños/metadatos q. Incluyen helpers,
hooks concretos, fallos y limpieza. La derivación comprobada suma secuencias,
toma máximo de ramas, multiplica cotas de bucle y sustituye recetas de llamadas;
recursión requiere medida decreciente acotada por q. Sin prueba no se activa
la migración. Evaluar cotas usa metadatos mantenidos, no recorridos gratuitos.

library_work es acumulado; library_frames/library_cells son ocupaciones
separadas. Los límites se derivan por perfil: suma máxima de W sobre las
operaciones permitidas por las cuotas lógicas de trabajo/celdas y máximo de
F/C sobre pilas permitidas por call_frames. Se publican fórmula, parámetros
y entero resultante comprobado antes de activar cada perfil; library_work
no hereda la cuota de trabajo lógico de F5. Cota no finita/representable rechaza el
artefacto. Un cuerpo conforme nunca agota esas reservas antes que las cuotas
lógicas; superar una cota certificada es InvalidIr, no fallo de usuario.

Antes de la primera mutación pública se precarga la receta lógica completa
en orden acumulativo A2/J/F5/K (trabajo, frames, celdas,
bytes vivos/acumulados de array, tamaño/bytes vivos/asignaciones de texto,
observaciones) y W/F/C más limpieza.
Déficit lógico conserva código, observed y sitio originales: se calcula el
primer cargo normativo fallido sin ejecutar el prefijo mutante. Los efectos
de argumentos ya evaluados permanecen. Subcargos consumen la reserva sin
volver a comprobar cuota a mitad de traslado. Reserva física, overflow y
callbacks fallables preceden al desarmado de slots o usan almacenamiento
provisional; la fase de compromiso no falla. Limpieza tiene reserva propia
que el trabajo ordinario no puede agotar.

El perfil aislado `f1-library-test-work-v1` recibe del arnés, antes de iniciar
la ejecución, un parámetro obligatorio L entero con 0<=L<9.007.199.254.740.991
y L no mayor que la reserva normal certificada; fija library_work=L.
No hay valor por defecto ni cambio durante la ejecución. El manifiesto de
la prueba registra L y la receta/certificado usados; las demás cuotas son
las del perfil normal elegido. Con A unidades ya precargadas y una nueva
reserva W, se comprueba W<=L−A antes de mutar. El déficit emite R0002,
data={runtime_budget_kind:"library_work",limit:L,observed:L+1},
Diagnostic.profile_id="f1-library-test-work-v1", primario llamada pública
y demás campos R0002 de A2; no se añade data.profile_id. No es perfil de
equivalencia normal. No se restituye trabajo consumido ni reinicia por llamada.
V24 fija L=A+W para el positivo y L=A+W−1 para el negativo, con W>=1 y
A contabilizado desde el inicio, incluida la construcción de los argumentos;
ambos valores deben cumplir el rango anterior. La limpieza ya reservada en A
se conserva. El arnés transcribe A/W desde el certificado, no los obtiene
ajustando el límite después de observar el fallo.
Frames/celdas administrativos se informan, sin nuevos valores R0002.
`bootstrap-v1` también admite R0002/library_work en precarga; sus cifras,
ámbito por ejecución y derivación por suma máxima de W se fijan únicamente
en D17 R50. No introduce un techo menor que esa suma certificada ni reinicia
el acumulado por llamada, archivo o fase del compilador. R0002 conserva
acumulativamente las cuotas de texto K y arrays F5 además de las de A2.
Fuera del perfil de inyección, agotar library_work con saldo lógico y
certificado correcto no es un resultado permitido de una receta conforme.

**R33.** Las recetas de coste se incluyen en el artefacto público std y
quedan cubiertas por package_digest de R24/D17 R10. Su modelo versionado
se identifica por perfil/versión semántica de la clave de artefacto D17 R10;
no añade campos a interface_hash. Solo asignan cargos a operaciones identificadas por
el contrato del lenguaje. No permiten que un paquete declare coste cero a
su código: fuera de la expansión registrada se cobran instrucciones ordinarias.
La instrumentación de auditoría cuenta asignaciones, bytes, copias y cierres
desde las hojas, independientemente de las recetas. Discrepancia bloquea la
migración. El rendimiento físico se mide aparte, con target/configuración;
no se exige igualdad de RSS, direcciones, instrucciones de máquina ni tiempos
contra Rust. CoW F2 modifica explícitamente el modelo físico y sus informes;
no se disfraza como equivalencia exacta de las cifras F0.

## 7. Invariantes de IR, ABI y checker

**R34.** M02 es un consumidor posterior a N01/N02 en F1, no requisito
retroactivo de I04–I07. FunctionId(String) y hash FNV-1a del modelo
actual se sustituyen por DefKey, body_hash/interface_hash/dependency_hash y clave de artefacto de D17 §2 «Contratos que
deben quedar fijados antes de F2», R09–R10. N02 es el único dueño e implementador de tabla de funciones, descriptores
de layout, IR01/IR02 y ABI; depende solo de N01 y M01. Retira String/FNV y
las representaciones previas en su propio corte. M02 aporta vectores de
consumo std y verifica integración; M05 baja operaciones contra esos
descriptores, sin otra tabla ni formato. No se afirma que el estado actual ya cumpla esos contratos.

Enmienda de alcance: A2 §17 «Orden de implementación», filas I04–I07,
conserva su dominio implementado y recibe esta migración posterior. J §6
«Un único motor y contrato de IR», F5 §6 «IR, verificación y un único motor»
y K §7 «Núcleo de tipos, IR y verificación» reciben las mismas identidades
y metadatos al integrar M02/N02, sin atribuirlos a su diseño aprobado ni
exigir reimplementaciones por corte. `unknown` es admisible exclusivamente
para capacidades diferidas enumeradas (layout físico, confianza aún no
activada); antes de ejecutar una capacidad todos sus datos son obligatorios.
La conformidad se informa por dominio; un unknown fuera de él no invalida
un corte anterior, ni cuenta como evidencia de la capacidad futura. IR03–IR11
son obligaciones de consumo de biblioteca: std debe aportar o conservar esos
datos al usar la IR única de D17; no definen una IR, solver o verificador
alternativos ni transfieren su implementación a un M.

| Invariante de consumo por biblioteca | Obligación precisa |
|---|---|
| IR01 Identidad | std y sus hooks usan únicamente DefKey e instancias definidas por D17 R09; sin claves derivadas de rutas privadas o nombres de handlers. |
| IR02 Integridad | std consume exclusivamente la definición y la clave de artefacto de D17 R10; no define campos adicionales ni excepciones. |
| IR03 Tipos | Una sola tabla TypeId/descriptores para builtin, usuario y std. Never, incógnita y contaminación separados. Propiedades transitivas sobre DAG/SCC resueltas con presupuestos; lowering consume la tabla, no vuelve a inferir. |
| IR04 Lugares | Raíz lógica con instancia de activación, proyecciones FieldId/índices evaluados, origen, LoanId y permiso padre. Copia/movimiento/retorno/RawView conservan todos los orígenes posibles; ninguna conversión a dirección borra procedencia antes de verificar. |
| IR05 Vida | Flags por slot/componente y plan explícito por salida: retorno, Err, break/continue, fallo de simulador y suspensión. Preparación de argumentos en orden escrito; instalación en orden de parámetros; limpieza inversa correspondiente. Nunca doble propietario al trasladar payload. |
| IR06 Efectos | Known/unknown separados; resúmenes de helpers, hooks, temporales y limpieza incluidos antes y después de lowering. create/copy/call/drop de closure, create/resume/drop de gen y cuatro operaciones async permanecen distintas. |
| IR07 Control | CFG único de caminos alcanzables e hipotéticos, guardas y retroaristas. Una guarda fallida no mueve; yield tiene continuación. Un solver agotado no entrega VerifiedIr ni prueba de rechazo definitivo por propiedad. |
| IR08 Layout | Descriptor versionado por target: tamaño/alineación, campos, tag/payload, flags y representación de préstamo. Estado `abstract` explícito en F0; `concrete` obligatorio antes de emitir nativo o llamar LayoutOf. Sin layout circular inline ni dependencia del enum Rust anfitrión. |
| IR09 Confianza | Cada operación insegura lleva sitio de permiso, contrato de hoja, procedencia, supuestos y módulo. Verificador comprueba cobertura; no acepta un booleano del lowering como prueba de seguridad. |
| IR10 Costes y origen | OriginId y CostOrigin conservan sitio de usuario y aristas hacia expansión/biblioteca. Un solo cargo por hecho lógico; contador independiente R32 para trabajo nuevo. Un helper sin origen o receta es IR inválida. |
| IR11 Frontera ejecutable | Solo artefacto opaco verificado, con perfil, interfaces, catálogo y body_hash/interface_hash/dependency_hash. Check/run comparten frontend/checker; ni AST ni IR arbitraria son entradas ejecutables públicas. |

**R35.** La ABI es propiedad exclusiva de D17 §2 «Contratos que deben
quedar fijados antes de F2», R08. std entrega firmas, modos, procedencias,
layouts lógicos y efectos de hooks a ese descriptor, y exige interface_hash
y clave de artefacto registrados iguales a los recomputados según D17 R10,
con catálogo y target compatibles al enlazar. No adapta layouts antiguos ni
mantiene ABI propia. C08 usa el perfil extranjero explícito de D17.

El contexto de asignación de biblioteca es un descriptor tipado propietario
o prestado `AllocationContext<A>`: identidad del proveedor A, estado, límites
y procedencia de almacenamiento; nunca un puntero entero o global mutable.
A implementa el contrato Allocate/Free con mismo origen e identidad en cada
bloque. El contexto por defecto es Heap sobre C04, sin estado ambiental;
variantes con asignador reciben el contexto explícito y lo retienen mientras
vivan sus bloques. Los efectos se especializan por A; un contexto de arena
no promete capacidades de regiones F3. Su préstamo vive al menos como cada
asignación que lo usa. M09 entrega Heap/contexto y pruebas de mezcla de
proveedores; D17 R08.5 decide únicamente su transporte oculto en IR cuando
la superficie cómoda lo elide, dejando dependencia y efectos en la firma.

**R36.** Closure concreta = entorno inline de campos ordenados más identidad
de código/capacidad Call*. Sin boxing implícito. Generador concreto = estado
inline de tamaño conocido, continuación, campos vivos y planes de limpieza;
Create no ejecuta cuerpo, Resume conserva suspend, Drop no suspende. Las
rutas internas son lógicas, relativas al estado, para que moverlo no deje
punteros autocontenidos inválidos. Un backend que materialice direcciones
debe fijar almacenamiento o reconstruirlas desde rutas; cualquier reserva
se declara. body_hash/interface_hash/dependency_hash de esta representación se calculan exclusivamente según D17 R10.
La máquina de estados baja al núcleo ordinario y sus funciones; no se conserva
una VM específica para generadores.

**R37.** Biblioteca y hooks publican dependencias necesarias para invalidar
consumidores. La tabla de funciones base pertenece a D17 N02; N14 añade sus generaciones
H1/H2 según D17 R45, sin duplicarla. H0 y recarga/reinicio pertenecen a
D17 §7 «Recarga en caliente». Una revisión std nunca migra
objetos vivos por nombre, body_hash o cast unsafe; consume la decisión
del controlador D17. D16 no entrega otro cargador ni otra tabla de generaciones.

## 8. Esbozos en B y su contrato

**R38.** Los dos bloques siguientes son módulos de diseño, no programas
admitidos por el F0 actual. Usan la extensión exacta R09 y APIs R21. El padre
declara `unsafe mod storage;`; el archivo `storage.n` contiene el primer
bloque. Los importados mem.Read/Init/Take son funciones unsafe; Allocate
aplica la política infalible/OOM. Allocate publica solo alloc; CopyList y
PushOne de I64 satisfacen no_block. R32 precarga la operación pública antes
de mutar; los bucles del esbozo no introducen fallos intermedios de cuota.

El primer esbozo muestra la separación crecimiento/elementos. Es un List
monomórfico de I64 para no ocultar reglas genéricas. CopyList/DropList se
definen en el mismo módulo por el segundo bloque. M09 generaliza mediante T
y sus propiedades; no conserva una implementación I64 paralela en std.

```nexo
use std.mem;

@storage(element: I64, copy: CopyList, drop: DropList)
struct ListI64 {
    slots: mem.RawAllocation<I64>;
    length: I64;
    capacity: I64;
}

fn Empty() -> ListI64 {
    unsafe {
        return ListI64(slots: mem.Allocate<I64>(0), length: 0, capacity: 0);
    }
}

fn PushOne(x: edit ListI64, item: view I64) {
    if x.length == x.capacity {
        let next = if x.capacity == 0 then 1 else x.capacity * 2;
        unsafe {
            var replacement = mem.Allocate<I64>(next);
            var i = 0;
            while i < x.length {
                let v = mem.Take(edit x.slots, i);
                mem.Init(edit replacement, i, v);
                i = i + 1;
            }
            x.slots = move replacement;
        }
        x.capacity = next;
    }
    let position = x.length;
    unsafe {
        mem.Init(edit x.slots, position, item);
    }
    x.length = position + 1;
}
```

**R39.** La multiplicación y suma del esbozo son comprobadas; un tamaño no
representable termina por la política de reserva, no hace wrap. En producción
M09 reemplaza ese cálculo por la comprobación de tamaño de biblioteca antes
de consumir elementos. El crecimiento transfiere I64, no lo copia; reemplazar
slots libera el bloque anterior ya vacío. No se lee x.length dentro de una
llamada que tenga prestado x.slots exclusivamente: se evalúa el límite fuera
de esa llamada. No se solicita disjunción de campos para estos ejemplos.

```nexo
fn CopyList(x: view ListI64) -> ListI64 with no_io, no_suspend {
    unsafe {
        var target = mem.Allocate<I64>(x.length);
        var i = 0;
        while i < x.length {
            let v = mem.Read(x.slots, i);
            mem.Init(edit target, i, v);
            i = i + 1;
        }
        return ListI64(slots: move target, length: x.length, capacity: x.length);
    }
}

fn DropList(x: edit ListI64) with no_suspend {
    var i = x.length;
    while i > 0 {
        i = i - 1;
        unsafe {
            let value = mem.Take(edit x.slots, i);
            Drop(value);
        }
    }
    x.length = 0;
}

fn Exercise() -> I64 {
    var a = Empty();
    PushOne(edit a, 7);
    var b = a;
    PushOne(edit b, 9);
    return a.length;
}
```

**R40.** Exercise devuelve 1; b tiene dos elementos y a uno. La copia tiene
capacidad igual a longitud, como F5; el destructor recorre en orden inverso.
Para T con destructor, Take devuelve un temporal propietario y Drop aplica
su limpieza. Una implementación debe hacer transaccional la instalación de
cada slot y registrar progreso parcial; el esbozo I64 no certifica ese caso.
String se construye por la misma receta sobre U8: Bytes propietario más
validación UTF-8 antes de construir el nominal, campos privados, copia de
bytes, SliceBytes validando ambos extremos, formato que escribe a sink.
No usa un intrínseco String detrás de la fachada Nexo. La ausencia de un
segundo esbozo String evita fingir ya disponible la API de Bytes en K.

## 9. Diagnósticos propuestos y matriz de aceptación

**R41.** [F0-CODIGOS](F0-CODIGOS.md) sigue siendo el único registro. Antes
de emitir se integran allí las siguientes entradas propuestas del área 16,
sin ocupar NX-E12xx–E15xx ni R05xx. Los perfiles existentes conservan
schema_version=1; el perfil de herramienta H1/H2 usa la ampliación D17 R47.
Los namespaces existentes se conservan. No se asignan códigos a fallos internos de IR, auditorías o
comparaciones diferenciales. R0002 conserva la unión A2/J/F5/K y añade library_work en los perfiles
R32/D17 R50, mediante las sustituciones acumulativas de D16 §12.1.

| Código / subcausas cerradas | Etapa / operación | Primario y data exacta |
|---|---|---|
| NX-E1601 / `missing_unsafe_block`, `module_not_unsafe`, `suspending_trust_scope`, `unsafe_function_value`, `async_unsafe_function` | ownership / ownership.check para permiso/valor; effects / effects.check para suspending_trust_scope; types / types.check para async_unsafe_function | Nombre del destino unsafe sin bloque; token unsafe del bloque/fn o atributo storage sin módulo autorizado; destino que suspende (incluido unknown) o token await/yield; nombre de función unsafe usado como valor; token async/gen de una declaración unsafe asíncrona/generadora, respectivamente. Data `{}`. |
| NX-E1602 / `unknown_primitive`, `primitive_contract_mismatch`, `target_primitive_unavailable` | types / types.check | Destino resuelto del catálogo o import de interfaz inválida; data `{"primitive_id":ID}`. ID es hoja canónica de R18, `Cdd.Nombre` con parámetros cuando corresponda; dos dígitos y Nombre ASCII PascalCase. Solo unknown_primitive admite ID ausente del catálogo, por ejemplo C99.Absent; las otras dos exigen miembro registrado. |
| NX-E1603 / `invalid_storage_shape`, `invalid_storage_hook`, `storage_copy_effect`, `mixed_variant_payloads` | types / types.check para forma/firma/payload; effects / effects.check para efecto | Tipo del campo prohibido; nombre de hook del atributo; nombre C del atributo; primer payload que mezcla formas según R09, respectivamente. Data `{}`. |
| NX-E1604 / `artifact_hash_mismatch`, `library_integrity_mismatch`, `catalog_version_mismatch` | names / names.resolve | Import que requiere el artefacto incompatible, o EOF de raíz si es std implícito. Data `{}`. |

NX-E1604 se comprueba en este orden: library_integrity_mismatch compara
bytes de paquete con package_digest esperado R24/D17 R10 antes de parsear su fuente
importada (el import del consumidor ya está resuelto); catalog_version_mismatch
compara la versión requerida con la disponible; artifact_hash_mismatch
compara interface_hash y clave de artefacto registrados con los recomputados
desde los registros canónicos D17 R10, una vez superadas las dos barreras
anteriores. No significa simplemente que dos revisiones válidas tengan
interfaces distintas. Mutar IR ya verificada en memoria es InvalidIr, no
un diagnóstico de importación.

Las hojas checked C01 conservan R0101 para suma, R0103 para resta,
R0501 para multiplicación, R0502 para división por cero, R0503 para cociente
no representable, R0504 para resto por cero y R0505 para negación no
representable. Al activar anchuras nuevas, la misma condición se evalúa
en su rango; MIN%−1 sigue dando cero según F5. IntConvert/try y shifts
devuelven Err(1) en rango inválido, sin reutilizar un código aritmético ajeno; un acceso seguro fuera de límites conserva R0102. OriginId apunta
al operador o selector `[indice]` original, no al cuerpo de std; los related
spans/etapa/operación y datos son los de A2 §13.12/A2 §13.15 y F5 §8. El chequeo
público precede a C05; incumplir una precondición unsafe en el modelo es
InvalidIr, no se falsea como R0102 de un acceso seguro. Try devuelve Err y
no un diagnóstico aritmético R. Los fallos R del simulador desenrollan con limpieza reservada R32;
no se implementan invocando C10. Abort/OOM irrecuperable D02 termina proceso
sin ejecutar ni garantizar destructores: no hay promesa de cierre tras Abort.
El fixture diferencial separa R0101/R0103/R0501–R0505 y R0102 con cierre único de Abort sin
expectativa de cierre, también en código optimizado.

**R42.** Para las entradas nuevas, subcause es siempre una de las enumeradas;
feature_id=null, campos de parser/token null y listas de tokens
vacías; no se inventa unexpected_token para un error semántico. Cause_chain
y repair son []; el informe de confianza R15 proporciona el grafo completo
sin alterar ese esquema. Related_spans usa únicamente `declaration` para
destino/hook escrito, `contract` para atributo storage o cota escrita R43, `context` para
declaración `unsafe mod`. Se incluyen solo las relaciones aplicables, con orden y deduplicación de A2 §13.6 (archivo, inicio, fin y role),
sin prioridad por clase de relación; sin fuente física disponible se omite. Missing_unsafe_block relaciona destino si tiene declaración fuente;
module_not_unsafe no fabrica un unsafe mod inexistente. Storage_copy_effect
relaciona la operación que origina el efecto como declaration. Los campos
comunes restantes siguen A2 §13.6. Mensaje traducido no forma parte del oráculo.

**R43.** La selección de barreras sigue A2/J/K: parser, nombres, asociación,
tipos, propiedad, efectos, lowering. Forma de storage inválida precede a
hooks derivados; sin permiso no se diagnostica escape derivado del acceso
rechazado. Una cota escrita sigue NX-E0412–0416 y sus datos/cadenas originales;
no se reemplaza por NX-E1601 salvo la prohibición intrínseca de suspensión
del ámbito de confianza, que prevalece para ese mismo sitio y relaciona la
cota escrita. Copiar recurso sigue NX-E0201; escapar un préstamo sigue el
código específico D11/D12/D13; privacidad conserva NX-E1308; afirmar Call*,
SuspendSafe o Send mediante impl conserva NX-E0110. Errores independientes
se conservan, no se selecciona por número menor de código.

**R44.** En F0 se reconocen las formas nuevas para alcance, sin ejecutarlas:
unsafe mod, unsafe fn/bloque y storage usan
attributes/later_adr, en parse/syntax.parse, primario unsafe o @storage
completo. Se propone ampliar la descripción de attributes para incluir
estos permisos de declaración/ámbito; el mensaje humano será «Permisos de
confianza fuera del perfil», no «atributo unsafe». Se conserva el ID histórico
para no alterar R46/V23, sin añadir un feature_id ni afirmar
que unsafe sea un atributo en la gramática. RawAllocation/RawHandle/RawView/Slice/Bytes
siguen unsupported_type/later_adr. Array con length sigue traits_generics
en el perfil F0, List escrito sigue unsupported_type. Los errores de sintaxis
reales preceden a alcance. Las capacidades permanentes fuera de F0 **nunca**
usan implementation_limit, ni M08 como pending_scope F0. M01 es inventario F0 y M02/M03 son consumidores F1, no barreras por programa. Cada futuro corte
F1 publicará sus capacidades temporales antes de implementarlas; D16 no
añade pendientes ficticios al perfil de J/F5/K.

**R45.** PASS/FAIL en esta tabla son **resultados esperados**, no veredictos
obtenidos. Cada fila se materializa con fuente B válida del perfil indicado,
entorno y única mutación descrita. Las pruebas de artefactos no se cuentan
como negativos fuente. Para spans exactos se usa la convención literal:
`⟦fragmento⟧` indica primario, las marcas no son bytes de entrada; el
intervalo es [bytes anteriores, bytes anteriores+UTF8(fragmento)). La fuente
completa de cada fixture queda fijada antes del emisor, sin consultar sus
diagnósticos ni el impresor para obtener offsets.

| ID / regla | PASS esperado | FAIL aislado y diagnóstico esperado |
|---|---|---|
| V01 R04/R10 | Lectura segura de Array bajo no_alloc | Añadir copia de List en ese cuerpo: NX-E0412, known_effect, value.copy y origen de copia, no permiso sugerido. |
| V02 R08 | `unsafe { mem.Read(block, 0); }` dentro de módulo autorizado | Quitar únicamente bloque: NX-E1601/missing_unsafe_block, primario `Read`, data {}, relación declaration si existe. |
| V03 R08 | Declaración padre unsafe mod para V02 positivo | Quitar unsafe solo del padre: NX-E1601/module_not_unsafe en `unsafe` del bloque, sin context ficticio. |
| V04 R10 | Closure cuyo cuerpo seguro no requiere permiso | Dentro de un bloque unsafe, construir closure con llamada a Read sin bloque propio: NX-E1601/missing_unsafe_block en Read interior. Caso separado: usar función unsafe como valor da NX-E1601/unsafe_function_value sobre su nombre. |
| V05 R10/D12 | Terminar permiso antes de llamar a generador | Llamar Next que suspende dentro del bloque: NX-E1601/suspending_trust_scope en Next, aun siendo síncrono. |
| V06 R11 | Vista cruda conservada dentro de duración de dueño | Retornarla tras destruir dueño: NX-E1318, origen real y salida; no aceptación por unsafe. Closure portadora conserva NX-E1104 prioritario. |
| V07 R12 | Storage con I64, metadatos escalares y RawAllocation<I64> | Añadir campo MockResource como único cambio: NX-E1603/invalid_storage_shape sobre MockResource, contract del atributo. |
| V08 R13 | CopyList con firma view S→S | Cambiar resultado a I64: NX-E1603/invalid_storage_hook sobre CopyList del atributo. |
| V09 R13 | Hook de copia sin io/suspend, con o sin block declarado | Añadir llamada io a C sin cota escrita para aislar obligación storage: NX-E1603/storage_copy_effect en nombre C del atributo, declaration de llamada. |
| V10 R14 | List de recurso se mueve | Copiar ese binding: NX-E0201; usar después de move: NX-E0202/definitely_consumed. Son dos negativos distintos. |
| V11 R18 | Entrada C07 soportada por target | Cambiar solo target a uno sin anchura atomic requerida: NX-E1602/target_primitive_unavailable y primitive_id exacto. |
| V12 R18 | Interfaz C05 con firma publicada | Mutar firma de interfaz importada: NX-E1602/primitive_contract_mismatch; un Name desconocido ordinario sigue NX-E0101, no unknown_primitive. |
| V13 R24 | Paquete std con package_digest y versión exactos | En la fuente ASCII `//a` sin LF final del paquete, sustituir únicamente a por b (byte 2) manteniendo package_digest esperado: NX-E1604/library_integrity_mismatch en import/EOF; ambas fuentes son comentarios válidos. Alterar versión de catálogo: caso separado catalog_version_mismatch. |
| V14 R25 | Reexportación pública de función baja segura | Acceso a miembro privado por ruta compiler.internal: NX-E1308/private_access si existe; NX-E0101 si no existe. Nunca ruta privilegiada std. |
| V15 R31 | Corpus migrado con biyección declarada | Cualquier cambio no autorizado de código/subcausa/evento/efecto: FAIL de equivalencia, sin NX-E inventado. |
| V16 R32 | Push con C=1,N=1 alcanza C'=2; pico de tres slots | Mutar crecimiento a +1: FAIL de costes a partir de C=2; cambiar Pop para reducir capacidad: FAIL de cuotas, aunque valores coincidan. |
| V17 IR04/IR05 | Transferencia conserva origen y desarma emisor | Quitar LoanId o flag mediante mutación IR: rechazo interno de verificador, nunca ejecución ni NX-E de usuario. |
| V18 IR02/IR08 | Insertar una línea de trivia antes de F: body_hash/interface_hash/clave de artefacto iguales y source_map_hash distinto; package_digest y file_digest de la fuente cambian y se actualiza la integridad esperada del paquete de desarrollo, con la misma revisión | Artefacto de interfaz: alterar solo interface_hash registrado, conservando los registros canónicos; recalcular package_digest y fijarlo como esperado. NX-E1604/artifact_hash_mismatch al importar, data={}, primario import/EOF y campos R42; la integridad R24 sí coincide. Caso aislado separado altera solo la clave de artefacto registrada con igual actualización exterior. Restaurar el valor recomputado acepta; mutación interna ya cargada es InvalidIr. |
| V19 R36 | Crear y descartar gen sin avanzar: sin eventos de cuerpo | Ejecutar cuerpo en Create, borrar suspend o duplicar Drop: FAIL del modelo/verificador y regresión; no nuevo código fuente. |
| V20 R06 | List crece; Array con length 2 conserva extensión | Llamar Push con Array: NX-E0102/type_mismatch sobre argumento, esperado List instanciado. Nunca crecimiento implícito. |
| V21 R07 | SliceBytes con extremos UTF-8 válidos | Extremo interior: Err(-9111) de K durante migración, diagnostics=[]; bytes malformados en constructor fallable no forman String seguro. |
| V22 R26 | Build std aislado con interfaces públicas | Retirar checkout privado del compilador no puede romper build; si rompe, FAIL arquitectónico, no error del usuario. |
| V23 R44 | Función ordinaria F0 ya admitida | `unsafe fn F(){}`: NX-E0006, feature_id=attributes, pending_scope=later_adr, primario unsafe [0,6), sin token ni relaciones. |
| V24 R32 | Precarga completa con saldo exacto W para Push que crece de 2 a 4 | Saldo W−1 con L=A+W−1 fijado antes de ejecutar según R32: R0002/library_work, data.limit=L, data.observed=L+1 y Diagnostic.profile_id=f1-library-test-work-v1 antes de Take; longitud/capacidad 2 y dos slots vivos intactos; limpieza del llamador toma cada elemento exactamente una vez, sin slot muerto. |

**R46.** Microfixtures literales adicionales, sin LF final:
`fn F(){⟦unsafe⟧{}}` en F0 da NX-E0006/attributes, subcause null,
data `{"pending_scope":"later_adr"}`, primario [7,13), parse/syntax.parse.
Su positivo elimina el único UnsafeStmt y conserva `fn F(){{}}`.
En F1 un módulo ordinario que contiene la misma fuente da
NX-E1601/module_not_unsafe en [7,13), data {}, relaciones=[];
el positivo cambia únicamente la declaración padre a unsafe mod.
La fuente literal `⟦unsafe⟧ mod m;` en F0 da NX-E0006/attributes,
subcause=null, parse/syntax.parse, primario [0,6), data únicamente
{"pending_scope":"later_adr"}, sin tokens, relaciones ni cadenas. La primera
exclusión D15 §3.6 es unsafe; quitarlo deja la exclusión independiente de
mod/modules_visibility sobre [0,3), no un positivo de módulos F0. En F1 con
módulos y archivo m.n presentes, `mod m;` y `unsafe mod m;` aceptan.
`pub unsafe mod m;` en F0 conserva primero pub/modules_visibility.
En módulo unsafe F1, `unsafe async fn F(){}` da
NX-E1601/async_unsafe_function sobre async [7,12), types/types.check,
data={}, sin relaciones; positivo `unsafe fn F(){}`. La variante gen
usa la misma subcausa sobre gen; suspensión transitiva dentro de un cuerpo
síncrono se comprueba posteriormente en effects como suspending_trust_scope.
`enum E{V(a:I64,I64)}` da NX-E1603/mixed_variant_payloads sobre el segundo
I64 [15,18); positivo `enum E{V(a:I64,b:I64)}`.
`enum E{V(a:I64,a:I64)}` da NX-E0104/null, names/names.declare,
data={"declaration_kind":"field"}, primario segundo a [15,16) y
previous_declaration primero [9,10); positivo cambia solo segundo a a b.
Estos ejemplos no necesitan Main para check de biblioteca y no fallan por
una entrada ejecutable ausente. NX-E1602/unknown_primitive se prueba
mediante una interfaz de catálogo con ID ajeno, señalando la referencia
que la importa; no mediante una función fuente inexistente.

## 10. Validación medible

**R47.** Cada R e IR tiene requisito trazable, positivo y negativo aislado
cuando es regla de rechazo, o contraejemplo de modelo cuando es obligación
de implementación. El informe independiente usa PASA/FALLA/NO EVALUADO/
INCONCLUSO, con fuente, revisión, entradas, salida y evidencia. No transforma
falta de implementación en PASS. La puerta de dos revisores sin discrepancias
y la aprobación de fases por el propietario permanecen pendientes.

**R48.** El validador construye su referencia con `git archive HEAD`, compara
check/run con su propia batería y conserva artefactos anteriores fuera del
producto para R31. Los esperados se transcriben de este ADR y de A2/J/F5/K,
no del sistema bajo prueba. Por regla se exige mutación inversa que elimine
o invierta la comprobación/obligación correspondiente y haga fallar al menos
un caso pertinente; restauración por file_digest (SHA-256 de cada archivo, D17 R10). En cada ronda
se reejecutan supervivientes y como máximo doce mutaciones nuevas, bloqueantes
solo para reglas explícitas. Los defectos corregidos dejan regresión nominal.
Un error de sintaxis accidental no prueba confianza, propiedad ni efectos.

**R49.** Cobertura de memoria: tamaños 0/1/2, fronteras de alineación/tamaño,
fallo antes y después de cada instalación, crecimiento con coexistencia,
copia anidada, recursos sin copia, cierre parcial y reemplazo. El modelo
independiente mantiene asignaciones, rangos inicializados, préstamos, owners
y secuencia de cierre; se exige cero doble dueño, cero lectura de slot muerto,
cero acceso fuera de rango y limpieza exacta. Se inyectan fallos del proveedor
en cada punto de reserva/E/S sin depender del OOM real del anfitrión.
Fixture de cuota V24: List con dos recursos trazables recursoUno/recursoDos y capacity=2;
Push prepara recursoTres. El arnés fija A/W y L según R32; en déficit W−1 la lista no cambia, recursoTres se limpia como argumento
preparado y el llamador limpia recursoDos/recursoUno una vez; no hay Take de slot muerto.
Con W exacto, crecimiento termina con tres vivos y la salida limpia recursoTres/recursoDos/recursoUno.
Se repite inyectando déficit antes de cada subcargo: solo la precarga puede
rechazar; después del compromiso no se permite R0002 de biblioteca. Se prueba
además recursión de usuario con 253 y 256 frames llamando Push, y más de
3 millones de ticks lógicos de copias: helpers no agotan cuotas de usuario
y library_work se rige por la reserva certificada del perfil, separada del trabajo lógico.
En bootstrap-v1 se verifica además la derivación D17 R50: una receta
de copia que necesita al menos cinco instrucciones administrativas por tick
no puede agotar library_work tras 200.000.000.000 ticks manteniendo saldo
lógico. Se verifica esa desigualdad con contadores exactos y certificados.
Las fronteras de inyección se ejecutan con Diagnostic.profile_id=
f1-library-test-work-v1 y L=A+W para el positivo, L=A+W−1 para el negativo,
según R32; no cambian las cifras de bootstrap-v1 ni exigen ejecutar esa
cantidad para una regresión. Todos los acumulados,
incluidos Array y text_allocations, se contrastan por ejecución completa
según D17 R50 y no se reinician entre compilaciones internas de R26. Celdas
y bytes vivos miden ocupación; el aislamiento de cada cuota se prueba con
casos separados, nunca reiniciándola dentro de una ejecución. Las cuotas
de análisis siguen el ámbito de compilación/check externo de D17 R50.
Para código unsafe no se usa provocar UB nativo como prueba; se ejecuta
en el modelo instrumentado y se audita la correspondencia con backend.

**R50.** Texto: todos los bordes de secuencias UTF-8 de 1–4 bytes, truncamientos,
surrogates, overlong, máximo escalar, NUL, combinación sin normalización,
longitud cero, MIN/MAX I64 y sintaxis inválida tras overflow. Interpolación
evalúa una vez en orden y conserva efectos previos al error. E/S: lecturas
parciales, EOF/error, escrituras parciales, cierre una vez, cola agotada,
cancelación solo cuando corresponda D12. Ningún resultado se deduce del nombre
de fixture o de observar que un mock no bloquea.

**R51.** Escala mínima de cada migración: corpus completo previo migrado y
10.000 programas pequeños generados con semillas publicadas dentro del dominio
común, más 10.000 entradas malformadas/fuzz de fuente, interfaces y memoria
cruda en el modelo. Deben terminar por resultado o cuota, sin panic ni stack
overflow del anfitrión. No se promete cota de RSS por limitar ticks. Se prueban
límites −1/igual/+1 de todas las cuotas afectadas, con ocupación de las otras
para acreditar aislamiento. Los nombres de tests describen reglas, no rondas
ni recuentos de pasadas.

**R52.** Antes de cerrar F2: diferencial intérprete/nativo debug/optimizado
en la matriz de plataformas del roadmap, corpus T01/T02/T07/T08/T09/T17–T26
aplicable y 10.000 programas de F2. Para IR02 y paquete se prueban al menos
100 ediciones por clase: cuerpo, trivia, firma, efectos, layout, defaults,
procedencia, catálogo y target; se exige invalidación correcta sin ejecutar
código antiguo. Proyectos de 10.000 y 100.000 líneas funcionales siguen T26,
con diez builds limpios y memoria/tiempos desglosados. Se conservan sus
umbrales originales; D16 no inventa cifras competitivas nuevas ni sustituye
las puertas H-PY/H-RS/H-CS por el número de tests.

## 11. Subtareas y dependencias

**R53.** Los M identifican entregables pequeños dentro de esta decisión;
no son pending_scope de F0. Una dependencia externa de fase se comprueba
antes de comenzar. Las dependencias M apuntan a filas M anteriores del
orden topológico; las referencias N apuntan a entregas D17 anteriores en
el grafo conjunto R04, y las de fase/A2/J/F5/K son prerrequisitos externos;
un corte de sustitución se activa cuando todos sus componentes están listos,
sin publicar una convivencia temporal de implementaciones.

| Paso | Entregable comprobable | Dependencias | Momento límite |
|---|---|---|---|
| M01 | Inventario de intrínsecas, APIs y catálogo cerrado con vectores por hoja. | — | F0, antes de congelar F1. |
| M02 | Integración std y vectores consumidores de DefKey/body_hash/interface_hash/dependency_hash/clave de artefacto/tabla/layout/ABI implementados exclusivamente en N02. | M01; A2 I04; D17 N01,N02 | F1, después de N02 y antes de M03/congelar núcleo F1. |
| M03 | Auditoría de invariantes de consumo IR01–IR11 y unknown por dominio. | M02; A2 I05–I07, J/F5/K completos | Antes de congelar F1. |
| M04 | R09, diagnósticos, permisos y registro de declaraciones genéricas sin ejecutar std; orden canónico pub unsafe async gen fn y microfixtures R46. | M03; módulos/genéricos F1 | F1. |
| M08 | Option/Result nominales fuente, etiquetas v y descriptor único; retirar constructores intrínsecos. No requiere allocator ni std precompilada. | M04 | F1 antes de M05. |
| M05 | C01–C05/C07 y C10.Abort en modelo instrumentado, U8/U32/U64/bits y RawCapacity, lowering interno de Array inline y bitmap; consume tabla/layout/ABI N02. La aceptación pública de Array inline pertenece únicamente a M09. | M08; D17 N02 | F1, después de N02/M02; antes de M09. |
| M06 | Hooks genéricos storage, propiedades, DestroyRaw move y limpieza parcial; corpus de slots muertos. | M05 | F1. |
| M07 | Constructor del paquete std desde fuentes e interfaces, prueba aislada y manifiesto interno; sin semilla std previa. | M06; traits/comptime F1 | F1. |
| M09 | Heap/contexto, Bytes/List, aceptación pública de Array fijo y recetas W/F/C; retirar Array dinámico y handlers. Migrar en el mismo corte fuentes B0 de N03–N05 ya existentes, Scalars/firmas/fixtures por símbolos; si N04 o N05 se difieren hasta después de M09, entregan List directamente. | M07 | Antes de congelar F1. |
| M09a | Operadores por traits Nexo sobre C01/C02; retirar dispatch numérico alto. Corpus MIN/MAX, try/wrapping/saturating y efectos; cortocircuito sigue núcleo. | M09; traits F1 | Antes de congelar F1. |
| M09b | HashMap Nexo con Eq/Hash, colisiones resueltas y crecimiento; búsqueda/internado por IDs y SHA-256 Nexo sobre C01 para integridad; corpus de colisiones, copia/eliminación y vectores SHA-256 independientes. | M09a | Antes de N06/C0, F1. |
| M09c | Ordenación estable por mergesort Nexo, buffer explícito y comparador acotado; vectores vacío/duplicados/inverso y serialización determinista de mapas por claves ordenadas. | M09b | Antes de N06/C0, F1. |
| M10 | Texto/formato/derivación e Iterator Nexo; retirar handlers; normalizar closures/gen reducidos. | M09c | F1; gen completo en M13b. |
| M11 | Proveedores de consola/archivos/Args/relojes, FileMakeDirectory/FileRename/FileKind/FileRemove R22 con recetas y errores, UTF-8/UTF-16 de consola, APIs Nexo y proveedor controlado único. | M10 | F1, matriz nativa antes de cierre F2. |
| M11a | Close fallable explícito devuelve Result y consume recurso; residual no suspende ni duplica cierre. El handle queda consumido también en Err, que informa el fallo sin reintento implícito ni segundo close residual; pruebas EOF/parcial/error y cierre único por proveedor. | M11 | F1, antes de N06/C0. |
| M12 | Artefacto std verificable y revisión/file_digest de fuentes y binarios/receta S0-std (D17 R10/R20); consumo nativo, equivalencia de biblioteca y símbolos permitidos. No congela C0. | M11a | S0-std antes de F2; prueba nativa durante F2. |
| M13a | CoW local Nexo con separación, débiles/publicación y modelo de costes explícito; corpus D09, cero cambios observables por alias y cero duplicaciones con unicidad demostrada, incluso en hijos. | M12 | Antes de cerrar F2. |
| M13b | Generadores completos sobre representación común, Resume/Drop y corpus T17; cero ejecución en Create, cierre único por slot y cero dispatcher especial. | M13a; D11 F2 | Antes de cerrar F2. |
| M13c | C08.ForeignCall y puente Python externo optativo al enlace; pruebas de propiedad, errores y layout, sin biblioteca std extranjera. | M13b; ABI/interop F2 | Antes de cerrar F2. |
| M14a | Asignadores de región Nexo y vistas; rechazo de cada escape anidado y cierre inverso único por salida, sin bytes vivos al cerrar región; compilador de regiones F3 es dependencia externa. | M13c; regiones F3 | Antes de cerrar F3. |
| M14b | API de unicidad y evidencia D09; cero copias en camino único probado, separación ante débiles/aliases/publicación y negativos por cada precondición. | M14a | Antes de cerrar F3. |
| M14c | Contenedores AoS/SoA sobre layouts F3; mismos valores, permisos y orden de destrucción en ambos, incluidos vacío/cola/alineación y T10–T14 aplicables. | M14b; layouts F3 | Antes de cerrar F3. |
| M14d | Persistencia Nexo y derivación completa; versión anterior inalterada tras cada actualización, cierre único de compartidos y costes de peor caso publicados; sin afirmar representación heterogénea no cubierta por R12. | M14c; contrato revisado si procede R59 | Antes de cerrar F3. |
| M15 | Perfiles C consumidores de C08/M13c, C09.SystemCall[selector=Entropy], catálogo de red por target y HTTP/TLS Nexo auditada; catálogo futuro de tareas/sincronización/TLS de hilo. | M14d | F4/F5 según roadmap. |
| M16 | Referencia de consumo de std: autoalojamiento y retirada Rust pertenecen únicamente a N03–N13. | M12 para std; D17 N12/N13 para arranque | Calendario y pivotes D17. |
| M17 | Referencia de consumo del catálogo: backend/recarga pertenecen a N14–N23; no otro emisor ni runtime. | M16 para consumo std; sin dependencia adicional de N16 | N14–N23, incluida recarga, según calendario D17. |

El propietario de una capacidad es quien entrega su implementación, no
quien inventaría, integra o valida. Para el catálogo base, el mapa exhaustivo
por operación es el siguiente; cada expansión de una familia conserva el
mismo propietario. C06 carece de operaciones y no puede tener proveedor.

| Capacidades del catálogo base | M propietario único |
|---|---|
| C01.IntAdd, C01.IntSub, C01.IntMul, C01.IntDiv, C01.IntRem, C01.IntNeg, C01.IntCompare, C01.IntConvert, C01.BitAnd, C01.BitOr, C01.BitXor, C01.BitNot, C01.ShiftLeft, C01.ShiftRight, C01.RotateRight | M05 |
| C02.BoolNot, C02.ScalarBits, C02.FloatOp, C02.IntToFloat, C02.FloatToInt | M05 |
| C03.LayoutOf | M05 |
| C04.Alloc, C04.Free | M05 |
| C05.RawCapacity, C05.RawOffset, C05.RawBorrow, C05.RawInit, C05.RawTake, C05.RawIsLive, C05.RawDisown, C05.RawAdopt | M05 |
| C07.AtomicLoad, C07.AtomicStore, C07.AtomicCas, C07.AtomicFetchAdd, C07.AtomicFetchSub, C07.AtomicFence | M05 |
| C08.ForeignCall | M13c |
| C09.SystemCall, despacho tipado común; selectores FileOpen, FileRead, FileWrite, FileClose, FileMakeDirectory, FileRename, FileKind, FileRemove, ConsoleRead, ConsoleWrite, Args, ClockMonotonic, ClockWall | M11 |
| C09.SystemCall, selectores Entropy, SocketOpen, SocketBind, SocketListen, SocketAccept, SocketConnect, SocketRecv, SocketSend, SocketClose | M15 |
| C10.Abort | M05 |

Las ampliaciones tienen dueño único según D17 R04: ProcessRun, N07a;
ImageCreate/Seal/Reserve/MapFile/Entry/Protect/Unmap y
NativeImageOpen/Lookup/Close, N14a. Consumen el despacho C09 de M11 y no
constituyen otra implementación suya; no se reasignan a un M. Los futuros
selectores de tareas/sincronización/TLS de hilo pertenecen a M15 y siguen
inactivos hasta la revisión exigida por R22. Perfiles C de M15 consumen
ForeignCall de M13c. R40/R45 de D17 asignan cada proveedor a estas
capacidades; una API auxiliar solo puede usarse dentro de la receta de la
capacidad indicada, nunca como llamada general por su nombre.

El inventario verifica igualdad entre operaciones de R18/selectores de R22
y claves de este mapa, con cardinalidad de propietarios exactamente uno por
clave; C06 tiene cardinalidad cero de operaciones. Quitar únicamente el
dueño de Abort o Entropy, o asignarle un segundo M, rechaza el inventario.
Para cada símbolo de D17 R40/R45 exige una capacidad de este mapa o de las
extensiones con dueño N explícito; quitar esa atribución rechaza su
manifiesto, aun si el símbolo sigue en la lista permitida. Las entradas
condicionadas F5 no se cuentan como activas antes de su revisión R22.

M08 se conserva como ID pero se ejecuta antes de M05: «anterior» significa
fila anterior de este orden topológico, no número menor. M13/M14 designan
solo los grupos M13a–c/M14a–d, no tareas adicionales. N02 no depende de
terminar M02: recibe el inventario M01; M02 verifica su integración.

La única tabla normativa M↔N se define en D17 R04. R53 y D16 §12.2 la
consumen por referencia; no mantienen una copia con propietarios alternativos.

**R54.** M04–M11a no adelantan hilos, regiones de usuario, FFI general ni
layouts alternativos a F1. La memoria cruda interna de std y los layouts
necesarios para su representación son una ampliación explícita de diseño
de F1; no se presentan como regiones/layouts alternativos ya completos.
Los permisos y APIs públicas bajas que se activen comparten esas mismas
limitaciones. F2+ no es excusa para dejar intrínsecas de List/String en
producción después de sus migraciones F1.

## 12. Enmiendas propuestas y compatibilidad

**R55.** Cada cita siguiente es texto del documento indicado. El catálogo
cerrado R18 incorpora BitAnd, BitOr, BitXor, BitNot, ShiftLeft, ShiftRight,
RotateRight e IntConvert parametrizado; M05 entrega I64/U8/U32/U64 y sus
recetas. R58 de D17 registra el adelanto a F1 respecto de F2. La decisión
nueva reemplaza solo su alcance declarado; el archivo de autoridad permanece
intacto hasta su integración autorizada.

| Fuente y cita exacta | Enmienda propuesta |
|---|---|
| Roadmap P01: «El modo de ejecución, el nivel de optimización y el grado de control no cambian el significado del programa.» | Concreción R04–R11: capas son APIs/contratos, nunca dos semánticas. No se cambia P01. |
| Roadmap P07: «Las operaciones de bajo nivel que no pueda verificar el sistema se confinarán a módulos `unsafe` pequeños, identificables y revisables.» | R08–R15 fija módulo más permiso por sitio y contrato por función; la auditoría es visible y no se hereda por nombre. |
| Roadmap D08: «Las propiedades de seguridad —copia admisible, independencia transitiva, transferencia, proyección disjunta— permanecen selladas o derivadas estructuralmente por el compilador.» | Extensión R12–R14: derivación por elementos lógicos de una representación confiada auditada, con hooks Nexo. Impl ordinario sigue sin poder afirmar propiedades. La confianza no se etiqueta verificada. |
| D11 §6: «D08 mantiene estas propiedades selladas o derivadas, nunca afirmables por un `impl` ordinario.» | Se conserva para impl; storage es contrato explícito de frontera, no conformidad ordinaria ni permiso para Call*/SuspendSafe manuales. |
| Roadmap F1: «El intérprete puede usar copias físicas sencillas para representar la semántica.» | Las copias físicas continúan, pero sus algoritmos de biblioteca serán Nexo antes de congelar F1. Compilador conserva solo copia estructural y hojas irreductibles. |
| Roadmap F1: «Hilos, regiones de usuario, FFI general, layouts alternativos y generación de código nativo.» | Conservar exclusiones; añadir memoria cruda auditada y layout interno para biblioteca F1, sin declarar entregadas las capacidades excluidas. |
| Roadmap §6.7: ««Incluido» significa disponible mediante `std` y probado con la versión del lenguaje; no que el equipo deba escribir desde cero cada parser, backend TLS o primitiva de sistema.» | Sustituir permiso de dependencia de biblioteca no Nexo por fuente Nexo propia o reutilizada con licencia/procedencia. Solo sistema y catálogo irreductible quedan exceptuados. |
| Roadmap §6.7: «El cliente HTTPS usa un proveedor TLS mantenido con validación de certificados y hostname por defecto; no se implementa criptografía propia para cerrar la fase.» | El proveedor debe estar escrito en Nexo, mantenido y auditado; se conserva validación por defecto y se prohíbe improvisar criptografía para cerrar una fase. Si no existe, se desarrolla/porta y audita con evidencia antes de cerrar la puerta; no se acepta TLS extranjero dentro de std ni se retira HTTPS del compromiso. |
| F5 §4.1: «Todos los arrays pueden crecer; no se añade FixedArray, List ni adaptación entre tipos.» | Se conserva exclusivamente en F0 hasta M09. Luego R06 sustituye de una vez nombres y operaciones dinámicas por List y define Array inline. No queda compatibilidad. |
| J §3.1: «Option es una unión nominal intrínseca de `Some(T)` y `None`, con un tag y, solo para Some, un payload.» | M08 conserva semántica/tags y sustituye declaración intrínseca por enum Nexo e identidad de biblioteca; descriptor común intacto. |
| K §3: «Todas las operaciones siguientes tienen identidades del manifiesto y tipos fijados en el checker.» | Vigente durante F0. M10 resuelve firmas desde interfaz pública Nexo; no hardcodea operaciones String en checker/intérprete. |
| K §5.2: «La derivación genera planes estructurales tipados en la tabla de tipos y operaciones IR verificables; no genera código fuente que se vuelva a parsear.» | M10 sustituye planes cerrados por recetas Nexo comptime y expansión tipada sobre la misma representación; no reparseo de cadenas ni segundo checker. |
| 0003 §10.1, fila Reservadas reconocidas pero no aceptadas, lista literal `and or not async await region struct enum type trait impl where dyn mod use pub extend extension comptime const self super package` | Sustituir la lista completa por `and or not async await region struct enum type trait impl where dyn mod use pub extend extension comptime const self super package unsafe`. Se conservan las activaciones de las demás palabras por los cortes posteriores; unsafe se reconoce para NX-E0006/attributes en F0 y se acepta en M04 según R09/R44/R46. storage y element siguen siendo Name contextuales del atributo, nunca reservadas globales. |
| D15 §2.2: «Los restantes identificadores son Name» | unsafe pasa a reservada y se añaden R09/R44. No hay nuevas grafías de B fuera de la gramática exacta propuesta. |
| F0-CODIGOS: «Los huecos no se asignan por inferencia ni se reutilizan.» | Insertar explícitamente NX-E1601–1604 y sus datos/etapas; no apropiarse de 12–15 ni R05. Revalidar unicidad al integrar, antes de emitir. |
| A2 §13.6: «`data` admite únicamente las claves registradas más abajo.» | Añadir primitive_id solo a NX-E1602; ampliar acumulativamente runtime_budget_kind según D16 §12.1 con library_work para R32/D17 R50 y descripción de attributes para R44. D16 no añade otros campos; esta fila y la fila A2 §13.6 de D17 R58 son acumulativas y disjuntas; las ampliaciones de recarga de D17 R47/D17 §12 se integran acumulativamente, sin sustituir estas claves ni las heredadas A2/J/F5/K. |

D04 y A2 §9.4 se remiten exclusivamente a D17 §12 «Enmiendas propuestas
y alternativas rechazadas». No hay texto sustitutorio de esas reglas en D16.

### 12.1. Sustituciones textuales adicionales

Las siguientes sustituciones se activan con su M, no cambian el perfil F0
anterior por anticipación. Las citas son literales y las filas afectadas
se enumeran; las demás reglas de cada sección permanecen.

| Fuente y cita exacta | Redacción sustituta y filas afectadas |
|---|---|
| D15 §2.2, fila Contextuales, siempre token Name, lista literal `view edit move copy value from result_from retains as in create call resume cancel drop unrestricted no_alloc no_heap_alloc no_block no_suspend no_io capacity executor local migrable layout derive` | Sustituir la lista completa por `view edit move copy value from result_from retains as in create call resume cancel drop unrestricted no_alloc no_heap_alloc no_block no_suspend no_io capacity executor local migrable layout derive storage element`. storage y element son contextuales exclusivamente en StorageAttribute de R09; fuera de él siguen siendo Name ordinarios. La sustitución es acumulativa con las activaciones de 0003/0004/0005 y con la lista de reservadas de R55: no retira términos ni convierte estos dos Name en reservadas globales. M04 entrega esta fila y R09; reconocer el atributo en F0 conserva R44. |
| F0-CODIGOS, tabla feature_id, attributes: «Atributos, incluido @derive.» | «Atributos, incluido @derive, y permisos de confianza unsafe mod/unsafe fn/unsafe bloque reconocidos fuera del perfil; unsafe no es un atributo gramatical.» Afecta fila attributes y su explicación; conserva el ID, etapa parse/syntax.parse y microfixtures R46/V23. unsafe mod empieza por unsafe/attributes; pub previo conserva modules_visibility, según primera exclusión D15 §3.6. |
| F0-CODIGOS, Runtime experimental, fila R0002: «Se excede una cuota estructural de ejecución. runtime_budget_kind pertenece exclusivamente a call_frames, live_value_cells u observation_bytes. Exige limit, observed, profile_id y span de la operación desencadenante. No utiliza budget_kind.» | Sustituir por: «Se excede una cuota de ejecución. runtime_budget_kind pertenece exclusivamente a call_frames, live_value_cells, observation_bytes, array_storage_bytes, array_allocation_bytes, string_bytes, live_text_bytes, text_allocations o library_work. Exige limit, observed, Diagnostic.profile_id y span de la operación desencadenante; no utiliza budget_kind ni data.profile_id.» Unión acumulativa de F5 §12 y K §13, no reemplazo por los tres valores A2. library_work solo se activa en f1-library-test-work-v1 y bootstrap-v1; observed=limit+1 y precarga R32. Los otros observed conservan total candidato o limit+1 según su regla original. |
| A2 §13.6, fila runtime_budget_kind: «`call_frames`, `live_value_cells` u `observation_bytes`; obligatorio para R0002.»; F5 §12: «array_storage_bytes o array_allocation_bytes»; K §13: «añadir string_bytes, live_text_bytes y text_allocations solo para R0002, además de array_storage_bytes/array_allocation_bytes heredados de F5.» | La fila enumera exactamente call_frames, live_value_cells, observation_bytes, array_storage_bytes, array_allocation_bytes, string_bytes, live_text_bytes, text_allocations y library_work, obligatorio solo para R0002. La integración conserva las ocho cuotas acumuladas y añade library_work, sin retirar texto ni arrays. |
| F0-CODIGOS, Explicaciones y perfil de F0-0002: «Runtime_budget_kind es un campo independiente, cerrado a call_frames, live_value_cells y observation_bytes para R0002. No amplía ni reutiliza la enumeración budget_kind de NX-E0003.» | Sustituir su enumeración por la unión de nueve nombres de la fila anterior, en perfiles D16/bootstrap-v1; conservar independencia respecto de budget_kind, namespaces y perfiles históricos. |
| Roadmap D07 «Semántica adicional que debe congelarse en F0»: «Las primitivas realmente no comprobadas quedan en `unsafe`, con obligación de rango documentada, y no se añaden como sugerencia automática.» | «El catálogo aritmético ofrece checked, try, wrapping y saturating; no expone variantes unchecked, ni siquiera en unsafe. El backend solo elimina comprobaciones tras prueba de rango que preserve el contrato. No se sugieren permisos automáticamente.» Afecta esa frase de D07 y C01; conserva mediciones F2/F3/F6 y separación de dominios modulares/comprobados. |
| K §3 «Texto: valor, acceso y operaciones», fila: `Scalars(self: view) -> Array<I64>`; contrato «Valores numéricos de escalares en orden; materializa explícitamente un array independiente.» | En M09 la firma es `Scalars(self: view) -> List<I64>` y el contrato «Valores numéricos de escalares en orden; materializa explícitamente una lista independiente.» Conserva alloc. Afecta fila Scalars, párrafo de iteración materializada K §3, contabilidad Scalars/Array de K §8.2 y fixtures de Scalars de K §§10–11 por el mapa de R31; no cambia valores ni UTF-8. M09 actualiza el descriptor todavía intrínseco; M10 lo sustituye por fuente Nexo. |
| K §9.1 «Tabla cerrada de alcance», fila traits_generics: «Array/Result/Option intrínsecos conservan su aridad.» | «Hasta M09 Array/Result/Option intrínsecos conservan su aridad F0. La forma completa Array con length nombrado se reconoce como traits_generics/later_adr; después de M09 Array exige tipo y length, y omitir length da NX-E0111/missing_argument según D16 R09. Option/Result mantienen aridades.» Afecta esa fila, frontera Array de R44 y diagnóstico de aplicación genérica; no convierte Array<I64> en List implícitamente. |
| K §4.1 «Identidad y construcción»: «Los payloads de enum son posicionales; etiquetas se rechazan con unknown_argument_label.» | «Cada variante tiene todos los payloads sin nombre o todos nombrados; mezclar usa NX-E1603/mixed_variant_payloads. Los anónimos rechazan etiquetas con unknown_argument_label; los nombrados admiten asociación posicional seguida de etiquetas según D16 R09. Nombres duplicados usan NX-E0104/declaration_kind=field; argumentos duplicados conservan NX-E0111/duplicate_argument.» Afecta esta frase, descriptor de variante y asociación de constructores; conserva los negativos K de variantes sin nombre. M08 declara v en Some/Ok/Err y conserva los positivos etiquetados de J §9.4. |
| D15 §3.5 «B completa: formas diferidas, con grafía fijada»: `Variant       = Name ["(" Type {"," Type} ")"] ;` | Sustituir esa producción por Variant/VariantPayload de R09; añadir alternativa const a GenericParam de R09. TypeArg/ConstTypeArg ya admiten argumento nombrado y no se modifican. La aplicación de Array y hooks genéricos se rige por R09/R13. |
| K §6, filas `Print(v: view T) -> ()`, `std.console.ReadLine() -> Result<Option<String>,I64>`, `std.console.Println(s: view String) -> Result<(),I64>`: efectos «io», «alloc, io», «io», respectivamente | En M11 sustituir efectos propios por io/block, alloc/io/block e io/block, respectivamente, según R31.5/C09. Antes de M11 conservan literalmente los efectos del proveedor simulado. Se conservan los efectos transitivos de Format y limpieza. |
| K §9.1, fila attributes: «Atributo distinto de derive Format/PartialEq, incluido Eq/Hash/JsonEncode.» | Añadir «y permisos unsafe mod/fn/bloque reconocidos; primario primer unsafe no precedido por otra exclusión; attributes/later_adr, parse/syntax.parse». No se retiran los rechazos restantes ni se trata unsafe como atributo gramatical. |
| D15 §5.1.7: «Modificadores completos en orden `pub async gen fn`, reconociendo la combinación inválida para diagnóstico.» | Orden `pub unsafe async gen fn`; conservar reconocimiento completo y diagnosticar combinaciones unsafe async/gen según R41/R46. El impresor no borra modificadores inválidos. |
| A2 §13.12, R0101/R0103; F5 §8.1, R0501–R0505 (sin cambiar su condición en I64) | Al activar las anchuras de C01, aplicar las mismas condiciones por operación en el rango declarado según R41, sin colisiones ni nuevo código. IntConvert/try y shifts fallan con Err(1). |
| D16 R18, catálogo nexo-core-v1 | C01 incluye BitAnd, BitOr, BitXor, BitNot, ShiftLeft, ShiftRight, RotateRight e IntConvert con mode definido; I64/U8/U32/U64 y conversión decimal ordinaria en M05. D17 R58 registra el adelanto F1; C05 añade RawCapacity y R21 fija receptor mutable. |

### 12.2. Contratos compartidos con D17

[D17](F0-D17-autoalojamiento-y-backend.md) define IR/ABI, arranque,
backend, recarga y las enmiendas a D04/A2 §9.4. Estas referencias son de
consumo; no proponen contratos paralelos.

| Contrato de D17 | Consumo por D16 |
|---|---|
| R04 y N02/N06 | Única tabla M↔N. N02 implementa tabla de funciones, layouts, IR01/IR02 y ABI desde N01/M01; M02/M05 consumen después. M09b/M09c/M11a implementan biblioteca; N06 integra y verifica. |
| R10 | Única tabla de package_digest, file_digest, manifest_digest, source_map_hash, body_hash, interface_hash, dependency_hash y clave de artefacto; IR02/R36 la citan sin redefinir campos. |
| R13 y R20–R29 | M12 entrega S0-std a N06 y artefacto nativo a N07c/N12; C0 y retirada pertenecen a N11/N13. M13a–c alimentan N07c; M16/M17 solo remiten al mapa R04. |
| R40/R45 | Única lista de APIs nativas y contrato de generaciones H1/H2. C04 usa VM; no introduce selector público VmReserve ni otro cargador. |
| R50 | Perfil bootstrap-v1, cifras y derivación W/F/C; R32/R49 conservan precarga y aislamiento de contadores. |

ProcessRun y carga ejecutable propuestos en D17 §3 «Prerrequisitos y
subconjunto de arranque» y D17 §7 «Recarga en caliente» requieren su revisión
de catálogo con firmas, efectos y manifiestos antes de activación. N07a/N14a
poseen esa entrega de arranque/recarga y presentan la ampliación a D16;
no se habilitan por C08 ni se presuponen selectores disponibles en v1.

**R56.** Interacciones que no admiten excepción por std:

| Decisión | Conservación y consecuencia |
|---|---|
| P01–P03 | Semántica única, independencia y exclusividad; List con recursos no copia y RawView no alarga vida. |
| P04–P06 | Optimizaciones preservan observaciones; confianza no es efecto; costes lógicos y mediciones físicas separados. |
| P07–P09 | TCB enumerada, sin dependencia de IA/servicio remoto, diagnósticos estructurados y sin reparación que añada permiso. |
| D01 | Return explícito, sintaxis B, `.n`, un parser; grammar R09 es enmienda concreta. |
| D02 | Result/Option recuperables; Abort no limpia ni se captura; mocks R no acreditan panic estable. |
| D03 | Recetas Nexo comptime puras/acotadas, target explícito; sin E/S ni intérprete de metaprogramación privado. |
| D04 | IR propia y stage0 Rust; std común a intérprete/nativo, backend independiente de su lenguaje anfitrión. |
| D05 | Asignador/región declarados, vidas comprobadas, sin redirección ambiental ni GC de ciclos prometido. |
| D06 | Procedencia y licencia por fuente reutilizada; referencia a APIs ajenas no autoriza copiar su código sin licencia. |
| D07 | Inicialización, enteros comprobados y limpieza; low no activa aritmética UB por optimización. |
| D08 | Traits nominales/coherencia y genéricos comprobados antes de instanciar; std no obtiene impl extranjera privilegiada. |
| D09 | CoW/RC en Nexo, conteo local separado de atómico; una raíz única no acredita hijos/débiles/publicación. |
| D10 | UTF-8 y errores, argumentos en orden escrito, Iterator Nexo, yield síncrono con suspend; REPL no cambia objetos vivos. |
| D11 | Capturas conservan cuatro modos, cuatro resúmenes y procedencias al pasar por List/Option; sin boxing por inferencia. |
| D12 | SuspendSafe estructural; permiso unsafe no admite guarda mutable en suspensión, cancelación no es drop ni rollback. |
| D13 | Identidad paquete/módulo/declaración, privacidad y from/retains; una dependencia implícita de compilador no es import autorizado. |
| D14/D15 | B única, PascalCase/camelCase, funciones con bloque, sin sintaxis A ni segundo frontend de bootstrap. |
| A2/J/F5/K | Un solver, tabla de tipos, CFG, motor de propiedad y dispatcher; diferidos permanentes de F0 usan later_adr. |

## 13. Alternativas rechazadas y límites reales

**R57.** Se rechazan dos dialectos o modo «rápido unsafe» porque cambiarían
P01; permiso de módulo implícito porque ocultaría sitios de confianza;
unsafe como efecto que se borra en wrappers porque no modela precondiciones;
List implementada con un handler privado Rust porque incumple acuerdos §9.3;
mantener Array dinámico y añadir List como alias porque duplica superficies;
cast libre o puntero entero porque borran procedencia; inferir seguridad
por ser std porque impediría bootstrap honesto; hooks de copia arbitrarios
porque romperían P02; ABI de structs Rust porque ata biblioteca a stage0;
CoW simultáneo a la migración porque impediría atribuir discrepancias;
TLS extranjero oculto bajo SystemCall porque ampliaría fraudulentamente
la excepción de sistema; y usar solo demos porque no cubren invariantes.

Las intrínsecas altas de F0 se mantienen solo hasta que existan los
prerrequisitos de su reemplazo: genéricos/traits, módulos, hooks y memoria.
Retirarlas antes dejaría un arranque circular o una biblioteca no comprobable;
añadir handlers altos nuevos fuera de A2/J/F5/K ampliaría esa deuda sin causa.
M08–M11 retiran cada familia en su activación atómica, sin fallback ni modo
legacy; M09a retira operadores altos al disponer de traits sobre C01/C02.

**R58.** Contraejemplos decisivos: copiar dos punteros al mismo búfer y luego
escribir viola independencia aunque los handles sean structs distintos;
envolver una vista local en List y retornarla sigue siendo escape; obtener
contador uno y publicar mientras existe una referencia débil no concede
unicidad; marcar un malloc como no_alloc no cambia la reserva; sustituir
String por llamada libc no hace su implementación Nexo; mantener un helper
Rust de UTF-8 tras migrar su API conserva dos capas falsas. Cada uno tiene
regresión de R47–R50 y no se corrige mediante whitelist de fuente.

**R59.** Incertidumbres de evidencia, no elecciones abiertas: coste y tamaño
de std Nexo frente a los handlers; disponibilidad de un proveedor TLS Nexo
auditable; esfuerzo de los manifiestos nativos por plataforma; suficiencia
práctica del contrato homogéneo storage para contenedores futuros; y coste
de self-hosting/backend propio. Un contenedor heterogéneo no expresable por
R12 necesita otro contrato revisado, no una ampliación silenciosa. Las
decisiones de este ADR se mantienen aunque sus tareas sean costosas; un
contraejemplo obliga a enmienda y regresión. Este documento no afirma pruebas
superadas, implementación, revisión independiente ni aprobación de fase.
