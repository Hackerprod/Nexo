# F0-0005 — Operadores, control y colecciones para programas de consola

Estado: **decisiones de arquitectura revisadas de forma independiente; sin implementación ni aprobación de fase**.

## 1. Autoridad, alcance y orden

Este ADR especifica un corte implementable; no aprueba F0 ni F1. Rigen
[AGENTS.md](../../AGENTS.md), [F0_ACUERDOS](../F0_ACUERDOS.md) §§2–4 y §9,
[la base 0.2](../baseline/Nexo_ROADMAP_rev0.2.md), P01–P09, D01–D10,
EX01–EX04, F0/F1 y T01–T28. La base permanece intacta. Se conservan los
contratos de [F0-0001](F0-0001-first-slice.md), de
[F0-0002](F0-0002-second-slice.md) —A2— y la superficie única B de
[D15](F0-D15-sintaxis-oficial.md). D14 es motivación histórica, no otra
gramática. D11/D12 conservan las reglas de capturas, suspensión y limpieza;
D13 conserva tipos de expresiones, pureza y constantes de módulo.

El dominio nuevo es I64 comprobado completo, lógica Bool, rangos finitos,
iteración de arrays, asignación compuesta, crecimiento de Array y constantes
escalares. No añade texto, tipos nominales de usuario, módulos, interfaces,
concurrencia, métodos de usuario, operadores sobrecargables ni E/S real.
Print sigue siendo la observación simulada de A2. Los algoritmos de §11 son
programas fuente de aceptación, no afirmaciones de ejecución del prototipo.

### 1.1. Orden y fronteras

Orden normativo: F0-0002 completo → F0-0003 → **F0-0005 → F0-0004**.
A2 designa F0-0002 y J designa [F0-0003](F0-0003-third-slice.md);
no son perfiles alternativos.
F0-0005 exige de F0-0003 Option<T>, Some/None y sus propiedades transitivas
para Pop; match para observarlo; closures, generadores y sus permisos;
J13 para el CFG común de `for`, break/continue y NX-E1326; NX-E1104 para
escapes de préstamos. Conserva la iteración de generadores y extiende el
mismo For a rangos y arrays. `for edit` permanece for_iteration/later_adr,
también para generadores. Las enmiendas propuestas de §12 delimitan las
filas de F0-0003 cuyo alcance cambia; no requieren editarlo en este corte.

[F0-0004](F0-0004-text-and-user-types.md) no es prerrequisito. String, nominales, derivación nominal de igualdad,
sus diagnósticos y sus perfiles de texto/miembros quedan a cargo de F0-0004.
Cuando incorpore String o tipos nominales, deberá definir su elegibilidad
transitiva como elementos de Array<T>, su igualdad y, si procede, su orden,
y los costes/diagnósticos de esas extensiones en la tabla central de TypeId.
Para cada tipo que declare copiable e independiente, la elegibilidad de
Array se derivará transitivamente de esas propiedades, sin listas por nombre.
La igualdad de Array exigirá la capacidad de igualdad de sus elementos;
los seis comparadores conservarán la precedencia de tipos distintos de A2.
Los cuatro operadores de orden compartirán el dominio que F0-0004 declare,
sin conceder orden automáticamente por ser nominal.
Este ADR no anticipa qué nominales serán ordenables ni cómo derivarán igualdad.
La extensión conservará copia independiente, permisos, cuotas de búfer y
un único motor; no se mantendrá un Array ni un comparador paralelo para texto.
El presente corte es implementable y validable con I64, Bool, (), arrays de
esos datos, recursos simulados A2 y Option/Result, closures y generadores J.

## 2. Superficie y tipos

### 2.1. Enteros y operadores

Solo I64, intervalo cerrado [-9223372036854775808, 9223372036854775807].
No hay promoción, conversión implícita Bool↔I64, cast, otras anchuras ni
aritmética modular/saturada en este corte. El motivo es completar una
semántica comprobable sin multiplicar anchuras, literales y conversiones.
TryAdd mantiene su contrato recuperable anterior; un fallo de los operadores
ordinarios es R del simulador, no Err ni excepción capturable.

| Operación | Dominio y resultado | Regla |
|---|---|---|
| `+ - *` | I64 × I64 → I64 | Resultado matemático exacto o fallo por desbordamiento. |
| `/` | I64 × I64 → I64 | Cociente truncado hacia cero; cero divisor falla; MIN/-1 falla. |
| `%` | I64 × I64 → I64 | Resto de división truncada, signo del dividendo o cero. MIN%-1 = 0, sin calcular un cociente I64 desbordado. Cero divisor falla. |
| `-x` | I64 → I64 | Negación comprobada; -MIN falla. |
| `< <= > >=` | I64 × I64 → Bool | Orden matemático sin resta intermedia. |
| `== !=` | Dos valores del mismo tipo copiable admitido por A2/J → Bool | Igualdad estructural de A2/J; != niega el resultado, sin segundo recorrido ni derivación adicional. |
| `and or` | Bool × Bool → Bool | Cortocircuito izquierdo; no hay conversión de verdad. |
| `not` | Bool → Bool | Negación lógica. |

Para divisor no nulo y salvo MIN/-1, a=q*b+r en enteros matemáticos,
abs(r)<abs(b). Ejemplos normativos: -7/3=-2, -7%3=-1, 7/-3=-2,
7%-3=1, -7/-3=2, -7%-3=-1. MIN%-1=0 es deliberadamente válido;
no hereda el fallo del operador de cociente del anfitrión. Cero se comprueba
antes que desbordamiento. Debug y release tienen idéntico comportamiento.

Se conserva la regla de literal con signo de D13/D15: `-` inmediatamente
aplicado a un token decimal, con espacios permitidos, constituye el literal
negativo normalizado. Así `-9223372036854775808` es válido. `-(x)` y `-x`
son negación unaria; `-(9223372036854775808)` contiene un literal positivo
fuera de rango y da NX-E0107, antes de ejecutar. `--1` es negación de -1.
El AST distingue literal firmado de Neg; no se repara por patrón de fuente.

De menor a mayor precedencia: or; and; comparación; rango; `+`/`-` binarios;
`*`/`/`/`%`; `not`/`-` unarios; ?; await/acceso/modos/postfijos según D15 §3.
Los binarios aritméticos, and y or asocian a izquierda. Los unarios a derecha.
Comparaciones y rangos no encadenan. `not a < b` es `(not a) < b`, no
`not (a < b)`; el segundo se escribe con paréntesis. `and`/`or` no son
`&&`/`||`; no se introduce `!`. Los operadores no adquieren efectos alloc,
block, suspend o io por sí solos; sus operandos y limpiezas sí pueden tenerlos.

### 2.2. Cortocircuito, propiedad y retorno

Se evalúa la izquierda una vez. En and false y or true se omite la derecha
en ejecución. Cada temporal de la izquierda se limpia antes de seleccionar
la derecha; sus préstamos de llamada terminan conforme A2. El Bool de salida
se construye después de la rama seleccionada. Si la derecha retorna, propaga
Err o falla, no se construye salida. Los efectos estáticos incluyen AMBOS
operandos y sus limpiezas, aunque la izquierda sea literal.

El CFG de propiedad tiene dos bordes, incluso para un Bool literal. Una
transferencia en la derecha puede dejar un origen maybe_consumed al salir;
el camino omitido no adquiere ni libera permisos de esa derecha. Las
restricciones de tipos de la derecha siempre se resuelven. Never no equivale
a Bool ni a incógnita: la operación exige Bool a los caminos con valor.
Se reutiliza la transferencia de estados reales/hipotéticos de I07c/J; no hay
un evaluador estático de expresiones muertas que derive tipos aparte.

`while true { ... }`, admitiendo solo agrupaciones alrededor del literal,
no tiene borde de condición falsa en el CFG alcanzable. No se propaga esa
prueba a aliases, constantes, `not false` ni llamadas. Conserva análisis de
tipos, permisos y efectos de todos los nodos con el mismo motor. Un break
alcanzable crea salida; return, Err y fallo no crean caída normal. La prueba
es estructural, no resuelve terminación general ni supone que una cuota
agotada sea retorno de la función. `while true{return 5;}` satisface ->I64;
`while true{break;}` sin retorno posterior conserva NX-E1317. `while false`
no recibe optimización especial de propiedad. Un for finito nunca demuestra
retorno total por conocer sus extremos: se admite siempre salida normal.

## 3. Control y lugares

### 3.1. Rangos

Se admite `for i in lo..<hi { ... }` y `for i in lo..=hi { ... }`.
Ambos extremos son I64, se evalúan una sola vez, de izquierda a derecha,
antes del primer cuerpo. El binding i es I64 inmutable, nuevo por iteración.
lo>hi da cero iteraciones en ambos; lo=hi da cero/semiabierto o una/inclusivo.
No hay paso configurable ni rango descendente. El inclusivo compara i==hi
antes de avanzar: jamás calcula hi+1. El semiabierto incrementa únicamente
si el valor aún es menor que hi; no desborda en MAX. Una ejecución cuyo
rango sea enorme termina por cuota; no reserva una lista de índices.

El rango solo puede ser iterando directo de for, con agrupaciones permitidas.
No es todavía un valor fuente almacenado, parámetro ni resultado, y no tiene
tipo fuente Range. Es una operación tipada interna de iteración, con ambos
extremos en la tabla de tipos. Expresiones como `let r=0..<3;` reconocen B y
dan range/later_adr; no inventan un tipo inferido.

### 3.2. Arrays y destinos de control

`for x in a` toma una lectura del Array completo durante el bucle; no copia
la colección. Cada x es un alias de lectura al elemento, válido solo durante
la iteración. Un uso propietario de x copia datos de acuerdo con A2; no
transfiere el elemento. Un iterando temporal se posee en el ámbito oculto
del bucle y se destruye al terminar. `for x in move a` mueve a ese ámbito y
después presta sus elementos. Se captura Len una vez. No se permite cambiar
elementos, crecer, Pop, destruir ni sustituir la raíz mientras esté prestada,
ni siquiera por otra proyección; se aplican NX-E1105 y la raíz lógica de A2.
Retener una closure que lee x más allá de la iteración falla por duración
(NX-E1104 de F0-0003). No se introduce una copia defensiva oculta.

Break y continue usan J13: destino léxico más cercano en el mismo invocable,
sin valores ni etiquetas. En for rango, continue limpia el cuerpo y avanza;
en for array libera el alias del elemento y avanza, conservando el préstamo
del array; en while limpia el cuerpo y vuelve a la condición. Break limpia
el cuerpo y el estado del for, libera sus préstamos y sale. Return/`?` hacen
lo mismo antes de la limpieza exterior. Los bordes nuevos participan en el
punto fijo I07b, incluidas transferencias en continue y joins de break.
Un invocable anidado no hereda destino de bucle; fuera de bucle, NX-E1326.

No se añade `loop`: D15 no lo reserva y `while true` tiene ya la semántica
requerida. `loop` puede seguir siendo un identificador. No hay etiquetas ni
`break valor`; una forma ajena a B recibe el error sintáctico correspondiente,
no un feature_id inventado. `for edit` sí es B reconocida y queda diferido.

### 3.3. Asignación compuesta

`+= -= *= /= %=` aceptan un Place mutable I64 y RHS I64, producen sentencia
sin valor. Se amplía Assignment de D15 con los cuatro tokens que faltan.
La raíz y los índices del lugar se evalúan una vez, de izquierda a derecha;
se comprueban disponibilidad, permiso y límites, se adquiere exclusividad,
se carga el escalar viejo y solo entonces se evalúa RHS. Se calcula la
operación comprobada y se instala el resultado; el préstamo termina al salir
de la sentencia, también ante fallo. La exclusividad prohíbe que RHS acceda
incompatiblemente a la misma raíz. `a[0]+=Len(a)` se rechaza; se puede guardar
Len antes. No se desazucara a dos evaluaciones del Place ni a una llamada
ordinaria que cambie el orden. La asignación simple conserva el orden A2
(RHS antes de validación final del destino); la distinción procede de la
exclusividad de la operación compuesta en D08/D13.

### 3.3.1. Límites diferidos

Límite deliberado del prototipo: `x+=x`, `a[i]+=a[i-1]` y `sum+=sum/2`
se rechazan con NX-E1105/active_borrow_conflict sobre la lectura RHS de la
raíz. Se conserva exclusividad desde la carga hasta la escritura, coherente
con D13 G.1 (lugar único y exclusividad) y los préstamos vivos de A2 §6.
La reserva de préstamo en dos fases queda como mejora pendiente de diseño:
permitiría lecturas del RHS durante la reserva y activaría exclusividad al
escribir, con reglas explícitas para llamadas, alias y fallos. La diferencia
ergonómica con `x=x+x`, aceptado, permanece visible; no se introduce esa
reserva implícitamente ni se relaja la exclusividad actual. La alternativa es calcular RHS en un local antes de la
compuesta; para lectura simple también sirve `x=x+x`. Las llamadas conservan
su orden: `Push(edit a,Len(a))` tiene el mismo conflicto; `let n=Len(a);
Push(edit a,n);` lo evita. No se confunde este límite con implementation_limit.

### 3.3.2. Identidades de préstamos

Identidades de los préstamos nuevos: for Array usa
loan:<inicio-del-iterando-sin-modo>:0, con origin sobre el iterando completo;
compuesta usa loan:<inicio-del-Place>:0, origin sobre el Place completo.
Un conflicto denuncia el nombre de la raíz del acceso incompatible, como A2,
con Entity binding/parameter de esa raíz. El alias de elemento tiene su
BindingId de declaración y permission_source del préstamo del iterando,
siguiendo J §8.2. Un représtamo subordinado no cambia la raíz. Continue
termina los permisos del alias de esa iteración antes de crear los siguientes.

## 4. Un único Array dinámico

### 4.1. Valores y API

Array<T> sigue siendo el mismo tipo sin longitud en el TypeId. Todos los
arrays pueden crecer; no se añade FixedArray, List ni adaptación entre tipos.
Los literales y `var a:Array<I64>=[];` construyen Array. No se necesita un
segundo constructor nominal: [] es el constructor vacío contextual.
Esta es una simplificación del prototipo respecto de F0_ACUERDOS §9.4:
el lenguaje completo conserva la obligación de niveles bajo y alto (array
y List construida sobre él). Array dinámico aquí permite validar programas
útiles sin anticipar esa representación ni crear un adaptador permanente;
no decide que List desaparezca del lenguaje completo. La separación posterior
requiere un ADR que defina un arreglo de bajo nivel con extensión y layout
explícitos y una colección dinámica List que posea ese almacenamiento y gestione
longitud, capacidad y crecimiento. List se construirá con las operaciones del
arreglo; ambos usarán la misma tabla de tipos y el mismo motor de propiedad.
Ese corte migrará las operaciones dinámicas y sus fixtures a List de una vez,
sin alias de compatibilidad, adaptadores permanentes ni implementaciones
paralelas de la semántica dinámica. La representación concreta, la sintaxis
y la migración de nombres quedan pendientes de ese ADR.

`let` admite anotación igual que var. I07c reúne restricciones de anotación,
retorno, argumentos y usos posteriores antes de resolver T; `let a=[];`
sin ninguna restricción suficiente sigue dando NX-E0103. Nunca elige I64
como valor por defecto ni infiere T durante ejecución.

Firmas del preámbulo, esquemáticas (T es un parámetro intrínseco sellado;
no habilita genéricos de usuario):

| Nombre | Firma y comportamiento |
|---|---|
| Len | `(a:view Array<T>)->I64`; longitud lógica, O(1), contrato anterior. |
| Push | `(a:edit Array<T>, value:move T)->()`; añade al final, O(1) amortizado para datos de tamaño fijo. |
| Pop | `(a:edit Array<T>)->Option<T>`; vacío None, si no Some del último elemento trasladado fuera. No copia T ni reduce capacidad. |
| JoinArrays | `(left:view Array<T>, right:view Array<T>)->Array<T>`; nuevo valor independiente, orden left seguido de right. |
| indexación | `a[i]`, i:I64; lectura o Place, 0<=i<Len(a), R0102 de A2 si no. |

Los nombres, etiquetas y sombreado siguen la resolución de símbolos del
preámbulo; no se detecta `Push` por el texto del identificador. Los argumentos
se evalúan en orden escrito, con préstamos de A2. Por ello
`Push(edit a,a[0])` tiene conflicto: se prepara antes una copia en otro
binding. `Push(edit a,move x)` transfiere x; `Push(edit a,x)` copia x si es
copiable. La instalación sucede solo después de preparar el argumento y
reservar capacidad; un fallo limpia temporales ya preparados y no instala
el elemento. Pop construye/reserva el tag Option antes de retirar el último
elemento; al fallar esa reserva la colección conserva su contenido.

Los elementos admitidos son I64, Bool, () y Array/Result/Option transitivos
de esos datos. La igualdad y el formato usan exclusivamente los dominios
copiables de A2/J. Siguen excluidos recursos, generadores, closures —incluso
copiables— y permisos retenidos. `Array<MockResource>=[]` también se rechaza:
la elegibilidad depende del tipo, no del contenido actual. No se introduce
extracción de recursos dejando huecos. F0-0003 sigue gobernando recursos en
Option/Result. Pop no habilita `move a[i]` ni Drop de proyecciones existentes.

La copia de un Array es profunda e independiente, incluyendo arrays anidados.
Push/Pop sobre una copia no cambian la otra. Mover transfiere búfer y capacidad
sin copiar elementos. Este corte no introduce RC ni CoW; una futura
optimización D09 debe conservar todos los efectos/costes prometidos o cambiar
explícitamente el perfil. JoinArrays copia cada elemento en orden; el coste incluye
su árbol completo. `+` sigue siendo I64: no concatena arrays ni texto.
Slices, vistas de subrango, capacidad observable, Reserve y Shrink quedan fuera.

### 4.2. Memoria, efectos y limpieza

Representación lógica: cabecera Array ya contada como una celda A2, longitud
N, capacidad C>=N y búfer de C slots. Cada slot reservado cuesta 32 bytes
lógicos de almacenamiento; un slot contiene escalar o handle, nunca duplica
en esos 32 bytes los búferes de arrays hijos. Un slot vacío no es una celda
de valor. Array vacío tiene C=0 y cero bytes de búfer. Literal/copia/JoinArrays
tienen C=N. Push con N=C elige C'=max(1,2*C); si N<C conserva C. No hay
realloc en sitio en el modelo: se reserva íntegro el búfer nuevo mientras el
viejo existe, se trasladan N handles y después se libera el viejo. El cálculo
de N+1, 2*C y 32*C se hace en contador exacto/saturado comprobado, nunca con
overflow del anfitrión. Una solicitud irrepresentable cuenta como mayor que
el límite y da R0002. No se espera a una asignación del anfitrión para decidir.

Dos cuotas adicionales: bytes de búfer vivos y bytes acumulados reservados
durante una ejecución. La primera baja al liberar; la segunda nunca baja.
Copiar recursivamente reserva cada búfer; mover conserva su cargo. Se incluye
el pico de crecimiento. Pop conserva C y sus bytes; Drop libera el búfer.
Los valores temporales y el argumento de Push cuentan en live_value_cells
antes de instalarse. Ni capacidad ni bytes lógicos pretenden medir RSS Rust.

Push tiene alloc conservador aunque haya espacio; JoinArrays y construcción/copia
Array tienen alloc incluso vacías, como A2. Pop/Len/indexación no tienen alloc
intrínseco: Option inline no asigna búfer. Copiar después un payload Array sí
tiene alloc. El resumen drop es transitivo y los préstamos duran hasta sus
salidas reales. No se añade no_heap_alloc. La limpieza conserva orden inverso
de inicialización/ámbitos/parámetros, reservas de cierres y cero ticks; no se
interrumpe por cuota ni produce un segundo diagnóstico. No se promete limpieza
ante aborto/OOM del anfitrión; se mantienen las garantías simuladas A2.

## 5. Constantes y evaluación acotada

Solo módulo privado de un archivo: `const nombre:Tipo = comptime Expr;`.
Los nombres privados de constantes usan camelCase, conforme D15 §2.3.
Tipo explícito I64, Bool o (). La constante puede usarse antes de declararse;
no existe almacenamiento global mutable ni inicializador en runtime. Leerla
produce el mismo escalar independiente que un literal de ese valor, con
origen fuente propio. Una constante no es un Place asignable; sustituirla
usa NX-E0105, declaración relacionada. No se añade const local ni `pub`.
Los nombres const participan en el espacio de declaraciones del módulo;
una colisión usa NX-E0104 con declaration_kind="constant" cuando la segunda
declaración es const, y previous_declaration de la primera. Leer k en un
argumento propietario copia su escalar; Drop(k) descarta ese temporal y no
invalida la constante. `move k`/`edit k` solicitan modificar o retirar
almacenamiento constante que no concede ese permiso: NX-E0105,
ownership/place.write, primario sobre k, data {}, relación declaration sobre
su nombre declarado y cadena vacía. Se reutiliza el código de modificación
sin permiso mutable de A2 §§13.12/13.15; no se introduce domain:place.
La lectura propietaria sin modo sigue siendo válida. Esta regla no prohíbe
mover un binding let propietario: una constante carece de ese almacenamiento.
La asignación a k conserva el mismo diagnóstico de inmutabilidad.


Se admiten literales, referencias a constantes, operadores de §2, IfExpr B
y llamadas directas a funciones ordinarias del módulo con parámetros y
resultado escalares. Sus cuerpos pueden usar locales escalares, llamadas
recursivas, while/for rango, control y return. Deben ser puros transitivamente:
sin alloc/block/suspend/io, efecto desconocido, recursos ni permisos externos.
Modificar un local escalar o un parámetro edit sobre un local del evaluador
no es estado global compartido y está permitido. La comprobación considera
todas las ramas, no solo las elegidas por un valor constante. Una llamada
impura da NX-E1316 antes de evaluar, incluso detrás de false and.

Arrays, Option/Result, closures, generadores e intrínsecos del preámbulo en el dominio transitivo de una evaluación const quedan fuera del
subconjunto escalar. Primero se denuncia impureza real cuando exista; un caso
puro que solo necesita esas capacidades da comptime/later_adr.
`comptime` fuera del inicializador de const queda diferido. Esta es una
restricción del corte, no una redefinición de pureza del lenguaje completo.

El grafo de dependencias de constantes incluye referencias transitivas a
través de funciones y ramas no ejecutadas. Sus nodos son únicamente constantes:
una referencia desde el cuerpo transitivamente llamado induce una arista
entre constantes, sin insertar la función en `cycle` ni en las relaciones
dependency. La exploración de funciones usa identidades y un conjunto visitado
para terminar ante recursión, con cargo a type_solver_steps. Cada SCC cíclica emite una sola
NX-E1323/constant_cycle durante types/types.check. Se elige el ciclo simple
con menos aristas; entre empates, cada ciclo se rota para empezar por su
menor offset de declaración y se elige la secuencia lexicográficamente menor
de offsets. El primario es ese primer nombre; los demás nombres del ciclo
se relacionan como dependency, en orden fuente. Data
`{"cycle":["a","b",...,"a"]}` enumera ese ciclo cerrado, no todos los nodos
del SCC. Cada visita de nodo/arista de SCC y búsqueda de ciclo se carga a
type_solver_steps; sin saldo no se publica un ciclo no demostrado. Los
registros de constantes/dependencias también cuentan en ir_items. Un SCC
independiente produce su propio diagnóstico. Recursión de funciones sin referencia circular a
constantes no es un ciclo de constantes: la acotan pasos y profundidad.

Se ejecuta la misma IR tipada y verificada, con el mismo dispatcher de
operadores/frames y una política de capacidades escalares. No hay host eval,
evaluador de texto, otro checker ni ejecución parcial antes de verificar
efectos. La fase lower incorpora una subetapa de evaluación const posterior
a types/ownership/effects y a verificación de IR. Errores aquí impiden emitir
un artefacto ejecutable. Una caché de constantes pertenece a la compilación;
el contador global no se reinicia por constante ni por llamada. El orden es
topológico, desempate por offset de nombre. El hash de función incluye valor
y dependencias de constantes usadas; no hay caché persistente sin identidad
de perfil, fuente e IR de dependencias.

## 6. IR, verificación y un único motor

La tabla central de TypeId/ExprId del checker es la única autoridad. Añade
operadores tipados enum (no cadenas), las firmas intrínsecas de Array y
constantes resueltas. Array no cambia de identidad. For tiene una estrategia
resuelta: rango I64, Array leído o generador de J13; no hace despacho por tipo
del valor en runtime. Rango guarda extremos I64, inclusividad y cursor en
slots de control internos; no expone un tipo de usuario ni materializa Array.

And/or bajan a BranchBool y join de valor; for a bloques de entrada, prueba,
cuerpo, avance y salida. Las instrucciones fuente conservan un OriginId y
sus cargos, sin cobrar ramas administrativas. While true elimina solo su
borde falso alcanzable. Los estados hipotéticos recorren los mismos nodos
y transferencias para errores independientes. Break/continue son Jump con
destino y plan de limpieza explícitos, no excepciones del intérprete.

Operaciones lógicas nuevas: CheckedMul/Div/Rem/Neg, BoolNot, orden e igualdad
negada; ArrayPush/Pop/JoinArrays; BeginArrayIteration y avance por índice;
PrepareCompoundPlace/Load/CheckedOp/Store/EndLoan; LoadConst. Se reutilizan
CopyValue/TransferValue/BeginLoan/EndLoan y flags de vida. No se reinterpreta
AST al ejecutar. El dispatcher mantiene frames explícitos; recorridos de
tipos, copias, comparación, limpieza y grafos const son iterativos.

Además de A2/J, el verificador independiente del lowering debe rechazar:

| Obligación | Mutación que debe detectar |
|---|---|
| Todos los operandos y resultados concuerdan con la tabla de tipos | Neg Bool, Div Array, Push con T distinto, tag Pop ajeno a Option<T>. |
| Cortocircuito y limpieza preceden al join | RHS ejecutado en ambos bordes, temporal izquierdo vivo sin plan, valor de join sin definir. |
| Todo ciclo conserva punto fijo y destino | Continue al cuerpo sin avance, break que omite limpieza, salto a otro frame. |
| Rango inclusivo termina sin hi+1 | Incremento sin prueba final cuando hi=MAX; extremo reevaluado en backedge. |
| Préstamos Array abarcan todos sus accesos | Store/Push/Pop bajo lector de for; alias de elemento después de EndLoan. |
| Lugar compuesto se prepara una vez | Índice duplicado, Store previo a CheckedOp, exclusividad ausente durante RHS. |
| Capacidad, contadores y propietarios son coherentes | C<N, celda retirada aún viva en origen y Some, bytes sin reserva o reserva doble. |
| Constante es independiente y verificada | LoadConst sin valor/tipo, operación prohibida, hash que omite dependencia. |
| Cuotas y efectos no se pierden al bajar | Tick de nodo omitido/duplicado, alloc de Push eliminado por capacidad conocida. |

Una IR inválida es fallo interno, nunca NX de usuario ni artefacto ejecutable.
Las mutaciones de IR se prueban separadas de los negativos de fuente.

## 7. Perfil y cuotas

### 7.1. Decisión del presupuesto por defecto

Perfil único del corte completo: `f0-0005-b-v1`, modelo de efectos
`f0-0005-sim-v1`, namespace runtime histórico `F0-0002-sim`. No hay modo de
compatibilidad de sintaxis. El presupuesto de trabajo runtime sube de
1.000.000 a **16.000.000**. Los demás límites A2/J no se relajan. El perfil
anterior sigue siendo referencia histórica y test de coste, no otra ruta
de ejecución del lenguaje.

Datos matemáticos reproducibles, sin atribuir una medición a la CLI:
Collatz(27) necesita 111 transiciones, 41 impares y 70 pares, máximo 9232.
Con multiplicación/división/resto nativos no necesita decenas de miles de
iteraciones emulando esas operaciones. Por ello su agotamiento
no demuestra por sí solo que 1.000.000 sea insuficiente con este corte.
Sí limita trabajo ordinario: la familia literal A2
`var i=0;while i<k{i=i+1;}` cuesta 14*k+14 ticks completos, de modo que
k=71.427 cuesta 999.992 y k=1.000.000 cuesta 14.000.014. El nuevo límite
permite este último caso y sigue poniendo una cota finita al bucle infinito.
Se elige 16 millones para esa carga de consola, no por una estimación de
velocidad del anfitrión. Tiempo real y RSS se medirán aparte, sin prometer SLA.

| Cuota | Límite |
|---|---|
| source_bytes; syntax_nesting; expression_tree_depth | 1.048.576; 128; 128 |
| array_type_depth; value_type_depth; function_count | 128; 128; 1.024, incluidos invocables J |
| type_solver_steps | 1.000.000 global |
| ownership_flow_steps | 1.000.000 por invocable y 4.000.000 global |
| effect_fixpoint_steps; ir_items | 1.000.000 global; 262.144 global |
| Trabajo runtime; call_frames; live_value_cells | 16.000.000; 256; 65.536 |
| observation_bytes, emitidos más reservas | 65.536 |
| array_storage_bytes, búferes vivos | 8.388.608 |
| array_allocation_bytes, suma de reservas de búfer | 67.108.864 |
| comptime_steps, global por compilación | 100.000 ticks del mismo modelo escalar |
| comptime_depth | 64 frames activos de evaluación, incluido inicializador |
| comptime_memory_bytes | 1.048.576 bytes lógicos vivos: 32 por celda escalar, más 32 por frame, más 32 por constante cacheada |

El inicializador const cuenta como frame pero no como función fuente. Su
entrada cuesta un tick y entregar el valor no introduce Return sintético.
La caché cobra su celda antes de liberar el resultado temporal; se mide el
pico. Leer un valor cacheado cobra expresión+celda igual que literal.
En const no se usan contadores runtime ni observaciones. El límite de pasos
const se comprueba antes de reservar frame/memoria. Tipos, propiedad y efectos
conservan sus contadores globales incluso al construir IR para const.

Prioridad de reserva runtime por operación: trabajo, frames, celdas,
array_storage_bytes, array_allocation_bytes, observaciones. Primero validar
la operación (p.ej. divisor no cero) después de evaluar operandos; luego
reservar su resultado. Una operación fallada no compromete reservas. Las
reservas de llamadas y de sus hijos son operaciones distintas ya ordenadas
por A2. Limpieza libera almacenamiento y celdas sin devolver trabajo,
asignación acumulada ni bytes ya observados.

El crecimiento concede sus N ticks de traslado antes de modificar ningún
handle: cada unidad pasa por la guarda ordinaria y un rechazo conserva los
ticks ya concedidos, sin trasladar nada. Solo después reserva el búfer y
traslada los handles sin cargos adicionales; así no hay crecimiento medio
comprometido por una cuota. Copias profundas/JoinArrays sí pueden construir un
prefijo temporal: lo limpian si falla una reserva posterior, sin modificar
las entradas.

### 7.2. Coste exacto nuevo

Se conservan todos los ticks de A2/J para el dominio anterior. Tabla aditiva
salvo donde se indica que sustituye la operación completa; hijos siempre
cobran sus nodos fuente una vez. Reservar o liberar bytes no cobra ticks
adicionales salvo los recorridos explícitos siguientes.

| Operación | Trabajo |
|---|---|
| *, /, %, orden nuevo | Igual que binario A2: nodo 1, hijos, resultado 1; comparación escalar además visita de par 2, como <. |
| Neg, not | Nodo 1, hijo, resultado 1; literal negativo normalizado conserva coste de literal. |
| != | Exactamente coste de ==, sin tick extra de not. |
| and/or | Nodo 1, izquierda, derecha solo si tomada, celda Bool de salida 1 si hay salida. |
| For rango | Introductor 1, extremos; construir cursor y límite 1 celda/tick cada uno; cada prueba 1; cada binding de índice 1 celda/tick; cada avance efectivo 1. El avance terminal inclusivo solo prueba, no incrementa; esa prueba cuesta 1. |
| For Array | Introductor 1, iterando; cursor y longitud 1 celda/tick cada uno; cada prueba 1; alias de elemento 0 celdas/ticks; cada avance 1. |
| break/continue | 1, como J13. |
| Compuesta | Sentencia 1, preparar Place con cargos A2, cargar viejo 1 celda/tick, RHS, operación 1, resultado 1; escribir/liberar cero. No cobra un nodo de nombre sintético. |
| Push | Llamada/nombre/entrada/argumentos A2; crecimiento: 1 por handle existente trasladado; instalar elemento 0; retorno unidad 1. |
| Pop | Llamada/nombre/entrada/argumentos A2; tag Some/None 1; mover payload 0. No unidad adicional. |
| JoinArrays | Llamada A2, cabecera de salida 1 y copia por cada celda de elementos, preorden izquierda y derecha. |
| LoadConst runtime | Nodo 1 y celda escalar 1; no trabajo de compilación cargado en runtime. |

Un For semiabierto terminado normalmente realiza N+1 pruebas de entrada y
N incrementos. El inclusivo vacío realiza una prueba; con N>0 realiza N
pruebas de entrada, N pruebas de final de cuerpo y N-1 incrementos: la última
prueba final salta directamente a salida. Continue pasa por esa misma prueba
final. Break no realiza avance ni
prueba final. Se cuenta la coexistencia de cursor/límite y binding escalar;
el Array prestado no duplica celdas, su cursor/longitud sí. Los slots de
control se destruyen al salir. Los cambios administrativos de CFG no añaden
celdas. Un fallo de resultado usa span de expresión; un cargo sintético de
for usa token for, y crecimiento usa llamada completa de Push.

Los extremos se leen y sus temporales permanecen hasta copiar sus escalares
al cursor/límite; esas dos copias son los dos cargos de celda de la tabla.
Después se limpian los temporales de extremos. No se cobran simultáneamente
dos modelos alternativos de mover/copiar esos slots. Len interno de for
Array se captura como escalar sin una llamada fuente ficticia.

### 7.3. Perfiles de frontera y diagnósticos de cuota

Se conservan los ocho perfiles reducidos J, sustituyendo el prefijo por
`f0-0005-b-test-` y manteniendo sus sufijos `types-32-v1`, `flow-32-v1`,
`flow-global-32-v1`, `effects-32-v1`, `ir-32-v1`, `frames-8-v1`,
`cells-64-v1`, `observations-64-v1`. Se añaden solo:
`work-1000000-v1`, `storage-96-v1`, `allocation-96-v1`,
`comptime-steps-32-v1`, `comptime-depth-8-v1`, `comptime-memory-96-v1`.
Cada uno cambia únicamente la cuota nombrada del perfil completo; no acepta
un límite arbitrario por CLI. Se seleccionan mediante el mecanismo de perfiles
de prueba A2, nunca mediante directivas dentro de la fuente.

R0001 mantiene limit/observed/work_consumed y primera operación rechazada.
R0002 amplía runtime_budget_kind con los dos nombres de bytes Array; observed
es el total candidato completo, no limit+1 artificial. Se heredan campos,
primarios y operación runtime.limit de A2. NX-E0003 para const usa stage lower,
operation analysis.limit, data exacta budget_kind de los tres comptime_*,
budget_scope="compilation", limit, observed=limit+1, incomplete_analysis="lower"; sin
analysis_function ni relaciones/cadena. El primario es el nodo cuyo cargo
falla, o nombre del const para su frame/caché. No se sustituye por NX-E1316.
La excepción de offset portable de A2 solo sigue aplicando a los cuatro
presupuestos de análisis costosos allí enumerados, no a estas cuotas const.

## 8. Diagnósticos y fronteras cerradas

### 8.1. Registro propuesto

Las siguientes son adiciones propuestas a F0-CODIGOS, no una edición de ese
catálogo. R0101/R0103 conservan suma/resta; R0102 índices; NX-E1326 se toma
de F0-0003. R0501–R0505 se reservan aquí, sin cambiar códigos históricos.

| Código / subcause | Etapa / operation | Primario y data |
|---|---|---|
| R0501 / null | runtime / value.mul | Expresión completa que desborda; {}. |
| R0502 / null | runtime / value.div | División completa por cero; {}. |
| R0503 / null | runtime / value.div | MIN/-1 completo; {}. |
| R0504 / null | runtime / value.rem | Resto completo por cero; {}. |
| R0505 / null | runtime / value.neg | Negación completa de MIN; {}. |
| NX-E1327 / arithmetic_fault | lower / const.evaluate | Operación completa fallada; {"simulator_code":"R0502"}, con el código aritmético aplicable R0101/R0103/R0501–R0505. |
| NX-E1316 / null | effects / const.check | Primer sitio impuro de cada inicializador transitivo; {"reason":"impure_operation"}; una relación constant sobre su nombre. |

NX-E1327 amplía el área 13 de constantes; no significa error de tipos
ni agotamiento y no abre una familia nueva. En compuesta el primario R
es Place+operador+RHS, excluido terminador. No se construye un span ficticio
de binario desazucarado. No hay R para MIN%-1. Los mensajes humanos explican
la operación; no son oráculo de identidad. NX-E0102 usa la prioridad A2:
dominio intrínseco incompatible sobre operando causante, found_type asociado;
igualdad con tipos iguales no comparables sobre operador, expected_type="domain:copyable_data".
And/or/not exigen Bool; aritmética I64. Los rangos `..<` y `..=` en
for directo exigen I64 en ambos extremos: NX-E0102/type_mismatch señala el
primer operando incompatible de izquierda a derecha, con expected_type=I64,
found_type de ese operando y única relación found_type sobre él; no hay
relación expected_type para este dominio intrínseco. No se aplica la prioridad
del operando derecho de las comparaciones: F5-B03 señala true en `true..<3`;
F5-B09 señala true en `1..<true`. Para `< <= > >= == !=`, tipos
concretos distintos denuncian siempre el derecho, expected_type del izquierdo,
relaciones expected_type al izquierdo y found_type al derecho (A2 §§4.6,
13.15 y 13.17). Tipos iguales inadmisibles denuncian el operador: I64 para
orden, domain:copyable_data para igualdad, found_type sobre el izquierdo.
Never conserva la regla de A2 §4.6. No se añade domain:ordered_scalar.
Las reglas de cardinalidad y
barreras A2 se mantienen, incluida ausencia de errores derivados.

NX-E1316 transitivo se ancla en la primera llamada del inicializador que
alcanza impureza; cause_chain usa los registros de llamadas/evidencia de A2,
en orden causal, hasta la operación impura. Para operación impura directa la
cadena es vacía. No se ejecuta nada para producirlo. La ausencia de pureza
de un camino no se suprime por cortocircuito. Un ciclo se denuncia antes de
evaluar, en types/types.check, conforme al registro de NX-E1323.

Extensiones cerradas del esquema: Diagnostic.operation añade value.mul,
value.div, value.rem, value.neg, const.evaluate y const.check;
RelatedSpan.role añade constant y dependency. Data añade
simulator_code exclusivamente a NX-E1327, reason exclusivamente a NX-E1316
con valor impure_operation, y cycle exclusivamente a constant_cycle. No se
reutiliza una Entity binding para un const: la entidad declarada es
constant:<offset-del-nombre>, kind="constant"; el esquema Entity añade ese
kind. declaration_kind añade constant para NX-E0104.
No se añade data.profile_id: el perfil está en Diagnostic.profile_id,
conforme a la lista cerrada Data de A2 §13.6. El ciclo de constantes no
altera recursive_value_layout de J. Explain registra
NX-E1327 y los códigos activados con fuentes pareadas de este ADR y
example_profile_id=f0-0005-b-v1; no anuncia una capacidad como implementada
por existir su entrada. El runtime mantiene su catálogo de fallos separado.

### 8.2. Tabla cerrada de pending_scope

No se crean feature_id: siguen siendo los 30 de D15. Para limitación incluida
pero aún no realizada se usa NX-E0006, feature_id=implementation_limit y
data exactamente {"pending_scope":valor}. Valores emitibles en este perfil:
L01…L10 para implementation_limit; later_adr para los feature_id de alcance.
L11/L12 son verificación, no excusas para rechazar un programa. No se emiten
I/J como pendientes: son prerrequisitos. Todo rechazo de alcance precede a ejecución; errores de
sintaxis reales conservan su barrera anterior.

| Capacidad | feature_id / pending_scope | Ancla y etapa |
|---|---|---|
| Operadores nuevos pendientes | implementation_limit / L01 o L02 | Operador, types; L01 aritmética/orden, L02 lógica. |
| Compuesta pendiente | implementation_limit / L03 | Operador, types. |
| Divergencia literal pendiente | implementation_limit / L04 | while literal true en función no unitaria sin retorno final, ownership; se decide por CFG, no por texto. |
| For rango pendiente | implementation_limit / L05 | for, types. |
| For Array pendiente | implementation_limit / L06 | for, types. |
| Push/Pop/JoinArrays pendiente | implementation_limit / L07 | Destino resuelto de llamada, types. |
| Cuotas nuevas aún pendientes | implementation_limit / L08 | Primer nodo de capacidad nueva, lower. |
| Const tipos/pureza pendiente; evaluación pendiente | implementation_limit / L09; L10 | const, types; inicializador, lower, respectivamente. |
| Rango fuera de for directo | range / later_adr | Operador de rango, types. |
| Índice rango sobre Array | range / later_adr | Selector [rango] completo, types; precede al rechazo range de ese hijo. |
| for edit | for_iteration / later_adr | edit, parse. |
| Const puro no escalar / intrínseco puro | comptime / later_adr | Tipo explícito o primer nodo que excede el dominio, types después de resolver sus tipos; impureza conocida se difiere a effects y tiene precedencia dentro de esta capacidad. |
| comptime fuera de const | comptime / later_adr | comptime, parse. |
| pub const | modules_visibility / later_adr | pub, parse; la exportación de constantes es una forma reconocida pero aún fuera del corte. |
| Array de closure transitiva | closure / later_adr | Construcción o anotación determinante, types, igual frontera J. |
| Array de recurso/estado | resource_array / later_adr | Construcción completa o anotación determinante, types. |
| move/Drop de proyección existente | partial_transfer / later_adr | Proyección, arguments, context sobre modo según A2/D15. |
| Otras anchuras / conversiones no definidas | unsupported_type / later_adr para tipo | Tipo, types; un nombre de función de conversión ausente es NX-E0101, no una API fingida. |
| async, region, default_argument, traits_generics, float_literal, tuple, no_heap_alloc, function_value, inferred_non_unit_result, provenance_clauses | Identificador existente / later_adr | Sitio/etapa D15/J para su dominio todavía excluido. |
| text, user_types, methods, modules_visibility, attributes, extensions | Identificador existente / later_adr | Formas excluidas de F0-0003 §2.2; Iterator.Next conserva su excepción intrínseca. F0-0004 definirá sus extensiones. |

La frontera de valores const no escalares se registra como obligación pendiente durante
types y solo se emite si effects termina sin impureza en ese inicializador;
no se ejecuta código ni se abre una segunda pasada de tipos. Los otros errores
de types siguen bloqueando effects. Esta precedencia local explícita evita
clasificar Print como una limitación de representación en lugar de impureza.
Un const local no es B: error sintáctico, no extensión implícita. Tras cerrar
F5-L01…F5-L10, sus rechazos desaparecen; el feature_id no es una manera de aprobar
un compilador que omite la comprobación de la capacidad implementada.

Operation de implementation_limit se fija por etapa, como J: parse→syntax.parse,
types→types.check, ownership→ownership.check, lower→ir.lower. Su message es
exactamente `Capacidad pendiente de implementación en {pending_scope}.`.
Los límites definitivos heredados conservan operation de su fila J/D15.
Los IDs documentales F5-L01…F5-L12 corresponden a las subtareas de §10;
el valor serializado pending_scope elimina únicamente F5-: L01…L10.
El prefijo documental F5- denota F0-0005, nunca la fase F5 del roadmap.
Los pending_scope L01…L10 pertenecen al perfil f0-0005-b-v1; las fixtures
L01/L02 de F0-0003 son IDs de casos de otro documento, no esos ámbitos
temporales. Toda referencia cruzada debe incluir documento o perfil.
No se emiten pending_scope F5-L01 ni F0-0005-L01.
La selección de varias capacidades en la misma barrera usa menor offset,
desempate L numérico y después pending_scope en orden de bytes UTF-8.
Un for no edit cuyo iterando tiene un tipo conocido distinto de Array,
generador o rango directo es NX-E0102/type_mismatch sobre el iterando:
expected_type="domain:iterable", found_type de la tabla del checker, solo
found_type relacionado. No es una limitación de representación que deba
ocultarse con for_iteration. domain:iterable se conserva del vocabulario cerrado
de dominios diagnósticos de J §2.2, sin crear un trait ni tipo fuente Iterable.

## 9. Fixtures de diagnóstico y oráculos de frontera

### 9.1. Campos comunes y ubicación

Cada fuente de tabla es un archivo `case.n`, ASCII, sin LF final; intervalos
de bytes UTF-8 semiabiertos. No contiene marcadores. La columna token es el
fragmento exacto de fuente para verificar ubicación, NO unexpected_token de
un error semántico. La sustitución positiva se aplica solo al sitio indicado.
Todos los negativos de §9.2 producen exactamente un diagnóstico; positivos
check/run producen cero, outcome unit, exit 0, stdout/events/observations [],
bytes/reservas 0, salvo salida expresamente indicada. Check negativo:
rejected/1; un R tiene check válido y run simulator_fault/1. Fallo const
rechaza tanto check como run sin comenzar Main.

Campos completos comunes, con sustitución de código/subcausa/etapa/operación
de la fila: schema_version=1, profile_id=f0-0005-b-v1,
diagnostic_namespace=Nexo-proposed-v1 para NX y F0-0002-sim para R,
severity=error; rule=código[/subcausa]; explanation_id=rule para NX y null
para R; message es texto humano no contractual; primary_span={file:"case.n",
start:a,end:b} en la representación Span de A2. related_spans/cause_chain=[]
salvo fila; feature_id=null salvo alcance; unexpected_token=null,
expected_tokens=[], expected_tokens_kind=null, parser_context=null,
recognized_prefix=null, repair=[]. Subcause null salvo explícita. Data exacta
sin claves adicionales. Las relaciones se ordenan por archivo/inicio/fin/rol.
No se permiten diagnósticos de sintaxis en estos negativos semánticos.

### 9.2. Pares mínimos literales

| ID | Fuente negativa completa | Código; primario/token | Data / relaciones; edición positiva |
|---|---|---|---|
| F5-O01 | `fn Main(){let x=9223372036854775807*2;}` | R0501; [16,37), `9223372036854775807*2` | {}; cambiar 2 por 1. |
| F5-O02 | `fn Main(){let x=7/0;}` | R0502; [16,19), `7/0` | {}; cambiar 0 por 2. |
| F5-O03 | `fn Main(){let x=-9223372036854775808/-1;}` | R0503; [16,39), `-9223372036854775808/-1` | {}; cambiar divisor -1 por 1. |
| F5-O04 | `fn Main(){let x=7%0;}` | R0504; [16,19), `7%0` | {}; cambiar 0 por 2. |
| F5-O05 | `fn Main(){let x=-9223372036854775808;let y=-x;}` | R0505; [43,45), `-x` | {}; cambiar inicializador x por -1. |
| F5-O06 | `fn Main(){let x=true*2;}` | NX-E0102/type_mismatch; [16,20), `true` | expected_type I64, found_type Bool; found_type [16,20), sin expected_type fuente; cambiar true por 1. |
| F5-O07 | `fn Main(){let x=not 1;}` | NX-E0102/type_mismatch; [20,21), `1` | expected_type Bool, found_type I64; found_type [20,21); cambiar 1 por true. |
| F5-O08 | `fn Main(){let x=true and 1;}` | NX-E0102/type_mismatch; [25,26), `1` | expected_type Bool, found_type I64; found_type [25,26); cambiar 1 por false. |
| F5-C01 | `fn Main(){let x=1;x+=2;}` | NX-E0105; [18,19), `x` | {}; declaration [14,15); ownership/place.write; cambiar let por var. |
| F5-C02 | `fn F()->I64{while true{break;}}fn Main(){}` | NX-E1317; [30,31), `}` | {}; expected_type [8,11); ownership/flow.return; cambiar break por return 5. |
| F5-C03 | `fn Main(){break;}` | NX-E1326/break_outside_loop; [10,15), `break` | {}; types/types.check; envolver break; en while true{break;}. |
| F5-C04 | `fn Main(){continue;}` | NX-E1326/continue_outside_loop; [10,18), `continue` | {}; types/types.check; envolver continue; en while false{continue;}. |
| F5-A01 | `fn Main(){let a=[];}` | NX-E0103; [16,18), `[]` | {}; types/types.infer; insertar :Array<I64> después de a. |
| F5-A02 | `fn Main(){var a=[1];Push(edit a,true);}` | NX-E0102/type_mismatch; [32,36), `true` | expected_type I64, found_type Bool; expected_type [17,18), found_type [32,36); cambiar true por 2. |
| F5-D01 | `fn Main(){let r=0..<3;}` | NX-E0006; [17,20), `..<` | feature_id range, pending_scope later_adr; types/types.check; sustituir 0..<3 por 0+3. |
| F5-D02 | `fn Main(){var a=[1];let b=a[0..<1];}` | NX-E0006; [27,34), `[0..<1]` | feature_id range, pending_scope later_adr; types/types.check; sustituir 0..<1 por 0. |
| F5-K01 | `const k:I64=comptime 7/0;fn Main(){}` | NX-E1327/arithmetic_fault; [21,24), `7/0` | simulator_code R0502; lower/const.evaluate; cambiar 0 por 1. |
| F5-K02 | `const k:()=comptime Print(1);fn Main(){}` | NX-E1316; [20,25), `Print` | reason impure_operation; constant [6,7); effects/const.check; sustituir Print(1) por (). |
| F5-K03 | `const a:I64=comptime b;const b:I64=comptime a;fn Main(){}` | NX-E1323/constant_cycle; [6,7), `a` | cycle ["a","b","a"]; dependency [29,30); types/types.check; sustituir el último a por 1. |

F5-O06–F5-O08/F5-A02 usan types/types.check; los R usan operaciones §8.1.
En F5-A02, expected_type relaciona el elemento `1` [17,18), que fija T=I64,
no el array `[1]` [16,19), cuyo tipo es Array<I64> (A2 §13.15).
Las tablas de A2/D15 conservan todos sus campos para reglas reutilizadas.
Cada sustitución positiva solo repara el atributo aislado, sin borrar una
operación para convertirla en otro caso. Para operadores restantes, las
siguientes familias literales definen materializaciones sin oráculo del
compilador: `fn Main(){let x=1 OP true;}` para OP de `-`, `*`, `/`, `%`, `<`,
`<=`, `>`, `>=`, `==`, `!=`; found_type/primario inicia en 19+len(OP) y
termina cuatro bytes después. Tipos esperados I64, encontrados Bool;
orden e igualdad añaden expected_type [16,17) por el izquierdo;
los operadores aritméticos no añaden esa relación.
Positivo sustituye true por 2. Para `and`/`or`, usar
`fn Main(){let x=true OP 1;}`: primario/found_type [22+len(OP),23+len(OP)),
dominio Bool sin expected_type fuente; positivo 1→false. Cada familia
materializada debe publicar fuente y JSON esperado antes de ejecutar tests.
Para los seis comparadores, F5-OP-R usa `fn Main(){let x=true OP 1;}`:
primario/found_type [22+len(OP),23+len(OP)), expected_type=Bool,
found_type=I64, expected_type [16,20). El positivo sustituye true por 2,
conservando el operador. En F5-OP-L (`1 OP true`) el positivo sustituye true
por 2 y conserva expected_type=I64 en el negativo. En ambos, types/types.check,
NX-E0102/type_mismatch, cadena vacía y solo esas dos relaciones. Aritmética
con Bool izquierdo usa el dominio I64 y primario izquierdo [16,20), sin
relación expected_type. Tipos distintos no se convierten en error de dominio
sobre el izquierdo aunque el tipo de este sea Bool.


Pares adicionales de permisos, efectos y fronteras, con el mismo contrato
de archivo/campos y exactamente un error. F5-B01/F5-B02 usan
ownership/ownership.access; F5-B03/F5-B09 types/types.check; F5-B04/F5-B06 ownership/place.write;
F5-B05 effects/intrinsic.call; F5-B07 types/types.check; F5-B08 parse/syntax.parse.

| ID | Fuente negativa completa | Código; primario/token | Positivo |
|---|---|---|---|
| F5-B01 | `fn Main(){var a=[1];for x in a{a[0]=2;}}` | NX-E1105/active_borrow_conflict; [31,32), `a` | Sustituir solo el iterando a por [1]. |
| F5-B02 | `fn Main(){var a=[1];a[0]+=a[0];}` | NX-E1105/active_borrow_conflict; [26,27), `a` | Sustituir RHS a[0] por 2. |
| F5-B03 | `fn Main(){for i in true..<3{}}` | NX-E0102/type_mismatch; [19,23), `true` | true→0. |
| F5-B04 | `fn Main(){let a=[1];Pop(edit a);}` | NX-E0105; [29,30), `a` | let→var. |
| F5-B05 | `fn F(a:edit Array<I64>) with no_alloc{Push(edit a,1);}fn Main(){}` | NX-E0412; [38,42), `Push` | Quitar solo with no_alloc. |
| F5-B06 | `const a:I64=comptime 1;fn Main(){a=2;}` | NX-E0105; [33,34), `a` | Sustituir a=2 por let x=2. |
| F5-B07 | `fn Main(){let a:Array<MockResource>=[];}` | NX-E0006; [16,35), `Array<MockResource>` | MockResource→I64. |
| F5-B08 | `fn Main(){for edit x in [1]{}}` | NX-E0006; [14,18), `edit` | Quitar edit. |
| F5-B09 | `fn Main(){for i in 1..<true{}}` | NX-E0102/type_mismatch; [23,27), `true` | true→3. |

Metadatos exactos (Entity se expande como {"kind":k,"id":id}; cada Cause
contiene kind, operation, span, entity y evidence="verified"):

| ID | Data | Related spans | Cause chain / feature_id |
|---|---|---|---|
| F5-B01 | origin=binding:14, binding=binding:14, loan=loan:29:0, active_permission=read, requested_access=write | declaration [14,15), origin [29,30) | Un borrow/borrow.read [29,30), Entity loan:29:0; feature null. |
| F5-B02 | origin=binding:14, binding=binding:14, loan=loan:20:0, active_permission=inout, requested_access=read | declaration [14,15), origin [20,24) | Un borrow/borrow.inout [20,24), Entity loan:20:0; feature null. |
| F5-B03 | expected_type=I64, found_type=Bool | found_type [19,23) | []; feature null. |
| F5-B04 | {} | declaration [14,15) | []; feature null. |
| F5-B05 | effect_model=f0-0005-sim-v1, effect_evidence=known_effect, required_effect=alloc, contract_origin=Span [29,37) | contract [29,37) | Un call/intrinsic.call [38,42), Entity intrinsic/prelude:push; feature null. |
| F5-B06 | {} | declaration [6,7) | []; feature null. |
| F5-B07 | pending_scope=later_adr | [] | []; feature resource_array. |
| F5-B08 | pending_scope=later_adr | [] | []; feature for_iteration. |
| F5-B09 | expected_type=I64, found_type=Bool | found_type [23,27) | []; feature null. |

Las identidades nuevas intrínsecas son prelude:push, prelude:pop y
prelude:join_arrays; la ortografía fuente de sus nombres no cambia esa identidad.
Para tipos no admitidos no se inventa un valor de elemento por estar vacío
el Array.
F5-B05 positivo no ejecuta F y no imprime nada; demuestra que la cota es el
único atributo inválido. F5-B06 cambia exclusivamente el destino constante por
un binding local nuevo. F5-B01/F5-B02 positivos deben permitir además escribir
la raíz después de terminar el préstamo, sin liberación anticipada ficticia.

### 9.2.1. Pares de fronteras y constantes

Campos comunes de §9.1; relaciones/cadenas vacías salvo indicación. Para
NX-E0104 se usa la operación de nombres de A2; cada fila de alcance tiene
solo pending_scope en data. Los positivos de estas filas no imprimen. F5-N02 usa names/names.declare.

| ID | Fuente negativa completa | Código; primario/token | Campos y positivo |
|---|---|---|---|
| F5-N01 | `fn Main(){for x in true{}}` | NX-E0102/type_mismatch; [19,23), `true` | types/types.check; expected_type=domain:iterable, found_type=Bool; found_type [19,23); positivo true→[true]. |
| F5-N02 | `const k:I64=comptime 1;const k:I64=comptime 2;fn Main(){}` | NX-E0104; [29,30), `k` | names/names.declare; declaration_kind=constant; previous_declaration [6,7); positivo segundo k→j. |
| F5-N03 | `const k:Option<I64>=comptime Some(1);fn Main(){}` | NX-E0006; [8,19), `Option<I64>` | types/types.check; feature_id=comptime, data={"pending_scope":"later_adr"}; positivo cambiar tipo a I64 e inicializador a 1 (misma restricción de representación escalar). |
| F5-N04 | `fn Main(){let x=comptime 1;}` | NX-E0006; [16,24), `comptime` | parse/syntax.parse; feature_id=comptime, data={"pending_scope":"later_adr"}; positivo quitar comptime y su espacio. |
| F5-N05 | `fn Main(){let f=fn [](){};let a=[f];}` | NX-E0006; [32,35), `[f]` | types/types.check; feature_id=closure, data={"pending_scope":"later_adr"}; positivo [f]→Some(f), como J L02. |
| F5-N06 | `pub const k:I64=comptime 1;fn Main(){}` | NX-E0006; [0,3), `pub` | parse/syntax.parse; feature_id=modules_visibility, data={"pending_scope":"later_adr"}; positivo quitar pub y su espacio. |
| F5-N07 | `const k:()=comptime F();fn F(){G();}fn G(){Print(1);}fn Main(){}` | NX-E1316; [20,21), `F` | effects/const.check; reason=impure_operation; constant [6,7); cadena F5-N07 abajo; positivo Print(1)→() en G, conservando ambas llamadas transitivas. |
| F5-N08 | `const k:I64=comptime 1;fn Main(){let x=move k;}` | NX-E0105; [44,45), `k` | ownership/place.write; data={}; declaration [6,7); positivo quitar move y su espacio; leer la constante copia su valor. |
| F5-N09 | `fn F(x:edit I64){}const k:I64=comptime 1;fn Main(){var v=k;F(edit k);}` | NX-E0105; [66,67), `k` | ownership/place.write; data={}; declaration [24,25); positivo cambiar solo el argumento k por v. |
| F5-N10 | `fn Main(){var x=7;x/=0;}` | R0502; [18,22), `x/=0` | runtime/value.div; data={}; positivo divisor 0→2. |

F5-N07 tiene la siguiente cause_chain, en este orden; evidence=verified
en los tres registros. kind=call; sin otros registros; conserva únicamente la relación constant
[6,7) indicada en F5-N07.

| operation | span | Entity kind/id |
|---|---|---|
| function.call | [20,21) | function / function:27 |
| function.call | [31,32) | function / function:39 |
| intrinsic.call | [43,48) | intrinsic / prelude:print |

F5-N08/F5-N09 no usan domain:place ni un error de tipo: el nombre resuelve
una constante inmutable. F5-N03 es puro; no enmascara NX-E1316. F5-N05 no
contiene recursos ni estado retenido y por ello no usa resource_array.
F5-C03/F5-C04 son regresiones de J F02/F03 con identificadores propios;
no redefinen sus diagnósticos.

Los pares siguientes fijan el límite de exclusividad de §3.3 y de llamadas
A2. Etapa ownership, operation ownership.access; restantes campos §9.1.
El positivo solo prepara el valor leído antes de adquirir exclusividad.

| ID | Fuente negativa | Código; primario/token | Data, relaciones y cadena |
|---|---|---|---|
| F5-N11 | `fn Main(){var x=1;x+=x;}` | NX-E1105/active_borrow_conflict; [21,22), `x` | origin=binding:14, binding=binding:14, loan=loan:18:0, active_permission=inout, requested_access=read; declaration [14,15), origin [18,19); un borrow/borrow.inout [18,19), Entity loan/loan:18:0, evidence=verified. |

Positivo F5-N11: `fn Main(){var x=1;let rhs=x;x+=rhs;}`.

| ID | Fuente negativa | Código; primario/token | Data, relaciones y cadena |
|---|---|---|---|
| F5-N12 | `fn Main(){var a=[1,2];let i=1;a[i]+=a[i-1];}` | NX-E1105/active_borrow_conflict; [36,37), `a` | origin=binding:14, binding=binding:14, loan=loan:30:0, active_permission=inout, requested_access=read; declaration [14,15), origin [30,34); un borrow/borrow.inout [30,34), Entity loan/loan:30:0, evidence=verified. |

Positivo F5-N12: `fn Main(){var a=[1,2];let i=1;let rhs=a[i-1];a[i]+=rhs;}`.

| ID | Fuente negativa | Código; primario/token | Data, relaciones y cadena |
|---|---|---|---|
| F5-N13 | `fn Main(){var sum=4;sum+=sum/2;}` | NX-E1105/active_borrow_conflict; [25,28), `sum` | origin=binding:14, binding=binding:14, loan=loan:20:0, active_permission=inout, requested_access=read; declaration [14,17), origin [20,23); un borrow/borrow.inout [20,23), Entity loan/loan:20:0, evidence=verified. |

Positivo F5-N13: `fn Main(){var sum=4;let rhs=sum/2;sum+=rhs;}`.

| ID | Fuente negativa | Código; primario/token | Data, relaciones y cadena |
|---|---|---|---|
| F5-N14 | `fn Main(){var a=[1];Push(edit a,Len(a));}` | NX-E1105/active_borrow_conflict; [36,37), `a` | origin=binding:14, binding=binding:14, loan=loan:25:0, active_permission=inout, requested_access=read; declaration [14,15), origin [25,31); un borrow/borrow.inout [25,31), Entity loan/loan:25:0, evidence=verified. |

Positivo F5-N14: `fn Main(){var a=[1];let n=Len(a);Push(edit a,n);}`.

### 9.2.2. Dominios, inferencia posterior e identidad constante

Rigen los campos comunes de §9.1. Los NX-E0102 son type_mismatch,
types/types.check; data contiene exactamente expected_type y found_type,
y no tienen cause_chain. Un dominio intrínseco no tiene relación expected_type.
La compuesta exige I64 por dominio tanto en Place como en RHS: no infiere
el dominio desde un Place inválido ni produce un conflicto de préstamo
por un literal RHS. El primer operando incompatible determina el primario.

| ID | Fuente negativa completa | Código; primario/token | Data, relaciones; edición positiva |
|---|---|---|---|
| F5-O09 | `fn Main(){let x=true<false;}` | NX-E0102/type_mismatch; [20,21), `<` | expected_type=I64, found_type=Bool; found_type [16,20); sustituir ambos operandos Bool por 0 y 1, preservando la igualdad de tipos y reparando solo el dominio común. |
| F5-O10 | `fn Main(){let x=1 and false;}` | NX-E0102/type_mismatch; [16,17), `1` | expected_type=Bool, found_type=I64; found_type [16,17); sustituir 1 por true. |
| F5-O11 | `fn Main(){let x=1 or false;}` | NX-E0102/type_mismatch; [16,17), `1` | expected_type=Bool, found_type=I64; found_type [16,17); sustituir 1 por true. |
| F5-O12 | `fn Main(){let x=-true;}` | NX-E0102/type_mismatch; [17,21), `true` | expected_type=I64, found_type=Bool; found_type [17,21); sustituir true por 1. |
| F5-O13 | `fn Main(){let x=-(9223372036854775808);}` | NX-E0107; [18,37), `9223372036854775808` | types/value.literal; data={}, related_spans=[], cause_chain=[]; sustituir el último dígito 8 por 7, manteniendo paréntesis y negación unaria. |
| F5-C05 | `fn Main(){var x=1;x+=true;}` | NX-E0102/type_mismatch; [21,25), `true` | expected_type=I64, found_type=Bool; found_type [21,25); sustituir true por 2. |
| F5-C06 | `fn Main(){var x=true;x+=1;}` | NX-E0102/type_mismatch; [21,22), `x` | expected_type=I64, found_type=Bool; found_type [21,22); sustituir el inicializador true por 0. |
| F5-A03 | `fn Main(){var a=[];Push(edit a,1);Push(edit a,true);}` | NX-E0102/type_mismatch; [46,50), `true` | expected_type=I64, found_type=Bool; expected_type [31,32), found_type [46,50); sustituir true por 2. El primer Push fija T=I64 por uso posterior al literal [] [16,18); no se emite NX-E0103. |
| F5-K04 | `const a:I64=comptime F();fn F()->I64{return a;}fn Main(){}` | NX-E1323/constant_cycle; [6,7), `a` | types/types.check; data={"cycle":["a","a"]}, related_spans=[], cause_chain=[]; sustituir solo a [44,45) en return por 1. El primario a [6,7) es el nombre declarado de la constante cíclica. La llamada F [21,22) alcanza la referencia a [44,45) e induce la arista a→a; F [28,29) es el nombre en la declaración de la función, no el sitio de llamada. Ninguno de esos F es nodo constante, primario ni relación dependency. |

El positivo F5-A03 conserva [] y ambos Push: obliga a resolver el tipo por
uso posterior sin anotación ni valor por defecto. Para los otros tres
operadores de orden se materializa F5-O09 sustituyendo `<` por `<=`, `>`
o `>=`: primario [20,20+len(OP)), found_type [16,20), mismos campos y
edición positiva. Cada fuente y su JSON se fijan antes de ejecutar el checker.

Fixture de Entity constante F5-E01: usa exactamente la fuente negativa y
positiva de F5-N08. La resolución de k [44,45) en ambas fuentes (en el
positivo, k pasa a [39,40)) debe referirse a la declaración [6,7) y producir
la Entity literal `{"kind":"constant","id":"constant:6"}` al serializar esa
identidad. Se compara ese objeto y su ida/vuelta por el esquema Entity;
`{"kind":"binding","id":"binding:6"}` no es un resultado admisible para
esa declaración. El negativo conserva NX-E0105, data={}, declaration [6,7)
y cause_chain=[]; el positivo copia el escalar y acepta check/run. Este
oráculo de identidad se comprueba en la resolución y serialización de
entidades, no añade un campo entity al Diagnostic ni fabrica un paso causal.

### 9.3. Semántica positiva y fallos apareados de comportamiento

Cada archivo siguiente es autónomo; LF entre líneas y LF final. Salvo
indicación, unidad sin observaciones. Los cambios negativos de comportamiento
son mutaciones del compilador, no fuentes que deban rechazarse.

```nexo
fn Main(){
    AssertEq(-7/3,-2);
    AssertEq(-7%3,-1);
    AssertEq(7/-3,-2);
    AssertEq(7%-3,1);
    AssertEq(-9223372036854775808%-1,0);
    AssertEq(2+3*4,14);
    AssertEq(20/3/2,3);
    AssertEq(not false and false or true,true);
    AssertEq(2<=2,true);
    AssertEq(2>=2,true);
    AssertEq(3>2,true);
    AssertEq([1]!=[2],true);
}
```

```nexo
fn Right()->Bool{Print(9);return true;}
fn Main(){
    AssertEq(false and Right(),false);
    AssertEq(true or Right(),true);
    AssertEq(true and Right(),true);
    AssertEq(false or Right(),true);
}
```

El segundo imprime exactamente `9\n9\n`, stdout ["9","9"], eventos [],
4 bytes de observación, reservas 0. Invertir el salto de cortocircuito debe
alterar esa traza y fallar el test. Añadir `with no_io` a Main es negativo
de efectos aun si se dejan solo las dos primeras llamadas omitidas en runtime;
se espera NX-E0415 en cada sitio causal conforme A2, no aceptación por folding.

```nexo
fn Main(){
    var a:Array<I64>=[];
    Push(edit a,1);
    var b=a;
    Push(edit b,2);
    AssertEq(a,[1]);
    AssertEq(b,[1,2]);
    AssertEq(Pop(edit b),Some(2));
    AssertEq(Pop(edit b),Some(1));
    let empty:Option<I64>=Pop(edit b);
    AssertEq(empty,None);
    AssertEq(JoinArrays(a,b),[1]);
    var count=0;
    for i in 9223372036854775807..=9223372036854775807{
        AssertEq(i,9223372036854775807);
        count+=1;
    }
    AssertEq(count,1);
    for i in 2..<2{count+=1;}
    for i in 3..=2{count+=1;}
    AssertEq(count,1);
}
```

Prueba de limpieza: `fn Main(){for i in 0..<2{let r=MockOpen(i);if i==0{continue;}break;}}`.
Eventos exactos open:0:0, close:0:0, open:1:1, close:1:1; stdout vacío,
observaciones ordenadas según A2, 94 bytes, reservas finales 0. Su variante
con `return;` en lugar de break conserva los eventos. Variante con `let x=1/0;`
en lugar de break falla R0502 tras open:1:1 y todavía cierra ambos recursos
en ese mismo orden; no produce R secundario. Las variantes `?` se realizan
con Main Result<(),I64>, `Err(7)?` contextualizado por
`let r:Result<(),I64>=Err(7);r?;`; esperan outcome err/exit1, misma limpieza.

Queda prohibido aprobar F5-L03 solo con sumas de variables: un índice llama a
una función que imprime 1 y devuelve 0, RHS imprime 2 y devuelve 3.
Sobre a=[4], `a[Index()]*=Rhs();` imprime `1\n2\n`, deja [12] y llama cada
función una vez. Cambiar Index para devolver 2 produce R0102 antes de Rhs,
stdout ["1"]. Cambiar RHS a `7/0` produce R0502 sin escribir el destino.
El test interno de estado comprueba que a sigue [4] antes de limpieza final;
no añade una captura de R inexistente al lenguaje.

### 9.4. Fronteras exactas y escala

Bajo storage-96, literal a=[0] reserva 32 bytes; primer Push crece C1→2,
pico 96 y después 64: se acepta. Segundo Push intenta C2→4, candidato vivo
192: R0002 con runtime_budget_kind=array_storage_bytes, limit=96,
observed=192. Bajo allocation-96, las primeras reservas 32+64=96 caben;
la siguiente produce observed=224 y array_allocation_bytes. Son los mismos
programas, sin otros buffers, celdas/observaciones lejos de límite. La pareja
negativa es añadir exactamente ese segundo Push. Frontera inferior separada:
solo literal, 32. Liberar a y construir otro comprueba que acumulado no baja.

Para trabajo antiguo se conserva la receta A2 k=71427, más p bloques vacíos
de coste uno: p=7/8/9 produce 999999/1000000/1000001. Se ejecuta bajo
work-1000000, con R0001 en el cargo 1000001. Bajo perfil por defecto,
k=1000000 termina a 14000014; la familia de límite nuevo usa
k=1142856 (15999998) y p=1/2/3 para 15999999/16000000/16000001.
No se calibra p a posteriori observando la implementación.

Las cuotas const se aíslan con IR escalar ya verificada, además de fuentes:
se prepara una secuencia de cargos con totales 31/32/33 para pasos, cadenas
de frames 7/8/9 para profundidad, y ocupaciones 64/96/128 bytes para memoria.
Los cargos y su OriginId se transcriben de §7.1; los tests internos no
sustituyen pruebas fuente de recursión y caché de varias constantes. La
prueba de fuente mínima `const k:I64=comptime 1;fn Main(){}` tiene pico
96 bytes: frame 32 + resultado 32 + entrada caché 32, acepta memory-96.
Añadir segunda constante escalar exige pico128 y falla al instalar su caché;
el nombre de esa constante es el primario y observed=97; el pico candidato
es 128, pero NX-E0003 conserva observed=limit+1 de A2 §11.1.

La fuente exacta de esa pareja es
`const a:I64=comptime 1;const b:I64=comptime 2;fn Main(){}`:
memory-96 rechaza b [29,30), data
{"budget_kind":"comptime_memory_bytes","budget_scope":"compilation",
"limit":96,"observed":97,"incomplete_analysis":"lower"}; sin relaciones
ni cadenas. Positivo quitar la segunda declaración. Para profundidad se
materializa `const k:I64=comptime F1();` seguido de N funciones, en orden
numérico: `fn Fi()->I64{return F(i+1)();}` salvo la última,
`fn FN()->I64{return 1;}`, y `fn Main(){}`. Fi/F(i+1)/FN son metanombres
sustituidos antes de analizar, nunca sintaxis ejecutada. El inicializador
más N llamadas requiere N+1 frames; N=6/7/8 prueba 7/8/9 en depth-8.
El fallo se ancla en la llamada F8() del cuerpo F7, con observed=9. El
span se calcula por concatenación de los literales materializados; no se
consulta la ubicación que produzca el compilador.

### 9.5. Totales y alcanzabilidad en el perfil normal

Los totales siguientes aplican A2 §§11.6–11.7 más §7.2, con entrada Main y
unidad final incluidas; sus cuerpos no añaden ticks por llaves contenedoras.
No son métricas de una implementación. Los cargos se recomputan por categorías.

| ID | Fuente completa | Cuenta | Ticks |
|---|---|---|---:|
| F5-T01 | `fn Main(){for i in 0..<3{}}` | entrada 1 + for 1 + extremos 4 + slots 2 + pruebas 4 + bindings 3 + avances 3 + unidad 1 | 19 |
| F5-T02 | `fn Main(){for i in 0..=2{}}` | entrada 1 + for 1 + extremos 4 + slots 2 + pruebas entrada 3 + bindings 3 + pruebas finales 3 + avances 2 + unidad 1 | 20 |
| F5-T03 | `fn Main(){for x in [1,2,3]{}}` | entrada 1 + for 1 + array (nodo 1 + elementos 6 + cabecera 1) + slots 2 + pruebas 4 + avances 3 + unidad 1 | 20 |
| F5-T04 | `fn Main(){let b=false and true;}` | entrada 1 + let 1 + operador 1 + izquierda 2 + salida 1 + unidad 1 | 7 |
| F5-T05 | `fn Main(){let b=true and false;}` | F5-T04 + derecha 2 | 9 |
| F5-T06 | `fn Main(){let b=true or false;}` | igual a F5-T04 | 7 |
| F5-T07 | `fn Main(){let b=false or true;}` | igual a F5-T05 | 9 |
| F5-T08 | `fn Main(){var x=1;x+=2;}` | entrada 1 + binding 3 + sentencia 1 + Place 0 + carga 1 + RHS 2 + operación 1 + resultado 1 + unidad 1 | 11 |

F5-Q01, frontera ajustada de bytes: `fn Main(){let a=[0,0,0];}` reserva
96 bytes y acepta storage-96 y allocation-96. Su negativo cambia únicamente
el literal a `[0,0,0,0]`: candidato 128 y R0002/runtime.limit sobre el literal
[16,25), runtime_budget_kind correspondiente, limit=96, observed=128.
Para aislar allocation se selecciona allocation-96 (storage conserva su
límite normal); para aislar storage se selecciona storage-96. El caso inferior
usa `[0,0]`, 64 bytes. Se conservan además los picos 96→192 y el acumulado
224 de §9.4: prueban crecimiento, no sustituyen esta frontera ajustada.

F5-Q02, array_storage_bytes normal: generar dentro de Main, en orden j=0…7,
`var aj:Array<I64>=[];for i in 0..<32768{Push(edit aj,0);}` seguido de
`for i in 0..<32768{Pop(edit aj);}`. Sustituir aj por a0…a7; no son arrays
anidados ni nombres emitidos por el compilador. Mantener todos los bindings
hasta el cierre de Main. Cada array vacío después de Pop conserva C=32768,
1.048.576 bytes. Los primeros siete se completan; en a7, i=16384, el
crecimiento C16384→32768 propone 7*1048576+524288+1048576=8.912.896 bytes.
R0002/runtime.limit, array_storage_bytes, limit=8.388.608, observed=8.912.896,
primario llamada Push completa de a7. El positivo conserva literalmente
las 21 sentencias (var, for Push y for Pop por cada array) de a0…a6 y sustituye únicamente ambos límites 32768
de a7 por 16384: `var a7:Array<I64>=[];for i in 0..<16384{Push(edit a7,0);}`
seguido de `for i in 0..<16384{Pop(edit a7);}`. Conserva los ocho bindings
hasta el cierre; termina con a7 vacío, C=16384, sin ningún Push adicional. Celdas simultáneas <32790,
asignaciones acumuladas <16 MiB, trabajo <6.000.000 y observaciones cero;
no hay otra cuota anterior. Los bucles anteriores sin salida no se eliminan.

F5-Q03, array_allocation_bytes normal: materializar una sola vez en fuente
`fn Main(){for i in 0..<2049{let a=[` + 1024 ceros separados por coma +
`];}}`. Cada iteración reserva 32768 bytes y los libera al salir; 2048
iteraciones llegan exactamente a 67.108.864. La siguiente propone 67.141.632:
R0002/runtime.limit, array_allocation_bytes, limit=67.108.864,
observed=67.141.632, primario literal completo. Positivo 2049→2048 e inferior
2047. Pico storage 32768, celdas <1030, trabajo <4.300.000, fuente <2200
bytes y observaciones cero. No se necesita exponer Reserve ni capacidad.

F5-Q04, comptime_steps normal: `const k:()=comptime F();fn F(){var i=0;while
 i<7141{i=i+1;}` + p bloques `{}` + `}fn Main(){}` (el salto mostrado después
de while es un espacio en la materialización). Cuesta 14*7141+17+p=99991+p:
Main no se ejecuta durante compilación; 17 incluye entrada del inicializador,
nodo/callee de F y el total de entrada/cuerpo/unidad de F de A2. p=8/9/10
produce 99999/100000/100001; el último rechaza la unidad implícita de F,
NX-E0003 lower/analysis.limit, comptime_steps, limit=100000, observed=100001.
Dos frames, menos de 256 bytes vivos y un único inicializador aíslan pasos.

F5-Q05, comptime_depth normal: la familia F1…FN de §9.4 con N=62/63/64
requiere 63/64/65 frames incluido el inicializador. El negativo rechaza la
llamada F64() de F63, NX-E0003 lower/analysis.limit, comptime_depth,
limit=64, observed=65. Menos de 400 pasos y 8192 bytes; 65 funciones
incluido Main, sin recursión de tipos. Spans derivados de concatenación
literal; el positivo conserva toda la cadena salvo su último eslabón.

F5-Q06, comptime_memory_bytes normal: concatenar para i=0…n-1
`const c{i}:I64=comptime 0;` sustituyendo {i} por decimal sin ceros iniciales,
seguido de `fn Main(){}`. Caché después de n constantes: 32*n bytes; pico al
instalar la enésima: 32*(n+2). n=32765/32766/32767 da
1048544/1048576/1048608 bytes candidatos. Solo el último rechaza el nombre
c32766 con NX-E0003 lower/analysis.limit, limit=1048576, observed=1048577,
budget_kind=comptime_memory_bytes. Cada const cuesta 3 ticks: el negativo
llega a 98301, menor que 100000; tiene un frame, no hay arrays ni efectos.
Su fuente mide 906377 bytes; no excede source_bytes. Resolver constantes
sin dependencias usa visitas lineales; no se permite buscar SCC mediante
comparación de todos los pares. Para este inicializador literal sin dependencias, se cobran siete registros
IR: constante (también nodo del grafo), bloque, slot de resultado,
materialización, operando literal, entrega a caché y limpieza. No hay
registro de función fuente, arista de dependencia ni préstamo. Total
7*32767=229369 más los registros de Main, menor que 262144. No se duplica
el nodo del grafo por mantener un índice auxiliar; análisis y memoria const
tienen contadores distintos. Es el mismo dispatcher IR con una entrada
const, no un evaluador paralelo.

Las materializaciones F5-Q02–Q06 no contienen espacios adicionales ni LF
final: Q02 concatena sus dos plantillas sin separador dentro de `fn Main(){`
y `}`; Q03 usa el literal publicado; Q04 usa un espacio entre while e i;
Q05 concatena inicializador, funciones y Main; Q06 concatena declaraciones.
Spans negativos exactos: Q02 [680,695), `Push(edit a7,0)`;
Q03 [34,2083), literal de 1024 ceros; Q04 [79,80), `}` de F;
Q05 [1766,1771), `F64()`; Q06 [906344,906350), `c32766`.
En F5-Q04–Q06 budget_scope=compilation e incomplete_analysis=lower;
subcause null, sin feature_id, relaciones ni cadenas, y demás campos §9.1.
La memoria candidata se conserva en el oráculo, no se publica como observed
de NX-E0003. Cada receta normal se contrasta además con los perfiles
reducidos de §9.4; ninguna frontera de una cuota puede aprobarse mediante
un rechazo previo por otra cuota (A2 §§11.3/11.5).

## 10. Subtareas y aceptación

Las filas definen obligaciones completas; no autorizan aceptar código antes
de disponer de efectos, limpieza y cuotas de su operación. F5-L08 integra la
contabilidad nueva y levanta su barrera global. F5-L01–F5-L07 se validan
con arnés de IR interno tipado y verificado, como las subtareas de F0-0003;
check/run mantienen implementation_limit/L08 para fuentes de capacidades
nuevas hasta completar esa contabilidad. Un éxito del arnés no equivale a
ejecución fuente aceptada antes de F5-L08. Cada paso exige pares por
regla, un negativo que llegue a su etapa correcta y mutación inversa que el
par detecte. No se reescriben fixtures antiguas para ocultar una regresión.

| Paso | Dependencias | Entregable y criterio independiente |
|---|---|---|
| F5-L01 | A2/J completos | Operadores I64/orden/!=, tokens/AST/IR/fmt; todos los signos, MIN/MAX, cero y precedencias; cero panics anfitrión. |
| F5-L02 | F5-L01 | and/or/not con CFG y limpieza; trazas omitidas/tomadas y efectos conservadores, una sola tabla de tipos. |
| F5-L03 | F5-L01 | Cinco compuestas, lugar único y exclusividad; índice/RHS con trazas y fallos; diagnóstico exacto de propiedad. |
| F5-L04 | J13, F5-L02 | Prueba literal de divergencia y retorno; par while true return/break; continue y joins I07b sin perder consumos. |
| F5-L05 | F5-L01,F5-L04 | For rango con límites únicos, vacío/MAX/MIN, control y limpieza; ninguna materialización de rango. |
| F5-L06 | F5-L04 | For Array prestado/temporal/movido; rechazos de alias/escape, préstamo terminado tras break y return. Se valida con arnés IR tipado y verificado; las fuentes que requieren el Array dinámico o Push/Pop/JoinArrays esperan F5-L07 y su ejecución fuente espera también la contabilidad F5-L08. |
| F5-L07 | F5-L01, J02 | Array dinámico y preámbulo Push/Pop/JoinArrays; independencia anidada, None, fallos parciales; no nuevo tipo List. |
| F5-L08 | F5-L01–F5-L07 | Perfil, contabilidad y reservas de bytes; fronteras exactas §9.4, ticks heredados intactos y limpieza no interrumpible. |
| F5-L09 | F5-L01–F5-L05 | Resolución const, dependencias, pureza y tipos escalares; ciclos/impureza/limitaciones diferenciados y sin evaluación anticipada. |
| F5-L10 | F5-L08,F5-L09 | Evaluación const en IR común, cuotas/caché/hash; reproducibilidad, fault frente a límite y ausencia de ejecución global. |
| F5-L11 | F5-L01–F5-L10 | Batería §11 y todos los pares fuente/JSON; fmt/reparseo conserva identidad B, ninguna ruta sintáctica antigua. |
| F5-L12 | F5-L11 | Revisión adversarial independiente, escala, fuzz, mutaciones de verificador; defectos corregidos con regresión y pendientes nominales. |

Por requisito se informa PASA/FALLA/NO EVALUADO/INCONCLUSO con evidencia.
La revisión no puede sustituirse por una demo ni por un conteo de tests.
El implementador y el validador son distintos; este último recibe código y
criterios y compara check/run con su propia batería sobre git archive HEAD.
Los esperados se transcriben del ADR; no los genera el código bajo prueba.
Mutaciones: una por regla, aislada, quitar/cambiar comprobación y comprobar
que falla el par pertinente; restauración del archivo verificada por SHA-256.
Se repiten las mutaciones supervivientes más un máximo de ocho nuevas, bloqueantes solo respecto de reglas explícitas. Una laguna real se
registra, no se rellena con una exigencia no escrita ni se llama aprobada.

Oráculos independientes: enteros matemáticos de precisión arbitraria con
división truncada implementada por signos (no // negativo de Python), tabla
de trazas de ámbitos, y contadores por la gramática de cargos de §7. Se
comparan valor, stdout, eventos, bytes, reservas, primer fallo y diagnósticos
completos. El oráculo no importa AST/IR/lowering/checker del prototipo.

Escala determinista: semillas 0…31, 1000 programas bien tipados de hasta
32 nodos por semilla, y 1000 mutaciones fuente por semilla; extremos I64,
arrays de longitudes 0/1/2/3/4/7/8/9/1024, anidamiento 1/32/127/128/129,
bucles anidados hasta la frontera sintáctica, break/continue y préstamos.
Un resultado por cuota es válido solo si coincide con el oráculo de esa
cuota, nunca sirve para encubrir divergencia semántica previa. Fuzz de bytes
con semilla registrada debe terminar sin panic ni superar límites; se
minimiza todo contraejemplo y se conserva nominalmente.

Pilas de hilo de 256 KiB y 512 KiB, debug/release, recorren los casos profundos
y las fronteras de frames; ningún recorrido por profundidad fuente/valor
puede depender de la pila Rust. Los límites deben fallar antes de agotar esa
pila. Se comparan también Windows y un segundo anfitrión cuando haya entorno;
ausencia de segundo anfitrión es NO EVALUADO, no prueba de portabilidad.

Aceptación del corte: nueve programas completos dan exactamente las salidas
de §11, diagnósticos []; todos los pares llegan a su regla, todas las cuotas
tienen fronteras aisladas, ninguna mutación de seguridad requerida sobrevive,
regresiones A2/J sin desvío salvo cambios explícitos de este ADR, verificador
rechaza cada IR inválida antes de ejecutar. Las puertas F0 de dos revisores
sin discrepancias, modelo reducido y aprobación del propietario siguen
pendientes hasta evidencia independiente; cerrar F5-L12 no aprueba una fase.

## 11. Programas de aceptación

Cada bloque es un archivo .n autónomo, LF y LF final, entre 30 y 60 líneas.
No requieren texto, módulos ni entradas de F0-0004; parámetros del problema
se fijan en Main y se imprimen resultados numéricos/arrays. En todos: outcome
unit, exit0, diagnostics/events [], reservas0; stdout es la lista de líneas
indicada, observation_bytes es la longitud UTF-8 de esas líneas con LF.
Son algoritmos escritos con las capacidades nuevas; no se afirma que sus
funciones matemáticas fueran incomputables mediante emulación en A2.

| Programa | observation_bytes exactos |
|---|---:|
| F5-P01 | 41 |
| F5-P02 | 37 |
| F5-P03 | 12 |
| F5-P04 | 38 |
| F5-P05 | 42 |
| F5-P06 | 16 |
| F5-P07 | 30 |
| F5-P08 | 36 |
| F5-P09 | 22 |

### F5-P01 — Primos por división de prueba

```nexo
const limit:I64=comptime 30;
fn Prime(n:view I64)->Bool{
    if n<2{
        return false;
    }
    var d=2;
    while d<=n/d{
        if n%d==0{
            return false;
        }
        d+=1;
    }
    return true;
}
fn Main(){
    var primes:Array<I64>=[];
    for n in 0..=limit{
        if not Prime(n){
            continue;
        }
        Push(edit primes,n);
    }
    AssertEq(Len(primes),10);
    AssertEq(primes[0],2);
    AssertEq(primes[9],29);
    var sum=0;
    for p in primes{
        sum+=p;
    }
    Print(primes);
    Print(sum);
}
```

Salida: `[2, 3, 5, 7, 11, 13, 17, 19, 23, 29]\n129\n`.

### F5-P02 — Criba

```nexo
const limit:I64=comptime 30;
fn Main(){
    var composite:Array<Bool>=[];
    for i in 0..=limit{
        Push(edit composite,false);
    }
    composite[0]=true;
    composite[1]=true;
    var p=2;
    while p<=limit/p{
        if composite[p]{
            p+=1;
            continue;
        }
        var multiple=p*p;
        while multiple<=limit{
            composite[multiple]=true;
            multiple+=p;
        }
        p+=1;
    }
    var primes:Array<I64>=[];
    for n in 2..=limit{
        if not composite[n]{
            Push(edit primes,n);
        }
    }
    AssertEq(Len(primes),10);
    AssertEq(composite[25],true);
    Print(primes);
}
```

Salida: `[2, 3, 5, 7, 11, 13, 17, 19, 23, 29]\n`.

### F5-P03 — Collatz

```nexo
const start:I64=comptime 27;
fn Step(n:view I64)->I64{
    if n%2==0{
        return n/2;
    }
    return 3*n+1;
}
fn Main(){
    var n=start;
    var steps=0;
    var peak=n;
    var odd=0;
    while true{
        if n==1{
            break;
        }
        if n%2!=0{
            odd+=1;
        }
        n=Step(n);
        steps+=1;
        if n>peak{
            peak=n;
        }
    }
    AssertEq(n,1);
    AssertEq(odd,41);
    Print(steps);
    Print(peak);
    Print(odd);
}
```

Salida: `111\n9232\n41\n`. Debe caber también en work-1000000.

### F5-P04 — Ordenación por inserción

```nexo
fn Sort(a:edit Array<I64>){
    let size=Len(a);
    for i in 1..<size{
        let value=a[i];
        var j=i;
        while j>0 and a[j-1]>value{
            a[j]=a[j-1];
            j-=1;
        }
        a[j]=value;
    }
}
fn Main(){
    var values:Array<I64>=[];
    Push(edit values,5);
    Push(edit values,2);
    Push(edit values,4);
    Push(edit values,2);
    Push(edit values,1);
    let original=values;
    Sort(edit values);
    AssertEq(original,[5,2,4,2,1]);
    AssertEq(values,[1,2,2,4,5]);
    let largest=Pop(edit values);
    AssertEq(largest,Some(5));
    Push(edit values,5);
    let joined=JoinArrays(values,[8,9]);
    AssertEq(Len(joined),7);
    Print(values);
    Print(joined);
}
```

Salida: `[1, 2, 2, 4, 5]\n[1, 2, 2, 4, 5, 8, 9]\n`.

### F5-P05 — Búsqueda binaria

```nexo
fn Search(a:view Array<I64>,key:view I64)->I64{
    var lo=0;
    var hi=Len(a);
    while lo<hi{
        let mid=lo+(hi-lo)/2;
        if a[mid]==key{
            return mid;
        }
        if a[mid]<key{
            lo=mid+1;
        }else{
            hi=mid;
        }
    }
    return -1;
}
fn Main(){
    var squares:Array<I64>=[];
    for n in 0..<10{
        Push(edit squares,n*n);
    }
    AssertEq(Len(squares),10);
    AssertEq(Search(squares,0),0);
    AssertEq(Search(squares,81),9);
    AssertEq(Search(squares,82),-1);
    let empty:Array<I64>=[];
    AssertEq(Search(empty,1),-1);
    Print(Search(squares,49));
    Print(Search(squares,50));
    Print(squares);
}
```

Salida: `7\n-1\n[0, 1, 4, 9, 16, 25, 36, 49, 64, 81]\n`.

### F5-P06 — Euclides y mínimo común múltiplo

```nexo
fn Gcd(a:view I64,b:view I64)->I64{
    var x=a;
    var y=b;
    while y!=0{
        let remainder=x%y;
        x=y;
        y=remainder;
    }
    return x;
}
fn Lcm(a:view I64,b:view I64)->I64{
    if a==0 or b==0{
        return 0;
    }
    return (a/Gcd(a,b))*b;
}
fn Main(){
    CheckZeros();
    var results:Array<I64>=[];
    Push(edit results,Gcd(48,18));
    Push(edit results,Gcd(270,192));
    Push(edit results,Gcd(0,7));
    Push(edit results,Gcd(7,0));
    AssertEq(results,[6,6,7,7]);
    Print(results);
    Print(Lcm(21,6));
}
fn CheckZeros(){
    AssertEq(Lcm(0,7),0);
    AssertEq(Gcd(1,1),1);
}
```

Salida: `[6, 6, 7, 7]\n42\n`. Dominio de estas funciones de ejemplo:
argumentos no negativos y productos representables; no prometen abs(MIN).

### F5-P07 — Potencia modular

```nexo
const modulus:I64=comptime 1000;
fn PowMod(base:view I64,exponent:view I64)->I64{
    var b=base%modulus;
    var e=exponent;
    var result=1;
    while e>0{
        if e%2!=0{
            result=(result*b)%modulus;
        }
        e/=2;
        if e==0{
            break;
        }
        b=(b*b)%modulus;
    }
    return result;
}
fn Main(){
    AssertEq(PowMod(2,0),1);
    AssertEq(PowMod(2,10),24);
    AssertEq(PowMod(7,8),801);
    var powers:Array<I64>=[];
    for exponent in 0..=5{
        Push(edit powers,PowMod(3,exponent));
    }
    AssertEq(Len(powers),6);
    AssertEq(powers[5],243);
    Print(PowMod(2,10));
    Print(PowMod(7,8));
    Print(powers);
}
```

Salida: `24\n801\n[1, 3, 9, 27, 81, 243]\n`. modulus=1000 y bases
no negativas mantienen los productos por debajo de un millón; no se supone
una multiplicación modular que evite overflow para módulos I64 arbitrarios.

### F5-P08 — Matriz identidad independiente

```nexo
const size:I64=comptime 3;
fn Identity()->Array<Array<I64>>{
    var matrix:Array<Array<I64>>=[];
    for rowIndex in 0..<size{
        var row:Array<I64>=[];
        for column in 0..<size{
            let cell=if rowIndex==column then{
                value:1;
            }else{
                value:0;
            };
            Push(edit row,cell);
        }
        Push(edit matrix,move row);
    }
    return move matrix;
}
fn Main(){
    let matrix=Identity();
    var copy=matrix;
    copy[0][0]=9;
    AssertEq(matrix[0][0],1);
    AssertEq(copy[0][0],9);
    var trace=0;
    for i in 0..<size{
        trace+=matrix[i][i];
    }
    AssertEq(trace,size);
    Print(matrix);
    Print(trace);
}
```

Salida: `[[1, 0, 0], [0, 1, 0], [0, 0, 1]]\n3\n`.

### F5-P09 — Histograma con filtrado

```nexo
const buckets:I64=comptime 5;
fn Main(){
    let values=[0,1,1,4,2,4,4,3,0,-1,5];
    var counts:Array<I64>=[];
    for bucket in 0..<buckets{
        Push(edit counts,0);
    }
    var rejected=0;
    for value in values{
        if value<0 or value>=buckets{
            rejected+=1;
            continue;
        }
        counts[value]+=1;
    }
    var total=0;
    var maximum=0;
    for count in counts{
        total+=count;
        if count>maximum{
            maximum=count;
        }
    }
    AssertEq(total+rejected,Len(values));
    AssertEq(counts,[2,2,1,1,3]);
    AssertEq(maximum,3);
    Print(counts);
    Print(total);
    Print(rejected);
    Print(maximum);
}
```

Salida: `[2, 2, 1, 1, 3]\n9\n2\n3\n`.

## 12. Enmiendas propuestas y trazabilidad

Las citas siguientes identifican el texto cuyo alcance cambia; no modifican
los documentos citados ni la base. Son decisiones cerradas de este ADR,
pendientes de revisión e integración documental con su implementación.

| Autoridad y cita exacta | Decisión propuesta |
|---|---|
| Roadmap D07: «Se adoptan evaluación secuencial definida, inicialización obligatoria, ausencia de nulos universales y aritmética entera comprobada por defecto, con operaciones modulares explícitas.» | Concreción: truncamiento, resto con signo del dividendo, MIN%-1 válido, otros fallos §2. No se cambia la base. La aritmética comprobada está en D07; D01 es superficie. |
| Roadmap D09: «El usuario observa valores independientes en todas las rutas.» | Array único dinámico con copia profunda. CoW/RC/publicación quedan en sus fases; capacidad no observable no autoriza alias de escritura. |
| D13 G.3: `ConstDecl     = ["pub"] "const" Name ":" Type "=" "comptime" Expr Terminator` | Activar el subconjunto privado escalar con evaluación acotada. No sustituir `= comptime` por otra sintaxis. Pureza completa, valores no escalares diferidos. |
| D13 §8: «Un inicializador que requiere E/S, creación de recursos o una operación fuera de ese subconjunto da NX-E1316» | Distinguir impureza real (NX-E1316) de capacidad pura aún fuera de representación (NX-E0006/comptime con pending_scope=later_adr), según §§5/8. No afirmar que el lenguaje prohíbe para siempre constantes no escalares. |
| D13 §8: «NX-E1323/alias_cycle y de constantes NX-E1323/constant_cycle, con el ciclo mínimo reproducible como causa» | Concretar mínimo por número de aristas y desempate de offsets; types/types.check, data.cycle y relaciones dependency, sin cadena de llamadas ficticia. |
| D15 §3.5: `Assignment    = Place ("=" \| "+=") Expr Terminator ;` | Añadir -=, *=, /=, %= al inventario léxico y producción Assignment, con exclusividad y semántica §3.3; una sola producción y un solo lowering. |
| D15 §8.7 F2-S27: `fn Main(){let x=1⟦*⟧2;}` y esperado NX-E0002 | Convertir esa frontera histórica y las de !=, >, <=, >= en positivos del perfil nuevo. Preservar negativos de tipos, sin aceptar comparaciones encadenadas. |
| D15 §8.7 F2-F24/F25/F26 y feature_id compound_assignment/for_iteration/loop_control | Levantar solo las formas incluidas; J13 conserva el control común. No eliminar los reconocedores de formas realmente diferidas. |
| D15 §3.6, `implementation_limit` y pending_scope de A2 | Ampliación cerrada §8.2; no añadir identificadores improvisados ni ocultar defectos semánticos. |
| A2 perfil `f0-0002-v2`, trabajo 1.000.000; J perfil `f0-0003-b-v1` | Nuevo perfil 16.000.000, contabilidad heredada y nuevas cuotas de búfer/const. Sin reinterpretar resultados históricos con otro presupuesto. |
| A2 §6: «No se supone que el bucle ejecutará cero o una iteración por observar una constante.» | Se conserva el punto fijo; se añade exclusivamente prueba estructural de ausencia de borde falso para while literal true. No equivale agotamiento de cuota a retorno ni elimina efectos. NX-E1317 sigue denunciando toda caída alcanzable. |

### 12.1. Enmiendas propuestas de léxico, gramática y catálogo

Estas sustituciones afectan al perfil F0-0005; las fuentes históricas se
conservan como evidencia de sus perfiles anteriores. La prohibición de
reescribir fixtures para ocultar regresiones no impide los cambios de
esperado enumerados aquí, que deben conservar ambos contratos identificados.

| Documento, sección y texto exacto que se enmienda | Nueva redacción normativa | Filas afectadas |
|---|---|---|
| A2 §13.12, fila completa de NX-E0006: «\| `legacy_move`, `float_literal`, `range`, `tuple`, `attributes`, `default_argument`, `closure`, `receiver_mode`, `no_heap_alloc`, `match`, `user_types`, `traits_generics`, `modules_visibility`, `extensions`, `provenance_clauses`, `comptime`, `text`, `methods`, `compound_assignment`, `for_iteration`, `loop_control`, `generator`, `region` \| `parse` / `syntax.parse` \|» | En F0-0005, range fuera de for directo y selector de rango usan types/types.check; comptime puro fuera del dominio escalar de const usa types/types.check, sujeto a la precedencia de impureza de §8.2. Comptime fuera de const conserva parse/syntax.parse; las formas aceptadas no emiten NX-E0006. Los demás feature_id conservan sus etapas salvo enmienda explícita. | §8.2, F5-D01/F5-D02, F5-N03/F5-N04. |
| D15 §2.4: «`-x` no lo es.» | El literal negativo se normaliza solo ante un token decimal; `-x` es negación unaria comprobada, con dominio I64. | Truncamiento después de `-`, F5-O05. |
| D15 §2.4: «`/`, `%` son operadores de B completa pero no aceptados en F0» | `/` y `%` se aceptan en F0-0005 sobre I64 conforme §2; se preservan los errores léxicos de símbolos ajenos a B. | F2-LX01/F2-LX02 pasan a positivos; F2-LX03–05 no cambian. |
| D15 §2.4, primera línea del inventario: `..< ..= += != -> => == <= >=` | `..< ..= += -= *= /= %= != -> => == <= >=`; la segunda línea se conserva. Coincidencia más larga; comentario `//` antes de `/`, `/=` antes de `/`. | Léxico de las cinco compuestas; F2-F24 y F5-L03. |
| D15 §3.2: «Se mantiene el dominio F0 de A2 §§3.5–3.6 y 4.6: solo `<` y `==` se aceptan.» | Se aceptan los operadores de §2 de F0-0005, con su dominio, precedencia y asociación; no se aceptan comparaciones encadenadas. | F2-S27/S28/S32/S33/S34, F2-LX01/LX02 y las tres filas lógicas de D15 §6.3. |
| D15 §3.2: «`*`, `/`, `%`, `!=`, `>`, `<=`, `>=`, `and` y `or` se tokenizan pero producen NX-E0002 en el reconocedor de operador excluido; `not` fuera de una producción F0 produce NX-E0002.» | Esos tokens se analizan como operadores aceptados; los tipos inválidos producen NX-E0102, nunca unsupported_binary_operator/unsupported_unary_operator por el solo token. | Mismas filas; negativos de tipos §9.2 y familias de ambos órdenes. |
| D15 §3.2: «`and/or` en posición infija usan `parser_context=unsupported_binary_operator`, `expected_tokens=["+","-","<","=="]`; `not` al comienzo de un operando usa `parser_context=unsupported_unary_operator`, `expected_tokens=["(","-","IntLiteral","Name","[","edit","false","if","move","true"]`.» | Se retiran esos rechazos para operadores incluidos. Los truncamientos usan FIRST de la producción comprometida y las listas de §12.2; no se emiten contextos unsupported_* para estos operadores. | D15 §6.3, tres filas not/and/or, ahora positivos con diagnostics=[]. |
| D15 §3.6: «`pending_scope` es `F0-0003` para `closure/generator/option/match`, `later_adr` para los restantes de alcance; `implementation_limit` usa exclusivamente `I08`, `I09`, `I10a`, `I10b` o `I11a` según §7.1.» | En F0-0005, A2/J completos no emiten pendientes I/J; implementation_limit usa solo L01…L10. Los alcances aún diferidos usan later_adr con los feature_id de §8.2, incluida closure para arrays de closures y for_iteration para for edit. | Validación cruzada y explain: range usa F5-D01, comptime F5-N04, closure F5-N05, for_iteration F5-B08; no se añaden feature_id. |
| D15 §3.6, última frase: «Los cinco operadores simbólicos excluidos originales conservan NX-E0002; los nuevos operadores completos excluidos `/ % and or not` también son sintaxis fuera de la gramática F0, sin crear nuevos feature_id.» | Los operadores incluidos en §2 de F0-0005 dejan de ser exclusiones sintácticas; se mantiene el conjunto de 30 feature_id y sus fronteras de §8.2. | Todas las filas de operadores anteriores; catálogo explain y validación cruzada feature_id/pending_scope. |
| F0-CODIGOS, «Operadores y proyecciones de F0-0002»: «La exclusión binaria mediante NX-E0002 incluye exactamente `*`, `!=`, `>`, `<=` y `>=`, con `parser_context = unsupported_binary_operator` y `expected_tokens = ["+","-","<","=="]`. Precede a NX-E0005 derivado y no emite una reparación de terminador.» | En F0-0005 esas operaciones se aceptan; NX-E0102 comprueba sus dominios. No se fabrica NX-E0005 ni una reparación por haber encontrado un operador válido. | F2-S27/S28/S32/S33/S34; ejemplos del catálogo y explain. |
| F0-CODIGOS, párrafo previo: «Los tokens `*` y `!=` en posición de operador binario usan NX-E0002, no NX-E0006.» | En el perfil F0-0005 ambos se aceptan según §2; antes de completar su subtarea solo se usa implementation_limit/L01. | Mismos dos tokens, catálogo y frontera temporal. |
| D15 §8.7 F2-F04: `fn Main(){let n=1;let x=0⟦..<⟧n;}`, feature_id `range`, span `[25,28)` | Se conserva range/later_adr sobre [25,28), pero types/types.check: es un rango almacenado, no iterando directo. El positivo histórico `+` se conserva. | F2-F04, F5-D01/F5-D02. |
| D15 §8.7 F2-F21: `⟦const⟧ x:I64=comptime 1;fn Main(){}`, feature_id `comptime`, span `[0,5)` | La fuente completa sin marcadores es positiva con diagnostics=[], unit, exit 0, observaciones vacías. Const no escalar puro y comptime fuera de const conservan comptime/later_adr. | F2-F21, F5-N03/F5-N04; explain usa F5-N04 como negativo vigente. |
| D15 §8.7 F2-S28: `fn Main(){let x=1⟦!=⟧2;}`, NX-E0002, span `[17,19)` | Fuente completa positiva, x=true; diagnostics=[], unit, exit 0 y observaciones vacías. No se cambia la fuente para simular aceptación. | F2-S28; explain de operadores y pares NX-E0102. |
| A2 §4.6: «No se admite `<` sobre `bool`, unidad, arrays ni `Result`.» | `< <= > >=` solo admiten I64 en F0-0005. `== !=` admiten el mismo dominio estructural de datos A2/J, con Option de J y arrays transitivos; != niega una única comparación. | Tabla A2 §4.6 y S16–S21 con operadores ampliados, F5-O y familias. |
| A2 §4.6: «Si ambos operandos aportan tipos concretos y estos difieren, corresponde `NX-E0102/type_mismatch` sobre el operando derecho.» | Se conserva literalmente para los seis operadores de comparación. expected_type y su relación proceden del izquierdo; si tipos iguales son inadmisibles, primario operador y found_type izquierdo. | A2 §§13.15/13.17; familias `1 OP true` y `true OP 1`. |

Enmienda propuesta de D15 §3.1/§3.5: Assignment usa una sola alternativa
`Place ("=" | "+=" | "-=" | "*=" | "/=" | "%=") Expr Terminator`.
Expr usa la precedencia de §2 y la producción `NegExpr = ("not" | "-") NegExpr | TryExpr` de D15 §3.5; el reconocedor de rango almacenado no le
asigna tipo Range. Se conserva selección IfExpr y la activación de MatchExpr,
closures y generadores de F0-0003. No aparece una segunda gramática aceptada.

#### 12.1.1. Enmiendas propuestas de vocabularios cerrados

Las sustituciones siguientes son literales para f0-0005-b-v1; los perfiles
históricos conservan sus enumeraciones. La grafía concreta I64/Bool sigue D15.
Las extensiones de Entity/roles/operation de §8.1 se integran en el mismo
esquema, sin una segunda variante de Diagnostic; F5-E01 fija Entity constant.

| Autoridad y texto exacto que se enmienda | Nueva redacción normativa |
|---|---|
| A2 §13.6, pending_scope: «`migration`, `F0-0003` o `later_adr`; obligatorio para NX-E0006.» | `later_adr` o uno de `L01`, `L02`, `L03`, `L04`, `L05`, `L06`, `L07`, `L08`, `L09`, `L10`; obligatorio para NX-E0006 en f0-0005-b-v1. L01…L10 exclusivamente con implementation_limit; later_adr con los feature_id de alcance de §8.2. No se emiten migration ni ámbitos I/J. |
| A2 §13.6, runtime_budget_kind: «`call_frames`, `live_value_cells` u `observation_bytes`; obligatorio para R0002.» | `call_frames`, `live_value_cells`, `observation_bytes`, `array_storage_bytes` o `array_allocation_bytes`; obligatorio para R0002 y ausente en NX-E0003. observed es el total candidato completo. |
| A2 §13.6, declaration_kind: «`binding`, `function` o `parameter`; obligatorio para NX-E0104.» | `binding`, `function`, `parameter` o `constant`; obligatorio para NX-E0104, según la segunda declaración. Una segunda declaración const usa constant. |
| A2 §13.12: «Estos diez valores agotan las emisiones de presupuestos de análisis del perfil.» | El perfil f0-0005-b-v1 conserva los diez budget_kind heredados y añade exactamente `comptime_steps`, `comptime_depth` y `comptime_memory_bytes`, los tres en stage=lower, operation=analysis.limit y budget_scope=compilation. Esta unión agota las emisiones de NX-E0003 del perfil; observed=limit+1 e incomplete_analysis=lower para los tres nuevos, sin analysis_function. Se conservan las etapas y reglas de recomprobación heredadas. |
| A2 §13.17, fila completa: «\| `+` o `-`, primer operando incompatible de izquierda a derecha \| `i64` \| Tipo de ese operando. \|» | `+ - * / %`, negación unaria y las cinco compuestas: expected_type=I64; found_type=tipo concreto del primer operando incompatible de izquierda a derecha (Place antes de RHS para compuesta). Primario y relación found_type sobre ese operando; sin expected_type fuente para el dominio intrínseco. `and or not` usan el mismo criterio con expected_type=Bool. |
| A2 §13.17, fila completa: «\| `<` o `==`, operandos de tipos distintos \| Tipo del izquierdo \| Tipo del derecho. \|» | `< <= > >= == !=`, operandos concretos de tipos distintos: expected_type=tipo del izquierdo y found_type=tipo del derecho; primario derecho, relaciones expected_type al izquierdo y found_type al derecho. Esta regla precede al dominio inadmisible. |
| A2 §13.17, fila completa: «\| `<`, tipos iguales pero inadmisibles \| `i64` \| Tipo común. \|» | `< <= > >=`, tipos iguales inadmisibles: expected_type=I64 y found_type=tipo común; primario operador, found_type sobre el izquierdo, sin expected_type fuente. |
| A2 §13.17, fila completa: «\| `==`, tipos iguales pero no copiables \| `domain:copyable_data` \| Tipo común. \|» | `== !=`, tipos iguales fuera del dominio estructural A2/J: expected_type=domain:copyable_data y found_type=tipo común; primario operador, found_type sobre el izquierdo, sin expected_type fuente. |
| F0-CODIGOS, fila R0002 (l.387): «Se excede una cuota estructural de ejecución. runtime_budget_kind pertenece exclusivamente a call_frames, live_value_cells u observation_bytes. Exige limit, observed, profile_id y span de la operación desencadenante. No utiliza budget_kind.» | En f0-0005-b-v1 se excede una cuota estructural de ejecución; runtime_budget_kind pertenece exclusivamente a call_frames, live_value_cells, observation_bytes, array_storage_bytes o array_allocation_bytes. Exige limit, observed (total candidato), Diagnostic.profile_id y span de la operación desencadenante; no utiliza budget_kind ni data.profile_id. |
| F0-CODIGOS (l.574): «Runtime_budget_kind es un campo independiente, cerrado a call_frames, live_value_cells y observation_bytes para R0002. No amplía ni reutiliza la enumeración budget_kind de NX-E0003.» | En f0-0005-b-v1 runtime_budget_kind es un campo independiente, cerrado a call_frames, live_value_cells, observation_bytes, array_storage_bytes y array_allocation_bytes para R0002. No amplía ni reutiliza budget_kind de NX-E0003; las tres ampliaciones comptime de este último se rigen por §7.3. |

### 12.2. Enmienda propuesta de D15 §6.2 y truncamientos

Texto exacto sustituido de D15 §6.2:

```text
ExprStart = ["(","-","IntLiteral","Name","[","edit","false","if","move","true"]
BinaryAccepted = ["+","-","<","=="]
BlockNext = ["(","-","IntLiteral","Name","[","edit","false","if","let","move","return","true","var","while","{","}"]
ModuleNext = ["EOF","fn"]
TryStart = ["(","-","IntLiteral","Name","[","edit","false","move","true"]
PostfixStart = ["(","-","IntLiteral","Name","[","false","true"]
GroupFirst = ["(",")","-","IntLiteral","Name","[","edit","false","if","move","true"]
ArrayFirst = ["(","-","IntLiteral","Name","[","]","edit","false","if","move","true"]
BranchFirst = ["(","-","IntLiteral","Name","[","edit","false","move","true","{"]
TypeStart = ["(","Array","Bool","I64","MockBlockingResource","MockResource","Result"]
```

Convención acumulativa: la base normativa es F0-0003 §10.1, aplicada sobre
D15 §6.2/§6.3. Cada lista es la unión de esa base y los añadidos de este
ADR, sin eliminar comienzos ya admitidos. Las listas históricas citadas
arriba identifican la enmienda a D15; no sustituyen la base de F0-0003.
Las listas activas son cerradas y se ordenan por bytes UTF-8:

```text
ExprStart = ["(","-","IntLiteral","Name","[","edit","false","fn","if","match","move","not","true"]
TryStart = ["(","-","IntLiteral","Name","[","edit","false","fn","move","true"]
PostfixStart = ["(","-","IntLiteral","Name","[","false","fn","true"]
NegStart = ["(","-","IntLiteral","Name","[","edit","false","fn","move","not","true"]
GroupFirst = ["(",")","-","IntLiteral","Name","[","edit","false","fn","if","match","move","not","true"]
ArrayFirst = ["(","-","IntLiteral","Name","[","]","edit","false","fn","if","match","move","not","true"]
BranchFirst = ["(","-","IntLiteral","Name","[","edit","false","fn","move","not","true","{"]
BlockNext = ["(","-","IntLiteral","Name","[","break","continue","edit","false","fn","for","if","let","match","move","not","return","true","var","while","yield","{","}"]
ModuleNext = ["EOF","const","fn","gen"]
TypeStart = ["(","Array","Bool","I64","MockBlockingResource","MockResource","Option","Result"]
BinaryAccepted = ["!=","%","*","+","-","..<","..=","/","<","<=","==",">",">=","and","or"]
```

BlockNext incluye yield en cualquier bloque. Su uso fuera de gen se rechaza
en types con NX-E1213 conforme a F0-0003 G02; no es un error de parse ni
requiere una lista GeneratorBlockNext. Break/continue y for también se
conservan de la base J. Const se añade solo a ModuleNext: no hay const local.
Los rangos y operadores infijos amplían BinaryAccepted; no añaden comienzos
de expresión. Not amplía ExprStart, NegStart y las listas que admiten
NegExpr; TryStart y PostfixStart conservan íntegramente sus listas J porque
not pertenece al nivel NegExpr, anterior a TryExpr.

NegStart es FIRST de NegExpr y también del operando tras `+`, por MulExpr;
if/match requieren agrupación en ese operando y en un operando unario o
BranchOperand. El reconocimiento de signo y decimal conserva un único
literal y sus ticks. Una negación incompleta ya no presupone un decimal.
Se conservan las listas de parámetros, contratos, ElseStmtFirst y
FunctionSuffix de D15 §6.2. Los FIRST internos y diagnósticos proceden de
esta misma gramática; BinaryAccepted no reactiva los rechazos unsupported_*
retirados para operadores aceptados. Los perfiles históricos conservan
sus propios esperados.

| ID propio / fila D15 §6.3 | Token/span | parser_context | expected_tokens recalculados | recognized_prefix |
|---|---|---|---|---|
| F5-S01, `fn Main(){` | eof [10,10) | block_item | `["(","-","IntLiteral","Name","[","break","continue","edit","false","fn","for","if","let","match","move","not","return","true","var","while","yield","{","}"]` | [9,10) |
| F5-S02, `fn Main(){let a=` | eof [16,16) | binding_initializer | `["(","-","IntLiteral","Name","[","edit","false","fn","if","match","move","not","true"]` | [10,16) |
| F5-S03, `fn Main(){let a=[1,` | eof [19,19) | array_element | `["(","-","IntLiteral","Name","[","edit","false","fn","if","match","move","not","true"]` | [16,19) |
| F5-S04, `fn Main(){let a=-` | eof [17,17) | grammar.NegExpr | `["(","-","IntLiteral","Name","[","edit","false","fn","move","not","true"]` | [16,17) |
| F5-S05, `fn Main(){let a=1;} garbage` | name garbage [20,27) | module_item | ["EOF","const","fn","gen"] | [0,20) |
| F5-S06, `// expect: NX-E0002` + LF + `fn Main() {} ignored` | name ignored [33,40) | module_item | ["EOF","const","fn","gen"] | [0,33) |
| F5-S07, misma fuente CRLF | name ignored [34,41) | module_item | ["EOF","const","fn","gen"] | [0,34) |
| F5-S08, `fn F(x:view){}` | symbol ) [11,12) | grammar.Type | `["(","Array","Bool","I64","MockBlockingResource","MockResource","Option","Result"]` | [11,11) |
| F5-S09, `fn Main(){let a=1+` | eof [18,18) | grammar.AddExpr | `["(","-","IntLiteral","Name","[","edit","false","fn","move","not","true"]` | [16,18) |
| F5-S10, `fn Main(){let a=not` | eof [19,19) | grammar.NegExpr | `["(","-","IntLiteral","Name","[","edit","false","fn","move","not","true"]` | [16,19) |
| F5-S11, `fn Main(){for` | eof [13,13) | grammar.ForStmt | ["Name","edit"] | [10,13) |
| F5-S12, `fn Main(){var x=1;x+=` | eof [21,21) | grammar.Assignment | `["(","-","IntLiteral","Name","[","edit","false","fn","if","match","move","not","true"]` | [18,21) |
| F5-S13, `const` | eof [5,5) | grammar.ConstDecl | ["Name"] | [0,5) |

Las listas de la tabla son expansiones literales de los conjuntos anteriores,
no el resultado del parser. Código NX-E0002, parse/syntax.parse,
subcause null, data {}, expected_tokens_kind=admissible_set, token.span
igual al primario, sin relaciones/cadenas/repair; contratos D15 §6.3 restantes.
Positivos pareados: F5-S01 añadir `}`; F5-S02 añadir `0;}`; F5-S03 añadir
`2];}`; F5-S04 añadir `1;}`; F5-S05–07 quitar únicamente el nombre sobrante;
F5-S08 insertar I64 después de view. Conservan perfil, unit y cero salida.
F5-S09 añade `1;}`; F5-S10 añade ` true;}`; F5-S11 añade
` i in 0..<1{}}`; F5-S12 añade `1;}`; F5-S13 añade
` k:I64=comptime 1;fn Main(){}`. EOF tiene text="". En S11, edit es
el prefijo reconocido de ForStmt; consumirlo conserva for_iteration/later_adr,
no autoriza iteración exclusiva. El positivo elige la alternativa sin edit.
Las filas de operadores ahora válidos no conservan expected_tokens como
si fueran errores: producen diagnostics=[]; sus negativos son los de tipos.

Las restantes filas negativas de D15 §6.3 conservan estos contratos;
los spans y prefijos son bytes de sus fuentes literales, sin LF final:

| Fuente / fila D15 §6.3 | Token/span | parser_context | expected_tokens | recognized_prefix |
|---|---|---|---|---|
| `fn` | eof [2,2) | function_name | ["Name"] | [0,2) |
| `fn Main(` | eof [8,8) | parameter_list | [")","Name"] | [7,8) |
| F2-S22, `fn Main(){let match=1;}` | keyword match [14,19) | binding_name | ["Name"] | [10,14) |
| F2-S30, `fn F(x:edit I64){} fn Main(){F(edit 1);}` | integer 1 [36,37) | inout_place | ["Place"] | [31,36) |
| F2-S31, `fn Main(){let x=move 1;}` | integer 1 [21,22) | consume_place | ["Place"] | [16,21) |
| F2-K02, `fn Main() ;` | symbol ; [10,11) | function_suffix | ["->","with","{"] | [0,10) |
| `fn F(x:I64){}` | name I64 [7,10) | grammar.ParamMode | ["edit","move","view"] | [7,7) |
| `fn Main(){let x=if true{value:1;}else{value:2;};}` | symbol { [23,24) | grammar.IfExpr | ["then"] | [16,23) |
| `fn Main(){let x=0..1;}` | symbol .. [17,19) | range_endpoint | ["..<","..="] | [16,17) |

F2-S27/S28/S32/S33/S34, F2-LX01/LX02 y las tres filas not/and/or
son positivas con diagnostics=[] según §12.1: no tienen token inesperado,
recognized_prefix ni expected_tokens diagnósticos. F2-K01 y F2-LX03–05
conservan NX-E0001, ["valid_token"], lexical_category, lexical_token y
prefijo [0,start), con sus spans literales de D15 §6.3.

La base F0-0003 §2.1 admite «Closures síncronas
locales con bloque, modos de captura completos, inferencia monomórfica local,
invocaciones de lectura, exclusiva y consumidora.» y «Declaraciones superiores
`gen fn`». Sus FIRST publicados en §10.1 se conservan en la unión anterior.

### 12.3. Enmiendas propuestas de F0-0003 y frontera con F0-0004

Se aplican estas reglas sobre el contrato acumulativo A2+J, sin modificar
el archivo de J. Coinciden con F0-0003 §2.2, sin enmienda, las filas
«for_iteration: for edit» / «later_adr», «closure: arrays con closure transitiva»
/ «later_adr» y la regla «`implementation_limit` solo identifica una capacidad
que un paso posterior de F0 implementa realmente; nunca una limitación permanente.»
La prioridad de resource_array sobre closure se conserva. Las citas siguientes
identifican exclusivamente las fronteras que amplía F0-0005.

| Texto exacto de F0-0003 | Nueva redacción / filas afectadas |
|---|---|
| §2.2: «for_iteration: Array» / «later_adr» | La iteración de Array entra en F0-0005 §3.2. Hasta F5-L06 usa implementation_limit/L06; después conserva préstamos, NX-E1104 y NX-E1326 de J13. No afecta a `for` generador ni a for edit. |
| §2.2: «float_literal, text, range, compound_assignment, tuple, no_heap_alloc» | range solo rechaza fuera de for directo y en slices; compound_assignment deja de ser exclusión para las cinco formas de §3.3; los demás IDs conservan su frontera. |
| §2.2: «attributes, comptime, default_argument, extensions, modules_visibility, traits_generics, user_types» | comptime admite solo §5; los demás conservan sus exclusiones. pub const sigue modules_visibility/later_adr. |

El catálogo propuesto conserva 30 feature_id y añade solo las extensiones
de esquema enumeradas en §8: NX-E1327/arithmetic_fault, operaciones value.mul,
value.div, value.rem, value.neg, const.evaluate, const.check; constant y
dependency como roles; constant como Entity/declaration_kind; y los tres
presupuestos comptime y dos de bytes Array. domain:iterable es heredado de
J §2.2, no un dominio nuevo de este corte. La constante sin almacenamiento
usa NX-E0105 conforme §5, sin ampliar el vocabulario de tipos.

F0-0004 deberá integrar los datos nuevos sobre estas reglas (frontera §1.1),
sin imponer aquí sus perfiles, su orden de String, su derivación nominal ni
sus códigos. JoinArrays es una función libre de Array y no ocupa nombres de métodos de
tipos que definan cortes posteriores. La nota de §4.1 preserva la obligación de niveles
de F0_ACUERDOS §9.4: «Las construcciones de alto nivel se escriben **con** las
de bajo nivel». La simplificación de este prototipo no enmienda esa visión.

P01/P03/P09 se concretan en una superficie, costes visibles y diagnósticos
causales; P02/P04–P08 conservan restricciones, reproducibilidad y límites de
la base sin prometer rendimiento o adopción medidos. D02 conserva la
distinción de errores tipados y fallos experimentales; D03 aporta const
acotado; D04–D06 no cambian backend, plataformas ni nombre. D08 no obtiene
traits/sobrecarga generales por estos intrínsecos; D10 conserva la frontera de
generadores. EX01 necesita además E/S/texto; EX02 HTTP/concurrencia; EX03
vistas/layout/float; EX04 completo conversiones y módulos. Ninguno se declara
cumplido por estos nueve algoritmos reducidos.

Pruebas directamente ampliadas: T01 independencia, T02 no copiar recursos,
T07 enteros, T08/T10 duración y T14 efectos; T17 operadores sin afirmar
coherencia genérica completa; T20 evaluación const sin derivación general;
T22 iteración/salida temprana; T25 diagnósticos; T26 escala de este intérprete
sin atribuir los objetivos del compilador grande. T03–T06/T09/T11–T13/
T15–T16/T18–T19/T21/T23–T24/T27–T28 conservan las obligaciones de su fase y
las regresiones A2/J que correspondan; se registran NO EVALUADO cuando requieran
componentes ausentes. No se usa
un test de consola para dar por probadas FFI, concurrencia, plataformas,
comprensión humana ni las puertas pendientes del roadmap.
