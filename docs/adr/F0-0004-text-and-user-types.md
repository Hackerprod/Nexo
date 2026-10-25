# F0-0004 — Texto UTF-8 y tipos de usuario

Estado: **decisiones de arquitectura revisadas de forma independiente; sin implementación ni aprobación de fase**.

## 1. Autoridad, producto y frontera

Este corte permite una aplicación de consola de un archivo que obtiene líneas simuladas, las convierte en registros nominales, acumula enteros con errores tipados y emite texto. No aprueba F0/F1 ni habilita E/S real. No modifica `docs/baseline/` ni crea una revisión 0.3.

Autoridad: [roadmap 0.2](../baseline/Nexo_ROADMAP_rev0.2.md), P01–P09, D01–D10, EX01–EX04, F0/F1 y T01–T28; [acuerdos](../F0_ACUERDOS.md), §§2–4 y 9. La única superficie es [D15](F0-D15-sintaxis-oficial.md), que sustituye las grafías históricas de [0001](F0-0001-first-slice.md), [0002](F0-0002-second-slice.md), [D11](F0-D11-closures.md), [D12](F0-D12-async.md), [D13](F0-D13-expresiones-modulos.md) y [D14](F0-D14-sintaxis-superficie.md). En adelante A2 significa F0-0002 con las enmiendas B de D15. El [catálogo](F0-CODIGOS.md) sigue siendo el único registro de diagnósticos; §9 especifica inserciones propuestas para su integración, no un registro competidor.

Se adoptan valores independientes, alternativas nominales, exclusividad explícita, métodos con efectos y diagnósticos localizables. La conveniencia de biblioteca se construye sobre esas reglas. No hay semántica especial por nombre de programa, contenido de una fixture o cadena fuente.

### 1.1. Orden y dependencias con F0-0003 y F0-0005

El orden obligatorio es `I07c → I08 → I09 → I10a → I10b → I11a → I11b → F0-0003 (J01–J16) → F0-0005 (F5-L01–F5-L12) → F0-0004 (K01–K12)`. I08 aporta Result/propagación; I09, efectos y su verificación; I10, cuotas completas; I11, entorno y observaciones. Una declaración de arquitectura no certifica que esos pasos estén terminados.

[F0-0003](F0-0003-third-slice.md) es propietario de Option, match, patrones, closures y generadores. F0-0004 **no depende de una nueva sintaxis de patrones**, ni diseña otro comprobador de exhaustividad. Requiere estas capacidades de su interfaz terminada:

1. `Option<T>`, `Some(v)`, `None`; lectura, edición y selección consumidora conforme a D13 §4 y D15 §3.5.
2. Match sobre Result/Option, Bool e I64, bloques de brazo y `value:`, guardas y diagnósticos de cobertura. Tablas de constructores con identidad, aridad y tipos de payload; no decisiones codificadas por la escritura `Ok` o `Some`.
3. Un tipo esperado y una tabla de tipos por nodo compartidos con el checker de A2; no inferencia paralela en la selección ni en el lowering.
4. Extracción tras guarda satisfecha y limpieza del temporal de selección, incluidos payloads no transferidos. Préstamos por defecto de bindings de patrón.

F0-0003 se implementa y valida con sus propios tipos intrínsecos, sin requerir String, declaraciones struct/enum, métodos de usuario ni esta biblioteca. K07 incorpora descriptores de enums a su algoritmo existente; las pruebas de integración con texto/nominales pertenecen a K07/K12. F0-0003 no necesita ninguna capacidad de K01–K12 para cerrar su corte. Las referencias de integración son sus §§3, 6, 7 y 8: residuos de patrones, TestVariant/ProjectPayload/ExtractPayload, cuotas y NX-E1325. No se cambian sus subtareas. La revisión de integración contrasta ambos contratos antes de K01; una discrepancia exige conciliarlos, nunca una implementación alternativa de match.

F0-0004 depende de F0-0003 completo y de [F0-0005](F0-0005-operators-control-collections.md) completo. F0-0005 es autocontenido sobre A2/J: ningún F5 depende de un K. La aplicación de §11 usa operadores completos, constantes escalares, Array dinámico y for de rangos/arrays de F0-0005, además de Option/match y control de F0-0003. Capturas y generadores con String/nominales conservan sus contratos J. No se crea otro motor de tipos, propiedad, comparación o colecciones.

Obligaciones heredadas, con citas textuales de F0-0005 (los saltos editoriales se representan como espacios):

| Fuente y cita exacta | Regla y entrega de este ADR |
|---|---|
| §1.1: «String, nominales, derivación nominal de igualdad, sus diagnósticos y sus perfiles de texto/miembros quedan a cargo de F0-0004.» | §§3–5, 8–10: seis comparadores String, PartialEq nominal, NX-E1403 y perfil K acumulativo; K02–K08. |
| §1.1: «Cuando incorpore String o tipos nominales, deberá definir su elegibilidad transitiva como elementos de Array<T>, su igualdad y, si procede, su orden, y los costes/diagnósticos de esas extensiones en la tabla central de TypeId.» | §§3.2, 5.2, 7–9: elegibilidad por propiedades; igualdad por componente; orden solo I64/String; K03, K04, K08. |
| §1.1: «Para cada tipo que declare copiable e independiente, la elegibilidad de Array se derivará transitivamente de esas propiedades, sin listas por nombre.» | §3.2: String y todo nominal copiable e independiente entran, incluidos wrappers Array/Option/Result; recursos/estados/closures conservan fronteras explícitas F5. |
| §1.1: «La igualdad de Array exigirá la capacidad de igualdad de sus elementos; los seis comparadores conservarán la precedencia de tipos distintos de A2.» | §§3, 5.2 y 10.7: capacidad estática incluso para Array vacío; tipos distintos preceden a dominio o capacidad; K03/K08. |
| §1.1: «Los cuatro operadores de orden compartirán el dominio que F0-0004 declare, sin conceder orden automáticamente por ser nominal.» | §§3 y 9: dominio I64/String para <, <=, >, >=; nominales sin orden aun con PartialEq. |
| §1.1: «La extensión conservará copia independiente, permisos, cuotas de búfer y un único motor; no se mantendrá un Array ni un comparador paralelo para texto.» | §§3.2, 7–8: Push/Pop/JoinArrays, for, crecimiento y limpieza del mismo Array; costes de subvalores añadidos una sola vez; K03/K04/K08. |
| §8.2: «text, user_types, methods, modules_visibility, attributes, extensions» y «F0-0004 definirá sus extensiones.» | §§2–6 y 9.1 levantan solo las formas incluidas. Const conserva el dominio escalar de F5 §5. |
| §12.3: «F0-0004 deberá integrar los datos nuevos sobre estas reglas (frontera §1.1), sin imponer aquí sus perfiles, su orden de String, su derivación nominal ni sus códigos.» | §8 compone el perfil K sobre F5; §9 mantiene códigos J/F5 y reserva solo NX-E1401–1404. K01 integra el contrato; K11/K12 lo verifican. |

Enmienda propuesta de F0-0005 §4.1, «Los elementos admitidos son I64, Bool, () y Array/Result/Option transitivos de esos datos.»: en el perfil K se añaden los datos propietarios de §3.2. Enmienda propuesta de §2.1, «Dos valores del mismo tipo copiable admitido por A2/J → Bool»: el dominio se amplía mediante PartialEq cerrado de §5.2. Enmienda propuesta de §8.1, «Tipos iguales inadmisibles denuncian el operador: I64 para orden, domain:copyable_data para igualdad, found_type sobre el izquierdo.»: orden usa domain:ordered_scalar; igualdad nominal sin capacidad usa NX-E1403 según §9, sin alterar la prioridad de tipos distintos ni los rechazos históricos de recursos/invocables.

### 1.2. Incluido y excluido

Incluido: String propietario UTF-8; literales/interpolación; operaciones de §3; struct/enum monomórficos de grafo de tipos acíclico; campos propietarios; constructores; proyecciones; reemplazo de campos; métodos de extensión locales activados explícitamente; `pub` y rutas del módulo raíz; imports del registro estándar; derivaciones cerradas Format/PartialEq; consola simulada. Los agregados pueden contener recursos y Result/Option, pero Array conserva su exclusión de elementos con recursos transitivos.

No se añaden tipos numéricos, Bytes, vistas almacenables, grafemas, normalización, indexación de String, interpolación con especificadores, mutación de bytes de texto, sobrecarga de operadores de usuario, destructores de usuario, herencia, alias de tipo, módulos fuente adicionales, paquetes, genéricos de usuario, traits de usuario, CoW, RC, async, archivos reales, red, REPL ni backend nativo. Los compromisos completos de D08–D10/F1 siguen pendientes.

## 2. Superficie y resolución

Se habilitan las producciones de D15 §3.5 para StructDecl, EnumDecl, Field, Variant, ExtendDecl, MethodDecl, BaseAttribute, rutas y selectores, con las restricciones de este ADR. No se introducen llaves constructoras, `new`, métodos dentro de struct ni `Self`. Ejemplos de formas completas:

```nexo
struct Punto { x: I64; y: I64; }
enum Evento { Inicio, Dato(String, I64) }
extend Punto {
    fn Leer(self: view) -> I64 { return self.x; }
    fn Fijar(self: edit, n: view I64) { self.x = n; }
    fn Consumir(self: move) { Drop(self); }
}
use extension package.Punto;
fn Main() {
    var p = Punto(y: 2, x: 1);
    edit p.Fijar(3);
    AssertEq(p.Leer(), 3);
    move p.Consumir();
}
```

Los separadores finales de campos/variantes siguen D15; las listas de argumentos/parámetros no admiten coma final. Una variante nula se escribe `Evento.Inicio`, sin llamada; la variante con payload usa `Evento.Dato("x", 1)`. `Evento.Inicio()` da NX-E0111/non_callable_target sobre el destino. `Dato()` con cero campos de struct sí es constructor invocable. No se importan implícitamente variantes de usuario al preámbulo.

Se registran declaraciones, nombres de miembros y firmas antes de comprobar cuerpos. Tipos, funciones e imports comparten el espacio de declaraciones de D13; no se selecciona por mayúscula. Los campos y variantes pertenecen al espacio de miembros de su tipo; dos campos/variantes iguales se rechazan, aunque no se usen. Un campo y un método pueden compartir nombre: `x.M()` selecciona método y `(x.M)()` selecciona el campo, sin reinterpretación por tipo. Un método que falta no se sustituye por un campo invocable. No hay sobrecarga por aridad, tipo de argumento ni resultado.

Las rutas `package.T` y, fuera de un cuerpo de método, `self.T` designan declaraciones de la raíz. En una expresión dentro de un método, `self` siempre es su receptor; se usa `package.T` para una declaración de módulo, sin decidirlo por la caja del selector. En posición de tipo, `self.T` sigue siendo ruta de módulo. La identidad de módulo del archivo raíz es `package`; no depende del nombre del archivo. `super` no tiene padre en este corte: NX-E0006/modules_visibility. Los identificadores de tipos nominales internos combinan identidad de módulo y declaración; sus tipos diagnósticos se imprimen `package.T`. Una ruta alias nunca crea otro tipo. Un tipo local Iterator sombrea la identidad de preámbulo conforme a F0-0003 §5.2 y no hereda Next ni iterabilidad por su nombre.

### 2.1. Interpolación: gramática completa del literal

Se sustituye la envoltura opaca F0 de D15 §2.4 por el análisis de B completa:

```ebnf
StringLiteral = '"' {TextScalar | Escape | '{{' | '}}' | Hole} '"' ;
Hole          = '{' Expr '}' ;
Escape        = '\"' | '\\' | '\n' | '\r' | '\t' ;
```

TextScalar excluye comillas, barra inversa, llaves y CR/LF crudos. Un Hole contiene exactamente una Expr de la única gramática aceptada, sin especificador de formato ni sentencia suelta. Las llaves de bloques de if/match y las cadenas interiores se analizan recursivamente; no cierran el Hole exterior. Se permiten sentencias dentro de los bloques que ya pertenecen a una Expr de control, con las reglas ordinarias de `return`, `?` y `value:`. Un `return` dentro de ese bloque retorna del invocable envolvente, no de un invocable inventado por interpolación. No se introducen closures para bajar el literal.

CR/LF crudos tampoco se admiten dentro del Hole: el literal completo ocupa una línea lógica. Esta restricción incluye comentarios `//` que intenten atravesar el cierre; se aplica antes de insertar terminadores. `"{if true then 1 else 2}"` y `"{"x"}"` pertenecen a la gramática; las comillas interiores no llevan escapes exteriores. `"{}"` es empty_interpolation; `"}"` es unescaped_closing_brace; EOF o comilla encontrada tras una Expr completa sin completar `}` es unterminated_interpolation. Una Expr internamente inválida conserva NX-E0002 y su contexto ordinario salvo las fronteras específicas siguientes. Un `:` posterior a una Expr completa espera exclusivamente `}`; no significa formato. Los cinco escapes de D15 conservan NX-E0001 para cualquier otro escape. No se habilita `\u` en fuente Nexo.

Los espacios ASCII y tabuladores entre `{` y `}` no constituyen Expr: NX-E1401/empty_interpolation sobre el Hole completo. Un CR, LF o CRLF crudo dentro de un Hole emite NX-E1401/raw_line_break_in_interpolation sobre el salto (CRLF es una unidad de dos bytes), antes de insertar terminadores. Un `//` en contexto de código del Hole emite NX-E1401/comment_in_interpolation sobre las dos barras, antes de consumir el comentario; dentro de una cadena anidada sigue siendo texto. Ambos se detectan en orden fuente y detienen ese literal tras el primer error, sin cascadas de delimitadores. La recuperación conserva la pila de contextos del literal y descarta tokens hasta consumir su comilla exterior de cierre, permitiendo atravesar saltos solo durante ese descarte. Balancea bloques/Holes y cadenas anidadas con sus escapes; tras // descarta las dos barras como token inválido y continúa ese balance sin activar un comentario de línea. No inserta terminadores ni emite diagnósticos adicionales dentro del literal descartado. Si alcanza EOF, termina allí sin repetir el error. Entrega un único nodo contaminado y reanuda la sentencia en el primer token posterior al cierre exterior. En H01 consume `}` [23,24) y la comilla [24,25), y reanuda en `;` [25,26); H02/H03 consumen `}` [22,23) y la comilla [23,24), reanudando en `;` [24,25). La última `}` cierra Main normalmente; no se interpreta `}";}` como otro literal o Hole.

Si se espera un operando y aparece una comilla, se intenta reconocer una cadena anidada completa dentro de la línea lógica y las cuotas comunes. Si no existe su cierre sin escape antes de CR/LF/EOF, esa comilla es la frontera exterior prematura: NX-E1401/incomplete_interpolation_expression sobre la comilla, en vez de unterminated string genérico. Si existe cierre, se analiza la cadena anidada normalmente; un fallo interior conserva su diagnóstico específico. Esto admite `"{1 + "2"}"` sintácticamente y rechaza `"{1 + ";` por Expr incompleta. EOF sin esa comilla conserva unterminated_interpolation sobre `{`; `}` tras operador conserva NX-E0002 ordinario. La recuperación no emite otro diagnóstico por la misma causa.

El AST guarda una secuencia ordenada de fragmentos decodificados y expresiones, con mapa de bytes fuente para cada fragmento/escape. La expresión interior conserva sus offsets originales, nunca relativos al comienzo del Hole. El impresor recodifica los cinco escapes y duplica llaves literales; no aplica búsquedas/reemplazos a la fuente. Los literales anidados participan de la misma altura/nesting de A2: no tienen un contador reiniciado ni un parser recursivo sin cuota.

## 3. Texto: valor, acceso y operaciones

String contiene una secuencia válida de escalares Unicode en UTF-8. Se aceptan NUL, caracteres combinantes y no caracteres Unicode; se rechazan surrogates y codificaciones inválidas en las fronteras que decodifican bytes. No hay normalización, cambio de caja, colación regional ni dependencia de tablas Unicode para las operaciones de este corte. Un punto de código contable es un escalar; no un grafema. La pareja `e` seguida de U+0301 tiene dos escalares y no es igual a U+00E9.

String es copiable e independiente. Cada copia propietaria realiza una copia física; el literal también materializa un propietario, incluso vacío. No hay identidad observable de búfer, interning semántico, CoW ni copia con callbacks. `view String` y `edit String` son permisos de parámetro/receptor, no tipos de referencia nuevos. Una lectura como argumento, comparación, formato o longitud no copia. Retornar un parámetro view en contexto propietario copia; nunca devuelve una vista. No existe StringView ni préstamo persistente de substring en este corte.

Todas las operaciones siguientes tienen identidades del manifiesto y tipos fijados en el checker. Son métodos propios intrínsecos de String; no necesitan `use extension`. Los nombres de parámetros publicados son las etiquetas aceptadas. `self` está implícito en la llamada de método; el tipo mostrado en la tabla es String.

| Operación / firma B | Resultado exacto | Efectos propios |
|---|---|---|
| `ByteLen(self: view) -> I64` | Número de bytes UTF-8; cabecera cacheada. | vacío |
| `ScalarLen(self: view) -> I64` | Número de escalares, decodificando en orden. | vacío |
| `IsEmpty(self: view) -> Bool` | ByteLen igual a cero. | vacío |
| `Concat(self: view, other: view String) -> String` | Bytes de self seguidos por other. No muta entradas. | alloc |
| `SliceBytes(self: view, start: view I64, end: view I64) -> Result<String,I64>` | Copia de intervalo semiabierto; primero `0 <= start <= end <= ByteLen`, después ambas fronteras UTF-8. Err(-9110) por rango; Err(-9111) por frontera. Vacío válido también en EOF. | alloc, conservador incluso si falla |
| `Scalars(self: view) -> Array<I64>` | Valores numéricos de escalares en orden; materializa explícitamente un array independiente. | alloc |
| `std.text.FromI64(n: view I64) -> String` | Decimal canónico ASCII, incluido mínimo I64, sin `+`, ceros iniciales ni `-0`. | alloc |
| `std.parse.I64(s: view String) -> Result<I64,I64>` | Gramática ASCII `-?[0-9]+`; admite ceros iniciales y `-0`; no espacios, `+`, separadores, dígitos Unicode ni cadena vacía. Err(-9101) por sintaxis, Err(-9102) por rango. | vacío |

El parser entero valida toda la sintaxis antes de decidir overflow: `999999999999999999999x` da -9101. Usa acumulación comprobada que admite -9223372036854775808, sin construir antes un positivo I64 fuera de rango. Una vez detectado overflow sigue validando los bytes restantes con cargo de trabajo; no multiplica un entero anfitrión desbordado.

La iteración materializada de texto es `for code in s.Scalars(){…}`, con Array dinámico de F0-0005. Es materialización visible, no otro protocolo Iterator. Iteración perezosa y grafemas quedan diferidos. `s[i]` con i:I64 da NX-E0102/not_indexable, aunque sea ASCII. `s[a..<b]` conserva range/later_adr sobre el selector completo en types, como F5 §8.2: el slice precede al rechazo del rango hijo y al de indexación. El único slicing de String de este corte es SliceBytes.

`==` compara longitud y bytes exactos, con cortocircuito; `!=` niega ese único resultado. `<`, `<=`, `>` y `>=` realizan una sola comparación lexicográfica de bytes UTF-8 sin signo y, tras prefijo igual, longitud; prueban respectivamente resultado negativo, no positivo, positivo o no negativo. En UTF-8 válido coincide con el orden de secuencias de escalares, no con colación humana. No se intercambian operandos para implementar >/>=: ambos se evalúan una vez, de izquierda a derecha, y se prestan sin copiar. Los cuatro operadores comparten dominio I64/String; no comparan String con I64. Conservan disponibilidad, solapamiento y prioridad de tipos distintos de A2/F5. `+` y las cinco compuestas siguen siendo I64; texto usa Concat. `Concat` es método String; F0-0005 §4.1 publica la función libre `JoinArrays`, identidad prelude:join_arrays: no hay colisión ni alias de compatibilidad.

### 3.1. Formato e interpolación

Cada Hole requiere capacidad nominal Format (§5), se evalúa una vez en orden escrito y se formatea inmediatamente antes de evaluar el siguiente. Un lugar se presta solo durante ese formato; el préstamo se libera antes del Hole siguiente. Los temporales se limpian en ese punto. Un `move` explícito consume el lugar y el temporal se destruye tras formatearlo. Si hay `?`, retorno o fallo, se descarta el constructor parcial de String y se conserva toda E/S anterior. No se evalúan Holes restantes. No hay rollback.

El constructor de String tiene `alloc`; se unen los efectos de todos los Holes, también los no alcanzables por constantes, más sus limpiezas. Format no llama métodos de usuario en este corte. Print escribe directamente al registro simulado; formatear allí no construye un String Nexo intermedio ni añade alloc. Print sigue teniendo io y conserva los efectos de evaluar su argumento. Un wrapper que retorna texto sí reserva.

Formato de String: su contenido exacto, sin comillas ni escapes añadidos, incluso dentro de agregados. Formato de I64/Bool/unidad/Array/Result conserva A2; Option conserva F0-0003. Esto es presentación, no serialización reversible. Un LF dentro de String permanece LF dentro del payload de observación y Print añade un LF final adicional. JSON transporta ese payload mediante escapes JSON; esos escapes no cuentan como bytes del modelo.

### 3.2. Extensión de Array dinámico y for

String es copiable e independiente y admite Array<String>. Todo struct/enum acíclico copiable e independiente admite Array<T>, sin exigir Format ni PartialEq para construir, indexar, copiar, Push, Pop, JoinArrays o iterar. El cálculo transitivo único incluye campos, todas las variantes, Array, Option y Result. No basta que la variante activa carezca de recursos. Se conservan resource_array/later_adr para recursos/estados transitivos y closure/later_adr para closures restantes; resource_array tiene prioridad. Una colección vacía no evita la comprobación.

Los slots de F5 §4.2 siguen costando 32 bytes por capacidad; los campos y bytes de cada elemento se cuentan separadamente, sin inflar el slot ni cobrar dos veces el valor. Crecer mueve handles sin copiar String ni campos nominales: no inicia búferes de texto. Copiar Array o JoinArrays copia profundamente componentes en orden; cada String copiado paga cabecera, bytes y text_allocations. Pop traslada el último valor a Some y conserva capacidad; no copia sus strings. Reemplazar un elemento limpia el antiguo después de preparar el nuevo. Una copia parcial fallida libera sus búferes y celdas, sin alterar entradas ni devolver cuotas acumuladas.

`for r in registros`, `for s in textos`, iterandos temporales y `for r in move registros` conservan F5 §3.2: préstamo de raíz, alias de lectura por elemento, ninguna copia al avanzar; usar el alias en contexto propietario copia sus componentes. Métodos view y match prestado admiten el alias; edit/move del elemento existente conserva sus restricciones. Break/continue/return/? liberan alias y préstamos en los mismos bordes J13/F5-L06. String y nominales por sí solos no son iterables: NX-E0102, expected_type=domain:iterable, primario/found_type sobre iterando, sin expected_type relacionado. `for edit` conserva for_iteration/later_adr sobre edit en parse. No se añade protocolo ni estado de iteración por nominal.

Igualdad de Array/Option/Result requiere PartialEq transitivo incluso vacíos o en una alternativa inactiva; formato requiere Format transitivo. String lo satisface intrínsecamente; un nominal solo por derive. Los envoltorios no obtienen orden por contener String. Const sigue limitado a I64/Bool/(): no se habilita String/nominal/Array comptime ni se usa const para generar derivaciones.

## 4. Struct, enum y propiedad

### 4.1. Identidad y construcción

Struct/enum son nominales monomórficos. No hay equivalencia estructural ni conversión automática entre dos declaraciones de igual forma. Los tipos de todos los campos/payloads se escriben y se resuelven antes de cuerpos. Se admite struct vacío. Enum exige al menos una variante en este corte; enum vacío se reconoce y rechaza con NX-E0006/user_types. Todos los campos son propietarios y obligatorios; no defaults ni inicializadores de campo.

El constructor de struct usa la asociación de A2 §4.2: posicionales en orden de declaración, después etiquetas, cada campo exactamente una vez. Cada argumento entra en contexto propietario como un parámetro move: un lugar se copia si admite copia; un recurso exige `move`; un temporal se entrega directamente. No se infiere transferencia por el tipo del campo. Los payloads de enum son posicionales; etiquetas se rechazan con unknown_argument_label. El tipo de enum lo determina el destino cualificado, sin necesitar contexto de resultado. No se admiten campos anónimos tipo tupla fuera del payload de variante.

En patrones, K07 acepta `Tipo.Variante(P1,…,Pn)` de aridad exacta y `Tipo.Variante` nula. Extiende la descomposición de un payload de F0-0003 a una lista ordenada de payloads, mediante el mismo algoritmo de residuos, sin enumerar productos cartesianos. Constructor_type precede a constructor_arity (NX-E1325). La identidad diagnóstica de constructor nominal es `package.Tipo.Variante`; `scrutinee_type` usa `package.Tipo`. El testigo residual recorre variantes en orden de declaración y payloads de izquierda a derecha, con las reglas de testigos de F0-0003 para cada componente. Struct se trata como dominio opaco: binding o `_`, sin desestructuración de campos en este corte; un patrón constructor de struct se rechaza con NX-E0006/user_types. No se añaden patrones literales de texto: String usa binding/`_` y guardas de igualdad prestada. La cobertura de String nunca enumera strings.

Primero se comprueba la asociación completa; los tipos incompatibles no sustituyen errores de aridad/etiquetas. La evaluación es en orden escrito, no en orden de campos. Hasta completar la construcción, los argumentos preparados son propietarios temporales en ese orden. Al completar se instalan por FieldId; se añade una celda de agregado y se transfieren componentes, sin copia adicional.

### 4.2. Propiedades transitivas y representación

Para struct, copia e independencia requieren las respectivas propiedades de **todos** los campos. Para enum requieren todas las alternativas, aunque la expresión construya una variante sin recurso. Las propiedades de transferencia/afinidad son intersecciones transitivas del dominio de cada componente; no se publicita transferencia entre tareas al no existir esas tareas en el perfil. Result/Option/Array/capturas consultan la misma tabla de propiedades. Poseer un MockResource convierte el agregado en no copiable; no existe una anotación de usuario para fingir copia, independencia o seguridad de suspensión.

El grafo nominal admitido es acíclico. Se detectan SCC antes de expandir propiedades/layouts. Un ciclo por campos inline de struct/enum/Result/Option es NX-E1323/recursive_value_layout; una arista por Array rompe la recursión de tamaño físico, pero los nominales recursivos con esa indirección siguen diferidos con NX-E0006/user_types, no se presentan como tamaño imposible. Por cada SCC cíclica se emite un diagnóstico: se elige el ciclo simple de menos aristas; cada ciclo se rota para comenzar en la declaración con menor offset, y los empates se resuelven lexicográficamente por la secuencia de offsets de referencias de tipo en sus aristas. El primario es la referencia de la última arista, que vuelve al nodo inicial; declaration relaciona el nombre de ese nodo. Los diagnósticos se ordenan por primario. Se respeta type_solver_steps sin enumeración exponencial de ciclos. No se usa el límite de profundidad para disfrazar un ciclo. Los DAG se recorren memoizados por TypeId; se cobra por nodo/arista lógico procesado, sin expandir exponencialmente propiedades.

Los agregados son inline en el modelo: construir su envoltorio no tiene alloc; construir/copiar String o Array de campos sí lo tiene. Copia de un agregado visita su envoltorio y componentes y produce independencia física. No hay recursos copiables por su representación anfitriona. Enum mantiene VariantId y solo el payload activo; no reserva espacio lógico para todas las alternativas. Su resumen estático de limpieza es la unión de todas, salvo evidencia de variante verificada conforme a A2 §8.5.

String propietario es SuspendSafe; un nominal lo es si todos sus componentes lo son. Esta propiedad se incorpora al mismo cálculo de F0-0003 §5.3, sin dispensar permisos edit retenidos. Un gen recibe String/nominal únicamente por parámetro move; su excepción view sigue limitada a I64/Bool/unidad. Las capturas copy/view/edit/move conservan propiedades y procedencias de F0-0003. No se permite nombrar un tipo anónimo de closure/generador como tipo de campo; Option/Result locales pueden seguir transportándolo según aquella dependencia. Publicar uno no inventa un tipo fuente.

### 4.3. Campos, asignación y préstamos

`p.campo` es Place si p lo es. La proyección usa FieldId, no búsqueda por nombre en el intérprete. Leer para un parámetro view/Print/comparación presta; leer para un propietario copia únicamente ese campo. Proyectar un temporal propietario entrega el campo seleccionado y limpia el resto; no crea un préstamo a un temporal destruido.

`p.campo = rhs` exige raíz modificable y permiso suficiente. Se evalúa ruta e índices una vez, después RHS; se comprueban disponibilidad y límites contra la raíz actual; se limpia el campo antiguo y se instala el nuevo. Un reemplazo completo de campo puede contener recurso: no deja un hueco. Si RHS falla, el antiguo sigue sujeto a limpieza. Si RHS consume la raíz necesaria para escribir, se rechaza estáticamente. Una asignación a una proyección no reinicializa una raíz consumida. Las cinco compuestas sobre campos I64 conservan F0-0005 §3.3: preparar Place una vez, exclusividad antes de RHS, operación comprobada e instalación; no se amplían a String o nominales. Su préstamo y el de for Array conservan identidades/anclas de F5 §3.3; un conflicto de esos préstamos denuncia la raíz del acceso incompatible, mientras los préstamos explícitos de campos conservan los primarios de los pares N07 de este ADR.

`move p.campo` y `Drop(p.campo)` permanecen NX-E0006/partial_transfer: no se introduce extracción parcial de lugares que sobreviven. La extracción consumidora de payload de **temporal de match** pertenece a F0-0003 y no es esa operación. Una función move sobre el agregado puede consumir todo `self`, inspeccionar campos o devolver copias de campos copiables; no obtiene por ello permiso para dejar huecos en self.

En conflictos de préstamos, el primario es el Place solicitado completo con sus proyecciones, sin el prefijo; origin conserva la adquisición anterior con prefijo. Los errores de mutabilidad/permisos sobre raíz (X10/N06) conservan el primario de raíz de A2. Todos los campos e índices de una raíz se consideran solapados. Dos campos de nombres distintos no acreditan disjunción. Los représtamos suspenden permisos padres y se liberan según A2. `view` no permite editar ni consumir un campo; `edit` permite reemplazo, pero no consumir la raíz prestada. La información viaja por OriginId y proyecciones, no por el nombre textual del receptor.

### 4.4. Limpieza

Un agregado completamente construido limpia campos en orden inverso de **declaración**; enum limpia solo payloads activos en orden posicional inverso. Un constructor incompleto limpia argumentos preparados en orden inverso de **evaluación**. La distinción es observable con etiquetas reordenadas y no depende del layout anfitrión. Una transferencia desarma exactamente el propietario anterior; transporta también reservas de cierre y metadatos de tamaño.

Se conserva A2 §6.4 para temporales, locales, parámetros, resultado separado y liberación de préstamos. Drop del agregado completo consume una vez y aplica el plan estructural; no ejecuta un método llamado Drop del usuario. No hay destructores personalizados ni propagación desde limpieza. Return, `?`, fallos R y agotamiento de cuotas limpian el prefijo realmente inicializado, sin omitir recursos por falta de trabajo. No se promete limpieza tras aborto/OOM del anfitrión.

## 5. Métodos, derivaciones y módulo único

### 5.1. Métodos

Se admiten bloques `extend T` para tipos nominales **de la raíz** y métodos con `self: view`, `self: edit` o `self: move` como primer parámetro obligatorio. Los otros parámetros y el resultado obedecen A2. Se prohíben métodos estáticos sin self en este corte con NX-E0006/methods; no se inventa `impl T` ni un constructor especial dentro de extend. Ampliar tipos estándar mediante extend está diferido con extensions.

Activación local explícita: `use extension package.T;` importa todos los bloques extend de esa raíz/tipo, independientemente del orden fuente. Sin activación, una llamada cuyo método existe da NX-E1315. Duplicar un método entre bloques o en el mismo bloque da NX-E1322 aunque no se active. No se usa el orden de imports como prioridad. Se admite la forma cualificada D15 `extension package.T.M(receptor, argumentos)`; selecciona la misma identidad y la misma asociación, sin requerir importación. No se crea otro cuerpo ni otro camino de tipos.

`x.M()` solicita lectura; `edit x.M()` solicita exclusividad; `move x.M()` consume receptor. Un modo inferior al de self da NX-E1107. Se permite modo más fuerte conforme a D11 §5.1: el `move` de un receptor Place crea un propietario temporal que puede prestarse a self:view/edit y destruirse al terminar; el método no gana permiso para extraer de un self prestado. Conforme a F0-0003 §4.2, edit/move de invocación exige receptor Place. Sobre un receptor temporal, la falta de modo de un método self:edit/move emite NX-E1107 con required_mode=inout/consume y requested_mode=read, primario sobre la expresión receptora completa, related_spans=[] y cause_chain=[]; no se fabrica declaration. Las llamadas completas `edit F()` / `move S()` resueltas a función ordinaria o constructor carecen de receptor: conservan NX-E0006/receiver_mode, pending_scope=later_adr, names/names.resolve y primario en el prefijo, conforme a F0-0003, «Invocación y efectos»; no se clasifican como NX-E0002. Fuera de esas llamadas sin receptor, si se escribe edit/move sobre un operando que no es Place, incluido un receptor temporal de método, se emite NX-E0002 en parse/syntax.parse, con parser_context=inout_place/consume_place respectivamente, expected_tokens=["Place"] y expected_tokens_kind=admissible_set. Se señala el primer token que impide derivar Place; recognized_prefix empieza en el prefijo y termina antes de ese token, incluida trivia. En S() ese token es la apertura de llamada, no el nombre S que por sí solo sí forma Place. Subcause=null, data={}, sin relaciones ni cadena; no se emite NX-E1107 además. D15 §3.2 y A2 §3.6 conservan esta clasificación, confirmada por F0-0003 §4.2 «Un operando que no satisface Place conserva NX-E0002». Una llamada sobre temporal nuevo solo solicita lectura; para editarlo o consumirlo se instala primero en un binding (var para edit). No se deduce transferencia de receptor por la firma.

El receptor se evalúa antes de los argumentos escritos, una vez; su préstamo dura hasta después de limpiar el callee, como el primer argumento de A2. No se retrasa su adquisición para permitir `edit x.M(x.campo)`. Un método se registra como función con parámetro self y metadatos de selección; usa el mismo asociador, CFG, inferidor, análisis de efectos, lowering y llamada que una función ordinaria. `edit x.M()` nunca se baja como edición del resultado.

### 5.2. Format y PartialEq

`@derive(Format)` y `@derive(PartialEq)` se admiten sobre struct/enum, por separado o en la misma lista. Cada petición aparece una vez por declaración. Las identidades intrínsecas Format/PartialEq se resuelven nominalmente desde el preámbulo; la escritura de un tipo de usuario homónimo no recibe privilegios. La derivación requiere, transitivamente, la misma capacidad en todos los campos/payloads. Un recurso no tiene ninguna; una derivación fallida señala el primer tipo de campo incompatible en orden de declaración y relaciona la petición.

La capacidad no se infiere por encontrar un método Format, Eq o similar. Sin derive, un nominal copiable no admite Print/interpolación ni igualdad. Los tipos intrínsecos escalares, String y los agregados intrínsecos copiables tienen las instancias cerradas por componente. `AssertEq` exige PartialEq; Print e interpolación exigen Format. Los usos de Print/igualdad/AssertEq ya rechazados por A2/F0-0003 sobre recursos, invocables, estados o envoltorios intrínsecos que los contienen conservan NX-E0102/type_mismatch y su dominio anterior: no se sustituyen por NX-E1403. Este último corresponde a la obligación nueva de una derivación, un nominal de usuario o una interpolación. El orden `< <= > >=` de nominales no se deriva: conserva NX-E0102/type_mismatch, dominio I64/String. No se habilitan Eq, Hash, JsonEncode, traits declarados, impl, where, parámetros genéricos ni operadores personalizados.

Formato derivado: struct `T(campo: valor, otro: valor)` en orden de declaración, incluido el nombre de campos privados; struct vacío `T()`. Enum nulo `T.Variante`; enum con carga `T.Variante(v1, v2)` en orden posicional. Se usa el nombre declarado del tipo, no el alias de acceso ni `package.`. El atributo es autorización explícita del módulo definidor para mostrar campos privados; no evita P09 ni constituye redacción de secretos. No hay reflectividad pública ni acceso privado otorgado a llamadas ordinarias.

Igualdad derivada: comprueba identidad nominal, luego campos en orden; enum compara VariantId antes de payloads. Cortocircuita al primer distinto. String compara bytes sin normalizar. Dos tipos diferentes fallan por tipo antes de consultar capacidad. Format no implica PartialEq ni viceversa; ambas conservan copia/independencia y no añaden capacidad a los campos.

La derivación genera planes estructurales tipados en la tabla de tipos y operaciones IR verificables; no genera código fuente que se vuelva a parsear. Es una simplificación explícita del prototipo respecto de la derivación general en comptime de D03/F1. No amplía el evaluador comptime escalar de F0-0005 ni acredita un solucionador de traits. Las instancias son nominales y únicas, con identidad de receta y campos; un futuro motor de traits deberá reemplazar esa representación en un solo corte, conservando semántica y corpus, sin un adaptador permanente.

### 5.3. Visibilidad

Hay un archivo fuente, cuyo módulo raíz es package. Las declaraciones y campos son privados por defecto; `pub` los exporta. El archivo completo pertenece al dominio privado definidor, incluidas sus funciones y extensiones: no se inventa privacidad por struct. Un enum público exporta todas sus alternativas/payloads. Firmas públicas, campos públicos y payloads de enum público solo mencionan tipos con ruta pública; los campos privados de un struct público pueden usar tipos privados. Una función o método pub escribe resultado, también `-> ()`, y WithClause válido. Falta resultado: NX-E1310/missing_signature_type; falta cota: missing_effect_bound; tipo inaccesible: unexported_type. Una firma inválida no origina cascadas en sus usos.

Se permiten `use std.console;`, `use std.parse;`, `use std.text;`, imports selectivos/grupos/alias de esos módulos y de declaraciones de package, y `pub use` de elementos públicos accesibles. No ejecutan inicializadores. Rutas estándar resuelven un manifiesto versionado, no archivos del equipo. `std` es la raíz reservada de biblioteca de D13; no se redefinen sus módulos mediante declaraciones de usuario. Un import normal no activa extend. Colisiones y repetición usan NX-E1309, también importar la propia declaración con el mismo nombre; un alias nuevo evita esa colisión; reexportación de privado accesible usa NX-E1308/private_reexport. La API interna de resolución también prueba acceso externo contra una interfaz exportada, sin inventar un segundo archivo fuente aceptado por la CLI. La privacidad entre módulos fuente adicionales sigue NO EVALUADA.

`mod`, rutas de paquetes externos y dependencias de archivos siguen modules_visibility. No hay búsqueda de archivos, alias de tipo ni inicializadores globales. El registro de exportaciones contiene identidades, campos públicos, alternativas, modos, capacidades derivadas, efectos y hashes de interfaz; cambios invalidan consumidores. No se declara entregado un servidor incremental o hot reload por conservar esos metadatos.

## 6. Preámbulo, biblioteca y E/S simulada

Se conservan todas las identidades y contratos de A2/F0-0003/F0-0005. String, Format y PartialEq se añaden al preámbulo. ByteLen y demás métodos de §3 están asociados a String; no se duplican como funciones libres. `std.text.FromI64` y `std.parse.I64` se resuelven por imports/ruta, no por comparación textual durante ejecución. Las intrínsecas son fronteras confiadas experimentales, no una decisión de mantener una biblioteca estándar en Rust contra acuerdos §9.3.

| Operación | Firma | Efectos propios | Retención |
|---|---|---|---|
| `Print(v: view T) -> ()` | T con Format cerrado | io | ninguna; formato prestado |
| `std.console.ReadLine() -> Result<Option<String>,I64>` | Una respuesta de cola; Some línea, None EOF explícito. | alloc, io | ninguna |
| `std.console.Println(s: view String) -> Result<(),I64>` | Consume una respuesta de escritura; solo éxito emite s más LF. | io | ninguna |

No hay stdin interactivo, archivos del programa, sockets ni acceso ambiental. El controlador puede leer el archivo fuente y JSON de configuración; esa infraestructura no es E/S ejecutable del lenguaje. Print es la primitiva infalible del simulador salvo cuotas R; Println permite probar fallos tipados de salida. Ambas usan un solo formateador/registro de observaciones y la misma codificación; no dos motores de salida.

### 6.1. Entorno simulado versión 2

Se reemplaza el esquema de entorno de A2 §12.4 por un único esquema 2; no se conserva un adaptador v1. En la migración de implementación se actualizan todas las fixtures del entorno en un corte. Las cuatro colas anteriores y sus contratos se conservan, y se añaden dos. Las siete claves son obligatorias, sin duplicadas ni adicionales:

```json
{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"console_read_line":[],"console_println":[]}
```

`console_read_line` contiene objetos exactamente `{"ok":"texto"}`, `{"ok":null}` o `{"err":"I64 canónico"}`. Una línea no contiene CR ni LF; tras decodificar JSON, cualquiera de esos escalares en ok produce invalid_mock_schema, igual que ok de tipo distinto de String/null. No es Err del programa. Vacío es una línea válida, distinto de EOF. `console_println` contiene unidades resultado como A2: `{"ok":null}` o `{"err":"I64 canónico"}`. El orden de claves es irrelevante. JSON debe ser UTF-8 válido y decodificar strings de escalares válidos: un surrogate aislado escapado es invalid_mock_json, no se reemplaza por U+FFFD. Se conservan límite 1 MiB, profundidad 16, errores y orden de validación del controlador de A2. Esquema 1 ahora es invalid_mock_schema. Sin `--mock-env`, las seis colas son vacías.

Una cola vacía produce Err(-9000), no EOF. None solo procede de `{"ok":null}`. Cada llamada consume una respuesta de su propia cola global al comprometerse. EOF no convierte respuestas posteriores en inalcanzables por regla: llamar otra vez consume la siguiente; la aplicación decide terminar. Se valida todo el entorno antes de ejecutar, sin reservar por adelantado valores Nexo para sus plantillas.

Eventos ASCII exactos, sin LF almacenado:

| Operación | Evento |
|---|---|
| ReadLine Some(s) | `read_line:ok:B`, B=ByteLen(s) decimal canónico |
| ReadLine None | `read_line:eof` |
| ReadLine Err(e) | `read_line:err:E`, E decimal de e |
| Println Ok | `println:ok:B`, B=ByteLen(s), excluye LF añadido |
| Println Err(e) | `println:err:E` |

Estos eventos usan prefijo virtual `mock-io:` y LF para observation_bytes. Print no genera evento. Println exitoso registra primero su evento y después un stdout con s; error registra solo evento. ReadLine no imprime el contenido. Los bytes originales se conservan en String; no se ajustan saltos ni se recorta texto.

Antes del compromiso se preparan/cobran resultado, trabajo y todas las observaciones necesarias. Si falla una cuota no se consume cola ni se emite evento/salida parcial. La limpieza destruye temporales preparados. Una llamada cuyo argumento ya produjo E/S no la revierte. No se imputa un error R al Err tipado. El formato de resultado CLI/JSON y códigos de salida de A2 permanecen; Main sigue `()` o `Result<(),I64>`, no acepta arbitrariamente un enum de error como entrada pública. La versión del objeto **de ejecución** sigue schema_version=1; la del archivo **de entorno** es 2 y es un protocolo diferente.

## 7. Núcleo de tipos, IR y verificación

Una tabla canónica del checker contiene TypeId, identidad nominal, campos/alternativas, capacidad de copia, independencia, afinidad, tamaño lógico, efectos de construir/copiar/destruir, planes Format/PartialEq y procedencias. Cada expresión/Place tiene una entrada TypeId y un origen de restricción; campos añaden FieldId, enums VariantId. Se conservan incógnita monomórfica, Never y contaminación como estados diferentes de I07c. Registrar nominales no convierte una incógnita en unidad ni decide el tipo por la variante usada.

Solo el checker resuelve restricciones y produce esa tabla. El flujo alcanzable y el hipotético emplean las mismas operaciones de tipos sobre el mismo almacén de restricciones; cambia el estado de disponibilidad, no las reglas o los tipos obtenidos. El lowering consume la tabla: no llama un `infer_expr` alternativo, no deduce tipos de valores constantes ni recorre ramas muertas con un checker distinto. El verificador comprueba coherencia de TypeId contra los descriptores, sin volver a inferir fuente. No se acepta «un tipo para run» distinto del de check.

Representaciones lógicas adicionales:

| Valor/IR | Contenido mínimo e invariantes |
|---|---|
| StringValue | Búfer propietario UTF-8 validado; longitud de bytes; cuenta lógica de almacenamiento. Ninguna vista a memoria fuente o plantilla mock. |
| StructValue | TypeId nominal, vector por FieldId, flag de vida de envoltorio/componentes durante construcción y limpieza. |
| EnumValue | TypeId, VariantId válido, vector de payloads de aridad/tipos exactos. |
| FieldPlace | OriginId raíz y secuencia FieldId/índices evaluados; permiso y padre de représtamo. |
| BuildString | Fragmentos y operaciones Format tipadas; progresión ordenada y borde de fallo/retorno. |
| BuildStruct / BuildVariant | Asociación de argumentos ya resuelta, orden de evaluación conservado, traslado de temporales y plan de fallo parcial. |
| ReadField / WritePlace | Mismos contextos de lectura/copia/transferencia y mismo mecanismo de lugar que arrays. |
| Compare / Format | Identidad de plan cerrado, no despacho por nombre; tipos y efectos conocidos. |

Se extienden las operaciones de construcción y proyección existentes; métodos usan CallDirect y préstamos ordinarios. La selección nominal usa TestVariant y el plan de selección de F0-0003, no otro switch con su propio control de propiedad. String/nominales no se ejecutan directamente desde AST. Un artefacto verificado contiene versión de perfil/manifiesto/tablas; el intérprete rechaza internamente uno incompatible, sin convertirlo en error del programa.

El verificador de A2 añade: pertenencia FieldId/VariantId; tipo y aridad de payload; acceso conforme a selección resuelta; no préstamo a un búfer temporal destruido; cobros de String/derivación presentes; correspondencia de plan Format/PartialEq con capacidades; máscaras de inicialización y limpieza; efectos de cada Hole; mismas propiedades de componentes en copia, captura, Option/Result y agregado. Los descriptores de builtin y nominal pasan por la misma API. Rechaza IR con byte inválido, variante inexistente, miembro de otro nominal, campo ausente, copia de recurso escondido o cierre duplicado. Esos son tests internos del verificador, no códigos NX-E para fuente inexistente.

Los recorridos de copia, comparación, formato, drop, validación de tipos y destrucción de AST/IR usan pilas explícitas o límites aplicados **antes** de descender. No basta acotar el parser si el drop del anfitrión recurre sin límite. El hash de interfaz incluye orden de campos/variantes, visibilidad, firmas, capacidades y efectos; el de función conserva identidad de código de A2/acuerdos §9.8. No se habilita recarga de objetos vivos.

## 8. Perfil y contabilidad

Perfil normal: `f0-0004-b-v1`; modelo de efectos: `f0-0004-sim-v1`. Es el único perfil completo del corte y extiende acumulativamente `f0-0005-b-v1`/`f0-0005-sim-v1`, que ya incluyen A2/J; no selecciona entre K y F5 al ejecutar. No es un dialecto, ni promete reproducir cifras con un modelo de costes distinto. Los programas que no usan las operaciones nuevas conservan los ticks/reglas de A2, F0-0003 y F0-0005. Se heredan sus cargos de patrones, capturas, estados, avance y limpieza, sin reinicio ni cambio de unidades.

El ensanchamiento del dominio de `< <= > >=` cambia expresamente el `expected_type` de su error de dominio a `domain:ordered_scalar`; su código, primario y regla de rechazo se conservan. Esta actualización y los nuevos identificadores de perfil/modelo se transcriben en las regresiones; no se atribuye identidad byte a byte a mensajes o metadatos ampliados.

### 8.1. Análisis

Se conservan todos los límites de A2 §11.1: source_bytes 1.048.576; syntax_nesting y expression_tree_depth 128; array_type_depth/value_type_depth 128; function_count 1.024; type_solver_steps 1.000.000; ownership_flow_steps 1.000.000 por función y 4.000.000 global; effect_fixpoint_steps 1.000.000; ir_items 262.144. function_count incluye funciones ordinarias, gen, literales closure y métodos; planes derivados no inventan declaraciones fuente para reiniciar cuotas. El orden de invocables y las cuotas locales/globales son los de F0-0003 §7.1. Todos los campos/payloads/capacidades se cobran en type_solver_steps por descriptor y arista procesada. Emisión de un campo/payload/fragmento/plan de derivación en IR cuesta un ir_item además de sus operandos/registros ordinarios. La tabla de TypeId no elude el cargo lógico por un uso que requiere comparación.

value_type_depth cuenta además envoltorios nominales y Option; String tiene profundidad escalar cero. La profundidad de Array sigue separada y conserva prioridad si ambas se exceden. Un nominal con cien campos planos tiene profundidad uno, no cien. Se añade `nominal_members`, límite global 16.384, un miembro por campo o variante y por cada posición de payload; reservado antes de registrar cada miembro, NX-E0003 en types. Se cobra aunque el nominal no tenga usos. Enum de una variante con dos payloads cuesta tres. Struct vacío cuesta cero miembros, pero su descriptor/registro consume source/type_solver/IR.

Se mantienen prioridad, continuación tras límite local, prefijo convergido y prohibición de artefacto verificado incompleto de A2. Las nuevas declaraciones también se comprueban si sus cuerpos son hipotéticos o no se llaman. Los límites nuevos señalan el nombre del miembro/variante o tipo de payload que intenta exceder, nunca un error de propiedad provisional.

### 8.2. Ejecución

Se conservan de F0-0005 §7 trabajo **16.000.000**, frames 256, celdas 65.536 y observaciones 65.536; array_storage_bytes 8.388.608 y array_allocation_bytes 67.108.864. Se conservan comptime_steps 100.000, comptime_depth 64 y comptime_memory_bytes 1.048.576 con sus prioridades y diagnósticos. Las cuotas de texto se suman, no reemplazan esas cuotas ni crean presupuestos por función/tipo. String añade una celda de cabecera; sus bytes tienen cuota separada y no son celdas I64. Struct/enum añaden una celda de envoltorio, más componentes activos; VariantId no añade una celda. No se cuentan préstamos ni slots desarmados. Nuevas cuotas, todas R0002:

| runtime_budget_kind | Límite | Unidad |
|---|---:|---|
| `string_bytes` | 65.536 | Bytes de un String, incluido constructor parcial. |
| `live_text_bytes` | 262.144 | Suma de bytes de todos los String propietarios vivos, incluidos temporales/constructores. |
| `text_allocations` | 4.096 | Número acumulado de búferes String lógicos iniciados; no disminuye al liberar. Vacío cuesta uno. |

Una transferencia no reserva ni copia. Construir literal, copiar String, Concat, SliceBytes exitoso, FromI64, interpolación y ReadLine Some inician exactamente un búfer resultado cada uno; incluso vacío. SliceBytes fallido y ReadLine None/Err no inician búfer. Interpolación tiene un solo búfer incremental exterior, además de los String que produzcan sus expresiones; no cuenta cada crecimiento físico del anfitrión como otra asignación Nexo. Array de Scalars utiliza capacidad N y contabilidad Array de F0-0005, no text_allocations; paga ambas cuotas de búfer Array. Format directo a observación no inicia String. Se conserva 4.096: §11 es un lote pequeño, no un procesador sin límite. Cada registro no vacío inicia al menos tres búferes (lectura, copia al registro, salida interpolada), además del resumen final; no se garantiza aceptar 1.366 registros. Para lotes pequeños como cien importes cortos deja margen; trabajo de 16.000.000 y observaciones de 65.536 heredados de F0-0005 pueden limitar antes según longitudes/eventos. No se aumenta aisladamente esta cuota para prometer miles de líneas que las otras no permiten. K10 exige medir conjuntamente asignaciones, trabajo y observaciones con cien líneas `1`, además de la app; no se reinicia por registro. Son asignaciones **lógicas del modelo**, no un contador malloc ni límite de RSS.

Ticks adicionales a sentencias/nodos/entradas de llamada/celdas de A2:

| Operación | Trabajo adicional por bytes/estructura |
|---|---|
| Materializar literal, copia, Concat, SliceBytes exitoso, FromI64 | Uno por byte escrito en el String resultado. La visita de su cabecera cuesta una unidad de construcción/copia, como toda celda. |
| ScalarLen/Scalars/parse.I64 | Uno por byte de entrada examinado. Scalars añade construcción de cada I64 y cabecera Array conforme A2. Parse valida en una sola pasada y no recorre gratis tras overflow. |
| SliceBytes: validación | Dos unidades por consultar fronteras, incluso extremos coincidentes; rango numérico inválido no consulta bytes. No recorre el prefijo. |
| String `== !=` / `< <= > >=` | Dos por pareja de bytes visitados; diferencia de longitud basta para igualdad; orden visita hasta diferencia o fin del prefijo. Dos cabeceras por A2. != cuesta exactamente ==; cada orden cuesta exactamente <, sin negación ni segundo recorrido adicionales. |
| Format | Una por celda visitada más una por byte String leído. Puntuación y decimal canónico no añaden ticks, conservando A2 §11.6; sus bytes sí cuentan como observaciones o como escritura al búfer de interpolación. Un byte String ya leído no se cobra otra vez por emitirlo a observaciones. |
| Interpolación | Formato por la fila anterior; bytes copiados al búfer exterior cuestan además uno por byte escrito. Fragmentos literales solo cuestan escritura, no un String temporal. |
| ReadLine | Además de llamada/callee/entrada ordinarias, construir Result y Option cuesta un tick por celda (una cada uno); Some añade cabecera String y un tick por byte copiado desde la plantilla. None no añade String; Err añade solo la celda I64 al Result. Cola vacía cuesta como Err(-9000). Inspeccionar la cola y construir el texto del evento no añade ticks. |
| Println | Además de llamada/callee/entrada y argumento prestado, visita una cabecera String y cada byte leído una vez (B+1), también si la respuesta es Err. Construye Result y unidad en Ok, o Result e I64 en Err (dos celdas, un tick por celda). El LF añadido y el texto del evento no añaden ticks, sí observaciones. No inicia String. |
| Agregado | Una visita de envoltorio y las visitas de componentes realizadas por construir/copiar/comparar/formatear. Traslado no recorre otra vez. |

ByteLen/IsEmpty leen cabecera y no escanean bytes; llamada/nodos conservan sus cargos ordinarios. Decodificar escapes de fuente corresponde a análisis, no a ejecución. String con solo texto es un literal; String con al menos un Hole es BuildString. La optimización no cambia estas unidades lógicas ni el orden de sus efectos.

Cargos sintácticos nuevos: literal String y BuildString cuestan un tick de expresión y uno por cabecera; cada Hole y fragmento es contenedor de coste sintáctico cero, además de sus expresiones/escrituras. Selección de campo como expresión cuesta un tick más su base en lectura; como proyección de Place cuesta uno sin volver a evaluar la raíz como nombre. Un constructor invocable de struct/variante usa los tres ticks de llamada/callee/entrada de A2, más argumentos y envoltorio. Variante nula usa un tick de expresión y uno de envoltorio, sin llamada. Una ruta de calificación no cobra por cada segmento; una llamada de método cobra llamada, receptor en su contexto, un tick del nombre de método, argumentos y entrada. Los prefijos edit/move conservan su tick y evalúan receptor como Place. No hay tick adicional de función sintética para Format/PartialEq.

Oráculos completos sin salida: `fn Main(){let s="é";}` consume 7 ticks y una asignación, máximo 2 bytes; `fn Main(){let a="é";let b=a;}` consume 12 ticks y dos asignaciones, máximo 4 bytes. El literal ASCII de ocho bytes de X24 positivo consume 13 ticks. En su negativo de nueve bytes bajo string-8 se conceden 4 ticks antes de R0002, sin iniciar búfer ni escribir bytes. Los tres casos incluyen entrada Main y unidad final cuando existe; no incluyen operaciones de instrumentación.

Al iniciar/materializar se comprueban, en orden: trabajo de operación/cabecera, frames si procede, celdas, array_storage_bytes, array_allocation_bytes, string_bytes, live_text_bytes, text_allocations y observaciones. Solo se consultan las cuotas que reserva la operación; reservas de padres e hijos son operaciones distintas. Crecimiento Array conserva ticks y compromiso de F5 §7.1. Para longitud conocida se reserva el tamaño exacto antes de escribir; para interpolación se comprueba cada fragmento formateado antes de añadirlo. Se cobra trabajo de cada byte antes de procesarlo. Aritmética de contadores comprobada. Reserva rechazada no modifica ocupación; trabajo ya concedido permanece. `observed` de tamaño es propuesta completa; del acumulado de asignaciones es limit+1. Un constructor abortado libera celdas/bytes, pero no devuelve asignaciones ya concedidas. Limpieza no inicia búferes String ni queda cortada por trabajo.

R0002 señala el literal/copiar-lugar/llamada completo que intenta reservar; en interpolación, Hole completo si el exceso nace al añadir su resultado, o fragmento fuente responsable si nace de texto literal. Desbordar en la inicialización del búfer vacío exterior señala el literal completo. La reserva para cierres mock de A2 permanece y se transporta dentro de agregados. No se intenta formatear String para cerrar un recurso.

### 8.3. Perfiles reducidos y alcance verificable

Se publican `f0-0004-b-test-string-8-v1`, `...-text-live-16-v1`, `...-text-alloc-2-v1` y `...-members-2-v1`, con una única cuota distinta (8, 16, 2 y 2 respectivamente). Se trasladan los catorce perfiles reducidos de F0-0005 §7.3 al prefijo `f0-0004-b-test-`: types-32-v1, flow-32-v1, flow-global-32-v1, effects-32-v1, ir-32-v1, frames-8-v1, cells-64-v1, observations-64-v1, work-1000000-v1, storage-96-v1, allocation-96-v1, comptime-steps-32-v1, comptime-depth-8-v1 y comptime-memory-96-v1. Cada uno cambia solo su cuota; los cuatro perfiles K de texto/miembros conservan trabajo 16.000.000 y todas las cuotas F5. Son dieciocho perfiles reducidos, no combinaciones implícitas. Ningún perfil rebaja requisitos semánticos o habilita sintaxis diferente.

Recetas de frontera normales, sin imprimir: literal de 65.535/65.536/65.537 bytes; cinco String de 60.000 bytes mediante copias sucesivas para live_text_bytes (los cuatro primeros suman 240.000; el quinto propone 300.000, con menos de 400.000 ticks); bucle que crea y descarta `""` 4.095/4.096/4.097 veces para asignaciones, con contador I64 y sin String fuera del cuerpo. Un literal de 65.537 ASCII cabe en source_bytes y alcanza string_bytes antes de los 16 millones de trabajo. Para nominal_members se generan 16.383/16.384/16.385 campos I64 de nombres cortos en un struct sin construirlo: fuente e IR bajo sus límites, sin frames/valores. Cada receta registra ocupación de **todas** las cuotas y demuestra que llega primero a la objetivo; no basta obtener cualquier R/NX-E0003.

## 9. Diagnósticos, catálogo y fronteras cerradas

Las inserciones siguientes se proponen para F0-CODIGOS antes de los emisores. Se heredan NX-E1214 (`?` en gen), NX-E1325 (constructor_type antes de constructor_arity), NX-E1326 (break/continue fuera del bucle del invocable) de F0-0003 §8.2. NX-E1323/recursive_value_layout procede de F0-CODIGOS (fila NX-E1323) y D13 §8; §4.2 concreta su emisión nominal. F0-0005 §8.1 reserva NX-E1327/arithmetic_fault, activa NX-E1316/const.check y NX-E1323/constant_cycle y añade R0501–R0505; ninguna regla K reutiliza esos números. La versión vigente de F0-0005 no reserva códigos NX-E15xx: no se inventa un alias 15xx de NX-E1403 ni de NX-E1327. Catálogo y explain incorporan esas identidades con los campos originales, sustituyendo únicamente perfil/modelo por K donde corresponde. Los números 1401–1404 abren el área texto/derivación/miembros; no reutilizan huecos de 01/13. Schema_version=1 conserva todos los campos Diagnostic de A2, con ampliaciones cerradas de valores/data indicadas aquí. Namespace NX-E sigue Nexo-proposed-v1; R conserva F0-0002-sim como identidad histórica del simulador, aunque effect_model/perfil sean nuevos.

| Código / subcausas cerradas | Regla, etapa y operación | Data exacta adicional |
|---|---|---|
| NX-E1401 / `empty_interpolation`, `unescaped_closing_brace`, `unterminated_interpolation`, `raw_line_break_in_interpolation`, `comment_in_interpolation`, `incomplete_interpolation_expression` | Estructura de literal inválida; parse / syntax.parse. Primario: Hole vacío completo, `}` solitaria, o `{` del Hole incompleto respectivamente; salto crudo completo, `//`, o comilla prematura para las tres subcausas nuevas (§2.1). | `{}` |
| NX-E1402 / `unknown_member` | Selector de campo/método/variante inexistente tras resolver tipo; types / types.check; primario nombre del miembro. | `{"member_name":NAME,"receiver_type":TYPE}` |
| NX-E1403 / `missing_format`, `missing_partial_eq` | Falta la capacidad requerida en uso o derivación; types / types.check. | `{"required_capability":"Format"}` o `{"required_capability":"PartialEq"}` |
| NX-E1404 / `duplicate_derive`, `invalid_derive_target` | Petición repetida o atributo sobre algo distinto de struct/enum; types / types.check. | `{}` |

En NX-E1401 unexpected_token es token symbol con texto/spans del `}` para empty/unescaped, o `{` para unterminated; expected_tokens, parser_context, recognized_prefix y expected_tokens_kind quedan vacíos/null: es un diagnóstico específico, no un NX-E0002 incompleto. Para raw_line_break_in_interpolation unexpected_token es kind=invalid, text el salto exacto y span primario; para comment_in_interpolation es kind=symbol, text=`//`, span primario; para incomplete_interpolation_expression es kind=symbol, text una comilla, span primario. Empty con espacios usa el token `}` final. Las seis subcausas tienen data={} y los demás campos parser vacíos/null. Se relaciona delimiter sobre la comilla exterior de apertura. No se duplica una misma causa con un unterminated string genérico. No se cambia la prioridad de escapes inválidos de D15.

NX-E1402 relaciona declaration del tipo receptor si es de usuario, nada si es builtin. NX-E1403 en uso señala operando de Print/Hole, u operador `==`/`!=`; en AssertEq directo señala el destino AssertEq. Relaciona declaration del primer nominal cuya capacidad falta, recorriendo componentes en orden declarado y envoltorios de fuera hacia dentro. Array/Option/Result de nominal sin derive usan esta regla; recursos/invocables/estados transitivos conservan primero NX-E0102 histórico. Si los tipos concretos difieren, prevalece NX-E0102 sobre el derecho con expected_type y found_type relacionados, antes de consultar capacidad o dominio. En derivación señala el tipo de campo/payload sin capacidad y relaciona contract con el nombre Format/PartialEq del atributo y declaration con el tipo nominal local que falla, si existe. NX-E1404/duplicate_derive señala la segunda petición y relaciona previous_declaration de la primera; invalid_derive_target señala nombre de la petición y relaciona context del nombre de la declaración destino. Ninguno de estos códigos tiene cause_chain ni reparaciones automáticas. No se añade una copia o permiso como reparación silenciosa.

Explain conserva A2 §13.10, con example_profile_id=f0-0004-b-v1 y una entrada por subcausa. Pares canónicos: 1401 usa X01/X02/X03 y H01/H04/H06 en el orden de su tabla; 1402 usa X05; 1403 usa X12/X13; 1404 usa X15/X16. N01/N02 amplían los ejemplos de patrones, sin crear códigos alternativos. Las variantes de derivación con campo incompatible usan la misma regla y conservan X14/N11 como regresiones adicionales. Los perfiles reducidos de cuotas se indican en sus ejemplos; no se anuncia como ejemplo normal un límite que solo existe bajo un perfil reducido. Las entradas NX-E0006 habilitadas globalmente conservan historia; sus rechazos parciales nuevos usan la primera fila de §10 que materialice la exclusión vigente, sin conservar un ejemplo que ahora sea válido.

Reutilización de catálogo: NX-E0104 añade declaration_kind `type`, `field`, `variant`; NX-E0101 añade use_kind `type` cuando una ruta que debía resolver una declaración nominal local no existe (los tipos reconocidos fuera del perfil conservan unsupported_type). NX-E0102 mantiene type_mismatch/not_indexable y el dominio `domain:array` para String indexado; el dominio de `< <= > >=` pasa a `domain:ordered_scalar` (I64 o String). NX-E0111 se aplica a constructores; missing_argument relaciona nombre de campo, argument_mode_mismatch su campo/parámetro, y las variantes sin etiquetas no inventan nombres de payload. NX-E0201/0202/1105 conservan sus datos y cadenas de A2/F0-0003. NX-E1107 conserva F0-0003 §8.2 para receptores Place y añade el caso temporal de §5.1: ownership/ownership.check, data required_mode/requested_mode en read/inout/consume, primario sobre receptor Place y declaration de su raíz; no se sustituye por el self de la firma. En temporal sin prefijo, primario sobre la expresión receptora y sin declaration; con prefijo inválido rige NX-E0002 de §5.1. NX-E1315 y NX-E1322 usan names/names.resolve y names/names.declare. NX-E1310 y NX-E1308/1309 usan names/names.resolve; NX-E1323, types/types.check. Estas emisiones nominales tienen data y cause_chain vacíos; relaciones y primarios se fijan en §10. NX-E1323/constant_cycle de F5 mantiene data.cycle y dependency; la recursión de estados J conserva NX-E0006/generator, later_adr y el ancla de F0-0003 §5.1, «Creación y representación propietaria», sin emitir NX-E1323 para esa exclusión. §4.2 solo extiende los ciclos nominales.

Efectos: se añade `value.build_string` a Diagnostic.operation y Cause.operation, kind=effect, Entity de tipo String, para literal/interpolación que incumple no_alloc; primario literal completo y contract de prohibición. Copias conservan value.copy; intrínsecas, intrinsic.call; métodos de usuario, function.call. Cadenas por campos usan planes de limpieza existentes y relacionan el sitio de campo mediante declaration; no inventan un span fuente del destructor builtin. Effect_model en data es f0-0004-sim-v1; los otros campos de efectos conservan A2. `nominal_members` se añade solo a budget_kind; `string_bytes/live_text_bytes/text_allocations`, solo a runtime_budget_kind. No se confunden presupuestos con errores parse/slice tipados.

### 9.1. Tabla cerrada de alcance

Los 30 feature_id de D15 permanecen como vocabulario histórico. No se añaden IDs por API. Las capacidades habilitadas dejan de rechazarse; no se emite NX-E0006/text sobre cualquier String para ocultar un defecto. Esta tabla define todo rechazo de alcance **añadido o conservado por F0-0004**; las capacidades J conservan la frontera acumulativa de F0-0005 §§8.2/12.3 y no vuelven a diferirse. Esas enmiendas fijan el canon de for edit y Array de closures; no se restablecen array_edit_iteration ni array_closure como pending_scope.

| feature_id | Construcción que permanece fuera | pending_scope |
|---|---|---|
| text | Operación String reconocida `Graphemes` o `View`; vistas/iteración de grafemas. No se usa para errores de UTF-8 ni método inventado. | later_adr |
| user_types | Enum vacío, nominal recursivo bajo Array y patrón constructor de struct. No existe producción de destructor de usuario ni se reserva un método ordinario llamado Drop. | later_adr |
| tuple | Valores/tipos de tupla; payload de enum no es tipo tupla. | later_adr |
| methods | Método de extend sin self. | later_adr |
| modules_visibility | mod, super en raíz, import de paquete/archivo fuera del registro raíz/std, pub const, pub gen y exportación de tipos anónimos de invocable/estado. | later_adr |
| traits_generics | trait, impl, where, parámetros/aplicaciones genéricos de usuario y dyn. Array/Result/Option intrínsecos conservan su aridad. | later_adr |
| attributes | Atributo distinto de derive Format/PartialEq, incluido Eq/Hash/JsonEncode. | later_adr |
| extensions | extend de tipo no local y activación de extensión de otro módulo. | later_adr |
| partial_transfer | Proyección de lugar superviviente: prefijo move en parse/syntax.parse; argumento de Drop en arguments/call.bind. | later_adr |
| resource_array | Array con recurso transitivo, también a través de nominal. | later_adr |
| unsupported_type | Bytes, StringView y demás tipos reconocidos fuera de las capacidades aceptadas. | later_adr |
| async, region, default_argument, float_literal, no_heap_alloc, provenance_clauses, inferred_non_unit_result | Frontera A2/J/F5; inferred_non_unit_result se aplica solo a funciones ordinarias. Gen sin flecha fija Item=() y se acepta, incluido `gen fn G(){}` (D15 F2-F27 y F0-0003 §5.1, «Creación y representación propietaria»). | later_adr |
| comptime | Fuera de const privado escalar o dominio transitivo puro no escalar/intrínseco; impureza real precede con NX-E1316 según F5 §8.2. | later_adr |
| function_value | Función nombrada como valor, salvo llamada directa. | later_adr |
| range | Fuera de iterando directo de for y selector slice sobre Array/String, según F5 §8.2; types/types.check. | later_adr |
| compound_assignment | Sin rechazo global: cinco compuestas I64, también sobre campos. Tipos incompatibles usan NX-E0102. | — |
| for_iteration | Solo for edit: token edit, parse/syntax.parse. Array/rango/generador habilitados; otro tipo conocido da NX-E0102/domain:iterable sobre iterando. | later_adr |
| receiver_mode | Prefijo edit/move sobre llamada resuelta a función ordinaria o constructor sin receptor; names/names.resolve, primario en el prefijo. Closures y métodos conservan §5.1 y F0-0003, «Invocación y efectos». | later_adr |
| generator, option, match, loop_control | No son rechazos globales: formas F0-0003 se comprueban. Las formas interiores excluidas usan las otras filas. | — |
| closure | Closures de F0-0003 habilitadas; solo Array con closure transitiva, incluso copiable, queda fuera según F5 §8.2; types/types.check sobre construcción/anotación determinante. resource_array precede con recurso/estado. | later_adr |
| implementation_limit | Barrera temporal por capacidad K incompleta, según tabla siguiente. Nunca por una discrepancia de tests/cuotas o un fallo interno. | uno de los K enumerados abajo |

Las formas J aceptadas conservan sus contratos aunque no usen ninguna capacidad K. Regresión de aceptación U01 (K01): `gen fn G(){}fn Main(){var g=G();AssertEq(Iterator.Next(edit g),None);}`; UTF-8 sin BOM, sin LF final, archivo U01.n. Item=(), Next devuelve None; check/run exit 0, diagnostics=[], outcome unit, stdout/events vacíos. No se exige anotación ni se emite inferred_non_unit_result, generator o implementation_limit por esta fuente. Las restricciones J de captura, parámetros, recursión de estados y efectos siguen siendo las de F0-0003 §4 y §5; ninguna fila K vuelve a excluir sus formas incluidas. La referencia a sus enmiendas es §10, «Concreciones y enmiendas propuestas», por título y regla, no por número de línea ni por una copia histórica de su fila sobre F0-0004.

Los únicos pending_scope emitibles bajo f0-0004-b-v1 son `later_adr` y los nueve valores `K02`, `K03`, `K04`, `K05`, `K06`, `K07`, `K08`, `K09`, `K10`. I08–I11, J01–J16 y F5-L01–F5-L12 permanecen prerrequisitos históricos: terminar sus capacidades es prerrequisito, no otro modo de ejecución de este perfil. K01/K11/K12 no son capacidades de programa y nunca se usan como excusa implementation_limit.

| Capacidad no terminada | pending_scope |
|---|---|
| String literal/propietario y contabilidad | K02 |
| Texto/parseo, != y cuatro órdenes String (== procede de K02), Array/for String | K03 |
| Nominales de datos, campos, constructores y Array/for nominal | K04 |
| Recursos dentro de nominales | K05 |
| Métodos y receptores | K06 |
| Patrones de nominales | K07 |
| Derivación nominal, ==/!= y capacidad transitiva de colecciones; interpolación | K08 |
| Exportación e imports | K09 |
| Colas de consola | K10 |

Las barreras K se reconocen deliberadamente en types/types.check para todos los K, a diferencia de las barreras por etapa de J/F5: K01 registra descriptores y necesidades de capacidad antes de comprobar cuerpos, y cada K habilita conjuntamente sintaxis, tipos, propiedad, efectos, IR y cuotas de su capacidad. No hay barreras K parciales de ownership/effects/lower ni artefactos ejecutables de una capacidad incompleta. Esto no cambia las etapas de sus errores reales ni reabre barreras J/F5 ya terminadas. La selección temporal conserva el resto del contrato de D15 §7.1: subcause null, data únicamente pending_scope, sin tokens/relaciones/cadenas/reparaciones; primario primera construcción que exige la capacidad, empate por orden K. Los reconocedores de alcance definitivo conservan etapas A2; text por método reconocido y user_types por ciclo se emiten en types/types.check, y modules_visibility por ruta en names/names.resolve. Una barrera sintáctica anterior prevalece. Al terminar la capacidad se elimina su emisión y el ejemplo de explain pasa a histórico; no se mantiene un frontend viejo tras una bandera.

## 10. Fixtures literales y oráculos

### 10.1. Convenciones exactas

Los IDs de fixtures y criterios sin prefijo (incluidos M01–M08, O01/O02, C01 y cualquier Lnn local) son locales a F0-0004. Su identidad documental externa es K4-ID; dentro de este ADR se conserva ID y el nombre de archivo ID.n, incluidos los file de diagnósticos. Los K01–K12 son pasos, no fixtures; F5-B01 y toda cita explícita a otro ADR mantienen su propietario.

Cada fuente siguiente es un archivo independiente `ID.n`, UTF-8 sin BOM, sin LF final. `⟦…⟧` marca el primario y **no forma parte de la fuente**. La tabla numérica se calcula sobre la fuente sin marcas. El positivo es la misma fuente con la única edición indicada; no puede fallar por otra regla. Se ejecuta check completo; cuando tenga Main, también run. Cada negativo produce exactamente un diagnóstico, salvo una fila que diga expresamente lo contrario. El oráculo de offsets se obtiene contando bytes UTF-8 de estos literales, independiente del parser/impresor/checker.

Contrato común C: schema_version=1, diagnostic_namespace=Nexo-proposed-v1, Diagnostic.profile_id=f0-0004-b-v1 salvo X24–X27, severity=error; rule y explanation_id son code[/subcause]. feature_id=null, unexpected_token=null, expected_tokens=[], expected_tokens_kind=null, parser_context=null, recognized_prefix=null, repair=[], cause_chain=[] y data={}, salvo sustitución explícita. primary_span y relacionados usan file=ID.n. Stage/operation se toman de §9 o A2 §13.12. Related_spans no indicados son []. Message no es oráculo de identidad: debe ser una línea sin controles, explicar la regla y mencionar el miembro/capacidad/permiso cuando proceda. No se acepta comparar solo code o buscar un substring del mensaje.

| ID | Fuente negativa completa | Código/subcausa | Edición positiva única |
|---|---|---|---|
| X01 | `fn Main(){let s="⟦{}⟧";}` | NX-E1401/empty_interpolation | `{}` → `{1}` |
| X02 | `fn Main(){let s="⟦}⟧";}` | NX-E1401/unescaped_closing_brace | `}` → `}}` |
| X03 | `fn Main(){let s="⟦{⟧1";}` | NX-E1401/unterminated_interpolation | insertar `}` después de `1` |
| X04 | `fn Main(){let s=⟦"é"⟧[0];}` | NX-E0102/not_indexable | sustituir `"é"` por `[233]` |
| X05 | `struct S{x:I64;}fn Main(){let s=S(1);s.⟦y⟧;}` | NX-E1402/unknown_member | `y` → `x` |
| X06 | `struct S{x:I64;⟦x⟧:I64;}` | NX-E0104/null | segundo `x` → `y` |
| X07 | `enum E{A,⟦A⟧}` | NX-E0104/null | segundo `A` → `B` |
| X08 | `struct S{x:I64;}fn Main(){let s=⟦S⟧();}` | NX-E0111/missing_argument | insertar `1` en la llamada |
| X09 | `struct S{x:I64;}fn Main(){let s=S(⟦z⟧:1);}` | NX-E0111/unknown_argument_label | `z` → `x` |
| X10 | `struct S{x:I64;}fn Main(){let s=S(1);⟦s⟧.x=2;}` | NX-E0105/null | `let s` → `var s` |
| X11 | `struct S{r:MockResource;}fn Main(){let s=S(MockOpen(1));let t=⟦s⟧;}` | NX-E0201/null | `s` del inicializador → `move s` |
| X12 | `struct S{x:I64;}fn Main(){let s=S(1);Print(⟦s⟧);}` | NX-E1403/missing_format | anteponer `@derive(Format)` |
| X13 | `struct S{x:I64;}fn Main(){let s=S(1);AssertEq(s⟦==⟧s,true);}` | NX-E1403/missing_partial_eq | anteponer `@derive(PartialEq)` |
| X14 | `@derive(Format)struct S{r:⟦MockResource⟧;}` | NX-E1403/missing_format | tipo de campo → `I64` |
| X15 | `@derive(Format,⟦Format⟧)struct S{}` | NX-E1404/duplicate_derive | quitar `,Format` segundo |
| X16 | `@derive(⟦Format⟧)fn F(){}` | NX-E1404/invalid_derive_target | `fn F(){}` → `struct F{}` |
| X17 | `pub fn F()⟦⟧ with unrestricted{}` | NX-E1310/missing_signature_type | insertar `->()` en la marca |
| X18 | `pub fn ⟦F⟧()->(){}` | NX-E1310/missing_effect_bound | insertar ` with unrestricted` antes de `{` |
| X19 | `struct S{}pub fn F(x:view ⟦S⟧)->() with unrestricted{}` | NX-E1310/unexported_type | anteponer `pub ` a struct |
| X20 | `struct S{}pub use package.⟦S⟧ as T;` | NX-E1308/private_reexport | anteponer `pub ` a struct |
| X21 | `struct S{x:⟦S⟧;}` | NX-E1323/recursive_value_layout | campo `S` → `I64` |
| X22 | `fn Main(){let s="a";let t=s.⟦Nope⟧();}` | NX-E1402/unknown_member | `Nope` → `ByteLen` |
| X23 | `fn F(s:view String) with no_alloc{let t=⟦s⟧;}` | NX-E0412/null | `let t=s;` → `s.ByteLen();` |
| X24 | `fn Main(){let s=⟦"abcdefghi"⟧;}` | R0002/null, perfil test-string-8 | quitar `i` del literal |
| X25 | `fn Main(){let a="12345678";let b="12345678";let c=⟦"x"⟧;}` | R0002/null, perfil test-text-live-16 | `"x"` → `""` |
| X26 | `fn Main(){let a="";let b="";let c=⟦""⟧;}` | R0002/null, perfil test-text-alloc-2 | `""` del tercer binding → `0` |
| X27 | `struct S{a:I64;b:I64;⟦c⟧:I64;}` | NX-E0003/null, perfil test-members-2 | quitar `c:I64;` |

X17 precisa un primario vacío en el final de `)` de parámetros; no se usa EOF ni se denuncia falta de retorno. La política de missing_signature_type de este perfil relaciona el nombre de F. Esto es una concreción propuesta de D13 §9. Los positivos X06/X07/X14–X21/X23/X27 son bibliotecas: check satisfactorio, no se exige inventar Main para run.

| ID | Primario en bytes | Related_spans (role e intervalo) | Data exacta |
|---|---|---|---|
| X01 | `[17,19)` | delimiter `[16,17)` | `{}` |
| X02 | `[17,18)` | delimiter `[16,17)` | `{}` |
| X03 | `[17,18)` | delimiter `[16,17)` | `{}` |
| X04 | `[16,20)` | found_type igual al primario | `{"expected_type":"domain:array","found_type":"String"}` |
| X05 | `[39,40)` | declaration `[7,8)` | `{"member_name":"y","receiver_type":"package.S"}` |
| X06 | `[15,16)` | previous_declaration `[9,10)` | `{"declaration_kind":"field"}` |
| X07 | `[9,10)` | previous_declaration `[7,8)` | `{"declaration_kind":"variant"}` |
| X08 | `[32,33)` | declaration `[9,10)` | `{}` |
| X09 | `[34,35)` | ninguna | `{}` |
| X10 | `[37,38)` | declaration `[30,31)` | `{}` |
| X11 | `[62,63)` | declaration `[39,40)` | `{}` |
| X12 | `[43,44)` | declaration `[7,8)` | `{"required_capability":"Format"}` |
| X13 | `[47,49)` | declaration `[7,8)` | `{"required_capability":"PartialEq"}` |
| X14 | `[26,38)` | contract `[8,14)` | `{"required_capability":"Format"}` |
| X15 | `[15,21)` | previous_declaration `[8,14)` | `{}` |
| X16 | `[8,14)` | context `[18,19)` | `{}` |
| X17 | `[10,10)` | declaration `[7,8)` | `{}` |
| X18 | `[7,8)` | ninguna | `{}` |
| X19 | `[26,27)` | declaration `[7,8)` | `{}` |
| X20 | `[26,27)` | declaration `[7,8)` | `{}` |
| X21 | `[11,12)` | declaration `[7,8)` | `{}` |
| X22 | `[28,32)` | ninguna | `{"member_name":"Nope","receiver_type":"String"}` |
| X23 | `[40,41)` | contract `[25,33)` | `{"effect_model":"f0-0004-sim-v1","effect_evidence":"known_effect","contract_origin":{"file":"X23.n","start":25,"end":33},"required_effect":"alloc"}` |
| X24 | `[16,27)` | ninguna | `{"runtime_budget_kind":"string_bytes","limit":8,"observed":9}` |
| X25 | `[50,53)` | ninguna | `{"runtime_budget_kind":"live_text_bytes","limit":16,"observed":17}` |
| X26 | `[34,36)` | ninguna | `{"runtime_budget_kind":"text_allocations","limit":2,"observed":3}` |
| X27 | `[21,22)` | ninguna | `{"budget_kind":"nominal_members","budget_scope":"compilation","incomplete_analysis":"types","limit":2,"observed":3}` |

En X01 el token inesperado es `}` en `[18,19)`; X02 `}` en `[17,18)`; X03 `{` en `[17,18)`. X23 tiene exactamente un paso causal: kind=copy, operation=value.copy, span igual al primario, entity=`{"kind":"parameter","id":"parameter:5"}`, evidence=verified. Los otros pasos son []. X24–X26 usan namespace F0-0002-sim, stage runtime, operation runtime.limit, rule=R0002, explanation_id=null; Diagnostic.profile_id es respectivamente `f0-0004-b-test-string-8-v1`, `f0-0004-b-test-text-live-16-v1` y `f0-0004-b-test-text-alloc-2-v1`. X27 usa operation analysis.limit, stage types y Diagnostic.profile_id=`f0-0004-b-test-members-2-v1`. Data es exactamente la tabla: profile_id pertenece exclusivamente al campo exterior Diagnostic. Los positivos de estas cuatro filas usan el mismo perfil reducido que su negativo. X23 usa operation=value.copy. Las demás filas heredan C sin campos ocultos.

### 10.2. Pares de métodos, alcance e invariantes

Las siguientes fuentes también usan C y un único diagnóstico; los intervalos numéricos se fijan en la tabla posterior. Las bibliotecas se comprueban completas. Los atributos y firmas no relacionados con la causa son válidos.

| ID | Negativo completo | Regla y positivo |
|---|---|---|
| M01 | `struct S{}extend S{fn F(self:edit){}}use extension package.S;fn Main(){var s=S();⟦s⟧.F();}` | NX-E1107, positivo insertar `edit ` delante de s.F(). Relacionar declaration del binding s. |
| M02 | `struct S{}extend S{fn F(self:view){}}fn Main(){let s=S();s.⟦F⟧();}` | NX-E1315, positivo insertar `use extension package.S;` antes de Main. Relacionar declaration del método F. |
| M03 | `struct S{}extend S{fn F(self:view){}fn ⟦F⟧(self:view){}}` | NX-E1322, positivo renombrar el segundo F a G. Relacionar previous_declaration del primer F. |
| M04 | `struct S{x:I64;}fn Main(){let s=S(1);let x=move ⟦s.x⟧;}` | NX-E0006/partial_transfer, positivo quitar `move `. Context=move; data={pending_scope:later_adr}; parse/syntax.parse. |
| M05 | `fn Main(){let s="a";s.⟦Graphemes⟧();}` | NX-E0006/text, positivo Graphemes→Scalars. types/types.check; data={pending_scope:later_adr}. |
| M06 | `⟦mod⟧ extra;` | NX-E0006/modules_visibility, positivo sustituir declaración por `struct Extra{}`. parse/syntax.parse; data={pending_scope:later_adr}. |
| M07 | `⟦trait⟧ T{}` | NX-E0006/traits_generics, positivo trait→struct. parse/syntax.parse; data={pending_scope:later_adr}. |
| M08 | `@derive(⟦Hash⟧)struct S{}` | NX-E0006/attributes, positivo Hash→PartialEq. parse/syntax.parse; data={pending_scope:later_adr}. |

| ID | Primario | Relaciones |
|---|---|---|
| M01 | `[81,82)` | declaration `[75,76)` |
| M02 | `[59,60)` | declaration `[22,23)` |
| M03 | `[39,40)` | previous_declaration `[22,23)` |
| M04 | `[48,51)` | context `[43,47)` |
| M05 | `[22,31)` | [] |
| M06 | `[0,3)` | [] |
| M07 | `[0,5)` | [] |
| M08 | `[8,12)` | [] |

M01 lleva data exacta `{"required_mode":"inout","requested_mode":"read"}` y cause_chain=[] conforme a F0-0003. M02/M03 llevan data={} y cause_chain=[]. Para las exclusiones M04–M08 feature_id es el de la fila, rule=explanation_id=NX-E0006, subcause=null; los demás campos C siguen vacíos/null. No se ofrece la edición positiva como repair applicable sin recomprobarla.

### 10.3. Datos negativos de texto que son Result, no diagnósticos

Cada fila es una pareja de programas completos, sustituyendo ARG por el literal exacto de la columna; no hay LF final. Check acepta ambos; run termina normalmente con diagnostics=[] y salida exacta `Err(...)` o `Ok(...)`. Fuente de parseo K03: `fn Main(){Print(std.parse.I64(ARG));}`. En K09 se repite cada par con `use std.parse;fn Main(){Print(parse.I64(ARG));}`, con idéntico resultado y observaciones. Fuente de slicing: `fn Main(){Print("é".SliceBytes(START,END));}`. Print de Result<String,I64> muestra contenido crudo según §3.1.

| Regla | Negativo | stdout negativo | Positivo pareado | stdout positivo |
|---|---|---|---|---|
| Cadena vacía no es entero | `""` | `Err(-9101)` | `"0"` | `Ok(0)` |
| Solo ASCII | `"١"` | `Err(-9101)` | `"1"` | `Ok(1)` |
| Sin whitespace implícito | `" 1"` | `Err(-9101)` | `"1"` | `Ok(1)` |
| Sin signo más | `"+1"` | `Err(-9101)` | `"1"` | `Ok(1)` |
| Rango superior | `"9223372036854775808"` | `Err(-9102)` | `"9223372036854775807"` | `Ok(9223372036854775807)` |
| Rango inferior | `"-9223372036854775809"` | `Err(-9102)` | `"-9223372036854775808"` | `Ok(-9223372036854775808)` |
| Frontera UTF-8 | START=0, END=1 | `Err(-9111)` | END=2 | `Ok(é)` |
| Rango fuera | START=0, END=3 | `Err(-9110)` | END=2 | `Ok(é)` |
| Rango invertido | START=2, END=0 | `Err(-9110)` | START=0 | `Ok()` |
| Rango negativo | START=-1, END=0 | `Err(-9110)` | START=0 | `Ok()` |

Una observación stdout por fila, sin eventos, LF añadido; bytes contados por UTF-8. Se añaden positivos de `"0007"`, `"-0"` y negativos de `"-"`, `"1x"`, con resultados 7, 0 y -9101/-9101. No se confunde un retorno Err de parseo con rechazo de compilación o R0101.

### 10.4. Integración nominal, permisos y barrera temporal

Se aplica C, archivo `Nnn.n`, fuente sin marcas ni LF final y un solo diagnóstico. La tabla incluye el positivo como edición única sobre la fuente completa. N06/N09/N11/N12 son bibliotecas. N13 es una prueba de la barrera de capacidades con K02 deshabilitado en un manifiesto interno de prueba, nunca un selector público de dialecto: su positivo debe poder comprobarse con el mismo manifiesto.

| ID | Fuente negativa | Código/subcausa, primario | Edición positiva |
|---|---|---|---|
| N01 | `enum E{A(I64,I64)}fn Main(){let e=E.A(1,2);match e{⟦E.A(x)⟧=>{}}}` | NX-E1325/constructor_arity `[51,57)` | patrón E.A(x) → E.A(x,y) |
| N02 | `enum E{A,B}fn Main(){⟦match⟧ E.A{E.A=>{}}}` | NX-E1304/non_exhaustive `[21,26)` | añadir brazo `,E.B=>{}` |
| N03 | `enum E{A}fn Main(){let e=E.⟦B⟧;}` | NX-E1402/unknown_member `[27,28)` | B → A |
| N04 | `struct A{}struct B{}fn Main(){let x:A=⟦B()⟧;}` | NX-E0102/type_mismatch `[38,41)` | constructor B → A |
| N05 | `enum E{A,B(MockResource)}fn Main(){let e=E.A;let f=⟦e⟧;}` | NX-E0201/null `[51,52)` | inicializador e → move e |
| N06 | `struct S{x:I64;}fn F(s:view S){⟦s⟧.x=2;}` | NX-E1105/permission_escalation `[31,32)` | view → edit |
| N07 | `struct S{x:I64;y:I64;}fn F(a:edit I64,b:edit I64){}fn Main(){var s=S(1,2);F(edit s.x,edit ⟦s.y⟧);}` | NX-E1105/overlapping_mutable_borrows `[90,93)` | insertar `var t=S(3,4);` antes de F y cambiar solo el segundo origen s a t |
| N08 | `struct S{}fn Main(){let s=S();Drop(s);let t=⟦s⟧;}` | NX-E0202/definitely_consumed `[44,45)` | quitar `Drop(s);` |
| N09 | `fn F() with no_alloc{let s=⟦"a"⟧;}` | NX-E0412/null `[27,30)` | literal → 0 |
| N10 | `struct S{}fn Main(){let s=S();let t="{⟦s⟧}";}` | NX-E1403/missing_format `[38,39)` | anteponer @derive(Format) |
| N11 | `@derive(PartialEq)struct S{r:⟦MockResource⟧;}` | NX-E1403/missing_partial_eq `[29,41)` | tipo de campo → I64 |
| N12 | `⟦enum⟧ E{}` | NX-E0006/user_types `[0,4)` | insertar A dentro de llaves |
| N13 | `fn Main(){let s=⟦"a"⟧;}` | NX-E0006/implementation_limit `[16,19)` | literal → 0 |

N07 cambia el atributo lógico «identidad de raíz del segundo préstamo»; la declaración adicional permite aislarlo sin dejar un nombre ausente. No demuestra disjunción de campos de una misma raíz.

| ID | Data exacta | Related_spans |
|---|---|---|
| N01 | `{"constructor":"package.E.A","expected_arity":2,"found_arity":1}` | [] |
| N02 | `{"missing_pattern":"package.E.B"}` | [] |
| N03 | `{"member_name":"B","receiver_type":"package.E"}` | declaration `[5,6)` |
| N04 | `{"expected_type":"package.A","found_type":"package.B"}` | expected_type `[36,37)`, found_type `[38,41)` |
| N05 | `{}` | declaration `[39,40)` |
| N06 | `{"origin":{"kind":"parameter","id":"parameter:21"},"parameter":{"kind":"parameter","id":"parameter:21"},"active_permission":"read","requested_access":"write"}` | declaration `[21,22)` |
| N07 | `{"origin":{"kind":"binding","id":"binding:65"},"binding":{"kind":"binding","id":"binding:65"},"loan":{"kind":"loan","id":"loan:76:0"},"active_permission":"inout","requested_access":"borrow_inout"}` | declaration `[65,66)`, origin `[76,84)` |
| N08 | `{}` | declaration `[24,25)`, consumption `[30,37)` |
| N09 | `{"effect_model":"f0-0004-sim-v1","effect_evidence":"known_effect","contract_origin":{"file":"N09.n","start":12,"end":20},"required_effect":"alloc"}` | contract `[12,20)` |
| N10 | `{"required_capability":"Format"}` | declaration `[7,8)` |
| N11 | `{"required_capability":"PartialEq"}` | contract `[8,17)` |
| N12 | `{"pending_scope":"later_adr"}` | [] |
| N13 | `{"pending_scope":"K02"}` | [] |

N01/N02: types/types.check de F0-0003; N12: parse/syntax.parse; N13: types/types.check, mensaje exacto `Capacidad pendiente de implementación en K02.`. N12/N13 usan feature_id de su fila, subcause=null, rule=explanation_id=NX-E0006. Los demás stages/operations son los de C, salvo N09 effects/value.build_string.

Cause_chain, un paso por cada fila siguiente y [] para las restantes; cada paso tiene evidence=verified, span con el file de la fila y todos sus campos obligatorios:

| ID | kind / operation | span | entity |
|---|---|---|---|
| N06 | borrow / borrow.read | `[21,22)` | `{"kind":"parameter","id":"parameter:21"}` |
| N07 | borrow / borrow.inout | `[76,84)` | `{"kind":"loan","id":"loan:76:0"}` |
| N08 | drop / value.drop | `[30,37)` | `{"kind":"binding","id":"binding:24"}` |
| N09 | effect / value.build_string | `[27,30)` | `{"kind":"type","id":"String"}` |

### 10.5. Fronteras de Hole, ciclos, exportación y temporales

Cada fuente se representa mediante **una cadena JSON**: primero se decodifica JSON una vez y después se codifica el resultado en UTF-8 sin BOM, sin añadir LF. Así `\r\n` en JSON representa bytes 0D 0A reales y `\\n` representa barra y letra n en fuente Nexo. Los offsets cuentan esa fuente, nunca el texto JSON. Se aplica C; H09/H10 son bibliotecas. Cada negativo da un único diagnóstico y cada positivo acepta check y, si tiene Main, run.

| ID | Fuente negativa completa (JSON) | Código/subcausa | Primario UTF-8 | Edición positiva |
|---|---|---|---|---|
| H01 | `"fn Main(){let s=\"é{1\r\n}\";}"` | NX-E1401/raw_line_break_in_interpolation | `[21,23)` | sustituir CRLF por un espacio |
| H02 | `"fn Main(){let s=\"é{1\r}\";}"` | NX-E1401/raw_line_break_in_interpolation | `[21,22)` | sustituir CR por un espacio |
| H03 | `"fn Main(){let s=\"é{1\n}\";}"` | NX-E1401/raw_line_break_in_interpolation | `[21,22)` | sustituir LF por un espacio |
| H04 | `"fn Main(){let s=\"é{1//}\";}"` | NX-E1401/comment_in_interpolation | `[21,23)` | quitar las dos barras |
| H05 | `"fn Main(){let s=\"é{ \t }\";}"` | NX-E1401/empty_interpolation | `[19,24)` | insertar `1` antes de `}` del Hole |
| H06 | `"fn Main(){let s=\"é{1 + \";}"` | NX-E1401/incomplete_interpolation_expression | `[24,25)` | insertar `2}` antes de la comilla primaria |
| H07 | `"struct S{}\r\nfn Main(){let s=S();let t=\"é\\n{s}\";}"` | NX-E1403/missing_format | `[44,45)` | anteponer `@derive(Format)` |
| H08 | `"struct S{}fn Main(){let s=S();let t=\"é\\\"\\\\{s}\";}"` | NX-E1403/missing_format | `[44,45)` | anteponer `@derive(Format)` |
| H09 | `"struct A{b:B;}struct B{a:A;}"` | NX-E1323/recursive_value_layout | `[25,26)` | sustituir el tipo primario `A` por `I64` |
| H10 | `"struct S{}pub enum E{A(S)}"` | NX-E1310/unexported_type | `[23,24)` | anteponer `pub ` a struct |
| H11 | `"enum E{A,B(I64,Bool)}fn Main(){match E.A{E.A=>{}}}"` | NX-E1304/non_exhaustive | `[31,36)` | añadir `,E.B(_,_)=>{}` después del brazo A |
| H12 | `"struct S{}extend S{fn F(self:edit){}}use extension package.S;fn Main(){S().F();}"` | NX-E1107/null | `[71,74)` | cambiar `self:edit` por `self:view` |
| H13 | `"struct S{}extend S{fn F(self:move){}}use extension package.S;fn Main(){S().F();}"` | NX-E1107/null | `[71,74)` | cambiar `self:move` por `self:view` |

| ID | Data exacta | Related_spans | Unexpected_token |
|---|---|---|---|
| H01 | `{}` | delimiter [16,17) | kind=invalid, text=`"\r\n"`, span=`[21,23)` |
| H02 | `{}` | delimiter [16,17) | kind=invalid, text=`"\r"`, span=`[21,22)` |
| H03 | `{}` | delimiter [16,17) | kind=invalid, text=`"\n"`, span=`[21,22)` |
| H04 | `{}` | delimiter [16,17) | kind=symbol, text=`"//"`, span=`[21,23)` |
| H05 | `{}` | delimiter [16,17) | kind=symbol, text=`"}"`, span=`[23,24)` |
| H06 | `{}` | delimiter [16,17) | kind=symbol, text=`"\""`, span=`[24,25)` |
| H07 | `{"required_capability":"Format"}` | declaration [7,8) | null |
| H08 | `{"required_capability":"Format"}` | declaration [7,8) | null |
| H09 | `{}` | declaration [7,8) | null |
| H10 | `{}` | declaration [7,8) | null |
| H11 | `{"missing_pattern":"package.E.B(0,false)"}` | [] | null |
| H12 | `{"required_mode":"inout","requested_mode":"read"}` | [] | null |
| H13 | `{"required_mode":"consume","requested_mode":"read"}` | [] | null |

Todos tienen cause_chain=[], repair=[] y los campos parser vacíos/null de C. H01–H06 usan parse/syntax.parse; H07–H09 y H11 types/types.check; H10 names/names.resolve; H12/H13 ownership/ownership.check. El file de todos los spans es ID.n. H07 tiene CRLF antes del literal, `é` de dos bytes y escape fuente `\n` antes del Hole; H08 tiene `é`, comilla escapada y barra escapada antes del Hole. No se convierten esos escapes en offsets de la cadena decodificada.

H09 tiene aristas A→B en [11,12) y B→A en [25,26): comienza en declaración A [7,8), por lo que el primario es la segunda arista aunque la primera tenga menor offset. H11 concreta el testigo por variantes en orden de declaración y payloads por las reglas de F0-0003 §3.3: I64 no cubierto empieza en 0, Bool en false. La representación es ruta nominal completa, paréntesis y payloads separados por coma sin espacios; un payload opaco usa `_`. No se omite el payload ni se inventa `package.E.B` como patrón nular.

Las variantes prefijadas conservan las firmas originales y usan C con las sustituciones siguientes. Cada negativo produce exactamente un NX-E0002, subcause=null, rule=explanation_id=NX-E0002, stage=parse, operation=syntax.parse, data={}, related_spans=[], cause_chain=[], repair=[], feature_id=null, expected_tokens=["Place"], expected_tokens_kind=admissible_set. Unexpected_token es {kind:"symbol",text:"(",span:primario}; recognized_prefix incluye el nombre S ya admitido como raíz. No se ejecuta propiedad sobre ese receptor inválido.

| ID | Negativo completo | Primario | parser_context; recognized_prefix | Positivo completo |
|---|---|---|---|---|
| H12-edit | `struct S{}extend S{fn F(self:edit){}}use extension package.S;fn Main(){edit S().F();}` | `[77,78)` | inout_place; `[71,77)` | `struct S{}extend S{fn F(self:edit){}}use extension package.S;fn Main(){var s=S();edit s.F();}` |
| H13-move | `struct S{}extend S{fn F(self:move){}}use extension package.S;fn Main(){move S().F();}` | `[77,78)` | consume_place; `[71,77)` | `struct S{}extend S{fn F(self:move){}}use extension package.S;fn Main(){let s=S();move s.F();}` |

Los archivos son H12-edit.n/H13-move.n; los positivos tienen diagnostics=[], outcome unit, exit 0, stdout/events vacíos. En ambos positivos el binding s está en [75,76), S() en [77,80), el prefijo en [81,85) y el receptor Place s en [86,87). En H12/H13 sin prefijo se conservan íntegros el primario [71,74), data de modos y positivos publicados arriba; el receptor S() del positivo sin prefijo sigue en [71,74).

### 10.6. Payloads inválidos de ReadLine

Programa común: `use std.console;fn Main()->Result<(),I64>{console.ReadLine()?;return Ok(());}`. Entorno base completo (una respuesta válida):

```json
{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"console_read_line":[{"ok":"é"}],"console_println":[]}
```

Cada negativo sustituye exclusivamente el valor ok de la respuesta. Los fragmentos de la tabla son JSON, no String Nexo.

| ID | Valor negativo / mutación | Razón exacta | Positivo |
|---|---|---|---|
| E01 | `7` | invalid_mock_schema | `"é"` |
| E02 | `"a\r"` | invalid_mock_schema | `"a"` |
| E03 | `"a\n"` | invalid_mock_schema | `"a"` |
| E04 | `"a\r\n"` | invalid_mock_schema | `"a"` |
| E05 | `"\ud800"` | invalid_mock_json | `"é"` |
| E06 | En los bytes UTF-8 del entorno base, sustituir únicamente C3 A9 (é) por C3 28 | invalid_mock_json | restaurar C3 A9 |

Cada negativo falla en el controlador, exit 2 y razón indicada, sin ejecutar Main, consumir colas ni emitir observaciones; no genera diagnóstico NX-E/R de programa ni Err tipado de ReadLine. Se valida el entorno completo incluso si la respuesta inválida está después de EOF. Cada positivo tiene diagnostics=[], outcome ok y exit 0; ningún stdout, un evento `read_line:ok:2` para é o `read_line:ok:1` para a. Los dos tipos de evento cuestan 23 observation_bytes (14 de texto, 8 de prefijo, 1 LF). Los surrogates escapados no se convierten a U+FFFD. Como control de error tipado válido, sustituir la respuesta por `{"err":"70"}` acepta el esquema y produce outcome Err(70), exit 1, evento `read_line:err:70` y ningún stdout.

### 10.7. Operadores y colecciones heredados, extendidos a texto y nominales

Se conservan íntegros los 48 primarios X01–X27/M01–M08/N01–N13 y los 13 H01–H13; los pares O son adicionales. Se aplica C: archivo ID.n, UTF-8 sin BOM, sin LF final; types/types.check salvo etapa explícita. Un solo diagnóstico por negativo, ninguna observación; positivo check/run sin diagnósticos, unidad, exit 0, stdout/events vacíos. Token significa fragmento primario, no unexpected_token: este permanece null. Cause_chain=[], repair=[] y relaciones solo las indicadas. Una familia OP se materializa como un archivo por operador, conservando espacios; L es su longitud ASCII. Ningún esperado procede del compilador.

| ID | Fuente negativa completa | Diagnóstico, primario/token, data y relaciones | Positivo pareado |
|---|---|---|---|
| O01; OP=<, <=, >, >=, ==, != | `fn Main(){let x="é" OP 1;}` | NX-E0102/type_mismatch; `[22+L,23+L)`, `1`; {expected_type=String, found_type=I64}; relaciones expected_type [16,20), found_type igual al primario. | reemplazar 1 por "z" |
| O02; OP=<, <=, >, >=, ==, != | `fn Main(){let x=1 OP "é";}` | NX-E0102/type_mismatch; `[19+L,23+L)`, `"é"`; {expected_type=I64, found_type=String}; relaciones expected_type [16,17), found_type igual al primario. | 1 izquierdo por "a" |
| O03; OP=<, <=, >, >= | `fn Main(){let x=true OP false;}` | NX-E0102/type_mismatch; `[21,21+L)`, `OP`; {expected_type=domain:ordered_scalar, found_type=Bool}; relaciones found_type [16,20). | true/false por "a"/"b" (solo el dominio de ambos operandos) |
| O04 | `fn Main(){let x="é"+"x";}` | NX-E0102/type_mismatch; `[16,20)`, `"é"`; {expected_type=I64, found_type=String}; relaciones found_type igual al primario. | + por == |
| O05; OP en ==, != | `struct S{}fn Main(){let s=S();let x=s OP s;}` | NX-E1403/missing_partial_eq; `[38,38+L)`, `OP`; {required_capability=PartialEq}; relaciones declaration [7,8). | anteponer @derive(PartialEq) |
| O06; OP en ==, != | `struct S{}fn Main(){let a:Array<S>=[];let x=a OP a;}` | NX-E1403/missing_partial_eq; `[46,46+L)`, `OP`; {required_capability=PartialEq}; relaciones declaration [7,8). | anteponer @derive(PartialEq) |
| O07; OP en ==, != | `@derive(PartialEq)struct A{}@derive(PartialEq)struct B{}fn Main(){let x=A() OP B();}` | NX-E0102/type_mismatch; `[77+L,80+L)`, `B()`; {expected_type=package.A, found_type=package.B}; relaciones expected_type [72,75), found_type igual al primario. | B() por A() |
| O08; OP=<, <=, >, >= | `@derive(PartialEq)struct S{}fn Main(){let s=S();let x=s OP s;}` | NX-E0102/type_mismatch; `[56,56+L)`, `OP`; {expected_type=domain:ordered_scalar, found_type=package.S}; relaciones found_type [54,55). | operador de orden por == |
| O09 | `struct S{r:MockResource;}fn Main(){let a:Array<S>=[];}` | NX-E0006; `[41,49)`, `Array<S>`; {pending_scope=later_adr; feature_id=resource_array}; relaciones []. | MockResource por I64 |
| O10 | `fn Main(){for x in "é"{}}` | NX-E0102/type_mismatch; `[19,23)`, `"é"`; {expected_type=domain:iterable, found_type=String}; relaciones found_type igual al primario. | "é" por ["é"] |
| O11 | `fn Main(){for edit x in ["é"]{}}` | NX-E0006; `[14,18)`, `edit`; {pending_scope=later_adr; feature_id=for_iteration; parse/syntax.parse}; relaciones []. | quitar edit y el espacio siguiente |
| O12 | `struct S{}fn Main(){for x in S(){}}` | NX-E0102/type_mismatch; `[29,32)`, `S()`; {expected_type=domain:iterable, found_type=package.S}; relaciones found_type igual al primario. | S() por [S()] |

En O07 solo se materializan ==/!=; el positivo cambia únicamente la identidad del constructor derecho. Las relaciones se ordenan por archivo/inicio/fin/rol. NX-E0006 usa rule=explanation_id=NX-E0006, subcause=null y solo pending_scope en data; feature_id y etapa de la tabla son campos exteriores. Los dominios diagnósticos no crean traits ni tipos fuente.

Pares adicionales de capacidad transitiva, con C y tipos/types.check (stage serializado `types`), sin salida; positivos anteponen `@derive(PartialEq)` a S, conservando toda la operación. NX-E1403/missing_partial_eq, data={"required_capability":"PartialEq"}, feature_id=null, token parser null y cadena vacía:

| ID | Fuente negativa completa | Primario/token | Relaciones |
|---|---|---|---|
| O14 | `struct S{}fn Main(){let a=Some(S());let x=a!=a;}` | [43,45), `!=` | declaration [7,8) |
| O15 | `struct S{}fn Main(){let a:Result<S,I64>=Ok(S());let x=a!=a;}` | [55,57), `!=` | declaration [7,8) |
| O16 | `struct S{}@derive(PartialEq)enum E{Nada,Dato(S)}fn Main(){}` | [45,46), `S` | declaration [7,8), contract [18,27) |

O16 comprueba todas las variantes, aunque el programa no construya Dato. Un Array<S> sin PartialEq sigue siendo construible e iterable: solo la operación de comparación impone O06.

Programa positivo de seis comparadores, UTF-8 sin normalización y valores independientes; LF entre líneas, LF final, salida/eventos vacíos:

```nexo
@derive(PartialEq)struct S{text:String;}
@derive(PartialEq)enum E{Nada,Dato(S)}
fn Main(){
    AssertEq("é"=="é",true);
    AssertEq("é"!="é",true);
    AssertEq("é"<"🙂",true);
    AssertEq("é"<="é",true);
    AssertEq("🙂">"é",true);
    AssertEq("é">="é",true);
    AssertEq("a"<"aa",true);
    AssertEq(""<"a",true);
    AssertEq("a">"aa",false);
    var a:Array<E>=[];
    Push(edit a,E.Dato(S("é")));
    Push(edit a,E.Nada);
    var b=a;
    AssertEq(Pop(edit b),Some(E.Nada));
    AssertEq(a!=b,true);
    Push(edit b,E.Nada);
    AssertEq(a==b,true);
    let c=JoinArrays(a,b);
    AssertEq(Len(c),4);
    var n=0;
    for item in c{
        match item{E.Nada=>{},E.Dato(s)=>{n+=s.text.ByteLen();}}
    }
    AssertEq(n,4);
}
```

El programa anterior integra K07/K08 y se exige en K11. K04 tiene este programa independiente sin derive ni match nominal; LF entre líneas y LF final, unidad, diagnostics=[], stdout/events=[], exit 0:

```nexo
struct S{text:String;}
enum E{Nada,Dato(S)}
fn Main(){
    var a=[S("é")];
    var b=a;
    Push(edit b,S("x"));
    AssertEq(Len(a),1);
    AssertEq(Len(b),2);
    var bytes=0;
    for r in b{bytes+=r.text.ByteLen();}
    AssertEq(bytes,3);
    var es=[E.Nada];
    Push(edit es,E.Dato(S("y")));
    let popped=Pop(edit es);
    match popped{None=>{AssertEq(false,true);},Some(v)=>{}}
    var count=0;
    for e in JoinArrays(es,es){count+=1;}
    AssertEq(count,2);
}
```

Los pares O05/O06 se ejecutan al completar K08; una barrera anterior no acredita missing_partial_eq.

Las seis comparaciones se prueban también con operandos obtenidos de funciones que imprimen 1 y 2 respectivamente: stdout `1\n2\n`, cuatro bytes, sin eventos, cada función llamada una vez. El orden lexicográfico no reevalúa ni invierte operandos. La matriz escalar incluye prefijo igual, vacío, U+0000, U+007F/U+0080, U+07FF/U+0800, U+FFFF/U+10000, y e+U+0301 frente a U+00E9; el oráculo usa bytes sin signo, no colación del anfitrión.

Bajo test-text-alloc-2, `fn Main(){var a=["é"];for x in a{x.ByteLen();}Push(edit a,"x");Pop(edit a);}` acepta: dos búferes String iniciados, C1→C2, 32+64=96 bytes Array acumulados, pico Array 96, pico texto 3, y Pop no copia. El alias de for no inicia otro búfer. Cambiar solo el argumento `"x"` por `"x".Concat("")` exige más de dos asignaciones y falla en el primer literal vacío que intenta iniciar la tercera, antes de llamar Concat. Negativo O13 completo: `fn Main(){var a=["é"];for x in a{x.ByteLen();}Push(edit a,"x".Concat(""));Pop(edit a);}`; R0002/runtime.limit, perfil f0-0004-b-test-text-alloc-2-v1, namespace F0-0002-sim, stage runtime, primario [70,72), token `""`, data={"runtime_budget_kind":"text_allocations","limit":2,"observed":3}, rule=R0002, explanation_id=null, sin relaciones/cadena; restantes campos C. Positivo: la fuente anterior con argumento simple `"x"`. Se conserva el prefijo de asignaciones y limpieza completa, sin observaciones. Los perfiles storage-96/allocation-96 de F5 se repiten con elementos String y nominales; sus slots y candidatos 96/192/224 no cambian, y las cuotas de texto normales no deben anticiparlos.

Los siguientes pares B01-text/B01-struct/B01-enum materializan F0-0005 §9.2, F5-B01. Se aplica C, archivos ID.n, stage=ownership, operation=ownership.access, code=NX-E1105, subcause=active_borrow_conflict, feature_id=null. Cada negativo tiene un solo diagnóstico, sin ejecución ni observaciones; cada positivo check/run aceptado, diagnostics=[], outcome unit, exit 0 y stdout/events vacíos. No requieren derive. Las fuentes positivas cambian únicamente el origen del iterando; la escritura conserva la raíz original a.

| ID | Negativo literal completo | Primario/token | Positivo literal completo |
|---|---|---|---|
| B01-text | `fn Main(){var a=["x"];for x in a{a[0]="y";}}` | [33,34), a | `fn Main(){var a=["x"];for x in ["x"]{a[0]="y";}}` |
| B01-struct | `struct S{x:I64;}fn Main(){var a=[S(1)];for x in a{a[0]=S(2);}}` | [50,51), a | `struct S{x:I64;}fn Main(){var a=[S(1)];for x in [S(1)]{a[0]=S(2);}}` |
| B01-enum | `enum E{V(I64)}fn Main(){var a=[E.V(1)];for x in a{a[0]=E.V(2);}}` | [50,51), a | `enum E{V(I64)}fn Main(){var a=[E.V(1)];for x in [E.V(1)]{a[0]=E.V(2);}}` |

| ID | Data exacta | Related_spans | Cause_chain (un paso) |
|---|---|---|---|
| B01-text | `{"origin":{"kind":"binding","id":"binding:14"},"binding":{"kind":"binding","id":"binding:14"},"loan":{"kind":"loan","id":"loan:31:0"},"active_permission":"read","requested_access":"write"}` | declaration [14,15), origin [31,32) | borrow / borrow.read, span [31,32), entity {kind:loan,id:loan:31:0}, evidence=verified |
| B01-struct | `{"origin":{"kind":"binding","id":"binding:30"},"binding":{"kind":"binding","id":"binding:30"},"loan":{"kind":"loan","id":"loan:48:0"},"active_permission":"read","requested_access":"write"}` | declaration [30,31), origin [48,49) | borrow / borrow.read, span [48,49), entity {kind:loan,id:loan:48:0}, evidence=verified |
| B01-enum | `{"origin":{"kind":"binding","id":"binding:28"},"binding":{"kind":"binding","id":"binding:28"},"loan":{"kind":"loan","id":"loan:48:0"},"active_permission":"read","requested_access":"write"}` | declaration [28,29), origin [48,49) | borrow / borrow.read, span [48,49), entity {kind:loan,id:loan:48:0}, evidence=verified |

B01-text cierra en K03; B01-struct/B01-enum en K04. En los positivos el iterando ocupa respectivamente [31,36), [48,54), [48,56), y la raíz escrita [37,38), [55,56), [57,58). El préstamo acaba al salir del for. La prohibición de escape del alias mantiene NX-E1104 de F0-0003 y F0-0005 §3.2; copiar explícitamente un elemento copiable produce un propietario independiente.

Mutaciones inversas obligatorias: conceder orden nominal por derive; omitir PartialEq de Array vacío; copiar al avanzar for; copiar String al crecer; copiar en Pop; comparar > invirtiendo evaluación; cobrar != dos veces; omitir cuotas de búfer o de texto. Restauración por SHA-256 por regla, sin sustituir pruebas de fuente por IR interna.

### 10.8. Pares nominales y acceso a interfaz

Se aplica C, con un único diagnóstico por negativo y los cambios de campos indicados. Los campos no indicados conservan C, incluidos unexpected_token=null y repair=[]. P01/P02/P03 son bibliotecas: check del positivo acepta sin diagnósticos; no se les exige Main. P05/P06 positivos también ejecutan con outcome unit, exit 0, diagnostics=[] y stdout/events vacíos. Los pares pertenecen a P01→K06, P02/P05→K04, P03/P04→K09 y P06→K04.

| ID | Fuente negativa completa | Diagnóstico; primario/token; etapa/operación | Positivo completo |
|---|---|---|---|
| P01 | `struct S{x:I64;}extend S{fn F(self:view){self.x=2;}}` | NX-E1105/permission_escalation; [41,45), self; ownership/ownership.access | `struct S{x:I64;}extend S{fn F(self:edit){self.x=2;}}` |
| P02 | `struct S{x:Result<S,I64>;}` | NX-E1323/recursive_value_layout; [18,19), S; types/types.check | `struct S{x:Result<I64,I64>;}` |
| P03 | `struct A{}struct B{}use package.A as B;` | NX-E1309/alias_collision; [37,38), B; names/names.resolve | `struct A{}struct B{}use package.A as C;` |
| P05 | `enum Evento{Inicio}fn Main(){Evento.Inicio();}` | NX-E0111/non_callable_target; [29,42), Evento.Inicio; types/call.bind | `enum Evento{Inicio}fn Main(){Evento.Inicio;}` |
| P06 | `struct S{campo:I64;}fn Main(){let p=S(1);Drop(p.campo);}` | NX-E0006, feature_id=partial_transfer; [46,53), p.campo; arguments/call.bind | `struct S{campo:I64;}fn Main(){let p=S(1);Drop(p);}` |

| ID | Data exacta | Related_spans | Cause_chain |
|---|---|---|---|
| P01 | `{"origin":{"kind":"parameter","id":"parameter:30"},"parameter":{"kind":"parameter","id":"parameter:30"},"active_permission":"read","requested_access":"write"}` | declaration [30,34) | Un paso borrow/borrow.read, span [30,34), entity {kind:parameter,id:parameter:30}, evidence=verified |
| P02 | `{}` | declaration [7,8) | [] |
| P03 | `{}` | previous_declaration [17,18) | [] |
| P05 | `{}` | [] | [] |
| P06 | `{"pending_scope":"later_adr"}` | context [41,45) | [] |

P02 tiene una sola arista nominal S→S por Result; no se transforma en límite de profundidad ni ciclo de estado J. P03 contrasta dos identidades nominales distintas; no es repetición del mismo import. P05 no relaciona un binding ocultante inexistente. En P06 subcause=null y rule=explanation_id=NX-E0006. Los sitios correspondientes de los positivos son P01 self [41,45), P02 I64 [18,21), P03 C [37,38), P05 Evento.Inicio [29,42), P06 p [46,47); ninguno es primario diagnóstico.

P04 es una fixture de la API interna de resolución de interfaces (§5.3), no un programa CLI de dos módulos. Entrada literal de declaración en P04-def.n: `struct S{}`; dominio definidor=package; identidad nominal=package.S; privado. Petición literal en P04.n: `use package.S as T;`; dominio solicitante=client, exterior al definidor. El registro interno contiene esa declaración y su visibilidad, sin cargar archivos ni habilitar imports de módulos fuente. Se ejecuta el resolvedor de acceso común con esos datos; no el reconocedor CLI de módulos pendientes.

Esperado P04: NX-E1308/private_access, names/names.resolve, primario {file:P04.n,start:12,end:13}, token fuente S, data={}, related_spans=[{role:declaration,span:{file:P04-def.n,start:7,end:8}}], cause_chain=[]; restantes campos C, sin instalar T ni producir observaciones. Positivo: sustituir exclusivamente la declaración por `pub struct S{}` (nombre S [11,12)), conservando dominio solicitante y petición. Resuelve T a package.S, diagnostics=[], sin ejecución ni observaciones. Se prueba acceso antes de exportabilidad; X20 sigue siendo el par independiente private_reexport en el propio dominio privado.

## 11. Aplicación de referencia

Archivo `total-registros.n`, 81 líneas, UTF-8 sin BOM, LF entre líneas y LF final. Procesa hasta cien líneas simuladas; EOF o `fin` terminan el lote, una línea vacía se ignora. Cada importe entero expresa euros entre 0 y 10.000; entre 10 y 100 inclusive se descuenta 10 %, y se añade IVA del 21 %, truncado a céntimos. Acumula registros propietarios antes de imprimirlos. Al alcanzar cien líneas no lee una respuesta adicional. El máximo total, 121.000.000 céntimos, cabe en I64. No hay atajo por fuente ni nombre de archivo.

```nexo
use std.console;
use std.parse;
use extension package.Resumen;
const maxLineas: I64 = comptime 100;
const iva: I64 = comptime 21;

@derive(Format, PartialEq)
struct Registro {
    linea: I64;
    original: String;
    centimos: I64;
}

@derive(Format, PartialEq)
enum Lectura {
    Vacia,
    Dato(Registro)
}

struct Resumen {
    cantidad: I64;
    total: I64;
}
extend Resumen {
    fn Agregar(self: edit, r: view Registro) {
        self.total += r.centimos;
        self.cantidad += 1;
    }
    fn Texto(self: view) -> String {
        var media = 0;
        if self.cantidad != 0 {
            media = self.total / self.cantidad;
        }
        return "Registros: {self.cantidad}; total: {self.total / 100} EUR y {self.total % 100} cent; media: {media}";
    }
}

fn Convertir(linea: view I64, texto: view String) -> Result<Lectura, I64> {
    if texto == "" {
        return Ok(Lectura.Vacia);
    }
    let importe = parse.I64(texto)?;
    if importe < 0 or importe > 10000 {
        return Err(42);
    }
    var base = importe * 100;
    if importe >= 10 and importe <= 100 {
        base = base * 90 / 100;
    }
    let centimos = base + base * iva / 100;
    return Ok(Lectura.Dato(Registro(linea, texto, centimos)));
}

fn Main() -> Result<(), I64> {
    var registros: Array<Registro> = [];
    for numero in 1..=maxLineas {
        let siguiente = console.ReadLine()?;
        match move siguiente {
            Some(move texto) => {
                if texto == "fin" {
                    break;
                }
                let lectura = Convertir(numero, texto)?;
                match move lectura {
                    Lectura.Vacia => { continue; },
                    Lectura.Dato(move r) => {
                        Push(edit registros, move r);
                    }
                }
            },
            None => { break; }
        }
    }
    var resumen = Resumen(0, 0);
    for r in registros {
        edit resumen.Agregar(r);
        console.Println("L{r.linea}: {r.original} EUR => {r.centimos} cent")?;
    }
    console.Println(resumen.Texto())?;
    return Ok(());
}
```

Entorno completo:

```json
{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"console_read_line":[{"ok":"12"},{"ok":""},{"ok":"8"},{"ok":"5"},{"ok":null}],"console_println":[{"ok":null},{"ok":null},{"ok":null},{"ok":null}]}
```

Cálculo del importe: 12×100×90/100=1080, IVA=1080×21/100=226, resultado 1306; 8×100+800×21/100=968; 5×100+500×21/100=605. Total=2879; cociente/resto por 100=28/79; media=2879/3=959. Las divisiones truncan hacia cero.

Stdout exacto, UTF-8 con LF final:

```text
L1: 12 EUR => 1306 cent
L3: 8 EUR => 968 cent
L4: 5 EUR => 605 cent
Registros: 3; total: 28 EUR y 79 cent; media: 959
```

| Observación, en orden | Bytes del payload | Bytes imputados |
|---|---:|---:|
| event `read_line:ok:2` | 14 | 23 |
| event `read_line:ok:0` | 14 | 23 |
| event `read_line:ok:1` | 14 | 23 |
| event `read_line:ok:1` | 14 | 23 |
| event `read_line:eof` | 13 | 22 |
| event `println:ok:23` | 13 | 22 |
| stdout `L1: 12 EUR => 1306 cent` | 23 | 24 |
| event `println:ok:21` | 13 | 22 |
| stdout `L3: 8 EUR => 968 cent` | 21 | 22 |
| event `println:ok:21` | 13 | 22 |
| stdout `L4: 5 EUR => 605 cent` | 21 | 22 |
| event `println:ok:49` | 13 | 22 |
| stdout `Registros: 3; total: 28 EUR y 79 cent; media: 959` | 49 | 50 |

Cada evento suma los ocho bytes ASCII de `mock-io:` y un LF; stdout añade solo LF. Las proyecciones events/stdout preservan el orden de esta tabla. Outcome `{"kind":"ok"}`, diagnostics=[], exit 0, observation_bytes=320 (202 de eventos y 118 de stdout), reserved_observation_bytes=0. No se fija un total de ticks de esta aplicación: cada cargo se rige por A2/J/F5 y §8, sin calibrarlo a la ejecución. Sí se fijan los bytes y las trazas completos; el lote debe caber en el perfil normal.

Las variantes siguientes conservan fuente y demás respuestas. Cada secuencia enumera todos los eventos; cada println exitoso va seguido de su stdout. Todos terminan sin reservas vivas ni diagnósticos de programa; Err tipado tiene exit 1, Ok exit 0.

| Variación | Eventos exactos | Stdout (lista de payloads, LF añadido a cada uno) | Resultado; bytes |
|---|---|---|---|
| Cambiar 8 por 18 | `read_line:ok:2`, `read_line:ok:0`, `read_line:ok:2`, `read_line:ok:1`, `read_line:eof`, `println:ok:23`, `println:ok:23`, `println:ok:21`, `println:ok:50` | `["L1: 12 EUR => 1306 cent","L3: 18 EUR => 1960 cent","L4: 5 EUR => 605 cent","Registros: 3; total: 38 EUR y 71 cent; media: 1290"]` | Ok; 323 |
| EOF como primera respuesta | `read_line:eof`, `println:ok:45` | `["Registros: 0; total: 0 EUR y 0 cent; media: 0"]` | Ok; 90 |
| fin como primera línea | `read_line:ok:3`, `println:ok:45` | `["Registros: 0; total: 0 EUR y 0 cent; media: 0"]` | Ok; 91 |
| x como primera línea | `read_line:ok:1` | `[]` | Err(-9101); 23 |
| Primera lectura Err(70) | `read_line:err:70` | `[]` | Err(70); 25 |
| Cambiar 8 por -8 | `read_line:ok:2`, `read_line:ok:0`, `read_line:ok:2` | `[]` | Err(42); 69 |
| 10001 como primera línea | `read_line:ok:5` | `[]` | Err(42); 23 |
| 9223372036854775808 como primera línea | `read_line:ok:19` | `[]` | Err(-9102); 24 |
| Primera escritura Err(71) | `read_line:ok:2`, `read_line:ok:0`, `read_line:ok:1`, `read_line:ok:1`, `read_line:eof`, `println:err:71` | `[]` | Err(71); 137 |
| Segunda escritura Err(71) | `read_line:ok:2`, `read_line:ok:0`, `read_line:ok:1`, `read_line:ok:1`, `read_line:eof`, `println:ok:23`, `println:err:71` | `["L1: 12 EUR => 1306 cent"]` | Err(71); 183 |
| Cola de lectura vacía | `read_line:err:-9000` | `[]` | Err(-9000); 28 |

La variante 18 calcula 1620+340=1960 y total 3871, media 1290. Un fallo de parseo/dominio impide toda salida porque la impresión sigue al lote; un fallo de escritura no deshace lecturas ni salidas previas. La primera escritura fallida ocurre después de las cinco lecturas del entorno normal.

### 11.1. Ocho programas pequeños completos

Cada bloque es independiente, con entorno vacío salvo A08. Todos deben pasar check y run con diagnostics=[]; AssertEq erróneo sería R0201 y no un rechazo estático.

A01, bytes/escalares y copia:

```nexo
fn Main(){let a="é🙂";var b=a;b="x";AssertEq(a.ByteLen(),6);AssertEq(a.ScalarLen(),2);AssertEq(b,"x");}
```

A02, escalar/iteración explícita y ausencia de normalización (e seguido de U+0301 en el segundo literal):

```nexo
fn Main(){let a="é".Scalars();let b="é".Scalars();AssertEq(a,[233]);AssertEq(b,[101,769]);var n=0;for code in b{n+=1;}Print(n);}
```

Salida `2\n`, dos bytes, sin eventos.

A03, extremos de parseo y conversión:

```nexo
use std.parse;use std.text;
fn Main()->Result<(),I64>{let n=parse.I64("-9223372036854775808")?;AssertEq(text.FromI64(n),"-9223372036854775808");AssertEq(parse.I64("+1"),Err(-9101));return Ok(());}
```

A04, copias anidadas de nominales:

```nexo
@derive(PartialEq)struct S{x:Array<String>;}
fn Main(){let a=S(["a"]);var b=a;Push(edit b.x,"b");AssertEq(a.x,["a"]);AssertEq(b.x,["a","b"]);AssertEq(a!=b,true);AssertEq(Pop(edit b.x),Some("b"));AssertEq(a==b,true);let c=JoinArrays(a.x,["é"]);var n=0;for text in c{n+=text.ByteLen();}AssertEq(n,3);}
```

A05, enum con recurso y limpieza solo activa:

```nexo
enum E{Nada,Vivo(MockResource)}
fn Main(){let e=E.Vivo(MockOpen(7));match move e{E.Nada=>{},E.Vivo(move r)=>{Inspect(r);}}}
```

Eventos exactos `open:0:7`, `inspect:0:7`, `close:0:7`; sin stdout. La variante Nada no cierra un recurso inexistente.

A06, evaluación de etiquetas y orden de drop:

```nexo
struct Par{a:MockResource;b:MockResource;}
fn Main(){let p=Par(b:MockOpen(2),a:MockOpen(1));Drop(p);}
```

Eventos `open:0:2`, `open:1:1`, `close:0:2`, `close:1:1`. Construcción escrita b,a; limpieza de objeto completo b,a por orden inverso de declaración. El negativo dinámico pareado sustituye el segundo argumento por una función `fn Fallar()->MockResource{Fail();return MockOpen(1);}` y llamada Fallar(): solo open:0:2 y close:0:2, R0301. El retorno tras Fail conserva comprobación hipotética; no se omite su tipo.

A07, préstamos por Hole y efectos una vez:

```nexo
fn Siguiente(n:edit I64)->I64{n=n+1;return n;}
fn Main(){var n=0;Print("{{{n}:{Siguiente(edit n)}:{n}}}");AssertEq(n,1);}
```

Salida exacta `{0:1:1}\n`, ocho bytes. El primer préstamo de n ya terminó antes del segundo Hole. Un préstamo que escape por una closure dentro de la expresión sigue las obligaciones F0-0003, no se libera ficticiamente por terminar el formato.

A08, texto literal/interpolado anidado y orden:

```nexo
use std.console;
fn Main()->Result<(),I64>{let s="{"é"}:{if true then 1 else 2}";console.Println(s)?;return Ok(());}
```

Entorno de esquema 2 con una respuesta `{"ok":null}` en console_println y las otras cinco colas vacías. Stdout `é:1\n` (5 bytes); evento `println:ok:4`; outcome ok. El positivo alternativo cambia el Hole interior por literal `é` y conserva el contenido, pero no exige iguales cuotas de asignación: la cadena interior materializa un propietario adicional.

### 11.2. Límite de cien líneas

La misma fuente de 81 líneas recibe cien líneas `1`; una respuesta centinela Err(70) queda después de ellas. Entorno literal completo (el centinela no se consume):

```json
{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"console_read_line":[{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"ok":"1"},{"err":"70"}],"console_println":[{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null},{"ok":null}]}
```

Cada importe produce 100+21=121 céntimos, sin descuento. El lote suma 12.100 céntimos; cantidad=100, total/100=121, total%100=0, media=121. No hay lectura 101 ni evento EOF. Stdout literal completo, UTF-8 con un LF por línea y LF final:

```text
L1: 1 EUR => 121 cent
L2: 1 EUR => 121 cent
L3: 1 EUR => 121 cent
L4: 1 EUR => 121 cent
L5: 1 EUR => 121 cent
L6: 1 EUR => 121 cent
L7: 1 EUR => 121 cent
L8: 1 EUR => 121 cent
L9: 1 EUR => 121 cent
L10: 1 EUR => 121 cent
L11: 1 EUR => 121 cent
L12: 1 EUR => 121 cent
L13: 1 EUR => 121 cent
L14: 1 EUR => 121 cent
L15: 1 EUR => 121 cent
L16: 1 EUR => 121 cent
L17: 1 EUR => 121 cent
L18: 1 EUR => 121 cent
L19: 1 EUR => 121 cent
L20: 1 EUR => 121 cent
L21: 1 EUR => 121 cent
L22: 1 EUR => 121 cent
L23: 1 EUR => 121 cent
L24: 1 EUR => 121 cent
L25: 1 EUR => 121 cent
L26: 1 EUR => 121 cent
L27: 1 EUR => 121 cent
L28: 1 EUR => 121 cent
L29: 1 EUR => 121 cent
L30: 1 EUR => 121 cent
L31: 1 EUR => 121 cent
L32: 1 EUR => 121 cent
L33: 1 EUR => 121 cent
L34: 1 EUR => 121 cent
L35: 1 EUR => 121 cent
L36: 1 EUR => 121 cent
L37: 1 EUR => 121 cent
L38: 1 EUR => 121 cent
L39: 1 EUR => 121 cent
L40: 1 EUR => 121 cent
L41: 1 EUR => 121 cent
L42: 1 EUR => 121 cent
L43: 1 EUR => 121 cent
L44: 1 EUR => 121 cent
L45: 1 EUR => 121 cent
L46: 1 EUR => 121 cent
L47: 1 EUR => 121 cent
L48: 1 EUR => 121 cent
L49: 1 EUR => 121 cent
L50: 1 EUR => 121 cent
L51: 1 EUR => 121 cent
L52: 1 EUR => 121 cent
L53: 1 EUR => 121 cent
L54: 1 EUR => 121 cent
L55: 1 EUR => 121 cent
L56: 1 EUR => 121 cent
L57: 1 EUR => 121 cent
L58: 1 EUR => 121 cent
L59: 1 EUR => 121 cent
L60: 1 EUR => 121 cent
L61: 1 EUR => 121 cent
L62: 1 EUR => 121 cent
L63: 1 EUR => 121 cent
L64: 1 EUR => 121 cent
L65: 1 EUR => 121 cent
L66: 1 EUR => 121 cent
L67: 1 EUR => 121 cent
L68: 1 EUR => 121 cent
L69: 1 EUR => 121 cent
L70: 1 EUR => 121 cent
L71: 1 EUR => 121 cent
L72: 1 EUR => 121 cent
L73: 1 EUR => 121 cent
L74: 1 EUR => 121 cent
L75: 1 EUR => 121 cent
L76: 1 EUR => 121 cent
L77: 1 EUR => 121 cent
L78: 1 EUR => 121 cent
L79: 1 EUR => 121 cent
L80: 1 EUR => 121 cent
L81: 1 EUR => 121 cent
L82: 1 EUR => 121 cent
L83: 1 EUR => 121 cent
L84: 1 EUR => 121 cent
L85: 1 EUR => 121 cent
L86: 1 EUR => 121 cent
L87: 1 EUR => 121 cent
L88: 1 EUR => 121 cent
L89: 1 EUR => 121 cent
L90: 1 EUR => 121 cent
L91: 1 EUR => 121 cent
L92: 1 EUR => 121 cent
L93: 1 EUR => 121 cent
L94: 1 EUR => 121 cent
L95: 1 EUR => 121 cent
L96: 1 EUR => 121 cent
L97: 1 EUR => 121 cent
L98: 1 EUR => 121 cent
L99: 1 EUR => 121 cent
L100: 1 EUR => 121 cent
Registros: 100; total: 121 EUR y 0 cent; media: 121
```

Proyección literal events, sin LF almacenado dentro de cada evento:

```json
[
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "read_line:ok:1",
  "println:ok:21",
  "println:ok:21",
  "println:ok:21",
  "println:ok:21",
  "println:ok:21",
  "println:ok:21",
  "println:ok:21",
  "println:ok:21",
  "println:ok:21",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:22",
  "println:ok:23",
  "println:ok:51"
]
```

El orden global es cien eventos de lectura y luego, para cada uno de los 101 println, su evento seguido por su stdout correspondiente. Los primeros nueve stdout tienen 21 bytes de payload; los siguientes noventa, 22; L100 tiene 23; el resumen, 51. Con LF: 9×22+90×23+24+52=2.344 bytes. Cada read_line:ok:1 cuesta 23 y cada println de esta lista cuesta 22 (prefijo mock-io: y LF incluidos): 100×23+101×22=4.522 bytes de eventos. Total observation_bytes=6.866, reserved_observation_bytes=0, 201 eventos y 101 salidas. Resultado literal {"kind":"ok"}, diagnostics=[], exit 0. El entorno conserva sin consumir {"err":"70"}; console_println queda vacío. Se exige caber en todas las cuotas normales, sin fijar ticks por calibración del intérprete. Cierre en K10 y repetición independiente en K12.

## 12. Subtareas y aceptación

Todas las filas requieren los prerrequisitos de §1.1. Las dependencias K citadas son anteriores; el cierre de cada fila usa sus propios pares, efectos, cuotas e IR, nunca una promesa de K posterior. Las pruebas que solo construyen descriptores internos se identifican como tales, no como ejecución de sintaxis aún pendiente.

| Paso | Dependencias | Alcance y prueba local necesaria |
|---|---|---|
| K01 | J01–J16 y F5-L01–F5-L12 completos | Integrar catálogo/esquema/perfil/manifiesto y barreras; modelo único de descriptores preparado. Tests de objetos diagnósticos, tablas cerradas y rechazo temporal; no ejecución de capacidades pendientes. |
| K02 | K01 | Literal String sin Holes, valor/copia/limpieza, IR, UTF-8 y nuevas cuotas de texto. Print/igualdad de String builtin y ByteLen. X04, X23–X26, A01 adaptado sin ScalarLen; fronteras 8/16/2 y normales. No se acepta interpolación parcial. |
| K03 | K02, F5-L01, F5-L06–F5-L08 | Resto de operaciones de §3 y firmas calificadas std.parse/std.text por registro nominal (sin sintaxis use todavía), cuotas y efectos por byte. X22 y pares §10.3 con rutas completas std, escalares, límites y fallback de E/S inexistente. Seis comparadores String y extensión Array/for de §3.2, con pares O y costes de copia/crecimiento. |
| K04 | K03, F5-L03, F5-L06–F5-L08 | Registro de struct/enum de datos, DAG, constructors, campos y actualización; IR y propiedades de datos. X05–X10/X21/X27, copias anidadas sin derive, orden de etiquetas mediante funciones que imprimen I64; Array/for struct y enum, Push/Pop/JoinArrays sin exigir derive. |
| K05 | K04 | Campos/payloads con recursos, limpieza parcial, traslados, propagación transitiva y préstamos de campos; cuotas en todas las salidas. X11/M04, A06 y variante fallida; regresiones de exclusión de recursos transitivos en Array y transporte en Option/Result. |
| K06 | K05 | Extend local, activación `use extension package.T`, self y métodos; única llamada/Place. M01–M03, ejemplo §2, exclusividad por raíz y método consumidor. Esta activación concreta se implementa aquí; imports ordinarios esperan K09. |
| K07 | K05, J03–J05 | Incorporar descriptores nominales al match de F0-0003. A05; cobertura, guardas fallidas sin transferencias, payloads anidados, préstamo/edit/move y limpieza; ningún algoritmo nuevo de selección. |
| K08 | K05, F5-L01 | Format/PartialEq derivados e interpolación recursiva, con efectos y costes. X01–X03/X12–X16, A04/A07 y A08 con std.console sustituido por Print; round-trip y expresiones interiores no duplicadas; ==/!= nominal y Array/Option/Result transitivos, NX-E1403 y prioridad de tipos distintos. |
| K09 | K06, K08 | pub, firmas/exportaciones, imports ordinarios del registro raíz/std, alias y pub use. X17–X20/M06, colisiones, tipos privados; A03 original. No carga de archivos extra. |
| K10 | K06, K07, K09 | Entorno esquema 2 y consola simulada, contabilidad transaccional de respuestas/observaciones. A08 y app completa; errores tipados y cuotas sin consumir cola en fallo. Migración íntegra de fixtures JSON previas, incluido F0-0003 §11 según §13. |
| K11 | K10 | Robustez integrada: escala, pilas pequeñas, mutaciones de IR, fuentes y entradas; cierres de todas las fronteras y conservación de ticks del subconjunto anterior. |
| K12 | K11 | Contraste independiente de check/run, modelo diferencial, ocho programas y aplicación/variantes; matriz de aceptación con evidencia por regla. No declara fase aprobada. |

K04 tiene tres entregables internos consecutivos: registro/DAG/asociación de constructores; Places/copia/actualización; Array dinámico y for nominal sin derive. K08 tiene tres entregables internos consecutivos: planes Format/PartialEq; igualdad ==/!= nominal y transitiva en colecciones; parser recursivo/BuildString. K03 tiene tres entregables internos consecutivos: operaciones de texto/parseo; != y cuatro órdenes String, conservando == de K02; Array dinámico/for String. Son subtareas acotadas con evidencias separadas; cada una añade sus cargos al mismo motor y la fila K no cierra hasta completarlas todas. Cada entregable exige sus pares y cuotas; no crea otro pending_scope ni publica una capacidad parcial. K07 requiere datos y propiedad de K05, no métodos. K08 requiere esos datos y propiedad, no selección nominal de K07; puede usar match intrínseco terminado en F0-0003. K10 reúne explícitamente las ramas de métodos, selección e imports.

Desglose de tamaño y dependencias de las extensiones: los identificadores con punto designan subtareas documentales dentro de la K, no valores serializados. Su pending_scope es siempre el Knn padre; no se emiten K03.1 ni prefijos F0-0004-K03. Cada fila es una entrega verificable, no una fracción aprobable sin efectos/cuotas.

| Subtarea | Depende de | Unidad y evidencia |
|---|---|---|
| K03.1 | K02 | Biblioteca String y parseo; firmas, cargos por byte y pares §10.3. |
| K03.2 | K03.1, F5-L01 | Completar los seis comparadores en el dispatcher común, sin cambiar ==; O01–O04 y orden de evaluación. |
| K03.3 | K03.2, F5-L06–F5-L08 | Array/for String, copia profunda y cuotas; O10/O11/O13 y fronteras storage/allocation/text. |
| K04.1 | K03 | Registro/DAG/asociación; X05–X09/X21/X27. |
| K04.2 | K04.1, F5-L03 | Proyección/copia/actualización, incluidas compuestas I64; X10 y aislamiento por raíz. |
| K04.3 | K04.2, F5-L06–F5-L08 | Array/for struct y enum sin derive; programa K04 de §10.7 y O12. |
| K08.1 | K05 | Planes Format/PartialEq, capacidades por todos los componentes y diagnósticos de derive. |
| K08.2 | K08.1, F5-L01 | ==/!= nominal y envoltorios, sin orden nominal; O05–O08/O14–O16. |
| K08.3 | K08.2 | Interpolación y parser recursivo compartido; X01–X03 y H01–H08, efecto/coste y limpieza por Hole. |

K03, K04 y K08 cierran únicamente después de sus tres subtareas. Las demás K conservan una unidad principal: K01 contrato, K02 String base, K05 recursos, K06 métodos, K07 selección, K09 exportaciones/imports, K10 consola, K11 robustez y K12 revisión. Todos los J y F5 citados preceden a K01; no hay dependencia hacia un corte posterior ni de una sub-K hacia una K futura.

Asignación obligatoria adicional: A01 completo (incluido ScalarLen) y A02 → K03; M05 y O01–O04 → K03; O05–O08/O14–O16 → K08; O12 → K04; O09 → K05; O10/O11/O13 → K03; M07 → K04; M08 → K08; N01/N02 → K07; N03/N04/N12 → K04; N05–N08 → K05; N09 → K02; N10/N11 → K08; N13 → K01 (barrera), repetido en K02 al retirarla. H01–H08 → K08; H09 → K04; H10 → K09; H11 → K07; H12/H13 y H12-edit/H13-move → K06; E01–E06 → K10. Las pruebas de barrera de K01 son diagnósticas; las parejas completas se ejecutan al cerrar la K que aporta la capacidad.

### 12.1. Matriz de criterios medibles

| Criterio | Evidencia necesaria |
|---|---|
| C01 superficie | Todos los literales de §10 alcanzan su regla, sin errores previos; parse–print–parse conserva AST normalizado y print es idempotente; comillas/llaves anidadas y offsets multibyte correctos. |
| C02 texto | Oráculo independiente de UTF-8 para bytes, escalares, orden y fronteras; todos los pares §10.3; ninguna normalización ni uso de columna como offset. |
| C03 nominalidad | Struct/enum distintos no unifican; propiedades transitivas para todo campo/alternativa; constructores asocian correctamente sin reordenar evaluación. |
| C04 propiedad | Cero copias de recurso; cada adquisición comprometida tiene exactamente un cierre, salvo transferencia a propietario final todavía vivo; disponibilidad, reemplazo y limpieza parcial en todas las salidas. |
| C05 efectos | Literal/copia/Concat/interpolación rechazan no_alloc; Print/ReadLine/Println rechazan no_io; destructores transitivos y expresiones no ejecutadas siguen en resúmenes. Desconocido no equivale a vacío. |
| C06 selección/métodos | Mismos TypeId y reglas de propiedad en vivo/hipotético, llamada libre/método y builtin/enum; guardas fallidas no mueven; modo insuficiente e import ausente diagnosticados por causa. |
| C07 IR | Verificador rechaza cada corrupción de §7; no existe acceso público que ejecute AST o IR sin verificar; cleanup y costes inspeccionables. |
| C08 simulación | Ninguna operación de programa abre stdin/archivo/red; colas y observaciones idénticas entre text/json; agotamiento no consume respuesta ni emite prefijo parcial. |
| C09 límites | L-1/L/L+1 de cuotas nuevas, perfiles normales/reducidos, continuidad de A2; cero panics, stack overflow o aceptación tras análisis incompleto. |
| C10 producto | App y ocho programas con resultados exactos, variantes de errores y entradas distintas; sin detección por texto fuente o nombre de fixture. |
| C11 conservación | Regresiones de A2, F0-0003 y F0-0005 con el mismo significado, nuevos campos y perfiles explícitos; base inmutable y pendientes nominales visibles. |

Cada regla con rechazo exige un negativo aislado, su positivo, campos completos y una mutación inversa. Las tablas §10 son el mínimo literal; antes de cerrar cada K se añaden pares aislados para todas las reglas de su fila, no se considera suficiente cubrir un código una vez. Se incluyen en particular dos préstamos edit de campos distintos, lectura tras move completo, escritura por self:view, igualdad de nominal sin derive, capacidad ausente anidada, enum con variante recurso inactiva, ciclo de tipos a través de Result, y publicación de tipo privado en un payload público. Los esperados se derivan de las reglas y spans de fuente, nunca se rellenan ejecutando el compilador bajo prueba.

### 12.2. Pruebas independientes y adversariales

El modelo diferencial usa strings de bytes/escalares y productos/sumas con identidades explícitas, independiente de las tablas/funciones del compilador. Para enteros usa aritmética arbitraria y valida gramática antes del rango; para UTF-8 usa un decodificador de referencia separado. No llama al intérprete para generar esperados. Genera casos deterministas con semillas publicadas, tamaños 0/1/2/7/8/15/16/31/32 y fronteras normales, combinando ASCII, U+0000, U+007F, U+0080, U+07FF, U+0800, U+D7FF, U+E000, U+FFFF y U+10FFFF. Inválidos: continuation aislada, sobrelonga, surrogate, truncamiento y fuera de U+10FFFF en entrada/controlador y verificadores internos. String nunca puede contenerlos en un programa ya verificado.

Mutaciones de implementación por regla: omitir frontera UTF-8; tratar ScalarLen como ByteLen; normalizar; convertir parse overflow en wrap; reordenar argumentos etiquetados; aceptar copia si solo variante activa es copiable; olvidar cierre de campo; prestar después de evaluar argumentos; conservar préstamos de Hole demasiado tiempo o liberarlos antes del uso; omitir efecto de Hole/copia; ignorar derive; aceptar privado exportado; ejecutar IR sin máscara; reiniciar contador en función/método; omitir cargo por byte. Cada mutación debe hacer fallar su regresión prevista, se restaura el contenido y se verifica SHA-256 antes de la siguiente. Reejecutar supervivientes previas y como máximo doce mutaciones nuevas por ronda; solo bloquean reglas explícitas de este ADR, no contenido inventado del roadmap.

Fuzz de lexer/parser/impresor/checker/serialización con fuentes truncadas en cada byte de los literales y con profundidad 127/128/129; incluir llaves, escapes y cadenas anidadas, CRLF/CR/LF, EOF y caracteres multibyte. Nunca se cuenta un fallo sintáctico accidental como negativo de propiedad. Los recorridos de types/format/drop se prueban también con IR/descriptores válidos construidos internamente, para alcanzar profundidad sin depender del parser.

Escala determinista: cadenas de DAG nominales hasta la profundidad admitida, grafos en diamante para detectar expansión exponencial, enums de muchas alternativas, métodos recursivos con frames explícitos y muchas interpolaciones consecutivas. Pilas de hilo de prueba de 256 KiB y 512 KiB, como J/F5 para análisis/destrucción/ejecución, debug y release; la limpieza se prueba tras agotar cada cuota con recursos anidados aún vivos. No se exige sobrevivir a una pila arbitrariamente pequeña de infraestructura. Los resultados observables y diagnósticos son iguales entre tamaños admitidos; se registra un bloqueo de infraestructura por separado, nunca como conformidad.

El revisor independiente reconstruye el HEAD evaluado desde `git archive HEAD` y compara su propia batería check/run con ese árbol; no reutiliza solo los tests del implementador ni los artefactos previos. Cada fallo corregido añade regresión, resultados íntegros y restauración comprobada; nombres de tests describen regla, no número de rondas. Se ejecutan los checks requeridos por el repositorio y la suite offline/locked en debug/release. Para cada criterio se registra PASA/FALLA/NO EVALUADO/INCONCLUSO con fuente, revisión, comando y observación. Demos y recuentos no sustituyen evidencia adversarial.

## 13. Enmiendas propuestas y obligaciones restantes

Las citas siguientes identifican exactamente qué se concreta o modifica, sin editar las fuentes de autoridad.

| Fuente y cita | Decisión propuesta |
|---|---|
| D10, Texto: «Las fronteras de slicing se comprueban.» | SliceBytes con intervalo semiabierto en bytes, prioridad rango antes de frontera, resultado propietario y Err(-9110/-9111). No añade vista escapable ni indexación entera. |
| D10, Texto: «Interpolación desde F1, con evaluación izquierda→derecha una sola vez y requisitos `Format` comprobados.» | Exploración F0 permitida por la fase; Format cerrado nominal, efecto alloc del constructor, préstamo por Hole y errores/retornos sin rollback. No acredita entrega F1. |
| D15 §2.4: «Las comillas y escapes de interpolaciones se analizan recursivamente en su contexto.» | Parser de contextos compartidos, spans originales y nesting común; CR/LF crudos prohibidos en todo el literal, incluidos Holes. La envoltura opaca del perfil F0 deja de aplicarse al habilitar texto. |
| D15 §2.4: «no admite sentencias ni campos de formato» | No hay sentencias sueltas ni especificadores en Hole; los bloques ya contenidos en una Expr if/match siguen su gramática y control normales. Evita prohibir de forma contradictoria una Expr admitida por `{Expr}`. |
| F0-0003 §8.2, NX-E1107: «required_mode y requested_mode: read/inout/consume; declaration sobre binding receptor» | Enmienda propuesta: la relación declaration se exige cuando hay raíz Place; para el receptor temporal de método, §5.1 fija primario de expresión, sin relación ficticia y con los mismos dos campos de data. H12/H13 fijan el caso temporal sin prefijo; H12-edit/H13-move conservan NX-E0002 de D15/A2 y no amplían NX-E1107. |
| D13 §10: «Las extensiones se activan únicamente mediante `use extension paquete.modulo.Tipo;`» | En módulo raíz, `use extension package.T;` activa el extend local; sin activación NX-E1315. No se introduce activación por estar en el mismo archivo. |
| D13 §10: «self es el receptor prestado del tipo extendido, como en D08.» | Se lee junto a D15 §3.5 «`self: view/edit/move` toma tipo del trait/impl/extend»: self:move es propietario; view/edit prestados. D15 prevalece sobre una interpretación que prohibiese todo receptor consumidor. |
| D13 §9: «Una firma pública exige tipos de parámetros/resultado» | También se escribe unidad; missing_signature_type al final de parámetros. Los privados mantienen unidad omitida de A2, sin habilitar inferencia de resultado no unitario. |
| D08: «Un conjunto fijo de símbolos y precedencias se resuelve mediante traits»; F1: «derivación `Format`/`PartialEq`» | Instancias cerradas nominales y planes estructurales en el prototipo, siguiendo la simplificación explícita A2 §4.8. No se afirma resolver traits/genéricos generales ni derivación comptime. |
| F0-0003 §10, «Concreciones y enmiendas propuestas», fila A2 §5.4: «Esta enmienda es propiedad de F0-0003.» | Referencia normativa a esa enmienda, sin reproducirla ni sustituirla aquí. Los campos propietarios de §4 conservan las procedencias del motor común; no independizan portadores de préstamos. |
| A2 §6.4: «Las alternativas de `Result` solo destruyen el payload activo.» | Se generaliza a enums nominales; agregado completo destruye en orden inverso de declaración, constructor parcial en orden inverso de evaluación. |
| D15 §3.6: «`implementation_limit` usa exclusivamente `I08`, `I09`, `I10a`, `I10b` o `I11a`» | Se añaden los nueve valores K cerrados de §9.1, se retiran sus emisiones al cerrar cada capacidad y se conserva historia sin compatibilidad de ejecución. |
| F0-0003 §11, entorno de la aplicación: `{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null},{"ok":null},{"ok":null}],"mock_request":[{"ok":"3"}],"mock_reply":[]}` | Enmienda propuesta para K10: sustituir únicamente schema_version por 2 y añadir `"console_read_line":[],"console_println":[]`; conservar las cuatro colas, sus respuestas y observaciones esperadas. Migrar también sus variantes de entorno. Antes de K10 su esquema 1 sigue siendo el de F0-0003; después se rechaza con invalid_mock_schema, sin lector compatible. |
| A2 §12.4: «Los cinco campos son obligatorios» | Entorno esquema 2 con siete claves y seis colas; migración completa de fixtures, sin lector v1 alternativo. El objeto de ejecución schema_version=1 es otro protocolo. |
| A2 §13.17, «Componentes no resueltos y dominios de tipos»: «Los únicos dominios adicionales permitidos en `expected_type` son:» y la lista «domain:copyable_data», «domain:array», «domain:mock_resource», «domain:result» | Enmienda propuesta: añadir domain:ordered_scalar para el dominio I64/String de los cuatro operadores de orden (§3). Conservar los dominios añadidos por J/F5, prioridad de tipos distintos y found_type concreto. No es un tipo fuente. Se propone derogar para el perfil K la frase de F0-0005, «Registro propuesto»: «No se añade domain:ordered_scalar.»; en F0-0005 aislado permanece vigente. |
| A2, «Vocabularios cerrados, ubicaciones y cadenas», NX-E0111/non_callable_target: «`declaration`: nombre declarado del binding que ocultó o sustituyó al destino invocable.» | Enmienda propuesta: exigir esa relación cuando exista el binding ocultante o sustituto; una variante nula usada como destino de llamada, como P05, no tiene tal binding y usa related_spans=[]. Conserva types/call.bind de F0-0003, «Invocación y efectos». |
| A2 §13.15, Diagnostic.operation: «El vocabulario cerrado es la unión de los literales de la tabla de §13.12 y los siguientes cinco valores de efectos:» | Enmienda propuesta: añadir value.build_string para literal/interpolación que aporta alloc; mantener la unión y ampliaciones J/F5. N09 fija el primario y contrato. |
| A2 §13.15, Cause.operation: «Enumeración cerrada y correspondencia:» | Enmienda propuesta: añadir value.build_string con kind=effect, entity de tipo String y evidence=verified; span de literal/interpolación que aporta alloc. No añadir operaciones de comprobación como pasos causales. |
| A2 §13.6, declaration_kind: «`binding`, `function` o `parameter`; obligatorio para NX-E0104.» | Enmienda propuesta: añadir type, field y variant exclusivamente a declaration_kind, conservando binding/function/parameter y constant añadido por F5; obligatorio para el mismo código (§9, X06/X07). |
| A2 §13.6, use_kind: «`value` o `call_target`; obligatorio para NX-E0101.» | Enmienda propuesta: añadir type para ruta nominal ausente en posición de tipo. Conservar call_target de constructores/patrones y value; tipos reconocidos diferidos mantienen unsupported_type. |
| A2 §13.6, runtime_budget_kind: «`call_frames`, `live_value_cells` u `observation_bytes`; obligatorio para R0002.» | Enmienda propuesta: añadir string_bytes, live_text_bytes y text_allocations solo para R0002, además de array_storage_bytes/array_allocation_bytes heredados de F5. Limit/observed siguen obligatorios; no se añade profile_id a data. |
| A2 §13.6, budget_kind: «String de la enumeración cerrada del catálogo; obligatorio para NX-E0003 y prohibido para R0002.» | Enmienda propuesta: añadir nominal_members a esa enumeración para NX-E0003; budget_scope=compilation, incomplete_analysis=types y operation=analysis.limit. Conservar los presupuestos J/F5, limit/observed y el perfil en Diagnostic.profile_id. |
| A2 §13.6: «`data` admite únicamente las claves registradas más abajo.» | Enmienda propuesta: añadir member_name y receiver_type, String obligatorios exclusivamente para NX-E1402/unknown_member; required_capability, enumeración Format/PartialEq obligatoria exclusivamente para NX-E1403/missing_format o missing_partial_eq respectivamente. NX-E1401/1404 tienen data={}; no admiten estas claves. Mantener las claves de patrones/modos añadidas por J y las de F5 solo en sus emisiones. Las formas exactas son §9 y tablas §10. |
| A2 §11.2: «Una celda es un escalar, unidad, recurso, cabecera de array o envoltorio de `Result`.» | Se añaden cabeceras String/nominales; bytes y asignaciones lógicas de texto tienen cuotas separadas y no evaden trabajo/observaciones. |

La propuesta decide el subconjunto; no deja a implementación elegir entre vista/copia, índices/caracteres, coerciones/nominalidad o dos motores de tipos. La interfaz de F0-0003 se integra conforme a §1.1; los cambios posteriores de su contrato reabren las pruebas afectadas. Las decisiones propuestas no se presentan como evidencia de implementación ni aprobación de fase.

Pendientes conservados: dos revisores sin discrepancias y aprobación del propietario para F0; F1 completo y sus mínimos diferenciados; D08 general; D09 CoW/publicación; Bytes y conversión UTF-8 fallable desde bytes del programa; grafemas y edición de biblioteca Unicode; vistas persistentes; recursos en Array; nominales recursivos; iteración perezosa de texto; módulos/paquetes fuente; E/S real; derivación comptime; pruebas completas T01–T28 y plataformas. Este corte aporta evidencia parcial a T01/T02/T07/T08/T14/T17/T20/T21/T22/T24/T25/T26; no cierra T03–T06/T09–T13/T15–T16/T18–T19/T23/T27–T28 por ejecutar la aplicación. EX01 completo requiere biblioteca real/F2; EX02, EX03 y EX04 mantienen sus fases y obligaciones.
