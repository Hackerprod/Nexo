# F0-0003 — Option, selección, closures locales y generadores propietarios

Estado: **decisiones de arquitectura revisadas de forma independiente; sin implementación ni aprobación de fase**.
Fecha: 2026-10-25. Perfil objetivo: `f0-0003-b-v1`.

## 1. Autoridad y compromiso

Este es un corte de implementación del prototipo, no una aprobación de F0/F1/F2.
Conserva [AGENTS.md](../../AGENTS.md), [F0_ACUERDOS](../F0_ACUERDOS.md),
§§2–4 y §9, y la base inmutable [rev 0.2](../baseline/Nexo_ROADMAP_rev0.2.md):
P01–P09, D01–D10, EX01–EX04, F0/F1 y T01–T28. No crea una rev 0.3.
La superficie es exclusivamente [D15](F0-D15-sintaxis-oficial.md), incluida
su B completa. [D14](F0-D14-sintaxis-superficie.md) aporta motivaciones, no
alternativas activas. [D11](F0-D11-closures.md), [D12](F0-D12-async.md) y
[D13](F0-D13-expresiones-modulos.md) aportan las reglas semánticas; §10 registra
las concreciones y enmiendas propuestas. El [catálogo](F0-CODIGOS.md) sigue
siendo el registro único: las adiciones de §8 deben integrarse allí al implementar.

Se conserva la semántica de [F0-0001](F0-0001-first-slice.md) enmendada por
[F0-0002](F0-0002-second-slice.md), abreviado A2, y D15. A2 §1 asigna a este
corte closures locales, préstamos retenidos, generadores propietarios, Option,
selección para Next y EX04 reducido. No se reemplaza el generador por un array
precocinado ni se interpreta el texto fuente durante ejecución.

El objetivo es una composición inspeccionable de valores independientes,
propiedad explícita, alternativas y efectos. La contribución a un lenguaje
agent friendly es poder determinar localmente adquisición, retención, avance
y liberación, con diagnósticos causales y límites declarados. No se atribuye
facilidad de uso medida a estas decisiones.

## 2. Alcance y fronteras

### 2.1. Dominio aceptado

Un archivo privado conserva todas las capacidades de A2 completado. Añade:

- `Option<T>`, `Some(e)` y `None`; payloads con los mismos tipos de A2 y
  anidamiento de Option/Result. También pueden contener tipos anónimos inferidos
  de closures y estados de generador dentro del módulo.
- `match` sentencia y expresión; patrones de Bool, I64, Option y Result,
  anidados, y `_`/bindings sobre cualquier tipo incluido; modos `edit`/`move`
  y guardas. Los otros tipos se tratan como dominios opacos para cobertura.
- Closures síncronas locales con bloque, modos de captura completos, inferencia
  monomórfica local, invocaciones de lectura, exclusiva y consumidora.
- Declaraciones superiores `gen fn`, Item explícito o unidad por omisión, parámetros propietarios
  y la materialización escalar definida en §5.1; `yield`, fin y avance síncrono.
- `Iterator.Next(edit g)` como operación nominal intrínseca; `for x in e`
  sobre un generador propietario, `break` y `continue` sin etiquetas en for/while.

No se añade `if let`: no es una producción de D15. No se añade aritmética,
rangos, conversión numérica ni un tipo fuente `Generator<T>`. Los estados tienen
identidad concreta inferida; escribir ese nombre de tipo conserva unsupported_type.
Las funciones ordinarias siguen anotando resultados no unitarios. Una closure
puede inferir parámetros/resultados a partir de restricciones de su cuerpo y
usos locales. En `gen fn`, omitir `->` fija Item=(), no infiere Item de
los yield ni cambia el tipo de la llamada: esta siempre construye un estado.
Un yield no unitario sin anotación da NX-E0102/type_mismatch, no
inferred_non_unit_result. D13 permite inferencia más amplia fuera del perfil.

Arrays siguen limitados a datos copiables e independientes: no admiten recursos,
generadores ni closures, incluso una closure vacía. Para estas últimas la
exclusión específica es §2.2, no una afirmación falsa de que toda closure sea
un recurso. Option/Result sí transportan closures con préstamos y conservan
sus procedencias. Igualdad, AssertEq y Print se amplían a Option de datos;
no se amplían a invocables ni estados, aunque una closure admita copia.

### 2.2. Tabla cerrada de exclusiones y capacidad pendiente

Se mantiene el conjunto de 30 feature_id de D15 §3.6, sin nuevos identificadores.
Los códigos de alcance son `NX-E0006`, subcause null, sin ejecutar nada.
Para las características que este ADR incorpora, una construcción todavía no
implementada usa `feature_id=implementation_limit` y su J concreto. Una vez
cerrada esa obligación, la misma entrada debe aceptarse o alcanzar su error
semántico; no puede conservar ese rechazo como sustituto de la comprobación.

| feature_id / dominio fuera del corte completo | pending_scope | Sitio y etapa |
|---|---|---|
| async, region | later_adr | Introductor; parse/syntax.parse. Los tipos escritos de futuro/tarea usan async, types/types.check, sobre el tipo completo (la etapa es la de A2 §13.12). Incluye cancelación y publicación. |
| attributes, comptime, default_argument, extensions, modules_visibility, traits_generics, user_types | later_adr | Reconocedor de D15; parse. Incluye Call* escrito, genéricos de usuario y dyn. |
| float_literal, text, range, compound_assignment, tuple, no_heap_alloc | later_adr | Reconocedor de D15; parse. Operadores excluidos conservan NX-E0002, no esta fila. |
| unsupported_type | later_adr | Ruta completa de tipo no incluido; types/types.check, data.type_spelling original además de pending_scope. Incluye Generator escrito; Option está admitido. |
| function_value | later_adr | Función nombrada en posición de valor; names. Llamarla directamente sigue admitido. |
| inferred_non_unit_result | later_adr | Solo función ordinaria según A2; types. Gen sin flecha fija Item=(). |
| provenance_clauses | later_adr | from/result_from/retains escritos; parse. Procedencia inferida local sí entra. |
| methods | later_adr | Selectores salvo la identidad intrínseca Iterator.Next y self expresión; names/names.resolve después de reconocer la forma. No se implementan métodos de usuario ni g.Next(). |
| partial_transfer | later_adr | move/Drop de proyección de propietario existente, conforme A2. La extracción de un patrón desde un temporal de selección es una operación distinta y admitida. |
| resource_array | later_adr | Array con recurso o estado transitivo; types, construcción completa. |
| closure: arrays con closure transitiva | later_adr | Construcción del array o anotación que fija el elemento, types. Incluye copiable sin préstamos; no inventa resource_array. |
| for_iteration: for edit | later_adr | Token edit, parse; toda iteración exclusiva permanece diferida, también sobre generador. |
| for_iteration: Array | later_adr | Expresión iterada, types; Array es iterable reconocido pero su recorrido queda fuera de este corte. |
| Tipo no iterable en for | — | NX-E0102/type_mismatch, types/types.check, expresión iterada; expected_type=domain:iterable y found_type concreto, relación found_type en la misma expresión. |
| receiver_mode: función ordinaria | later_adr | Prefijo edit/move completo sobre llamada resuelta a función ordinaria; names/names.resolve. No se aplica a invocación de closure. |
| generator: recursión de estados | later_adr | Nombre de la primera declaración gen del componente cíclico por offset; types/types.check, según §5.1. |
| closure, generator, option, match, loop_control | — | No son rechazos globales del perfil completo: sus formas incluidas se comprueban. Las exclusiones interiores conservan el ID pertinente de otras filas. |

`implementation_limit` solo identifica una capacidad que un paso posterior de
F0 implementa realmente; nunca una limitación permanente. No se permite usarla
para otra capacidad. El orden es F0-0003 → F0-0005 → F0-0004. Los ámbitos I,
J, F5 y K pertenecen a sus respectivos perfiles; aquí los únicos pending_scope
emitibles son `later_adr`, `J02`, `J03`, `J04`, `J05`, `J06`, `J07`, `J08`,
`J09`, `J10`, `J11`, `J12`, `J13`, `J14`. I08–I11 son prerrequisitos; no se
emiten ni esos ámbitos ni F0-0003 como barrera de este perfil. Las capacidades
fuera del perfil mantienen su feature_id con later_adr; al entrar en un corte
posterior solo su obligación concreta puede usar implementation_limit.
Los cortes posteriores deben conservar el mismo canon: for edit usa
for_iteration/later_adr y arrays con closure usan closure/later_adr; la
iteración por valor de Array requiere una obligación de un corte posterior.
Si un array contiene una closure con recurso o estado, resource_array tiene
prioridad; closure/later_adr cubre las restantes closures, incluidas las
copiables. Un J completo no conserva su rechazo temporal.

| Capacidad incluida aún pendiente | pending_scope | Ancla del rechazo temporal |
|---|---|---|
| Option en tipos/constructores | J02 | Tipo completo o constructor, types |
| match de lectura y cobertura/tipos | J03 | match, types |
| match consumidor/exclusivo y préstamos de patrón | J04 | Modo del scrutinee, ownership |
| Guardas de match | J05 | if de la guarda, ownership |
| Closure con campos propietarios | J06 | fn del literal, types |
| Captura retenida view/edit, explícita o inferida | J07 | fn del literal, ownership |
| Modo de receptor e invocación de closure | J08 | Destino de llamada, arguments |
| Resúmenes create/copy/call/drop de closure | J09 | fn del literal, effects |
| Construcción/descarte de generador | J10 | gen, types |
| Next, yield y estado suspendido | J11 | Next en llamada o yield en cuerpo, types |
| SuspendSafe y resúmenes de generador | J12 | gen, effects |
| for propietario, break/continue | J13 | Introductor, parse |
| Contabilidad completa de las nuevas operaciones | J14 | Primer nodo nuevo por orden fuente, lower |

Las barreras son por capacidad, no por patrones de fuente. Se reconocen AST y
símbolos; no se acepta un artefacto ejecutable que dependa de una obligación
todavía pendiente. Subpasos previos pueden validar IR/modelo internamente, pero
no presentar como comprobado un programa al que falten efectos o cuotas.
J15/J16 no son excusas de alcance: integran y verifican capacidades ya completas.
Si concurren límites, se aplica la primera barrera de A2 §13.9 y, en ella,
el primer sitio fuente. Los errores sintácticos reales preceden al alcance.

## 3. Option y selección

### 3.1. Valor opcional

Option es una unión nominal intrínseca de `Some(T)` y `None`, con un tag y,
solo para Some, un payload. None es expresión nular, no llamada `None()`.
Some toma un único argumento propietario con etiqueta `v`, igual que Ok/Err en D15 §2.3; no acepta
argumentos de tipo explícitos en expresión. Su inferencia reutiliza las
restricciones de A2 I07c/I08; None necesita determinar T y emite NX-E0113
con `unresolved_components=["$.some"]` si no hay solución única. Some puede
resolver T por su operando. Nunca se elige unidad o I64 para un hueco.

Copia, independencia, permisos retenidos y destrucción se derivan
transitivamente. Option<MockResource> no es copiable, ni siquiera en None.
Construir el tag inline no tiene alloc; copiar un payload Array sí lo tiene.
La ejecución destruye solo el payload activo. El resumen estático de drop
incluye T salvo evidencia de variante verificada; no altera la copiabilidad.
Print produce `None` o `Some(<payload>)`; igualdad compara tag y después
payload únicamente cuando ambos son Some. No existe null ni conversión a Bool. `None()` se parsea como llamada pero
usa NX-E0111/non_callable_target en types/call.bind, primario None, Data {},
sin relaciones: None es variante nular, no invocable. Se suprime la inferencia
del payload dependiente de esa llamada inválida. `Some<I64>(1)` es GenericHead
de D15 completo y conserva NX-E0006/traits_generics en parse, primario `<`,
pending_scope=later_adr; no se reinterpreta como comparación.
`?` permanece exclusivo de Result: Option no adquiere propagación de ausencia.

### 3.2. Gramática y resolución de patrones

Se activan MatchExpr, MatchStmt y Arm de D15 §3.5, sin modificar la separación
entre sentencia y expresión ni los terminadores. El subset de Pattern es `_`,
entero I64, true/false, Name, `edit Name`, `move Name`, y constructores
Some/Ok/Err con un patrón y None sin argumentos. Agrupar un patrón no crea
producto; la tupla continúa diferida. Se permiten anidamientos finitos.

La resolución se hace por identidad del constructor y tipo esperado. Un Name
desnudo que no sea variante nular del tipo esperado introduce binding de lectura,
con independencia de su caja. Por ello `Some` sin paréntesis en patrón de
Option es un binding según D13 G.2, no una variante nular inventada. Un nombre
con argumentos debe resolver un constructor: un nombre ausente da NX-E0101;
constructor de otra unión o aridad incorrecta usa NX-E1325 (§8). Bajo un scrutinee Result, `None` desnudo introduce un binding de lectura;
no resuelve la variante de Option ni produce constructor_type. La repetición
de un binding en un patrón usa NX-E0104/declaration_kind=binding.
Un patrón literal de tipo distinto al seleccionado usa NX-E0102/type_mismatch:
primario/found_type son el literal y expected_type relaciona el scrutinee que
impone el tipo. Los dominios opacos solo se cubren con `_` o binding; no se
descomponen arrays, recursos, closures ni estados mediante patrones nuevos.

El scrutinee se evalúa una vez. Un Place sin modo se presta para lectura;
`match edit p` toma un préstamo exclusivo; `match move p` transfiere a un
temporal propietario. Una expresión temporal nueva ya es propietaria. No se
copia un Place implícitamente para convertirlo en scrutinee propietario.

Los bindings sin modo son aliases de lectura limitados al brazo. Los `edit`
exigen permiso exclusivo del scrutinee; los `move` exigen temporal propietario.
Se comprueban permisos incluso en ramas no ejecutadas. Un temporal permite
préstamos de lectura pero no concede exclusividad implícita: edit exige el modo
escrito del scrutinee. Los bindings move son propietarios inmutables. Edit
permite reemplazar el payload, sin extraerlo dejando hueco en el origen prestado.

El match propietario desarma el binding original antes de seleccionar un brazo.
Cada extracción move desarma solo la ruta del temporal correspondiente. El
temporal termina al salir del brazo; limpia los campos restantes exactamente
una vez, también ante return, `?`, break, continue o fallo experimental. Un
match prestado termina sus permisos sin destruir el origen. `_` no copia,
no extrae y no descarta anticipadamente el propietario de selección.

### 3.3. Cobertura, guardas y uniones

Match es exhaustivo como sentencia y expresión, incluso sobre una constante.
La cobertura usa el tipo, no el valor conocido del scrutinee. Bool tiene dos
alternativas; Option/Result descomponen recursivamente sus payloads; I64 exige
un patrón residual salvo que se cubra realmente su dominio completo, caso que
el perfil no intenta enumerar. El algoritmo mantiene residuos simbólicos, sin
expandir productos cartesianos de valores. Se carga cada descomposición,
intersección y comparación de nodo al presupuesto de tipos. No se acepta una
exhaustividad no terminada por falta de cuota.

Se elige un testigo residual canónico: orden false/true, None/Some, Ok/Err;
para I64, primer entero no cubierto de 0, 1, -1, 2, -2, etc.; para un payload
opaco, `_`. Se emite solo el primer testigo. `missing_pattern` contiene su patrón en
texto canónico sin espacios: `false`, `true`, `None`, `_`, `Some(P)`, `Ok(P)`
o `Err(P)`, recursivamente. I64 se escribe en decimal sin ceros iniciales ni
signo +; cero es `0` y los negativos llevan `-`. No lleva etiqueta de payload,
calificación prelude, comillas internas ni paréntesis de agrupación. Por ejemplo,
`Some(Err(false))` y `-1` son cadenas completas, no descripciones de dominio. Un brazo cubierto por la unión
de patrones anteriores sin guarda usa NX-E1304/unreachable_arm. Una guarda
no aporta cobertura; una literal true tampoco. Se denuncia el primer brazo
inalcanzable y se suprime el no_exhaustive derivado de ese mismo match inválido.

Tras casar estructura, todos los nombres de patrón se exponen provisionalmente
como lecturas para la guarda. Solo si esta devuelve true se adquieren permisos
edit o se transfieren payloads. False libera las lecturas y prueba el siguiente
brazo sin cambiar propiedad. No se evalúan guardas de patrones que no casan.
La guarda debe ser Bool, sin transferencia, mutación, alloc, block, suspend,
io ni efectos desconocidos. Se aplica NX-E1305: primero guard_transfer,
después guard_mutation, después guard_effect por sitio causal; un mismo sitio
solo emite el primero aplicable. Esta precedencia no borra sitios independientes.
El análisis de intención detecta edit/move aun si el callee no usa su permiso.
En ownership, antes de aplicar transferencias de la guarda, se detectan también
consumos intrínsecos como Drop. Guard_transfer y guard_mutation tienen prioridad
causal sobre maybe_consumed, definitely_consumed, NX-E0105 y conflictos de
préstamo derivados de esa operación prohibida. La operación inválida se trata
como lectura para continuar el análisis; no consume ni cambia el estado unido.
Esto no suprime errores independientes en ownership ni adelanta esta barrera
respecto de names, arguments o types.

Una guarda no puede abandonar su evaluación: `?`, return, break, continue y
yield en su contexto inmediato usan NX-E1305/guard_control en ownership,
primario en el token de control, Data {}, relaciones y cadena vacías. Incluye
return dentro de un if expresión. Los cuerpos de invocables anidados tienen
su propio contexto; sus efectos de llamada siguen sujetos a guard_effect.
La comprobación de salida precede a los flujos derivados del mismo nodo;
no se ejecuta ni compromete extracción si existe ese diagnóstico.

Matriz de barreras: parse (forma inválida), names (resolución), arguments
(asociación), types (Bool y tipos de operandos), ownership (guard_control,
guard_transfer, guard_mutation y errores independientes), effects (guard_effect),
lower. En un mismo sitio control precede a transfer y transfer a mutation;
los sitios independientes se ordenan según A2. Q02–Q04/Q06/Q07 alcanzan ownership;
Q01 alcanza effects solo si ownership no falla.

Cada continuación normal de MatchExpr termina en `value:` directo; Return y
propagación tienen su destino invocable normal. MatchStmt no recibe value:.
La unificación es la de A2 §§4.1.1/4.7: tipo esperado implica NX-E0102;
sin él, discrepancia entre brazos implica NX-E1303. Never no es incógnita.
Dos closures de sitios distintos siguen siendo tipos distintos aunque sus
firmas coincidan. No hay boxing ni conversión para unirlas.

Propiedad se une sobre caminos que continúan; préstamos conservan la unión
de orígenes posibles, no la vida más larga. No se deducen correlaciones entre
matches sucesivos. Efectos incluyen scrutinee, guardas, todos los brazos y
limpiezas, incluidas ramas constantes no tomadas. Yield sí tiene continuación
al reanudar: no se excluye de la unión como si fuera Return.

## 4. Closures locales

### 4.1. Entorno e inferencia

Solo se admite el bloque de D15; no hay cuerpo con flecha. Cada literal tiene
identidad nominal anónima por sitio y un entorno con campos ordenados. Sin
lista, captura view de todos los libres en primera aparición textual, incluidas
capturas de closures interiores. Con lista, esta es exhaustiva y sin duplicados;
[] significa vacía. Funciones superiores e intrínsecos no son capturas.
Una entrada explícita no usada conserva adquisición, efectos y limpieza.

Capture admite exclusivamente bindings completos. View retiene lectura, edit
retiene exclusividad, copy crea valor lógico independiente cuando su tipo lo
permite y move transfiere propiedad. Copy de una closure con lectores copia
sus permisos y conserva sus orígenes: no fabrica independencia. Copy de un
permiso exclusivo o recurso usa NX-E0201. Campos copy/move conservan la
mutabilidad del binding original; view no la adquiere por observar un var.

Las restricciones de tipos de una closure se resuelven en el mismo grafo
monomórfico del invocable contenedor. El modo de cada parámetro siempre se
escribe; su tipo puede inferirse del cuerpo y de todas las llamadas locales,
sin generalización por llamada. Los return determinan el resultado; sin return
no unitario, caída implica unidad; una anotación no unitaria exige retorno
en todas las continuaciones. Una firma sin solución única usa NX-E1111, antes
de comprobar sus usos dependientes. No se infiere un tipo provisional I64.
El nombre del binding no está en ámbito durante su inicializador; no hay
autocaptura implícita. El cuerpo interior tiene su propio return y contexto yield.

### 4.2. Invocación y efectos

Modo mínimo: lectura si solo observa; mutable si modifica un campo o usa edit
capturado; consumidor si transfiere/destruye un campo propietario en cualquier
camino. Se generan capacidades nominales Call/CallMut/CallOnce de D11; no
se ofrece todavía su gramática como cota de usuario. Lectura implementa las
tres, mutable las dos últimas y consumidor solo CallOnce. Poseer un recurso
no obliga a consumir el receptor si el cuerpo solo lo inspecciona.

`f(args)` solicita lectura; `edit f(args)` solicita exclusividad y binding
mutable; `move f(args)` consume el entorno entero. El prefijo califica al
receptor de la última llamada. Un modo insuficiente usa NX-E1107; un receptor
inmutable con modo edit suficiente usa NX-E0105. La invocación consumidora
desarma f aunque la rama tomada no consuma campos. Sus restos se destruyen
al terminar, también por Err o fallo experimental. Se permiten modos más
fuertes explícitos. Etiquetas, evaluación de argumentos y préstamos durante
la llamada conservan A2: receptor primero, argumentos en orden escrito.

`edit Foo()` o `move Foo()` con Foo resuelta a función ordinaria carece de
receptor: conserva NX-E0006/receiver_mode, pending_scope=later_adr, ahora
names/names.resolve, primario en el prefijo. No se inventa un Place mutable
para una función ni se aplica el modo al resultado. La identidad de una closure
sí activa las reglas anteriores. La clasificación de un binding invocable se
resuelve en el grafo común: non_callable_target pasa a types/call.bind; los
restantes defectos de asociación conservan arguments/call.bind cuando la firma
está determinada. No se rechaza un binding solo por ser binding.

El receptor se mantiene prestado durante argumentos y cuerpo; un argumento
que intenta consumirlo o adquirir permiso incompatible es rechazado. Una
closure recién construida puede llamarse por lectura y su temporal se destruye
al acabar; edit/move sobre llamada exigen receptor Place, por lo que una
closure consumidora temporal se almacena primero en un binding. Un operando
que no satisface Place conserva NX-E0002 y el contexto gramatical de D15;
no se introduce un dominio de tipos place ni se mueve el resultado de la llamada.

Cuatro resúmenes: create, copy, call y drop. La representación del entorno
es inline, de tamaño conocido; su cabecera no aporta alloc. Capturar/copiar
Array conserva alloc; capturar move un recurso no ejecuta su destructor.
Call incluye cuerpo, temporales y todas las salidas, y los restos del entorno
solo en llamada consumidora. Drop incluye campos propios todavía vivos y
liberación de permisos. Crear y abandonar requiere create ∪ drop, sin call.
Una cota simple del literal limita call. Los mapas siguen D15 §3.3 y sus
prioridades, sin dar efectos vacíos a operaciones omitidas en una cota escrita.
La inferencia del cuerpo y la obligación escrita son datos separados.

### 4.3. Retención, escape y limpieza

Los permisos retenidos duran hasta destruir el portador, no hasta su última
llamada. Moverlo conserva LoanId/procedencia; copiar lectores crea una obligación
adicional. Destruir uno de dos lectores no libera el otro. Option/Result
propagan estos hechos, incluidas alternativas que el análisis no distingue.
Las raíces y rutas identifican almacenamiento lógico, nunca texto de nombres.

El análisis prueba duración en cada instalación, entrega de rama, retorno y
reasignación de un agregado; una closure prestada a un local que termina antes
que su destino da NX-E1104 incluso envuelta en Some/Err. No se adelanta su
destrucción ni se reserva memoria para salvar el escape. No se habilita
retención externa: no hay parámetros Call*, globals ni contratos from/retains.
Se admite entregar entre ámbitos un entorno puramente propietario o prestado
de un origen que siga vivo. La inferencia local conserva todas las procedencias.

Desde un campo view no se obtiene edit/move del origen. Desde edit puede
crearse un représtamo subordinado: congela los accesos incompatibles del padre
hasta su destrucción. Se usa el mismo motor que en préstamos de argumentos.
Copiar/mover un portador no elimina la congelación ni cambia el origen.

Capturas se adquieren en el orden de lista o primera aparición; limpiar es
en orden inverso de adquisición. Ante fallo parcial solo se limpian campos
instalados, sin revivir los transferidos. Las liberaciones de préstamos se
ordenan tras los destructores que puedan necesitar esos permisos. Cada recurso
mantiene una identidad y una reserva de cierre única a través de todos los
portadores. Una asignación evalúa RHS antes de destruir el portador reemplazado;
por ello no puede adquirir un préstamo incompatible confiando en que el viejo
se liberará después.

## 5. Generadores e iteración

### 5.1. Creación y representación propietaria

`gen fn G(...) -> T` declara Item=T; sin `-> T`, Item=(). G(...) produce
estado concreto no copiable incluso con Item unitario. `yield ();` entrega
Some(()); caída o return vacío finalizan con None. Un yield de otro tipo usa
NX-E0102 sobre su operando, expected_type=(), found_type concreto y relación
found_type sobre ese operando; sin flecha no se inventa related_span de tipo
esperado. NX-E1324 para return con operando conserva su ancla return; si no
hay flecha, sus relaciones son [] en vez de apuntar a una anotación inexistente.
No ejecuta ninguna sentencia del cuerpo ni inicializador local. Evalúa argumentos
una vez en orden escrito, los prepara y transfiere al estado en orden de parámetros.
Un parámetro move recibe un temporal o copia/transferencia explícita según A2.
Un parámetro view de I64, Bool o () se materializa como copia propietaria al
crear el estado; no retiene un préstamo al llamador. Se conserva como permiso
de lectura dentro del cuerpo. Los restantes parámetros view y todos los edit
dan NX-E1110 en su modo, incluso si no se usan. Para Array se exige un parámetro
move y una copia o transferencia en la llamada. Esta concreción escalar de D10
y D14 §7.5 es explícita en §10; no se generaliza a closures o fn ordinarias.

El estado usa slots de tamaño fijo conocidos tras resolver tipos, flags de
vida y contador de continuación; no heap implícito del modelo. Los valores
Array que contenga conservan su almacenamiento/coste propio. El grafo de
contención inline debe ser acíclico: contener otro estado del mismo ciclo
de layout, incluso indirectamente por Option/closure, usa
NX-E0006/generator, pending_scope=later_adr, types/types.check, primario en
el nombre de la primera declaración gen del componente por offset, Data solo
pending_scope, relaciones y cadena []. Es una limitación permanente de F0:
se reserva layout para todos los slots declarados, incluso estados locales
que se destruyen antes de yield o que aparecen en caminos no ejecutados. Por
ello se rechaza toda recursión directa o mutua entre construcciones gen, sin
optimizar slots por vida para admitirla. No se atribuye esa restricción al
lenguaje completo ni a D10. Las llamadas ordinarias recursivas siguen
permitidas; no equivalen por sí solas a contención de estados. No se inserta
boxing para resolver el ciclo. NX-E1323 conserva su registro para tamaño
recursivo; no es la emisión fuente de esta exclusión. El verificador sigue
rechazando ciclos inline en IR alterada y comprueba propiedades transitivas.

### 5.2. Next, Yield y finalización

El preámbulo resuelve Iterator y su miembro Next por identidad nominal sellada.
La forma fuente es `Iterator.Next(edit g)`, con receptor explícito sin etiqueta
(D15 §3.5, llamada nominal `Trait.M(edit x)`; D13 G.1). `self` es palabra
reservada, no una etiqueta Name. No hay
alias libre Next ni reconocimiento por coincidencia de texto. Un binding que
sombrea Iterator impide resolver ese intrínseco; no obtiene privilegios.
Una declaración de tipo de usuario `Iterator` conserva en este perfil
NX-E0006/user_types en parse sobre su introductor. En el dominio que admita
tipos de usuario, el homónimo sombrea el preámbulo por resolución ordinaria;
no adquiere Next ni conformidad por su grafía. §10 propone esta regla para
los documentos que amplíen ese dominio.
El parser reconoce selectores; el resolvedor distingue esta operación de
métodos pendientes. Next exige Place mutable con estado de generador, presta
exclusivamente durante el avance y devuelve Option<Item> propietario.
Antes de tipos, omitir edit o escribir move en el argumento usa
NX-E0111/argument_mode_mismatch, arguments/call.bind, primario en el argumento
completo, Data {}, relaciones y cadena []; el receptor intrínseco no tiene
parámetro declarado en fuente que pueda relacionarse. No se copia/consume el
argumento inválido ni se deriva NX-E0201 de esa asociación rechazada.
Un tipo concreto distinto de estado usa NX-E0102/type_mismatch en el argumento,
expected_type=domain:generator y found_type concreto; se añade ese dominio
cerrado a A2 §13.17. Relaciona found_type con el mismo argumento, sin anotación
esperada ficticia. Un estado inmutable conserva NX-E0105. No se copia g para
crear un receptor mutable oculto.

Estados: Creado → Ejecutando → Suspendido o Completado; Suspendido →
Ejecutando; Creado/Suspendido/Completado → Destruido mediante drop. En
Completado, Next devuelve None repetidamente y no vuelve a ejecutar cuerpo
ni limpieza. Ejecutando no admite reentrada ni transferencia; una transición
imposible es fallo interno, no un nuevo error fuente. Mover Creado/Suspendido
preserva continuación, identidades y permisos internos.

Yield evalúa su operando en contexto propietario, lo prepara fuera del estado,
desarma los slots transferidos, conserva los demás vivos y devuelve Some al
llamador. Al siguiente avance continúa después del yield, sin reevaluar el
operando ni reconstruir los locales. Yield no sale de ámbitos activos: sus
recursos sobreviven hasta que el control los abandona o se descarta el estado.
No hay préstamo a un elemento entregado después de devolver Next.

Caída al final o `return;` limpian una vez y producen None. `return E`, incluso
unidad, usa NX-E1324. Yield requiere que el invocable léxico más próximo sea
gen; una closure ordinaria interior no hereda ese permiso (NX-E1213).
`yield` se parsea en cualquier bloque, incluidos los de funciones ordinarias
y closures; fuera de gen se rechaza en types/types.check con NX-E1213
sobre el token yield, como fija G02. La gramática no restringe su reconocimiento
a bloques de generador.
El Item escrito puede ser Result<T,E>, pero no es el resultado final del cuerpo.
`?` directamente en ese cuerpo usa el nuevo NX-E1214: se exige seleccionar
Result y escribir yield Err/Ok y return vacío según la política deseada.
En `1?` dentro de gen, NX-E1214 precede causalmente a NX-E0102 por dominio
Result del mismo operador: no se emiten ambos. Se conservan errores independientes
en el operando. `?` en una closure ordinaria anidada conserva su firma Result.
Yield Err no termina automáticamente la secuencia: el siguiente avance
continúa salvo return escrito. No hay canal oculto de error de Next.

### 5.3. Suspensión y limpieza

Los parámetros retenidos son propietarios; los préstamos locales internos
pueden referir slots del mismo estado mediante rutas lógicas estables.
No hay punteros físicos autocontenidos que mover invalide. Se deriva
SuspendSafe por cada valor/permiso vivo en yield. Escalares, arrays de datos,
Option/Result de esos datos y ambos recursos mock tienen evidencia compatible;
los mocks declaran esa propiedad en el manifiesto del modelo. Un préstamo de
lectura interno exige origen vivo y estable en el estado. Un permiso edit
retenido, incluso dentro de Some/closure, es incompatible (NX-E1204).
Desconocido tampoco prueba SuspendSafe. No se relaja la regla por tratarse
de un generador síncrono. Un préstamo terminado antes del yield no cuenta;
uno sin llamadas futuras pero cuyo portador sigue vivo sí cuenta.

Para una closure viva, SuspendSafe se deriva de todos sus campos propietarios
y permisos retenidos, no de su resumen call: todos compatibles implica
compatible; cualquier incompatible domina; en ausencia de incompatibles,
algún desconocido implica unknown. Un entorno vacío es compatible. Para un
estado gen vivo, se aplica la misma conjunción a parámetros, locales,
temporales e iteradores ocultos vivos de cada continuación posible: Creado
incluye parámetros; Suspendido incluye sus slots vivos; Completado es vacío.
Si el análisis no distingue continuaciones, une sus obligaciones, sin borrar
permisos por estar envueltos en otro estado. Un for sobre un gen interno
mantiene vivo ese iterador y los valores de la iteración al alcanzar un yield
externo. Los préstamos read deben conservar origen vivo y estable en el estado
contenedor; edit es incompatible también a través del iterador. El mismo motor
de propiedad deriva estas obligaciones transitivas sobre el grafo acíclico de
layout; no usa una marca genéricamente compatible para closures o generadores.
Para NX-E1204 se elige el primer campo incompatible por orden de slots/capturas;
si solo hay unknown, el primero desconocido. La cadena continúa por la
adquisición que demuestra el permiso incompatible, como G05.

Descarte Creado limpia parámetros propietarios sin ejecutar locales del cuerpo.
Descarte Suspendido limpia locales/temporales vivos por ámbitos inversos y
después parámetros; nunca limpia el Item ya entregado. Fin normal hace esa
limpieza y deja estado vacío Completado. Err del consumidor con `?`, return,
break y fallo R destruyen el iterador si el consumidor lo posee; consumir
un elemento Err no altera por sí mismo un generador que siga poseído fuera.

Un fallo experimental durante Next aborta la ejecución del simulador con su
primer R, limpia el avance y el estado completo y después los propietarios
del llamador según A2. No permite reanudar el generador fallado. No se promete
limpieza ante OOM, panic o aborto del anfitrión. La destrucción es iterativa,
no suspende y no propaga errores tipados. Los efectos de un destructor
bloqueante se conservan: síncrono no implica no_block.

### 5.4. Efectos y for

El estado tiene create/resume/drop; `resume` es el resumen que Next incorpora.
Se usa esa etiqueta existente de WithClause, sin introducir una palabra next
en mapas. Una cota simple de gen limita resume; un mapa admite únicamente
create/resume/drop y exige resume. Las prioridades NX-E0417 son las de D15.
Las operaciones opcionales omitidas de un mapa escrito significan unrestricted;
sin with se infieren. No hay cancel ni copy del estado. Create no ejecuta
resume. Resume incluye cuerpo, Next interiores, llamadas, salidas y cada yield
como suspend, aunque su rama sea constante falsa. Drop incluye la unión de
limpiezas de los estados alcanzables. Se recomprueba tras bajar la máquina de
estados; convertir yield en Return interno no elimina suspend.

`for x in e` evalúa e una vez en contexto propietario, instala un iterador
oculto mutable, llama Next hasta None y mueve cada payload Some al binding
inmutable x de una iteración. Un generador nombrado requiere `move g`; `for
x in g` intenta copia y da NX-E0201. No se retiene un préstamo a g. Al salir
de cada iteración se destruye x si sigue vivo; continue hace esa limpieza y
avanza; break limpia x e iterador y sigue fuera; return/`?` limpian ambos
antes de abandonar el invocable. El siguiente Next ocurre después de limpiar
el elemento anterior y los locales del cuerpo. No se precalculan elementos.

Break/continue pertenecen al bucle léxico más próximo del mismo invocable.
Fuera de bucle usan NX-E1326; no atraviesan una closure. While adquiere esos
bordes sin duplicar su análisis. Los efectos del for incluyen iterando,
Next, cuerpo y limpieza. `for edit` queda diferido; no se finge una vista
mutable sobre un Item propietario. El protocolo nominal de usuario Iterator,
iteradores prestados y adaptadores de colección siguen pendientes de F1/F3.

## 6. Un único motor y contrato de IR

### 6.1. Tipos, flujo y barreras

Se amplía la representación central, no se envuelve el checker con un camino
especial por feature. Un único grafo de restricciones contiene tipos de cuerpos,
capturas, patrones, parámetros, slots y resultados. Never, incógnita y
contaminación permanecen separados. La misma función de transferencia opera
sobre estados reales e hipotéticos con una marca de alcanzabilidad; no hay
checker de código muerto, solver paralelo ni copia de las reglas de propiedad.
El estado hipotético conserva errores independientes sin contaminar las
continuaciones reales; A2 I07c y D15 T01–T07 siguen siendo regresiones.

El CFG de análisis representa explícitamente selección, guardas, adquisiciones,
extracciones, entrega, suspensión/reanudación, break y continue. Yield tiene
borde de reanudación; los caminos de guarda fallida vuelven a la selección
sin transferencias. Resúmenes de invocables incorporan permisos requeridos,
consumos, procedencias de resultados y efectos por operación. Analizar una
closure una vez no autoriza aplicar sus precondiciones a otro entorno de tipo
distinto. Los sitios de llamada conservan su propia evidencia y cuotas.

Se conservan las barreras de A2 §13.9. Forma de captura y resolución son names;
asociación de llamadas es arguments; patrones, cobertura, retorno/yield y
unificación son types; disponibilidad, permisos, escape y SuspendSafe son
ownership, incluidas transferencia, mutación y salidas de guarda; cotas y
efectos de guardas son effects. Un fallo de tipos en un
patrón impide cobertura derivada de ese match. Los conflictos ownership
no se adelantan para ocultar errores de tipos independientes.

### 6.2. Delta de IR e intérprete

TypeId añade Option, Closure con identidad de literal y campos ordenados, y
Generator con identidad de declaración, Item y layout. Option y Result son
instancias del mismo descriptor de unión nominal; Option no tiene un solver,
flujo ni algoritmo de patrones separado.

| TypeId nominal | Variante / tag estable | Payload ordenado | Ruta de inferencia |
|---|---|---|---|
| Option<T> | prelude:None / 0 | Ninguno | — |
| Option<T> | prelude:Some / 1 | T | .some |
| Result<T,E> | prelude:Ok / 0 | T | .ok |
| Result<T,E> | prelude:Err / 1 | E | .err |

La tabla compartida gobierna construcción, cobertura, proyección, extracción,
unificación, copia, drop, igualdad, formato y procedencias. BuildSome/BuildNone
y los constructores históricos Ok/Err son nombres del constructor genérico
BuildVariant(TypeId, tag, payload), no motores diferentes. El wrapper tiene
una celda; el tag no es otra celda. El verificador consulta el descriptor y
exige prueba de variante antes de proyectar. Los órdenes de testigos y de
componentes no resueltos son los publicados, no el orden de interning.

La IR conserva IDs
de función estables y hash por función según AC §9.8; el hash de un cuerpo
generador incluye layout, continuaciones y planes de limpieza. No se implementa
recarga en caliente ni se modifica un estado vivo para otra versión de código.

Se incorporan operaciones lógicas BuildSome/BuildNone, TestVariant,
ProjectPayload, ExtractPayload, BuildClosure, CallClosure, BuildGenerator,
ResumeGenerator y los terminadores Yield/GeneratorDone. El match baja a
TestVariant y BranchBool/Jump; for a un único bucle de ResumeGenerator y
selección de Option. No existe un segundo intérprete de match o de for.
Una tabla de continuaciones conecta Yield con su siguiente bloque. Capturas
se instalan con CopyValue/TransferValue/BeginLoan existentes; cada campo
incluye origen y flag. Las instrucciones administrativas no inventan ticks.

El entorno/estado es un propietario de slots; los permisos son rutas lógicas
con raíz e instancia de frame. Los LoanId fuente son estáticos; la máquina
los instancia por activación para no confundir dos llamadas al mismo literal.
Cerrar un portador retira exactamente sus obligaciones, incluidas copias de
lectores. Los planes de limpieza incluyen dependencias y flags; no recorren
el AST ni deducen el orden de los nombres. Un solo dispatcher ejecuta IR
verificada, con frames explícitos para llamadas directas, closures y avances.

Además de A2 §9.3, el verificador exige:

| Invariante | Rechazo interno obligatorio ante mutación |
|---|---|
| Tag y payload coinciden con el tipo | Some sin payload, None con payload o extracción de variante no demostrada |
| Toda ruta de selección está cubierta | Borde sin destino, guarda false que salta la selección o payload movido antes de guarda |
| Captura tiene modo, origen y único dueño/permiso | Campo sin inicialización, copia de edit, préstamo sin dependencia, flag sin desarmar |
| CallClosure respeta identidad, firma y modo | Receptor consumido reutilizado, llamada read a CallOnce o resumen ajeno |
| Yield publica exactamente un Item propietario | Payload prestado externo, slot simultáneamente vivo en frame y resultado |
| Estado suspendido conserva continuación y obligaciones | Reanudación inexistente, local vivo ausente o permiso incompatible |
| Limpieza única en todos los bordes | Drop de Item entregado, doble drop del estado, omisión por break/Err/R |
| Efectos cubren operaciones latentes y ejecutadas | Create que ejecuta cuerpo, Resume sin suspend de yield, drop bloqueante omitido |
| Layout y recorridos acotados | Ciclo inline, IDs inválidos, presupuesto incompleto o recursión del anfitrión por profundidad de valores |

No se ejecuta una IR inválida y no se convierte su rechazo en NX-E de usuario.
Las mutaciones internas del verificador se prueban separadas de negativos
fuente. El verificador rederiva obligaciones desde operaciones; no acepta como
prueba el booleano «verificado» emitido por el mismo lowering que se está probando.

## 7. Perfil de cuotas y contabilidad

### 7.1. Relación con A2

`f0-0003-b-v1` hereda los límites y unidades de `f0-0002-v2`, con su superficie
`f0-0002-b-v2` de D15 §6.1. No vuelve a v1 de A2 ni elimina el límite global
de propiedad. Los programas del dominio anterior conservan sus ticks, efectos,
observaciones y errores, excepto profile_id, effect_model y las fronteras levantadas.
El modelo ampliado de efectos se identifica `f0-0003-sim-v1`; el namespace
runtime sigue `F0-0002-sim` para no renombrar R históricos.

| Presupuesto | Límite y delta de dominio |
|---|---|
| source_bytes | 1.048.576; mismos bytes originales |
| syntax_nesting | 128; añade entrada Pattern por cada patrón/agrupación anidada, además de Block/Expr/Type |
| expression_tree_depth | 128; literal closure es átomo de altura 1 en el contenedor; cada expresión de su cuerpo se mide por separado; match suma 1 al máximo de scrutinee/guardas/expresiones de brazos |
| array_type_depth | 128; igual a A2 |
| value_type_depth | 128; añade envoltorios Option, Closure y Generator y recorre campos/layout, después de rechazar ciclos |
| function_count | 1.024 invocables fuente, incluidos gen y literales closure; un literal cuenta una vez aunque se evalúe repetidamente |
| type_solver_steps | 1.000.000 globales; también firmas anónimas, propiedades estructurales y residuos de patrones |
| ownership_flow_steps | 1.000.000 por invocable, 4.000.000 globales; las mismas unidades dispersas, con hechos de campos/permisos y bordes nuevos |
| effect_fixpoint_steps | 1.000.000 globales; cada operación de resumen y sitio de llamada, no solo pares de funciones deduplicados |
| ir_items | 262.144 globales; cada campo, entrada de continuación, patrón normalizado, residuo y dependencia de limpieza cuenta como registro lógico adicional |
| Trabajo runtime | 1.000.000; nuevos cargos de §7.2 |
| call_frames | 256 activos, incluido avance; un estado suspendido no es un frame activo |
| live_value_cells | 65.536; también cabeceras y celdas retenidas de closures/generadores |
| observation_bytes | 65.536 emitidos más reservas; modelo exacto de A2 §12.4 |

Los invocables se ordenan por offset de su nombre (gen/fn) o token fn (closure).
Análisis y recomprobaciones comparten contadores. El límite local de una closure
no reinicia el global ni consume el saldo local del padre; el análisis del sitio
de construcción sí carga al padre. Su `analysis_function` es Entity function
con ID `function:<offset-fn>` para el literal. Se conserva la continuación
después de un límite local y la detención global de I10a. Sin convergencia no
hay artefacto ejecutable. No se duplican cuotas por resumen create/resume/drop.

Los ocho perfiles reducidos se llaman `f0-0003-b-test-types-32-v1`,
`f0-0003-b-test-flow-32-v1`, `f0-0003-b-test-flow-global-32-v1`,
`f0-0003-b-test-effects-32-v1`, `f0-0003-b-test-ir-32-v1`,
`f0-0003-b-test-frames-8-v1`, `f0-0003-b-test-cells-64-v1` y
`f0-0003-b-test-observations-64-v1`; cada uno cambia exclusivamente la cuota
indicada igual que D15. No hay selector de sintaxis ni cuotas arbitrarias.
Los cuatro presupuestos costosos conservan la excepción de span de A2 §13.13:
válido, no vacío y asociado al cargo; no se fija un offset portable ficticio.
Para function_count excedido por literal, el ancla exacta es su token fn.

### 7.2. Nuevos cargos y ocupaciones

Todos los cargos son anteriores al compromiso y heredan prioridades de A2:
trabajo, frames, celdas, observaciones. Reservas rechazadas no se comprometen.
La limpieza cuesta cero ticks y no puede interrumpirse por cuotas. Los
argumentos/capturas ya adquiridos se limpian si falla una reserva posterior.

| Operación nueva | Trabajo adicional a hijos y operaciones heredadas |
|---|---|
| Some(e) | Llamada 1, nombre Some 1, entrada 1 como Ok; un tick de construcción de celda del wrapper con tag después del payload |
| None | Un tick de expresión y uno de celda del wrapper con tag; no llamada sintética |
| match | Un tick en match; cada nodo de patrón probado cobra uno antes de inspeccionarlo, incluidos `_` y binding; solo hijos efectivamente probados, en preorden |
| Guarda | Expresión ordinaria una vez tras casar; ningún tick de if sintético |
| Binding/extracción de patrón | Transferencia y préstamo cero; copia posterior del binding usa cargos ordinarios |
| Literal closure | Un tick en fn, uno por cabecera, luego capturas en orden; copy cobra sus celdas, view/edit una celda de permiso cada una, move cero |
| CallClosure | Llamada 1, selección del callee 1, entrada 1; resolver el receptor Place no añade lectura ni copia del entorno; prefijo escrito edit/move añade 1 |
| Crear generador | Llamada 1, nombre 1, entrada 1 de A2, un tick por cabecera de estado y por copia escalar view; move no recorre payload; no entrada al cuerpo |
| Next | Llamada 1, callee nominal Iterator.Next 1 (ruta completa: una identidad, sin cargo adicional de selector), entrada 1; edit del argumento 1, Place raíz 0; un tick de avance antes de consultar estado, un tick por wrapper Some/None de salida; ejecutar cuerpo cobra sus nodos ordinarios |
| yield | Un tick en yield antes del operando; transferir Item cero; Some pertenece al cargo de Next, no se cobra dos veces |
| Fin de gen | Cero unidad final; produce el None ya cobrado por Next |
| for | Un tick en for, luego iterando; por intento de avance: dos ticks de llamada/entrada, uno de avance y uno de wrapper con tag; un tick para probar Some/None; sin nombre/AST sintéticos cobrables |
| break / continue | Un tick en su palabra; limpieza y saltos cero |

Un entorno cuesta una celda de cabecera más celdas de campos propios y una
por cada permiso capturado. Un estado cuesta una celda de cabecera más valores
vivos en slots; slots vacíos, flags, contador y tablas de layout son metadatos
de IR, no celdas de valor. La cabecera representa todo el control fijo y no
se multiplica al reanudar. Option y Result cuestan uno más payload activo. Copiar un
entorno recorre cabecera y campos en preorden; transferirlo no aumenta ocupación.
Los estados suspendidos siguen ocupando celdas y reservando cierres. Next
prepara su tag de salida antes de limpiar el estado en finalización; se cuenta
la coexistencia y no se adelantan drops para aparentar caber. Los parámetros
move pasan de la zona temporal al estado sin duplicar celdas.

Los cargos nuevos se anclan en el introductor o llamada especificados; una
captura usa su entrada completa, un patrón su nodo completo, Next explícito
su llamada completa y Next sintético el token for. R de operaciones del cuerpo
conserva el span de esa operación. No se añade un R secundario en limpieza.

Frontera fuente aislante de celdas: `gen fn G()->I64{}` seguido de Main con
N bindings `let gK=G();` distintos, y Drop de cada uno antes del cierre.
Con N=63/64/65 bajo cells-64, las construcciones solicitan 63/64/65 celdas;
la última de 65 falla antes de instalarla. Los Drop de los aceptados construyen
su unidad después de liberar su estado, conforme A2. Esta receta no depende
de tamaños Rust ni de cuántos elementos se producirían. Frontera de frames:
cadena de closures locales que llaman funciones ordinarias y Next anidados,
con profundidad activa total 7/8/9; los suspendidos se prueban por separado.
Las fronteras de trabajo se calculan con esta tabla y A2, nunca calibrando
ticks contra la implementación que se valida.

Oráculo mínimo exacto: `gen fn G()->I64{}fn Main(){let g=G();Drop(g);}`
requiere **12 ticks** y pico de una celda: entrada Main 1, binding 1,
G() = llamada 1 + nombre G 1 + entrada 1 + cabecera 1;
Drop(g) = sentencia 1 + llamada 1 + nombre Drop 1 + entrada 1 + unidad 1;
unidad final Main 1. Total: 1 + 1 + 4 + 5 + 1 = 12. No ejecuta el cuerpo G ni emite observaciones. La creación
usa un frame activo de llamada durante instalación y lo retira al devolver
el estado; no conserva ese frame activo mientras está frío o suspendido.

Los siguientes totales aplican A2 §11.6/§11.7 nodo a nodo. Las fuentes son
archivos completos, ASCII, sin LF final; no hay entorno mock ni observaciones.
Los cargos de celdas indicados son trabajo de construcción, no una segunda
reserva de ocupación. Una llamada de closure cobra selección del callee una
vez, incluso con prefijo; Next nominal cobra una vez por toda la ruta.

| Oráculo | Fuente completa | Desglose de ticks | Total |
|---|---|---|---:|
| T01 Some | `fn Main(){let x:Option<I64>=Some(1);}` | Main 1 + binding 1 + llamada/nombre/entrada 3 + literal/celda 2 + tag 1 + unidad final 1 | 9 |
| T02 None | `fn Main(){let x:Option<I64>=None;}` | Main 1 + binding 1 + expresión/tag 2 + unidad final 1 | 5 |
| T03 closure | `fn Main(){let f=fn [](){};f();Drop(f);}` | Main 1 + binding/literal/cabecera 3 + sentencia/llamada/callee/entrada/unidad de f 5 + Drop 5 + unidad Main 1 | 15 |
| T04 Next vacío | `gen fn G()->I64{}fn Main(){var g=G();let n=Iterator.Next(edit g);Drop(n);Drop(g);}` | Main 1 + binding/G 5 + binding/llamada/callee/edit/entrada/avance/tag 7 + Drop(n) 5 + Drop(g) 5 + unidad Main 1 | 24 |
| T05 Next con yield | `gen fn G()->I64{yield 1;}fn Main(){var g=G();let n=Iterator.Next(edit g);Drop(n);Drop(g);}` | T04 24 + yield 1 + literal/celda 2 | 27 |
| T06 for vacío | `gen fn G()->I64{}fn Main(){for x in G(){}}` | Main 1 + for 1 + G 4 + intento (llamada/entrada/avance/tag/prueba) 5 + unidad Main 1 | 12 |
| T07 for agotado | `gen fn G()->I64{yield 1;}fn Main(){for x in G(){}}` | Main 1 + for 1 + G 4 + dos intentos 10 + yield/literal/celda 3 + unidad Main 1 | 20 |
| T08 for break | `gen fn G()->I64{yield 1;}fn Main(){for x in G(){break;}}` | Main 1 + for 1 + G 4 + intento 5 + yield/literal/celda 3 + break 1 + unidad Main 1 | 16 |
| T09 for continue | `gen fn G()->I64{yield 1;}fn Main(){for x in G(){continue;}}` | T07 20 + continue 1 | 21 |
| T10 view escalar | `gen fn G(n:view I64)->I64{}fn Main(){let g=G(1);Drop(g);}` | Oráculo mínimo 12 + literal/celda del argumento 2 + copia escalar retenida 1 | 15 |
| T11 captura copy | `fn Main(){let x=1;let f=fn [copy x](){};Drop(f);}` | Main 1 + binding/literal/celda de x 3 + binding/literal/cabecera/copia de x 4 + Drop 5 + unidad Main 1 | 14 |
| T12 captura view | `fn Main(){let x=1;let f=fn [view x](){};Drop(f);}` | Main 1 + x 3 + binding/literal/cabecera/permiso 4 + Drop 5 + unidad Main 1 | 14 |
| T13 receptor consumidor | `fn Main(){let f=fn [](){};move f();}` | Main 1 + binding/literal/cabecera 3 + sentencia/prefijo/llamada/callee/entrada/unidad 6 + unidad Main 1 | 11 |
| T14 Next completado | `gen fn G()->I64{}fn Main(){var g=G();let a=Iterator.Next(edit g);Drop(a);let b=Iterator.Next(edit g);Drop(b);Drop(g);}` | T04 24 + segundo binding/Next 7 + Drop(b) 5; segundo avance no ejecuta cuerpo ni limpieza adicional | 36 |

Un intento sintético de for cuesta 5, sin cargo por callee, prefijo edit ni
binding sintéticos. Un Next explícito cuesta 6 antes del cuerpo (llamada,
callee, edit, entrada, avance y tag); el binding fuente añade 1. No se cobran
además las operaciones AST que el lowering use para representarlos. Una captura
es una entrada declarativa: copy/view/edit cobran las celdas indicadas; no
hay nodo de expresión de nombre ni prefijo de expresión sintético. Move de
captura cuesta cero además del literal/cabecera. Estas reglas fijan J14.

Fuentes literales de frames-8, con profundidad máxima incluyendo Main. Las
primitivas no añaden frames; Next ocupa el frame del avance, no dos frames
para intrínseco y cuerpo. La creación fría ocupa uno transitorio ya retirado.

| Profundidad | Fuente completa | Esperado |
|---|---|---|
| 7 | `fn F1(){F2();}fn F2(){F3();}fn F3(){F4();}fn F4(){}gen fn G()->I64{F1();}fn Main(){let g=G();let f=fn [move g](){var h=move g;let x=Iterator.Next(edit h);};move f();}` | Sin diagnósticos; unit, exit 0, observaciones [], reservas finales 0. Main, closure, avance G, F1, F2, F3, F4. |
| 8 | `fn F1(){F2();}fn F2(){F3();}fn F3(){F4();}fn F4(){F5();}fn F5(){}gen fn G()->I64{F1();}fn Main(){let g=G();let f=fn [move g](){var h=move g;let x=Iterator.Next(edit h);};move f();}` | Igual, con F5 como octavo frame. |
| 9 | `fn F1(){F2();}fn F2(){F3();}fn F3(){F4();}fn F4(){F5();}fn F5(){F6();}fn F6(){}gen fn G()->I64{F1();}fn Main(){let g=G();let f=fn [move g](){var h=move g;let x=Iterator.Next(edit h);};move f();}` | Único R0002/runtime.limit en llamada F6() [64,68), budget_kind=call_frames, limit=8, observed=9; exit 1, observaciones [], reservas finales 0. Otros campos según A2. |

## 8. Diagnósticos y extensiones del esquema

### 8.1. Reglas comunes

Se incorpora íntegramente A2 §§13.6–13.17 con las sustituciones B de D15 §6.
schema_version permanece 1: las extensiones siguientes registran claves dentro
de Data y vocabularios, no eliminan campos de Diagnostic. El perfil nuevo
distingue las expectativas nuevas. Todo NX-E usa Nexo-proposed-v1, severity
error, rule y explanation_id iguales a code/subcause, o code cuando subcause
es null. Message es presentación traducible, salvo el literal de límite.
Todos los campos de Diagnostic siguen siendo obligatorios.

Para todos los negativos semánticos de §9: feature_id null salvo las filas
NX-E0006, que llevan el feature_id especificado;
unexpected_token, expected_tokens_kind, parser_context y recognized_prefix
son null; expected_tokens y repair son []; no se fabrican tokens sintácticos
para errores semánticos. RelatedSpan y Cause usan los objetos y orden de A2.
Las listas no exigidas son []; Data contiene exactamente las claves pertinentes,
sin callee, campos de permiso ni expected_type ajenos a la regla. Cada negativo
aislado produce un solo diagnóstico, check/run exit 1, sin observaciones.
Cada positivo produce cero diagnósticos en check, exit 0, una vez completado J14.
Antes de J14, los positivos J02–J13 se validan por el arnés interno hasta
su barrera, con expectativa CLI NX-E0006/implementation_limit, pending_scope
J14 en lower/ir.lower; no acreditan todavía éxito público de check. La cardinalidad de
programas con varios defectos conserva las barreras de A2, no una regla global
de «un error por archivo».

Las filas de alcance conservadas de D15, incluido unsupported_type y su
type_spelling, siguen aplicándose conforme a la tabla cerrada de §2.2.
Para implementation_limit, §2 sustituye explícitamente etapa/ancla de D15
§7.1: operation es syntax.parse, call.bind, types.check,
ownership.check, effects.check o ir.lower según la etapa indicada; en caso
de etapa names es names.resolve. Ownership.check, effects.check e ir.lower
son adiciones explícitas al vocabulario; los valores serializados van en
minúsculas como en esta enumeración. El mensaje conserva
`Capacidad pendiente de implementación en {pending_scope}.`; data contiene
solo pending_scope. Empates de sitio se resuelven por J numérico ascendente.
La ampliación de etapas se registra en §10; no se aplica retrospectivamente
a los perfiles de A2. Una solicitud de perfil CLI desconocido sigue dando
error de configuración exit 2, nunca NX-E0006.

### 8.2. Códigos nuevos y códigos activados

Se proponen exactamente tres códigos nuevos, sin reutilizar códigos retirados.
El catálogo, explain y el serializador deberán incorporar estas filas juntos.

| Código/subcause | Regla y ancla | stage / operation | Data exacto |
|---|---|---|---|
| NX-E1214 / null | Propagación `?` cuyo invocable inmediato es gen; token `?` | types / types.check | {} |
| NX-E1325 / constructor_arity | Constructor de patrón de unión con aridad incorrecta; patrón completo | types / types.check | constructor:String, expected_arity:UInt, found_arity:UInt |
| NX-E1325 / constructor_type | Constructor de otra unión; patrón completo | types / types.check | constructor:String, scrutinee_type:String |
| NX-E1326 / break_outside_loop | break sin bucle del invocable inmediato; token break | types / types.check | {} |
| NX-E1326 / continue_outside_loop | continue sin ese bucle; token continue | types / types.check | {} |

Constructor se serializa con su identidad de preámbulo `prelude:Some`,
`prelude:None`, `prelude:Ok` o `prelude:Err`. Constructor_type precede a
constructor_arity si ambas fallan; no se tipan los bindings de un patrón
inválido para producir derivados. Las tres reglas nuevas llevan related_spans
y cause_chain vacíos. Su mensaje canónico inicial es respectivamente:
`El cuerpo generador no propaga con ?.`; `Aridad de patrón incorrecta.`;
`Constructor incompatible con el tipo seleccionado.`;
`break requiere un bucle en este invocable.`;
`continue requiere un bucle en este invocable.`

| Código activado | Etapa / operación | Data y relaciones añadidas al perfil |
|---|---|---|
| NX-E1101 | names / names.resolve | {}; declaration sobre el libre ausente |
| NX-E1102 | names / names.resolve | {}; previous_declaration sobre nombre de la primera captura |
| NX-E1104 | ownership / ownership.check | {}; origin sobre captura que retiene; declaration sobre raíz; exit sobre cierre del ámbito que acaba antes que el destino |
| NX-E1107 | ownership / ownership.check | required_mode y requested_mode: read/inout/consume; declaration sobre binding receptor |
| NX-E1110 | ownership / ownership.check | {}; declaration sobre nombre del parámetro retenido |
| NX-E1111 | types / types.check | {}; declaration sobre parámetro no determinado; para resultado, context sobre token fn |
| NX-E1204 | ownership / ownership.check | property_evidence: incompatible/unknown; suspension_kind: yield; origin sobre adquisición incompatible; declaration sobre portador |
| NX-E1213 | types / types.check | {}; sin relaciones |
| NX-E0417/missing_body_operation | parse / syntax.parse | Mapa no vacío con operaciones admisibles pero sin call en closure síncrona o sin resume en gen; primario desde la llave de apertura hasta la de cierre del mapa, ambas incluidas. Data {}, related_spans=[], cause_chain=[]; no relación inventada a una operación ausente. |
| NX-E0111/argument_mode_mismatch en Next | arguments / call.bind | Argumento completo sin edit o con move; Data {}, related_spans=[], cause_chain=[] porque el parámetro intrínseco no tiene declaración fuente. |
| NX-E1304/non_exhaustive | types / types.check | missing_pattern:String, testigo canónico de §3; sin relaciones |
| NX-E1304/unreachable_arm | types / types.check | {}; context sobre cada patrón anterior sin guarda necesario para cubrir el denunciado |
| NX-E1305/guard_transfer, guard_mutation o guard_control | ownership / ownership.check | {}; sin relaciones |
| NX-E1305/guard_effect | effects / effects.check | effect_model, effect_evidence, required_effect; sin contract_origin porque la prohibición es intrínseca a la guarda |
| NX-E1324 | types / types.check | {}; expected_type sobre la cláusula completa `-> T` de Item si está escrita; sin flecha, relaciones [] |

La subcausa guard_control amplía NX-E1305; no añade un código ni reemplaza
las tres subcausas existentes. Missing_body_operation conserva las prioridades
D15 §3.3: mapa vacío, repetición, mezcla y operación inadmisible se resuelven
antes; no se emite la ausencia derivada del mismo mapa inválido.

Los identificadores names.resolve, ownership.check y effects.check se añaden
al vocabulario Diagnostic.operation si no estaban ya presentes; no sustituyen
operaciones heredadas para códigos anteriores. NX-E1304 primario: token match
si falta cobertura, patrón completo del primer brazo inalcanzable en otro caso.
NX-E1305 primario: token de salida para guard_control; prefijo move/edit
que solicita la acción (destino Drop si el consumo no lleva move), o destino de
llamada que aporta efecto, o expresión constructora/copiante si no es llamada.
NX-E1101 señala el primer uso libre; NX-E1102 el segundo nombre capturado;
NX-E1104 el valor entregado/instalado que escapa; NX-E1107 el nombre receptor;
NX-E1111 el primer parámetro indeterminado, o fn si solo falta resultado;
NX-E1110 el modo; NX-E1204 el token yield; NX-E1324 el token return.

Una ampliación de NX-E1105/permission_escalation distingue el origen del
permiso: para un parámetro conserva exactamente A2; para captura o patrón
reemplaza únicamente la clave parameter por permission_source, Entity de
loan. Origin sigue siendo la raíz. Declaration señala el nombre de la raíz;
origin adicional señala captura o scrutinee prestado. No coexisten parameter
y permission_source. `borrow_inout` significa solicitar un préstamo exclusivo
mediante argumento edit, captura edit o binding de patrón edit; conserva el
nombre serializado de A2 y no implica que se haya creado el préstamo rechazado.
No significa mutación directa, que sigue siendo write. Las demás subcausas
conservan sus campos. Un préstamo
de captura usa loan:<inicio-entrada>:0; uno de selección usa
loan:<inicio-scrutinee-sin-modo>:0. La captura implícita usa el primer uso libre.
La cadena usa borrow.read/borrow.inout con esa adquisición completa. Para
overlapping_mutable_borrows se mantiene la prioridad y formato de A2.

Las cadenas nuevas reutilizan kind de A2. Cause.operation añade
closure.create, closure.call, generator.create, generator.next,
generator.yield y property.suspend_safe. Las primeras cuatro tienen kind
call; generator.yield y property.suspend_safe tienen kind effect. Los pasos
de captura usan borrow.read/borrow.inout, value.copy o value.consume;
el traslado de un portador no cambia la identidad del préstamo. Evidencia
es verified, incluido el manifiesto mock, salvo hoja de efecto desconocido
(unknown); no se emite external_assumption sin interfaces externas. Para
NX-E1104 la cadena es la adquisición
borrow que demuestra el escape. Para NX-E1204 es primero property.suspend_safe
en yield con Entity del portador, después la adquisición incompatible. Los
otros nuevos errores directos usan cadena vacía, salvo guard_effect, que usa
el testigo de efectos de A2 sin contrato escrito. Se conserva selección
canónica del testigo más corto y desempate fuente de A2 §13.15.

Diagnostic.operation añade closure.create/closure.call/generator.create/
generator.next/generator.yield para NX-E0412–NX-E0415 cuando el sitio aporta
ese efecto; copia y drop conservan value.copy/value.drop. Next apunta al
nombre Next, no a Iterator ni al argumento. La cadena directa de suspensión
por Next consta de call/generator.next hacia la función gen, seguida de
effect/generator.yield en el yield de esa misma función; ambos verified.
Los cuatro campos effect_model, effect_evidence, contract_origin,
required_effect siguen obligatorios para NX-E0412–NX-E0415. Una creación no
hereda los efectos de resume/call por compartir la misma función fuente.

Option usa escritura canónica `Option<T>`; los anónimos se imprimen en datos
de diagnóstico como `Closure@N` y `Generator@N`, donde N es offset del token
fn del literal o del nombre de gen. No es sintaxis fuente aceptable.
unresolved_components amplía A2 con `.some`; se conserva el preorden de A2,
con `.ok` antes de `.err`, sin ordenación lexicográfica posterior.
Los tipos dentro de Data usan grafías B; ningún campo serializa Never como
si fuera el tipo de un valor inexistente.

Explain conserva el protocolo A2 §13.10: una entrada por regla/subcausa,
ejemplos B pareados de §9 y example_profile_id=f0-0003-b-v1. Las nuevas
reglas se declaran in_profile cuando su J se completa; no se inventan
resultados observados. Los 30 feature_id siguen siendo 30. Los ejemplos de
rechazo global de option, match y loop_control pasan a availability=historical;
closure, generator y receiver_mode conservan ejemplos vigentes de sus
exclusiones parciales: L02, G08 y edit Foo(), respectivamente. L01 sigue vigente
para for_iteration. Los ejemplos globales anteriores de closure/generator son
históricos, sin convertir en historical toda su entrada. El número de códigos aumenta en tres;
no se fija un inventario duplicado del catálogo entero.

## 9. Fixtures normativas pareadas

### 9.1. Fuentes y expectativas exactas

Cada fuente entre backticks es un archivo completo ASCII, sin LF final ni
marcadores. El identificador de archivo de la API es `<memory>`; al ejecutarla
por CLI se sustituye ese identificador en todos los spans por la ruta recibida.
El positivo es un segundo archivo completo, no una sugerencia que deba adivinar
el implementador. Los rangos son bytes UTF-8 [inicio,fin). Las fuentes prueban
una causa: un rechazo léxico, de alcance o sintáctico accidental no satisface
un negativo semántico. Estas son expectativas de arquitectura, no resultados
obtenidos del compilador.

En la tabla de metadatos, `rol:[a,b]` abrevia el objeto RelatedSpan de A2;
los rangos de Cause se expanden igualmente a Span con file `<memory>`.
Entity se escribe completa. Subcause es null si la regla no contiene `/`.
Las etapas/operaciones son §8 y A2 §13.12: en particular NX-E0201 es
ownership/value.copy, NX-E1105 ownership/ownership.access, NX-E0113
types/types.infer, NX-E0414 effects/generator.next. L01 es parse/syntax.parse;
L02 types/types.check. Los campos generales son exactamente §8.1. En Q01,
alloc es el primero de los cuatro efectos desconocidos en el orden
alloc,block,suspend,io: se emite un solo guard_effect por ese sitio.

| ID | Fuente negativa | Regla | Primario |
|---|---|---|---|
| O01 | `fn Main(){let o=None;}` | NX-E0113 | [16,20) |
| O02 | `fn Main()->Result<(),I64>{let o:Option<I64>=Some(1);let x=o?;return Ok(());}` | NX-E0102/type_mismatch | [59,60) |
| M01 | `fn Main(){let o:Option<I64>=Some(1);match o{Some(n)=>{}}}` | NX-E1304/non_exhaustive | [36,41) |
| M02 | `fn Main(){match true{true=>{},false=>{},_=>{}}}` | NX-E1304/unreachable_arm | [40,41) |
| M03 | `fn Main(){let x=match true{true=>{value:1;},false=>{value:false;}};}` | NX-E1303 | [58,63) |
| M04 | `fn Main(){let o:Option<I64>=Some(1);match o{Some(a,b)=>{},None=>{}}}` | NX-E1325/constructor_arity | [44,53) |
| M05 | `fn Main(){let o:Option<I64>=Some(1);match o{Ok(n)=>{},_=>{}}}` | NX-E1325/constructor_type | [44,49) |
| C01 | `fn Main(){let x=1;let f=fn []()->I64{return x;};}` | NX-E1101 | [44,45) |
| C02 | `fn Main(){let x=1;let f=fn [copy x,copy x](){};}` | NX-E1102 | [40,41) |
| C03 | `fn Main(){let r=MockOpen(1);let f=fn [copy r](){};}` | NX-E0201 | [43,44) |
| C04 | `fn Main(){var x=1;let f=fn [view x](){};x=2;Drop(f);}` | NX-E1105/active_borrow_conflict | [40,41) |
| C05 | `fn Main(){var x=1;var f=fn [edit x](){x=2;};f();}` | NX-E1107 | [44,45) |
| C06 | `fn Main(){let f=if true then {let x=1;value:fn [view x]()->I64{return x;};}else{return;};Drop(f);}` | NX-E1104 | [44,73) |
| C07 | `fn Main(){let f=fn [](x:view){return x;};}` | NX-E1111 | [22,23) |
| G01 | `gen fn G()->I64{return 1;}fn Main(){}` | NX-E1324 | [16,22) |
| G02 | `fn Main(){yield 1;}` | NX-E1213 | [10,15) |
| G03 | `gen fn G()->Result<I64,I64>{let x=TryAdd(1,2)?;yield Ok(x);}fn Main(){}` | NX-E1214 | [45,46) |
| G04 | `gen fn G(r:view MockResource)->I64{Inspect(r);yield 1;}fn Main(){}` | NX-E1110 | [11,15) |
| G05 | `gen fn G()->I64{var x=0;var f=fn [edit x](){x=1;};yield 1;edit f();}fn Main(){}` | NX-E1204 | [50,55) |
| G06 | `gen fn G()->I64{yield 1;}fn Main() with no_suspend{var g=G();let x=Iterator.Next(edit g);}` | NX-E0414 | [76,80) |
| F01 | `gen fn G()->I64{yield 1;}fn Main(){let g=G();for x in g{Print(x);}}` | NX-E0201 | [54,55) |
| F02 | `fn Main(){break;}` | NX-E1326/break_outside_loop | [10,15) |
| F03 | `fn Main(){continue;}` | NX-E1326/continue_outside_loop | [10,18) |
| L01 | `gen fn G()->I64{yield 1;}fn Main(){for edit x in G(){}}` | NX-E0006 | [39,43) |
| L02 | `fn Main(){let f=fn [](){};let a=[f];}` | NX-E0006 | [32,35) |
| Q01 | `fn Main(){match true{true if MockUnknown()==()=>{},_=>{}}}` | NX-E1305/guard_effect | [29,40) |
| Q02 | `fn P(x:edit I64)->Bool{return true;}fn Main(){var x=0;match true{true if P(edit x)=>{},_=>{}}}` | NX-E1305/guard_mutation | [75,79) |
| Q03 | `fn P(x:move I64)->Bool{return true;}fn Main(){let x=0;match true{true if P(move x)=>{},_=>{}}}` | NX-E1305/guard_transfer | [75,79) |

| ID | Fuente positiva completa |
|---|---|
| O01 | `fn Main(){let o:Option<I64>=None;}` |
| O02 | `fn Main()->Result<(),I64>{let o:Option<I64>=Some(1);let x=TryAdd(1,2)?;return Ok(());}` |
| M01 | `fn Main(){let o:Option<I64>=Some(1);match o{Some(n)=>{},None=>{}}}` |
| M02 | `fn Main(){match true{true=>{},false=>{}}}` |
| M03 | `fn Main(){let x=match true{true=>{value:1;},false=>{value:2;}};}` |
| M04 | `fn Main(){let o:Option<I64>=Some(1);match o{Some(a)=>{},None=>{}}}` |
| M05 | `fn Main(){let o:Option<I64>=Some(1);match o{Some(n)=>{},_=>{}}}` |
| C01 | `fn Main(){let x=1;let f=fn [copy x]()->I64{return x;};}` |
| C02 | `fn Main(){let x=1;let f=fn [copy x](){};}` |
| C03 | `fn Main(){let r=MockOpen(1);let f=fn [move r](){};}` |
| C04 | `fn Main(){var x=1;let f=fn [view x](){};Drop(f);x=2;}` |
| C05 | `fn Main(){var x=1;var f=fn [edit x](){x=2;};edit f();}` |
| C06 | `fn Main(){let f=if true then {let x=1;value:fn [copy x]()->I64{return x;};}else{return;};Drop(f);}` |
| C07 | `fn Main(){let f=fn [](x:view I64){return x;};}` |
| G01 | `gen fn G()->I64{return;}fn Main(){}` |
| G02 | `gen fn G()->I64{yield 1;}fn Main(){}` |
| G03 | `gen fn G()->Result<I64,I64>{yield TryAdd(1,2);}fn Main(){}` |
| G04 | `gen fn G(r:move MockResource)->I64{Inspect(r);yield 1;}fn Main(){}` |
| G05 | `gen fn G()->I64{var x=0;var f=fn [edit x](){x=1;};edit f();Drop(f);yield 1;}fn Main(){}` |
| G06 | `gen fn G()->I64{yield 1;}fn Main(){var g=G();let x=Iterator.Next(edit g);}` |
| F01 | `gen fn G()->I64{yield 1;}fn Main(){let g=G();for x in move g{Print(x);}}` |
| F02 | `fn Main(){while true{break;}}` |
| F03 | `fn Main(){while false{continue;}}` |
| L01 | `gen fn G()->I64{yield 1;}fn Main(){for x in G(){}}` |
| L02 | `fn Main(){let f=fn [](){};let a=Some(f);}` |
| Q01 | `fn Main(){match true{true if true=>{},_=>{}}}` |
| Q02 | `fn P(x:edit I64)->Bool{return true;}fn Main(){var x=0;match true{true if true=>{},_=>{}}}` |
| Q03 | `fn P(x:move I64)->Bool{return true;}fn Main(){let x=0;match true{true if true=>{},_=>{}}}` |

| ID | Data exacto | Related spans | Cause chain | feature_id |
|---|---|---|---|---|
| O01 | `{"unresolved_components":["$.some"]}` | `[]` | `[]` | `null` |
| O02 | `{"expected_type":"domain:result","found_type":"Option<I64>"}` | `[["found_type",[58,59]]]` | `[]` | `null` |
| M01 | `{"missing_pattern":"None"}` | `[]` | `[]` | `null` |
| M02 | `{}` | `[["context",[21,25]],["context",[30,35]]]` | `[]` | `null` |
| M03 | `{"expected_type":"I64","found_type":"Bool"}` | `[["expected_type",[40,41]],["found_type",[58,63]]]` | `[]` | `null` |
| M04 | `{"constructor":"prelude:Some","expected_arity":1,"found_arity":2}` | `[]` | `[]` | `null` |
| M05 | `{"constructor":"prelude:Ok","scrutinee_type":"Option<I64>"}` | `[]` | `[]` | `null` |
| C01 | `{}` | `[["declaration",[14,15]]]` | `[]` | `null` |
| C02 | `{}` | `[["previous_declaration",[33,34]]]` | `[]` | `null` |
| C03 | `{}` | `[["declaration",[14,15]]]` | `[]` | `null` |
| C04 | `{"origin":{"kind":"binding","id":"binding:14"},"binding":{"kind":"binding","id":"binding:14"},"loan":{"kind":"loan","id":"loan:28:0"},"active_permission":"read","requested_access":"write"}` | `[["declaration",[14,15]],["origin",[28,34]]]` | `[{"kind":"borrow","operation":"borrow.read","span":[28,34],"entity":{"kind":"loan","id":"loan:28:0"},"evidence":"verified"}]` | `null` |
| C05 | `{"required_mode":"inout","requested_mode":"read"}` | `[["declaration",[22,23]]]` | `[]` | `null` |
| C06 | `{}` | `[["declaration",[34,35]],["origin",[48,54]],["exit",[74,75]]]` | `[{"kind":"borrow","operation":"borrow.read","span":[48,54],"entity":{"kind":"loan","id":"loan:48:0"},"evidence":"verified"}]` | `null` |
| C07 | `{}` | `[["declaration",[22,23]]]` | `[]` | `null` |
| G01 | `{}` | `[["expected_type",[10,15]]]` | `[]` | `null` |
| G02 | `{}` | `[]` | `[]` | `null` |
| G03 | `{}` | `[]` | `[]` | `null` |
| G04 | `{}` | `[["declaration",[9,10]]]` | `[]` | `null` |
| G05 | `{"property_evidence":"incompatible","suspension_kind":"yield"}` | `[["declaration",[28,29]],["origin",[34,40]]]` | `[{"kind":"effect","operation":"property.suspend_safe","span":[50,55],"entity":{"kind":"binding","id":"binding:28"},"evidence":"verified"},{"kind":"borrow","operation":"borrow.inout","span":[34,40],"entity":{"kind":"loan","id":"loan:34:0"},"evidence":"verified"}]` | `null` |
| G06 | `{"effect_model":"f0-0003-sim-v1","effect_evidence":"known_effect","required_effect":"suspend","contract_origin":{"file":"<memory>","start":40,"end":50}}` | `[["contract",[40,50]]]` | `[{"kind":"call","operation":"generator.next","span":[76,80],"entity":{"kind":"function","id":"function:7"},"evidence":"verified"},{"kind":"effect","operation":"generator.yield","span":[16,21],"entity":{"kind":"function","id":"function:7"},"evidence":"verified"}]` | `null` |
| F01 | `{}` | `[["declaration",[39,40]]]` | `[]` | `null` |
| F02 | `{}` | `[]` | `[]` | `null` |
| F03 | `{}` | `[]` | `[]` | `null` |
| L01 | `{"pending_scope":"later_adr"}` | `[]` | `[]` | `"for_iteration"` |
| L02 | `{"pending_scope":"later_adr"}` | `[]` | `[]` | `"closure"` |
| Q01 | `{"effect_model":"f0-0003-sim-v1","effect_evidence":"unknown_effect","required_effect":"alloc"}` | `[]` | `[{"kind":"call","operation":"intrinsic.call","span":[29,40],"entity":{"kind":"intrinsic","id":"prelude:mock_unknown"},"evidence":"unknown"}]` | `null` |
| Q02 | `{}` | `[]` | `[]` | `null` |
| Q03 | `{}` | `[]` | `[]` | `null` |

Filas adicionales de permiso, consumo y avance. M06 tiene operation
ownership.access; C08 ownership.use; C09 ownership.check; N01 types.check;
G07 generator.yield. Sus etapas se mantienen en la tabla de reglas.
Para M06 el primario es la entrada de binding de patrón completa. El consumo
de receptor de C08 abarca la llamada prefijada completa, sin terminador.

| ID | Fuente negativa | Regla | Primario |
|---|---|---|---|
| M06 | `fn Main(){var o:Option<I64>=Some(1);match o{Some(edit n)=>{},None=>{}}}` | NX-E1105/permission_escalation | [49,55) |
| C08 | `fn Main(){let f=fn [](){};move f();f();}` | NX-E0202/definitely_consumed | [35,36) |
| C09 | `fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};f();}` | NX-E1107 | [58,59) |
| N01 | `fn Main(){var x=1;let n=Iterator.Next(edit x);}` | NX-E0102/type_mismatch | [38,44) |
| G07 | `gen fn G()->I64 with no_suspend{yield 1;}fn Main(){}` | NX-E0414 | [32,37) |

| ID | Fuente positiva completa |
|---|---|
| M06 | `fn Main(){var o:Option<I64>=Some(1);match edit o{Some(edit n)=>{},None=>{}}}` |
| C08 | `fn Main(){let f=fn [](){};move f();}` |
| C09 | `fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();}` |
| N01 | `gen fn G()->I64{}fn Main(){var x=G();let n=Iterator.Next(edit x);}` |
| G07 | `gen fn G()->I64{yield 1;}fn Main(){}` |

| ID | Data exacto | Related spans | Cause chain |
|---|---|---|---|
| M06 | `{"origin":{"kind":"binding","id":"binding:14"},"permission_source":{"kind":"loan","id":"loan:42:0"},"active_permission":"read","requested_access":"borrow_inout"}` | `[["declaration",[14,15]],["origin",[42,43]]]` | `[{"kind":"borrow","operation":"borrow.read","span":[42,43],"entity":{"kind":"loan","id":"loan:42:0"},"evidence":"verified"}]` |
| C08 | `{}` | `[["declaration",[14,15]],["consumption",[26,34]]]` | `[{"kind":"consume","operation":"value.consume","span":[26,34],"entity":{"kind":"binding","id":"binding:14"},"evidence":"verified"}]` |
| C09 | `{"required_mode":"consume","requested_mode":"read"}` | `[["declaration",[32,33]]]` | `[]` |
| N01 | `{"expected_type":"domain:generator","found_type":"I64"}` | `[["found_type",[38,44]]]` | `[]` |
| G07 | `{"effect_model":"f0-0003-sim-v1","effect_evidence":"known_effect","required_effect":"suspend","contract_origin":{"file":"<memory>","start":21,"end":31}}` | `[["contract",[21,31]]]` | `[{"kind":"effect","operation":"generator.yield","span":[32,37],"entity":{"kind":"function","id":"function:7"},"evidence":"verified"}]` |

### 9.2. Ejecuciones discriminantes y matrices de regresión

Los IDs canónicos externos llevan prefijo F3-, por ejemplo F3-C04; este
documento permite omitirlo. Las tablas anteriores fijan el núcleo literal;
las familias siguientes son obligaciones adicionales, no reemplazos por demos.
Cada negativo derivado debe publicar antes de ejecutarse su fuente B completa,
positivo, número exacto de errores y objeto esperado, calculando spans desde
bytes de la fuente, nunca copiando el JSON del compilador. Se aplican las
reglas de selección de §8; no basta un assert que busque el código como texto.

| Regla | Variación aislada y positivo discriminante |
|---|---|
| Permiso de patrón | Some(edit n) en match de lectura: NX-E1105/permission_escalation; agregar edit al scrutinee mutable permite escribir; mover desde match edit sigue prohibido. |
| Movimiento por brazo | Consumir r en un brazo y usarlo después da NX-E0202/maybe_consumed; consumir en ambos da definitely_consumed; reinicializar var r en cada continuación permite usarlo. Una rama que retorna no contamina la unión. |
| Guardas antes de extracción | let o=Some(MockOpen(1)); match move o, primer Some(move r) con guarda false y segundo Some(move r) sin guarda: solo el segundo recibe r; inspect y close únicos. Cambiar false a true elige el primero, sin segundo movimiento. |
| Cobertura anidada | Option<Result<Bool,I64>> con None, Some(Ok(true)), Some(Ok(false)), Some(Err(_)); quitar una hoja da su testigo residual exacto; cambiar un literal por _ restaura cobertura. |
| Contexto de tipo | Anotar x:I64 en M03 cambia únicamente código a NX-E0102/type_mismatch y expected_type relacionado a esa anotación; conservar Bool es el negativo, sustituirlo por 2 es el positivo. |
| Selección de recursos | Sobre o:Option<MockResource>, match move o con Some(move r) entrega propiedad; match o con Some(r) solo permite Inspect, no Drop ni move. Consumir la raíz y usarla después debe alcanzar NX-E0202, nunca partial_transfer. |
| Retención | Repetir C04 con captura inferida, Some(f), copia de closure lectora y reasignación de portador. Destruir solo uno de dos lectores sigue rechazando escritura; destruir ambos la permite. |
| Escape | C06 envuelta en Some o Err con tipo opuesto resuelto; NX-E1104 sobre agregado entregado. Una captura copy del dato independiente permite el mismo destino. |
| Consumidor | Closure que Drop(r) exige move f(); llamada consumidora seguida de f() da NX-E0202. Quitar la segunda llamada es positivo; poseer r y solo Inspect(r) permite dos llamadas read. |
| Copia y captura | Copy de Array independiente: modificar el original después no cambia el capturado. Copy de closure con permiso edit da NX-E0201; move de esa closure conserva exclusividad y no copia el permiso. |
| Efectos por operación | Capturar Array por copy en no_alloc da NX-E0412 en create; move evita esa copia. Literal cuyo cuerpo Print no incumple no_io hasta llamarlo; cierre de recurso bloqueante incumple no_block incluso sin llamada. Los contratos copy y drop escritos también se comprueban. |
| Cotas gen | resume(no_suspend) con yield da NX-E0414; gen sin yield satisface la cota. Un mapa con call da NX-E0417/invalid_operation; cambiarlo por resume lo repara. Mapa sin resume da missing_body_operation; no es error derivado de operación inválida. |
| Propiedad del estado | Copiar g da NX-E0201; move g y usar origen da NX-E0202. El positivo usa solo el destino y conserva la continuación. View escalar copia el valor inicial aunque el original var cambie antes de Next. |
| SuspendSafe | G05 con permiso transitivo en Option, closure interior o dos portadores; terminar todos los permisos antes de yield repara. Un permiso read interno válido permite suspensión y traslado del estado. |
| Contexto léxico | Yield dentro de closure ordinaria anidada en gen conserva NX-E1213; mover ese yield al cuerpo gen lo permite. Break dentro de closure anidada en while conserva NX-E1326; un while propio lo permite. |
| Flujos hipotéticos | Añadir return antes de C03 conserva NX-E0201; antes de C01 conserva NX-E1101. A2 TD01–TD07 se amplían con match y closures sin emitir errores de propiedad solo por faltar valor tras Never. |

Fuentes de ejecución con observaciones normativas:

```nexo
fn Main(){let r=MockOpen(7);let f=fn [move r](){Inspect(r);};f();f();Drop(f);}
```

Resultado unit, events exactamente open:0:7, inspect:0:7, inspect:0:7,
close:0:7; stdout []; observaciones en ese orden. Se prueba también sin las
dos llamadas: open y close, sin inspect. Ninguna lectura duplica el recurso.

```nexo
gen fn G(r:move MockResource)->I64{Inspect(r);yield 1;Inspect(r);yield 2;}
fn Main(){var g=G(MockOpen(7));let a=Iterator.Next(edit g);AssertEq(a,Some(1));Drop(g);}
```

Resultado unit; open:0:7, inspect:0:7, close:0:7. Quitar Next y AssertEq
produce solo open/close. Repetir Next hasta cuatro veces produce
Some(1), Some(2), None, None, dos inspect y un close antes de que el tercero
devuelva; Drop final no añade cierre. Mover g después del primer avance y
avanzar el destino produce Some(2), sin reinicio ni pérdida del recurso.

```nexo
gen fn G()->MockResource{yield MockOpen(1);yield MockOpen(2);}
fn Main(){for r in G(){Inspect(r);break;}}
```

Resultado unit; open:0:1, inspect:0:1, close:0:1. No se abre el recurso 2.
Cambiar break por continue produce además open:1:2, inspect:1:2, close:1:2.
Cada Item se cierra antes del siguiente avance; el generador no conserva
un flag vivo del Item entregado. Eliminar Inspect conserva los cierres.

```nexo
gen fn G()->Result<I64,I64>{yield Err(9);yield Ok(2);}
fn Main(){var g=G();let a=Iterator.Next(edit g);let b=Iterator.Next(edit g);AssertEq(a,Some(Err(9)));AssertEq(b,Some(Ok(2)));}
```

Resultado unit sin observaciones: entregar Err no termina automáticamente
G. El consumidor que selecciona Some(move item) y aplica item? dentro de
Main()->Result<(),I64> debe salir con Err(9) y limpiar g; son dos políticas
explícitas distintas y ambas se prueban.

### 9.3. Guardas, ciclos y operaciones: fuentes literales

Cada fila negativa produce un diagnóstico. M07 conserva la limitación de
aislamiento expresa más abajo. Las fuentes conservan las convenciones de §9.1; todos los
positivos son archivos completos y requieren cero diagnósticos tras J14.

| ID | Fuente negativa | Regla | Primario | Fuente positiva |
|---|---|---|---|---|
| Q04 | `fn Take(r:move MockResource)->Bool{Drop(r);return true;}fn Main(){let r=MockOpen(1);match true{true if Take(move r)=>{},_=>{}}Inspect(r);}` | NX-E1305/guard_transfer | [108,112) | `fn Take(r:view MockResource)->Bool{return true;}fn Main(){let r=MockOpen(1);match true{true if Take(r)=>{},_=>{}}Inspect(r);}` |
| Q06 | `fn Main()->Result<(),I64>{match true{true if TryAdd(1,2)?==3=>{},_=>{}}return Ok(());}` | NX-E1305/guard_control | [56,57) | `fn Main()->Result<(),I64>{match true{true if TryAdd(1,2)==Ok(3)=>{},_=>{}}return Ok(());}` |
| Q07 | `fn Main(){match true{true if if true then {return;}else{value:true;}=>{},_=>{}}}` | NX-E1305/guard_control | [43,49) | `fn Main(){match true{true if if true then {value:true;}else{value:true;}=>{},_=>{}}}` |
| G08 | `gen fn G()->I64{let g=G();}fn Main(){}` | NX-E0006/generator | [7,8) | `gen fn G()->I64{let g=H();}gen fn H()->I64{}fn Main(){}` |
| M07 | `fn Main(){let o:Option<I64>=Some(1);match o{Some(x,x)=>{},None=>{}}}` | NX-E0104 | [51,52) | `fn Main(){let o:Option<I64>=Some(1);match o{Some(x)=>{},None=>{}}}` |
| M08 | `fn Main(){let o:Option<I64>=Some(1);match o{Missing(x)=>{},None=>{}}}` | NX-E0101 | [44,51) | `fn Main(){let o:Option<I64>=Some(1);match o{Some(x)=>{},None=>{}}}` |
| M09 | `fn F(b:view Bool){let r=MockOpen(1);match b{true=>{Drop(r);},false=>{}}Inspect(r);}fn Main(){}` | NX-E0202/maybe_consumed | [79,80) | `fn F(b:view Bool){let r=MockOpen(1);match b{true=>{Inspect(r);},false=>{}}Inspect(r);}fn Main(){}` |
| F04 | `gen fn G()->I64{yield 1;}fn Main() with no_suspend{for x in G(){}}` | NX-E0414 | [51,54) | `gen fn G()->I64{}fn Main() with no_suspend{for x in G(){}}` |
| E01 | `fn Main(){let a=[1];let f=fn [copy a]() with {create(no_alloc),call(unrestricted)}{};}` | NX-E0412 | [26,28) | `fn Main(){let a=[1];let f=fn [move a]() with {create(no_alloc),call(unrestricted)}{};}` |
| E02 | `fn Main(){let a=[1];let f=fn [move a]() with {copy(no_alloc),call(unrestricted)}{};}` | NX-E0412 | [26,28) | `fn Main(){let a=1;let f=fn [move a]() with {copy(no_alloc),call(unrestricted)}{};}` |
| E03 | `fn Main(){let r=MockBlockingOpen(1);let f=fn [move r]() with {drop(no_block),call(unrestricted)}{};}` | NX-E0413 | [42,44) | `fn Main(){let r=MockOpen(1);let f=fn [move r]() with {drop(no_block),call(unrestricted)}{};}` |
| E04 | `fn Main(){let r=MockOpen(1);let f=fn [move r]() with {drop(no_io),call(unrestricted)}{};}` | NX-E0415 | [34,36) | `fn Main(){let r=1;let f=fn [move r]() with {drop(no_io),call(unrestricted)}{};}` |
| E05 | `gen fn G()->I64{yield 1;}fn Main(){let g=G();let f=fn [move g]() with no_suspend{var h=move g;let n=Iterator.Next(edit h);};}` | NX-E0414 | [109,113) | `gen fn G()->I64{}fn Main(){let g=G();let f=fn [move g]() with no_suspend{var h=move g;let n=Iterator.Next(edit h);};}` |

Q04/Q06/Q07 tienen Data {}, relaciones [], cadena [], ownership/ownership.check.
Q04 no produce NX-E0202 por Inspect posterior. Q02 aísla guard_mutation con raíz mutable; §3.3 conserva la prioridad causal sobre NX-E0105.
G08 usa types/types.check, subcause null, feature_id=generator,
Data {"pending_scope":"later_adr"}, relaciones y cadena []; el primario es el
nombre de la declaración del primer estado del componente cíclico por offset.
El ciclo G contiene G es comprobable sin inferir una firma de retorno recursiva:
el resultado de llamar a un gen ya es su identidad de estado.
M08 usa names/names.resolve, Data {"use_kind":"call_target"}, relaciones y
cadena []. Constructor de patrón ausente amplía el dominio de call_target.
M07 usa names/names.declare, Data {"declaration_kind":"binding"}, relación
previous_declaration al primer x [49,50), cadena []. En el subset actual las
variantes tienen cero o un payload: ningún patrón válido puede ligar dos nombres
iguales. Some(x,x) comprueba la prioridad de names sobre constructor_arity, no
un negativo aislado de duplicación. La regla aislada requiere el arnés interno
con descriptor binario y patrón de aridad dos, o el dominio futuro de variantes
con varios payloads; no se acredita por esta fuente ni por una sintaxis de tupla
fuera del perfil. La limitación queda explícita en J03/J16. En el arnés de names, la entrada
literal de M07 tiene resultado único NX-E0104 ya especificado; la variante
`fn Main(){let o:Option<I64>=Some(1);match o{Some(x,y)=>{},None=>{}}}`
tiene cero diagnósticos de names y NX-E1325/constructor_arity al continuar
a types. Este par aísla la condición en la barrera names, pero no sustituye
un positivo de check completo ni acredita una variante binaria aceptada.
M09 usa ownership/ownership.use, Data {}, declaration [22,23), consumption
[51,58), cadena única drop/value.drop [51,58), Entity binding:22, verified.

E01–E04 comprueban las cotas del mapa del literal incluso sin ejecutar/copy/drop
explícito. El primario fn es el sitio que expone el resumen de operación bajo
su propia cota; no se atribuye al cuerpo call un efecto de create/copy/drop.
Operaciones diagnósticas: E01 closure.create, E02 value.copy, E03/E04 value.drop.
Data contiene exactamente effect_model=f0-0003-sim-v1, effect_evidence=known_effect,
required_effect=alloc/alloc/block/io respectivamente y contract_origin sobre
el token no_alloc/no_alloc/no_block/no_io. Related_spans contiene solo contract
en ese mismo rango. Para E01/E02 la cadena es una hoja copy/value.copy,
span de la captura completa, Entity binding:14 (origen del campo), verified.
Para E03/E04 es drop/value.drop sobre la captura, Entity binding:14,
seguida de drop/resource.drop con span null y Entity de tipo MockBlockingResource
o MockResource respectivamente; todos verified. El resumen conserva la identidad
del origen del campo aunque se traslade al entorno. No se usa kind effect para
value.copy/value.drop ni se inventa un span del manifiesto. Una cota propia de
operación se diagnostica al declarar el literal; no es una limpieza implícita
del CFG de Main y no lleva relaciones origin/exit de esa limpieza.

| ID | contract_origin / contract | Hoja local (span, Entity) |
|---|---|---|
| E01 | [53,61) | [30,36), binding:14 |
| E02 | [51,59) | [30,36), binding:14 |
| E03 | [67,75) | [46,52), binding:14 |
| E04 | [59,64) | [38,44), binding:14 |

F04 y E05 usan effects/generator.next, Data de suspensión conocida con los
cuatro campos obligatorios; contract_origin/contract señalan no_suspend.
F04 primaria for: el avance implícito conserva cadena call/generator.next en
for y effect/generator.yield en yield, ambas Entity function:7, verified.
F04: contract_origin/contract [40,50), call [51,54), yield [16,21).
E05: contract_origin/contract [70,80), call [109,113), yield [16,21).
E05 usa la misma cadena con Next explícito. No existe una fuente aislada que
cree/copie/destruya una closure y añada suspend en este perfil: esas operaciones
son síncronas y no ejecutan call. E05 demuestra que suspend pertenece a call;
su positivo sin yield y el siguiente positivo separan el cuerpo latente de las
otras tres operaciones, sin exigir un NX-E0414 imposible.

`gen fn G()->I64{yield 1;}fn Main(){let f=fn []() with {create(no_suspend),copy(no_suspend),call(unrestricted),drop(no_suspend)}{var g=G();let n=Iterator.Next(edit g);};let h=f;Drop(h);Drop(f);}`

Formas límite con decisión nominal y sintáctica explícita:

| Fuente completa | Resultado normativo |
|---|---|
| `fn Foo(){}fn Main(){edit Foo();}` | NX-E0006/receiver_mode, names/names.resolve, primario edit [20,24), Data {"pending_scope":"later_adr"}; relaciones y cadena []. |
| `fn Main(){let x:Option<I64>=None();}` | NX-E0111/non_callable_target, types/call.bind, primario None [28,32), Data {}, relaciones y cadena []; sin NX-E0113 derivado. |
| `fn Main(){let x=Some<I64>(1);}` | NX-E0006/traits_generics, parse/syntax.parse, primario `<` [20,21), Data {"pending_scope":"later_adr"}; relaciones y cadena []. |
| `gen fn G()->I64{let x=1?;}fn Main(){}` | Único NX-E1214, types/types.check, primario `?` [23,24), Data {}, relaciones y cadena []; no NX-E0102 derivado. |
| `fn Main(){let r:Result<I64,I64>=Ok(1);match r{None=>{AssertEq(None,r);}}}` | Positivo: None es binding de lectura de tipo Result<I64,I64>; unit, diagnostics=[], observations=[], exit 0. |
| `struct Iterator{}fn Main(){}` | NX-E0006/user_types, parse/syntax.parse, primario struct [0,6), Data {"pending_scope":"later_adr"}; no habilita ni redefine el intrínseco en este perfil. |
| `fn Main(){for x in 1{}}` | NX-E0102/type_mismatch, types/types.check, primario 1 [19,20), Data {"expected_type":"domain:iterable","found_type":"I64"}, related_spans=[["found_type",[19,20]]], cadena []. |

Las filas de alcance usan feature_id igual al identificador indicado tras
NX-E0006, subcause null y los campos sintácticos nulos según A2; no son
NX-E0002. Las restantes usan feature_id null. Eliminar edit en la primera,
los paréntesis de None y `<I64>` de Some en las siguientes, y reemplazar `1?`
por `1` en gen da positivos completos con el resto de cada fuente intacto.
El tipo de usuario continúa fuera del perfil; sustituir su declaración por
`gen fn G()->I64{}` deja `fn Main(){}` positivo sin simular un tipo Iterator.
El positivo de for sobre escalar sustituye su fuente completa por
`gen fn G()->I64{yield 1;}fn Main(){for x in G(){}}`.

### 9.4. Contratos, unidad y testigos canónicos

Estas fuentes usan las convenciones de §9.1: ASCII sin LF final, offsets en
bytes, un único diagnóstico negativo, cero observaciones, check/run exit 1.
Los positivos completos dan check exit 0 y diagnostics=[] tras J14; run produce
unidad, exit 0 y observations=[]. No se infieren expectativas del checker.

| ID | Fuente negativa | Código/subcause | Primario y texto | Fuente positiva |
|---|---|---|---|---|
| W08 | `gen fn G()->I64 with {create(no_io)}{}fn Main(){}` | NX-E0417/missing_body_operation | [21,36), `{create(no_io)}` | `gen fn G()->I64 with {create(no_io),resume(unrestricted)}{}fn Main(){}` |
| W09 | `fn Main(){let f=fn []() with {drop(no_io)}{};}` | NX-E0417/missing_body_operation | [29,42), `{drop(no_io)}` | `fn Main(){let f=fn []() with {drop(no_io),call(unrestricted)}{};}` |
| N02 | `gen fn G()->I64{}fn Main(){var g=G();let n=Iterator.Next(g);}` | NX-E0111/argument_mode_mismatch | [57,58), `g` | `gen fn G()->I64{}fn Main(){var g=G();let n=Iterator.Next(edit g);}` |
| G09 | `gen fn G(){yield 1;}fn Main(){}` | NX-E0102/type_mismatch | [17,18), `1` | `gen fn G(){yield ();}fn Main(){}` |
| G10 | `gen fn G(){return ();}fn Main(){}` | NX-E1324/null | [11,17), `return` | `gen fn G(){return;}fn Main(){}` |
| M10 | `fn Main(){let o:Option<Result<I64,Bool>>=None;match o{None=>{},Some(Ok(_))=>{},Some(Err(true))=>{}}}` | NX-E1304/non_exhaustive | [46,51), `match` | `fn Main(){let o:Option<Result<I64,Bool>>=None;match o{None=>{},Some(Ok(_))=>{},Some(Err(true))=>{},Some(Err(false))=>{}}}` |
| M11 | `fn Main(){match 0{0=>{},1=>{}}}` | NX-E1304/non_exhaustive | [10,15), `match` | `fn Main(){match 0{0=>{},1=>{},_=>{}}}` |

W08/W09: parse/syntax.parse, Data {}, related_spans=[], cause_chain=[].
N02: arguments/call.bind, Data {}, related_spans=[], cause_chain=[].
G09: types/types.check, Data {"expected_type":"()","found_type":"I64"},
related_spans=[["found_type",[17,18]]], cause_chain=[]. G10: types/types.check,
Data {}, related_spans=[], cause_chain=[]. M10/M11: types/types.check,
Data respectivamente {"missing_pattern":"Some(Err(false))"} y
{"missing_pattern":"-1"}; related_spans=[], cause_chain=[]. Todos llevan
feature_id null y los campos generales de §8.1; no se generan tokens de
error sintáctico por el mero hecho de que W08/W09 pertenezcan a parse.

La fuente histórica F2-F27 sin marcadores, `gen fn G(){} fn Main(){}`, es
positiva: check/run exit 0, diagnostics=[], unidad y observations=[]. Se
exigen además estos positivos de la misma regla, sin inferencia de Item:
`gen fn G(){yield ();}fn Main(){var g=G();AssertEq(Iterator.Next(edit g),Some(()));AssertEq(Iterator.Next(edit g),None);}`
y `gen fn G(){}fn Main(){var g=G();AssertEq(Iterator.Next(edit g),None);}`.
G09 también se repara escribiendo `->I64` después de G(); esa variante conserva
el yield no unitario y solo añade el tipo explícito. G10 verifica la ausencia
de relación inventada cuando tampoco hay cláusula Item.

Some acepta `fn Main(){AssertEq(Some(v:1),Some(1));}` con resultado unitario,
sin diagnósticos ni observaciones; la etiqueta no altera el payload ni su cargo.

La derivación transitiva de SuspendSafe tiene el positivo compuesto
`gen fn H()->I64{yield 1;}gen fn G()->I64{let f=fn [](){};for x in H(){yield x;f();}}fn Main(){for x in G(){AssertEq(x,1);}}`:
el entorno vacío y el iterador interno suspendido permanecen vivos en yield,
son compatibles y no producen NX-E1204. G05 conserva el negativo pareado de
closure con permiso edit vivo y su reparación terminándolo antes de yield.
El mismo hecho envuelto en Option/Result no cambia la propiedad; no se usa la
compatibilidad del wrapper para borrar un permiso incompatible.

### 9.5. Resolución nominal, mutabilidad de Next y mapa de gen

Fuentes ASCII sin LF final; campos comunes de §8.1/§9.1. Cada negativo tiene
un diagnóstico, check/run exit 1 y observations=[]. Cada positivo completo
tiene diagnostics=[], check/run exit 0, unidad y observations=[] tras J14.
Cause_chain=[], repair=[] y campos sintácticos nulos en los tres: un contrato
invalid_operation no es NX-E0002. Subcause=null salvo W10 (invalid_operation).

| ID | Fuente negativa | Regla | Primario | Fuente positiva |
|---|---|---|---|---|
| N03 | `gen fn G(){}fn Main(){let Iterator=0;var g=G();let n=Iterator.Next(edit g);}` | NX-E0006 | [61,62) | `gen fn G(){}fn Main(){let other=0;var g=G();let n=Iterator.Next(edit g);}` |
| N04 | `gen fn G(){}fn Main(){let g=G();let n=Iterator.Next(edit g);}` | NX-E0105 | [57,58) | `gen fn G(){}fn Main(){var g=G();let n=Iterator.Next(edit g);}` |
| W10 | `gen fn G() with {call(unrestricted)}{}fn Main(){}` | NX-E0417/invalid_operation | [17,21) | `gen fn G() with {resume(unrestricted)}{}fn Main(){}` |

| ID | stage/operation | Data | related_spans | feature_id |
|---|---|---|---|---|
| N03 | names/names.resolve | `{"pending_scope":"later_adr"}` | `[]` | `methods` |
| N04 | ownership/place.write | `{}` | `[["declaration",[26,27]]]` | `null` |
| W10 | parse/syntax.parse | `{}` | `[["context",[7,8]]]` | `null` |

N03 resuelve el binding Iterator antes del selector; el punto no adquiere la
identidad prelude:Iterator.Next. N04 solicita exclusividad sobre una raíz
inmutable: primario g, sin fabricar préstamo adquirido. W10 rechaza call antes
de la ausencia derivada de resume; context relaciona el nombre declarado G.

## 10. Concreciones y enmiendas propuestas

Estas decisiones solo sustituyen las cláusulas indicadas para el perfil de
este corte. No editan documentos de autoridad ni cambian la base 0.2.

| Fuente y texto exacto que se enmienda | Nueva redacción para este perfil | Filas afectadas |
|---|---|---|
| D15 §3.6: «`pending_scope` es `F0-0003` para `closure/generator/option/match`, `later_adr` para los restantes de alcance; `implementation_limit` usa exclusivamente `I08`, `I09`, `I10a`, `I10b` o `I11a` según §7.1.» | Sustituir por tabla cerrada §2 para f0-0003-b-v1; retirar los rechazos globales incorporados. Se amplían las etapas de implementation_limit de forma expresa, no se encubre un InvalidIr. | §2/J02–J14; L01/L02 |
| A2 §13.12: «Los códigos y subcausas del catálogo no enumerados aquí no se emiten en este perfil.» y «Las subcausas no emitibles de NX-E0417 siguen siendo las de §14.4.»; A2 §14.4: «`missing_body_operation` no tiene una emisión aislada en una función ordinaria de este perfil: mapa vacío e inclusión de otra operación tienen causas prioritarias específicas.» | Enmienda propuesta: la afirmación se conserva para funciones ordinarias; para closure síncrona y gen se añade NX-E0417/missing_body_operation, parse/syntax.parse, mapa completo incluido delimitadores como primario, Data {}, related_spans=[], cause_chain=[]. Se emite solo tras las prioridades D15 §3.3 cuando falte call o resume respectivamente; missing_with_clause y operation_list_required continúan no emitibles en este perfil. | §8.2; W08/W09; D11 C42; J09/J12 |
| D15 §3.1: «Omitir `->` significa unidad en F0; inferir otro resultado sigue diferido con `inferred_non_unit_result` (A2 §§4.1 y 4.4).»; D13 §1.1: «T es **Item**, el tipo de cada elemento» | Enmienda propuesta de concreción: para gen la unidad omitida es Item=(), no el tipo de la llamada; no se infiere Item de yield. Yield no unitario produce NX-E0102/type_mismatch sobre su operando con expected_type=(), found_type concreto y relación found_type en ese operando; no related_span de rol context ni expected_type sin anotación escrita. La inferencia de resultados ordinarios conserva A2. | F2-F27; G09/G10; §2.1/§5.1 |
| D10: «Devuelven un recurso de estado concreto; el cuerpo comienza al avanzar, no al crear el generador.» | Concreción del perfil: omitir Item fija (), pero G() sigue devolviendo estado; Next produce Option<()> y yield () entrega Some(()). La limitación permanente de layout rechaza recursión directa/mutua de estados con NX-E0006/generator, pending_scope=later_adr, types/types.check sobre el primer nombre gen del componente; no añade boxing implícito ni modifica D10. | F2-F27; G08/G09; §5.1 |
| A2 §13.15: «`context`: primer retorno no unitario que exige anotar el resultado, por orden fuente, desde `return` hasta el final del operando.» | Conservar exclusivamente para NX-E0006/inferred_non_unit_result de función ordinaria. Gen sin flecha no emite ese límite ni fabrica un context; return con operando usa NX-E1324 y yield incompatible usa NX-E0102 según §5.1. | G09/G10; F2-F27 |
| D15 §8.7 F2-F08: «`fn Main(){let f=⟦fn⟧[](){return ();};}`», «`closure`», «`[16,18)`» | La fuente completa sin marcadores se acepta: diagnostics=[], check/run exit 0, resultado unidad, observations=[]. La exclusión vigente closure se ejemplifica por L02. | F2-F08; L02 |
| D15 §8.7 F2-F14: «`fn Main(){⟦match⟧ true{true=>{},false=>{}}}`», «`match`», «`[10,15)`» | La fuente completa sin marcadores se acepta: diagnostics=[], check/run exit 0, unidad, observations=[]. El rechazo global es histórico; los errores de cobertura usan NX-E1304. | F2-F14; M01–M03 |
| D15 §8.7 F2-F15: «`fn F(x:view ⟦Option⟧<I64>){} fn Main(){}`», «`option`», «`[12,18)`» | La fuente completa sin marcadores se acepta: diagnostics=[], check/run exit 0, unidad, observations=[]. El rechazo global es histórico; la inferencia incompleta conserva NX-E0113. | F2-F15; O01 |
| D15 §8.7 F2-F27: «`⟦gen⟧ fn G(){} fn Main(){}`», «`generator`», «`[0,3)`» | La fuente completa sin marcadores se acepta con Item=(): diagnostics=[], check/run exit 0, unidad, observations=[]. No NX-E0006/inferred_non_unit_result. La exclusión generator vigente es recursión de estados, G08. | F2-F27; §9.4 |
| D15 §6.1, fila «§13.17 rutas indeterminadas»: «`$.element`, `$.ok`, `$.err` sin cambio; el ejemplo anotado usa `Result<I64,I64>`»; A2 §13.17: «ComponentPath = "$" { ".element" \| ".ok" \| ".err" }» | Enmienda propuesta: ComponentPath = "$" { ".element" \| ".ok" \| ".err" \| ".some" }. Añadir .some por cada payload Option y conservar composición recursiva y preorden, con .ok antes de .err. No ordenar las rutas lexicográficamente ni añadir ruta para None. | O01; §3.1/§6.2/§8.2 |
| A2 §13.15, Diagnostic.operation: «El vocabulario cerrado es la unión de los literales de la tabla de §13.12 y los siguientes cinco valores de efectos:» | Enmienda propuesta: conservar esa unión y añadir ownership.check, effects.check, ir.lower, closure.create, closure.call, generator.create, generator.next y generator.yield; names.resolve se conserva de §13.12. Los tres primeros sirven a las barreras/reglas de §8; los cinco últimos a NX-E0412–NX-E0415 según el sitio causal. property.suspend_safe pertenece únicamente a Cause.operation. No se admiten comodines closure.* o generator.*. | §8.1/§8.2; J02–J14; G06/G07 |
| A2 §13.15, Cause.operation: «Enumeración cerrada y correspondencia:» | Enmienda propuesta: añadir closure.create, closure.call, generator.create, generator.next con kind=call; generator.yield y property.suspend_safe con kind=effect. Conservar los literales anteriores y sus kind. Evidencia verified salvo hoja desconocida unknown según §8.2. ownership.check, effects.check e ir.lower no son pasos causales. | C06/G05/G06/G07/Q01; §8.2 |
| A2 §13.15: «NX-E0111/missing_argument o argument_mode_mismatch» / «`declaration`: nombre del parámetro requerido.» | Enmienda propuesta: conservar cuando exista declaración fuente; para el receptor sellado Iterator.Next no fabricar span de parámetro intrínseco. Sin edit: arguments/call.bind, primario argumento completo, Data {}, relaciones y cadena [], NX-E0111/argument_mode_mismatch. | N02; §5.2/§8.2 |
| A2 §13.6: «`borrow_inout`, solicitar un préstamo exclusivo mediante un argumento `inout`.» | Enmienda propuesta: borrow_inout solicita préstamo exclusivo mediante argumento edit, captura edit o binding de patrón edit; no implica préstamo adquirido ni mutación write. La solicitud rechazada no crea LoanId. | M06; §8.2 |
| D15 §2.3: «`Ok`, `Err` sin cambio; etiqueta de payload `v` para ambos, conservando `src/check/names.rs::intrinsic`» | Enmienda propuesta: añadir Some con etiqueta v para su único payload; se acepta Some(v:1), igual que Ok/Err. No se introduce una etiqueta value alternativa. | §3.1; §9.4 |
| D15 §6.1: «`effect_model = f0-0002-sim-v1` permanece.» | El modelo nuevo es f0-0003-sim-v1, con resumen de suspensión, entornos y manifiesto SuspendSafe; mantiene los eventos y namespace R históricos. Los costes y efectos del dominio anterior no cambian. | §7–§9; G06/G07, Q01 |
| D15 §3.3, fila «Closure síncrona»: «`create copy call drop`; `call` obligatoria» y fila «Async privada/local, incluida closure»: «`create resume cancel drop`; `resume` obligatoria» | Añadir fila gen síncrono: create/resume/drop, resume obligatoria; lista simple limita resume. No heredar cancel ni semántica Future. Es una laguna de la tabla, no permiso para dos gramáticas de with. | §5.4; cotas gen/J12 |
| D10: «Las capturas retenidas tienen que ser propietarias; no se devuelve un generador con préstamos locales escapados.» | La retención externa es propietaria. View escalar se materializa por copia al crear, conservando lectura en el cuerpo. Edit y view no escalar se rechazan; mover Array es explícito. Los préstamos internos referencian slots propios estables. | §5.1; G04 |
| D14 §7.5: «El parámetro de EX04 conserva el modo del original y sus obligaciones D10; materializar el escalar retenido debe justificarse por las reglas del generador, no por cambiarlo a transferencia para una sola candidata.» | §5.1 fija esa regla de materialización; no se cambia view de EX04 a move para ocultar el problema. La reducción usa I64, único entero del perfil, sin añadir U32. | §5.1 y EX04 |
| D10: «Un generador fallable entrega `Result<Item,E>`; no existe una excepción oculta al iterar.» | Next devuelve Option<Result<...>>, yield Err no termina por sí mismo. `?` directo en gen se rechaza con NX-E1214 porque no tiene resultado final Result; el usuario escribe selección/yield/return vacío. No se modifica el `?` ordinario de I08. | G03; §9.2 |
| D12 §1: «En un cuerpo ordinario se emite NX-E1213, con primary_span del token yield» | Se conserva también en closure ordinaria interior: el contexto invocable más próximo decide. No se transmite la condición gen al árbol completo. | G02; contexto léxico/J11 |
| D15 §3.6: «`indirect_call` permanece retirado por inalcanzable» | No se resucita ese ID al admitir closures. La llamada resuelta se valida por firma y capacidad; funciones nombradas como valores y métodos de usuario siguen diferidos. Iterator.Next es identidad nominal intrínseca, no método abierto. | C05/C08/C09; N01 |
| A2 §13.6: «Los cuatro campos de efectos son obligatorios exclusivamente para NX-E0412–NX-E0415.» | Guard_effect incorpora tres, sin contract_origin ficticio; los demás datos nuevos se cierran en §8.2. Se añaden permission_source, las siete claves de patrones/receptores (missing_pattern, constructor, expected_arity, found_arity, scrutinee_type, required_mode, requested_mode) y las dos de SuspendSafe solo a sus códigos. | M01/M04/M05/C05/C09/G05/Q01 |
| A2 §13.15: «Las demás reglas, incluidos los siete R, tienen `cause_chain = []`.» | Añadir las cadenas explícitas de escape, SuspendSafe y guard_effect de §8.2; los R conservan cadena vacía. No se introduce stack trace variable. | C06/G05/Q01 |
| A2 §5.4: «Ningún tipo aceptado almacena préstamos.» | Los tipos del dominio previo no almacenan préstamos; las closures de este perfil pueden retener permisos view/edit y Option/Result los propagan sin independizarlos. Su duración, copia, traslado y escape se comprueban por §4.3 con el mismo motor de propiedad. Parámetros ordinarios y generadores conservan sus restricciones respectivas. Esta enmienda es propiedad de F0-0003. | C04/C06/M06/G05; J04/J07/J12 |
| A2 §13.12: «`non_callable_target` no necesita inferir el tipo completo de un binding: en este perfil ningún binding puede contener un valor invocable aceptado. La identidad resuelta permite rechazarlo en `arguments`.»; fila «NX-E0111, sus siete subcausas» | Un binding puede contener una closure. non_callable_target se decide tras resolver su tipo en types/call.bind; las otras seis subcausas conservan arguments/call.bind cuando la firma es conocida. Para firmas inferidas se recogen asociaciones y restricciones en el mismo grafo antes de decidir el destino; un destino contaminado no produce cascada de asociación. None llamado es no invocable sin inferir T; un local escalar conserva relación declaration de A2. | A2 S07; C05/C08/C09; None(); §4.2 |
| A2 §13.17: «Los únicos dominios adicionales permitidos en `expected_type` son:» seguido de «domain:copyable_data», «domain:array», «domain:mock_resource», «domain:result» | Añadir domain:generator para el receptor de Next y domain:iterable para for sobre tipo no iterable. No son tipos fuente ni traits; found_type siempre es concreto. | N01; for sobre I64; §2.2/§5.2 |
| D15 §2.3: «Las grafías intrínsecas son entradas explícitas de un manifiesto, como cualquier API sensible a caja (D14 §§2.2–2.3; A2 §§4.4 y 8.4).» | Añadir identidad sellada prelude:Iterator y miembro prelude:Iterator.Next al manifiesto. La llamada nominal es Iterator.Next(edit g), sin etiqueta para el receptor; recibe estado exclusivo y devuelve Option<Item>. Resolución y sombreado son nominales; un homónimo de usuario no adquiere la operación. | N01/G06; todas las llamadas Next; §5.2 |
| D15 §7.1: «`stage=types`, `operation=types.check`; primario sobre la primera construcción que necesita la capacidad pendiente, o EOF vacío para una petición del arnés interno sin construcción fuente.» | Para implementation_limit usar etapa, ancla y pending_scope de §2.2, con operation de §8.1. Los rechazos J14 son lower/ir.lower. Arrays de closure y for edit son exclusiones permanentes con sus propios feature_id, no implementation_limit; conservan types/types.check y parse/syntax.parse respectivamente. Conservar EOF vacío solo para petición interna sin fuente. | J02–J14; L01/L02; positivos previos a J14 |
| D15 §8.7 F2-F25: «`fn Main(){⟦for⟧ n in [1]{}}`», «`for_iteration`», «`[10,13)`» | Sustituir por NX-E0006/for_iteration, pending_scope=later_adr, types/types.check, primario [19,22) sobre [1]. Array es iterable pero todavía fuera del corte. Para tipo no iterable usar NX-E0102/domain:iterable; para generador propietario aceptar según §5.4. | F2-F25; §2.2; F01 |
| D15 §8.7 F2-F26: «`fn Main(){while false{⟦break⟧;}}`», «`loop_control`», «`[22,27)`» | La fuente sin marcadores es positiva: check exit 0, diagnostics=[]; run unit, exit 0, observations=[]. break/continue sin bucle del mismo invocable usan NX-E1326 en types, no un rechazo global de parse. | F2-F26; F02/F03 |
| A2 §13.12, fila de alcance con «`methods`» asignada a «`parse` / `syntax.parse`»; D15 §8.7 F2-F23: «`fn Main(){let a=[1];a⟦.⟧Foo();}`», «`methods`», «`[21,22)`» | El selector se reconoce en parse y se resuelve en names/names.resolve. Salvo Iterator.Next nominal, NX-E0006/methods, pending_scope=later_adr, primario en punto [21,22) para F2-F23. No se acusa sintaxis ni se toma un miembro por coincidencia textual. | F2-F23; §2.2/§5.2 |
| D15 §8.7 F2-F10: «`fn F(){} fn Main(){⟦move⟧ F();}`», «`receiver_mode`», «`[19,23)`»; A2 §13.12, fila de alcance con «`receiver_mode`» asignada a «`parse` / `syntax.parse`» | Receptor de función ordinaria conserva NX-E0006/receiver_mode sobre el prefijo, pending_scope=later_adr, pero pasa a names/names.resolve. Receptor closure se comprueba por §4.2 y ya no está globalmente excluido. | F2-F10; edit Foo(); C05/C08/C09 |
| D13 §1.1.3: «`return ();`: NX-E1324, span del return y referencia al -> T de elemento.» | Concretar referencia como related_spans=[expected_type sobre la cláusula completa -> T]. Mantener Data {}, primario return y cadena []; si Item=() por omisión, related_spans=[] porque no hay cláusula escrita. No inventar una referencia al nombre del gen. | G01, relación [10,15); §8.2 |
| D13 §4 y catálogo NX-E1305: «`guard_effect`, `guard_mutation`, `guard_transfer`» | Añadir guard_control por salida de la evaluación de guarda. Transfer/mutation/control pertenecen a ownership/ownership.check; los efectos prohibidos a effects/effects.check. Mantener las barreras y supresión causal de §3.3. | Q01–Q04/Q06/Q07; §8.2; J05 |
| A2 §13.15: «Una limpieza implícita comienza en el propietario y termina en `resource.drop`.» | Conservar esa regla para sitios de ejecución. Para una cota create/copy/drop escrita en el literal closure, comprobar el resumen al declarar el literal, con primario fn y relación contract; la cadena estructural conserva la captura como origen, copia/destrucción con sus kind y hoja resource.drop sin span cuando corresponda. No añadir origin/exit de una limpieza de Main que no es el sitio comprobado. | E01–E04; §9.3 |
| A2 §13.6: «`use_kind`» con «`value` o `call_target`» | call_target incluye el nombre de constructor de patrón que no resuelve; no añadir otra clave ni subcausa. | M08; NX-E0101 |

### 10.1. Enmienda propuesta a D15: gramática activa y FIRST

Las listas de F0-0003 son la base normativa. F0-0005 y cortes posteriores
las extienden de forma acumulativa: añaden sus propios comienzos y conservan
los ya admitidos, sin sustituirlas por las listas históricas de D15. Se mantiene
una sola gramática activa y un único motor de tipos/propiedad. BlockNext
incluye yield en cualquier bloque; G02 debe llegar a types/types.check,
no fallar en parse por pertenecer a una función ordinaria.

D15 §6.2 dice literalmente: «Listas literales ordenadas por bytes; Name incluye los contextuales, pero se enumeran también los prefijos explícitos en los estados que los admiten. Estas listas **no** incluyen construcciones diferidas como si fueran aceptadas.»
La regla se conserva; las construcciones incorporadas dejan de ser diferidas.
Se sustituyen las listas siguientes; las demás listas de §6.2 no cambian.

| Texto exacto vigente de D15 §6.2 | Lista literal para F0-0003 |
|---|---|
| `ExprStart = ["(","-","IntLiteral","Name","[","edit","false","if","move","true"]` | `ExprStart = ["(","-","IntLiteral","Name","[","edit","false","fn","if","match","move","true"]` |
| `TryStart = ["(","-","IntLiteral","Name","[","edit","false","move","true"]` | `TryStart = ["(","-","IntLiteral","Name","[","edit","false","fn","move","true"]` |
| `PostfixStart = ["(","-","IntLiteral","Name","[","false","true"]` | `PostfixStart = ["(","-","IntLiteral","Name","[","false","fn","true"]` |
| `BlockNext = ["(","-","IntLiteral","Name","[","edit","false","if","let","move","return","true","var","while","{","}"]` | `BlockNext = ["(","-","IntLiteral","Name","[","break","continue","edit","false","fn","for","if","let","match","move","return","true","var","while","yield","{","}"]` |
| `TypeStart = ["(","Array","Bool","I64","MockBlockingResource","MockResource","Result"]` | `TypeStart = ["(","Array","Bool","I64","MockBlockingResource","MockResource","Option","Result"]` |
| `GroupFirst = ["(",")","-","IntLiteral","Name","[","edit","false","if","move","true"]` | `GroupFirst = ["(",")","-","IntLiteral","Name","[","edit","false","fn","if","match","move","true"]` |
| `ArrayFirst = ["(","-","IntLiteral","Name","[","]","edit","false","if","move","true"]` | `ArrayFirst = ["(","-","IntLiteral","Name","[","]","edit","false","fn","if","match","move","true"]` |
| `BranchFirst = ["(","-","IntLiteral","Name","[","edit","false","move","true","{"]` | `BranchFirst = ["(","-","IntLiteral","Name","[","edit","false","fn","move","true","{"]` |
| `ModuleNext = ["EOF","fn"]` | `ModuleNext = ["EOF","fn","gen"]` |

D15 §6.3 dice literalmente: «Listas nombradas se expanden literalmente según §6.2.» Sus filas afectadas vigentes son:

| Fuente / ID | Token, span | Contexto | Esperados | Prefijo |
|---|---|---|---|---|
| `fn Main(){` | EOF `[10,10)` | `block_item` | BlockNext | `[9,10)` |
| `fn Main(){let a=` | EOF `[16,16)` | `binding_initializer` | ExprStart | `[10,16)` |
| `fn Main(){let a=[1,` | EOF `[19,19)` | `array_element` | ExprStart | `[16,19)` |
| `fn Main(){let a=1;} garbage` | Name `garbage` `[20,27)` | `module_item` | `["EOF","fn"]` | `[0,20)` |
| `// expect: NX-E0002` + LF + `fn Main() {} ignored` | Name `ignored` `[33,40)` | `module_item` | `["EOF","fn"]` | `[0,33)` |
| Mismo con CRLF | Name `ignored` `[34,41)` | `module_item` | `["EOF","fn"]` | `[0,34)` |
| `fn F(x:view){}` | Symbol `)` `[11,12)` | `grammar.Type` | TypeStart | `[11,11)` |
| `fn Main(){let x=not true;}` | keyword not `[16,19)` | unsupported_unary_operator | ExprStart | `[10,16)` |

Enmienda propuesta: sustituirlas por las siguientes filas, con expected_tokens
literales ordenados por bytes. Se conservan todos los campos comunes NX-E0002
de D15 §6.3, incluidos token.span=primary_span, subcause=null y Data {}.

| Fuente / ID | Token, span | Contexto | expected_tokens | recognized_prefix |
|---|---|---|---|---|
| `fn Main(){` | EOF `[10,10)` | `block_item` | `["(","-","IntLiteral","Name","[","break","continue","edit","false","fn","for","if","let","match","move","return","true","var","while","yield","{","}"]` | `[9,10)` |
| `fn Main(){let a=` | EOF `[16,16)` | `binding_initializer` | `["(","-","IntLiteral","Name","[","edit","false","fn","if","match","move","true"]` | `[10,16)` |
| `fn Main(){let a=[1,` | EOF `[19,19)` | `array_element` | `["(","-","IntLiteral","Name","[","edit","false","fn","if","match","move","true"]` | `[16,19)` |
| `fn Main(){let a=1;} garbage` | Name `garbage` `[20,27)` | `module_item` | `["EOF","fn","gen"]` | `[0,20)` |
| `// expect: NX-E0002` + LF + `fn Main() {} ignored` | Name `ignored` `[33,40)` | `module_item` | `["EOF","fn","gen"]` | `[0,33)` |
| Mismo con CRLF | Name `ignored` `[34,41)` | `module_item` | `["EOF","fn","gen"]` | `[0,34)` |
| `fn F(x:view){}` | Symbol `)` `[11,12)` | `grammar.Type` | `["(","Array","Bool","I64","MockBlockingResource","MockResource","Option","Result"]` | `[11,11)` |
| `fn Main(){let x=not true;}` | keyword not `[16,19)` | unsupported_unary_operator | `["(","-","IntLiteral","Name","[","edit","false","fn","if","match","move","true"]` | `[10,16)` |

D15 §3.2, texto exacto vigente:

> Se mantiene el dominio F0 de A2 §§3.5–3.6 y 4.6: solo `<` y `==` se aceptan. `*`, `/`, `%`, `!=`, `>`, `<=`, `>=`, `and` y `or` se tokenizan pero producen NX-E0002 en el reconocedor de operador excluido; `not` fuera de una producción F0 produce NX-E0002. B completa fija su precedencia en §3.5. La migración de superficie no implementa aritmética ni lógica adicional. `and/or` en posición infija usan `parser_context=unsupported_binary_operator`, `expected_tokens=["+","-","<","=="]`; `not` al comienzo de un operando usa `parser_context=unsupported_unary_operator`, `expected_tokens=["(","-","IntLiteral","Name","[","edit","false","if","move","true"]`. Los tres tienen NX-E0002, subcause null, stage parse, operation syntax.parse, token kind keyword sobre la palabra completa, expected_tokens_kind admissible_set, data {}, repair [] y recognized_prefix desde el comienzo de la sentencia activa hasta ese token. En posiciones que exigen un nombre rige el contexto de nombre de §6.2. Estas causas preceden al terminador ausente derivado. Una selección descartada en posición de sentencia usa `(if condición then a else b);`; sin paréntesis, el introductor `if` selecciona IfStmt.

Enmienda propuesta: conservar esa regla salvo la lista literal de
unsupported_unary_operator, que pasa a `["(","-","IntLiteral","Name","[","edit","false","fn","if","match","move","true"]`.
La fila not anterior fija fuente, token y prefijo; no habilita not.

D15 §3.2, texto exacto vigente:

> Clasificación de prefijos F0, en este orden (A2 §3.6): llamada completa → `receiver_mode`; operando que no es Place → NX-E0002, contexto estable `consume_place` o `inout_place`; `move` de Place con índices → `partial_transfer`; binding completo o Place editable → comprobación semántica. `move (x)` conserva transferencia completa. `edit` no se acepta como permiso general fuera de los contextos admitidos: un `edit` fuera de un argumento de llamada se rechaza en el análisis sintáctico con NX-E0002, `parser_context=inout_place`, `expected_tokens=["Place"]`, primario sobre el prefijo `edit` completo, prefijo reconocido vacío en ese sitio y `data={}`, antes de cualquier bajada a IR. `view` no es un prefijo de expresión.

Enmienda propuesta: reconocer edit en argumento de llamada sobre Place,
en scrutinee de match sobre Place y como modo de receptor de una llamada
completa, incluida `edit f();`. Resolver esta última antes de decidir su
capacidad: closure sigue §4.2; función ordinaria usa receiver_mode en
names/names.resolve. En argumento y scrutinee, un operando no Place conserva
NX-E0002/inout_place; una llamada completa con prefijo es receptor, no préstamo
de su resultado. Edit sobre Place fuera de argumento o scrutinee conserva
NX-E0002/inout_place sobre edit completo, expected_tokens=["Place"], prefijo
reconocido vacío y Data {}. Edit de parámetro, captura y patrón pertenece a
sus respectivas producciones, no a un permiso general de expresión. For edit
sigue excluido según §2.2. Move conserva las reglas de §3/§4.2 y view no es
prefijo de expresión.

D15 §2.2, filas literales vigentes:

| Clase | Lista cerrada |
|---|---|
| Reservadas F0 | `fn let var return if then else while true false with` |
| Reservadas de B completa, reconocidas pero no aceptadas en F0 | `and or not match for break continue gen yield async await region struct enum type trait impl where dyn mod use pub extend extension comptime const self super package` |

Enmienda propuesta: sustituirlas por estas listas; no cambia la categoría
léxica keyword de las palabras activadas ni las reglas de nombres.

| Clase | Lista cerrada |
|---|---|
| Reservadas aceptadas | `fn let var return if then else while true false with match for break continue gen yield` |
| Reservadas reconocidas pero no aceptadas | `and or not async await region struct enum type trait impl where dyn mod use pub extend extension comptime const self super package` |

### 10.2. Enmiendas propuestas de etapas y atribución

A2 §13.12, fila literal vigente:

| feature_id | stage / operation |
|---|---|
| `legacy_move`, `float_literal`, `range`, `tuple`, `attributes`, `default_argument`, `closure`, `receiver_mode`, `no_heap_alloc`, `match`, `user_types`, `traits_generics`, `modules_visibility`, `extensions`, `provenance_clauses`, `comptime`, `text`, `methods`, `compound_assignment`, `for_iteration`, `loop_control`, `generator`, `region` | `parse` / `syntax.parse` |

Enmienda propuesta: closure para arrays con closure transitiva y generator
para ciclos de estados usan types/types.check (L02 y G08). Requieren conocer
el tipo del elemento y el grafo de estados respectivamente; parse reconoce
las formas, pero no decide esas propiedades. No son rechazos globales de sus
introductores. Métodos/selectores y self expresión pasan a names/names.resolve;
las otras etapas se conservan salvo enmienda expresa de §10.

D15 §8.7, fila literal vigente:

| ID | Fuente negativa marcada | feature_id | Primario | Edición positiva |
|---|---|---|---|---|
| F2-F33 | `fn Main(){let x=⟦self⟧;}` | `methods` | `[16,20)` | Sustituir `self` por `0`. |

Enmienda propuesta: sin marcadores, NX-E0006, subcause=null, feature_id=methods,
Data {"pending_scope":"later_adr"}, names/names.resolve; primario self [16,20),
related_spans=[], cause_chain=[] y campos sintácticos nulos según §8.1.
Positivo completo: `fn Main(){let x=0;}`, diagnostics=[], check/run exit 0,
unidad y observations=[]. Self se reconoce para este rechazo de alcance,
no como receptor implementado ni como nombre libre que produzca NX-E0101.

F0-0003 no define ni emite NX-E1323/recursive_value_layout: la recursión de
estados usa NX-E0006/generator (G08). Los cortes posteriores deben conservar
esta atribución: cualquier emisión de NX-E1323 requiere su propia definición,
campos y fixtures; no se hereda de los estados J ni amplía el catálogo
emitible de este corte.

Las referencias externas a pasos J01–J16 deben citar el nombre F0-0003 y
la capacidad; no se transplantan esos IDs al espacio de subtareas de otros ADR.
Los cortes posteriores deben conservar Item=() al omitir la flecha en gen
y limitar inferred_non_unit_result a funciones ordinarias. Al habilitar tipos
de usuario, un tipo llamado Iterator debe sombrear la identidad del preámbulo
sin heredar Next, conforme a la resolución nominal de §5.2. Deben conservar
for_iteration/later_adr para for edit, closure/later_adr para arrays de closure
y domain:iterable para tipos no iterables según §2.2; habilitar Array por valor
requiere una obligación propia del corte que lo incorpore. Estas obligaciones
hacia cortes posteriores no son dependencias de ejecución ni condiciones
previas para implementar este documento.

El dominio de for de D10/D13 se concreta en generadores propietarios;
Iterator de usuario, for edit y Array quedan rechazados nominalmente en §2.
Los tipos anónimos dentro de arrays se difieren por representación; esta
limitación no altera la regla general de copiabilidad de D11. Las funciones
gen públicas, préstamos externos con procedencia, tipos de callback escritos,
retains/from, actores, regiones y publicación son límites explícitos, no
supuestos satisfechos por el modelo local.

## 11. EX04 reducido ejecutable

La reducción conserva generación perezosa, for, entrega por valor y consumo
temprano. Usa I64 y suma de impares porque U32 y multiplicación no pertenecen
a A2; no se amplía aritmética por una fixture. MockRequest aporta el límite;
el dominio admitido por esta aplicación es 0..1000 inclusive. Un límite
fuera del dominio devuelve Err(41) antes de adquirir recurso. El recurso 4
representa estado retenido y permite observar abandono y limpieza. No
representa una API de filesystem ni un scheduler.

```nexo
gen fn Cuadrados(limite:view I64,r:move MockResource)->I64 {
    var n=0;
    var cuadrado=0;
    var impar=1;
    while n<limite {
        Inspect(r);
        yield cuadrado;
        n=n+1;
        if n<limite {
            cuadrado=cuadrado+impar;
            impar=impar+2;
        }
    }
}
fn Main()->Result<(),I64> {
    let limite=MockRequest()?;
    if limite<0 {return Err(41);}
    if 1000<limite {return Err(41);}
    for valor in Cuadrados(limite,MockOpen(4)) {
        MockWriteI64(valor)?;
    }
    return Ok(());
}
```

La recurrencia produce 0,1,4,9,...; no usa array de resultados ni respuesta
especial por nombre de archivo. Main no promete no_suspend: su for avanza
un generador que suspende. Una variante con ruptura tras el primer elemento
se obtiene insertando `break;` inmediatamente después de MockWriteI64;
el resto de la fuente es idéntico.

Entorno de éxito para límite 3, con el esquema completo heredado:

```json
{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null},{"ok":null},{"ok":null}],"mock_request":[{"ok":"3"}],"mock_reply":[]}
```

| Caso | Variación del entorno | Resultado y observación requerida |
|---|---|---|
| Completo | El JSON anterior | Ok, exit 0; stdout ["0","1","4"]; eventos request:ok:3, open:0:4, inspect:0:4, write_i64:0:ok, inspect:0:4, write_i64:1:ok, inspect:0:4, write_i64:4:ok, close:0:4. Cada escritura exitosa va seguida inmediatamente de su stdout. |
| Vacío | Petición Ok("0"), cola de escritura vacía | Ok; request:ok:0, open:0:4, close:0:4; sin inspect ni stdout. |
| Entrada fallida | Petición Err("10"), escritura vacía | Err(10), exit 1; solo request:err:10; sin recurso. |
| Dominio | Petición Ok("-1") o Ok("1001") | Err(41), exit 1; solo request:ok:-1 o request:ok:1001. |
| Escritura fallida | Petición Ok("3"); escrituras Ok, Err("40"), Ok | Err(40), exit 1; request/open; inspect/write de 0 y stdout 0; inspect/write_i64:1:err:40; close. No tercer inspect ni escritura de 4. |
| Cola agotada | Petición Ok("1"), sin escrituras | Err(-9000); request/open/inspect/write_i64:0:err:-9000/close; stdout []. |
| Abandono | Fuente con break; petición Ok("3"), tres escrituras Ok | Ok; request/open/inspect/write de 0/stdout 0/close; no segundo avance. |

En todas las filas diagnostics=[], reservas finales cero. Err tipado no
produce R. El caso completo suma **221** bytes observables: request 21, apertura
23, tres inspect de 26, tres escrituras de 23, tres stdout de 2 y cierre 24. Los tamaños se verifican
contra los bytes literales con prefijos A2, no contra la longitud JSON.
Las demás filas suman las mismas unidades de su registro correspondiente.
La secuencia de eventos y stdout es normativa; stdout/events del informe
son proyecciones de observations, no listas producidas por separado.

Variante R literal, bajo f0-0003-b-v1, sin reducir cuotas:

```nexo
gen fn Cuadrados(limite:view I64,r:move MockResource)->I64 {
    var n=0;
    var cuadrado=0;
    var impar=1;
    while n<limite {
        Inspect(r);
        yield cuadrado;
        n=n+1;
        if n<limite {
            cuadrado=9223372036854775807+1;
            impar=impar+2;
        }
    }
}
fn Main()->Result<(),I64> {
    let limite=MockRequest()?;
    if limite<0 {return Err(41);}
    if 1000<limite {return Err(41);}
    for valor in Cuadrados(limite,MockOpen(4)) {
        MockWriteI64(valor)?;
    }
    return Ok(());
}
```

Entorno exacto:

```json
{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null},{"ok":null}],"mock_request":[{"ok":"2"}],"mock_reply":[]}
```

Observations, en orden: event request:ok:2, event open:0:4,
event inspect:0:4, event write_i64:0:ok, stdout "0", event close:0:4.
La proyección events contiene esos cinco eventos en ese orden y stdout=["0"].
No hay segundo inspect ni escritura; queda una respuesta de escritura sin usar.
La ejecución termina con exit 1 y único R0101, operation=value.add,
namespace F0-0002-sim, stage=runtime, sin resultado Err ni Some adicional;
Data, relaciones y cadena según A2 para overflow de suma. Primario: expresión
`9223372036854775807+1` en [234,255), contando el contenido del bloque
ASCII con LF y sin LF final. Reservas finales cero; un solo cierre de r.
Las observaciones suman 119 bytes (21+23+26+23+2+24). La reserva de cierre
se consume al abortar el avance. Las variantes de cuotas de trabajo/celdas/
observaciones deben conservar el cierre durante un avance interrumpido;
no se sustituyen por este caso aritmético como evidencia de esas cuotas.

## 12. Subtareas y dependencias

### 12.1. Entrada desde el segundo corte

I07c es requisito del motor monomórfico y flujo hipotético único. I08 aporta
Result/`?`, propagación y limpieza; I09 aporta efectos transitivos; I10a
aporta cuotas completas de análisis y I10b ejecución/reservas. J01 requiere
los cinco completados según A2/D15, no simplemente la existencia de código.
J02–J14 pueden construirse sin I11a: usan operaciones mock anteriores o
pruebas internas acotadas. J15 requiere I11a para colas/observaciones y los
tres flujos. J16 requiere I11b para conservar su evidencia integrada.
Ningún I depende de un J. No se altera el alcance de I08–I11 para resolver
por adelantado obligaciones de este ADR.

### 12.2. Orden de construcción

Cada fila entrega una unidad verificable y conserva las barreras temporales
de §2 hasta completar efectos/cuotas. No exige ejecutar prematuramente una
capacidad incompleta. El orden lineal siguiente es una ordenación topológica;
las dependencias enumeradas son las necesarias, incluidas las transitivas
a través de otra fila. No hay pares a/b sin una separación técnica real.

| Paso | Depende de | Entrega y validación independiente mínima |
|---|---|---|
| J01 | I07c,I08,I09,I10a,I10b | Perfil, fronteras, delta de catálogo/schema y manifiesto; pruebas de conjuntos cerrados y barreras, regresión de dominio A2. Todavía rechaza capacidades J pendientes. |
| J02 | J01 | Option en representación, solver y operaciones IR de tag/payload; None inferido, copia/drop transitivos, O01/O02, formas límite None() y Some<I64> de §9.3 con sus positivos, y destrucción única de Some(resource). |
| J03 | J02 | AST B y selección de lectura, cobertura simbólica/unificación; M01–M05/M07/M08/M10/M11 y residuos anidados; verificador de acceso demostrado; M07 solo acredita prioridad, con aislamiento fuente fuera del subset; forma límite del binding `None` bajo un scrutinee Result de §9.3 con su positivo. |
| J04 | J03 | Adquisición y extracción por brazo, joins y planes de limpieza; M06/M09 y matrices de movimiento/permiso y ramas divergentes; sin guardas todavía. |
| J05 | J04 | Guardas y secuencia provisional/compromiso, pureza y efectos; Q01–Q04/Q06/Q07, guarda false sin movimiento, alcance y cardinalidad. |
| J06 | J02 | Identidad/firma/entorno de closure con copy/move, adquisición parcial y destrucción; C01–C03/C07, independencia Array y rechazo de arrays de closures. |
| J07 | J06,J04 | Retención view/edit y propagación por uniones, movimiento/copia y escape; C04/C06 y préstamos múltiples; mismo motor de propiedad. |
| J08 | J07 | CallClosure, capacidades/mutabilidad, argumentos y modos de receptor; C05/C08/C09, forma límite edit Foo() de §9.3 con su positivo, consumo único, IR alterada y frame explícito. |
| J09 | J08,J05 | Resúmenes create/copy/call/drop, cotas y testigos; W09, E01–E04, separar cuerpo latente/destructor, ciclos de llamadas y unknown. |
| J10 | J09 | Layout de estado propietario, preparación de argumentos, construcción fría/descarte; G04 y exclusión permanente de recursión G08; no ejecutar aún Next. |
| J11 | J10 | Yield/continuaciones/Next/finalización y resultado Option; G01–G03/G09/G10, N01/N02/N03/N04, formas límite 1? y `struct Iterator{}` de §9.3 con sus positivos, repetición None, transferencia de Item y traslado suspendido. |
| J12 | J11 | SuspendSafe y create/resume/drop, cotas y limpieza de avance fallido; G05/G06/G07, W08/W10, E05 y efectos transitivos por closure. |
| J13 | J12,J05 | For propietario y bordes break/continue en while/for; F01–F04/L01, forma límite for x in 1 de §9.3 con su positivo, cierre en return/`?` y elemento anterior antes del siguiente Next. |
| J14 | J13 | Contabilidad incremental de las operaciones nuevas y perfiles reducidos; fronteras, reservas y pilas pequeñas. Retira las barreras J completadas, no los límites L01, L02 y G08. |
| J15 | J14,I11a | EX04 de §11 mediante interfaz CLI real y entradas variables; errores y abandono; regresión de EX01–EX03 del perfil anterior. |
| J16 | J15,I11b | Batería integrada independiente, mutaciones supervivientes, escala y contrato exacto de diagnósticos/observaciones; matriz de obligaciones y límites restantes. |

Tipos, propiedad, efectos y cuotas no se posponen «hasta la integración»:
cada J añade sus cargos y evidencia unitaria; J14 verifica composición y
fronteras completas. Un fallo corregido añade la regresión discriminante en
su J antes de continuar. Los límites transitorios no prueban la semántica
final de una regla. Al terminar J14 solo persisten las exclusiones estables de §2.2, incluidas
closure/later_adr para arrays y for_iteration/later_adr para for edit.

## 13. Aceptación medible y pruebas adversariales

### 13.1. Oráculos y mutaciones

La aceptación de una regla exige su negativo aislado y positivo pareado,
objetos esperados transcritos de este ADR y A2/D15, y comportamiento observado
del pipeline check/run. El oráculo de ejecución es un modelo separado de
valores/recursos/continuaciones y contabilidad: no importa módulos del
compilador, no analiza dumps para decidir el esperado ni replica su AST.
Puede generar programas desde una gramática acotada junto con una traza
abstracta; el resultado de esa traza se determina antes de compilar.
Los spans se calculan por conteo de bytes, con auditoría manual de literales.

Por regla explícita se invierte al menos una condición del comprobador o del
verificador y se prueba que una fixture discriminante cambia de aceptada a
rechazada o viceversa, o detecta la observación incorrecta. Una mutación no
cuenta si solo rompe compilación, sintaxis o cuota ajena. Se registra sitio,
regla, fixture, efecto esperado, observado y supervivencia. Se restaura y
verifica SHA-256 de cada archivo antes de seguir. No se deja mutación activa.
En una revisión posterior se reejecutan supervivientes y como máximo ocho
mutaciones nuevas, bloqueantes únicamente para reglas explícitas del ADR.

Se exigen mutaciones de tag, guarda antes de extracción, préstamo perdido
por copy/move, escape en Option, CallOnce reutilizado, retorno de gen con
operando, yield sin suspend, avance frío ejecutado al crear, continuación
reiniciada, flag de Item no desarmado, cierre omitido en break/Err/R, frame
suspendido confundido con activo y cargo de reserva omitido. IR alterada
se valida en el arnés del verificador; no se atribuye esa evidencia al
modelo de ejecución independiente.

El revisor compara check/run contra HEAD mediante git archive HEAD y su
propia batería. No modifica src vivo de otro agente para obtener esa
comparación. Las diferencias deliberadas de perfiles y alcance de §2 son
una lista cerrada; las demás regresiones exigen explicación y corrección.
Nombres de pruebas describen la propiedad, sin recuentos de rondas o pasadas.

### 13.2. Escala y pila acotadas

Se conserva toda la matriz de cuotas de A2, incluidos local/global de flujo,
sin reiniciar contadores al entrar en closure, estado o nuevo análisis. Las
familias nuevas usan tamaños 0,1,2,8,32,128,512 sujetos a source_bytes; el
resultado esperado puede ser aceptación o la guarda concreta previamente
calculada, nunca cualquier error. Semilla fija 0x4E45584F; expansión en orden
numérico y fuente reproducible. Se registra fuente exacta y hash de cada
caso generado. No se elige tamaño midiendo cuándo falla el producto.

| Familia | Criterio medible |
|---|---|
| Option/Result anidados y patrón residual | Profundidad 127/128/129 contando envoltorios y entradas de sintaxis; elegir primero la guarda según prioridad heredada. Casos aplanados separan profundidad de coste de cobertura. |
| Lectores retenidos | N copias de una closure lectora, destrucción en varios órdenes; la raíz solo se libera tras la última obligación, con N adquisiciones y N liberaciones verificables. |
| Suspendidos | N estados vacíos conforme §7.2; 63/64/65 en cells-64; cero ejecuciones de cuerpo al construir o descartar. |
| Avances | Secuencia de N yields mediante bucle, no N nodos anidados; N Some y dos None finales, N elementos y exactamente un cierre de estado. |
| Frames | Profundidad activa 7/8/9 en frames-8, sumando llamadas y avances; estados suspendidos independientes no consumen frames activos. |
| Match | N brazos de I64 más residual y patrones anidados; sin enumeración del dominio I64. Las visitas se cargan al presupuesto, no se ocultan en colecciones auxiliares. |
| Entornos y efectos | N funciones/closures en cadena y ciclo de llamadas; manifiestos conocidos/unknown, fixed point acotado y testigos deterministas. |
| Bytes observables | Una apertura con cierre reservado, recurso en closure/estado y fallo antes/durante avance; emitted+reserved nunca excede el límite, cierre transfiere reserva y reserva final es cero. |

Se ejecutan análisis, verificación, ejecución y destrucción con pila de
anfitrión de 256 KiB. Si la plataforma exige un mínimo mayor, se registra
ese mínimo y se repite en una plataforma que permita 256 KiB; no se sustituye
por RUST_MIN_STACK enorme. No hay recursión del anfitrión proporcional a
longitud de instrucciones, valores, capturas ni número de estados. El arnés
incluye destrucción después de rechazo parcial, no solo ejecución exitosa.

Fuzz determinista: todos los prefijos truncados de las fuentes de §9 y §11,
mutación de un token a la vez, bytes UTF-8 inválidos, comentarios Unicode y
CRLF como variantes de entrada. Se comprueba ausencia de panic/hang, límite
de trabajo antes de ejecución no acotada, spans válidos, orden estable y
limpieza observada. El documento permanece LF; CRLF es dato de prueba.
Los negativos semánticos conservan su batería separada: un truncamiento que
produce NX-E0002 no acredita préstamo, exhaustividad ni limpieza.

### 13.3. Condiciones de cierre del corte y límites de evidencia

Todas las filas J y de §9 deben tener evidencia por requisito con estados
PASA/FALLA/NO EVALUADO/INCONCLUSO, fuente, perfil, comando, expected/actual
y trazas pertinentes. Una discrepancia de spans, cardinalidad, campos,
observaciones, cuotas o limpieza es un defecto, aunque el código general
coincida. Ninguna fixture fuera de alcance acredita la regla que habría
seguido a NX-E0006. Las obligaciones de emisión de un J posterior se anotan
como pendientes del J correspondiente, sin adaptar el esperado al stub.

Se exige ausencia de sintaxis alternativa en fuentes activas, ningún adaptador
por texto ni flujo duplicado de tipos/propiedad, y conservación de hashes
de la base contra blobs de git. No bastan número de tests, demostraciones,
capturas de pantalla ni una ejecución de EX04. Las discrepancias descubiertas
en documentos normativos se registran con regla y comportamiento actual,
sin inventar contenido ni rebajar la prueba para cerrarlas.

La evidencia contribuye a T01/T02/T08/T10 (independencia, afinidad y duración),
T14 (efectos), T17/T18/T25 (propiedades y diagnóstico) y T22 (generadores).
T01–T28 conservan sus puertas completas: no acredita publicación D09,
CoW físico, actores, regiones, asincronía, backend nativo, REPL, biblioteca
general ni integración Python. D08 aquí es resolución nominal intrínseca,
no conformidad genérica de interfaces de usuario. EX04 completo mantiene
sus tipos/APIs de base fuera del perfil reducido.

F0 conserva dos revisores sin discrepancias y el resto de sus entregables;
F1 y fases posteriores no se aprueban por este corte. El dictamen independiente
puede cerrar requisitos de este ADR; la aprobación de fases pertenece al
propietario. Este documento especifica arquitectura pendiente de revisión,
sin atribuir implementación, dictamen ni publicación a sus expectativas.
