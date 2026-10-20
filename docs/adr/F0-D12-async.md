# F0-D12 — Async explícito, efectos por operación y cancelación estructurada

Estado: **iteración 6 propuesta; pendiente de nueva validación**.
La propuesta inicial y las iteraciones 1–5 recibieron **FAIL de Sonnet 5.5**,
según las devoluciones del coordinador. Esta iteración 6 atiende B1 y N1–N4;
conserva pendientes visibles y no afirma PASS.
Responsable: Codex, rol arquitecto. Fecha: 2026-10-19.
No acredita runtime, validación independiente ni aprobación de fase.

## Contexto y autoridad

[Acuerdos §3](../F0_ACUERDOS.md) pide decidir declaración async, inferencia,
creación/avance, cancelación y limpieza. Referencias del
[roadmap 0.2](../baseline/Nexo_ROADMAP_rev0.2.md): P02–P06/P09, D02/D07–D10,
F0/F1 (599–630), F5 (687–703), T06/T10/T14/T16/T18/T22 y §6.1–6.2.
[F0-0001](F0-0001-first-slice.md) no ejecuta estas operaciones.

Se toma de roadmap §2 la composición asíncrona, las tareas con ámbito y la
inferencia de efectos, conservando propiedad, afinidad y costes explícitos.
Un agente debe distinguir «creé trabajo», «lo ejecuté», «pedí cancelar» y
«ya puedo liberar sus dependencias». No son estados equivalentes.

No se propone modificar D10: yield síncrono no crea tarea/hilo; su suspensión
figura en next, incluso después del lowering. No hay **modificación semántica
propuesta** de esa regla ni adelanto del runtime F5.

### Ampliaciones sobre la base

Estas decisiones completan acuerdos §3 y las obligaciones de §6.1–6.2;
no se atribuyen retrospectivamente a D08/D10: async fn/closure, await y su
precedencia con consume/?, Future/TaskHandle/TaskOutcome y sus constructores,
grupo ligado a region, executor local/migrable, spawn/request_cancel,
cancelación cooperativa, cuatro operaciones y drop intrínsecamente no_suspend.
La lista with por operación usa exclusivamente la EBNF de D11 §5.2.
**SuspendSafe es una propiedad sellada nueva, ampliación sobre la lista de
D08**, con derivación y diagnóstico fijados en §9. La prohibición de retener
guardas a través de yield también es ampliación (§9), manteniendo el efecto
suspend exigido por D10. Son elecciones de este ADR, no soporte ya implementado.

## Decisión

### 1. Async explícito; suspensión inferida como efecto independiente

Se elige `async fn` para funciones y closures asíncronas. `await` solo aparece
en un cuerpo async, consume un futuro/handle de espera compatible y conserva
el contexto de procedencia. Un cuerpo ordinario no se convierte en async por
inferencia ni cambia su tipo de retorno al añadir una llamada.

```nexo
async fn identidad(consume x: i64) -> i64 {
    return x;
}
async fn componer() -> i64 {
    let pendiente = identidad(7);
    let n = await consume pendiente;
    return n;
}
```

La primera función es async aunque no contenga await. Su declaración decide
que la llamada construye trabajo diferido; su cuerpo puede tener resumen de
avance no_suspend. Todo await se clasifica conservadoramente con suspend,
aunque un futuro concreto esté ya listo. No se borra por optimización.

Un `fn` ordinario puede tener suspend al avanzar un generador: es suspensión
local D10, no permiso para usar await ni creación implícita de tarea. No se
aceptan `async gen fn` ni yield en un cuerpo async en 1.0.

**Completa lo pendiente de D10:** YieldStmt se reconoce dentro de Block antes
de comprobar el contexto. Solo se admite si el invocable léxico más próximo
es gen fn; una closure ordinaria anidada en gen no hereda ese permiso.
En un cuerpo ordinario se emite NX-E1213, con primary_span del token yield
y el invocable como ubicación relacionada. En async se conserva
NX-E1206/yield_in_async; async gen se rechaza primero en la declaración por
NX-E1206/async_generator_declaration. Añadir yield nunca infiere gen ni async.
A40/A41 y sus positivos concretan esta regla sin cambiar D10: next registra
suspend y no crea una tarea.

### 2. Futuro concreto, afín, inicialmente sin ejecución

Llamar `async fn f(...) -> T` produce un futuro propietario y no copiable cuyo
resultado final es T; no T inmediato. `Future<T>` designa esta familia nominal:
cada declaración conserva un estado concreto opaco y sus metadatos; no significa
boxing ni un layout único para todos los futuros de resultado T.

El llamador evalúa argumentos izquierda→derecha en orden escrito (D07/D10),
adquiere préstamos/copias/transferencias declaradas y construye el estado.
No ejecuta sentencias del cuerpo, registra E/S, lanza tareas ni invoca callbacks
del cuerpo. Los efectos de evaluar argumentos pertenecen al llamador además
de los de construcción. Copiar/mover un argumento sigue P02 y D11. Un temporal propietario recién
creado, como el literal de identidad(7), se entrega directamente a un parámetro
consume; para transferir un binding existente se escribe consume nombre.

Parámetros ordinarios conservan su modo prestado de D08: si el futuro los
retiene, su duración lo refleja. Obtener propiedad requiere parámetro consume
o preparación explícita antes de llamar, no copia oculta. Para escalares el
compilador puede materializar una copia sin cambiar la procedencia contractual
prometida; eliminar el préstamo como optimización no amplía usos admitidos.

Una closure `async fn [capturas](...) -> T` usa D11. Se decide inicialmente
que su invocación consume el entorno (`consume f(args)`) y lo transfiere al
futuro. Puede contener préstamos de lectura cuya procedencia siga vigente,
pero no fabricar un futuro que preste de un entorno ya destruido. Esta regla
evita inferir una familia adicional de futuros que presten del receptor.
Copiar antes una closure copiable permite varias invocaciones explícitas.

### 3. Await, handles y grupo asociado a region

**Completa lo pendiente de D12 y §6.2; ampliación sobre la base.** El operador
await recibe un valor propietario conforme a D11 §3.1. Un binding requiere
`await consume f`; `await f` intenta copiarlo y da NX-E0201 si no es copiable.
Una llamada que produce un futuro nuevo permite `await listo()`. Se retira
la transferencia implícita de `await f` del borrador anterior. Drop conserva
su sintaxis intrínseca consumidora explícitamente enumerada en D11.

La EBNF única de llamadas, prefijos y precedencia está en D13, gramática
G.1: AccessExpr incluye consume e inout; AwaitExpr espera su resultado y
TryExpr aplica ? después. OwnedPostfix se retira para no mantener una segunda
producción incompleta. Paréntesis admiten una expresión completa.

Así, `await e()?` es `(await e())?`, nunca `await (e()?)`.
`await (crear_fallable()?)` permite explícitamente propagar el error de
construir un futuro antes de esperarlo. `await consume f?` es
`(await consume f)?`. El formateador conserva estos árboles sin consultar tipos.
Si el operando es `consume f(args)`, consume el receptor de la llamada
conforme a D11 §5 y el futuro nuevo resultante es el operando propietario
de await; no se consume dos veces.
Otras capas de precedencia del lenguaje no convierten ? en parte del operando
de await sin los paréntesis escritos.

* `await` de Future<T> produce T por la ruta normal y no crea tarea hermana.
  Cancelación del contexto actual entra en su limpieza y no entrega un T parcial.
* `await` de TaskHandle<T> consume el handle, reúne esa hija y produce
  TaskOutcome<T>: Completed(T) o Cancelled. Cancelación de la hija puede ser
  observada como Cancelled; cancelación del padre no se captura por esta operación.
* TaskOutcome y TaskHandle son tipos nominales intrínsecos de esta ampliación;
  Completed/Cancelled se resuelven como sus constructores de núcleo. Un
  `TaskOutcome<Result<U,E>>` no es Result: `await consume h?` da NX-E1210.
  Se exige match; solo el payload de Completed puede recibir ?.

Se fija la sintaxis de la región y su grupo, conservando la forma de §6.2:

```text
RegionStmt = "region" Name "(" "capacity:" Expr
             ["," "executor:" ("local" | "migrable")] ")" Block
SpawnExpr  = RegionName ".spawn(" Expr ")"
```

Cada region es propietaria de un grupo de tareas con la misma duración; executor
omitido significa local, fijado al ejecutor actual. `region r(capacity: 0)`
permite un grupo sin almacenamiento regional de usuario; no promete que el
registro de tareas esté libre de reservas. Un grupo sin hijas no necesita
ejecutar reunión y sus costes se comprueban por su representación.

`r.spawn(consume f)` es la única operación de lanzamiento de este núcleo;
acepta un futuro propietario y devuelve TaskHandle<T> ligado a r. Un temporal
nuevo se entrega directamente: `r.spawn(trabajo())`. El grupo conserva el
registro operativo y la obligación de reunión; el handle es el derecho afín a
obtener el outcome una vez. Copiarlo está prohibido; transferirlo no lo desliga
del grupo. Ningún handle escapa de su región. No hay detach.

Spawn requiere cuerpo async y no avanza el cuerpo del futuro como parte de la
construcción previa: lo registra para ejecución con la política del grupo.
Su propia reserva/publicación y eventual reentrada del runtime tienen
contratos; no se equiparan a los de crear el futuro. Solo un controlador
avanza cada estado; reentrada/avance concurrente es un invariante del runtime,
no un nuevo diagnóstico estático para un programa que ya no puede expresarlo.

La salida de region, también return/?/cancelación, solicita cancelación de
hijas pendientes y las reúne antes de limpiar recursos y liberar almacenamiento.
Esa operación de control tiene los efectos cancel/join de §8, incluido suspend
cuando proceda. No es un destructor de recurso que introduzca await a escondidas.
Si una región puede deber reunión asíncrona, se rechaza bajo no_suspend por el
contrato escrito. Una region síncrona sin spawn/await sigue siendo posible.

La forma exacta de la base `region peticion(capacity: capacidad) { ... await
responder_prestado(salida)?; }` es válida en un cuerpo async: el futuro temporal
se transfiere directamente, primero se espera y luego se propaga su Result.
El grupo puede estar vacío; la espera directa mantiene viva la región.

Ejemplo fuente completo del operador de espera sobre handle:

```nexo
async fn trabajo() -> Result<i64, i64> { return Ok(7); }
async fn padre() -> Result<i64, i64> {
    region peticion(capacity: 0, executor: local) {
        let h = peticion.spawn(trabajo());
        let outcome = await consume h;
        return match consume outcome {
            Completed(consume r) => { value: consume r; },
            Cancelled => { value: Err(9); },
        };
    }
}
```

La conversión de Cancelled a Err(9) es una decisión explícita de esta aplicación,
no del lenguaje. Una alternativa explícita puede devolver TaskOutcome al
supervisor. El runtime F5 proveerá la entrada desde código síncrono con límites
de bloqueo/suspensión; no existe block_on implícito.

### 4. Contratos públicos; inferencia local acotada

Se infieren los cuatro resúmenes `create/resume/cancel/drop` de cuerpos locales
y funciones privadas dentro del módulo. Recursión usa un punto fijo conservador
sobre conjuntos de efectos finitos; agotar el presupuesto diagnostica sin
aceptar. Una firma pública async debe escribir los cuatro límites. Sintaxis
decidida para esa interfaz:

```nexo
pub async fn constante() -> i64
    with {
        create(no_alloc, no_block, no_suspend, no_io),
        resume(no_alloc, no_block, no_suspend, no_io),
        cancel(no_alloc, no_block, no_suspend, no_io),
        drop(no_alloc, no_block, no_suspend, no_io)
    } {
    return 7;
}
```

Dentro de cada cláusula, los contratos enumerados son prohibiciones; los no
enumerados quedan permitidos. `unrestricted` es la alternativa explícita para
un límite sin prohibiciones, nunca una lista vacía inferida. Una declaración
pública no infiere efectos a partir del cuerpo de dependencias. La propiedad
intrínseca de no suspensión de la destrucción sigue vigente aun cuando se
escriba unrestricted. Construcción del estado no suspende; argumentos pueden
hacerlo en el contexto que los evalúa.

Una anotación simple `with no_alloc, ...` sobre async limita resume; se admite
solo localmente. Es la alternativa simple de la única EBNF WithClause
de D11 §5.2. La lista por operación exige llaves también en fn: no se admite
`with no_alloc, create(...)` ni una segunda cláusula with. Las interfaces
públicas y los tipos Future<T> escriben las cuatro operaciones; falta de
cualquiera da NX-E1202. En un mapa async privado resume es obligatorio
(NX-E0417/missing_body_operation), los restantes omitidos son unrestricted.
Una firma ordinaria conserva su forma D08. Los tipos de llamadas indirectas,
futuros opacos, traits y exportaciones conservan límites, modo de acceso,
procedencia, afinidad y obligaciones de limpieza. Si falta información, el
efecto es desconocido, no vacío (§6.1).

### 5. Composición de efectos sin descontar costes retenidos

* create: evaluación propia de capturas/retención y almacenamiento del estado;
  no efectos del cuerpo. Pila fija no es alloc; boxing o almacenamiento dinámico
  sí lo es, según §6.1 y representación elegida.
* resume: todos los caminos del cuerpo, avances de futuros esperados,
  callbacks, temporales y limpieza normal/error.
* cancel: detener operaciones, esperar quiescencia y reunir hijos, más limpieza
  de cada estado alcanzable. Puede incluir suspend aunque drop no lo incluya.
* drop: destruir un estado inerte y sus campos aún propietarios. Incluye
  decremento final, liberación, bloqueo/E/S/reservas residuales pertinentes.

Retornar un futuro exporta esos resúmenes y obligaciones, no los ejecuta.
Crear y descartarlo sin empezar ejecuta create y drop, no resume.
Esperarlo incluye sus efectos de avance y salidas; el contexto cancelable
incluye también su protocolo cancel. Un contrato sobre una operación debe
cubrir todas sus salidas, no solo éxito. Un no_suspend de creación no certifica
avance ni cancelación. Un no_io del cuerpo no certifica la limpieza de handles.

La limpieza automática de estados retenidos tiene una prohibición intrínseca
de suspensión. Su violación usa **NX-E1211**, no NX-E0414: no necesita un contrato
with escrito. El catálogo distingue intrinsic_rule de contract_origin.
Un recurso que necesite cierre async proporciona una operación explícita que
se espera antes y una limpieza residual síncrona. No se inserta await en
destructores. Un destructor bloqueante sigue teniendo block: síncrono no
significa no_block.

La composición de una closure async con su futuro usa las dos capas C/Q y
la gramática escrita de D11 §5.2–5.3. C.call incluye Q.create y transferencia;
no incluye Q.resume. C.drop no limpia capturas ya transferidas a Q. Los
cuatro límites del futuro se transportan en su tipo de resultado indirecto.

### 6. Máquina de estados y propietario único

| Estado | Propiedad/operaciones | Salidas permitidas |
|---|---|---|
| Creado | Estado propietario; cuerpo nunca iniciado. | Primer avance, transferencia o descarte/cancelación sin empezar. |
| Ejecutando | Control exclusivo del ejecutor. | Suspendido, completando o cancelando en punto cooperativo. |
| Suspendido | Conserva campos vivos y registros de espera. | Reanudación exclusiva o cancelando. |
| Cancelando | Impide reanudar código de usuario; conserva dependencias. | Esperar retiro de retenciones/hijos; luego limpieza y cancelado. |
| Completando | Resultado inicializado aún no publicado; limpieza de locales. | Comprometer resultado y completar. |
| Completado / Cancelado | No hay trabajo ni retención operativa pendientes. | Extraer una vez el resultado/outcome y destruir handle. |
| Destruido | Ninguna propiedad restante en este estado. | Ninguna operación. |

Transferir cambia propietario, no ejecuta ni duplica. Flags por campo impiden
doble cierre tras inicialización parcial, transferencia al resultado o error.
Completar devuelve propiedad de T; no destruye campos que ya forman el resultado.
El futuro cancelado no entrega T parcial. La implementación debe conservar el
estado concreto y su procedencia hasta verificar lowering (D04).

### 7. Cancelación cooperativa, sin excepciones ni rollback

`r.request_cancel(h)` solicita cancelación al grupo r y presta h durante la
llamada: no consume el handle ni espera la terminación. Requiere que h
pertenezca a r; grupo distinto da NX-E1212 con ambas identidades y los spans
de spawn y request_cancel. No se selecciona el grupo a partir del tipo T.
Solicitar cancelación marca una petición idempotente. No es prueba de que una
operación haya terminado, ni permite liberar región/argumentos. Se observa
antes del primer avance, en entradas de reanudación, al llegar a await y en
puntos explícitos del runtime de cooperación. Un bucle sin puntos puede no
terminar; no se promete cancelación inmediata ni tiempo acotado (F5/§11).

Una petición observada impide continuar el cuerpo ordinario. El supervisor
obtiene `TaskOutcome<T> = Completed(T) | Cancelled`, separado de Result;
para T = Result<U,E> no se convierte Cancelled en Err(E) ni se propaga con ?.
No hay captura de excepciones de cancelación ni continuación de un estado
parcial. E/S ya realizada no se revierte.

Si completar ya comprometió su resultado, prevalece Completed. Si se observó
cancelación en un punto previo al compromiso, prevalece Cancelled y se limpia
el resultado parcial, si existe. El modelo fija ese orden; las intercalaciones
admitidas se prueban, no se promete un ganador temporal universal.

### 8. Cancelar, reunir y después liberar

El padre conserva la región, recursos y préstamos hasta retirar toda retención.
La salida de un grupo solicita cancelación a hijos activos cuando proceda y
los reúne antes de destruir dependencias. Orden inicial de cierre del grupo:
solicitar a todos por orden inverso de registro; reunir en ese mismo orden;
después destruir recursos del padre en orden inverso de inicialización.
Dentro de cada tarea se respeta el orden léxico D07; entre tareas en ejecución
normal no se promete un orden global de efectos.

Una espera extranjera/transporte debe definir cómo deja de retener memoria:
desregistro confirmado, finalización o transferencia a almacenamiento propietario
independiente. «Cancelación solicitada» o «respuesta encolada» no son confirmación
(§6.2). Mientras falte esa confirmación, el estado permanece Cancelando y el
padre no puede salir de su región. El protocolo cancel puede suspender; esa
espera es una operación explícita del grupo/await, no un destructor oculto.

Los handles de tareas no finalizadas permanecen en el registro propietario del
grupo. Descartar un handle solicita cancelación, pero el grupo sigue debiendo
reunirlo; no genera una tarea huérfana ni libera sus préstamos. Un futuro
creado y nunca avanzado se destruye inmediatamente sin ejecutar el cuerpo.
Un futuro ya entregado a await/spawn no sigue disponible para drop del llamador.

Si una API no ofrece prueba de retiro de retención, no admite préstamo regional
cancelable en código seguro; requiere adaptación propietaria o se rechaza.
Error recuperable, cancelación y retorno normal limpian exactamente una vez.
Panic/aborto conservan D02; no prometen limpieza ni aislamiento de tarea.

### 9. Qué puede sobrevivir a suspensión

Cada campo vivo al suspender debe tener propietario/procedencia vigente, afinidad
compatible y contrato sellado de retención segura en suspensión. No se equipara
esta propiedad, llamada aquí `SuspendSafe`, con copia, independencia o Send.
Es una **ampliación sobre la base D08**: su lista de propiedades selladas
(copia, independencia, transferencia y proyección disjunta) no contenía
SuspendSafe. Se deriva transitivamente; no hay impl ni @derive de usuario
que la afirme. Solo se exige para campos realmente retenidos en el punto
de suspensión.

**Resolución del nombre:** el preámbulo implícito del compilador expone
SuspendSafe en el espacio de tipos/traits de cada módulo, igual que Call*.
No requiere use ni dependencia declarada. Se resuelve por identidad nominal
intrínseca, no por coincidencia de texto. La búsqueda normal toma primero
declaraciones/imports del ámbito y, si no existe ese nombre, el preámbulo.
Un trait de usuario homónimo no adquiere la propiedad ni altera su derivación.
A39 no declara ni importa un homónimo: su ImplDecl resuelve el intrínseco y
falla por NX-E0110 antes de conformidad/derivación, nunca por NX-E0101.
Esta exposición del nombre es parte de la ampliación SuspendSafe, no una
palabra reservada adicional.

No conseguir derivarla da **NX-E1204**, tanto por un permiso
conocido incompatible como por contrato desconocido: property = SuspendSafe,
property_evidence = incompatible o unknown, y cause_chain identifica el campo
y su procedencia. No es una promesa no_suspend escrita ni un error NX-E0414.

La declaración manual de una propiedad sellada se rechaza por NX-E0110,
regla común también para Call*. Esta ampliación reabre T06/T10/T14/T17/T22/T25.

**Ampliación sobre la base, completa lo pendiente de D10/D12:** la regla
conservadora siguiente extiende a yield la comprobación de retención segura.
D10 ya fija el efecto suspend; esta prohibición adicional de permisos vivos
no estaba especificada por aquella regla y debe revisarse con T22.

En la regla inicial se rechaza cualquier préstamo mutable, guarda de exclusividad
o lock retenido a través de await/yield, incluso anidado. Se permiten recursos
propietarios con contrato de suspensión y préstamos de lectura cuya fuente
permanezca estable/viva durante toda la espera y cancelación. Una operación
que necesita mutar temporalmente obtiene la guarda después de reanudar y la
libera antes de suspender. Un propietario mutable sin aliases puede permanecer
en el estado; no es lo mismo que una guarda mutable activa.

Un préstamo a un local del frame puede sobrevivir solo si su procedencia sigue
válida y la representación no lo invalida al mover el estado. No se aceptan
referencias físicas autocontenidas a campos móviles: el primer modelo usa
rutas lógicas estables del frame; un backend debe fijar almacenamiento o
rechazar la representación antes de bajar a punteros. Preparación dinámica,
si la necesita, cuenta como alloc. No se atribuye no_alloc por usar async.

### 10. Aislamiento y generadores sin excepciones

Grupo local: ejecutor fijado al hilo, compatible con LocalRc/afinidad declarada.
Grupo migrable: exige propiedad/transferencia de todo el grafo conforme D09 y
ningún contador local accesible desde otro trabajador. Await no migra ni
publica automáticamente. Una instantánea publicada se prepara explícitamente;
su coste y sincronización se atribuyen a la frontera. Contador uno en la raíz
no prueba unicidad de hijos, débiles o callbacks (D09/T18).

Dos tareas pueden compartir un préstamo de lectura estable dentro del grupo;
no un permiso mutable duplicado. Compartición mutable requiere actor/mutex u
otro aislamiento declarado, y la guarda no atraviesa suspensión. Los grupos
se reúnen antes de destruir la región, también al propagar un error con ?.

D10 permanece literal en significado: gen fn es una máquina síncrona; crear
no ejecuta cuerpo; next incluye suspend por yield; no crea tarea/hilo; drop
limpia una vez aun sin comenzar. D13 §1.1 fija que -> T declara Item: el
final del cuerpo y `return;` terminan la secuencia, sin aplicar NX-E1317;
`return` con operando da NX-E1324. Una función/closure async que llame next
acumula ese efecto además del de await, sin confundir los dos mecanismos.

## Alternativas rechazadas

1. Inferir async al encontrar suspensión: confunde yield con planificación y
   cambia interfaces al editar cuerpos. Se infiere el efecto, no el tipo de llamada.
2. Ejecutar hasta el primer await al llamar: mezcla creación con efectos del
   cuerpo y complica cancelación de estados nunca empezados.
3. Lanzar una tarea con cada llamada/await: añade concurrencia, costes y afinidad
   no escritos. Solo spawn explícito crea una hija.
4. Cancelar destruyendo inmediatamente el frame: invalida préstamos retenidos
   por hijos o transportes.
5. Destructores que esperan implícitamente: esconden suspensión; se separa
   protocolo cancel/reunión y destrucción de estado inerte.
6. Usar Result como canal universal de cancelación o aislamiento de panic:
   contradice la separación D02 y su política de aborto.
7. Borrar yield del resumen tras bajar a máquina de estados: reabre D10 y
   falsea no_suspend; se rechaza expresamente.

## Contraejemplos y tabla PASS/FAIL

Resultados previstos sobre la gramática de esta iteración, no pruebas ejecutadas.
Para las filas marcadas «sentencias async», la fuente se inserta en
`async fn caso() -> () { ... return (); }`; para «sentencias sync», en
`fn main() { ... }`. Se adjuntan estas declaraciones de núcleo a las que las usen:

```nexo
async fn listo() -> i64 { return 1; }
async fn leer(x: i64) -> i64 from(x) { return x; }
gen fn uno() -> i64 { yield 1; }
```

Mock_open/inspect/drop conservan los contratos de F0-0001. Una fila con
funciones completas sustituye la plantilla. No se inventan APIs de transporte,
LocalRc o destructores para simular que un caso pendiente tiene fuente válida.

| ID | Fuente concreta o pendiente | Resultado previsto | Diagnóstico / observación |
|---|---|---|---|
| A01 | Sync: `let q = listo(); drop(q);` | PASS | —; cuerpo cero veces. |
| A02 | `fn caso() -> i64 { let n = await listo(); return n; }` | FAIL | NX-E1201. |
| A03 | `async fn caso() -> i64 with no_suspend { return 1; }` | PASS | —; resume no_suspend. |
| A04 | `async fn caso() -> i64 with no_suspend { return await listo(); }` | FAIL | NX-E0414, contrato explícito sobre resume. |
| A05 | `pub async fn caso() -> i64 { return 1; }` | FAIL | NX-E1202. |
| A06 | Sync: `let a = listo(); let b = listo(); drop(a); drop(b);` | PASS | —; dos estados, ninguna tarea/cuerpo ejecutado. |
| A07 | Sync: `let f = listo(); let copia = f;` | FAIL | NX-E0201; nombre en inicializador intenta copiar. |
| A08 | Async: `let f = listo(); await consume f; await consume f;` | FAIL | NX-E0202/definitely_consumed. |
| A09 | Falta arnés que fuerce reentrada/avance simultáneo interno. | NO EVALUADO | Invariante de modelo; **sin código de compilador**. |
| A10 | Async: `var x = 0; var guarda = fn [inout x]() { x = 1; return (); }; let n = await listo(); inout guarda();` | FAIL | NX-E1204; property = SuspendSafe, cause_chain = frame → guarda → préstamo inout x. |
| A11 | Async: `var x = 0; var guarda = fn [inout x]() { x = 1; return (); }; inout guarda(); drop(guarda); let n = await listo();` | PASS | —; permiso termina antes, no hay campo retenido cuya propiedad SuspendSafe deba derivarse. |
| A12 | `async fn caso() -> i64 { region r(capacity: 0) { let x = 1; let n = await leer(x); return n; } }` | PASS | —; préstamo de lectura y región vivos durante la espera. |
| A13 | `fn fabrica() { let x = 1; return leer(x); } fn main() { let q = fabrica(); }` | FAIL | NX-E1205. |
| A14 | `fn fabrica(b: bool) { let x = 1; if b { return Err(leer(x)); } return Ok(()); } fn main() { let r = fabrica(true); }` | FAIL | NX-E1205; tipo Result<(), futuro> inferible. |
| A15a | `async gen fn caso() -> i64 { yield 1; }` | FAIL | NX-E1206/async_generator_declaration; sintaxis deliberadamente excluida. |
| A15b | `async fn caso() -> i64 { yield 1; return 1; }` | FAIL | NX-E1206/yield_in_async. |
| A16 | `fn caso() with no_suspend { var g = uno(); let n = Iterator.next(inout g); }` | FAIL | NX-E0414, next → yield. |
| A17 | `fn caso() with no_suspend { let g = uno(); drop(g); }` | PASS | —; no avance. |
| A18 | Sync: `let r = mock_open(1); let f = async fn [consume r]() { inspect(r); return (); }; let q = consume f(); drop(q);` | PASS | —; cierre una vez sin empezar cuerpo. |
| A19 | Falta fixture de transporte con confirmación de retiro diferida. | NO EVALUADO | Objetivo: región viva hasta confirmación/reunión y un cierre. |
| A20 | Falta fixture de transporte sin protocolo de retiro. | NO EVALUADO | Objetivo negativo NX-E1208; no parse error sustituto. |
| A21 | Falta arnés de transición que libere región antes de reunión. | NO EVALUADO | Invariante del modelo; **sin código de compilador**. La sintaxis segura reúne automáticamente. |
| A22 | Falta fixture de futuro con destructor bloqueante bajo no_block. | NO EVALUADO | Objetivo negativo NX-E0413. |
| A23 | Falta fixture de boxing con representación y reserva declaradas. | NO EVALUADO | Objetivo negativo NX-E0412 en create. |
| A24 | Falta fixture LocalRc anidado enviado a grupo migrable. | NO EVALUADO | Objetivo negativo NX-E1109, entrada única del catálogo. |
| A25 | Falta positivo pareado de A24 en grupo local compatible. | NO EVALUADO | Objetivo: conservar afinidad y no migrar. |
| A26 | Fuente A26 abajo. | FAIL | NX-E1210: await de handle devuelve TaskOutcome, no Result. |
| A27 | Falta arnés de dos peticiones de cancelación tras resultado comprometido. | NO EVALUADO | Objetivo: idempotencia y Completed; sin diagnóstico estático. |
| A28 | `async fn caso() with no_suspend { region r(capacity: 0) { let h = r.spawn(listo()); return (); } }` | FAIL | NX-E0414: salida de región puede reunir/suspender bajo contrato explícito. |
| A29 | Async: `let r = mock_open(1); let f = async fn [consume r]() { inspect(r); return (); }; let q = consume f(); await consume q;` | PASS | —; transferencia a futuro y limpieza una vez. |
| A30 | Falta fixture de recurso cuyo drop suspende, retenido en futuro sin with escrito. | NO EVALUADO | Objetivo negativo **NX-E1211**, violación intrínseca, nunca NX-E0414. |
| A31 | Fuente A31 abajo: guarda local viva en yield. | FAIL | NX-E1204, suspension_kind = yield. |
| A32 | Fuente A32 abajo: liberar guarda antes de yield. | PASS | —; positivo pareado de la ampliación D10. |
| A33 | Sync: `let f = listo(); let g = consume f; drop(g);` | PASS | —; transferencia escrita, sin copiar futuro. |
| A34 | Async: `let f = listo(); await f;` | FAIL | NX-E0201; la nueva regla exige consume para binding no copiable. |
| A26b | Fuente A26b abajo; mismo listo, grupo r y TaskHandle<i64> de A26. | PASS | —; se elimina ? sobre TaskOutcome y se tratan explícitamente Completed(n) y Cancelled. |
| A35 | Fuente A35 abajo; cancelación desde el grupo ajeno s. | FAIL | NX-E1212; actual_group = r, requested_group = s. |
| A35b | A35 sustituyendo solo `s.request_cancel(h)` por `r.request_cancel(h)`. | PASS | —; petición prestada al grupo propietario; no asegura qué outcome gana. |
| A36 | `async fn caso() -> i64 with { create(no_io) } { return 1; }` | FAIL | NX-E0417/missing_body_operation; falta resume local. |
| A37 | Función constante de §4. | PASS | —; cuatro operaciones con llaves. |
| A39 | `struct Dato {} impl SuspendSafe for Dato {} fn main() { return (); }` | FAIL | NX-E0110; nombre intrínseco resoluble, afirmación manual de propiedad sellada prohibida. |
| A39b | A39 eliminando `impl SuspendSafe for Dato {}`. | PASS | —; el compilador deriva según campos, no por declaración de usuario. |
| A40 | `fn caso() { yield 1; }` | FAIL | NX-E1213; YieldStmt reconocido, token yield con primary_span [12,17), invocable ordinario caso. |
| A40b | A40 añadiendo solo gen antes de fn. | PASS | —; Item i64 inferible de yield 1 según D13 G.3; final normal de generador. |
| A41 | `gen fn caso() -> i64 { let f = fn []() { yield 1; }; yield 2; }` | FAIL | NX-E1213; primer yield, primary_span [41,46), invocable más próximo closure ordinaria; el gen exterior no concede permiso. |
| A41b | A41 reemplazando solo yield 1; por return (); dentro de la closure. | PASS | —; el único yield restante pertenece al gen exterior. |

Fuente A26 (listo es la función común de esta tabla):

```nexo
async fn caso() -> Result<i64, i64> {
    region r(capacity: 0) {
        let h = r.spawn(listo());
        let n = await consume h?;
        return Ok(n);
    }
}
```

Fuente A26b, positivo editado de A26: se conserva la creación/espera del
mismo handle y el tipo de resultado; se sustituye únicamente el uso de su
outcome por una selección explícita. No cambia listo a Future<Result<...>>.

```nexo
async fn caso() -> Result<i64, i64> {
    region r(capacity: 0) {
        let h = r.spawn(listo());
        let outcome = await consume h;
        return match consume outcome {
            Completed(n) => { value: Ok(n); },
            Cancelled => { value: Err(9); },
        };
    }
}
```


Fuente A31; préstamo creado dentro del frame, sin capturas externas que
provoquen antes el rechazo F2 de D10:

```nexo
gen fn caso() -> i64 {
    var x = 0;
    var guarda = fn [inout x]() { x = 1; return (); };
    yield 1;
    inout guarda();
}
fn main() { var g = caso(); let n = Iterator.next(inout g); }
```

Fuente A32:

```nexo
gen fn caso() -> i64 {
    var x = 0;
    var guarda = fn [inout x]() { x = 1; return (); };
    inout guarda();
    drop(guarda);
    yield x;
}
fn main() { var g = caso(); let n = Iterator.next(inout g); }
```

Fuente A35 (declaración listo de la plantilla). No hay escape, préstamo
mutable ni segundo await que oculten el error de identidad del grupo:

```nexo
async fn caso() -> () {
    region r(capacity: 0) {
        let h = r.spawn(listo());
        region s(capacity: 0) {
            s.request_cancel(h);
        }
        let outcome = await consume h;
        match consume outcome {
            Completed(n) => { return (); },
            Cancelled => { return (); },
        }
    }
}
```

A09/A21 dejan de atribuir al compilador errores internos que no tienen fuente.
Sus invariantes siguen obligatorios. Los casos NO EVALUADO necesitan fixtures
o arneses, además de ejecución en su fase. El modelo deberá inyectar cancelación
antes/después de registro, suspensión y compromiso de resultado, junto con
error ?, adquisición parcial y un hijo que conserve la región. A16/A17 y
A31/A32 mantienen explícitamente las obligaciones de T22.

## Diagnósticos y reparación

El único registro de códigos, propietarios, subcausas, procedencia y numeración
es [F0-CODIGOS](F0-CODIGOS.md). Los casos referencian sus entradas. En particular:
NX-E0201/NX-E0202 son propiedad común; NX-E1109 es compartido con D11/D13;
NX-E1202 pertenece a la firma pública async; NX-E1211 es la prohibición
intrínseca de drop. NX-E0412 es el ejemplo de la base y NX-E0413–NX-E0416
son propuestos, todos para contratos declarados con contract_origin.

El esquema estructurado añade punto de suspensión, campos vivos, grupo
propietario, origen de retención, operación y confirmación pendiente. No se
reescribe la causa como efecto vacío ni se inventa un with para NX-E1211.

Reparaciones candidatas: terminar una guarda antes de await/yield; conservar
el grupo dentro de la región; preparar un propietario independiente; ejecutar
en grupo local si satisface la aplicación. Copiar, cambiar política o cancelar
trabajo cambia comportamiento/coste y requiere elección explícita. No se
ofrece quitar no_suspend ni añadir unsafe. Una reparación anunciada aplicable
se recomprueba, incluido el protocolo de cancelación y su positivo pareado.

## Interacciones y alcance por fase

* D11: modos explícitos y propiedades de cada captura; invocación consumidora
  de closure async, procedencia transitiva y destructor del entorno.
* D13: return obligatorio, unión de efectos en ramas y firma pública que
  conserva los cuatro resúmenes incluso al reexportar.
* D08: llamadas indirectas/traits con límites conocidos, no conformidad
  estructural ni efectos concedidos por un nombre.
* D09 y §6.2: publicación real, afinidad, retención hasta reunión y limpieza
  antes de liberar almacenamiento regional.
* D10: generador síncrono propietario, next con suspend, ningún task oculto.

**Prototipo F0 previsto, no realizado aquí:** reconocer declaraciones y efectos,
modelo determinista de futuros fríos y de la máquina de estados con recursos
mock, uno/dos ejecutores simbólicos y transporte que confirma retiro en pasos
enumerados. Explorar cancelación, flags de propiedad, ramas y procedencia.
No utilizar timers/hilos/red reales ni presentar el simulador como runtime.
Un caso fuera del modelo se marca NO EVALUADO, no PASS por falta de ejecución.

F1 implementa las bases de tipos/IR/préstamos/efectos necesarias sin exigir
scheduler; F2 preserva D10 y sus generadores síncronos; F3 integra regiones y
resúmenes transitivos; F5 implementa runtime, grupos, publicación, cancelación
real y pruebas de plataformas. No se promueven fases por tener un modelo F0.

| Pruebas T | Evidencia requerida |
|---|---|
| T06/T10 | Capturas anidadas, escapes por futuro/error/tarea, alias mutable, región viva durante cancelación. |
| T08/T14 | Error ?, inicialización parcial, cada estado limpio una vez, efectos de create/resume/cancel/drop y llamadas indirectas. |
| T16/T18 | Publicación, afinidad, intercalaciones de cancelación, último RC y retención sin tareas huérfanas; ejecución real F5. |
| T22 | Yield síncrono visible, construcción perezosa, generador sin comenzar y salida temprana. |
| T09/T17/T25 | Cambio de efecto/afinidad invalida interfaz; traits compatibles; causa, ubicaciones y reparación. |
| T28 | Futuras comprobaciones de atomics, ABI y runtime en cuatro plataformas; no acreditadas por modelo. |

## Riesgos abiertos y validación pendiente

La prohibición conservadora de guardas a través de toda suspensión puede
rechazar algoritmos seguros. Su relajación necesitaría evidencia específica,
sin ocultar yield ni confundir dominio local y tarea. Las rutas lógicas del
modelo no demuestran estabilidad de direcciones del futuro backend.

El protocolo de cancelación externa y su latencia son el punto de mayor riesgo:
esperar quiescencia puede ser lento o no terminar. Se preserva memoria, pero
no se promete progreso universal. Los adaptadores reales y sus supuestos
permanecen en la base de confianza P07; se rechaza préstamo seguro sin contrato.

Cuatro límites públicos pueden resultar verbosos; antes de abreviarlos se
evaluará si las herramientas explican bien cada operación y si una abreviatura
conserva las obligaciones. El formato binario/ABI de metadatos no se decide aquí,
pero no puede perder límites ni procedencia.

Las seis revisiones anteriores fueron **FAIL**. Esta iteración 6 no tiene dictamen:
sus cambios y casos previstos están **NO EVALUADOS** por validación nueva.
Siguen pendientes las fixtures señaladas, los dos revisores sin discrepancias
de F0 y la aprobación del propietario. No se aprueba una fase por estas tablas.

## Registro de corrección — iteración 1

Motivos 2/3/10: catálogo único, NX-E0202 compartido y NX-E1211 intrínseco,
con procedencia de NX-E0412 separada de códigos propuestos. Motivos 4/5/9:
composición con C.call, transferencia escrita del operando await, precedencia
con ? y grupo ligado a region con TaskOutcome. Motivo 6: restricción en yield
declarada ampliación y par A31/A32. Motivo 8: transiciones de modelo y fixtures
externas NO EVALUADAS sin diagnósticos de compilador ficticios.

## Registro de corrección — iteración 2

Motivo 2: cancelación de handle ajeno, NX-E1212 y A35/A35b. Motivo 4:
SuspendSafe se declara ampliación sellada D08, derivación fallida NX-E1204
(A10/A11; A38/A38b eran duplicados históricos) y afirmación manual NX-E0110 (A39/A39b). Motivo 5: encabezado de
máquina de estados reparado. Motivo 6: todas las operaciones with usan llaves
y remiten a una EBNF, incluidas las anotaciones de closure async. Observaciones:
A26b concreta el positivo TaskOutcome/match; la referencia NX-E1210 apunta a §3.
No se acredita runtime F5 ni un nuevo dictamen de validación.

## Registro de corrección — iteración 3

B1: propiedad SuspendSafe reescrita con saltos y párrafos reales. B2: el tipo
de gen es Item; el final de su cuerpo no activa NX-E1317. C22/A16/A31/A32/E18
siguen exigiendo sus diagnósticos de efectos o suspensión, no retorno.
N3 sustituye OwnedPostfix por referencia a la gramática única AccessExpr de
D13 G.1. N5 elimina A38/A38b como filas duplicadas; su contenido se integra
en A10/A11, sin contar cobertura adicional. Los IDs retirados no se reutilizan.
No se afirma ejecución de runtime ni nuevo dictamen.

## Registro de corrección — iteración 4

N3: A26b conserva el TaskHandle<i64> de A26; cambia el uso inválido de ?
por match de Completed/Cancelled con resultado Result<i64, i64>.
N4 restaura la grafía `return;` del generador. B1/B2 y N1/N2 se trazan
en la gramática/inventario de D13 y en el catálogo compartido. Las reglas
solo NO EVALUADAS de N5 siguen pendientes; no se adelanta runtime F5.

## Registro de corrección — iteración 5

B1 migra los match de TaskOutcome a value:, sin reservar el identificador value
ni cambiar cancelación o retención. La elección **corrige choque con EX01/EX04**.
N2 fija SuspendSafe en el preámbulo implícito, su resolución nominal y el motivo
por el que A39 alcanza NX-E0110. Las funciones async con nombre conservan la
regla de valores de función de D11 §5.1; la closure async sigue siendo CallOnce.
No se implementa runtime ni se acredita nueva validación.

## Registro de corrección — iteración 6

N3: NX-E1213 y A40/A41 fijan yield en cuerpo ordinario y closure anidada:
el permiso pertenece al invocable léxico más próximo. Los positivos mantienen
la generación síncrona de D10 y no crean tareas. Async conserva NX-E1206,
sin duplicar el rechazo del modificador async gen. Se completa explícitamente
lo pendiente de D10. B1/N1/N2/N4 y las otras cotas se trazan en D13/D11/catálogo.
La comprobación de SuspendSafe y los casos NO EVALUADOS siguen pendientes;
no se implementa runtime F5 ni se acredita un nuevo dictamen.
