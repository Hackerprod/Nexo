# F0-D13 — Expresiones de control, módulos y contratos de paquete

Estado: **iteración 6 propuesta; pendiente de nueva validación**.
La propuesta inicial y las iteraciones 1–5 recibieron **FAIL de Sonnet 5.5**,
según las devoluciones del coordinador. Esta iteración 6 atiende B1 y N1–N4;
conserva pendientes visibles y no afirma PASS.
Responsable: Codex, rol arquitecto. Fecha: 2026-10-19.
No acredita implementación, validación independiente ni aprobación de fase.

## Contexto y autoridad

[Acuerdos §4](../F0_ACUERDOS.md) pide decidir valores de if/match, tipos,
exhaustividad, efectos, movimientos y visibilidad/importación. Autoridad:
[roadmap 0.2](../baseline/Nexo_ROADMAP_rev0.2.md), P01–P09,
D01 (362–370), D07/D08/D09/D10, F0/F1, T01–T28 y §6.1–6.2.

D01 exige return explícito y excluye expresión final implícita en 1.0.
D08 fija traits nominales, coherencia del grafo y extensiones activadas
explícitamente. D07 hace públicos los tipos y efectos prometidos. Se integran
las influencias de §2: alternativas e inferencia de ML, módulos/herramientas,
valores y propiedad, sin añadir coerciones ni resolución por orden de imports.

[F0-0001](F0-0001-first-slice.md) todavía no implementa estas construcciones.
Este ADR complementa lo pendiente; no contiene **modificación semántica
propuesta** de D01. Introduce un marcador explícito de valor de rama, no una
excepción al retorno de funciones.

### Ampliaciones respecto a la base, explícitas en esta propuesta

* **Completa lo pendiente de D01:** regla léxica parcial de salto de línea
  inmediatamente después de return/value: (§2). No se atribuye a la base ni
  se declara resuelta toda su inserción opcional de terminadores.
* **Ampliación sobre la base, completa acuerdos §4/D13:** if/match con valor
  y marcador contextual value: (§1), con value como Name ordinario.
  No se reintroduce retorno final implícito;
  return sigue siendo obligatorio para resultados de función/closure.
* **Completa lo pendiente de D08/§6.1:** firmas invocables y cotas escritas
  conforme a D11 §5.2, incluidos parámetros genéricos y dyn explícito.
* **Ampliación sobre la base:** dominio de privacidad de campos y gramática
  de procedencia/retención; deben contrastarse con T10/T17/T20/T25.

* **Completa lo pendiente de D01/D03/D08 y acuerdos §4:** las producciones const/comptime y type
  como declaraciones de módulo (§8); mod/use/pub use, raíces self/super/package,
  activación use extension y calificación de método (§§8–10); selección única
  de revisión de paquete e invalidación por contratos (§§11–12). Sus grafías,
  privacidad y restricciones son decisiones nuevas de esta propuesta.
* **Ampliación sobre la base:** from/result_from/retains son marcadores
  contextuales de procedencia; value no se reserva: value: es el marcador
  contextual de §1 que **corrige choque con EX01/EX04**. No hay retorno
  implícito ni inferencia global de duración;
  Never es un tipo interno de análisis sin nueva grafía de usuario.


### Inventario transversal de sintaxis y reglas añadidas

Este inventario cubre G, las EBNF de D11 §5.2/D12 §3 y las formas usadas en
las matrices. «Completa» fija una grafía o regla que la base dejó pendiente;
«ampliación» añade una elección que no se atribuye a la rev 0.2. Ninguna entrada
es una afirmación de implementación.

| Familia / formas | Relación con la base y decisión que las define |
|---|---|
| Resta -, signo de entero y += | **Completa D01/D07** para V01/V02: resta asociativa a izquierda, signo separado sin dependencia de espacios y aritmética comprobada. += se toma de D08; G.1/G.2 concretan las producciones reducidas. |
| Llaves, fn/let/var/struct, return explícito, comentarios, rutas con punto, tipos, etiquetas/defaults y modos de parámetros inout/consume/self | Ya planteados en D01/D07/D08/D10; G concreta sus producciones y la inferencia solo privada (§9). Los literales i64/bool, +, arrays e índices del corte proceden también de F0-0001. |
| if/else y match; value:; brazos con => y bloque; Never interno | **Ampliación sobre la base** para acuerdos §4: ValueStmt contextual y final; selector Name(value) + : con un token de anticipación; no reserva value y **corrige choque con EX01/EX04**. Unificación exacta sin retorno implícito (§§1–4). |
| Guardas `Patron if condicion`; patrones `_`, literales, binding, constructor anidado y producto; `Some(inout n)`, `Some(consume p)`; `match inout x`, `match consume x` | **Ampliación sobre la base**: D01 y §2 proponen patrones/exhaustividad, pero no deciden estas formas, permisos ni el orden guarda/transferencia. §4/G.2 fijan lectura por defecto, guardas sin efectos/mutación/transferencia y cobertura por patrones previos sin guarda. |
| Capturas [borrow/copy/inout/consume nombre], fn anónima larga/corta, => return, prefijos inout/consume sobre receptor de llamada | **Ampliación sobre la base** de D11 §§1–5/G.1–G.2. => expr y el terminador ausente se reconocen solo para diagnóstico, nunca como alternativas aceptadas. G.1 distingue método x.m() de campo invocable (x.m)() y conserva Place entre paréntesis. |
| Call/CallMut/CallOnce sellados, dyn (CallableBound), parámetros nombrados en cotas, with { operación(restricciones) }, unrestricted | **Ampliación sobre la base** de D11 §5.2: concreta §6.1 y amplía D08. create/copy/call/resume/cancel/drop son nombres contextuales de operación. With simple y los cinco no_* proceden de §6.1; no hay operadores de efectos nuevos. Las formas sin with, mapa vacío y unrestricted combinado se reconocen solo para diagnóstico; no se aceptan. |
| from, result_from, retains(..., in: ...); propagación por genéricos, receptores y campos | **Ampliación sobre la base**, §6/G.1 y D12: gramática y obligaciones locales para la procedencia pendiente de P03/D08/§6.2. |
| async fn/closure; await y agrupación con consume/?; Future, TaskHandle, TaskOutcome, Completed/Cancelled; SuspendSafe sellado | **Ampliación sobre la base** de D12: concreta la composición anunciada en §2/F5, sin adelantar runtime. ? ya existe en D02; aquí se decide su precedencia con await y su exclusión de TaskOutcome. |
| region con capacity y executor: local/migrable; spawn/request_cancel; reunión y cancelación del grupo; drop no_suspend intrínseco | **Ampliación sobre la base** de D12 §3–§9 para concretar la región de §6.2 y tareas de F5. Son contextos/APIs intrínsecas; no importan permiso de suspender a código síncrono. |
| gen fn, yield y fin de generador | D10 ya fija generación síncrona. §1.1 **completa lo pendiente de D10**: caída normal y return vacío opcional; return con operando reconocido y rechazado. Yield requiere que el invocable más próximo sea gen; NX-E1213 cubre el cuerpo ordinario. |
| Terminadores insertados de sentencias, use/mod/type/const y requisitos de trait; recuperación MissingTerminator | **Completa lo pendiente de D01**, §2–§2.1/G: saltos tras return/value:, continuación y declaraciones sin ; compatibles con EX01/D08. La recuperación no amplía programas aceptados. |
| pub, mod, use/pub use, as, grupos de imports, self/super/package, type, const = comptime; visibilidad de campos/variantes; rutas y grafo de paquetes | **Ampliación sobre la base** en las concreciones de §§8–12/G.3. Use con punto y comptime ya existen en D01/D03/EX01; la declaración global pura, alias, privacidad, revisión única e invalidación se deciden aquí. No hay inicialización global ejecutable. |
| trait/impl/for, extend, dyn, where, T: Trait y requisitos múltiples con +; enum y constructores con payload | D08/§2 anuncian estas capacidades. **Ampliación sobre la base** en sus producciones concretas G.3 (incluido + entre cotas, distinto de suma), miembros, variantes, tipos nominales y reglas locales de resolución. |
| use extension y calificación extension ruta.Tipo.metodo(...) | **Ampliación sobre la base** de §10/G.1/G.3 que concreta activación y desambiguación de D08, sin conformidades implícitas. |
| (), agrupación, argumentos/constructores con nombre; ->, =>, :, =, ., [], <>, coma, ; y ? | Unidad, anotaciones, genéricos y argumentos ya se ilustran en la base; G fija las combinaciones usadas aquí. => pertenece a las ampliaciones de brazos/closures anteriores; + entre requisitos queda declarado en la fila de traits. No se añaden coerciones ni símbolos sobrecargables por paquete. |

Option/Result/Ok/Err/Some/None son tipos y constructores del preámbulo de la
base, no palabras reservadas nuevas. Iterator.next procede de D10. Los nombres
mock_open/inspect/drop/assert_eq/print/fail del arnés conservan el carácter
experimental de F0-0001; no declaran biblioteca de producción. Otros nombres
de las fixtures se declaran en sus fuentes o identifican obligaciones todavía
NO EVALUADAS, no primitivas implícitas añadidas.

## Decisión

### 1. Bloques de sentencias; valores de rama explícitos

Los bloques ordinarios contienen sentencias y no adquieren el valor de su
última expresión. Funciones/closures ordinarias o async que producen un
resultado final T distinto de unidad deben
retornar explícitamente en todo camino normal; una caída normal sin return
produce NX-E1317, aunque el programa sea sintácticamente válido. Para unidad
se permite terminar el bloque. Una expresión descartada necesita su terminador conforme D01,
sin que el tipo esperado convierta ese descarte en retorno.

If y match se admiten como expresiones con bloques de rama delimitados. Se
introduce `value: expresión;` para entregar el valor de esa rama:

```nexo
fn elegir(b: bool) -> i64 {
    let n = if b {
        value: 1;
    } else {
        value: 2;
    };
    return n;
}
fn desempaquetar(x: Option<i64>) -> i64 {
    return match x {
        Some(n) => { value: n; },
        None => { value: 0; },
    };
}
```

`value` es un **identificador ordinario**, nunca un token reservado. Se elige
la opción (a) del juez con `value: expresión;`: el separador : es obligatorio.
La antigua forma `value Expr` queda retirada. Esta **ampliación sobre la base
corrige choque con EX01/EX04**, sin reescribir esos ejemplos.

1. Solo al comenzar una sentencia, Name con lexema exacto value seguido del
   token : **en la misma línea** selecciona ValueStmt. El selector consulta
   únicamente ese token siguiente; ignora espacios horizontales, pero no LF.
   En cualquier otro caso selecciona las producciones ordinarias de Name.
   No consulta bindings, tipos, copiabilidad ni resultado esperado.
2. ValueStmt solo es válido como **última sentencia directa** de un bloque
   de rama de if/match usado como expresión. No salta desde un bloque, if de
   sentencia, función/closure o bucle anidado a una rama exterior. Su operando
   puede ser otro if/match expresión. Los caminos que retornan o divergen no
   necesitan ValueStmt; los demás terminan con uno.
3. La forma value: se reconoce fuera de ese lugar para diagnosticar: sin
   bloque destinatario directo da NX-E1302/invalid_value_target; con él pero
   con otra sentencia después da NX-E1302/non_final_branch_value. El span
   primario es el introductor value: (ambos tokens); la sentencia posterior
   es ubicación relacionada. Prioridad para la misma causa: invalid_value_target,
   missing_value_operand, non_final_branch_value y después errores de flujo
   derivados. Comprobar posición usa el bloque ya analizado, no anticipación
   adicional para clasificar value.
4. value seguido de -, (, [, =, +=, ., ? o terminador sigue siendo Name:
   resta, llamada, índice, asignación, acceso, propagación o descarte ordinarios.
   `value - 1` y `value -1` tienen idénticos tokens/AST. Un binding value
   visible no cambia la selección; en `value: value;` el primero introduce
   la sentencia contextual y el segundo es el identificador ordinario.
5. Un salto después de **value sin :** sigue la continuación ordinaria de
   expresiones de §2: puede acabar el descarte o continuar una llamada/índice.
   Un salto después de **value:** termina un ValueStmt vacío según §2.
   Separar value y : por LF no forma el introductor.
6. Los nombres value en let/var, parámetros, funciones, tipos, campos, rutas,
   imports, capturas, patrones, variables de for, etiquetas e interpolaciones
   son Name ordinarios. `let value: i64 = 1;` es BindingStmt, no ValueStmt:
   el selector solo actúa al inicio de una sentencia, nunca dentro de
   Field/ParamDecl/Argument/Pattern.

La salida por value: evalúa el operando, transfiere/copia al resultado y limpia
temporales/locales de la rama antes de continuar. No retorna de una función,
no crea un generador y no suspende. Return abandona la función o closure más
cercana, con su limpieza, incluso dentro de una rama.

La gramática distingue posición de sentencia y de operando sin consultar tipos.
Un if/match como sentencia no es destinatario de value: y puede contener
return/continue/break. Un if/match en un inicializador, argumento, operando de
return o entre paréntesis es expresión. Para descartar una expresión de
control se escribe entre paréntesis y con terminador. Un Name value
descartado al final de una rama no entrega valor ni se convierte en marcador.

### 1.1. Terminación de un generador

**Completa lo pendiente de D10**, compatible con EX04 (roadmap líneas
210–216): en `gen fn g(...) -> T`, T es **Item**, el tipo de cada elemento
entregado por yield; no es el resultado de un return del cuerpo. La llamada
construye el estado de generador y next produce Option<T> según D10.

1. Llegar al final del cuerpo es terminación normal de la secuencia, también
   sin haber ejecutado yield. No exige return y **no aplica NX-E1317**.
2. `return;` sin operando está permitido, nunca es obligatorio: finaliza
   anticipadamente sin entregar otro elemento. Ejecuta la limpieza pendiente
   una vez, como la terminación al final.
3. `return expresión;` está prohibido en el cuerpo generador, incluso
   `return ();`: NX-E1324, span del return y referencia al -> T de elemento.
   No hay un segundo canal de resultado final. Entregar T requiere
   `yield expresión;`; finalizar después puede escribirse `return;`.
4. El contexto es el cuerpo más cercano: una closure ordinaria anidada en un
   generador sigue las reglas de retorno ordinario. Async conserva su resultado
   final T y no adquiere esta exención.
5. Yield sigue siendo suspensión síncrona en next. Esta aclaración no oculta
   suspend, no crea tarea y no levanta restricciones de préstamos de D10/D12.

Permitir return vacío y rechazar un valor final son elecciones explícitas de
esta propuesta para el punto que D10 no escribía; no se presentan como texto
literal de la base. EX04 ya exige admitir caída al final sin return. D01 sigue
exigiendo return explícito para resultados de funciones ordinarias/async;
yield no es una expresión final implícita.

### 2. Forma corta, terminadores y saltos de línea

D11 define `fn ... => return E`, expandido a bloque con return explícito.
Puede escribirse `=> return if b { value: 1; } else { value: 2; }`.
`=> E` se reconoce como ImplicitClosureBody (G.2) y se rechaza con
NX-E1301/implicit_closure_body, con span del operando y => relacionado.
No se sustituye ese diagnóstico por un error genérico de parseo. Una expresión final
descartada no devuelve un valor; si está terminada correctamente y falta un
retorno final ordinario/async, corresponde NX-E1317. No hay abreviatura de
brazo `P => E`: cada brazo tiene bloque y salida explícita.

**Completa lo pendiente de D01; decisión explícita de producto de esta
propuesta, no regla ya escrita en la base.** Se fija para estas fixtures:

1. `;` termina la sentencia. Un salto de línea puede insertar terminador
   después de una expresión completa fuera de paréntesis/corchetes abiertos
   de esa sentencia. No lo inserta si la línea siguiente continúa con llamada,
   índice, acceso, operador binario, coma o `?`. Estas continuaciones se
   reconocen por tokens, no por tipos. Un salto antes de `}` sí termina una
   expresión completa. Los detalles del léxico general aún no cubiertos por
   estas producciones siguen pendientes de corpus D01; no alteran estos casos.
2. **Una llave `}` en la misma línea no inserta terminador.** Por tanto
   `{ expr }` carece de separador, mientras `{ expr; }` es descarte válido.
   Si hay un salto de línea tras expr y antes de `}`, es descarte terminado
   por inserción. Ninguna de las tres formas equivale a `return expr;`.
   La regla se aplica también a return/value:/yield con operando, no a llaves
   que cierran una declaración estructural o sentencia de control.
3. Tras reconocer una sentencia sin el terminador exigido se emite
   **NX-E0005**, subcause = null, parser_context = statement_terminator.
   primary_span abarca el siguiente token (la `}` en estos ejemplos);
   en EOF es el rango vacío al final. unexpected_token identifica ese token,
   expected_tokens = [";"] indica el **token canónico de inserción sugerido**,
   no el conjunto exhaustivo de terminadores (también existe un salto insertable).
   expected_tokens_kind = canonical_insertion lo distingue de un conjunto
   exhaustivo de tokens admisibles. Related_spans contiene la expresión precedente.
   Tiene prioridad sobre errores de flujo derivados del cuerpo mal formado.
4. Reparación sugerida para un cuerpo con resultado final T: escribir
   `return expr;` si expr tiene el tipo requerido y todas las rutas quedan
   correctas. Para una rama destinataria de valor, escribir `value: expr;`
   bajo la misma comprobación. Son decisiones explícitas del autor: la edición
   añade un salto de control y se ofrece como aplicable **solo tras recomprobar
   tipos, efectos, propiedad y flujo**. Insertar únicamente `;` es una
   alternativa de descarte, no una reparación completa si después da NX-E1317
   o NX-E1302/missing_branch_value. El positivo pareado debe compilar completo.
5. Una nueva línea inmediatamente después de `return` o de `value:` termina esa
   sentencia, incluso si la siguiente empieza por `(`. Para continuar el
   operando, se escribe la apertura **antes** del salto: `return (` o
   `value: (`. El parser admite ambas formas vacías para dar diagnóstico
   semántico preciso: return vacío tiene resultado unidad en fn ordinaria/
   async (NX-E0102/type_mismatch si exige i64, por ejemplo), y finaliza gen
   conforme §1.1. ValueStmt vacío siempre da NX-E1302/missing_value_operand;
   para entregar unidad se escribe `value: ();`.
6. En esos dos errores de operando, el span primario es return o el
   introductor completo `value:`;
   related_spans señala el salto de línea y el tipo/destinatario requerido.
   No se informa una caída NX-E1317 adicional por el return vacío mal tipado.
   La sugerencia es mover `(` antes del salto o poner el operando en esa
   misma línea, y volver a comprobar. Value sin : sigue siendo un nombre
   ordinario (V11/V12). Los pares E33/E33b y E34/E34b fijan
   el AST y el efecto de esa reparación.

Estas reglas no introducen retorno implícito ni modifican el significado de
EX04: allí yield termina por salto de línea y el generador termina sin return.
El formateador debe preservar tanto el AST como los terminadores insertados.

### 2.1. Terminación de declaraciones y requisitos de trait

**Completa lo pendiente de D01**, compatible con `use std.app` de EX01
(roadmap línea 87) y el requisito sin ; de D08 (línea 454).

1. `use` (también pub use/use extension), `mod`, `type` y `const`
   terminan por ; o por un salto tras su producción completa. No necesitan
   acabar en una expresión: un nombre/ruta/tipo puede cerrar la declaración.
   La misma regla termina un TraitMember sin cuerpo tras Signature/WhereClause.
   FnDecl, métodos con cuerpo y declaraciones con llaves terminan en su cierre;
   no requieren un terminador adicional.
2. La inserción solo ocurre fuera de paréntesis/corchetes, argumentos de tipo,
   grupos de imports y mapas with abiertos de **esa declaración o firma**.
   Las llaves del módulo/trait contenedor no impiden insertar el terminador.
   Se consulta la categoría sintáctica y el siguiente token, nunca un tipo inferido.
3. Si el siguiente token puede continuar la producción actual, no se inserta:
   por ejemplo . o as en un import; = al completar un alias/const; ->, from,
   result_from, retains, with o where tras una firma; coma o + al continuar
   una lista/cota. En un inicializador const también rige la continuación de
   expresiones de §2. Una llave de apertura después de la firma de TraitMember
   continúa su cuerpo incluso en otra línea. Una llave de cierre en la línea
   siguiente sí permite terminar el requisito sin cuerpo.
4. En la misma línea dos declaraciones/requisitos simples se separan con ;.
   Una } contigua no lo inserta para un requisito sin cuerpo. Si falta el
   terminador de una producción completa se usa NX-E0005, con
   parser_context = declaration_terminator, span del siguiente token y
   sugerencia de ; como inserción canónica. EOF no crea un salto de línea
   inexistente: fuente simple sin ; ni salto final usa el mismo diagnóstico.
5. La EBNF comparte Terminator/InsertedTerminator con §2. Cambiar un salto
   terminador por ; conserva el AST; moverlo hacia una continuación no tiene
   esa garantía. M31/M31b concretan la equivalencia y la continuación de un
   método con cuerpo; no se atribuye soporte de inserción a F0-0001.

### 3. Unificación local y exacta de tipos

Cada camino normal de la expresión produce el mismo tipo nominal concreto.
Se utiliza una anotación esperada disponible o se resuelven restricciones
locales; si no hay solución única se exige anotación. No se insertan conversiones
numéricas, Option/Result, uniones, dyn ni boxing para reconciliar ramas.
Literales reciben el tipo esperado compatible; sin él se aplica el tipo
predeterminado del núcleo (i64 para enteros del corte), no una promoción entre
ramas ya tipadas.

Un camino que retorna, propaga error con ? o diverge no entrega valor a la unión.
El tipo interno Never de ausencia de continuación no obliga a convertir las
otras ramas. Si no hay continuación en ninguna rama, la expresión es Never.
Los caminos normales restantes sí requieren `value:`, incluso para unidad:
`value: ();`. Un operador no termina un camino porque pueda fallar en ejecución:
se analiza también su salida normal.

Dos expresiones de closure tienen tipos anónimos distintos (D11), aunque sus
firmas coincidan. Un if entre ambas no infiere un contenedor común. Se puede
elegir un dato/enum explícito y ejecutar una sola closure, o usar borrado
explícito con sus efectos cuando F3 lo soporte.

### 4. Exhaustividad y patrones

If expresión requiere else. Match es exhaustivo tanto como sentencia como
expresión. Se comprueban constructores de enums, bool, productos y patrones
anidados. En dominios abiertos como enteros/texto se exige un patrón residual
`_` o binding irrefutable salvo cobertura finita demostrable; no se enumeran
infinitos valores. No se introduce null universal.

Una guarda se escribe `Patron if condicion => { ... }`.
Los brazos se consideran en orden fuente. Un brazo es estructuralmente
inalcanzable si **todos** los valores que casa su patrón están incluidos en
la **unión de los patrones de brazos anteriores sin guarda**: se emite
NX-E1304/unreachable_arm, tenga o no guarda el brazo actual. No hace falta
que un único brazo anterior lo cubra (E10b); los brazos anteriores con guarda
no contribuyen a esa unión. No se elige una impl ni conversión para hacerlo
alcanzable. Los guards se evalúan después de casar estructuralmente y antes
de mover payloads. Una guarda puede resultar falsa, así que nunca aporta
cobertura exhaustiva por sí sola. La primera versión exige guards de lectura,
sin mutación, alloc, block, suspend ni io, y sin transferencia; fallos de esta
regla se diagnostican específicamente. No se introduce un solucionador de
arbitrariedad para probar que un guard «siempre es verdadero».

El scrutinee se evalúa una vez. Los bindings de patrón prestan por lectura
por defecto. `match consume x` transfiere x a un temporal de selección;
un patrón `Some(consume payload)` transfiere ese campo solo tras seleccionar
el brazo. Un patrón `inout payload` exige scrutinee con permiso mutable
explícito `match inout x`; conserva exclusividad y vida. No se mueve bajo
guards fallidos. Un patrón inout sin match inout da
NX-E1105/permission_escalation; match inout sobre binding inmutable da
NX-E0105. Un patrón no puede aumentar el permiso del scrutinee.
Un temporal propietario limpia los campos no transferidos al terminar el
brazo; uno prestado no destruye el objeto origen.

### 5. Movimientos e inicialización por camino

Condición/scrutinee se evalúan antes de la rama; solo se ejecuta una. Durante
comprobación, cada rama parte del estado posterior a esa evaluación. El resultado
por value:/return copia datos copiables o exige `consume` para transferir
recursos; no se inserta copia de un recurso para unificar ramas.

Al unir caminos que continúan, un binding solo está disponible si está
inicializado y no consumido en todos ellos. Si se movió en uno, cualquier uso
posterior que requiera valor se rechaza. Los caminos que retornan/divergen no
participan en esa disponibilidad, pero sí en efectos y limpieza. No se
reconstruye correlación de valores entre dos if independientes para permitir
un uso que la unión marcó inválido.

Flags de ejecución conservan qué propietario sigue vivo en cada camino.
La salida de ámbito destruye solo los propietarios restantes, una vez. Se
permite reinicializar un var completo con RHS válido y sin préstamos activos:
primero se evalúa RHS, luego se limpia el valor anterior si ese camino aún
lo posee, y se instala el nuevo. Si RHS falla, el anterior sigue sujeto a su
limpieza normal. No se infiere reinicialización parcial de recursos en F0.

Esta regla amplía lo pendiente respecto al subconjunto F0-0001, que rechaza
reasignación de recursos: no afirma que ya se soporte ni modifica aquel código.

### 6. Préstamos y procedencias de los resultados

El resultado prestado debe ser válido en todo camino que lo produzca. Se
conserva el conjunto de orígenes posibles y se exige una duración que cubran
todos, junto con la unión conservadora de restricciones de acceso. No se
elige la duración más larga. Una rama que presta un local no puede entregar
esa vista fuera de su ámbito; tampoco en un error, closure o futuro.

D11 conserva identidad, independencia, afinidad y préstamos de capturas;
D12 conserva campos retenidos y operación de limpieza. Un enum de futuros o
closures propaga los requisitos de todas sus variantes posibles. Envolver
o mover no prolonga almacenamiento (§6.2).

Para interfaces Nexo verificadas se decide una cláusula de procedencia mínima:
`-> T from(x, y)` limita préstamos contenidos en el valor devuelto a esos
parámetros/propietarios y sus procedencias transitivas; ambos deben permanecer
válidos para usar el resultado cuando no se distingue la alternativa.
Cuando un parámetro consume se transfiere al resultado, from(v) transporta
las procedencias prestadas contenidas transitivamente en v, no la dirección
de su slot local ya destruido. Nunca autoriza devolver una vista al propio
almacenamiento del parámetro que desaparece al retornar.
Ausencia de from en una firma pública verificada prohíbe préstamos externos
en el resultado. No convierte tipos compartidos en independientes.

En async, from describe el futuro devuelto; `result_from(x, y)` describe
préstamos en su T final. El cuerpo debe probar ambas relaciones. Si el origen
es propiedad del frame que termina, el resultado solo puede salir conservando
un propietario suficiente, no una vista desnuda. Los detalles internos de
campos no se exponen como lifetimes públicos arbitrarios.

Las cláusulas se escriben después del tipo de resultado, en orden from,
result_from, retains, y antes de with; las no necesarias se omiten.
Retención en un argumento receptor usa su propio contrato: los parámetros
ordinarios son de lectura y no retención por defecto en Nexo verificado;
retener un préstamo en un propietario proporcionado se declara
`retains(v, in: owner)`, exige acceso/vida suficientes del owner y extiende
el préstamo hasta su liberación. Sin esa cláusula el cuerpo no puede almacenar
el préstamo fuera de la llamada. Un contrato externo ausente es desconocido,
no esta promesa verificada. Callbacks obedecen las mismas reglas.

Reglas diagnósticas, también para funciones privadas y portadores genéricos:

1. NX-E1318 rechaza duración insuficiente de un préstamo transportado por un
   agregado/genérico fuera del caso específico de closure, futuro o resultado
   de rama. El análisis propaga procedencias simbólicas de parámetros de tipo;
   un T desconocido no se supone independiente. NX-E1104/NX-E1205/NX-E1307
   conservan prioridad en sus contextos específicos; una envoltura no los borra.
2. NX-E1319 rechaza un cuerpo que almacena procedencia prestada en un argumento
   propietario sin la cláusula retains correspondiente. También se comprueba
   antes de instanciar genéricos: un F sin requisito de independencia puede
   contener préstamos. Transferir consume v a owner no los elimina. El análisis
   reconoce asignaciones directas y la **retención transitiva de una llamada
   directa o indirecta**, obtenida de su contrato retains. No depende del
   nombre guardar ni de inspeccionar el cuerpo del callback. La envolvente
   debe declarar los orígenes/destino retenidos; C47b usa esta misma regla.
3. NX-E1320 comprueba una cláusula escrita from/result_from/retains cuyo conjunto
   de orígenes no cubre el cuerpo o cuyo destino de retención no coincide.
   Incluye una firma privada: no depende de exportación. La ausencia de from
   requerida en un resultado público usa NX-E1310/missing_provenance; su presencia
   incorrecta usa NX-E1320, retirando la subcausa anterior provenance_mismatch.
   Cuando no hay contrato público escrito y tampoco es necesario escribirlo,
   se infieren procedencias locales, nunca se asumen vacías.
4. Un retains correcto no demuestra por sí solo duración suficiente en el
   llamador: si el owner sobrevive al origen se aplica el código de escape del
   portador. Al comprobar una declaración se informa primero del contrato
   omitido/contradicho y se evita diagnosticar usos derivados de esa firma inválida.

### 7. Efectos de condición, ramas y salidas

El resumen estático es unión de condición/scrutinee, guards y todos los caminos
sintácticamente posibles, incluidos movimientos, copias, temporales y limpieza
normal, return, ? y cancelación aplicable. No se suman como si se ejecutaran
ambas ramas: para costes medidos se registra la ejecutada; para cotas solo se
usa una combinación justificable. Efectos de ramas no ejecutadas no se ejecutan.

No se elimina un efecto por inferir que una condición constante elegirá otra
rama; el contrato de fuentes sigue comprobándose antes de optimizar. Una
construcción comptime de selección, si se especifica separadamente, no se
confunde con if ordinario. Las entradas NX-E0412 a NX-E0416 del [catálogo único](F0-CODIGOS.md) señalan
la rama/operación culpable. Desconocido no es conjunto vacío (§6.1).

### 8. Unidad módulo y nombres

Un módulo es una unidad léxica de declaraciones, con un archivo raíz explícito
en el manifiesto nexo.toml (D01) o suministrado al compilador en el modelo F0.
`mod nombre;` declara un submódulo en archivo, sin ejecutar código.
Para raíz `raiz.nx`, `mod red;` carga `red.nx`; desde red,
`mod http;` carga `red/http.nx`. No hay alternativa implícita mod.nx ni
búsqueda heurística en directorios. No hay módulos inline en la primera gramática.
Un archivo tiene una única identidad de módulo en un paquete; declararlo dos
veces por alias de ruta es error. El resolvedor detecta colisiones de rutas
en los cuatro destinos (T28), sin cambiar semántica según mayúsculas del disco.

La ausencia del archivo requerido por mod produce
NX-E1321/module_file_missing. Dos declaraciones del mismo archivo canónico
producen NX-E1321/duplicate_module_file, incluida repetición de mod; esta
comprobación precede a colisión de nombres derivada. Para rutas del subconjunto
de identificadores ASCII se usa además una clave portable con separadores
normalizados y case-fold ASCII: dos rutas normalizadas distintas con esa
misma clave dan NX-E1321/path_collision, aunque el anfitrión las resuelva
al mismo archivo. Esta comparación precede a la identidad física para que
a.nx/A.nx no cambie de diagnóstico según el disco. Para rutas sin esa colisión,
los alias del sistema se resuelven a identidad canónica; si apuntan al mismo
archivo se aplica duplicate_module_file. No se sigue una segunda identidad
por un enlace. El modelo de arnés fija esas
identidades y permite probar colisiones sin depender del sistema anfitrión.
La resolución Unicode y enlaces en cada plataforma requiere corpus T28;
no se amplía aquí el alfabeto del subconjunto.

Las rutas usan puntos: `self` módulo actual, `super` padre, `package` raíz
actual; un alias de dependencia encabeza una ruta externa, incluido std reservado.
Tipos, valores y módulos comparten un espacio de nombres de declaraciones por
ámbito, con constructores de enum cualificados o importados explícitamente.
No hay sobrecarga libre por resultado. Los locales pueden sombrear un nombre;
la ruta cualificada sigue disponible y el import duplicado es diagnóstico.

Las declaraciones de nivel módulo son funciones, tipos/traits/impl/extensiones,
módulos, imports y constantes comptime puras. **Completa lo pendiente de
D01/D03/D08, ampliación sobre la base:** se fijan las formas const y type
(alias transparente), que la base no había escrito como producción:

La EBNF única de estas declaraciones está en G.3; FnDecl, StructDecl,
ImplDecl y las producciones auxiliares ya no quedan como nombres indefinidos.

Expr se analiza con la gramática ordinaria; solo después se comprueban el
contexto comptime y el tipo. Una constante debe ser un valor independiente,
copiable, sin recurso vivo/estado mutable compartido, obtenible mediante el
subconjunto puro y acotado de D03. Un inicializador que requiere E/S, creación
de recursos o una operación fuera de ese subconjunto da NX-E1316; no se
ejecuta al cargar el módulo ni se rechaza fingiendo sintaxis inválida.
Una constante numérica pura sí se admite. **Completa lo pendiente de D03:**
el subconjunto permite llamadas a fn Nexo con cuerpo disponible formado por
operaciones admitidas por ese evaluador; se comprueban transitivamente antes
de evaluar, sin exigir otra palabra para marcar la función. Así, la llamada
a numero en M17c es válida tras quitar print; M17 falla por io, no por ser
una llamada ordinaria ni por su tipo. Los límites de evaluación siguen
NX-E0003 con budget_kind cerrado comptime_steps, comptime_depth o
comptime_memory_bytes, definidos en F0-CODIGOS; no se convierte agotamiento
en valor ni se añade un nombre de presupuesto desde un mensaje libre.
No se aceptan let/var de nivel módulo: no pertenecen a ModuleDecl y dan
NX-E0002 en el token let/var, con parser_context = module_declaration y
los spans/prefijos exactos de M27l/M27v. La anterior M17 no era evidencia
semántica; su sustituta usa ConstDecl.
La entrada de ejecutable main no necesita ser la primera declaración; las
bibliotecas no necesitan main. Esta ampliación del lenguaje no cambia la
gramática experimental de F0-0001, que aún solo acepta una fn main.

Pueden existir referencias cíclicas entre funciones/módulos si no requieren
una definición circular de valor/tipo. Se construye el grafo de definiciones
antes de expandir aliases o evaluar constantes: ciclos de alias dan
NX-E1323/alias_cycle y de constantes NX-E1323/constant_cycle, con el ciclo
mínimo reproducible como causa; un struct de tamaño recursivo sin indirección
da NX-E1323/recursive_value_layout. No se confunden con package_cycle.
Un alias no crea identidad ni propiedad local para las reglas de impl.

### 9. Privado por defecto, pub explícito

Sin pub una declaración es visible en su módulo y descendientes; los padres
no obtienen acceso a privados del hijo. `pub` permite acceso por una ruta
exportada. Para acceso externo todos los segmentos de esa ruta deben ser
públicos, salvo el acceso interno autorizado al construir una reexportación.
Campos de struct son privados por defecto. Su dominio es exactamente el
módulo que **define el tipo y sus módulos descendientes**, no el módulo que
lo importa, reexporta o añade una impl/extensión. Se permite acceso desde
descendientes; se rechaza desde padre, hermanos y dependencias externas.
La construcción literal con campos privados obedece ese mismo dominio.
La construcción pública desde fuera usa una función autorizada. Una derivación
solo accede a ellos con autorización del módulo definidor (D03).


Un enum público exporta sus variantes y payloads como contrato exhaustivo;
no hay variantes secretas de un enum público ni non_exhaustive implícito en
1.0. Añadir variante cambia la API y reabre matches dependientes. Un trait
público exporta sus requisitos; visibilidad no permite omitir métodos para
conformar. Los efectos y tipos de cada operación siguen D08.

Las funciones privadas pueden inferir el resultado a partir de todos sus
return y del contexto local; si recursión o restricciones impiden una solución
única, se exige anotación. Los parámetros de funciones nombradas se anotan.
No se convierte una inferencia privada en contrato público.

Una firma pública exige tipos de parámetros/resultado, etiquetas, defaults
comptime, modos de acceso, límites de efectos y procedencia/retención cuando
corresponda. En fn ordinaria basta cualquier WithClause válida para ese
contexto: with no_alloc publica solo esa prohibición; with unrestricted
publica ninguna, y with { call(unrestricted) } es igualmente explícito.
Las restricciones no escritas quedan permitidas; «firma completa» no exige
las cuatro/cinco prohibiciones simultáneas. Así M29b/M29c/M29d y las firmas
de M14/M15/M19 siguen la misma regla. No basta un with mal formado ni omitir
la operación obligatoria en un mapa. Async usa las cuatro
operaciones de D12. Un tipo anónimo de closure no se filtra como API inferida:
se ofrece un wrapper nominal público con tipos de campos nombrables, un
parámetro `F: CallableBound` o, en F3, `dyn (CallableBound)`. La gramática de
WithClause (única para fn, tipos y cotas), CallableBound/FutureType y el
ejemplo completo de cotas están en D11 §5.2;
no son alternativas con sintaxis abierta. Un parámetro genérico recibe el
invocable, no habilita retornar un tipo anónimo arbitrario.

Cada tipo mencionado públicamente, incluidos parámetros asociados, errores,
payloads y contextos de región, necesita una ruta pública alcanzable. Puede
vivir en módulo privado y reexportarse; no exige publicar todos sus campos.
No se permite una firma que el cliente no pueda nombrar por faltar exportación.

### 10. Importación y reexportación sin efectos ni prioridad

```nexo
use std.fs;
use package.modelo.{Dato, Error};
use dependencia.api.Dato as DatoExterno;
pub use package.interno.Dato;
```

Use solo crea nombres locales; no reexporta, ejecuta inicializadores ni activa
extensiones. Se importan módulos o nombres concretos, con grupos enumerados.
No hay glob imports en 1.0 (M33 fija token y span del rechazo). Dos nombres distintos con el mismo alias local son
error; se exige as o calificación. Repetir exactamente el mismo import es
redundante y se diagnostica sin crear una segunda identidad.

`pub use` es la única forma de reexportar; no hay un keyword export adicional.
**También crea el mismo nombre local que use**, bajo el nombre final o el
alias as escrito. El exportador puede usarlo sin otro import; M03 (main usa
Dato) y M15 (extend Dato en e1/e2) dependen de esta regla. Las entradas local
y exportada comparten identidad/contrato. Alias_collision y duplicate_import
se aplican a use/pub use conjuntamente: no se necesita duplicar use.
Solo se instalan ambas entradas si acceso y exportabilidad son correctos.
Solo reexporta declaraciones pub accesibles al exportador. Puede exponer un
elemento pub de un módulo privado propio sin hacer público todo el módulo.
No vuelve público un elemento privado ni permite eludir acceso de dependencias.
Reexportar/renombrar conserva identidad de origen, efectos, modos y afinidad.

Para diagnosticar un pub use se comprueba primero si el exportador tiene
acceso al destino. Si no lo tiene, corresponde NX-E1308/private_access.
Si sí lo tiene pero el destino no es pub, corresponde
NX-E1308/private_reexport. No se emiten ambas por la misma declaración ni se
crea un alias público válido tras el fallo. La fixture M04 usa el segundo
caso sin un consumidor del alias inválido; M02 conserva el primero.

Las extensiones se activan únicamente mediante
`use extension paquete.modulo.Tipo;`: importa explícitamente los bloques
extend de ese módulo para ese tipo nominal. Un use normal no los activa.
Métodos propios tienen prioridad; traits/extensiones igualmente aplicables
producen ambigüedad, no selección por orden. La llamada cualificada identifica
trait o módulo de extensión: `Trait.metodo(receptor, ...)` o
`extension paquete.modulo.Tipo.metodo(receptor, ...)`, respectivamente.
Dos bloques extend del mismo módulo/tipo no pueden definir el mismo método
(NX-E1322), tampoco dentro de un solo bloque. Se diagnostica al registrar
declaraciones, aunque nadie importe la extensión. Importar una extensión no
crea una impl canónica. Los métodos pub de un extend son exportables; self
es el receptor prestado del tipo extendido, como en D08.

### 11. Identidad y coherencia del grafo de paquetes

El manifiesto declara nombre del paquete, raíz y dependencias directas con
alias locales; no se accede por accidente a dependencias transitivas. El grafo
resuelto registra origen, nombre, versión/revisión e integridad de cada paquete.
Para origen+nombre se admite una sola revisión seleccionada en el grafo inicial:
restricciones incompatibles se rechazan, no se escogen por orden. Un alias
distinto no crea otro paquete ni vuelve local un tipo extranjero.

La identidad nominal es identidad resuelta del paquete + módulo + declaración.
Las rutas/aliases son nombres de acceso. Mover una declaración pública de
identidad sin compatibilidad explícita cambia API; una simple reexportación
no genera identidad nueva. Cachés guardan identidades y contratos, no solo
texto de imports. En F0 el arnés puede fijar el grafo sin gestor de paquetes;
F2 entrega manifiesto/lockfile local y F4 publicación conforme roadmap.

D08 manda: un paquete implementa un trait que declara o lo implementa para
un tipo nominal exterior que declara. Alias y colección extranjera de tipo
local no cuentan como tipo propio; nuevo wrapper nominal local sí.
Las implementaciones se registran para todo el grafo antes de resolver
llamadas, incluso si no se importa el módulo que las contiene. La visibilidad
no oculta una impl a la comprobación de coherencia; no hay impl privadas
competidoras seleccionables mediante use.

Dos impl potencialmente solapadas se rechazan si no hay prueba de disjunción.
No hay prioridades, especialización semántica ni orden de enlace que resuelva
el conflicto. Un paquete nuevo puede revelar un conflicto diagnosticado,
nunca cambiar silenciosamente una implementación escogida. Resolver sobre
aliases canónicos impide eludir esta regla con pub use.

### 12. Interfaces verificables e invalidación

El artefacto de interfaz incluye identidad del paquete, exportaciones canónicas,
tipos, variantes, etiquetas/defaults, traits/impl y extensiones; propiedades
selladas de copia/independencia/transferencia/afinidad; contratos de procedencia,
retención, efectos por operación y limpieza. Los contratos se verifican contra
el cuerpo Nexo antes de exportar; supuestos extranjeros se etiquetan (P06/P07).

Cambios de cualquiera de esos datos, destino o configuración pertinente
invalidan consultas y dependientes afectados (T09/T17/T18/T20/T21/T26).
Renombrar/reexportar no permite reutilizar una firma vieja que ocultó un efecto,
un recurso o una suspensión. La salida CLI/LSP/REPL comparte el mismo dato
semántico P09; no se vuelven a inferir APIs diferentes en cada herramienta.

## Gramática común de las fixtures — G

**Completa lo pendiente de D01/D08/D10 y acuerdos §2–§4.** Este es el
subconjunto documental que necesitan las fuentes concretas de D11–D13, no
una afirmación de soporte del parser F0-0001. Reutiliza de aquel corte Name
ASCII, literales i64/bool, comentarios, arrays y bloques; lo extiende con
tipos, llamadas, modos, patrones y declaraciones. Tokens como ->, =>, :, .,
<, >, ? y las palabras aquí escritas se reconocen antes del análisis de tipos.
Las producciones WithClause, CallableBound y FutureType tienen **una sola
definición en D11 §5.2**; RegionStmt, una en D12 §3. Las demás de este
subconjunto se definen a continuación.

G es una **extensión de la base, no su sustitución ni una lista de exclusiones**.
Se incorporan sin cambios las formas propuestas en EX01–EX04/D01/D03/D08/D10:
for (incluido for inout), continue, cadenas e interpolación UTF-8, !=, *,
rango .., with ... as, Table<..., layout: ...> y @derive. Su ausencia de las
producciones reducidas no autoriza rechazarlas ni cambiar EX01–EX04. La sintaxis
heredada sigue necesitando su corpus y biblioteca, sin reescrituras de los
archivos de aceptación (roadmap línea 78).

**Fases de la base:** EX01 y EX04 completos son puertas F2; EX03 es F3 y EX02
es F5. Los cuatro son aceptación obligatoria en su fase, no se trasladan todos
a F2. X01/X04 fijan aceptación prevista del archivo original y de sus líneas
verbatim con value como Name, sin acreditar ejecución de las puertas.
X04r es la única fixture rotulada «EX04 reducido», contenida en G reducida.



### G.1. Expresiones, modos y procedencia

Name es el token de identificador ASCII de F0-0001, syntax::lex (líneas 91–100):
patrón [A-Za-z_][A-Za-z0-9_]*, con coincidencia máxima. Las palabras reservadas
usadas como terminales de G se distinguen por contexto léxico/sintáctico;
value conserva siempre su categoría Name y la selección local de §1.
No se heredan como palabras reservadas del lenguaje completo los nombres
experimentales mock_open/print/drop/etc. excluidos por syntax::Parser::name
en aquel corte; siguen siendo nombres intrínsecos resolubles del arnés.

```text
Mode             = "inout" | "consume"
IntLiteral       = ["-"] UnsignedInteger
UnsignedInteger  = Digit {Digit}
Digit            = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
Type             = TypePath | "()" | "(" FutureType ")" | "dyn" "(" CallableBound ")"
TypePath         = Path ["<" Type {"," Type} ">"]
Path             = PathHead {"." Name}
PathHead         = Name | "self" | "super" | "package"
Expr             = IfForm | MatchForm | AddExpr
AddExpr          = TryExpr {("+" | "-") TryExpr}
TryExpr          = (AwaitExpr | AccessExpr) ["?"]
AwaitExpr        = "await" AccessExpr
AccessExpr       = [Mode] Postfix
Postfix          = Primary {Selector}
Selector         = "." Name | "[" Expr "]" | Arguments
Arguments        = "(" [Argument {"," Argument}] ")"
Argument         = [Name ":"] Expr
Primary          = Name | "self" | "super" | "package" | IntLiteral | BoolLiteral
                 | "(" [Expr] ")" | "[" [Expr {"," Expr}] "]" | ClosureExpr
                 | "extension" Path
Place            = PlaceRoot {("." Name) | ("[" Expr "]")}
PlaceRoot        = Name | "self" | "(" Place ")"
ProvenanceClauses = [FromClause] [ResultFromClause] {RetainsClause}
FromClause       = "from" "(" Origin {"," Origin} ")"
ResultFromClause = "result_from" "(" Origin {"," Origin} ")"
RetainsClause    = "retains" "(" Origin "," "in" ":" Origin ")"
Origin           = Name | "self"
```

Para V01/V02, G concreta resta binaria con la precedencia de + y asociación
a izquierda, y literales enteros negativos en posición de operando. **Completa
D01/D07**: - es token separado del entero, sin depender de espacios;
`value -1` es resta, no Name seguido de entero firmado. Rige la aritmética
comprobada D07. += se toma de D08: lugar evaluado una vez, exclusividad y
suma comprobada. Estas concreciones no afirman soporte de F0-0001.

En este subconjunto, Future es el nombre intrínseco de FutureType, no una
alternativa TypePath desnuda que permita omitir sus cotas. Su uso como tipo
se escribe entre paréntesis cuando lo requiere Type/ResultType; los nombres
de los parámetros genéricos y tipos nominales ordinarios siguen TypePath.

AccessExpr es la única producción de prefijos. Si Postfix termina en Arguments,
el prefijo califica el receptor de **esa última llamada**, con esta selección
explícita que amplía/concreta las formas de llamada de D08:

1. En f(args), el receptor invocable es f. En (x.m)(args), la agrupación hace
   explícito que se invoca el valor del campo x.m; inout (x.m)() presta ese
   campo, no self = x. Los paréntesis conservan su categoría Place.
2. En x.m(args), si x denota un valor, la forma con punto sin agrupación es
   llamada de método: inout x.m() califica **self = x**, y consume x.m()
   transfiere x al método. El modo declarado de self debe admitirlo; nunca
   se convierte por inferencia en llamada de un campo invocable. Para ese
   campo se escribe (x.m)(args), también si no existe un método homónimo.
   Así método y campo no compiten por tener un tipo invocable. Si m resuelve
   un campo en esta forma de método, da NX-E0102/field_requires_grouped_call (member_kind =
   field, expected_member_kind = method); la reparación agrupa (x.m) y se
   recomprueba. C52d/C52c ejercitan ese par.
3. Si el prefijo de la ruta nombra un módulo/tipo/trait, es calificación de
   nombre, no un self implícito. Iterator.next(inout g) y Trait.metodo(inout x)
   pasan el receptor como argumento explícito según D08. Clasificar un nombre
   como módulo/tipo o valor usa resolución nominal local, nunca inferencia de
   efectos o la existencia accidental de un campo invocable.
4. Los modos de los argumentos son independientes; inout/consume no se
   propaga a llamadas anteriores de la cadena. Si Postfix termina en un Place
   sin llamada, consume lo transfiere e inout pide su préstamo exclusivo
   (por ejemplo g(inout x) o match inout x).

C52–C52c de D11 fijan método, receptor inmutable y campo invocable agrupado.
Inout requiere un lugar modificable; un valor sin lugar modificable da
NX-E0102/type_mismatch, y un binding inmutable, NX-E0105. Un temporal nuevo
entregado como propietario no se duplica por escribir consume.

Await espera el valor producido por AccessExpr: `await inout f()` invoca
mutablemente f y espera el futuro nuevo que devuelva; **no** permite esperar
un handle prestado por `await inout h` (NX-E0102/type_mismatch, requiere
operando propietario). La invocación de una closure async continúa requiriendo
consume por D12. `await e()?` sigue siendo `(await e())?`;
`await (e()?)` propaga antes de esperar. Se elimina OwnedPostfix como segunda
gramática divergente; D12 refiere a esta producción.

From/result_from/retains comparten esta sintaxis en fn y CallableBound.
Los Origin resuelven parámetros nombrados o el receptor self de la firma,
nunca un nombre libre del llamador. En una cota invocable, parámetros sin
nombre siguen siendo posicionales y no pueden nombrarse en procedencias.
Una cota con nombres publica también esas etiquetas según D10; el candidato
debe respetarlas y sus contratos. Self representa el receptor invocable,
no un permiso para prestar de un entorno ya consumido/destruido.

From describe préstamos del resultado inmediato; si este es Future<T>,
result_from describe los de T al completarse, como en D12. Retains declara
retención en un argumento/receptor modificable. En una cota escrita, omitir
estas cláusulas promete ausencia de esas procedencias/retenciones, no inferencia
desde el callback concreto; el cuerpo genérico se comprueba contra la cota.
Un contrato escrito insuficiente da NX-E1320. Un origen que no resuelve
es un binding desconocido NX-E0101; result_from sin resultado Future/async
aplicable da NX-E0102/type_mismatch. No se certifica un préstamo con solo declarar from.
Las reparaciones y duración siguen §6.

### G.2. Patrones, bloques y control

```text
Block         = "{" {Statement} "}"
Statement     = BindingStmt | AssignStmt | ReturnStmt | ValueStmt | YieldStmt
              | ExprStmt | IfForm | MatchForm | Block | RegionStmt
BindingStmt   = ("let" | "var") Name [":" Type] "=" Expr Terminator
AssignStmt    = Place ("=" | "+=") Expr Terminator
ReturnStmt    = "return" [Expr] Terminator
ValueStmt     = ValueName ":" [Expr] Terminator
ValueName     = Name  (* lexema exacto value; selector local de §1 *)
YieldStmt     = "yield" Expr Terminator
ExprStmt      = AddExpr Terminator
Terminator    = ";" | InsertedTerminator | MissingTerminator
MissingTerminator = ε  (* recuperación diagnóstica; nunca aceptada *)
IfForm        = "if" Expr Block ["else" (Block | IfForm)]
MatchForm     = "match" Expr "{" [Arm {"," Arm} [","]] "}"
Arm           = Pattern ["if" Expr] "=>" Block
Pattern       = "_" | IntLiteral | BoolLiteral | Mode Name
              | Path ["(" Pattern {"," Pattern} ")"]
              | "(" Pattern {"," Pattern} ")"
ClosureExpr   = ["async"] "fn" [CaptureList]
                "(" [ClosureParam {"," ClosureParam}] ")" ["->" ResultType]
                ProvenanceClauses [WithClause] ClosureBody
ClosureBody   = Block | "=>" "return" Expr | ImplicitClosureBody
ImplicitClosureBody = "=>" Expr  (* reconocida y rechazada: NX-E1301 *)
ClosureParam  = [Mode] Name [":" Type]
CaptureList   = "[" [Capture {"," Capture}] "]"
Capture       = ("borrow" | "copy" | "inout" | "consume") Name
```

InsertedTerminator es exclusivamente la regla de tokens de §2/§2.1, no una
inserción por tipo esperado. MissingTerminator es una producción de
**recuperación diagnóstica**, no un terminador aceptado: su ε solo se sintetiza
en un punto que exige Terminator después de consumir la sentencia/declaración
completa, si no hay ; ni InsertedTerminator y el siguiente token no continúa
esa producción. Conserva ese token sin consumirlo y emite obligatoriamente
NX-E0005. No puede utilizarse para inventar un operando, saltarse un token
incomprendido o aceptar el archivo; cada recuperación ya consumió su prefijo.
La aceptación exige cero nodos de recuperación o formas rechazadas.

ImplicitClosureBody reconoce => Expr entero y registra su operando como nodo
inválido; no le asigna retorno implícito. Emite NX-E1301/implicit_closure_body
con primary_span del Expr completo y related_span de =>; se prioriza esa
causa sobre errores semánticos derivados de usar dicho cuerpo inválido.
Return es una palabra clave, no un Name/Primary que compita con esa alternativa.
Un token que no inicia Expr ni return tras => sigue dando NX-E0002; no se
disfraza de NX-E1301. C25/E03 comprueban expresiones completas reconocibles.

El lexer emite Name para value. Al inicio de Statement solo Name(value)
seguido de : en la misma línea selecciona ValueStmt; en otro caso rigen
AssignStmt/ExprStmt. Se reconoce ValueStmt fuera de posición para diagnosticar
§1, sin aceptarlo ni transformar un Name ordinario en entrega implícita.
IfForm/MatchForm en posición Statement no reciben value:; en posición Expr
sí, solo como última sentencia directa de su rama. ExprStmt usa AddExpr para exigir paréntesis al
descartar deliberadamente un if/match expresión. La ausencia de else se
reconoce sintácticamente y se diagnostica semánticamente solo en el if expresión.
Return y ValueStmt vacíos se reconocen para aplicar §1.1/§2, no como parse error.

| Forma reconocida y rechazada | Producción / control posterior | Diagnóstico exigido |
|---|---|---|
| Closure corta => Expr sin return | ClosureExpr → ClosureBody → ImplicitClosureBody | NX-E1301/implicit_closure_body; C25/E03. |
| Sentencia/declaración completa sin terminador | Terminator → MissingTerminator, recuperación obligatoriamente errónea | NX-E0005; C44/E31/E32; §2.1 incluye declaraciones. |
| Bloque ordinario/async que cae sin retorno requerido | FnDecl o ClosureExpr con Block y cero o más Statement: sintaxis válida; análisis de flujo | NX-E1317; E21/E22/E35. |
| return con operando, incluso (), en gen fn | FnDecl con gen y ReturnStmt con Expr: sintaxis válida; comprobar contexto generador | NX-E1324; E30/E30c. |
| return/value: vacíos | ReturnStmt/ValueStmt con [Expr] ausente | Return de unidad permitido según contexto; NX-E0102/type_mismatch o NX-E1302/missing_value_operand según §2; E33/E34. |
| if expresión sin else, rama sin value: o value: fuera de posición | IfForm con else opcional, Block y ValueStmt válidos; comprobar posición y flujo | NX-E1302/missing_else, missing_branch_value, invalid_value_target o non_final_branch_value; E02/E04/E20/V14/V15. |
| async gen / yield fuera de gen | FnDecl admite ambos modificadores; YieldStmt admite yield en Block antes del chequeo del invocable más próximo | NX-E1206/async_generator_declaration o yield_in_async; NX-E1213 en fn/closure ordinaria; A15a/A15b/A40/A41. |
| Firma pública sin resultado/cota explícitos | Signature admite [-> ResultType] y [WithClause] ausentes; comprobar visibilidad | NX-E1310/missing_signature_type o missing_effect_bound; M30/M29; async usa NX-E1202. |
| Operación with incorrecta para el contexto o fase ausente | OperationList admite cada Operation; obligatoriedad/admisibilidad se comprueba después | NX-E0417 o código específico D11 §5.2/D12; C35/C38–C42/C45/A05/A36. |
| Cota invocable sin with, mapa vacío o unrestricted combinado | CallableBound con [WithClause] ausente, OperationList con contenido opcional y Restrictions con RestrictionItem; formas reconocidas solo para diagnóstico D11 §5.2 | NX-E0417/missing_with_clause, empty_operation_list o unrestricted_mixed; C53–C56. |

Un Path de patrón sin argumentos ni calificación se resuelve como constructor
nulo si nombra una variante del tipo esperado; en otro caso es binding de
lectura. Los paths cualificados y los que tienen argumentos deben nombrar
constructores. Esta resolución no altera el árbol de tokens. Mode Name permite
Some(inout n) y Some(consume n), sujetos al permiso del scrutinee (§4).
El guard se evalúa antes de transferir payloads. Se conservan exhaustividad,
tipos exactos y diagnósticos de permisos; no se inventa una forma de patrón
para reparar errores de estas reglas.

### G.3. Declaraciones

```text
Module        = {ModuleDecl} EOF
ModuleDecl    = {BaseAttribute} BareModuleDecl
BareModuleDecl = ConstDecl | AliasDecl | FnDecl | StructDecl | EnumDecl
               | TraitDecl | ImplDecl | ExtendDecl | ModDecl | UseDecl
ConstDecl     = ["pub"] "const" Name ":" Type "=" "comptime" Expr Terminator
AliasDecl     = ["pub"] "type" Name "=" Type Terminator
FnDecl        = ["pub"] ["async"] ["gen"] "fn" Name [GenericParams]
                Signature [WhereClause] Block
Signature     = "(" [ParamDecl {"," ParamDecl}] ")" ["->" ResultType]
                ProvenanceClauses [WithClause]
ParamDecl     = [Mode] ("self" | Name ":" Type) ["=" Expr]
GenericParams = "<" GenericParam {"," GenericParam} ">"
GenericParam  = Name [":" Bound {"+" Bound}]
Bound         = CallableBound | TypePath
WhereClause   = "where" Constraint {"," Constraint}
Constraint    = Name ":" Bound {"+" Bound}
StructDecl    = ["pub"] "struct" Name [GenericParams] "{"
                [Field {FieldSeparator Field} [FieldSeparator]] "}"
Field         = ["pub"] Name ":" Type
FieldSeparator = "," | ";" | InsertedTerminator
EnumDecl      = ["pub"] "enum" Name [GenericParams] "{"
                [Variant {"," Variant} [","]] "}"
Variant       = Name ["(" Type {"," Type} ")"]
TraitDecl     = ["pub"] "trait" Name [GenericParams] "{" {TraitMember} "}"
TraitMember   = "fn" Name [GenericParams] Signature [WhereClause] (Terminator | Block)
ImplDecl      = "impl" [GenericParams] TypePath "for" Type [WhereClause]
                "{" {MethodDecl} "}"
ExtendDecl    = "extend" Type "{" {MethodDecl} "}"
MethodDecl    = ["pub"] "fn" Name [GenericParams] Signature [WhereClause] Block
ModDecl       = ["pub"] "mod" Name Terminator
UseDecl       = ["pub"] "use" (ImportSpec | "extension" Path) Terminator
ImportSpec    = Path ["as" Name] | Path "." "{" ImportName {"," ImportName} "}"
ImportName    = Name ["as" Name]
```

BaseAttribute refiere a los atributos heredados de D03/EX02, con token inicial @;
@derive(JsonEncode) ante StructDecl es el caso de la base, sin cambiar su
sintaxis ni inferir nuevas capacidades de derivación. Esta referencia incorpora
atributos al inicio de ModuleDecl; ModuleDeclStart debe incluir @, también
después de otras declaraciones completas (S27l/S27v).

ResultType es el de D11 §5.2; WithClause no se redefine aquí. La anotación
-> se puede omitir en fuente privada para inferencia, pero la firma pública
sin ella da NX-E1310/missing_signature_type, no error de parser. Los parámetros
nombrados de fn se anotan; self toma el tipo del impl/trait/extend. La
combinación async gen se reconoce para NX-E1206/async_generator_declaration,
aunque no se admite semánticamente. Un gen requiere tipo de elemento
deducible/anotado y sigue §1.1; no es una fn ordinaria a la que aplicar NX-E1317.

FieldSeparator admite el salto tras un Type completo; la última Field puede
cerrar en la misma línea porque no es una sentencia de expresión. Esto no
permite omitir Terminator de `{ expr }`. Los métodos de trait con cuerpo
usan las mismas reglas de retorno; los requisitos sin cuerpo terminan con
Terminator (salto insertable o ;) según §2.1, como el ejemplo de D08.
Los mod/imports no aceptan let/var de nivel módulo, y el analizador conserva
el prefijo de declaraciones correcto al diagnosticar ese caso.

## Alternativas rechazadas

1. Expresión final de bloque/closure: contradice D01. Value introduce una
   salida explícita de rama sin convertir el bloque de función en expresión.
2. Unificar por coerción, Any/dyn o unión inventada: oculta representación,
   asignación, propiedad y cambios de tipo entre ramas.
3. Match no exhaustivo con fallo implícito: añade una salida no escrita y
   dificulta reparar cambios de enums de forma comprobable.
4. Mantener disponible un recurso si alguna rama no lo mueve: permite
   uso sin propietario en la otra. Se une de forma conservadora.
5. Imports glob, inicialización de módulos con efectos y prioridad por orden:
   vuelven no local la resolución y los costes.
6. Exportación universal o convertir privado en público al reexportar:
   debilita límites de API/invariantes. Se decide pub explícito y pub use.
7. Coherencia solo de impl importadas o identidad nueva por alias:
   contradice D08 y permite conflictos dependientes del archivo cliente.
8. Resolver múltiples versiones incompatibles automáticamente en el grafo
   inicial: complica identidad y coherencia antes de cerrar su modelo F0.

## Contraejemplos y tabla PASS/FAIL

PASS/FAIL significa resultado previsto de la fuente conforme a esta propuesta,
no ejecución actual. Si falta fuente/arnés, la fila dice NO EVALUADO, sin
«PASS si...». Para celdas de sentencias se utiliza exactamente:

```nexo
fn caso(b: bool) {
    let x: i64 = 1;
    let r = mock_open(10);
    // Insertar las sentencias de una sola fila.
}
fn main() { caso(true); caso(false); }
```

Mock_open/inspect/drop son las primitivas documentadas de F0-0001, sin convertir
su código actual en soporte de ramas/consume. Las celdas con funciones completas
sustituyen esa plantilla. Los tipos privados de funciones auxiliares pueden
inferirse localmente de todos sus `return`; en firmas públicas no se permite.
Las fuentes de paquetes se describen como archivos completos y grafo fijo del
arnés F0, permitido por §8, no como imports a archivos inexistentes.

| ID | Fuente concreta o trabajo pendiente | Resultado previsto | Diagnóstico / observación |
|---|---|---|---|
| E01 | `let n = if b { value: 1; } else { value: 2; };` | PASS | —; i64, solo una rama ejecutada. |
| E02 | `let n = if b { value: 1; };` | FAIL | NX-E1302/missing_else. |
| E03 | `fn main() { let f = fn []() -> i64 => 1; }` | FAIL | NX-E1301/implicit_closure_body; operando 1, span S03; ImplicitClosureBody reconocido. |
| E03b | E03 insertando return entre => y 1. | PASS | —; mismo operando, retorno explícito. |
| E04 | `let n = if b { 1; } else { value: 2; };` | FAIL | NX-E1302/missing_branch_value. |
| E05 | `let n = if b { value: x; } else { value: true; };` | FAIL | NX-E1303. |
| E06 | `fn f(b: bool) -> i64 { let n = if b { return 0; } else { value: 1; }; return n; }` | PASS | —; solo else llega a n. |
| E07 | `let opcion: Option<i64> = Some(1); let n = match opcion { Some(n) => { value: n; }, };` | FAIL | NX-E1304/non_exhaustive, testigo None. |
| E08 | `let n = match b { true if b => { value: 1; }, false => { value: 0; }, };` | FAIL | NX-E1304/non_exhaustive; guard no acredita true. |
| E09 | Falta fixture pareada de guarda con reserva/transferencia. | NO EVALUADO | Objetivo NX-E1305/guard_effect con reserva real; E37 cubre NX-E1305/guard_transfer por separado. No usar error sintáctico. |
| E10 | `let n = match b { _ => { value: 0; }, true => { value: 1; }, };` | FAIL | NX-E1304/unreachable_arm; subcausa exacta. |
| E10b | `let n = match b { true => { value: 1; }, false => { value: 0; }, _ => { value: 2; }, };` | FAIL | NX-E1304/unreachable_arm; _ cubierto por la unión de true y false anteriores sin guarda. |
| E10c | E10b eliminando solo el último brazo _. | PASS | —; exhaustivo y sin brazo cubierto. |
| E11 | `if b { drop(r); } inspect(r);` | FAIL | NX-E0202/maybe_consumed. |
| E12 | `if b { drop(r); return (); } inspect(r);` | PASS | —; solo continuación propietaria llega a inspect. |
| E13 | `let r1 = mock_open(1); let r2 = mock_open(2); let elegido = if b { value: consume r1; } else { value: consume r2; }; inspect(elegido); drop(elegido);` | PASS | —; cierra elegido y luego no elegido una sola vez. |
| E14 | `let r1 = mock_open(1); let r2 = mock_open(2); let elegido = if b { value: consume r1; } else { value: consume r2; }; inspect(r1);` | FAIL | NX-E0202/maybe_consumed. |
| E15 | Falta fixture de vista de rama dentro de error/wrapper con tipo concreto. | NO EVALUADO | Objetivo NX-E1307; tipos de vista indefinidos no cuentan. |
| E16 | Fuente E16 abajo. | FAIL | NX-E0412; posible alloc por contrato, incluso con condición false. |
| E17 | Falta recurso con destructor bloqueante retenido por closure de rama. | NO EVALUADO | Objetivo negativo NX-E0413. |
| E18 | Fuente E18 abajo. | FAIL | NX-E0414, next → yield. |
| E19 | `let f = if b { value: fn []() -> i64 => return 1; } else { value: fn []() -> i64 => return 2; };` | FAIL | NX-E1303; tipos anónimos diferentes. |
| E20 | `value: 1;` en la plantilla de sentencias | FAIL | NX-E1302/invalid_value_target. |
| E21 | `fn f(b: bool) -> i64 { if b { return 1; } }` | FAIL | **NX-E1317**, camino false cae sin return; parsing válido. |
| E21b | `fn f(b: bool) -> i64 { if b { return 1; } return 0; }` | PASS | —; reparación pareada de E21. |
| E22 | `fn f() -> i64 { 1; }` | FAIL | NX-E1317; descarte válido seguido de caída, no error de sintaxis. |
| E23 | `fn caso() { var o: Option<i64> = Some(1); match o { Some(inout n) => { n = 2; }, None => {}, } }` | FAIL | NX-E1105/permission_escalation; patrón mutable sin match inout. |
| E23b | E23 sustituyendo solo `match o` por `match inout o`. | PASS | —; permiso explícito sobre binding mutable. |
| E24 | Pendiente: retornar de un local una vista dentro de Caja<T>, sin closure/futuro ni rama. | NO EVALUADO | Objetivo NX-E1318; falta definir tipo/constructor de vista ordinaria y fixture de origen. No se usa una closure que desviaría el caso a NX-E1104. |
| E25 | Fuente E25 abajo: guardar F posiblemente prestado sin retains. | FAIL | NX-E1319; comprobar cuerpo genérico antes de instanciar. |
| E25b | E25 añadiendo `retains(v, in: owner)` después de `-> ()`. | PASS | —; declara retención, llamadores aún deben probar duración. |
| E26 | Fuente E26 abajo: from(x) no cubre procedencia de v. | FAIL | NX-E1320; cláusula escrita privada insuficiente. |
| E26b | E26 sustituyendo solo `from(x)` por `from(v)`. | PASS | —; conserva todas las procedencias del F transferido. |
| E27 | Pendiente: llamar E25b con vista de local cuyo owner sobrevive al local. | NO EVALUADO | Objetivo NX-E1318; falta la misma fixture de vista que E24. Prueba distinta: contrato válido no demuestra duración del llamador. |
| E28 | `gen fn g() -> i64 { yield 1; }` | PASS | —; i64 es Item, final normal sin return, compatible con EX04. |
| E29 | `gen fn g(b: bool) -> i64 { if b { return; } yield 1; }` | PASS | —; retorno vacío opcional, limpieza y fin sin elemento adicional. |
| E30 | `gen fn g() -> i64 { return 1; }` | FAIL | NX-E1324; return no es yield ni canal de resultado final. |
| E30b | `gen fn g() -> i64 { yield 1; return; }` | PASS | —; reparación explícita de E30. |
| E30c | `gen fn g() -> i64 { return (); }` | FAIL | NX-E1324; también se excluye un operando unidad. |
| E31 | `fn f() -> i64 { 1 }` | FAIL | NX-E0005, token/spans de S31 abajo; aún no se analiza caída sin retorno. |
| E31b | `fn f() -> i64 { return 1; }` | PASS | —; reparación explícita completa de E31. |
| E32 | `fn f(b: bool) -> i64 { return if b { 1 } else { value: 2; }; }` | FAIL | NX-E0005, primera llave después de 1; S32 fija span. |
| E32b | E32 sustituyendo `{ 1 }` por `{ value: 1; }`. | PASS | —; destinatario value: comprobado. |
| E33 | Fuente E33 con salto inmediatamente después de return. | FAIL | NX-E0102/type_mismatch, span del return, esperado i64/encontrado unidad. |
| E33b | Fuente E33b con apertura de paréntesis antes del salto. | PASS | —; operando de return conservado. |
| E34 | Fuente E34 con salto inmediatamente después de value:. | FAIL | NX-E1302/missing_value_operand, span del value: vacío. |
| E34b | Fuente E34b con apertura de paréntesis antes del salto. | PASS | —; entrega valor a la rama. |
| E35 | Fuente E35 con expresión final terminada por salto de línea. | FAIL | NX-E1317; el salto termina el descarte sin convertirlo en retorno. |
| E36 | Fuente E36 abajo: guarda llama a cambiar(inout x). | FAIL | NX-E1305/guard_mutation; la función auxiliar no tiene efectos prohibidos, pero modifica x. |
| E36b | E36 reemplazando cambiar por `fn cambiar(x: i64) -> bool with no_alloc, no_block, no_suspend, no_io { return true; }` y llamando `cambiar(x)`. | PASS | —; guarda solo lee. |
| E37 | Fuente E37 abajo: guarda transfiere x a tomar. | FAIL | NX-E1305/guard_transfer; transferencia incluso de un i64 copiable. |
| E37b | E37 quitando consume de la firma de tomar y del argumento. | PASS | —; préstamo ordinario, misma exhaustividad. |
| E38 | Fuente E38 abajo: cota pública sin no_heap_alloc. | FAIL | NX-E0416; no se declara no_alloc, por lo que no puede sustituirlo NX-E0412. |
| E38b | E38 añadiendo no_heap_alloc a la firma de posible (sustituye unrestricted). | PASS | —; cuerpo escalar satisface la nueva cota pública. |
| E39 | `fn f(b: bool) -> i64 { return match b { true => 1, false => { value: 0; }, }; }` | FAIL | NX-E0002; unexpected_token = 1, span S39; brazo sin Block, no closure implícita. |
| E39b | E39 sustituyendo solo el primer 1 por { value: 1; }. | PASS | —; brazo con bloque y entrega explícita. |
| M01 | Fixture M01 abajo, alias Externo de a.Dato. | PASS | —; identidad original. |
| M02 | Fixture M02 abajo, acceso del hermano cliente. | FAIL | NX-E1308/private_access. |
| M02b | Fixture M02 sin módulo cliente; hijo conserva acceso al campo. | PASS | —; descendiente del módulo definidor permitido. |
| M03 | Fixture M03 abajo, pub use de elemento pub en módulo privado. | PASS | —; nueva ruta sin identidad nueva. |
| M04 | Fixture M04 propia: salida reexporta un privado accesible de su padre. | FAIL | NX-E1308/private_reexport; no hay fallo previo de acceso ni uso del alias inválido. |
| M04b | M04 haciendo pub la declaración Dato de la raíz. | PASS | —; destino accesible y público. |
| M05 | Fixture M05 abajo, colisión entre a.Dato y b.Dato. | FAIL | NX-E1309/alias_collision. |
| M06 | Fuente M06 abajo, error privado en firma pública. | FAIL | NX-E1310/unexported_type. |
| M07 | Falta fixture pública de vista válida retornada sin from. | NO EVALUADO | Objetivo NX-E1310/missing_provenance; falta tipo/constructor de vista de API. Una cláusula from escrita incorrecta se comprueba separadamente con NX-E1320 (E26). |
| M08 | Fuente M08 abajo, falta cancel. | FAIL | NX-E1202, propietario único D12 en catálogo. |
| M09 | Fixture M09: p usa b sin arista directa. | FAIL | NX-E1311/undeclared_dependency. |
| M09b | M09 añadiendo la arista directa p → b ya resuelta. | PASS | —; misma identidad, acceso explícito. |
| M10v | Fixture M10v: p y a exigen revisiones distintas de b. | FAIL | NX-E1311/incompatible_versions. |
| M10vb | M10v cambiando el requisito de a a b@1 y eliminando b@2 del grafo. | PASS | —; única revisión seleccionada. |
| M10c | Fixture M10c: p → a → p. | FAIL | NX-E1311/package_cycle. |
| M10cb | M10c eliminando la arista a → p. | PASS | —; grafo acíclico, fuentes intactas. |
| M11 | Fixture M11: impl extranjera mediante alias. | FAIL | NX-E1312. |
| M12 | Fixture M12: impl para contenedor extranjero de tipo local. | FAIL | NX-E1312. |
| M13 | Fixture M13: wrapper nominal declarado en p. | PASS | —; propietario autorizado del tipo exterior. |
| M14 | Fixture M14: impl genérica de a y concreta de b se solapan. | FAIL | NX-E1313; oculto.nx registrado sin import cliente. |
| M14b | M14 eliminando la impl de b; conserva Dato y aristas. | PASS | —; solo la impl canónica de a. |
| M15 | Fixture M15: dos extensiones igualmente aplicables. | FAIL | NX-E1314. |
| M15b | M15 cambiando `d.leer()` por `extension e1.Dato.leer(d)`. | PASS | —; calificación explícita sin prioridad por importación. |
| M16 | Variante M16 de M15: solo e1 declarada, sin use extension. | FAIL | NX-E1315. |
| M16b | M16 añadiendo `use extension e1.Dato;`. | PASS | —; activación explícita. |
| M17 | Fuente M17 abajo: const i64 llama a numero con salida print. | FAIL | NX-E1316; gramática y tipo válidos, efecto io transitivo excluido de comptime. |
| M17b | Archivo raíz: `const UNO: i64 = comptime 1; fn main() { let n = UNO; }` | PASS | —; constante pura, sin inicialización global en ejecución. |
| M17c | Fuente M17 eliminando solo `print(1);`. | PASS | —; inicializador i64 puro, positivo pareado. |
| M18 | Reexportación de afinidad y consumidor migrable incompatible. | NO EVALUADO | Objetivo NX-E1109. Falta un constructor y contrato concretos de LocalRc/recurso afín exportable: un struct de datos vacío no prueba afinidad y un inventado impl Send violaría D08. Requiere fixture de la base confiada D09, distinta del caso de efectos M19. |
| M19 | Dos revisiones y pasos de caché de M19 abajo. | NO EVALUADO | Objetivo: ejecución fría B y caliente A→B coinciden en NX-E0412; falta arnés incremental y formato de interfaz. La fuente está fijada, pero un único compile no prueba invalidación. |
| M20 | Fixture M20: dos declaraciones del mismo archivo. | FAIL | NX-E1321/duplicate_module_file. |
| M20b | M20 eliminando la segunda `mod a;`. | PASS | —; un módulo por archivo. |
| M21 | Fixture virtual M21: a.nx y A.nx distintos con igual clave portable. | FAIL | NX-E1321/path_collision. |
| M21b | M21 cambiando `mod A;` por `mod b;` y renombrando A.nx a b.nx. | PASS | —; claves distintas. |
| M22 | Solo p/raiz.nx: `mod ausente; fn main() { return (); }`; ningún ausente.nx. | FAIL | NX-E1321/module_file_missing. |
| M22b | M22 añadiendo p/ausente.nx vacío. | PASS | —; Module permite cero declaraciones. |
| M23 | Fuente M23: dos métodos iguales de extensión en el mismo módulo. | FAIL | NX-E1322. |
| M23b | M23 eliminando el segundo bloque extend. | PASS | —; método único. |
| M24 | Raíz: `type A = B; type B = A; fn main() { return (); }` | FAIL | NX-E1323/alias_cycle. |
| M24b | M24 cambiando `type B = A;` por `type B = i64;`. | PASS | —; alias acíclicos. |
| M25 | Raíz: `const A: i64 = comptime B; const B: i64 = comptime A; fn main() { return (); }` | FAIL | NX-E1323/constant_cycle. |
| M25b | M25 cambiando el inicializador de B por `comptime 1`. | PASS | —; evaluación pura acíclica. |
| M26 | Raíz: `struct Nodo { hijo: Nodo } fn main() { return (); }` | FAIL | NX-E1323/recursive_value_layout. |
| M26b | M26 cambiando el tipo del campo hijo por i64. | PASS | —; tamaño finito. |
| M27l | Fuente M27 abajo con let en la tercera línea de raíz. | FAIL | NX-E0002, token let y span S27l; prefijo ConstDecl + FnDecl reconocido. |
| M27v | Fuente M27 sustituyendo solo let por var. | FAIL | NX-E0002, token var y span S27v; mismo prefijo reconocido. |
| M27b | Fuente positiva M27b abajo, binding dentro de main. | PASS | —; no inicializador global. |
| M28 | Fixture M01 duplicando la línea `use a.Dato as Externo;` del cliente p. | FAIL | NX-E1309/duplicate_import; mismo destino y alias, no alias_collision. |
| M28b | Fixture M01 sin esa duplicación. | PASS | —; import único. |
| M29 | `pub fn f() -> i64 { return 1; }` | FAIL | NX-E1310/missing_effect_bound; resultado público sí anotado. |
| M29b | M29 añadiendo `with no_alloc, no_block, no_suspend, no_io` antes del cuerpo. | PASS | —; firma pública completa. |
| M29c | M29 añadiendo solo with no_alloc antes del cuerpo. | PASS | —; firma completa con una prohibición explícita, como M14/M15. |
| M29d | M29 añadiendo with unrestricted antes del cuerpo. | PASS | —; firma completa sin prohibiciones, como revisión B de M19. |
| M30 | `pub fn f() with no_alloc, no_block, no_suspend, no_io { return 1; }` | FAIL | NX-E1310/missing_signature_type; falta -> i64, no error de parser. |
| M30b | M30 añadiendo `-> i64` después de los parámetros. | PASS | —; resultado público explícito. |
| M31 | Fixture M31 abajo: declaraciones y requisito de trait terminados por salto de línea. | PASS | —; compatibilidad de terminadores con EX01/D08, incluidas firmas continuadas. |
| M31b | M31 con las cinco inserciones de ; indicadas abajo. | PASS | —; mismo AST que M31. |
| M32 | `trait Medida { fn valor(self) -> i64 with no_alloc }` | FAIL | NX-E0005; requisito completo sin terminador ante } en la misma línea, span S32m. |
| M32b | M32 insertando ; después de no_alloc. | PASS | —; requisito terminado explícitamente. |
| M33 | `struct Dato {} use package.*; fn main() { return (); }` como p/raiz.nx sin dependencias. | FAIL | NX-E0002; unexpected_token = *, span S33m; glob import excluido. |
| M33b | M33 sustituyendo * por Dato as Otro. | PASS | —; import enumerado con alias local distinto de Dato. |

### Compatibilidad verbatim con la base y ejemplo reducido

X01/X04 reciben como fuente **el bloque nexo completo original** de EX01/EX04
en el blob HEAD del roadmap 0.2, sin cambiar nombres, expresiones ni imports.
Los spans siguientes se cuentan desde su primera u de use, con sus LF
originales; las líneas mostradas debajo son extractos verbatim, no programas
aislados ni sustitutos. PASS es aceptación prevista de compilación conforme
a G + formas/APIs heredadas de la base; su ejecución con biblioteca real
sigue pendiente de F2. Ninguna fila declara ejecutado el compilador futuro.

| ID | Fuente | Resultado previsto | Token / span | Obligación |
|---|---|---|---|---|
| X01 | EX01 completo, roadmap líneas 87–109; líneas relevantes verbatim abajo. | PASS | Name(value) declaración [355,360); argumento [411,416) | let value y total.try_add(value) compilan sin escape/renombre; no son ValueStmt. |
| X04 | EX04 completo, roadmap líneas 207–223; líneas relevantes verbatim abajo. | PASS | Name(value) binding de for [244,249); interpolación [302,307) | for value e interpolación conservan sus identificadores y semántica. |
| X04r | `gen fn cuadrados() -> i64 { yield 1; yield 4; } fn main() { var g = cuadrados(); let value = Iterator.next(inout g); drop(g); }` | PASS | Name(value) [85,90) | EX04 reducido: generador propietario, avance explícito y binding value; no reemplaza el original completo. |

Extracto verbatim X01:

```nexo
        let value = parse.i64(field)?
        total = total.try_add(value)?
```

Extracto verbatim X04:

```nexo
    for value in cuadrados(limite: 4) {
        console.println("{value}")?
```

### Casos del marcador contextual — B1 de iteración 5

Cada fila es un módulo de biblioteca independiente. PASS/FAIL es expectativa
de la propuesta, no ejecución. Fuentes sin salto inicial y con LF real;
spans UTF-8 desde cero, fin excluido. En PASS el span identifica la selección
léxica; en FAIL es el primary_span del diagnóstico indicado. La anticipación
corresponde al token tras Name(value), contando LF antes de insertar terminadores.
Para value: se muestra el span del introductor completo.

| ID | Fuente | Resultado previsto | Token / span / anticipación | Diagnóstico y decisión |
|---|---|---|---|---|
| V01 | `fn f() -> i64 { let value = 2; return if true { value - 1; value: value; } else { value: 0; }; }` | PASS | value / [48,53) / - | —; Name; resta ordinaria, después entrega explícita. |
| V02 | `fn f() -> i64 { let value = 2; return if true { value -1; value: value; } else { value: 0; }; }` | PASS | value / [48,53) / - | —; Mismos tokens/AST que V01; el espacio no decide la gramática. |
| V03 | `fn value(x: i64) -> i64 { return x; } fn f() -> i64 { let x = 1; return if true { value (x); value: 2; } else { value: 0; }; }` | PASS | value / [82,87) / ( | —; Llamada al identificador value, incluso en rama destinataria; N3 conserva Call. |
| V04 | `fn f() -> i64 { let value = [1]; let i = 0; return if true { value [i]; value: 2; } else { value: 0; }; }` | PASS | value / [61,66) / [ | —; Índice del array, no operando de ValueStmt. |
| V05 | `fn f() -> i64 { var value = 1; return if true { value = 2; value: value; } else { value: 0; }; }` | PASS | value / [48,53) / = | —; Asignación ordinaria al binding mutable. |
| V06 | `fn f() -> i64 { var value = 1; return if true { value += 2; value: value; } else { value: 0; }; }` | PASS | value / [48,53) / += | —; Asignación compuesta D08, no marcador de rama. |
| V07 | `struct Caja { campo: i64 } fn f() -> i64 { let value = Caja(campo: 1); return if true { value.campo; value: 2; } else { value: 0; }; }` | PASS | value / [88,93) / . | —; Acceso a campo; nombre y permisos ordinarios. |
| V08 | `fn f() -> Result<i64, i64> { let value: Result<i64, i64> = Ok(1); return if true { value?; value: Ok(2); } else { value: Ok(0); }; }` | PASS | value / [83,88) / ? | —; Propagación de Result D02; value sigue siendo Name. |
| V09 | `fn f() -> i64 { let value = 1; return if true { value; value: 2; } else { value: 0; }; }` | PASS | value / [48,53) / ; | —; Lectura descartada; el binding visible no lo convierte en marcador. |
| V09b | `fn f() -> i64 { return if true { value; value: 2; } else { value: 0; }; }` | FAIL | value / [33,38) / ; | NX-E0101; Mismo árbol Name; ahora el nombre no resuelve. Span primario del Name. |
| V09c | `fn f() -> i64 { let value = 1; return if true { value; } else { value: 0; }; }` | FAIL | } / [55,56) / — | NX-E1302/missing_branch_value; El Name final se descarta; span primario del cierre de rama, no retorno implícito. |
| V10 | `fn value(value: i64) -> i64 { return if true { value: value; } else { value: 0; }; }` | PASS | value: / [47,53) / : | —; Nombre de función/parámetro legal. Primer value contextual; segundo Name del parámetro. |
| V11 | Fuente V11 abajo. | PASS | value / [64,69) / LF | —; LF termina el descarte ordinario porque la línea siguiente no continúa la expresión. |
| V12 | Fuente V12 abajo. | PASS | value / [98,103) / LF | —; No hay introductor; D01 permite que ( continúe la llamada ordinaria en la línea siguiente. |
| V13 | Fuente V13 abajo. | FAIL | value: / [45,51) / : | NX-E1302/missing_value_operand; Salto tras el introductor completo: operando vacío. Precede a non_final_branch_value derivado. |
| V13b | Fuente V13b abajo. | PASS | value: / [45,51) / : | —; Apertura antes del salto; ValueStmt final con operando. |
| V14 | `fn f() -> i64 { return if true { value: 1; 2; } else { value: 0; }; }` | FAIL | value: / [33,39) / : | NX-E1302/non_final_branch_value; Introductor reconocido; hay otra sentencia directa después, relacionada con el diagnóstico. |
| V14b | `fn f() -> i64 { return if true { value: 1; } else { value: 0; }; }` | PASS | value: / [33,39) / : | —; Reparación de V14: eliminar el descarte posterior. |
| V15 | `fn f() -> i64 { return if true { if true { value: 1; } value: 2; } else { value: 0; }; }` | FAIL | value: / [43,49) / : | NX-E1302/invalid_value_target; Bloque inmediato de if sentencia, no rama destinataria; no salta al if exterior. |
| V15b | `fn f() -> i64 { return if true { if true { 1; } value: 2; } else { value: 0; }; }` | PASS | value: / [48,54) / : | —; Reparación de V15: descarte en el if sentencia y entrega al final de la rama exterior. |
| V16 | `fn f() -> i64 { let value = 2; return if true { value 1; } else { value: 0; }; }` | FAIL | 1 / [54,55) / — | NX-E0005; Name value reconocido como descarte sin terminador; el token 1 no continúa su expresión. No se conserva value Expr. |
| V16b | `fn f() -> i64 { let value = 2; return if true { value: 1; } else { value: 0; }; }` | PASS | value: / [48,54) / : | —; Reparación explícita de V16: insertar :, con comprobación completa. |

V16 exige parser_context = statement_terminator, expected_tokens = [";"]
y expected_tokens_kind = canonical_insertion. Debe reconocer la firma, el if
y el Name value antes de fallar en 1; V16b es su positivo completo. Los errores
de posición de V14/V15 no se sustituyen por parse error ni por falta de retorno.

Fuente V11:

```nexo
fn f() -> i64 {
    let value = 1;
    return if true {
        value
        value: 2;
    } else { value: 0; };
}
```

Fuente V12:

```nexo
fn value(x: i64) -> i64 { return x; }
fn f() -> i64 {
    let x = 1;
    return if true {
        value
        (x);
        value: 2;
    } else { value: 0; };
}
```

Fuente V13:

```nexo
fn f() -> i64 {
    return if true {
        value:
        (1);
    } else { value: 0; };
}
```

Fuente V13b:

```nexo
fn f() -> i64 {
    return if true {
        value: (
            1);
    } else { value: 0; };
}
```

### Ubicaciones exigidas para closures, terminadores y declaraciones

Las fuentes de celda son exactamente su código, sin backticks; M27 usa el
bloque raiz.nx sin comentarios ni líneas vacías iniciales. Offsets UTF-8
desde cero, fin excluido. Cada fixture exige alcanzar el token señalado.

| Referencia | Fila | unexpected_token / primary_span | Prefijo y expectativa exigidos |
|---|---|---|---|
| S39 | E39 | 1 / [48,49) | Firma completa, return match b { y true => reconocidos; parser_context = match_arm_body; expected_tokens = ["{"], expected_tokens_kind = admissible_set. |
| S33m | M33 | * / [27,28) | StructDecl y use package. reconocidos; parser_context = import_after_dot; expected_tokens = [Name, "{"], expected_tokens_kind = admissible_set. |
| S03 | E03 | No es error de token; operando 1 / [38,39) | ClosureExpr con ImplicitClosureBody completo; related_span de => = [35,37); exigir NX-E1301/implicit_closure_body, no error de parseo genérico. |
| S31 | E31 | } / [18,19) | Firma fn f() -> i64 y expresión 1 reconocidas; statement_terminator, expected_tokens = [";"] como inserción canónica, no conjunto exhaustivo (§2). |
| S32 | E32 | } / [39,40) | Return, if y expresión 1 de la primera rama reconocidos; statement_terminator, expected_tokens = [";"] como inserción canónica, no conjunto exhaustivo (§2). |
| S32m | M32 | } / [51,52) | TraitMember con firma y cota completas; declaration_terminator, expected_tokens = [";"] como inserción canónica (§2.1). |
| S27l | M27l | let / [54,57) | ConstDecl UNO y FnDecl main completas; module_declaration, expected_tokens = ModuleDeclStart (incluye "@"), expected_tokens_kind = admissible_set. |
| S27v | M27v | var / [54,57) | Mismo prefijo completo; module_declaration, expected_tokens = ModuleDeclStart (incluye "@"), expected_tokens_kind = admissible_set. |

ModuleDeclStart = [pub, const, type, fn, async, gen, struct, enum, trait, impl,
extend, mod, use, "@", EOF]. Let/var se acepta como inicio de sentencia local en
M27b, nunca de declaración global. Por tanto un rechazo anticipado de const,
comptime, i64 o de una segunda declaración no satisface M27l/M27v.

### Barrido de listas cerradas de expected_tokens — iteración 6

Las listas siguientes se contrastan con las posiciones de EX01–EX04 completas
(roadmap líneas 82–228), no solo con G reducida. Las expectativas del parser
describen continuaciones reconocibles en ese estado; algunas formas reconocidas
se rechazan después por una regla específica, como un mapa vacío. No son una
promesa de aceptación semántica. Las inserciones canónicas y categorías léxicas
se distinguen de listas exhaustivas. No se ejecutó un parser ampliado.

| Posición / referencias | Conjunto y contraste con EX01–EX04 |
|---|---|
| Inicio de declaración: S27l/S27v | ModuleDeclStart incluye use (los cuatro), fn (los cuatro), struct (EX02/EX03), gen (EX04) y @ (EX02, @derive(JsonEncode)); además las declaraciones nuevas de G.3 y EOF. BaseAttribute conserva el atributo sin añadir otra forma de declaración. |
| Inicio de with: S37d | {, unrestricted y los cinco no_*; EX03 empieza la lista simple con no_alloc. With de sentencia seguido de ps.prepare_unique() as view es otra producción heredada, no el estado de una cláusula de firma. |
| Restricción tras coma: S37 | unrestricted y los cinco no_*; EX03 usa no_block, no_suspend y no_io. Un unrestricted posterior se reconoce para NX-E0417/unrestricted_mixed, no se acepta como mezcla. |
| Operación tras with {: S37c | create/copy/call/resume/cancel/drop y }; EX01–EX04 no usan mapas por operación. } reconoce el mapa vacío rechazado por NX-E0417/empty_operation_list. |
| Restricción inicial de operación: S56e | unrestricted y los cinco no_*; no hay mapas de operación en EX01–EX04. Se reutiliza el mismo vocabulario de límites simples de §6.1. |
| Después de punto de import: S33m | Name o {; EX01/EX02/EX04 usan rutas con Name y EX03 usa std.table.{...}. No se admite *, que ninguna aceptación de la base necesita. |
| Cuerpo de brazo: S39 | {; EX01–EX04 no contienen match. Arm exige bloque y no hereda expresión de closure; => Expr sigue excluido de brazos. |
| Después de nombre capturado: S57 | , o ]; EX01–EX04 no contienen listas de capturas. Sus accesos ordinarios .campo no convierten Capture en un Place. |
| Terminadores: S31/S32/S32m/S44/V16 | [";"] es canonical_insertion, no conjunto cerrado admisible. Permanecen saltos de línea, continuaciones y el cierre de Field propios de §2/§2.1/G.3, usados por los cuatro ejemplos. |
| Lexer K01 | [valid_token] es lexical_category de F0-0001, no una enumeración de tokens de G ni una prohibición de @, cadenas u otros tokens heredados. |
| Cuerpo de main K02 | ["{"] es admissible_set **solo con diagnostic_namespace = F0-0001**, donde la cabecera termina en main(). En G, tras fn main(), las continuaciones reconocibles son ["->", from, result_from, retains, with, where, "{"]; EX01/EX02/EX03/EX04 usan -> y EX03 muestra with después de los parámetros de avanzar. No se aplica la lista del prototipo a esas fuentes. |

El conjunto ModuleDeclStart se define una vez arriba; S27l/S27v lo comparan
completo, incluido @. K02 conserva su mapa histórico y no puede aprobarse o
rechazarse frente al conjunto de otro dialecto. Cada futuro cambio en una
producción de estas posiciones debe actualizar la lista y su fixture asociada.

Fixture M31: paquete único p, raíz raiz.nx; no dependencias. Los saltos
que terminan declaraciones/requisitos son LF reales, incluido el salto
después de no_alloc. La línea with continúa la firma anterior; la apertura
del cuerpo de cero continúa su firma pese al salto.

```nexo
// p/raiz.nx
mod datos
use datos.Dato as Local
type Numero = i64
const UNO: Numero = comptime 1
trait Medida {
    fn valor(self) -> Numero
        with no_alloc
    fn cero(self) -> Numero with unrestricted
    {
        return UNO;
    }
}
impl Medida for Local {
    fn valor(self) -> Numero with no_alloc { return UNO; }
}
fn main() { let d = Local(n: UNO); let n = d.valor(); }
```

```nexo
// p/datos.nx
pub struct Dato { pub n: i64 }
```

M31b añade ; al final de las cuatro primeras declaraciones (mod/use/type/const)
y tras no_alloc del requisito valor, conservando todos los demás saltos.
Debe producir el mismo AST que M31, incluidos la cota de valor y el cuerpo
por defecto de cero. No se inserta ; entre la firma de cero y su cuerpo.

Fuentes de salto de línea (cada bloque es un módulo independiente de biblioteca).
Los saltos son bytes LF reales, no escapes en un string:

```nexo
// E33
fn f() -> i64 {
    return
    (1);
}
```

```nexo
// E33b
fn f() -> i64 {
    return (
        1
    );
}
```

```nexo
// E34
fn f(b: bool) -> i64 {
    return if b {
        value:
        (1);
    } else { value: 2; };
}
```

```nexo
// E34b
fn f(b: bool) -> i64 {
    return if b {
        value: (
            1
        );
    } else { value: 2; };
}
```

```nexo
// E35
fn f() -> i64 {
    1
}
```

Fuente E36:

```nexo
fn cambiar(inout x: i64) -> bool with no_alloc, no_block, no_suspend, no_io {
    x = 1;
    return true;
}
fn f() -> i64 {
    var x = 0;
    return match true {
        true if cambiar(inout x) => { value: 1; },
        _ => { value: 0; },
    };
}
```

Fuente E37:

```nexo
fn tomar(consume x: i64) -> bool with no_alloc, no_block, no_suspend, no_io {
    return true;
}
fn f() -> i64 {
    let x = 1;
    return match true {
        true if tomar(consume x) => { value: 1; },
        _ => { value: 0; },
    };
}
```

Fuente E38:

```nexo
pub fn posible() -> i64 with unrestricted { return 1; }
fn f() -> i64 with no_heap_alloc { return posible(); }
```

Fuente E16. Una llamada a interfaz pública usa su cota declarada; no consulta
el cuerpo para fortalecerla en el consumidor. Unrestricted permite alloc aunque
este cuerpo concreto no lo ejecute; aquí se prueba imposibilidad de verificar
el contrato, no una medición de reservas:

```nexo
pub fn posible() -> i64 with unrestricted { return 1; }
fn caso() -> i64 with no_alloc {
    return if false { value: posible(); } else { value: 0; };
}
fn main() { let n = caso(); }
```

Fuente E18:

```nexo
gen fn uno() -> i64 { yield 1; }
fn caso(b: bool) -> Option<i64> with no_suspend {
    var g = uno();
    return if b { value: Iterator.next(inout g); } else { value: None; };
}
fn main() { let n = caso(true); }
```

Fuentes E25/E26: no utilizan un tipo de vista inventado. F es un parámetro
de tipo sin requisito de independencia; puede contener procedencias simbólicas.
Cada bloque se comprueba como módulo privado independiente, sin instanciación.

```nexo
// E25
fn guardar<F>(inout owner: Option<F>, consume v: F) -> () {
    owner = Some(consume v);
    return ();
}
fn main() { return (); }
```

```nexo
// E26
fn elegir<F>(x: i64, consume v: F) -> F from(x) {
    return consume v;
}
fn main() { return (); }
```

### Fuentes de módulos M01–M08

El arnés F0 recibe paquetes con estos archivos exactos, raíz raiz.nx y las
aristas declaradas aquí. No necesita deducir la sintaxis futura de nexo.toml.
Cada caso usa un directorio de fixture nuevo; no son escrituras en este repo.

M01: paquete a sin dependencias; paquete p depende directamente de a.

```nexo
// a/raiz.nx
pub struct Dato {
    pub n: i64
}
// p/raiz.nx
use a.Dato as Externo;
fn main() { let x = Externo(n: 1); }
```

M02: un paquete p, sin dependencias. El caso positivo M02b elimina exactamente
`mod cliente;` y el archivo cliente.nx; los otros archivos se conservan.

```nexo
// p/raiz.nx
pub mod modelo;
mod cliente;
fn main() { return (); }
// p/modelo.nx
pub struct Dato {
    n: i64
}
pub mod hijo;
// p/modelo/hijo.nx
pub fn leer(d: super.Dato) -> i64 with unrestricted { return d.n; }
// p/cliente.nx
use package.modelo.Dato;
fn leer(d: Dato) -> i64 { return d.n; }
```

M03: un paquete p sin dependencias. M04 tiene su propia fixture más abajo;
no se obtiene quitando pub del hijo interno, pues eso fallaría por acceso.

```nexo
// p/raiz.nx
mod interno;
pub use package.interno.Dato;
fn main() { let d = Dato(n: 1); }
// p/interno.nx
pub struct Dato {
    pub n: i64
}
```

M04: un paquete p sin dependencias. salida es descendiente del módulo raíz
que define Dato y puede acceder a él, pero no elevar su visibilidad. Main
no utiliza el alias rechazado. M04b cambia solo struct Dato por pub struct Dato.

```nexo
// p/raiz.nx
struct Dato { pub n: i64 }
pub mod salida;
fn main() { return (); }
// p/salida.nx
pub use package.Dato as Publico;
```

M05: a y b sin dependencias; p declara dependencias directas a y b.

```nexo
// a/raiz.nx
pub struct Dato { pub n: i64 }
// b/raiz.nx
pub struct Dato { pub n: i64 }
// p/raiz.nx
use a.Dato;
use b.Dato;
fn main() { return (); }
```

M06: un único archivo raíz, sin dependencias.

```nexo
struct Error {}
pub fn fallar() -> Result<i64, Error> with unrestricted {
    return Err(Error());
}
fn main() { return (); }
```

M08: un único archivo raíz, sin dependencias.

```nexo
pub async fn dato() -> i64
    with { create(unrestricted), resume(unrestricted), drop(no_suspend) } {
    return 1;
}
fn main() { return (); }
```

### Fuentes de paquetes M09–M16

Todas las raíces se llaman raiz.nx. El grafo que antecede cada bloque es dato
exacto del arnés, no sintaxis de nexo.toml. Cada nombre identifica un paquete
de origen fixture, revisión 1 salvo indicación expresa. Las aristas dan aliases
del mismo nombre. No se cargan archivos de otra fixture.

M09: p → a; a → b; b sin dependencias. b pertenece al grafo, pero no al ámbito
de dependencias directas de p. El positivo M09b añade p → b.

```nexo
// b/raiz.nx
pub struct Dato { pub n: i64 }
// a/raiz.nx
pub use b.Dato;
// p/raiz.nx
use b.Dato;
fn main() { let d = Dato(n: 1); }
```

M10v: requisitos exactos p → a@1 y b@1; a@1 → b@2. Ambas revisiones de b
tienen el mismo origen fixture y nombre b; no hay una selección que satisfaga
el grafo. La revisión no se deduce del nombre de directorio: el arnés la fija.

```nexo
// b@1/raiz.nx
pub struct Dato { pub n: i64 }
// b@2/raiz.nx
pub struct Dato { pub n: i64 }
// a@1/raiz.nx
pub use b.Dato;
// p/raiz.nx
fn main() { return (); }
```

M10c: p → a y a → p, sin otras aristas. El rechazo ocurre al construir el
grafo, no necesita un import ni un ciclo de funciones.

```nexo
// p/raiz.nx
fn main() { return (); }
// a/raiz.nx
pub struct Dato { pub n: i64 }
```

M11/M12/M13: a sin dependencias; p → a. El archivo a es idéntico para los
tres casos; cada archivo p siguiente constituye una fixture distinta.
AliasDecl usa la producción type de §8; un alias nunca declara tipo nominal.

```nexo
// a/raiz.nx (común)
pub trait Marca {
    fn marca(self) -> i64 with no_alloc, no_block, no_suspend, no_io;
}
pub struct Dato { pub n: i64 }
pub struct Caja<T> { pub item: T }
```

```nexo
// M11: p/raiz.nx
use a.Marca;
type Alias = a.Dato;
impl Marca for Alias {
    fn marca(self) -> i64 with no_alloc, no_block, no_suspend, no_io {
        return 1;
    }
}
fn main() { return (); }
```

```nexo
// M12: p/raiz.nx
use a.Marca;
struct Local { n: i64 }
impl Marca for a.Caja<Local> {
    fn marca(self) -> i64 with no_alloc, no_block, no_suspend, no_io {
        return 1;
    }
}
fn main() { return (); }
```

```nexo
// M13: p/raiz.nx
use a.Marca;
struct Local { dato: a.Dato }
impl Marca for Local {
    fn marca(self) -> i64 with no_alloc, no_block, no_suspend, no_io {
        return 1;
    }
}
fn main() { let d = Local(dato: a.Dato(n: 1)); let n = Marca.marca(d); }
```

M14: p → b; b → a; a sin dependencias. El paquete a es propietario de Marca
y puede declarar su impl genérica; b es propietario de Dato y puede declarar
la concreta. Ninguna es huérfana por separado: el defecto es solapamiento.
El módulo privado oculto está declarado aunque ningún cliente lo importe.

```nexo
// a/raiz.nx
pub trait Marca { fn marca(self) -> i64 with no_alloc; }
mod oculto;
// a/oculto.nx
impl<T> package.Marca for T {
    fn marca(self) -> i64 with no_alloc { return 1; }
}
// b/raiz.nx
pub struct Dato { pub n: i64 }
impl a.Marca for Dato {
    fn marca(self) -> i64 with no_alloc { return 2; }
}
// p/raiz.nx
fn main() { return (); }
```

M15: p → a, e1, e2; e1 → a; e2 → a. Ambos paquetes de extensión reexportan
la misma identidad Dato. La ruta de activación identifica el módulo que
declara extend además de ese tipo; no una copia del tipo.

```nexo
// a/raiz.nx
pub struct Dato { pub n: i64 }
// e1/raiz.nx
pub use a.Dato;
extend Dato { pub fn leer(self) -> i64 with no_alloc { return 1; } }
// e2/raiz.nx
pub use a.Dato;
extend Dato { pub fn leer(self) -> i64 with no_alloc { return 2; } }
// p/raiz.nx
use a.Dato;
use extension e1.Dato;
use extension e2.Dato;
fn main() { let d = Dato(n: 0); let n = d.leer(); }
```

M16 usa exactamente a/raiz.nx y e1/raiz.nx de M15, aristas p → a, e1 y
e1 → a, y este único cliente; e2 no pertenece al grafo:

```nexo
// p/raiz.nx
use a.Dato;
fn main() { let d = Dato(n: 0); let n = d.leer(); }
```

M17: módulo único sin dependencias. i64 es un tipo nombrable del núcleo;
no se presupone que MockResource tenga sintaxis de tipo. print es la primitiva
de salida del arnés F0-0001: para el modelo de efectos de esta fixture su
contrato es lectura del argumento, resultado unidad y efecto io. No se
ejecuta durante comptime; el comprobador detecta el efecto transitivo.
M17c elimina solo `print(1);`; el resto de la fuente permanece idéntico.

```nexo
fn numero() -> i64 { print(1); return 1; }
const R: i64 = comptime numero();
fn main() { return (); }
```

### Fuentes y procedimiento de caché M19

Grafo p → q → a. Dos estados secuenciales del mismo grafo; nunca se seleccionan
ambas revisiones de a simultáneamente. q y p no cambian al pasar de A a B.
El identificador/contenido de interfaz de a sí debe cambiar.

```nexo
// a/raiz.nx, revisión A
pub fn leer() -> i64 with no_alloc { return 1; }
// q/raiz.nx, idéntico en A y B
pub use a.leer;
// p/raiz.nx, idéntico en A y B
fn main() with no_alloc { let n = q.leer(); }
```

```nexo
// a/raiz.nx, revisión B: sustituye exclusivamente el archivo de A
pub fn leer() -> i64 with unrestricted { return 1; }
```

El arnés deberá: comprobar A en frío y conservar su caché (resultado previsto
PASS); actualizar a a revisión B y comprobar sin borrar caché (FAIL NX-E0412);
comprobar B en frío y comparar código, contract_origin y cadena p → q → a.
El cuerpo idéntico no permite reforzar la interfaz pública unrestricted.
Sigue NO EVALUADO porque no hay protocolo ejecutable de caché/interfaz; estas
fuentes no acreditan por sí solas que una invalidación ocurra.

Fuente M27: contenido exacto de raiz.nx, sin cabecera de comentario. El
front-end debe haber consumido la ConstDecl y la FnDecl completas antes de
rechazar let/var; el parser F0-0001 que rechaza const no satisface este caso.

```nexo
const UNO: i64 = comptime 1;
fn main() { return (); }
let x: i64 = UNO;
```

Fuente M27b (positivo):

```nexo
const UNO: i64 = comptime 1;
fn main() { let x: i64 = UNO; return (); }
```

### Fuentes de carga de módulos y extensiones M20–M23

M20: un paquete p; ambas mod seleccionan el mismo archivo canónico.

```nexo
// p/raiz.nx
mod a;
mod a;
fn main() { return (); }
// p/a.nx
const UNO: i64 = comptime 1;
```

M21: el sistema virtual del arnés declara p/a.nx y p/A.nx como archivos con
identidades distintas. La clave portable de §8 coincide. No depende de poder
crearlos simultáneamente en el disco Windows.

```nexo
// p/raiz.nx
mod a;
mod A;
fn main() { return (); }
// p/a.nx
const UNO: i64 = comptime 1;
// p/A.nx
const DOS: i64 = comptime 2;
```

M23: un paquete p sin dependencias. No hay llamada ambigua ni imports;
ambos métodos pertenecen al mismo módulo y tipo.

```nexo
// p/raiz.nx
struct Dato { n: i64 }
extend Dato { fn leer(self) -> i64 { return 1; } }
extend Dato { fn leer(self) -> i64 { return 2; } }
fn main() { return (); }
```

Las filas NO EVALUADO mantienen sus puertas y códigos objetivo. Se requieren
fixtures de grafo, permisos, recursos y caché antes de contarlas como evidencia;
no basta cambiar un import hasta que el parser falle. M11/M12/M14 deben repetir
el mismo resultado al permutar imports/aliases/reexportaciones. Cada fallo
corregido dejará negativo y reparación positiva al implementar.

## Diagnósticos y reparación comprobable

El [catálogo único](F0-CODIGOS.md) es la única definición de códigos, propietarios
y subcausas, incluidas sus referencias históricas. Las filas usan el par exacto
code/subcause; NX-E1317 distingue ausencia de return por flujo de la forma
sintáctica NX-E1301. NX-E0202 es común a recursos, closures, futuros y ramas.

El dato estructurado añade tipo esperado/encontrado, camino sin valor,
testigo no cubierto, origen de consumo y punto de unión; para módulos,
identidad canónica, ruta exportada/importada, declaraciones en conflicto,
grafo y versión de interfaz. No se traduce un error de acceso privado en
un nombre desconocido si la causa real es visibilidad.

Agregar return/value: solo se ofrece como edición comprobable cuando el
destinatario del control y el resultado estén determinados. En E21 no se
inventa automáticamente un valor para la rama false: el positivo E21b es
una elección explícita del autor. Un brazo faltante recibe un patrón testigo,
nunca un panic/default inventado. Copiar, consumir, publicar o añadir una
dependencia cambia semántica/API y se explica como alternativa.

Una colisión puede repararse con alias/calificación y comprobando identidad
referida. No se repara coherencia reordenando imports u ocultando una impl.
Todo fix anunciado aplicable se vuelve a comprobar con tipos, propiedad,
efectos y observaciones. Los errores sintácticos deliberados se separan de
los negativos semánticos y de los casos aún NO EVALUADOS.

## Interacciones y alcance de implementación

* D11: closures cortas mantienen `return`; tipos anónimos distintos y propiedades
  de campos transitivas; una rama no borra recursos, afinidad o préstamos.
* D12: efectos por operación en firmas y reexportaciones; cancelación dentro
  de ramas conserva regiones/hijos y no se convierte en Result implícito.
* D08: identidad nominal, propiedad de impl, solapamiento y extensiones
  explícitas; ningún import altera permisos ni propiedades selladas.
* D09: if/match no producen unicidad/publicación a partir de un alias;
  unión conservadora y metadatos públicos preservan confinamiento.
* D10: patrones/argumentos conservan evaluación secuencial; yield y next
  mantienen suspend. Valores por defecto públicos siguen siendo comptime puros.
* P02/P03 y §6.1–6.2: propiedad, procedencia y efectos se comprueban antes y
  después de introducir temporales y limpiezas.

**Siguiente prototipo F0 previsto, no implementado en esta entrega:** parser
de if/match con value:, bool/Option/enum finito y tipos exactos; comprobador por
caminos con flags, recursos mock, efectos de ramas y préstamos reducidos.
Un grafo de dos/tres paquetes locales fijado por el arnés prueba nombres,
pub/use/pub use y coherencia de impl explícitas sin gestor público.
La frontera que aún no pueda comprobarse se rechaza como no soportada y su
requisito semántico queda NO EVALUADO.

F1: núcleo completo de módulos/patrones, traits estáticos, extensiones sin
ambigüedad y catálogo; F2: manifiesto/lockfile local, caché, formato, REPL,
doctests y generadores; F3: vistas/efectos/dyn completos; F4: publicación de
paquetes; F5: efectos de tareas/cancelación en biblioteca real.
No se promete resolver en F0 el gestor, solver completo, ABI ni todos los
contratos públicos. La gramática decidida aquí debe incorporarse a su
especificación revisable antes de promover fases.

| T asociadas | Evidencia exigida |
|---|---|
| T01/T02/T07/T08 | Copia/transferencia por rama, orden, tipos, return/?, inicialización parcial y cierre único. |
| T06/T10 | No esconder captura mutable, vista o futuro regional en alternativa, campo, error o exportación. |
| T14 | Efectos transitivos de ramas, llamadas indirectas y destructores; desconocido no vacío. |
| T22 | Next con yield bajo no_suspend, generadores abandonados/retornados en ramas y préstamos. |
| T09/T17/T18 | Coherencia de grafo, alias, extensiones, cambios de efecto/afinidad e invalidación. |
| T20/T21 | Derivación respeta privados/propiedades; etiquetas/defaults forman parte de API. |
| T23/T25 | Formato y archivo/REPL equivalentes; códigos, spans, testigos y fixes comprobados. |
| T26/T28 | Límites del análisis de ramas/grafo, cachés y rutas reproducibles en cuatro plataformas; pendientes de medir. |

T03–T05/T11–T13/T15/T16/T19/T24/T27 conservan sus contratos y fases del roadmap;
este ADR aporta fronteras de tipos/nombres pero no acredita sus pruebas de
arenas, unicidad, SIMD, historial, FFI, concurrencia, Python, biblioteca o web.
Las tres propuestas tampoco sustituyen los protocolos completos T01–T28 de F0.

## Riesgos abiertos y estado de revisión

Value evita retorno implícito, pero añade vocabulario y puede hacer if/match
más verbosos. Se necesita un piloto de lectura, formato y reparación; el
resultado puede exigir otra propuesta explícita, nunca reinterpretar D01.

La unión conservadora pierde correlaciones útiles y puede retener préstamos
de más. El prototipo debe aportar contraejemplos pequeños antes de ampliar
el análisis; no se promete inferencia global de duraciones ni identidades.

El contrato from/retains y los cuatro resúmenes async deben representar
composición real entre módulos, no solo casos locales. La compatibilidad de
enums exhaustivos públicos y una sola revisión por paquete es restrictiva;
la elección está tomada, pero su ergonomía necesita evidencia. Una revisión
futura deberá conservar coherencia y explicitar dependencias reabiertas.

Las seis revisiones anteriores fueron **FAIL**. Esta iteración 6 no tiene dictamen:
sus cambios y casos previstos están **NO EVALUADOS** por validación nueva.
Siguen pendientes las fixtures señaladas, los dos revisores sin discrepancias
de F0 y la aprobación del propietario. No se aprueba una fase por estas tablas.

## Registro de corrección — iteración 1

Motivos 1/2/10: subcausas canónicas y propiedad común, sin catálogo paralelo.
Motivos 4/5: gramática invocable de D11 y privacidad de campos con dominio
exacto. Motivo 6: value:/if/match y regla léxica return/value: declarados como
ampliaciones o cierre parcial de D01. Motivo 7: NX-E1317 con E21/E21b y E22.
Motivo 8: fuentes de ramas/módulos y pendientes NO EVALUADOS, incluidos M18/M19.
Motivo 9 se integra por referencia normativa a la sintaxis region/await de D12.

## Registro de corrección — iteración 2

Motivo 2: diagnósticos de procedencia genérica/retains/from, permisos de patrón,
carga de módulos, duplicación en extend y ciclos; casos E23–E27/M20–M26.
E24/E27 conservan NO EVALUADO con tipo de vista pendiente explícito. Motivo 3:
ConstDecl/AliasDecl y M17/M17b alcanzan reglas semánticas; la M17 anterior no
servía como negativo de inicialización. Motivos 4/6: ampliaciones enumeradas,
with por operación siempre con llaves y una EBNF en D11. Observación iv:
M09–M16 cuentan con grafo, archivos y reparaciones; M18 justifica por qué
falta la fixture confiada de afinidad; M19 fija fuentes y procedimiento, pero
sigue NO EVALUADO sin arnés incremental. No se declara ejecutado ningún caso.

## Registro de corrección — iteración 3

B2: §1.1 completa D10 con Item, caída al final y return vacío opcional;
return con operando se rechaza por NX-E1324 (E28–E30c). B3: M04 tiene acceso
legítimo al destino privado y falla solo al reexportarlo. B4: M17 usa i64 y
una llamada con io; M17c elimina ese único defecto.
B5: M27l/M27v y spans S27l/S27v exigen llegar al let/var global, con prefijo
válido. B6: §2 fija terminadores/caída, NX-E0005 y pares E31–E35, incluidos
saltos tras return/value:. La no inserción de terminador ante llave en la misma
línea es una **decisión de producto propuesta que completa D01**; no se deduce
de la base ni se introduce silenciosamente. Return vacío de generador y la
ausencia de un valor final son la concreción propuesta pendiente de D10.

N1 añade E36–E38 y M28–M30 para subcausas/códigos sin fila. N2/N3 quedan en
gramática G: procedencia invocable, llamadas, patrones y declaraciones. N4
refiere a presupuestos cerrados del catálogo. B1 incluye barrido de escapes
literales y encabezados. Permanecen sin ejecución y sin aprobación de fase.

## Registro de corrección — iteración 4

B1: ClosureBody reconoce => Expr como ImplicitClosureBody inválido; E03/S03
y E03b fijan diagnóstico, span y reparación. G.2 también explicita la
recuperación MissingTerminator y relaciona cada forma reconocida y rechazada
con su producción y chequeo; no se amplía el lenguaje aceptado con esas formas.

B2: el inventario transversal declara guardas, patrones con modo y match
inout/consume como ampliaciones; relaciona las demás grafías de G, D11 y
D12 con la base o la decisión que las añade. N2 completa D01 para terminación
de declaraciones/requisitos y continuidad de firmas (M31/M31b, M32/S32m/M32b).
Aclara que una cota pública ordinaria puede ser solo no_alloc o unrestricted
(M29c/M29d); las fases obligatorias de async/invocables siguen separadas.

N3: unreachable_arm usa la unión de patrones previos sin guarda (E10b/E10c);
S31/S32 identifican ; como inserción canónica, no como conjunto exhaustivo.
N4 revisa la presentación de `return`; N1/N5 se mantienen en el catálogo.
Se requiere revisión independiente de esta propuesta y materializar sus fixtures.

## Registro de corrección — iteración 5

B1 adopta (a): value es Name ordinario; solo Name(value) + : en la misma línea,
al inicio de una sentencia, reconoce ValueStmt. La aceptación exige posición
final directa de rama destinataria. El separador obligatorio elimina competencia
con llamadas, índices, resta, asignación, acceso y propagación sin consultar tipos.
Esta elección **corrige choque con EX01/EX04**. No se conserva value Expr;
se migraron ejemplos y reparaciones, incluidos los de D12 y E34/E34b.

V01–V16b fijan casos, tokens y spans, con LF real y prioridad de diagnósticos.
NX-E1302 añade la subcausa cerrada non_final_branch_value; no se renumeran
códigos. X01/X04 conservan fuente original verbatim, y X04r solo es reducido.
G hereda las formas de la base y conserva fases: EX01/EX04 F2, EX03 F3, EX02 F5.

N1 aclara nombre local de pub use (M03/M15). N2/N3/N4 se concretan en D12/D11
y son ampliaciones declaradas del preámbulo/invocables/capturas. N5 extiende
explícitamente la misma regla de retención a llamadas indirectas. N6 añade
E39/S39 y M33/S33m con positivos y prefijo reconocido. N7 corrige el catálogo.
T07/T17/T21/T22/T25 y el corpus léxico/formateador D01 deben materializar estas
regresiones; todavía no se ejecutaron ni acreditan una puerta de fase.

## Registro de corrección — iteración 6

B1 añade @ a ModuleDeclStart y a las expectativas completas de S27l/S27v.
BaseAttribute remite a @derive heredado; el barrido de todas las listas
expected_tokens está documentado junto a los spans, contra EX01–EX04.
Se distingue el dialecto de K02 y se incluyen continuaciones reconocedoras
de cotas inválidas en S37/S37c; las inserciones canónicas no se confunden
con conjuntos exhaustivos. No se reescribe ninguna aceptación de la base.

N2 concreta y declara la distinción método x.m() / campo invocable (x.m)()
y conserva Place agrupado; C52–C52d son sus regresiones de diseño. N3
relaciona la gramática reconocedora de cotas inválidas y yield fuera del
invocable gen con los diagnósticos únicos. N4 define Name desde el léxico
ASCII, retira E31c por duplicar E22 (ID retirado, no reutilizable) y remite
a las referencias S39/S33m corregidas en el catálogo. N1 se explica allí
como inalcanzabilidad del prototipo, no como fixture aislante pendiente.
Estas son expectativas documentales; quedan por materializar las pruebas,
la lectura/formato de métodos/campos y la validación independiente.
