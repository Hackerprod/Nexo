# F0-D11 — Closures con capturas locales y propiedad explícita

Estado: **iteración 6 propuesta; pendiente de nueva validación**.
La propuesta inicial y las iteraciones 1–5 recibieron **FAIL de Sonnet 5.5**,
según las devoluciones del coordinador. Esta iteración 6 atiende B1 y N1–N4;
conserva pendientes visibles y no afirma PASS.
Responsable de redacción: Codex, rol arquitecto. Fecha: 2026-10-19.
No acredita implementación, validación independiente ni aprobación de F0.

## Contexto y autoridad

[Acuerdos §2](../F0_ACUERDOS.md) exige decidir capturas y propagación transitiva.
Autoridad: [roadmap 0.2](../baseline/Nexo_ROADMAP_rev0.2.md), P02/P03
(líneas 246–268), P04–P06/P09, D01/D02/D07–D10, §6.1–6.2 y T06/T10/T14/T22.
La base permanece intacta. [F0-0001](F0-0001-first-slice.md) todavía no implementa
closures ni préstamos. No se reinterpretan sus resultados como evidencia nueva.

Según roadmap §2, se integran valores de Hylo/Swift, exclusividad, propiedad y
traits nominales, inferencia de efectos y límites públicos explícitos. La
aportación buscada para agentes es explicar localmente qué se retiene, copia,
consume o publica, con una cadena causal y una reparación comprobable.

No hay **modificación semántica propuesta** de D01: la abreviatura conserva
`return` escrito. Las restricciones conservadoras nuevas se deciden abajo.

### Ampliaciones sobre la base

Se completan acuerdos §2 y lo pendiente de D01/D07/D08: grafías de closure,
lista de capturas y palabras contextuales borrow/copy, prefijos consume/inout
sobre invocación, regla de copia/transferencia por contexto, propagación de
procedencias y cuatro operaciones. Se conserva consume de la base y return
explícito. Call*, su sellado, dyn (CallableBound), restricciones escritas
unrestricted y mapa with por operación son ampliaciones declaradas en §5.2.
La conformidad Call* de funciones con nombre y la captura explícita no usada
se concretan como ampliaciones de D08/acuerdos §2 en §§2/5.1.
La retención hasta destrucción y la composición C/Q son restricciones nuevas
de esta propuesta, no afirmaciones de soporte del prototipo.

## Decisión

### 1. Dos formas con retorno explícito

```nexo
let largo = fn [copy base](x: i64) -> i64 {
    return base + x;
};
let corto = fn [copy base](x: i64) -> i64 => return base + x;
```

La única forma corta aceptada es `=> return expresión`, expandida a
`{ return expresión; }`. Su terminador pertenece a la sentencia exterior.
Para unidad se usa `=> return ()`. No admite una segunda sentencia.
`fn(x) => x` se reconoce mediante ImplicitClosureBody (D13 G.2) y se rechaza
con NX-E1301/implicit_closure_body, no NX-E0002. El span primario es el
operando completo después de =>; el de => es relacionado. Se conserva el
AST de expresión inválida para ofrecer la edición explícita => return expr,
sujeta a recomprobación. Una expresión
final nunca infiere retorno. D13 §2 fija: `{ expr }` en una línea da
NX-E0005 en la llave por falta de terminador; `{ expr; }` descarta y, si
falta un retorno de resultado ordinario/async, da NX-E1317. La reparación
`{ return expr; }` solo se anuncia aplicable tras recomprobar el cuerpo.
Un salto de línea que termina expr también descarta: no infiere return.
No hay sintaxis con barras.

Los tipos de parámetros/resultado locales pueden inferirse cuando el contexto
inmediato y el cuerpo determinan una solución única; de lo contrario se pide
anotación (`NX-E1111`). Un contrato `with` va después del resultado.
`return` abandona la closure actual, nunca la función donde se construyó.
La sintaxis async conserva estas formas y las reglas específicas de D12.

### 2. Préstamo de lectura por defecto; lista explícita exhaustiva

Sin lista se captura por préstamo de lectura cada binding léxico libre,
incluidos los necesarios para construir closures interiores. No se infiere
copia, mutación ni transferencia. P03 permite prestar lecturas ordinarias;
retenerlas añade una obligación de duración al tipo del entorno.

Con lista, cada binding libre aparece exactamente una vez con modo; `[]`
significa ninguno. No se completan listas parciales. Funciones y constantes
de módulo no son capturas, pero sus tipos, efectos y afinidad sí cuentan.
No se introducen globales mutables implícitas.
Una captura explícita listada y no usada **es válida**: la lista también
ordena adquisición/retención de estado. Se comprueban copia, préstamo y
transferencia y se construye el campo aunque el cuerpo no lo lea. Su efecto,
vida y limpieza no se eliminan por ausencia de usos; optimizarlo requiere
probar las observaciones P04. Por eso C04 falla al copiar r y C08 consume r.
C49/C50/C51 concretan capturas no usadas de copia, recurso y préstamo.

Orden de captura: el escrito en la lista o, sin ella, primera aparición textual
del binding, incluidas las listas de closures interiores. Se fija antes de
optimizar y se conserva para destrucción e inicialización parcial.

### 3. Cuatro modos, sobre bindings completos

| Entrada | Acción al construir | Condición |
|---|---|---|
| `borrow x` | Retiene lectura y procedencia de `x`. | Coexiste con lectores compatibles; no independiza. |
| `inout x` | Retiene préstamo mutable exclusivo. | Origen modificable; excluye accesos incompatibles. |
| `copy x` | Crea una copia lógica propietaria. | Todo el contenido debe admitir copia; no duplica recursos. |
| `consume x` | Transfiere propiedad al entorno. | Invalida propietario anterior; no prolonga préstamos contenidos. |

La primera gramática captura bindings completos, no `x.campo` ni expresiones.
Tras el Name de Capture, solo coma o ] completan la lista: x.campo produce
NX-E0002 en el punto, con prefijo y span exigidos por C57/S57.
Preparar otro binding es una operación previa ordinaria. Si se necesita
granularidad, una proyección debe acreditar disjunción; otro nombre no basta.
Repetir un binding es error incluso con modos distintos.

Un campo propietario conserva la mutabilidad del binding fuente: copiar
`var x` crea un campo privado modificable; copiar `let x`, uno inmutable.
Mutar la copia de datos independientes no modifica el original. `borrow`
siempre es de lectura; `inout` modifica el origen. La mutación interior de
identidad compartida solo procede por su mecanismo explícito de acceso.

Se elige `consume`, como D08 y la base. El `move` de F0-0001 conserva su
condición de grafía provisional de aquel corte, no de segundo sinónimo estable.
Su migración futura deberá adaptar parser, casos y diagnósticos; no se realiza
en esta entrega.


### 3.1. Contextos de lectura, copia y transferencia

**Completa lo pendiente de D07/D08 y acuerdos §2.** La categoría sintáctica
del destino decide la operación, nunca la copiabilidad descubierta después:

| Contexto | Un nombre/Place existente | Un temporal propietario nuevo |
|---|---|---|
| Lectura, parámetro ordinario, condición, scrutinee ordinario | Préstamo conforme a P03/D08. | Préstamo durante el uso; duración del temporal comprobada. |
| Inicializador, RHS, elemento propietario, return/value:, argumento consume, operando de await | Copia lógica. Si no admite copia, NX-E0201; **no hay movimiento implícito alternativo**. `consume p` transfiere expresamente. | Se entrega propiedad directamente; no hay origen nombrado que invalidar. |
| `consume p` | Transfiere y deja p consumido, también si era copiable. | No se necesita consume; no crea una segunda propiedad. |
| `drop(p)` | Operación intrínseca de destrucción: consume p por su propia sintaxis. No es una llamada ordinaria que intente copiarlo. | Destruye el temporal. |

Place es un binding o proyección con procedencia comprobada. `consume f(args)`
es la forma de invocación consumidora de §5: consume el receptor f y produce
el resultado de la llamada; no significa consumir dos veces el resultado.
No se deduce modo consume de una firma sin la grafía correspondiente en un
argumento que sea un binding. La excepción sintáctica drop está enumerada;
una función de usuario llamada parecido no adquiere ese privilegio.

Así, `let h = f;` copia f o produce NX-E0201; `let h = consume f;` mueve.
La copia de una closure copiable se escribe precisamente `let h = f;`.
`copy` en una lista de capturas nombra esa misma operación, no crea un
segundo operador general de copia. `await consume f` sigue esta tabla;
`await f` intenta copiar f y falla si es un futuro/handle no copiable.

Todo uso posterior al consumo comparte `NX-E0202/definitely_consumed` o
`NX-E0202/maybe_consumed`, definidos en [el catálogo único](F0-CODIGOS.md). No hay código
distinto por tratarse de recurso, closure, futuro o unión de ramas.

### 4. Construcción, copia y fallo parcial

Construir evalúa capturas en el orden de la regla 2 y no ejecuta el cuerpo.
Cada transferencia cambia de propietario en su punto de evaluación. Ante un
fallo recuperable durante una construcción se limpian, en orden inverso, solo
campos inicializados todavía propietarios; no se reviven valores transferidos.
Los préstamos se liberan después de la limpieza que los necesite.

La copia lógica no llama constructores arbitrarios con efectos externos (P02).
Puede requerir almacenamiento y RC conocidos, que deben aparecer en los efectos.
OOM/panic mantienen D02: no se promete limpieza durante aborto del proceso.
Los efectos de una captura fallida incluyen el prefijo construido y su limpieza.

### 5. Invocación y cotas

### 5.1. Modo de invocación distinto de capacidad de copia

El cuerpo determina el modo mínimo del receptor:

| Modo | Uso del entorno | Invocación |
|---|---|---|
| Lectura | Lee campos o usa compartición explícita sin modificarlos. | `f(args)` |
| Mutable | Modifica un campo o utiliza una captura `inout`. | `inout f(args)`, con receptor modificable y exclusivo. |
| Consumidor | Transfiere/destruye un campo propietario en algún camino. | `consume f(args)`, que consume toda la closure. |

Los prefijos califican al receptor de la llamada completa, no a sus argumentos.
G.1 de D13 fija la selección: inout x.m() califica self = x de un método;
inout (x.m)() invoca el campo invocable x.m. La distinción por paréntesis es
una ampliación explícita de las formas de llamada de D08 (C52–C52c).
Estos conservan sus modos propios. Se puede pedir explícitamente un modo más
fuerte: mutable para un cuerpo de lectura; consumidor para cualquiera, limpiando
el resto del entorno al terminar. Nunca se fortalece el modo silenciosamente.
Una llamada consumidora invalida `f` aunque la rama ejecutada no transfiera
campos. Sus campos todavía vivos se destruyen al salir.
Un prefijo inout correcto sobre un binding inmutable da NX-E0105; no se
confunde con NX-E1107, que señala haber solicitado un modo insuficiente.

Las interfaces son traits nominales sellados `Call`, `CallMut`, `CallOnce`,
con parámetros, resultado y resúmenes asociados. El compilador genera las
implementaciones admitidas: lectura implementa los tres, mutable implementa
CallMut/CallOnce y consumidor solo CallOnce. Nombrar un método `call` no fabrica
conformidad. Un argumento que no implementa el protocolo exigido por la
cota se rechaza con NX-E1113, aunque sus parámetros y resultado coincidan.
Un requisito `F: Call` no concede capacidad de copiar `F`. Cada expresión de
closure tiene un tipo anónimo distinto; no hay boxing, `dyn` ni conversión
entre ellos por defecto.

**Funciones con nombre (completa D08 y EX02):** un nombre de fn en posición
de valor denota un valor de función con identidad de código y entorno vacío.
El compilador genera Call, CallMut y CallOnce; es copiable e independiente,
sin reserva ni efectos de create/copy/drop del propio valor. Consumir un
binding de ese valor invalida ese binding, no la declaración de función.
No se inventa una closure con capturas ni un impl del usuario.

La firma, etiquetas, modos, procedencias y cota de call son los de la función:
inferencia privada o límite público, nunca efecto vacío por ser función con
nombre. Las instancias genéricas conservan los requisitos de D08. No hay
conversión implícita a dyn. C48 concreta `handler: responder` de EX02 (roadmap línea 151).

Para una async fn con nombre, call crea un Future nuevo y usa create del
contrato async, sin ejecutar resume; para gen fn crea un estado nuevo sin
avanzarlo. Sus parámetros retenidos y el estado resultante conservan los
contratos D10/D12. El valor de función vacío sigue siendo Call y reutilizable.
La regla CallOnce de **todo literal de closure async, incluso []**, sigue
D12 y no se aplica a una declaración async fn. Una optimización no cambia
esa categoría sintáctica ni elimina transferencias exigidas en argumentos.

Poseer un recurso no implica llamada única: una closure puede inspeccionarlo
varias veces, ser `Call` y no copiable. Su destructor conserva la obligación.
Reentrada y callbacks deben respetar el préstamo de receptor ya activo.


### 5.2. Gramática de cotas para llamadas indirectas

**Completa lo pendiente de §6.1 y amplía D08.** Call, CallMut y CallOnce son
traits nominales intrínsecos sellados generados por el compilador: este sellado
de protocolos invocables es una **ampliación sobre la base**, cuya lista
explícita de propiedades selladas no los incluía. No se usa conformidad
estructural ni se permite un impl ordinario que fabrique un protocolo invocable.
Intentar implementar Call* manualmente da NX-E0110. Call, CallMut y CallOnce
son nombres de tipos/traits del preámbulo implícito, resolubles sin use por la
misma regla de identidad nominal y sombreado de SuspendSafe (D12 §9).
Una impl que resuelve uno de esos intrínsecos se rechaza antes de comprobar
aridad, métodos o conformidad; C58/C58m/C58o no fallan por nombre desconocido.
Las demás impl nominales siguen D08 sin cambios.

```text
WithClause     = "with" (Restrictions | OperationList)
OperationList  = "{" [OperationLimit {"," OperationLimit}] "}"
OperationLimit = Operation "(" Restrictions ")"
Operation      = "create" | "copy" | "call" | "resume" | "cancel" | "drop"
Restrictions   = RestrictionItem {"," RestrictionItem}
RestrictionItem = "unrestricted" | Restriction
Restriction    = "no_alloc" | "no_heap_alloc" | "no_block" | "no_suspend" | "no_io"
CallableBound  = ("Call" | "CallMut" | "CallOnce")
                 "(" [CallableParam {"," CallableParam}] ")" "->" ResultType
                 ProvenanceClauses [WithClause]
CallableParam  = ["inout" | "consume"] [Name ":"] Type
FutureType     = "Future" "<" Type ">" WithClause
ResultType     = Type
```

Esta EBNF incluye tres formas reconocidas **solo para diagnóstico**: CallableBound
sin WithClause, OperationList vacía y Restrictions con unrestricted acompañado.
La aceptación exige with y mapa no vacío en toda cota invocable, y unrestricted
como único elemento de su lista; no se amplían las cotas aceptadas.
El control posterior obligatorio usa NX-E0417/missing_with_clause,
empty_operation_list o unrestricted_mixed, respectivamente. Un unrestricted
repetido sin otra restricción usa duplicate_restriction. Si hay repetición
y mezcla, se informa primero la primera repetición; después, la mezcla.
La lista interna de una OperationLimit tampoco puede estar vacía; esa forma
no pertenece a Restrictions y sigue siendo error sintáctico.

Type, Name y ProvenanceClauses se definen una sola vez en D13, gramática G.
Por ejemplo, `Call(consume v: T) -> T from(v) with { call(unrestricted) }`
conserva procedencias de v. Esta cota publica retención:

```text
CallMut(inout owner: Option<T>, consume v: T) -> ()
    retains(v, in: owner) with { call(unrestricted) }
```
No son comentarios ni tipos sin capacidad de expresar préstamos.

Esta es la **única producción WithClause** de D11–D13. Una declaración fn
o literal de closure escribe como máximo una WithClause, después de resultado
y procedencias y antes de su cuerpo; tipos Future y cotas invocables usan la
misma producción. Los paréntesis del resultado Future separan su with del
with de la cota invocable exterior (§5.3). Las restricciones de contexto son:

| Contexto | Lista simple | Operaciones admitidas en lista con llaves |
|---|---|---|
| fn ordinaria | Limita call. Conserva la grafía de D08. | Solo call; si se escribe lista, call es obligatorio. |
| Closure síncrona | Limita call. | create/copy/call/drop; call obligatorio. |
| async fn, incluida closure async | Limita resume; solo privada/local. | create/resume/cancel/drop; resume obligatorio local, las cuatro obligatorias públicas. |
| Cota Call/CallMut/CallOnce | No admitida. | create/copy/call/drop; call obligatorio privado, las cuatro públicas. |
| Tipo Future<T> | No admitida. | create/resume/cancel/drop; las cuatro obligatorias. |

La closure async anota aquí **Q**, el futuro; los límites de su entorno **C**
se expresan en la cota CallOnce contextual de §5.3. Así un no_suspend simple
en el literal async limita Q.resume, no C.call. Una closure síncrona limita
C.call. No se mezclan ambos espacios de operaciones.

La lista simple conserva la sintaxis de la base; las llaves identifican
únicamente el mapa por operación en **todos** los contextos. Es una ampliación
sobre la base para completar §6.1, no dos grafías del mismo mapa. Se eliminan
las listas de operaciones sin llaves de la iteración 1.
`with no_alloc, create(no_io)`, `with { no_alloc, create(no_io) }` y
`with create(no_io)` no pertenecen a WithClause: NX-E0002 deliberado.
Las listas de operaciones vacías y la mezcla unrestricted con prohibiciones
se reconocen y rechazan por los diagnósticos específicos anteriores, en vez
de NX-E0002. Missing_with_clause precede a cualquier ausencia de operación;
empty_operation_list precede a las ausencias de call/resume u operaciones públicas.
La ausencia total de with en una cota invocable pública o privada da siempre
missing_with_clause; NX-E1112/NX-E1114 quedan para mapas no vacíos incompletos.
Una operación repetida da NX-E0417/duplicate_operation; una no admitida para
el contexto, NX-E0417/invalid_operation; una restricción repetida en la misma
lista, NX-E0417/duplicate_restriction. La falta de call en una fn/closure
síncrona con mapa o de resume en async local da NX-E0417/missing_body_operation.
Una forma simple en un tipo Future o cota invocable da
NX-E0417/operation_list_required. Estas son comprobaciones sobre sintaxis
reconocida. Las omisiones de firmas públicas/cotas privadas conservan los
diagnósticos específicos indicados abajo y en D12; tienen prioridad sobre
missing_body_operation. Además, **invalid_operation precede a
missing_body_operation**: primero se comprueban las operaciones presentes
contra las admitidas en ese contexto; una resume inválida en fn síncrona no
se informa como ausencia de call (C39). Las operaciones opcionales omitidas significan
unrestricted. La ausencia total de with en cuerpos privados permite inferencia
local; no equivale a omitir with de un tipo indirecto, que lo exige.
En fn ordinaria pública, cualquier WithClause válida para ese contexto
basta como cota explícita: with no_alloc, with unrestricted o
with { call(unrestricted) }. No hay que enumerar todas las prohibiciones;
las no escritas quedan permitidas. Esto no elimina las cuatro operaciones
obligatorias de una cota invocable pública o de async/Future (tabla anterior).

La forma completa de un requisito es `F: CallableBound`, dentro de parámetros
genéricos o where. Concreción sin tipos ni operadores nuevos de efectos:

```nexo
fn aplicar<F: Call(i64) -> i64 with {
    create(unrestricted), copy(unrestricted),
    call(no_alloc, no_block, no_suspend, no_io), drop(unrestricted)
}>(f: F, x: i64) -> i64
    with no_alloc, no_block, no_suspend, no_io {
    return f(x);
}
```

La función presta f; no construye, copia ni destruye su entorno. Para
`CallMut`, recibe `inout f: F` e invoca `inout f(x)`; para `CallOnce`,
recibe `consume f: F` e invoca `consume f(x)`. El límite de call consumidor
incluye la limpieza de campos no transferidos.

En una cota privada escrita call es obligatorio: su omisión da NX-E1114.
Las otras operaciones omitidas se interpretan unrestricted, jamás se
infieren como vacías.
Una interfaz pública escribe las cuatro; omitirlas da NX-E1112. No se repite
una operación ni se mezcla unrestricted con otras restricciones en la misma
lista. Un límite de copy no autoriza copiar: la propiedad estructural de
copia debe satisfacerse además. Un incumplimiento de la cota usa el código
de contrato correspondiente y señala esta declaración como contract_origin.

Las operaciones de FutureType se escriben las cuatro. Esta notación restringe
el resultado asociado concreto de la llamada; no borra ni unifica los tipos
anónimos de distintos estados. La especialización conserva su representación.
En F3 `dyn (CallableBound)` fija exactamente esas cotas, parámetros y resultado;
solo es una vista prestada hasta que un contenedor propietario explícito
declare reserva, procedencia y limpieza. No es una opción sin gramática.

Un wrapper público nominal declara campos con tipos ya nombrables y operaciones
con estas cotas; no exporta un campo de tipo anónimo inferido. Una función
genérica pública puede recibir F con la gramática anterior. El retorno público
de una closure requiere representación nominal o dyn explícita; no se añade
impl Trait ni boxing por inferencia.

### 5.3. Composición de una closure async y su futuro

Hay dos objetos y dos espacios de nombres: C es closure; Q, su futuro.
La cota invocable describe C y puede contener FutureType como resultado:

```nexo
fn iniciar<F: CallOnce() -> (Future<i64> with {
    create(no_suspend), resume(unrestricted),
    cancel(unrestricted), drop(no_suspend)
}) with {
    create(unrestricted), copy(unrestricted),
    call(no_suspend), drop(no_suspend)
}>(consume f: F) with no_suspend {
    let q = consume f();
    return consume q;
}
```

Iniciar es privado y su resultado concreto se infiere; la firma pública
necesitaría publicar el contrato opaco completo. Que resume permita suspend
no invalida el no_suspend de iniciar: no avanza Q y transfiere su propiedad.

| Operación | Resumen que se ejecuta |
|---|---|
| Construir C | C.create, capturas incluidas; ningún cuerpo async. |
| Copiar C | C.copy, solo si C admite copia; no crea Q. |
| `consume C(args)` | Argumentos + C.call = transferencia de campos + Q.create + limpieza de inicialización parcial/restos no transferidos. No Q.resume. |
| Abandonar C sin invocación | C.drop; sus capturas aún le pertenecen. |
| Esperar Q | Q.resume y las salidas/limpiezas que correspondan, más efecto await; cancelación usa Q.cancel. |
| Descartar Q nunca empezado | Q.drop, sin Q.resume. |

C no vuelve a limpiar campos transferidos a Q: sus flags quedan desarmados.
Q.resume incluye el cuerpo async y limpiezas de sus salidas; Q.cancel incluye
retiro de retenciones y limpieza; Q.drop destruye solo estado inerte.
Son conjuntos de efectos, no sumar costes dos veces por mostrar dos rutas.
La copia de C, si es posible, crea otro entorno con sus obligaciones; no copia Q.
D12 usa exactamente esta correspondencia.

### 6. Propagación estructural de propiedades separadas

Cada captura y campo anidado conserva una ruta en el resumen:

* **Copia:** todos los campos admiten copia. Un préstamo compartido puede
  duplicarse conservando procedencia/duración; uno mutable no. Un recurso
  propietario hace no copiable todo el entorno.
* **Independencia:** todos los campos son propietarios e independientes
  transitivamente. Un préstamo retenido no acredita independencia. Un handle
  de actor puede ser copiable sin ser instantánea de su estado.
* **Transferencia y afinidad:** intersección de dominios de uso y destrucción
  admitidos por campos y operaciones del cuerpo. `LocalRc` o un recurso afín
  confina todo el entorno. Préstamos locales no se vuelven transferibles entre
  ejecutores. Incluso una closure vacía conserva requisitos de afinidad de sus
  llamadas a funciones de módulo.
* **Efectos:** separados por operación según la regla 9; retener un efecto
  latente no lo ejecuta.

D08 mantiene estas propiedades selladas o derivadas, nunca afirmables por un
`impl` ordinario. Se recorren structs, enums, colecciones, genéricos y closures;
el caso conservador de un enum usa todas las variantes posibles. Agotar el
solucionador produce diagnóstico, no una prueba de seguridad. `consume` no
añade independencia, copia ni transferencia a un valor.

### 7. Duración y escape

El entorno prestado vive como máximo dentro de todas sus procedencias.
Retornarlo, guardarlo en una colección/error/closure, entregarlo a callback o
retenerlo en un futuro exige la misma prueba transitiva. Mover/copiar el handle
no prolonga bytes. Un préstamo a un local no sale de la función; uno a un
parámetro solo sale con una relación válida que la interfaz conserve.

La inferencia es local. Las firmas públicas con retención/resultados prestados
declaran procedencia (P03, D08, D13). Contrato externo ausente no significa
«no retiene». No hay promoción a propiedad mediante reserva oculta: el usuario
prepara una copia suficientemente independiente, cuando sea posible.

Regla inicial conservadora: un préstamo retenido acaba al destruir/consumir el
entorno o salir de su ámbito, no por adivinar su última llamada. `drop(f)`
consume y destruye explícitamente. Eliminar almacenamiento interno conserva
observaciones y orden de limpieza de P04/D07.

### 8. Exclusividad a través de anidación

Los permisos se refieren a procedencias y caminos de almacenamiento, no a
nombres. Desde `borrow x`, una closure interior no obtiene `inout x` ni
`consume x`. Desde `inout x` puede obtener un représtamo subordinado: mientras
viva, la exterior no utiliza permisos incompatibles. Su escape necesita
procedencia del receptor; consumir este no prolonga el représtamo.

Copiar un lector crea otro lector. Envolver un permiso mutable en structs,
arrays o closures no permite copiarlo o enviarlo dos veces. D13 une los permisos
de ramas sin inventar exclusividad. `inout` tampoco acredita unicidad física
de almacenamiento CoW; §6.1 exige preparación/comprobación recursiva cuando
una mutación deba cumplir `no_alloc`.

### 9. Cuatro resúmenes de efectos

Se distinguen `create`, `copy`, `call`, `drop`, además de modo, afinidad y
procedencias. `create` incluye capturas y representación; `copy`, duplicar
el entorno; `call`, cuerpo, temporales y limpiezas de sus salidas; `drop`,
campos aún propietarios y decrementos finales. En llamada consumidora,
`call` incluye destruir el resto del entorno. En llamada prestada,
la destrucción del entorno sigue correspondiendo a su propietario.

Crear y abandonar exige justificar `create ∪ drop`. Retornar una closure
justifica la creación y exporta las obligaciones del valor retornado; su
deuda de limpieza llega al propietario final. No se oculta por empaquetado.
Una llamada indirecta necesita límites conocidos (§6.1).

`with no_alloc` escrito en una closure síncrona limita `call`, no certifica
`create/copy/drop`. El [catálogo único](F0-CODIGOS.md), entradas NX-E0412 a NX-E0416,
identifica contratos incumplidos y muestra operación y cadena causante.
Lowering, adaptadores y borrado de tipos vuelven a comprobar los efectos
introducidos. No se declara un coste desconocido como vacío.

### 10. Recursos, generadores y tareas

Cada campo tiene estado de inicialización/transferencia. Destruir limpia los
campos propietarios restantes en orden inverso de captura, una sola vez.
Los prestados liberan permisos, no el objeto. Un cierre explícito fallable
declara qué propiedad conserva en cada resultado; el destructor residual no
propaga `?` (D02).

Copiar un handle compartido crea una participación: cada handle la libera
una vez, y el último destruye el objeto. No se promete resolver ciclos (D05).
Generadores F2 solo retienen propietarios (D10), incluso si reciben una
closure: copiar un entorno prestado no cumple esa condición. F3 podrá admitir
procedencia comprobada. `yield` sigue síncrono y suspendiendo en `next`.
D13 §1.1 separa Item del retorno del cuerpo: un gen fn termina al caer al
final o con `return;` opcional; `return` con operando da NX-E1324, no NX-E1317.

[D12](F0-D12-async.md) decide futuros y suspensión. Capturar no lanza tareas.
Publicar aplica D09 al grafo entero; ni envoltura ni `await` convierten RC local
en atómico. Compartir estado mutable entre tareas exige aislamiento explícito,
no dos aliases del mismo permiso (T06).

## Alternativas rechazadas

1. Captura universal por copia: oculta costes y no resuelve recursos ni identidad.
2. Inferir captura mutable/movimiento por uso: una edición interna cambia los
   permisos del llamador. Se exige elegir el modo explícitamente.
3. Referencias con extensión automática de vida: ocultan reserva y retención.
4. Equiparar propiedad con independencia/transferencia: contradice P02/D09.
5. `=> expresión` con retorno implícito: contradice D01 y acuerdos §4.
6. Destruir recursos al último uso inferido: cambia observaciones de D07.

## Contraejemplos y casos PASS/FAIL

PASS/FAIL son **resultados de diseño previstos**, no ejecución ni dictamen.
Una fila sin fuente concreta se marca NO EVALUADO y no acredita la regla.
En filas de sentencias, la fuente exacta se obtiene insertando la celda en:

```nexo
fn main() {
    var x: i64 = 1;
    var a = [[1]];
    let r = mock_open(10);
    // Insertar aquí las sentencias de una sola fila.
}
```

Se usan únicamente mock_open/inspect/drop/assert_eq del arnés de F0-0001,
con sus contratos documentados; esta revisión añade consume y closures a su
gramática prevista, no afirma que el frontend actual pueda ejecutarla.
Las filas que contienen funciones completas sustituyen la plantilla.
Los permisos y efectos futuros sin fixture definida permanecen pendientes.

| ID | Fuente concreta o trabajo pendiente | Resultado previsto | Diagnóstico / observación |
|---|---|---|---|
| C01 | `let f = fn() => return x; let n = f();` | PASS | —; préstamo local. |
| C02 | `let f = fn[]() => return x;` | FAIL | NX-E1101. |
| C03 | `let f = fn [copy x, borrow x]() => return x;` | FAIL | NX-E1102. |
| C04 | `let f = fn [copy r]() { return (); };` | FAIL | NX-E0201. |
| C05 | `var f = fn [copy a]() { a[0][0] = 9; return (); }; inout f(); assert_eq(a, [[1]]);` | PASS | —; original independiente. |
| C06 | Falta fixture de actor anidado y exigencia de independencia. | NO EVALUADO | Objetivo negativo NX-E1108; no PASS/FAIL acreditado. |
| C07 | `let f = fn [consume r]() { inspect(r); return (); }; f(); f(); drop(f);` | PASS | —; dos lecturas y un cierre. |
| C08 | `let f = fn [consume r]() { return (); }; inspect(r);` | FAIL | NX-E0202/definitely_consumed. |
| C09 | `let f = fn [consume r]() { drop(r); return (); }; consume f(); consume f();` | FAIL | NX-E0202/definitely_consumed. |
| C10 | `let f = fn [consume r]() { inspect(r); return (); }; let h = f;` | FAIL | NX-E0201; initializer es copia, no movimiento. |
| C11 | `var f = fn [inout x]() { x = 2; return (); }; f();` | FAIL | NX-E1107. |
| C12 | `let f = fn [borrow x]() => return x; x = 2; f();` | FAIL | NX-E1105/active_borrow_conflict. |
| C13 | `let f = fn [borrow x]() => return x; drop(f); x = 2;` | PASS | —; termina préstamo. |
| C14 | `fn fabrica() { let x = 1; return fn [borrow x]() => return x; } fn main() { let f = fabrica(); }` | FAIL | NX-E1104. |
| C15 | `fn fabrica(b: bool) { let x = 1; let f = fn [borrow x]() => return x; if b { return Err([f]); } return Ok(()); } fn main() { let r = fabrica(true); }` | FAIL | NX-E1104; Result<(), Array<closure>> inferible, no fallo por tipo de Ok desconocido. |
| C16 | `let f = fn [borrow x]() { let g = fn [inout x]() { x = 2; return (); }; drop(g); return (); };` | FAIL | NX-E1105/permission_escalation. |
| C17 | `let caja = [fn [inout x]() { x = 2; return (); }]; let g = fn [inout x]() { x = 3; return (); };` | FAIL | NX-E1105/overlapping_mutable_borrows; el primer permiso está anidado. |
| C18 | `var f = fn [inout x]() { var g = fn [inout x]() { x = 2; return (); }; inout g(); drop(g); x = 3; return (); }; inout f(); drop(f); assert_eq(x, 3);` | PASS | —; représtamo finalizado. |
| C19 | Falta fixture ejecutable de LocalRc anidado y publicación entre ejecutores. | NO EVALUADO | Objetivo negativo NX-E1109; mantiene T06/T18 pendiente. |
| C20 | Falta fixture de reserva indirecta con contrato y representación fijados. | NO EVALUADO | Objetivo negativo NX-E0412 en call. |
| C21 | Falta recurso fixture cuyo destructor bloquea de forma declarada. | NO EVALUADO | Objetivo negativo NX-E0413 en drop. |
| C22 | Fuente C22 abajo. | FAIL | NX-E0414, call → next → yield. |
| C23 | Falta generador F2 genérico que retenga closure prestada válida al construir. | NO EVALUADO | Objetivo negativo NX-E1110, no error accidental de escape. |
| C24 | `let f = fn [consume r]() { inspect(r); return (); }; drop(f);` | PASS | —; cuerpo cero veces, recurso cerrado una vez. |
| C25 | `fn main() { let x = 1; let f = fn [copy x]() => x; }` | FAIL | NX-E1301/implicit_closure_body; operando x, span S25; ImplicitClosureBody reconocido. |
| C25b | C25 insertando return entre => y x. | PASS | —; retorno explícito, mismo operando copiable. |
| C26 | `let f = fn [](x) => return x;` | FAIL | NX-E1111. |
| C27 | Falta arnés de fallo parcial entre capturas. | NO EVALUADO | Objetivo: prefijo limpio una vez; sin diagnóstico estático inventado. |
| C28 | Falta fixture de dos tareas y actor compartido por mensajes. | NO EVALUADO | Objetivo positivo T06; sin sustituirlo por dos closures locales. |
| C29 | `let f = fn [borrow x]() => return x; let h = f; drop(f); drop(h); x = 2;` | PASS | —; copia escrita como initializer; ambos lectores terminan. |
| C29b | `let f = fn [borrow x]() => return x; let h = f; drop(f); x = 2; h();` | FAIL | NX-E1105/active_borrow_conflict. |
| C30 | Falta fixture de captura de array con reserva de create fijada. | NO EVALUADO | Objetivo negativo NX-E0412; pureza de call no prueba create. |
| C31 | Función aplicar de §5.2 más `fn main() { let f = fn [](x: i64) -> i64 => return x; let n = aplicar(f, 1); }` | PASS | —; cota indirecta escrita. |
| C32 | Fuente C32 abajo. | FAIL | NX-E0415; llamada indirecta sin límite no_io. |
| C33 | Función iniciar de §5.3 más `fn main() { let f = async fn []() -> i64 { return 1; }; let q = iniciar(consume f); drop(q); }` | PASS | —; call no_suspend crea futuro, no lo avanza. |
| C34 | Fuente C34 abajo, closure consumidora usada como Call. | FAIL | NX-E1113; required_protocol = Call, available_protocols = [CallOnce]. |
| C34b | Fuente C34 sustituyendo la closure por `fn [](x: i64) -> i64 => return x`. | PASS | —; requisito Call satisfecho, no se copia F. |
| C35 | `fn aceptar<F: Call() -> i64 with { drop(unrestricted) }>(f: F) { return (); } fn main() { return (); }` | FAIL | NX-E1114; falta call en cota privada escrita. |
| C35b | C35 añadiendo `call(unrestricted)` a la lista. | PASS | —; cota explícita completa para uso privado. |
| C36 | `var n = 0; let f = fn [copy n]() { n = 1; return (); }; inout f();` | FAIL | NX-E0105; el campo es modificable, el binding receptor f no. |
| C36b | C36 cambiando solo `let f` por `var f`. | PASS | —; receptor mutable sin otro préstamo. |
| C37 | `fn caso() with no_alloc, create(no_io) { return (); }` | FAIL | NX-E0002; unexpected_token = create, span S37 y prefijo reconocido exigidos abajo. |
| C37b | `fn caso() with no_alloc, no_io { return (); }` | PASS | —; positivo de C37 conserva la cabecera y el primer límite. |
| C37c | `fn caso() with { no_alloc, create(no_io) } { return (); }` | FAIL | NX-E0002; unexpected_token = no_alloc, span S37c y prefijo reconocido abajo. |
| C37cb | C37c reemplazando solo { no_alloc, create(no_io) } por { call(no_alloc, no_io) }. | PASS | —; mapa reconocido con operación call admitida. |
| C37d | `fn caso() with create(no_io) { return (); }` | FAIL | NX-E0002; unexpected_token = create, span S37d y prefijo reconocido abajo. |
| C37db | C37d reemplazando create(no_io) por { call(no_io) }. | PASS | —; mapa con llaves y operación admitida en fn ordinaria. |
| C38 | `fn caso() with { call(no_io), call(no_alloc) } { return (); }` | FAIL | NX-E0417/duplicate_operation. |
| C39 | `fn caso() with { resume(no_io) } { return (); }` | FAIL | NX-E0417/invalid_operation; se comprueba antes de ausencia de call. |
| C40 | `fn caso() with { call(no_io, no_io) } { return (); }` | FAIL | NX-E0417/duplicate_restriction. |
| C41 | `fn aceptar<F: Call() -> i64 with no_io>(f: F) { return (); }` | FAIL | NX-E0417/operation_list_required. |
| C42 | `fn main() { let f = fn []() with { drop(no_io) } { return (); }; }` | FAIL | NX-E0417/missing_body_operation. |
| C43 | `fn caso() with { call(no_alloc, no_io) } { return (); }` | PASS | —; mismo WithClause por operación que tipos e invocables. |
| C44 | `fn main() { let f = fn []() -> i64 { 1 }; }` | FAIL | NX-E0005; llave del cuerpo de closure, span S44 abajo. |
| C44b | C44 sustituyendo `{ 1 }` por `{ return 1; }`. | PASS | —; reparación explícita comprobable. |
| C44c | C44 sustituyendo `{ 1 }` por `{ 1; }`. | FAIL | NX-E1317; solo se reparó la sintaxis, falta retorno. |
| C45 | `pub fn usar<F: Call() -> i64 with { call(unrestricted) }>(f: F) -> i64 with unrestricted { return f(); }` | FAIL | NX-E1112; falta create/copy/drop en cota pública, firma de usar completa. |
| C45b | C45 añadiendo `create(unrestricted), copy(unrestricted), drop(unrestricted)` al mapa de la cota. | PASS | —; cuatro operaciones públicas. |
| C46 | Fuente C46 abajo: llamada indirecta consume T con from(v). | PASS | —; cota escrita transporta las procedencias simbólicas del resultado. |
| C47 | Fuente C47 abajo: callback declara retains(v, in: owner). | PASS | —; la función envolvente publica la misma obligación. |
| C47b | C47 eliminando solo retains de la firma de guardar, no de la cota. | FAIL | NX-E1319; llamada indirecta también puede retener. |
| C48 | Fuente C48 abajo, handler: responder. | PASS | —; la función con nombre implementa Call* y conserva la cota escrita de call. |
| C49 | `let f = fn [copy x]() { return (); }; drop(f);` | PASS | —; captura explícita no usada válida, copia y destrucción del campo. |
| C50 | `let f = fn [consume r]() { return (); }; drop(f);` | PASS | —; adquisición transferida aunque el cuerpo no la lea; cierre una vez. |
| C51 | `let f = fn [borrow x]() { return (); }; x = 2; drop(f);` | FAIL | NX-E1105/active_borrow_conflict; captura no usada conserva el préstamo hasta drop. |
| C51b | C51 colocando drop(f) antes de x = 2. | PASS | —; termina el préstamo antes de mutar. |
| C52 | `struct Caja { n: i64 } extend Caja { fn incrementar(inout self) { self.n += 1; return (); } } use extension package.Caja; fn main() { var x = Caja(n: 0); inout x.incrementar(); assert_eq(x.n, 1); }` | PASS | —; extensión activada explícitamente; inout presta x como self del método, no un campo incrementar. |
| C52b | C52 sustituyendo solo var x por let x. | FAIL | NX-E0105; mismo método, receptor x inmutable. |
| C52c | `struct Caja<F> { m: F } fn llamar<F: CallMut() -> () with { call(unrestricted) }>(inout x: Caja<F>) { inout (x.m)(); return (); }` | PASS | —; (x.m) selecciona el campo F y su CallMut, sin self implícito de Caja. |
| C52d | C52c quitando solo los paréntesis de (x.m): inout x.m(). | FAIL | NX-E0102/field_requires_grouped_call; member_kind = field, expected_member_kind = method. El positivo editado es C52c. |
| C53 | `fn aceptar<F: Call() -> i64>(f: F) { return (); }` | FAIL | NX-E0417/missing_with_clause; cota reconocida completa, inserción antes de >, primary_span [27,27). |
| C53b | C53 insertando with { call(unrestricted) } entre i64 y >. | PASS | —; cota privada explícita. |
| C53p | `pub fn aceptar<F: Call() -> i64>(f: F) -> () with unrestricted { return (); }` | FAIL | NX-E0417/missing_with_clause; inserción antes de >, primary_span [31,31); precede a NX-E1112. |
| C53pb | C53p insertando with { create(unrestricted), copy(unrestricted), call(unrestricted), drop(unrestricted) } entre i64 y >. | PASS | —; las cuatro operaciones públicas y firma exterior completa. |
| C54 | `fn caso() with unrestricted, no_io { return (); }` | FAIL | NX-E0417/unrestricted_mixed; lista reconocida, primary_span [15,34). |
| C54b | C54 eliminando `unrestricted, ` (incluida su coma y espacio). | PASS | —; queda with no_io. |
| C54r | `fn caso() with no_io, unrestricted { return (); }` | FAIL | NX-E0417/unrestricted_mixed; primary_span [15,34); el orden inverso no evita la regla. |
| C54rb | C54r eliminando `, unrestricted`. | PASS | —; queda with no_io. |
| C55 | `fn caso() with { call(unrestricted, no_io) } { return (); }` | FAIL | NX-E0417/unrestricted_mixed; lista interna de call, primary_span [22,41). |
| C55b | C55 eliminando `unrestricted, ` (incluida su coma y espacio). | PASS | —; queda call(no_io). |
| C56 | `fn caso() with {} { return (); }` | FAIL | NX-E0417/empty_operation_list; mapa reconocido {}, primary_span [15,17); no missing_body_operation. |
| C56b | C56 reemplazando solo {} por { call(unrestricted) }. | PASS | —; mapa no vacío con operación obligatoria. |
| C56e | `fn caso() with { call() } { return (); }` | FAIL | NX-E0002; unexpected_token = ), span S56e y prefijo reconocido abajo. |
| C56eb | C56e insertando unrestricted entre los paréntesis de call(). | PASS | —; una lista interna explícita. |
| C57 | `struct Dato { campo: i64 } fn main() { let x = Dato(campo: 1); let f = fn [copy x.campo]() { return (); }; }` | FAIL | NX-E0002; unexpected_token = ., span S57 y prefijo reconocido abajo. |
| C57b | C57 reemplazando solo copy x.campo por copy x. | PASS | —; captura el binding copiable completo; una captura no usada es válida. |
| C58 | `struct Dato {} impl Call for Dato {} fn main() { return (); }` | FAIL | NX-E0110; Call resuelve el intrínseco del preámbulo antes de comprobar aridad o métodos. |
| C58m | C58 reemplazando únicamente Call por CallMut. | FAIL | NX-E0110; misma prohibición de impl manual. |
| C58o | C58 reemplazando únicamente Call por CallOnce. | FAIL | NX-E0110; misma prohibición de impl manual. |
| C58b | C58/C58m/C58o eliminando únicamente su ImplDecl. | PASS | —; Dato vacío válido; ninguna conformidad Call* afirmada por el usuario. |


### Ubicaciones exigidas para C25/C37/C44/C56e/C57

Cada fuente es exactamente la celda de la tabla, sin backticks ni salto inicial.
Los offsets son bytes UTF-8 desde cero, fin excluido. Además del código se
comprueban estos datos; rechazar fn, caso, with, : o -> antes no acredita el caso.

| Referencia | Fila | unexpected_token / primary_span | Prefijo y expectativa exigidos |
|---|---|---|---|
| S25 | C25 | No es error de token; operando x / [48,49) | ClosureExpr con ImplicitClosureBody completo; related_span de => = [45,47). Exigir NX-E1301/implicit_closure_body antes de errores derivados de esa forma. |
| S37 | C37 | create / [25,31) | Firma fn caso() reconocida y with no_alloc, consumido; parser_context = with_restriction_after_comma; expected_tokens = [unrestricted, no_alloc, no_heap_alloc, no_block, no_suspend, no_io], expected_tokens_kind = admissible_set; unrestricted se reconoce aquí para diagnosticar la mezcla. |
| S37c | C37c | no_alloc / [17,25) | Firma fn caso() y with { reconocidos; parser_context = with_operation; expected_tokens = [create, copy, call, resume, cancel, drop, "}"], expected_tokens_kind = admissible_set; } reconoce el mapa vacío que luego rechaza NX-E0417. |
| S37d | C37d | create / [15,21) | Firma fn caso() y with reconocidos; parser_context = with_clause_start; expected_tokens = ["{", unrestricted, no_alloc, no_heap_alloc, no_block, no_suspend, no_io], expected_tokens_kind = admissible_set. |
| S44 | C44 | } / [39,40) | Cabecera de closure y expresión 1 reconocidas; parser_context = statement_terminator; expected_tokens = [";"] como inserción canónica, no conjunto exhaustivo (§2 de D13). |
| S56e | C56e | ) / [22,23) | Firma, with { y call( reconocidos; parser_context = with_restriction_start; expected_tokens = [unrestricted, no_alloc, no_heap_alloc, no_block, no_suspend, no_io], expected_tokens_kind = admissible_set. |
| S57 | C57 | . / [81,82) | StructDecl Dato, declaración x, ClosureExpr y Capture copy x reconocidos; parser_context = capture_after_name; expected_tokens = [",", "]"], expected_tokens_kind = admissible_set. |

C25b, C37b, C37cb y C37db deben completar el análisis del prefijo respectivo
y de todo el módulo con la edición indicada. C44b cambia
explícitamente el control a return y completa la comprobación. Son pruebas
del frontend ampliado; un E0002 genérico de F0-0001 no las acredita.
C56eb/C57b deben reconocer el mismo prefijo y completar la comprobación.
En C53/C53p el span vacío sitúa la inserción justo después del ResultType;
el > siguiente se conserva. En C54/C55 se señala Restrictions completo,
sin delimitadores; C56 señala el OperationList completo, incluidas sus llaves.
Son errores estructurales específicos sobre formas reconocidas, no NX-E0002.

Fuente C48, paso de una función con nombre como callback; la etiqueta
handler coincide con la forma de EX02, sin simular una biblioteca HTTP:

```nexo
fn responder(x: i64) -> i64 with no_alloc, no_block, no_suspend, no_io {
    return x;
}
fn servir<F: Call(i64) -> i64 with {
    call(no_alloc, no_block, no_suspend, no_io)
}>(handler: F) -> i64 with no_alloc, no_block, no_suspend, no_io {
    return handler(1);
}
fn main() {
    let n = servir(handler: responder);
    assert_eq(n, 1);
}
```

Fuentes C46/C47, módulos de biblioteca independientes. El genérico se
comprueba contra la cota sin inventar un tipo concreto de vista:

```nexo
fn pasar<T, F: CallOnce(consume v: T) -> T from(v)
    with { call(unrestricted) }>(consume f: F, consume v: T) -> T from(v) {
    return consume f(consume v);
}
```

```nexo
fn guardar<T, F: Call(inout owner: Option<T>, consume v: T) -> ()
    retains(v, in: owner) with { call(unrestricted) }>
    (f: F, inout owner: Option<T>, consume v: T) -> ()
    retains(v, in: owner) {
    f(inout owner, consume v);
    return ();
}
```

Fuente C22 (Iterator es el protocolo de núcleo de D10):

```nexo
gen fn uno() -> i64 { yield 1; }
fn main() with no_suspend {
    var g = uno();
    var f = fn [consume g]() { return Iterator.next(inout g); };
    let n = inout f();
}
```

Fuente C32, comprobada antes de instanciar F (D08):

```nexo
fn aplicar<F: Call(i64) -> i64 with { call(unrestricted) }>
    (f: F, x: i64) -> i64 with no_io {
    return f(x);
}
fn main() { return (); }
```

Fuente C34, usando aplicar de §5.2. La llamada presta f: el fallo es de
conformidad nominal, no intento de copia ni invocación con prefijo erróneo.

```nexo
fn main() {
    let r = mock_open(1);
    let f = fn [consume r](x: i64) -> i64 { drop(r); return x; };
    let n = aplicar(f, 1);
}
```

Cada defecto requiere al implementar un negativo que alcance su código y un
positivo reparado. C16/C18 conservan el permiso como variable discriminante.
Los casos NO EVALUADO conservan su obligación y código objetivo; faltan
fixtures/arneses, no se consideran dispensados. Ninguna fila acredita que el
compilador actual soporte la sintaxis propuesta.

## Diagnósticos y reparaciones comprobables

El [catálogo único](F0-CODIGOS.md) define códigos, propietarios, subcausas y
mapeo del prototipo. Un par code/subcause conserva su identidad aunque
cambie el idioma del mensaje. P09/T25 exigen `code`, `subcause`, versión de esquema, regla,
severidad, ubicación principal, ubicaciones relacionadas, cadena causal,
operación create/copy/call/drop y rutas de campos/procedencias. Spans: archivo
y bytes UTF-8; línea/columna son presentación. Las causas identifican
declaraciones, no solo nombres impresos.

NX-E1104 ofrece reducir retención o preparar copia solo si satisface duración,
copia e independencia exigidas. NX-E1105 ofrece terminar el préstamo antes del
acceso incompatible. Copiar un actor no repara independencia; transferir cambia
propiedad. Las alternativas con consecuencias se explican, no se aplican como
equivalentes. No se añade unsafe ni se debilitan contratos.

Cada reparación anunciada aplicable se vuelve a comprobar en tipos, propiedad
y efectos y conserva un positivo pareado con su observación. La causa primaria
más próxima precede a derivados: permiso de captura antes que escape posterior.

## Interacciones y alcance por fase

* D13/D01: return explícito, ramas con value:, unificación sin boxing, unión
  conservadora de movimientos/préstamos y exportación sin cambiar identidad.
* D12/D10: efectos latentes separados de creación, cierre de estados
  suspendidos y yield visible en next.
* D08/D09 y §6.1–6.2: propiedades selladas, procedencia, publicación y costes
  transitivos incluso al reexportar o borrar tipos.

**Siguiente prototipo F0 previsto, no implementado aquí:** parser de ambas
formas, llamadas directas, entornos estructurales, copias físicas, recursos
simulados y flags de inicialización; préstamos léxicos y représtamos reducidos;
resúmenes create/copy/call/drop. Modelo finito de campos anidados, afinidad y
fronteras D09 sin ejecutar hilos/atomics. Debe rechazar explícitamente lo no
soportado sin contarlo como evidencia de la regla futura.

F1: núcleo, traits estáticos y diagnósticos; F2: generadores propietarios y CoW;
F3: regiones, proyecciones y despacho dinámico; F5: tareas/publicación reales.
IR/backend, estados D09 completos y tres flujos reducidos siguen pendientes.

| Familia T | Evidencia exigida |
|---|---|
| T01/T02/T08 | Independencia anidada, recurso no duplicable, cierre único, retorno temprano e inicialización parcial. |
| T06 | Capturas incompatibles directas/indirectas; positivo con aislamiento. Modelo F0 separado de ejecución F5. |
| T10 | Escape por closure, campo, error, callback y tarea; positivo de procedencia suficiente. |
| T14 | Cuatro operaciones, alloc indirecto, destructor bloqueante y efectos desconocidos. |
| T22 | Generador no empezado, abandono temprano, recurso capturado y préstamo rechazado correctamente. |
| T17/T18/T25 | Propiedades selladas, confinamiento transitivo, causas y reparación en CLI/LSP/REPL. |

## Riesgos abiertos y estado de revisión

Capturar prestado por defecto puede exigir listas frecuentes al retornar
closures; se medirá comprensión y reparación. Retener permisos hasta destrucción
rechaza algunos usos seguros: relajar esta regla exige revisar T08/T10/T14 y
no cambiar orden observable de recursos.

Los prefijos de llamada y la forma corta necesitan corpus de ambigüedad y
round-trip de formato D01. Los contratos públicos de invocables deben caber
en D08 sin inferencia global. Las pruebas de transferencia de grafos D09 siguen
siendo un riesgo independiente del diseño de tipos.

Las seis revisiones anteriores fueron **FAIL**. Esta iteración 6 no tiene dictamen:
sus cambios y casos previstos están **NO EVALUADOS** por validación nueva.
Siguen pendientes las fixtures señaladas, los dos revisores sin discrepancias
de F0 y la aprobación del propietario. No se aprueba una fase por estas tablas.

## Registro de corrección — iteración 1

Motivos 1/2/10: catálogo único y subcausas; copia y uso tras consumo comunes,
con mapa del prototipo. Motivos 4/5: cotas invocables escritas y dos capas de
closure async, matriz de contextos para copia/consume. Motivo 6: Call* sellados
identificados como ampliación D08. Motivo 8: fuentes concretas y casos sin
fixture expresamente NO EVALUADOS. Las regresiones de diseño C12/C16/C17,
C29/C29b y C31–C33 deben materializarse al implementar; no están ejecutadas.

## Registro de corrección — iteración 2

Motivo 2: NX-E1113/NX-E1114 y NX-E0105 cubren protocolo, cota privada y
receptor; pares C34/C34b, C35/C35b y C36/C36b. Motivos 4/6: ampliaciones
inventariadas y única EBNF WithClause, con errores deliberados C37–C42 y
positivo C43. La composición de closure async distingue anotación de Q y
cota invocable de C. T14/T17/T25 requieren materializar estos casos; no se
han ejecutado. Motivos 1/5 se registran en catálogo/revisión documental.

## Registro de corrección — iteración 3

B5/B6: C37/S37 exige create y su span tras reconocer with; C37b es el positivo.
C44/S44 distingue terminador ausente de caída sin retorno y ofrece retorno
explícito comprobable. B2 remite a la terminación de generadores de D13 §1.1.
N1 añade C45/C45b para NX-E1112; N2 incorpora ProvenanceClauses en CallableBound
y C46/C47/C47b para callbacks. N3 refiere a G de D13 para prefijos, tipos y
declaraciones. N5 añade §5.1 sin renumerar las referencias a §5.2/§5.3.
Las fixtures siguen siendo expectativas de diseño, sin ejecución nueva.

## Registro de corrección — iteración 4

B1: D13 G.2 reconoce ImplicitClosureBody y lo rechaza con NX-E1301;
C25/S25 y C25b fijan span y reparación sin retorno implícito. B2 remite al
inventario transversal de ampliaciones de D13. N2 precisa la cota de una
fn ordinaria pública. N3 eleva la prioridad invalid_operation sobre
missing_body_operation a §5.2, añade C37c/C37d con sus spans y positivos,
y distingue el token canónico de reparación en S44. N4 restaura `return;`.
N1/N5 conservan cobertura pendiente en el catálogo. No hay ejecución nueva.

## Registro de corrección — iteración 5

B1 migra referencias de entrega de rama a value: (D13 §1): value sigue siendo
Name ordinario y la elección **corrige choque con EX01/EX04**.
N3 concreta funciones con nombre como valores sin entorno conformes a Call*;
C48 usa handler: responder y conserva sus efectos. N4 admite capturas explícitas
no usadas con adquisición/retención/limpieza reales (C49–C51b); C04/C08 mantienen
sus diagnósticos. N5 conserva C47b bajo NX-E1319 por retención transitiva indirecta.
Los casos siguen siendo expectativas de diseño, no ejecución ni aprobación.

## Registro de corrección — iteración 6

B1: S37/S37c/S37d y las nuevas S56e/S57 participan en el barrido de listas
cerradas de D13. Las continuaciones de formas reconocidas inválidas se hacen
explícitas sin aceptarlas. N2: G.1 decide método x.m() frente a campo invocable
(x.m)(); C52–C52d cubren receptor mutable/inmutable y agrupación.
N3: C53–C56eb añaden cotas sin with, unrestricted combinado, mapa vacío y
lista interna vacía; las tres primeras tienen subcausas nuevas de NX-E0417
y producciones reconocedoras. C57/S57 aísla captura de proyección y su positivo;
C58/C58m/C58o ejercitan NX-E0110 para cada protocolo Call* resoluble del preámbulo.
N4 corrige la notación de NX-E0202. La captura de campos exige binding previo,
y las formas de llamada nuevas son ampliaciones explícitas de D08.
N1 queda en el catálogo; no se modifica ni se ejecuta Rust, ni se afirma
validación independiente. T07/T14/T17/T25 necesitan materializar estos pares.
