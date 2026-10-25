# F0-D17 — Autoalojamiento, cadena de arranque y backend propio

Estado: **decisiones de arquitectura revisadas de forma independiente; sin implementación ni aprobación de fase**.
Fecha: 2026-10-25.

## 1. Autoridad y decisiones de alcance

**R01.** Rigen [AGENTS.md](../../AGENTS.md), [F0_ACUERDOS](../F0_ACUERDOS.md), acuerdos
§§9.1–9.4 y 9.7–9.8, y [roadmap rev 0.2](../baseline/Nexo_ROADMAP_rev0.2.md),
P01–P09, D01–D10, roadmap §§4.2–4.3, F0–F7, roadmap §§6–8 y roadmap §11. La base es inmutable;
las sustituciones propuestas se enumeran en D17 §12, sin crear una rev 0.3.
Este ADR no acredita implementación, resultados ni aprobación de fase.
A2, J, F5 y K designan F0-0002, F0-0003, F0-0005 y F0-0004
([D16](F0-D16-capas-y-bibliotecas.md) R02).
0001–0005 designan, respectivamente, [F0-0001](F0-0001-first-slice.md),
[F0-0002](F0-0002-second-slice.md), [F0-0003](F0-0003-third-slice.md),
[F0-0004](F0-0004-text-and-user-types.md) y
[F0-0005](F0-0005-operators-control-collections.md);
I01–I11 designan las subtareas de F0-0002 §17.
J01–J16, F5-L01–F5-L12 y K01–K12 designan las subtareas de
F0-0003 §12, F0-0005 §10 y F0-0004 §12, respectivamente.
F5 designa ese documento cuando acompaña a §, a un código F5- o a las
siglas A2/J/K; en el calendario F0–F7 designa la fase del roadmap.
«B» sin dígitos designa la sintaxis oficial de
[F0-D15](F0-D15-sintaxis-oficial.md).
Las referencias R01–R60, B01–B56 y N01–N24 sin documento explícito pertenecen
a D17; N incluye las subtareas con sufijo de R55. M y V designan tareas y
casos de D16, respectivamente; C01–C10 e IR01–IR11 designan su catálogo e invariantes.
Las reglas R de otro documento llevan su autoridad explícita, también en
enumeraciones. B0/B1 son los subconjuntos de R14/R18, no casos B01–B56;
R0001/R0002 son códigos de diagnóstico de A2 y R05xx es la familia runtime
de F5, no reglas R de este ADR.
Las secciones § sin autoridad dentro de una cita literal pertenecen al
documento citado; fuera de citas, la autoridad acompaña a la sección.
En las tablas de enmiendas, la autoridad de la primera columna no se extiende
a la segunda: sus referencias siguen estas mismas reglas.
La superficie y sus contratos complementarios son [D11](F0-D11-closures.md),
[D12](F0-D12-async.md), [D13](F0-D13-expresiones-modulos.md),
[D14](F0-D14-sintaxis-superficie.md) y [D15](F0-D15-sintaxis-oficial.md).

**R02.** Se decide un compilador, runtime y herramientas oficiales escritos
en Nexo, construibles por una versión anterior de Nexo. Rust es la herramienta
de arranque transitoria. El backend final también se escribe en Nexo. Se
conserva LLVM durante la validación original y, posteriormente, como opción
transitoria de optimización hasta cerrar las puertas de sustitución de D17 §6.
No se promete que el prototipo F0 pueda compilarse a sí mismo.

**R03.** Son hitos diferentes:

| Hito | Condición | Dependencias que todavía admite |
|---|---|---|
| Frontend en Nexo | Equivalencia léxico/AST/formato de R16 | Intérprete y compilador Rust de referencia; E/S simulada en F0 |
| Autoalojamiento sin Rust | R20–R26 y retirada atómica R29 | LLVM, enlazador y receta externa de construcción fijados |
| Backend propio de desarrollo | R33–R38 | LLVM como oráculo externo y opción de optimización transitoria |
| Independencia de construcción final | R39 y R40 | Semilla Nexo anterior, sistema operativo, SDK/API nativas y herramientas de firma externas declaradas |

La última fila excluye Rust, LLVM, Cranelift, libc/CRT (salvo libSystem limitado por R40) y bibliotecas de otros
lenguajes como dependencias transitivas del compilador/runtime/biblioteca
oficial. El kernel, cargador y API nativa del sistema son frontera irreductible,
no una promesa de escribir el sistema operativo en Nexo. Las aplicaciones
que opten por perfiles C o Python declaran esas dependencias separadamente.

**R04.** [D16](F0-D16-capas-y-bibliotecas.md) decide qué pertenece a biblioteca y qué queda como intrínseca,
incluida la transición Array/List y el acceso a sistema. D17 consume esos
contratos: no asigna nombres nuevos a colecciones, no crea otro asignador,
no convierte las intrínsecas simuladas de F0 en biblioteca de producción.
La ausencia de un contrato D16 revisado bloquea N06, no autoriza un sustituto
Rust permanente. No hay sintaxis adicional en este ADR.
Se adoptan expresamente D16 R06–R07 (Array inline/List dinámica), D16 R18–R22
(catálogo cerrado), D16 R24–R27 (std y arranque), D16 R31–R33 (equivalencia y
costes de migración), D16 R35–R37 (ABI y recarga) e IR01–IR11. Los contratos de semilla y recarga están en este ADR; D16 los consume.
D17 §12 identifica las enmiendas a autoridades previas, sin duplicar definiciones.

La tabla siguiente es el único mapa normativo M↔N; D16 R53/D16 §12.2 remiten
a él. Implementación y consumo no intercambian propietarios.

| M ↔ N | Único propietario de capacidad y consumidor |
|---|---|
| M01/M02/M03/M05 ↔ N01/N02 | M01 inventaría catálogo; N01 concilia diseño. N02 implementa tabla de funciones, DefKey/body_hash/interface_hash/dependency_hash/clave de artefacto IR01/IR02, layouts y ABI; depende solo de N01 y M01. M02/M03 integran y auditan consumo std después de N02; M05 implementa C01–C05/C07/C10 y lowering de memoria, consumiendo sus descriptores. |
| M04/M06/M07/M08/M09/M09a–c/M10/M11/M11a ↔ N06 | D16 implementa permisos/hooks, constructor std, Option/Result nominales (M08), contexto, operadores, mapas hash, búsqueda/internado/SHA-256 (M09b), ordenación (M09c), texto/E/S y cierre fallable (M11a). N06 integra B1 y verifica esas entregas, sin reimplementarlas; módulos/genéricos son F1/D08/D13. |
| M12 ↔ N06/N07c/N12 | D16 entrega S0-std y artefactos std; N06 registra S0-std, N07c consume M12 nativo y N12 usa std en la cadena. C0 pertenece a N11. |
| M13a–c ↔ N07c | D16 entrega CoW/generadores/interop; D17 integra evidencia E2E-F2. |
| M14a–d/M15 ↔ N15 | D16 entrega regiones/unicidad/layouts/persistencia, Entropy y manifiestos C/red de M15, consumiendo ForeignCall de M13c; N15 integra puertas F3–F6, sin duplicar biblioteca. |
| M16 ↔ N03–N13 | D17 implementa compilador/arranque y posee semillas/retirada; M16 solo remite. |
| M17 ↔ N14–N23 | D17 implementa backend/carga/recarga, incluidas N14/N14a/N14b/N21; D16 mantiene catálogo y biblioteca única. |
| Sin M implementador ↔ N24 | N24 posee el ADR futuro H3 y consume N21; no implementa biblioteca ni habilita migración de objetos por esta referencia. |

Los intervalos N03–N13 y N14–N23 incluyen todas sus subtareas con sufijo
de R55: N07a/N07b/N07c y N14a/N14b, respectivamente. N07 y M13/M14 no
son tareas adicionales. Las referencias agregadas M16/M17 no añaden aristas
al grafo de implementación.
N15 aparece en la fila M14/M15 como consumidor de sus entregas y en M17
solo por pertenecer al intervalo agregado N14–N23; no tiene dos dueños.

N07a posee ProcessRun; N14a el catálogo H2 definido en R45, incluida la
observación de sellos F_GET_SEALS de ImageSeal; N14b posee H2 LLVM y N21 su habilitación
propia, sobre un runtime Nexo único. Son ampliaciones propuestas a D16 «4. Catálogo cerrado de confianza», R22. M12 congela S0-std y N06 solo lo registra.

**R05.** Se mantienen dos fronteras distintas: fuente B → IR tipada verificada,
y esa IR → representación de máquina. Ningún backend acepta fuente por su
cuenta ni vuelve a inferir propiedad/efectos. El intérprete permanece como
modo oficial y oráculo semántico; tener intérprete y backend no significa
mantener dos definiciones de la semántica.

## 2. Contratos que deben quedar fijados antes de F2

**R06.** Se conserva F0-0002 §§9.1–9.4: CFG, slots y temporales tipados,
identidades resueltas, procedencias y représtamos, efectos conocidos/desconocidos,
flags de inicialización/transferencia, planes de limpieza y mapas fuente.
Se verifica antes de ejecutar y después de cada transformación que pueda
afectar esos contratos. Un agotamiento de análisis nunca produce un artefacto
verificado. Los volcados de IR no son entradas ejecutables.

**R07.** La interfaz lógica de backend recibe exclusivamente un artefacto
verificado con estos campos obligatorios: versión de esquema IR, versión
semántica, perfil de capacidades/cuotas, objetivo y características CPU,
versión ABI, funciones, tipos/layouts, intrínsecas resueltas y contratos,
planes de limpieza, mapas fuente e inventario de dependencias. Devuelve
objetos, mapas de depuración, informes de costes y manifiesto de importaciones.
Un backend declara una matriz cerrada de operaciones/tipos/destinos soportados.
Una operación ausente rechaza la construcción completa; no hay fallback por
función a Rust, a otro backend ni a interpretación oculta.

**R08.** La ABI interna inicial es monomórfica, específica del objetivo y
versionada; no es una ABI universal estable 1.0. Se deciden ya estas reglas:

1. Los parámetros `view`/`edit` pasan el descriptor de préstamo definido
   aquí: identidad/generación del dueño, ruta lógica o base/offset/rango del
   target, permiso, origen y duración, también si el tipo es escalar.
   Conserva los datos lógicos consumidos por D16 IR04. Su representación física se fija por
   objetivo; eliminar un descriptor requiere conservar su contrato completo.
   Los escalares propietarios se pasan según la convención nativa del objetivo.
   Agregados, entornos y estados se pasan por dirección a almacenamiento tipado;
   resultados agregados usan un destino de resultado explícito del llamador.
2. `view` transporta acceso de lectura y procedencia; `edit`, acceso exclusivo;
   `move`, propiedad. La dirección de un agregado movido no habilita al callee
   a conservar la dirección de la pila del llamador: debe instalar el valor
   en su destino propietario si escapa. Se desarma exactamente el origen.
3. Un resultado se compromete después de inicializarlo por completo. Los
   planes de salida distinguen resultado comprometido, argumentos preparados,
   campos inicializados y préstamos; `Err` es un valor etiquetado, no unwinding.
4. `Option`/`Result`/enum conservan tag y payload activo. La ABI inicial no usa
   nichos ni padding sin inicializar como datos observables. Tamaño, alineación,
   offsets y tag están en un descriptor de layout por objetivo.
5. Un contexto de asignación explícito, cuando una operación lo requiere,
   viaja como parámetro oculto declarado en su firma IR. Transporta exactamente
   AllocationContext de D16 R35; no define un segundo contexto ni proveedor. No se obtiene del
   nombre del programa, de un global mutable ni de un cambio de región implícito.
6. ABI, layout, modos, resumen de efectos de cada función (por operación
   invocable cuando corresponda D11/D12), procedencias, afinidad, versiones
   de runtime y CPU forman parte de la clave de artefacto (R10). Una diferencia
   exige reconstruir el cierre dependiente; no se enlazan módulos incompatibles.
7. La ABI C se aplica solo a exportaciones/importaciones declaradas. Sus
   wrappers Nexo conservan validación, propiedad, efectos y traducción de
   errores. No se expone la representación interna de genéricos por accidente.

N02 implementa los descriptores concretos para los cuatro destinos y sus
vectores al inicio de F1, con N01/M01 como únicas dependencias. M02/M05 los
consumen después; la ejecución nativa F2 verifica su conformidad. Es especificación de
representaciones elegidas, no una elección futura entre ABIs alternativas.

**R09.** Se definen las identidades persistentes DefKey que D16 IR01 consume, separadas de las
identidades de diagnóstico de F0-0002 §13. Sus `function:<offset>` y demás
anclas fuente siguen sirviendo para diagnósticos; no sirven como identidad
estable frente a inserciones de texto. La clave de una declaración superior
(función, struct/enum, constante, trait o alias de tipo) es
la tupla `(origen lógico de paquete, nombre de paquete, ruta de módulo,
nombre de declaración)`. No incluye revisión, ruta absoluta, offset ni cuerpo.
La revisión se añade a la identidad nominal instanciada; package_digest
registra por separado la integridad conforme R10/D13 §11, sin formar parte
de la identidad nominal ni de interface_hash. La identidad resuelta que
participa en esos registros es exclusivamente (origen, nombre, revisión). Renombrar o mover
una declaración cambia su clave; reexportarla no. Una instancia añade sus
argumentos de tipo/constantes canónicos y las identidades de impl resueltas.
En una sesión se prohíben dos declaraciones con la misma clave.
La ruta de módulo es una String ASCII: `package` para la raíz; para cada
submódulo se concatena punto y su Name canónico, desde la raíz hasta la
declaración, por ejemplo `package.net.http`. Los segmentos cumplen Name de
D15 y no contienen puntos; no se serializan alias de importación, rutas de
archivo, segmentos vacíos, self/super ni separadores del sistema. La tupla
DefKey se codifica como cuatro String separadas y con longitud según R10;
declaration_key de R47 es el array JSON de esas cuatro String. La identidad
de `package.a.bc` difiere de `package.ab.c` sin ambigüedad.
N02 implementa una tabla base indexada por DefKey/instancia; cada registro
contiene firma/ABI, cuerpo VerifiedIr, body_hash/interface_hash/dependency_hash de R10 y referencias a tipos/layouts
y dependencias. Lowering, intérprete y emisores consumen esa misma tabla.
La forma base no instala ediciones en vivo; N14 añade generaciones R45 sobre
ella sin una tabla paralela de funciones para recarga.
N02 implementa la representación y validación de DefKey tomando descriptores
explícitos de paquete/módulo/declaración como datos; sus vectores incluyen
orígenes, paquetes homónimos y módulos anidados sin ejecutar un resolvedor.
No implementa ni presupone módulos/paquetes fuente ya disponibles. El
resolvedor F1/D13 produce esos descriptores y N06 verifica su conexión con
la tabla N02, con F1/D13 como dependencia explícita. Hasta esa integración
no se acredita resolución de paquetes desde fuente; N02 sigue dependiendo
solo de N01/M01, sin deducir identidades de rutas ni inventar un paquete global.
La misma separación se aplica a todos los insumos de R10: interfaces D13,
impls resueltas y aristas de inlining/comptime/derivación se suministran como
registros explícitos en los vectores N02; N06 conecta los productores F1.
N02 prueba codificación, SCC e invalidación, no acredita esos productores.

Los métodos de impl/extend ocupan la misma tabla, con una DefKey de cuatro
String cuyo último componente es `$method:<clase>:<ordinal>:<nombre>`.
clase es impl o extend; ordinal es el índice desde cero del bloque entre
los bloques de esa misma clase en el módulo, en orden de declaración, decimal
sin ceros iniciales; nombre es el Name del método. Los otros tres componentes
identifican el paquete y módulo que declara el bloque, no el paquete del tipo
receptor ni el del trait. `$` y `:` no pertenecen a Name: estas claves no
colisionan con declaraciones superiores ni entre clases, bloques o métodos
válidos; duplicados en un bloque son errores ordinarios de nombres. Las
instancias añaden los argumentos e impls resueltas conforme a esta misma R09.
Esta clave no promete continuidad al insertar, eliminar o reordenar bloques:
H1/H2 rechazan toda edición semántica que afecte a un método, incluida su
adición, eliminación, cambio de identidad o de un cuerpo anónimo contenido,
con method_reload_unsupported de R47. La trivia que conserva estos registros
solo actualiza mapas fuente y admite recarga. No se atribuye el método a una
función superior ficticia ni a la declaración del tipo receptor.

**R10.** Esta regla es la única definición de las identidades de
contenido de D16/D17. Todos los resultados SHA-256 se representan como
64 dígitos hexadecimales minúsculos; los registros los transportan como
32 bytes. `file_digest` aplica SHA-256 a todos los bytes exactos de un archivo,
desde offset 0 hasta longitud exclusiva, incluidos BOM, trivia, finales de
línea, secciones y firma si ese archivo está firmado; sin prefijo, nombre,
ruta, timestamps ni permisos del filesystem. Fuentes archivadas C0/S0-std,
binarios, herramientas, salidas, fuentes de Span.revision y restauración de
archivos usan este nombre. R24 compara el archivo sin firmar completo,
identificado separadamente de su copia firmada.
La tabla de registros canónicos siguiente aplica SHA-256 con dominio igual
al nombre de cada fila. El registro lleva esa etiqueta, número de campos,
los campos propios enumerados en orden en la fila y, al final,
version_esquema: String y version_semantica: String, en ese orden, como dos
campos comunes obligatorios. Número de campos = campos propios + 2; la
etiqueta y el contador no cuentan como campos. version_esquema identifica
el esquema conjunto de registros R10 e IR tipada R07; version_semantica
identifica la semántica del lenguaje. Son IDs exactos compartidos por todos
los registros de un artefacto. No hay
etiquetas de dominio derivadas de una clase ni campos implícitos adicionales.
El valor de cada fila es el resultado SHA-256, no el registro de entrada;
`artifact_key` es la etiqueta de dominio; ambos ADR la nombran "clave de artefacto".
Igualdad semántica exige comparar también los registros canónicos: la
coincidencia de body_hash/interface_hash/dependency_hash no basta.

La codificación canónica usa registros etiquetados con campos en el orden
normativo de su esquema; cada etiqueta/String se codifica como longitud
U64 little-endian y bytes UTF-8 exactos, sin normalización. Bytes arbitrarios
anteponen longitud U64 little-endian y conservan todos sus bytes; un resultado
SHA-256 dentro de un registro usa exactamente sus 32 bytes, sin convertirlos
a texto ni añadir longitud variable. Un registro
antepone etiqueta y número U64 de campos; una lista antepone cantidad U64
y codifica sus elementos en orden. Enteros tienen anchura/tipo declarados
y little-endian; Bool usa un byte 0/1; un opcional antepone un byte 0/1 y solo 1
lleva valor. Mapas/conjuntos se codifican como listas ordenadas por los bytes
canónicos de sus claves, sin duplicados; no se ordenan listas semánticas
como parámetros, campos o instrucciones. No entran padding, direcciones
ni numeración de interning. N02 publica el esquema de cada registro y
vectores de sus bytes antes de aceptar artefactos; un campo desconocido
o una versión no soportada rechaza el artefacto.

La notación de tipos de esta tabla es de serialización, no sintaxis Nexo.
Hash256 son los 32 bytes indicados; `Par<A,B>` concatena A y B sin contador
ni etiqueta; `Lista<T>` y `Opcional<T>` usan los prefijos ya definidos. Solo los
siete registros de la tabla añaden los dos campos comunes. Sus nombres de
campo y los signos de esta notación no se escriben en el registro.
ObjetoCanon es un mapa String→ValorCanon: cantidad U64 y pares clave/valor
ordenados por bytes de la String codificada, sin duplicados. ValorCanon
lleva un discriminante U8 seguido del valor: 0=Bool, 1=I64 (complemento a
dos de 64 bits), 2=U64, 3=String, 4=Bytes, 5=Hash256, 6=`Lista<ValorCanon>`,
7=ObjetoCanon. No hay otros discriminantes ni conversiones implícitas entre
ellos; una lista heterogénea conserva el discriminante de cada elemento.
Los campos cuyo tipo ya aparece en la tabla no llevan ese discriminante,
salvo los declarados ValorCanon. Cada descriptor lógico se representa con
ObjetoCanon, sin etiqueta ni versiones comunes interiores; N02 fija sus
claves y tipos admisibles en el esquema versionado antes de aceptar
artefactos. No se admite una elección de representación por productor:
un campo de descriptor tiene un solo tipo en ese esquema; ausente, lista
vacía y valor cero nunca se intercambian. Los órdenes semánticos de la tabla
se conservan dentro de listas, incluso cuando el objeto que las contiene
ordena sus claves. Un esquema distinto exige otro version_esquema.

| Nombre y dominio | Campos propios en orden; contenido y exclusiones |
|---|---|
| `package_digest` | Campos: `(archivos:Lista<Par<String,Bytes>>)`. archivos es la lista exhaustiva de pares (ruta relativa, bytes completos del archivo) de su contenido distribuido, ordenada por bytes UTF-8 de ruta. Incluye fuentes .n, manifiesto de paquete, interfaz derivada, mapas fuente distribuidos, lockfile, catálogo requerido, target, edición Unicode y recetas, ya sean archivos propios o campos de esos archivos; no se recodifican los bytes interiores. Rutas con /, sin prefijo /, . ni .., sin duplicados ni enlaces; solo archivos regulares. No entran metadatos del filesystem, archivo contenedor/compresión ni firma/envoltura externa de integridad. El valor esperado se guarda fuera del contenido (registro del resolvedor/lockfile del consumidor); el lockfile incluido no contiene la integridad del propio paquete. Ningún archivo cubierto puede contener package_digest de sí mismo. |
| `body_hash` | Campos: `(cfg:Lista<Par<String,ObjetoCanon>>, valores_constantes:Lista<ObjetoCanon>, tipos_logicos:Lista<ObjetoCanon>, modos:Lista<String>, operaciones:Lista<ObjetoCanon>, continuaciones:Lista<ObjetoCanon>, limpiezas:Lista<ObjetoCanon>, referencias:Lista<ObjetoCanon>)`. CFG tipado canónico, valores constantes, identidades lógicas de tipos (sin package_digest), modos, operaciones, continuaciones, limpiezas y referencias por DefKey/instancia, respectivamente. Excluye layouts físicos, catálogo, trivia, spans, file_digest y source_map_hash. Referenciar hoja usa identidad lógica, sin versión/firma/disponibilidad. Bloques en recorrido determinista desde entrada, sucesores en orden del terminador; slots en orden de introducción. |
| `interface_hash` | Campos: `(firmas:Lista<ObjetoCanon>, modos:Lista<String>, procedencias:Lista<ObjetoCanon>, afinidad:Lista<ObjetoCanon>, cotas:Lista<ObjetoCanon>, efectos:Lista<ObjetoCanon>, catalogo_requerido:Lista<ObjetoCanon>, abi:ObjetoCanon, layouts:Lista<ObjetoCanon>, datos_publicos:ObjetoCanon, descriptor_propio:Opcional<ObjetoCanon>)`. Firmas (incluido el tipo declarado de constantes y los requisitos de traits/alias), modos, procedencias, afinidad, cotas escritas, resumen de efectos inferidos por operación invocable D11/D12, catálogo requerido, ABI, layouts públicos y entornos/estados, y datos públicos D13 §12, respectivamente; descriptor_propio es el opcional de almacenamiento del tipo descrito a continuación, ausente para las demás clases. Identidad de paquete = (origen, nombre, revisión), **excluyendo package_digest** y cualquier file_digest de sus fuentes. Cada tipo nominal, incluso privado, tiene además su propio registro de interfaz por DefKey: descriptor de almacenamiento físico y tipado, propiedades y limpieza; su registro no publica miembros privados a consumidores. Excluye cuerpo, trivia, spans, source_map_hash, recetas y contenido físico del paquete. |
| `dependency_hash` | Campos: `(interfaces_destino:Lista<ObjetoCanon>, cuerpos_consumidos:Lista<ObjetoCanon>, claves_instancia:Lista<ObjetoCanon>, aristas:Lista<ObjetoCanon>, componentes_scc:Lista<ObjetoCanon>)`. Interfaces de destinos llamados/referenciados y cuerpos consumidos por inlining/comptime/derivación (incluidos valores de constantes), con claves de instancias y aristas etiquetadas. SCC codificadas como listas ordenadas de claves, registros de interfaz/cuerpo y aristas internas/externas; no se calcula un dependency_hash recursivo sobre sí mismo. Excluye package_digest, file_digest, source_map_hash y dependencias no alcanzadas. |
| `source_map_hash` | Campos: `(archivos:Lista<ObjetoCanon>, spans:Lista<ObjetoCanon>, asociaciones_origen:Lista<ObjetoCanon>)`. Mapa completo: archivos por identidad lógica y file_digest de fuente, spans en bytes y asociaciones de origen de instrucciones/declaraciones; orden por clave canónica y ubicación. Excluye rutas absolutas del checkout y datos de entorno. Mover una línea cambia este registro sin cambiar necesariamente código. |
| `manifest_digest` | Campos: `(clase:String, contenido:Bytes)`. clase es exactamente la String ASCII `construccion` para R22 o `imagen` para R45, codificada con longitud U64 little-endian (12 o 6) y sus bytes UTF-8 sin NUL; es el primer campo y no sustituye el dominio manifest_digest. contenido son los bytes de la secuencia tipada del manifiesto definida abajo, con prefijo de longitud Bytes y sin dominio, contador de campos ni versiones interiores, incluidas entradas y salidas con file_digest/package_digest cuando corresponda. Excluye únicamente su propio campo manifest_digest y firma/envoltura externa. R22 separa este manifiesto de procedencia de los productos compilados; manifest_digest no entra en la clave de artefacto. |
| `artifact_key` | Campos: `(body_hash:Hash256, interface_hash:Hash256, dependency_hash:Hash256, perfil:ObjetoCanon, target_CPU:ObjetoCanon, flags:ObjetoCanon, version_ABI:String, version_runtime:String, catalogo_resuelto:Lista<ObjetoCanon>, layouts_concretos_usados:Lista<ObjetoCanon>)`. Excluye package_digest, file_digest de fuentes/mapas/productos, source_map_hash y manifest_digest. Los insumos externos que afectan código se incluyen mediante flags/catalogo_resuelto, nunca mediante integridad del paquete completo. |

Los totales U64 de campos son package_digest=3, body_hash=10,
interface_hash=13, dependency_hash=7, source_map_hash=5, manifest_digest=4
y artifact_key=12. En manifest_digest, contenido concatena exclusivamente
los valores tipados siguientes, en el orden de la fila; los nombres de
campo no son bytes del contenido. Objetos y listas conservan su propia
codificación definida arriba. No incluye clase ni una versión del
manifiesto: ambas se fijan por clase y los campos comunes del registro
exterior. Esquema IR/semántica de R22 son esos mismos campos comunes y no
se repiten en contenido. ABI/perfil sí son insumos distintos explícitos.

| Clase literal | Campos de contenido en orden |
|---|---|
| `construccion` | `fuentes:Lista<ObjetoCanon>, semilla:ObjetoCanon, ABI:String, perfil:ObjetoCanon, paquetes:Lista<ObjetoCanon>, target_CPU:ObjetoCanon, herramientas:Lista<ObjetoCanon>, entorno:ObjetoCanon, mapa_rutas:Lista<Par<String,String>>, orden_entradas:Lista<String>, epoca:I64, limites:ObjetoCanon, comandos:Lista<ObjetoCanon>, salidas:Lista<ObjetoCanon>` |
| `imagen` | `generacion:U64, target_CPU:ObjetoCanon, ABI:String, artifact_key:Hash256, archivos:Lista<ObjetoCanon>, imports_exports:ObjetoCanon, dependencias:Lista<ObjetoCanon>, segmentos:Lista<ObjetoCanon>` |

En construcción, fuentes contiene identidad/file_digest; semilla contiene
identidad/file_digest y número de generación real; herramientas incluye
compilador/backend/enlazador/SDK/runtime, sus file_digest y opciones. Los
restantes campos corresponden a R22, con integridad de paquetes y salidas
sin firmar. En imagen, archivos contiene ruta privada absoluta, longitud
U64 y file_digest de cada archivo final; imports_exports conserva nombres
y descriptores tipados, y segmentos sus rangos/permisos R45. Cada objeto
usa las claves fijas del esquema N02; los nombres de archivo, herramientas
e imports son datos, no claves de esquema nuevas. Las listas que R22/R45
no declaran secuenciales se ordenan por los bytes canónicos completos de
sus elementos y rechazan duplicados; fuentes, paquetes, herramientas,
salidas, archivos, dependencias y segmentos siguen ese orden. orden_entradas
y comandos conservan el orden de ejecución; mapa_rutas se ordena por la
String origen codificada y rechaza dos destinos para un mismo origen.

En la clasificación de recarga R47, declaration_kind=type designa struct/enum;
trait y alias conservan su clase propia. declaration_kind no es un campo
adicional de los registros de esta tabla. El campo firmas incluye el tipo
de la constante, los requisitos del trait o el destino resuelto del alias
cuando corresponda. Un alias no adquiere descriptor nominal propio por
tener DefKey. En body_hash, cfg es una lista de pares (identificador local,
CFG): vacía sin cuerpo ejecutable, un par con identificador vacío para una
función/método, y los defaults de trait ordenados por Name para un trait.
valores_constantes contiene los valores tipados canónicos de la constante
o de los CFG enumerados; los demás campos enumeran los datos de esos
cuerpos/valores en su orden canónico, con lista vacía si no hay elementos.

El catálogo requerido es el cierre transitivo de hojas del cuerpo y de
funciones/hooks llamados, incluidas std.core; SCC y llamadas indirectas usan
todos los destinos admitidos. Sus registros incluyen ID canónico, versión,
firma, efectos, pre/postcondiciones, procedencia y disponibilidad por target.
No depende de la grafía directa de una hoja en fuente ni de hojas ajenas
a ese cierre. Un tipo sin función también tiene interface_hash por DefKey.

Los componentes exclusivos de la clave de artefacto se definen así:

- `catalogo_resuelto`: lista ordenada por ID de las entradas requeridas por
  el artefacto y su cierre dependiente, con contrato requerido más proveedor
  concreto seleccionado, target, convención, símbolo/imagen, versión y
  file_digest de los insumos de proveedor fijados en el manifiesto. Excluye
  entradas instaladas pero no utilizadas, rutas de checkout y logs. El
  catálogo requerido expresa contrato; catalogo_resuelto fija su realización.
- `layouts_concretos_usados`: lista ordenada por identidad de tipo instanciado
  de todos los descriptores usados en código/datos del artefacto y su cierre,
  incluidos privados, temporales, entornos, estados, flags de vida y préstamos.
  Incluye tipos de slots, tamaño/alineación, offsets, tags y representación
  para target; excluye tipos sin uso, direcciones y contenido de valores.
  Los layouts públicos de interface_hash son solo la parte del contrato
  público, salvo el registro propio de cada tipo descrito en la tabla.
- `flags`: mapa cerrado del esquema de emisión con backend y su versión,
  file_digest de las herramientas externas de emisión/enlace y entradas SDK utilizadas,
  nivel/pasadas de optimización, instrumentación, depuración y opciones de
  enlace que afectan código/datos. Cada opción tiene valor explícito, también
  el predeterminado; opción desconocida rechaza. Excluye opciones cosméticas
  de CLI, rutas de salida, logs, firma posterior y valores ya representados
  por perfil, target/CPU o versiones ABI/runtime. El ejecutable semilla/compilador
  y el número de stage son procedencia R22, excluida de flags; las fuentes de
  su emisor integrado participan por las dependencias normales, sin incrustar
  el file_digest del propio compilador en sus productos.
- `perfil`: registro de ID y versión de perfil, capacidades habilitadas,
  modelo de costes/recetas y todos los límites de análisis/ejecución y sus
  ámbitos. Incluye parámetros del arnés como L en D16 R32 cuando se usa ese
  perfil; excluye saldos consumidos, tiempos medidos, RSS y reloj de pared.
  Cambiar una receta o parámetro normativo cambia el registro, aunque el
  ID legible se conserve. No añade recetas a interface_hash.

Target/CPU (campo target_CPU) identifica sistema, ISA, endianness y características
habilitadas; version_ABI y version_runtime son IDs exactos versionados, no
nombres móviles. Esquema IR y semántica se codifican una sola vez, como
version_esquema y version_semantica comunes; artifact_key no los repite
entre sus campos propios. R08.6 se incorpora mediante esos componentes
y los tres componentes body_hash/interface_hash/dependency_hash, sin agregar
datos implícitos. Cambio de layout, catálogo, efectos, target o flags invalida
el cierre consumidor aunque body_hash coincida. source_map_hash adjunta la
ubicación por separado; una edición de trivia con revisión resuelta fija
actualiza file_digest/package_digest esperados y el mapa fuente, conservando
los tres componentes semánticos y la clave de artefacto. No permite modificar
silenciosamente un paquete publicado inmutable: las fixtures usan un paquete
de desarrollo cuyo registro de integridad se recompone explícitamente.

**R11.** Closures/generadores/async tienen clave de función contenedora y ruta
estructural local del sitio creador, más layout de entorno/estado. No se
promete identidad estable si se insertan sitios anónimos que cambian esa ruta.
Una duda de correspondencia obliga a reconstruir/reiniciar; no se casa por
similitud textual. La recarga inicial de código solo sustituye funciones superiores
identificadas sin ambigüedad; los valores de constantes y las altas se rigen
por R47, y los estados activos por R44.
Dentro de un método, la clave contenedora es la DefKey de método de R09;
su cambio se rechaza por method_reload_unsupported, incluso sin estados vivos.

**R12.** El lowering a máquina conserva evidencia hasta cerrar las obligaciones:
no marca `noalias` por el mero `edit` si hay CoW; no borra limpieza observable;
no elimina comprobaciones enteras sin prueba de rango; no reasocia flotantes
estrictos; no convierte efectos desconocidos en vacíos; no baja un préstamo
a un campo móvil suspendido como puntero sin estabilidad demostrada.
Los helpers, derrames, asignación, RC, publicación y llamadas de sistema
deben figurar en efectos/costes. Un mapa de depuración relaciona instrucciones
sintéticas con su causa. Se conserva un informe verificable de cada descarga
de obligación antes de borrar metadatos semánticos.

## 3. Prerrequisitos y subconjunto de arranque

**R13.** Se fija esta lista cerrada de capacidades necesarias para la primera
cadena completa. El orden es de incorporación; una capacidad más rica no
puede introducirse en las fuentes del compilador hasta que la semilla anterior
la admita. Los cortes futuros indicados son tareas de D17 §10, no ADR inventados
ni capacidades que ya se afirmen implementadas.

| Orden | Capacidad requerida | Autoridad y primera entrega | Necesaria para |
|---|---|---|---|
| 1 | I64/Bool/unidad, funciones/recursión, Result/`?`, CFG, propiedad, efectos y limpieza estable | 0001; 0002 I03–I11 completo | Todo componente |
| 2 | Option, match exhaustivo, cierres locales y generadores propietarios con cuotas | 0003 J01–J16 | Alternativas; validar que el frontend reconoce estas formas |
| 3 | Aritmética comprobada, lógica, bucles/rangos, Array dinámico, constantes escalares | 0005 F5-L01–L12 | Tablas, pilas explícitas, índices y ordenación inicial |
| 4 | String UTF-8, escalares y spans en bytes, structs/enums acíclicos, métodos, Format/PartialEq | 0004 K01–K12 | Tokens y nodos por ID, errores y formateador |
| 5 | U8/U32/U64, conversiones fallables, operaciones modulares/de bits y Bytes | D16 R18/C01–C02, M05 (enteros/bits/conversiones decimales) y M09 (Bytes); N06 consume en F1 | Entrada binaria, file_digest/package_digest, objeto y ABI |
| 6 | Módulos y paquetes locales, identidades, visibilidad, firmas públicas y lockfile | F1/D13; tabla/identidades N02; N06 consume; lockfile N07b F2 | Descomposición del compilador y construcción cerrada |
| 7 | Genéricos estáticos nominales, traits/impl coherentes, tipos asociados, Eq/Hash y cotas Call* | F1/D08/D11; N06 consume | Colecciones reutilizables y algoritmos; nunca especialización solapada |
| 8 | Almacenamiento propietario dinámico D16, contexto de asignación, recursos con cierre fallable explícito y limpieza residual estable | D16 M05–M10 y M11a; N06 consume en F1 | Compilador de tamaño no limitado a una fixture |
| 9 | Mapas hash, ordenación determinista, búsqueda y tablas de internado escritas en Nexo | D16 M09b/M09c únicos implementadores en F1; N06 integra y verifica | Nombres, tipos, grafos, cachés y serialización |
| 10 | E/S real acotada de archivos, rutas nativas, salida y argumentos; entradas de construcción enumeradas en manifiesto | D10, F1/F2 y D16 M11, N06 y N07a–N07c | Leer proyecto y emitir artefactos sin exploración implícita del disco |
| 11 | Emisión nativa LLVM, ABI, interfaces de paquetes, informes JSON, depuración y construcción determinista | D04, ABI N02, F2 y N07a–N07c/N11 | Compilador Nexo ejecutable y cadena stage1–3 |

La lista excluye como prerrequisitos: `dyn`, regiones de usuario, arenas
generacionales reutilizables, SoA/SIMD, CoW optimizado, concurrencia, async,
red/TLS, UI, hot reload y backend propio. Permanecen compromisos del roadmap
donde corresponda, pero no se usan para escribir el primer compilador.
Closures se permiten solo con las cotas y capturas ya soportadas; la primera
cadena puede usar funciones directas y no depende de capturas escapadas.
Las anchuras U8/U32/U64, operaciones modulares, BitAnd/BitOr/BitXor/BitNot,
ShiftLeft/ShiftRight/RotateRight, conversiones y sus recetas tienen definición
única en D16 R18/C01–C02 y propietario M05. La construcción de constantes
decimales U32 también se rige por D16 R18; no se añaden hexadecimales ni una
intrínseca SHA. M09b implementa SHA-256 Nexo y sus vectores; N06 verifica
integración para file_digest/package_digest y serialización. R58 registra el adelanto numérico F1.

N07a entrega antes de N07b y del cierre F2 la enmienda propuesta a D16 «4. Catálogo cerrado de confianza», R22: selector confiado ProcessRun síncrono,
efectos io/block/alloc, sin suspend. Recibe ejecutable por ruta absoluta,
argumentos separados, entorno cerrado, directorio de trabajo y buffers de
salida con capacidad. Sin shell ni búsqueda PATH; rechaza NUL/rutas inválidas
y capacidades inválidas antes de lanzar. El directorio debe coincidir con el
directorio de trabajo fijado al iniciar el driver; otro valor se rechaza,
sin cambiar el directorio global ni depender de extensiones de spawn.
Devuelve Result con estado de
salida/señal y longitudes inicializadas exactas; fallo de lanzamiento no crea
hijo. Espera/recolecta un hijo y libera handles exactamente una vez. Si salida
excede capacidad, termina/recolecta al hijo, devuelve error y descarta productos
incompletos. No reintenta ni convierte salida distinta de cero en éxito.
La operación externa es observable; no promete determinismo ante otro entorno.
El driver Nexo usa ProcessRun para LLVM/enlazador en `build`, `build --python`
y `test`, también después de N13. La receta externa de arranque usa el mismo
manifiesto R22, sin lógica de tipos ni generación. N20 retira el enlazador
externo; N22 retira LLVM según R39. ProcessRun sigue como API general,
sin adaptador de
compilación permanente. N07a entrega proveedores de cuatro destinos, pruebas
de fallos/efectos e inventario R40. Incluye el proveedor ProcessRun del
intérprete Rust candidato que se archivará en C0 para R26: ejecutar S bajo
ese intérprete debe poder lanzar LLVM/enlazador con el mismo manifiesto,
resultados, límites y efectos. N10 porta ese contrato al intérprete Nexo;
no se exige ejecución nativa de S para obtener D1. No se necesita listar
directorios para
arrancar: entradas enumeradas en manifiesto. M11 entrega FileMakeDirectory,
FileRename, FileKind y FileRemove (io/block, sin alloc/suspend propios), con rutas/buffers del
llamador, permisos y errores nativos explícitos. El driver crea, en orden de
padres a hijos, los directorios de salida enumerados; ya existente solo
acepta FileKind=Directory sin enlace, nunca un archivo homónimo. Publica mediante
rename en el mismo volumen y falla antes de publicar si cruza volumen;
no ejecuta un shell ni inventa exploración implícita. FileRemove elimina
solo temporales propios enumerados, según D16 R22; FileKind clasifica entradas,
sin metadatos generales, seek ni flush
ni ofrece durabilidad frente a caída del sistema. R40 fija proveedores.

**R14.** El subconjunto mínimo B0 para el primer frontend son las filas 1–4,
sin exigir que su implementación use closures o generadores. Se escribe
como funciones de un archivo con tipos monomórficos. El grafo AST se representa
por tablas de registros propietarios y IDs I64 comprobados, sin tipos de
tamaño recursivo, referencias prestadas almacenadas ni punteros. La pila del
parser y los recorridos son datos explícitos acotados; no se obtiene robustez
con recursión ilimitada del intérprete. Puede reconocer toda B sin usar toda
B en su propia implementación.

**R15.** En B0 se recibe String ya válido y se devuelven tokens/AST/diagnósticos/
texto. El método Scalars de String (K §3) permite materializar códigos;
se acumula el ancho UTF-8
de cada escalar para conservar offsets de bytes originales. No se normalizan
CR/LF/CRLF ni Unicode. El rechazo de bytes inválidos antes del lexer requiere
Bytes/E/S de N06 y no se acredita con esta entrada String. El AST conserva
modos, operadores, ramas, etiquetas y orden; el CST conserva trivia según D15.
No se llama al parser Rust desde el código Nexo.

**R16.** La aceptación del primer frontend exige igualdad de tokens y spans,
AST normalizado, diagnósticos estructurados y salida canónica con el referente
Rust congelado y con esperados transcritos de D15. Incluye todas las fixtures
D15, las de 0002/0003/0005/0004 aplicables al perfil y todo el corpus congelado,
positivos y negativos. Se exige parseo–formato–parseo e idempotencia; preservar
solo el resultado de ejecutar no prueba igualdad de AST. El estilo que cambia
nombres sigue la resolución y las colisiones de D14/D15, no heurísticas del
lexer. Formas todavía excluidas tienen idéntico diagnóstico de alcance.

Para B0, las entradas UTF-8 inválidas se marcan NO EVALUADO y bloquean la
retirada del frontend Rust; N06 las incorpora antes de la equivalencia total.
Se añaden 10.000 fuentes generadas determinísticamente por familia de perfil
y 10.000 mutaciones de bytes con semillas registradas, profundidades y tamaños
de frontera. Toda discrepancia se reduce y conserva. El referente no puede
legitimar una discrepancia frente al ADR: se corrige la causa en ambos lados
o se revisa expresamente la regla antes de aceptar.

**R17.** El siguiente esbozo B0 de lexer numérico usa únicamente formas de B
y capacidades de 0003/0005. No sustituye al lexer B completo ni demuestra
rendimiento. Recibe códigos ASCII como I64; reconoce dígitos, `+` y espacio,
devuelve pares planos `(clase, offset)` y Err(offset) al hallar otro código.
Separa préstamos antes de Push y de asignaciones compuestas.
I64, Bool, Array, Result, Len, Ok, Err y AssertEq pertenecen al preámbulo
documentado en D15 §2.3; Push es la intrínseca de F5 §4.1. IsDigit y
LexNumbers se declaran en el propio módulo; las demás referencias de valor
son parámetros o bindings locales.

```nexo
fn IsDigit(code: view I64) -> Bool {
    return code >= 48 and code <= 57;
}

fn LexNumbers(codes: view Array<I64>) -> Result<Array<I64>, I64> {
    var tokens: Array<I64> = [];
    var pos = 0;
    let size = Len(codes);
    while pos < size {
        let code = codes[pos];
        if code == 32 {
            pos += 1;
            continue;
        }
        let start = pos;
        if IsDigit(code) {
            pos += 1;
            while pos < size {
                let next = codes[pos];
                if not IsDigit(next) {
                    break;
                }
                pos += 1;
            }
            Push(edit tokens, 1);
            Push(edit tokens, start);
        } else {
            if code == 43 {
                Push(edit tokens, 2);
                Push(edit tokens, start);
                pos += 1;
            } else {
                return Err(pos);
            }
        }
    }
    return Ok(move tokens);
}

fn Main() {
    let input = [49, 50, 32, 43, 51];
    let result = LexNumbers(input);
    AssertEq(result, Ok([1, 0, 2, 3, 1, 4]));
    AssertEq(LexNumbers([64]), Err(0));
}
```

**R18.** El subconjunto B1 para el compilador completo añade filas 5–11.
Las tablas de símbolos usan mapas hash cuya iteración nunca define el orden
de diagnósticos, de objetos ni de IDs. Las salidas se ordenan por claves
canónicas; semillas aleatorias de mapas solo afectan coste interno. La
ordenación inicial es mergesort estable con comparador total y espacio
auxiliar explícito. Para tablas de nodos no se reciclan IDs durante una
compilación: cada acceso comprueba tabla, tipo y rango. No se promete la
seguridad generacional de D05 antes de F3 ni se hace indispensable para arrancar.

B1 limita a 1.024 frames de usuario por ejecución; los helpers administrativos
usan la reserva separada library_frames de R50 conforme D16 R32.
El intento 1.025 emite R0002/call_frames, limit=1024, observed=1025, primario
en llamada desencadenante y limpieza única conforme A2. El intérprete usa pila
explícita; el nativo comprueba profundidad antes de reservar frame, con máximo
conjunto de 5.120 frames (1.024 de usuario y 4.096 administrativos). Recorridos
no acotados usan pilas de datos. La pila anfitriona no implementa la cuota;
un fallo físico de pila/guard page aborta el proceso e invalida la cadena,
sin continuar ni publicar artefactos.

**R19.** Cambiar representaciones experimentales de F0 por las de D16/F1
es una migración única: algoritmos, fixtures y fuentes del frontend se migran
juntos en M09, incluida toda fuente B0 ya entregada en N03–N05, sus
firmas Array<I64>, Scalars y fixtures. N06 verifica esa migración; N04 y
N05 que comiencen después de M09, incluidos los diferidos por R51, usan
directamente List y el perfil F1 vigente. N03 fija capacidades del perfil
disponible, sin obligar a escribir contra Array dinámico después de retirarlo.
R17 queda solo como esbozo histórico B0,
no una fuente activa que se recompila tras M09. No se añade un Array antiguo
paralelo, alias de compatibilidad ni
adaptador para compilar el código de arranque. Si B0 supera las cuotas F0,
se informa el límite; no se eleva selectivamente por reconocer que la fuente
es el compilador. El primer frontend completo puede esperar F1 si la escala
no cabe; eso no convierte el esbozo en cumplimiento de R16. Bootstrap-v1
es el perfil F1 explícito de R50 para cualquier fuente B1: eleva cuotas de
forma publicada y uniforme, no selectivamente para el compilador. B0/F0 y
el corpus histórico conservan sus perfiles; no se compara agotamiento bajo
perfiles distintos como divergencia semántica.

## 4. Semilla y cadena de tres generaciones

**R20.** C0, identificada como `nexo-bootstrap-r0`, será la primera revisión
inmutable del compilador Rust que cumpla conjuntamente: F1 aprobado; puerta
F2 de R31 cumplida; B1 implementado; capacidad de compilar todas las fuentes
del candidato Nexo; y equivalencia completa de componentes R28. Su identidad
es commit completo más file_digest del archivo de fuentes y de cada binario por
destino (R10), nunca una rama móvil ni último Rust. No se inventa ahora un file_digest
o una versión que todavía no existe. El registro se rellena al cerrar N11;
sin esos valores concretos no existe una semilla aceptada.
Se distingue de S0-std: la revisión Rust, file_digest de fuentes/binarios y receta congelados **antes
de F2** por M12 y este R20 (D16 R27 remite aquí) para reconstruir std desde fuentes sin std previa.
S0-std es un punto de recuperación de F1; C0 es su sucesor con backend y
puertas F2 cerradas, utilizado en la primera cadena nativa de tres generaciones.
Ambas revisiones se archivan sin dos implementaciones activas. No se atribuye
a S0-std una capacidad nativa que F1 excluye ni se retrasa su congelación
hasta elegir C0. M12 fija S0-std, N06 lo registra; N11 fija C0.

**R21.** La semilla de distribución es un binario nativo auditado por destino,
acompañado de fuentes, manifiesto de construcción y evidencia reproducible.
Se elige binario, no IR serializada congelada: F0-0002 no acepta volcados como
ejecutables y un lector/intérprete nuevo sería otra base de confianza. La IR
canónica se archiva para comparación, sin darle permiso de ejecución.
Inicialmente el binario semilla es C0; después de N12 es stage2 en Nexo.
La aceptación de C0 exige reconstruir también sus binarios desde sus fuentes
Rust en dos entornos limpios por destino y obtener igualdad byte a byte de
los artefactos no firmados. Verificar solo file_digest del binario descargado
no acredita esa reconstrucción. Rust y sus dependencias se fijan por versión
y file_digest en ese manifiesto histórico.

PE/COFF fija timestamps a la época del manifiesto, remapea rutas y deriva
GUID de debug del contenido. Mach-O fija UUID por contenido, orden de load
commands y rutas SDK. Se conserva el producto previo a firma para file_digest y
comparación; macOS arm64 ejecuta una copia firmada ad hoc. Se archivan ambas
y la receta de firma. Solo se excluyen envoltura de firma y campos de enlace
especificados por formato: se verifica que firmar no cambie código/datos/debug.
No se hace strip arbitrario para obtener igualdad. R24 y las rutas diversas
aplican esta misma separación entre contenido reproducible y firma.

**R22.** El manifiesto de cada reconstrucción contiene, sin campos implícitos:
identidad y file_digest de fuentes; semilla y file_digest; esquema/semántica/ABI/perfil;
grafo completo de paquetes con integridad; target/CPU; file_digest y opciones de
compilador, backend, enlazador, SDK y runtime; variables de entorno admitidas;
mapa de rutas; orden de entradas; época temporal fija; límites; comandos y
file_digest de salidas sin firmar. La red está deshabilitada tras materializar
entradas. Reloj, usuario, hostname, ruta de checkout, semilla aleatoria y
orden de directorio no entran accidentalmente en el resultado. Ninguna
dependencia se resuelve de nuevo contra «latest» durante una reconstrucción.
El manifiesto de procedencia y los logs son evidencia externa: registran la
semilla y el número de generación reales y por ello pueden diferir entre
stages. No se incrustan en objetos ni ejecutables. La comparación R24 abarca
los productos compilados y su inventario de contenido, no esos informes de
procedencia; no autoriza otras exclusiones de productos para lograr igualdad.

**R23.** Sean S las mismas fuentes Nexo completas, L la misma biblioteca Nexo,
y E el mismo conjunto de backend/enlazador/SDK/flags. Se ejecuta:

```text
stage1 = C0.compilar(S, L, E)
stage2 = stage1.compilar(S, L, E)
stage3 = stage2.compilar(S, L, E)
```

Cada llamada incluye construir biblioteca/runtime y herramientas entregadas;
no enlaza inadvertidamente una biblioteca Rust de C0. La entrada de stage2
es stage1 ejecutándose nativamente, no el intérprete Rust ejecutando S. Cada
stage supera antes de actuar como compilador el corpus y las verificaciones
de IR. No se exige que stage1 y stage2 tengan los mismos bytes.
`compilar` en esa notación representa la receta completa de R13, no una
nueva API fuente ni una capacidad SystemCall implícita del compilador.

**R24.** Puerta de punto fijo: inventarios idénticos y file_digest idénticos,
confirmados por comparación byte a byte, para **todos** los artefactos no
firmados de stage2 y stage3: ejecutables, bibliotecas, objetos distribuidos,
interfaces y depuración. Las IR canónicas también coinciden, pero no pueden
compensar binarios distintos. No se elimina una sección de código, dato,
relocación o debug para fabricar igualdad. Firma/notarización se realiza
después y se archiva aparte, como F6 permite. Se ejecuta la cadena en dos
entornos limpios por cada uno de los cuatro destinos. No se exige igualdad
entre sistemas operativos o ISAs diferentes.

**R25.** A partir de la primera versión autoalojada V, la semilla de V+1 es
la versión V aceptada, con manifiesto. Las fuentes usadas para construir V+1
solo emplean capacidades admitidas por V. Una capacidad nueva se implementa
sin usarla en las fuentes del compilador hasta la siguiente versión. No se
salta la cadena ni se reincorpora Rust para facilitar una actualización.
También se repiten tres generaciones con la semilla anterior y punto fijo.

**R26.** Reproducibilidad y punto fijo no prueban ausencia de una inserción
maliciosa heredada. Se exige compilación diversa antes de retirar Rust:
ejecutar S bajo el intérprete de referencia Rust
archivado con bootstrap-v1 de R50 produce un compilador D1; D1 compila S para producir D2; D2 debe
ser idéntico a stage2 bajo R24. Ese intérprete se reconstruye desde fuentes
auditadas en dos entornos y usa un camino de ejecución distinto del backend
nativo. Los revisores examinan fuente, herramientas y diferencias de confianza,
incluidas versiones/toolchains que siguen siendo comunes.

Esta comprobación detecta divergencias de esa ruta; compartir parser, sistema,
SDK o LLVM deja confianza común. No se denomina demostración independiente
de toda la cadena. Si ejecutar el compilador bajo el intérprete no termina
en el presupuesto R50, la puerta queda INCONCLUSA: no se omite la prueba ni
se sustituye por compilar dos veces con el mismo ejecutable.
Después de retirar Rust, cada cambio de semilla conserva esta prueba con
el intérprete de la versión Nexo anterior aceptada como ruta de ejecución
diversa. R25 garantiza que puede ejecutar las fuentes S; no se exige a C0
entender extensiones posteriores ni se vuelve a depender de Rust para
publicar. Se registra la ascendencia común de ambas rutas. Una discrepancia
bloquea la semilla aunque las dos sean ya Nexo.

**R27.** Antes de retirar LLVM se compara A, versión Nexo anterior aceptada
con LLVM archivada, y P, semilla de la ruta propia nueva. S/L son idénticas
fuentes que incluyen el emisor propio; Epropio fija target/SDK/enlazador y
opciones finales propias. P se obtiene sin circularidad: sea Sprev la fuente
de A, cuya versión de desarrollo ya incorpora el emisor propio N16–N20
cerrado; se construye `P = A.compilar(Sprev, Lprev, Epropio)`, se verifica
con corpus/IR y se fijan file_digest y manifiesto en dos entornos. A genera la semilla
P con su emisor propio, mientras la ruta a1 usa LLVM; P no se obtiene de p2,
p3 ni de un ejecutable supuesto. R25 exige que A/P admitan las fuentes S.
Con P así fijada se ejecuta:

```text
a1 = A.compilar(S, L, Ellvm)
a2 = a1.compilar(S, L, Epropio)
p1 = P.compilar(S, L, Epropio)
p2 = p1.compilar(S, L, Epropio)
p3 = p2.compilar(S, L, Epropio)
a2 == p2 == p3  (inventarios y bytes bajo R24)
```

No se exige a1==p1. Cada generación supera corpus/verificador antes de usarse.
Además se repite R26 con intérprete Nexo anterior, perfil R50 y salida Epropio;
su D2 coincide con p2. Se declara confianza común. Semillas y fuentes históricas
se archivan fuera del árbol activo y de la construcción ordinaria.

## 5. Sustitución por componentes y retirada de Rust

**R28.** Se acepta cada componente en este orden. Los esperados se obtienen
de la norma y de un referente histórico fijado, nunca solo del candidato.
La igualdad diagnóstica excluye texto localizado, duración y rutas absolutas
remapeadas; incluye orden, códigos, subcausas, spans originales, tokens,
tipos, relaciones, cadenas causales, reparaciones, perfil y datos de cuotas.
La comparación previa a C0 utiliza una revisión candidata Rust R, fijada
por file_digest de fuentes/binarios y **posterior** a M08–M11 de D16,
con std/List y recetas de coste ya migradas. No compara el perfil F0 anterior
con el F1 nuevo fingiendo igualdad literal. Para migraciones de biblioteca
rige exclusivamente el mapa de diferencias autorizado D16 R31–R33; D17
no agrega excepciones nuevas al portar el compilador de ese mismo perfil.
Las seis filas ejecutan las fuentes Nexo bajo el intérprete de R con R50;
la fila 5 pasa su LLVM IR a E y ejecuta el corpus nativo resultante. El referente
usa componentes Rust sobre iguales entradas. No requieren stage1 ni C0 aceptado.
Tras R28 y reconstrucción de R en dos entornos R21 se congela R como C0.
Orden: S0-std → R con R31 → R28/R21 → C0 → R23 → R24/R26 → R29;
R25 gobierna versiones posteriores y R27 solo retirada LLVM. Los programas
probados conservan su propio perfil: R50 limita el proceso compilador, no
modifica cuotas observables del corpus alojado.

| Orden | Componente Nexo | Equivalencia obligatoria | Costes y retiro |
|---|---|---|---|
| 1 | Lexer, parser, CST y formateador | R16, incluido UTF-8 inválido tras N06 | Bytes leídos, límites y cuotas iguales; tiempo/RSS medidos |
| 2 | Resolución, tipos, propiedad, efectos y genéricos | Mismo conjunto admitido/rechazado, tipos nominales, permisos y diagnósticos; límites del mismo perfil | Contadores de análisis iguales en el perfil transcrito; ninguna cuota omitida |
| 3 | Lowering y verificador IR | IR canónica igual, planes/flags/procedencias iguales; mismas mutaciones inválidas rechazadas | Inventario IR y contabilidad iguales; sin nodos AST pendientes |
| 4 | Intérprete | Resultado, salida, eventos, primer fallo y limpieza en orden; cuotas lógicas exactas | Ticks y bytes del modelo iguales; RSS/tiempo no se confunden con ellos |
| 5 | Emisor LLVM, runtime y acceso a sistema | IR verificada aceptada, corpus nativo y ABI; R31 | Semántica/costes contractuales iguales; medición sin puerta temporal previa a C0 |
| 6 | CLI, gestor local, servidor incremental, REPL y LSP | Interfaces consultan el mismo núcleo; T09/T23/T25 y funcionalidad de cargas T26, sin puerta temporal previa a C0 | Sin reinferencias paralelas ni caché que oculte diferencias |

**R29.** La aceptación de una fila no activa una segunda implementación de
producción. Los candidatos se evalúan en un checkout aislado de migración;
las referencias Rust son artefactos archivados del HEAD anterior. La sustitución
del compilador se integra en **un solo corte** cuando las seis filas, R24, R26 y R30
están cerradas y se cumple R25 en las actualizaciones: se eliminan físicamente código Rust, Cargo, fixtures obsoletas,
wrappers y caminos de ejecución sustituidos; quedan las fuentes Nexo, tests
migrados y una construcción normal única. No se mezclan componentes por ABI
Rust/Nexo ni se distribuye un selector de frontend.

El criterio de retiro de cada componente Rust es su fila R28 más la puerta
global, no el momento en que su traducción empieza a funcionar. El histórico
de git y las semillas archivadas conservan la comparación. Un inventario
de dependencias, símbolos enlazados y procesos de construcción debe mostrar
cero ejecución de rustc/cargo y cero enlace de runtime Rust en stage2/3.

La retirada incluye Cranelift y su crate/adaptador **en N13**, según R41.
Si se promovió antes, R41 exige primero demostrar el retorno a LLVM dentro
de R30; de otro modo N13 se difiere. No se pierde un backend sin sustituto.

**R30.** Para autorizar esa sustitución, en las máquinas fijas de roadmap §4.3 se
miden 100 ediciones por categoría y 10 construcciones limpias, alternando
referencia/candidato con idénticas entradas. El cociente de medianas de tiempo
limpio y RSS máximo candidato/Rust es ≤2 para el compilador completo y para
los proyectos T26 de 10k/100k líneas. Las métricas incrementales p95 tienen el mismo límite de migración
(cociente candidato/Rust ≤2); no se admiten nuevas regresiones del
corpus. Los máximos roadmap §4.3 de 750 ms/4 s/60 s y 2 s/15 s/300 s siguen como puerta
F6; N13 antes de F3 no los exige anticipadamente. El presupuesto 2× es de migración,
no una sustitución de las hipótesis de producto ni de roadmap §7.6.

Las cuotas exactas de F0 se conservan al portar el mismo perfil. Un algoritmo
nuevo con otro modelo de análisis requiere perfil nuevo, regresiones y
enmienda de sus cuotas; no se compara como si sus contadores fueran idénticos.
Los costes de ejecución nativa eliminables pueden variar según P04; los
contratos prohibidos nunca. Las mediciones incluyen formateo, limpieza,
asignación, enlace y arranque donde corresponda.

## 6. Backend propio y puerta de extremo a extremo

**R31.** La puerta E2E-F2 exige F1 y F2 aprobados por el propietario, con
evidencia de todos estos elementos, sin NO EVALUADO/FALLA/INCONCLUSO:

- T09, T18 local, T19, T20–T23, T25 y T26; EX01/EX04 completos con biblioteca real.
- Corpus F1 y al menos 10.000 programas generados registrados: intérprete
  frente a LLVM sin optimizar y optimizado, mismas observaciones definidas.
- REPL/archivo coherentes, fmt idempotente/AST conservado, invalidación de
  traits, efectos, defaults, metadatos y objetivo; diagnósticos CLI/LSP/REPL.
- Puente Python mínimo instalable, rangos/UTF-8/vacío/error/liberación una vez,
  sin escapes ni exclusividad deducida de readonly.
- Linux x86-64 y macOS arm64 completos; Windows x86-64 y Linux arm64 con
  generación/enlace/ejecución de escalares, llamadas, texto y limpieza.
- T26 con ambos tamaños, desglose, memoria y contingencia D04 cuando proceda.

**R32.** E2E-F6 exige además F1–F6 aprobados, T01–T28 completos en su alcance,
cuatro plataformas nativas, revisión independiente de préstamos/regiones/
D08/D09/fronteras, cero defectos de seguridad o semántica conocidos, roadmap §4.3 y
roadmap §7.6 cumplidos, H-PY/H-RS/H-CS confirmadas con el protocolo vigente, puertas
de usabilidad y adopción, instalación limpia, tres proyectos externos sin
parches privados y reconstrucción en dos entornos por cadena. No bastan demos,
conteo de tests ni una cohorte favorable. La propuesta H-PY de acuerdos §5
no cambia por sí sola el protocolo confirmado.

**R33.** Se fija ahora el diseño, pero la sustitución de LLVM se habilita solo
tras E2E-F2 **y** E2E-F6. Hasta entonces se implementan ganchos, autoalojamiento
con LLVM y, únicamente antes de N13, la contingencia D04; no se desplaza la validación original por
prometer un optimizador futuro. El backend propio entra como extensión
posterior a F6, sin convertirlo retroactivamente en puerta de la primera 1.0.
Si el propietario cambia el alcance confirmado, hará falta enmienda explícita
de esta puerta; no se interpreta un pivote parcial como E2E-F6 completo.

**R34.** Primer backend propio: selección escalar directa, asignación de
registros lineal con derrames explícitos, llamadas directas/indirectas,
control, enteros comprobados, flotantes estrictos y limpieza. Sin vectorización,
inlining global ni reordenación agresiva inicial. Esta restricción es de
optimización, no de semántica: antes de distribuir un destino de desarrollo
debe bajar todas sus operaciones comprometidas, incluidos atomics, TLS de
hilo, futuros/generadores, publicación y fronteras de FFI. Las operaciones
complejas pueden ser helpers escritos en Nexo con contratos verificados.

**R35.** Orden de habilitación propia: (1) Linux x86-64, (2) Windows x86-64,
(3) macOS arm64, (4) Linux arm64. Se reutiliza por ISA la selección y se
separan ABI/formato/sistema. Esta secuencia posterior a F6 no cambia el orden
de evidencia LLVM de roadmap §4.2. Cada destino se prueba en hardware nativo; ARM
real es obligatorio para ABI, atomics y rendimiento. No se retira ningún
destino del producto durante la transición; los aún no cerrados siguen con
la distribución LLVM anterior. No se prometen todos los pares de cruce.

**R36.** El emisor propio escribe directamente objetos estándar ELF64,
PE/COFF y Mach-O64, en ese orden de plataformas; no emite ensamblador textual
ni inventa un ejecutable Nexo que necesite otro cargador. Los ejecutables
usan los formatos/cargadores nativos. Inicialmente el enlace puede usar
la herramienta fijada en E; se identifica como dependencia transitoria.
Un enlazador escrito en Nexo la sustituye antes de R40: resuelve símbolos,
relocaciones, imports/exports, bibliotecas compartidas y mapas de depuración
del conjunto emitido por el compilador y de los perfiles de interoperabilidad
comprometidos. No se promete consumir todos los objetos de terceros.

El enlazador debe soportar ejecutables y bibliotecas cargables por CPython
en los cuatro destinos, SDK/importaciones nativas, alineación y protección
de secciones, TLS de hilo y carga dinámica H2. Produce salidas deterministas;
firma/notarización queda externa y posterior. Un formato/relocación no
soportado se rechaza explícitamente, nunca se acepta ignorándolo. Se conserva
Wasm32 y la ruta de herramientas al Worker de F4: el emisor Wasm escribe
módulos estándar desde la misma IR, sin exigir LLVM en el navegador.

**R37.** Mientras exista LLVM, se confrontan tres ejecuciones del mismo
programa y entrada: intérprete, LLVM y backend propio. Igualdad de valores,
salida, errores definidos, efectos externos y limpieza; concurrencia usa
intercalaciones controladas y conjuntos de resultados permitidos P04/D09,
no igualdad de un scheduling accidental. Float compara el contrato estricto
y sus observaciones de bits, no una tolerancia elegida tras ver diferencias.
OOM y tiempos externos siguen su política ambiental, no se simulan como
errores deterministas del lenguaje.

**R38.** Cada destino propio necesita corpus completo aplicable T01–T28,
10.000 programas generados por subconjunto publicado, negativos de verificador,
errores de enlace y ABI bidireccional contra bibliotecas de prueba independientes.
Las familias incluyen ceros, MIN/MAX, overflow, NaN/inf/cero firmado, alineación,
argumentos en registros/pila, estructuras, retornos fallables, limpieza parcial,
callbacks, atomics y cancelación. Toda transformación optimizadora nueva añade
un par discriminante y vuelve a contrastar la ruta entera.

El PASS de un destino autoriza modo de desarrollo solo si también cumple
T26/roadmap §4.3, sin quitar comprobaciones. Si falta semántica, no se distribuye;
si falta rendimiento de producción, LLVM permanece como opción explícita
de optimización. El modo usado figura en todo artefacto/informe y en caché.

**R39.** LLVM se retira del árbol, distribución y construcción normal en un
solo corte cuando los cuatro destinos propios y Wasm hayan cerrado R38;
el backend propio satisfaga roadmap §4.3, roadmap §7.6 y las tres hipótesis H-PY/H-RS/H-CS
con datos nuevos donde el cambio invalide los anteriores; estén cerrados
ABI/interop, biblioteca, herramientas y proyectos externos afectados; y
R24/R27 vuelvan a cerrar con el backend propio. H-RS/H-CS incluyen herramientas
y reparación humana, no son meros benchmarks del emisor. Se archivan LLVM y
sus oráculos para auditoría, sin dependencia obligatoria ni selector residual.

**R40.** Independencia final exige además enlazador Nexo, runtime/biblioteca
D16 en Nexo, inventario transitivo sin Rust/LLVM/Cranelift/libc/CRT salvo
la lista macOS siguiente, y construir
la siguiente versión desde la anterior en máquinas limpias con solo las
dependencias permitidas por R03. Examinar extensiones de archivo no basta:
se registran procesos ejecutados, símbolos importados y bibliotecas cargadas.
Los módulos C/Python voluntarios se prueban aparte y no se infiltran en std
ni en la construcción del compilador.

macOS permite exclusivamente estas APIs documentadas de libSystem:
mmap/munmap, open/read/write/close/mkdir/rename/unlink/rmdir/lstat/getcwd,
pipe/fcntl/poll, clock_gettime,
getentropy, posix_spawn/posix_spawn_file_actions_init/
posix_spawn_file_actions_adddup2/posix_spawn_file_actions_addclose/
posix_spawn_file_actions_destroy, waitpid/kill, _exit, __error; H2 añade
dlopen/dlsym/dlclose/dlerror; R45 limita H2 a dylibs firmadas, sin JIT anónimo. F5 añade pthread_create/
pthread_join/pthread_mutex_init/pthread_mutex_destroy/pthread_mutex_lock/
pthread_mutex_unlock/pthread_cond_init/pthread_cond_destroy/pthread_cond_wait/
pthread_cond_signal/pthread_cond_broadcast/pthread_key_create/pthread_key_delete/
pthread_getspecific/pthread_setspecific y socket/bind/listen/accept/connect/
recv/send/shutdown/getsockopt/setsockopt. Los argumentos y el entorno de Args
proceden del punto de entrada; el directorio inicial se obtiene con getcwd.
Las dependencias internas de libSystem se registran por imagen/versión; no
autorizan importar otros símbolos desde Nexo. Se excluyen malloc/free, stdio,
strings/formato, colecciones, JSON y TLS: siguen escritos en Nexo.
La excepción responde a la ABI estable documentada de macOS en libSystem,
sin asumir syscalls directas estables. M05/M11/M15 y N07a/N14a fijan disponibilidad
por SDK/OS y motivo por símbolo; N23 audita imports, enlaces débiles, resolución
dinámica y bibliotecas cargadas. Fuera de lista bloquea independencia.
mkdir/rename crean directorios y publican salidas del driver; unlink/rmdir
implementan FileRemove D16 R22; el driver elimina únicamente temporales
propios declarados. getcwd captura el directorio inicial para Args/manifiesto
en buffer del llamador, sin F_GETPATH; no se usa fcntl para consultar rutas.
lstat solo implementa FileKind sin seguir enlaces, para FileMakeDirectory
idempotente y comprobación previa de FileRemove; no expone metadatos generales.
Seek, fstat y flush quedan fuera del catálogo y de estas listas. Sin
directorios de salida
creables, N07a no acredita construcción desde un directorio vacío.

Windows x86-64 permite exclusivamente la siguiente lista de APIs nativas,
con imports por nombre/imagen fijados al SDK de cada manifiesto:

| Dominio / primera entrega | Símbolos permitidos |
|---|---|
| VM y errores, M05/M11 | VirtualAlloc, VirtualFree, GetSystemInfo, GetLastError, ExitProcess; VM de datos/guardas sin permiso de ejecución |
| Archivos/directorios/consola, M11 | CreateFileW, ReadFile, WriteFile, CloseHandle, GetFileAttributesW, CreateDirectoryW, MoveFileExW, DeleteFileW, RemoveDirectoryW, GetStdHandle, GetConsoleMode, ReadConsoleW, WriteConsoleW |
| Entrada/relojes, M11 | GetCommandLineW, GetCurrentDirectoryW, GetEnvironmentStringsW, FreeEnvironmentStringsW, QueryPerformanceCounter, QueryPerformanceFrequency, GetSystemTimePreciseAsFileTime |
| Entropía, M15 | BCryptGenRandom con proveedor del sistema; no hashing/TLS de BCrypt |
| Proceso, N07a | CreateProcessW, CreatePipe, SetHandleInformation, PeekNamedPipe, WaitForSingleObject, GetExitCodeProcess, TerminateProcess, InitializeProcThreadAttributeList, UpdateProcThreadAttribute, DeleteProcThreadAttributeList |
| Carga H2, N14a | LoadLibraryExW, GetProcAddress, FreeLibrary; exclusivamente imágenes generacionales R45, sin VirtualAlloc ejecutable ni parche de instrucciones |
| Hilos/sincronización/TLS de hilo, F5 | CreateThread, WaitForMultipleObjects, InitializeSRWLock, AcquireSRWLockExclusive, ReleaseSRWLockExclusive, AcquireSRWLockShared, ReleaseSRWLockShared, InitializeConditionVariable, SleepConditionVariableSRW, WakeConditionVariable, WakeAllConditionVariable, TlsAlloc, TlsFree, TlsGetValue, TlsSetValue |
| Red, M15/F5 | WSAStartup, WSACleanup, WSAGetLastError, socket, bind, listen, accept, connect, recv, send, shutdown, closesocket, getsockopt, setsockopt, ioctlsocket, WSAPoll |

Las imágenes de importación son Kernel32/KernelBase y API sets documentados
para las filas de sistema, BCrypt para entropía y Ws2_32 para red; forwarders
y dependencias internas se registran por versión, sin habilitar imports Nexo
arbitrarios a Ntdll ni UCRT/MSVCRT. ConsoleRead/Write distinguen consola
mediante GetConsoleMode: usan ReadConsoleW/WriteConsoleW con unidades UTF-16LE
en pares U8 y buffers alineados a dos bytes, según la firma cruda D16 R22;
para redirección usan ReadFile/WriteFile con bytes. La conversión UTF-8/UTF-16
es Nexo y cobra sus recorridos; SetConsoleOutputCP no es necesario porque
no se usa la variante ANSI ni se modifica la página global. Se conserva
el número real de unidades leídas/escritas, incluidos pares suplentes y
salidas parciales. Esta selección sigue el contrato de
[WriteConsole de Microsoft](https://learn.microsoft.com/en-us/windows/console/writeconsole).
Parsear argumentos, UTF-16/rutas, DNS,
colecciones y TLS sigue siendo Nexo. GetFileAttributesW implementa únicamente
FileKind: reparse point es Link, DIRECTORY es Directory, un archivo ordinario
es Regular. FileKind en Windows admite solo rutas de archivos de volumen
local o UNC (incluida su forma extendida), no espacios de dispositivos,
pipes ni GLOBALROOT; el wrapper los rechaza antes de consultar con
Err(ERROR_INVALID_NAME=123). No emite Other en ese destino. Un estado de
atributos no clasificable (incluido DEVICE, reservado) devuelve error de
estado no soportado (ERROR_NOT_SUPPORTED=50), no inventa un Regular.
La regresión especial Windows es ese rechazo, no un Other positivo.
MoveFileExW publica en el mismo volumen con
MOVEFILE_REPLACE_EXISTING, sin MOVEFILE_COPY_ALLOWED. LoadLibraryExW recibe ruta absoluta más
LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR y LOAD_LIBRARY_SEARCH_SYSTEM32; el inventario
cierra dependencias del directorio y del sistema, sin PATH/CWD ni DLL ajena.
Esta selección sigue la [API documentada por Microsoft](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw);
no constituye un selector de carga arbitraria de std.

Los símbolos pthread/CreateThread/SRW/TLS de hilo/WSA y los símbolos BSD
socket/bind/listen/accept/connect/recv/send/shutdown/getsockopt/setsockopt
de macOS marcados F5 solo se
habilitan tras la revisión de catálogo de M15 que enumere sus selectores,
firmas, efectos, costes y proveedores, conforme D16 R22.
Las listas son autoridad única para D16 R22/D16 R26 y para N07a/N14a.
Añadir símbolo exige enmienda y evidencia, incluso si otra API permitida lo
resuelve dinámicamente. N23 audita las tres plataformas, incluidos los manifiestos
Linux por ISA. Los canales Linux de ProcessRun permiten exclusivamente las
syscalls pipe2, fcntl, ppoll, dup3, read, write y close: creación con cierre
en exec, no bloqueo/flags de descriptor, sondeo, duplicación a stdout/stderr,
lectura de salida, escritura del estado de fallo de lanzamiento y cierre,
respectivamente. fcntl solo admite F_GETFL/F_SETFL/F_GETFD/F_SETFD en esa
receta; el sellado H2 usa los comandos separados de ImageSeal. El arranque,
ejecución, recolección y terminación del hijo permiten exclusivamente clone,
execve, wait4, kill y exit en ambos Linux; clone crea un proceso separado,
sin compartir memoria ni tabla de descriptores, con señal de salida SIGCHLD.
exit termina el hijo si execve falla, tras escribir el estado al canal.
La lista de operaciones es cerrada en los cuatro destinos; N07a fija solo
números, convención y flags de esa receta por ISA, sin añadir syscalls.
Ninguna llamada interna se publica como selector adicional. Esta selección
usa los nombres de las tablas del kernel [x86-64](https://raw.githubusercontent.com/torvalds/linux/master/arch/x86/entry/syscalls/syscall_64.tbl)
y [ABI genérica de 64 bits](https://raw.githubusercontent.com/torvalds/linux/master/include/uapi/asm-generic/unistd.h);
N07a fija números/convenciones y versión mínima por destino.
Linux usa syscalls directas para FileRemove (unlink/rmdir o
unlinkat con flags fijos equivalentes), FileKind (newfstatat en ambas ISA
con AT_SYMLINK_NOFOLLOW) y getcwd, sin libc ni dlopen. En ambos Linux,
FileOpen usa openat, FileMakeDirectory mkdirat y FileRename renameat;
los argumentos de directorio son AT_FDCWD y las rutas ya validadas por M11.
Los modos de apertura se traducen al conjunto cerrado de flags del selector,
sin añadir operaciones de archivo. renameat conserva reemplazo atómico en
el mismo volumen y devuelve error entre volúmenes, sin copia de respaldo.
ClockMonotonic y ClockWall usan clock_gettime con CLOCK_MONOTONIC y
CLOCK_REALTIME, respectivamente; Entropy usa getrandom con flags=0 y
conserva errores/lecturas parciales; Abort usa exit_group para terminar el
proceso completo. Args toma argumentos y entorno del punto de entrada,
y getcwd aporta solo el directorio inicial. El estado de error se obtiene
del retorno de syscall, sin símbolo auxiliar ni errno de libc. M05/M11/M15
fijan números, convención y versión mínima por ISA para estas mismas
operaciones; no pueden ampliar la lista mediante un manifiesto.
La red Linux F5 permite socket/bind/listen/accept/connect/recvfrom/sendto/
shutdown/getsockopt/setsockopt y close bajo la revisión de catálogo de M15;
SocketRecv/SocketSend usan recvfrom/sendto sin dirección remota adicional
en el socket conectado. Opciones/estado/cierre son pasos privados de esas
recetas, como en las otras plataformas, sin selector genérico. Las tareas,
sincronización y TLS de hilo siguen sin selector activo hasta M15 en los
tres sistemas; sus syscalls Linux requieren la misma enmienda y evidencia
que cualquier símbolo añadido. N14a
habilita mmap/munmap/mprotect para imágenes ELF de R45; PROT_EXEC solo para
segmentos de archivo verificados, nunca para memoria anónima ni páginas W+X.
N14a añade memfd_create y F_ADD_SEALS/F_GET_SEALS de fcntl exclusivamente
para la imagen privada R45; no habilita otros comandos de fcntl ni carga de código arbitrario.
La excepción macOS es propuesta a acuerdos §9.3(a) y roadmap §4.2.

La atribución siguiente cubre los símbolos permitidos anteriores y las
hojas R45; no amplía esas listas ni habilita las filas condicionales F5.
La notación C09.X abrevia C09.SystemCall[selector=X]. Cada receta con
varias hojas registra cuál invoca el auxiliar; compartir una API de OS no
crea una segunda implementación de la capacidad. El propietario de despacho
C09 es M11; propietarios de selectores son D16 R53 y D17 R04.

| Target | Símbolos de proveedor | Capacidad y propietario |
|---|---|---|
| macOS/Linux | `mmap`, `munmap` | C04.Alloc/Free (M05); Linux ImageReserve/MapFile/Unmap (N14a) |
| macOS | `open`, `read`, `write`, `close`, `mkdir`, `rename`, `unlink`, `rmdir`, `lstat`, `getcwd` | C09.FileOpen/FileRead/FileWrite/FileClose/FileMakeDirectory/FileRename/FileRemove/FileKind/Args; read/write también C09.ConsoleRead/ConsoleWrite (M11); read/close también canales de C09.ProcessRun (N07a); close también C09.SocketClose (M15/F5) |
| macOS | `pipe`, `fcntl`, `poll`, `posix_spawn`, `posix_spawn_file_actions_init`, `posix_spawn_file_actions_adddup2`, `posix_spawn_file_actions_addclose`, `posix_spawn_file_actions_destroy`, `waitpid`, `kill` | C09.ProcessRun (N07a); canales, acciones y recolección privados de esa operación |
| macOS | `clock_gettime` | C09.ClockMonotonic/ClockWall (M11) |
| macOS | `getentropy` | C09.Entropy (M15) |
| macOS | `_exit` | C10.Abort (M05) |
| macOS | `__error` | Estado de error auxiliar de C04 (M05), selectores C09 de M11/M15 y ProcessRun (N07a); no hoja pública adicional |
| macOS | `dlopen`, `dlsym`, `dlclose`, `dlerror` | C09.NativeImageOpen/NativeImageLookup/NativeImageClose, estado auxiliar dlerror (N14a) |
| macOS/F5 | `pthread_create`, `pthread_join`, `pthread_mutex_init`, `pthread_mutex_destroy`, `pthread_mutex_lock`, `pthread_mutex_unlock`, `pthread_cond_init`, `pthread_cond_destroy`, `pthread_cond_wait`, `pthread_cond_signal`, `pthread_cond_broadcast`, `pthread_key_create`, `pthread_key_delete`, `pthread_getspecific`, `pthread_setspecific` | C09 tareas/sincronización/TLS de hilo, catálogo de M15 pendiente de activación D16 R22; sin selector utilizable en v1 |
| macOS/F5 | `socket`, `bind`, `listen`, `accept`, `connect`, `recv`, `send`, `shutdown`, `getsockopt`, `setsockopt` | C09.SocketOpen/SocketBind/SocketListen/SocketAccept/SocketConnect/SocketRecv/SocketSend/SocketClose (M15); opciones/estado/cierre son pasos internos de esas recetas, no operaciones generales; close libera el descriptor de SocketClose |
| Windows | `VirtualAlloc`, `VirtualFree`, `GetSystemInfo` | C04.Alloc/Free, geometría de páginas de Alloc (M05) |
| Windows | `GetLastError` | Estado de error auxiliar de C04 (M05), selectores C09 de M11/M15, C09.ProcessRun (N07a) y C09.NativeImageOpen/NativeImageLookup/NativeImageClose (N14a); no hoja pública adicional |
| Windows | `ExitProcess` | C10.Abort (M05) |
| Windows | `CreateFileW`, `ReadFile`, `WriteFile`, `CloseHandle`, `GetFileAttributesW`, `CreateDirectoryW`, `MoveFileExW`, `DeleteFileW`, `RemoveDirectoryW`, `GetStdHandle`, `GetConsoleMode`, `ReadConsoleW`, `WriteConsoleW` | C09.FileOpen/FileRead/FileWrite/FileClose/FileKind/FileMakeDirectory/FileRename/FileRemove y C09.ConsoleRead/ConsoleWrite (M11); CloseHandle/ReadFile también limpian/leen canales internos C09.ProcessRun (N07a) |
| Windows | `GetCommandLineW`, `GetCurrentDirectoryW`, `GetEnvironmentStringsW`, `FreeEnvironmentStringsW` | C09.Args, captura y liberación del entorno inicial (M11) |
| Windows | `QueryPerformanceCounter`, `QueryPerformanceFrequency`, `GetSystemTimePreciseAsFileTime` | C09.ClockMonotonic/ClockWall (M11) |
| Windows | `BCryptGenRandom` | C09.Entropy (M15) |
| Windows | `CreateProcessW`, `CreatePipe`, `SetHandleInformation`, `PeekNamedPipe`, `WaitForSingleObject`, `GetExitCodeProcess`, `TerminateProcess`, `InitializeProcThreadAttributeList`, `UpdateProcThreadAttribute`, `DeleteProcThreadAttributeList` | C09.ProcessRun, preparación y recolección del hijo (N07a); espera de hilo F5 usa la misma API bajo catálogo M15 |
| Windows | `LoadLibraryExW`, `GetProcAddress`, `FreeLibrary` | C09.NativeImageOpen/NativeImageLookup/NativeImageClose (N14a) |
| Windows/F5 | `CreateThread`, `WaitForMultipleObjects`, `InitializeSRWLock`, `AcquireSRWLockExclusive`, `ReleaseSRWLockExclusive`, `AcquireSRWLockShared`, `ReleaseSRWLockShared`, `InitializeConditionVariable`, `SleepConditionVariableSRW`, `WakeConditionVariable`, `WakeAllConditionVariable`, `TlsAlloc`, `TlsFree`, `TlsGetValue`, `TlsSetValue` | C09 tareas/sincronización/TLS de hilo, catálogo de M15 pendiente de activación D16 R22; sin selector utilizable en v1 |
| Windows/F5 | `WSAStartup`, `WSACleanup`, `WSAGetLastError`, `socket`, `bind`, `listen`, `accept`, `connect`, `recv`, `send`, `shutdown`, `closesocket`, `getsockopt`, `setsockopt`, `ioctlsocket`, `WSAPoll` | C09.SocketOpen/SocketBind/SocketListen/SocketAccept/SocketConnect/SocketRecv/SocketSend/SocketClose (M15); inicialización/estado/opciones/sondeo son pasos internos de las recetas de red |
| Linux | `unlink`, `rmdir`, `unlinkat`, `newfstatat`, `getcwd` | C09.FileRemove/FileKind/Args (M11), según ISA y flags fijos R40 |
| Linux | `pipe2`, `fcntl`, `ppoll`, `dup3`, `read`, `write`, `close` | C09.ProcessRun (N07a), exclusivamente canales y comandos R40; read/write/close también C09.FileRead/FileWrite/FileClose y read/write C09.ConsoleRead/ConsoleWrite (M11); close también C09.SocketClose (M15) |
| Linux | `clone`, `execve`, `wait4`, `kill`, `exit` | C09.ProcessRun (N07a), creación/ejecución/recolección/terminación y fallo de lanzamiento según R40; exit solo termina el hijo de esa receta si falla execve |
| Linux | `openat`, `mkdirat`, `renameat` | C09.FileOpen/FileMakeDirectory/FileRename (M11); FileOpen también es consumido por el controlador N14a, sin cambiar su dueño |
| Linux | `clock_gettime` | C09.ClockMonotonic/ClockWall (M11) |
| Linux | `getrandom` | C09.Entropy (M15) |
| Linux | `exit_group` | C10.Abort (M05); distinto de exit del hijo ProcessRun |
| Linux/F5 | `socket`, `bind`, `listen`, `accept`, `connect`, `recvfrom`, `sendto`, `shutdown`, `getsockopt`, `setsockopt`, `close` | C09.SocketOpen/SocketBind/SocketListen/SocketAccept/SocketConnect/SocketRecv/SocketSend/SocketClose (M15); opciones/estado/cierre privados de las recetas, solo tras revisión y activación F5 |
| Linux/H2 | `memfd_create`, `fcntl`, `mmap`, `mprotect`, `munmap` | C09.ImageCreate/ImageSeal/ImageReserve/ImageMapFile/ImageProtect/ImageUnmap (N14a), solo comandos y permisos R45 |

La matriz siguiente cubre las capacidades de sistema por plataforma;
Linux incluye x86-64 y arm64. Cada celda remite a símbolos de las listas
anteriores, a representación sin syscall o a una exclusión explícita.
Las filas de red siguen condicionadas a M15/F5 y las de carga a N14a/H2;
la matriz no adelanta su activación. Los auxiliares de cada receta y sus
restricciones son los de la tabla de atribución, sin nuevos imports.

| Capacidad / dueño | Linux | macOS | Windows |
|---|---|---|---|
| C04.Alloc/Free / M05 | mmap/munmap | mmap/munmap | VirtualAlloc/VirtualFree/GetSystemInfo |
| FileOpen / M11 | openat | open | CreateFileW |
| FileRead / M11 | read | read | ReadFile |
| FileWrite / M11 | write | write | WriteFile |
| FileClose / M11 | close | close | CloseHandle |
| FileMakeDirectory / M11 | mkdirat | mkdir | CreateDirectoryW |
| FileRename / M11 | renameat | rename | MoveFileExW |
| FileKind / M11 | newfstatat (ambas ISA) | lstat | GetFileAttributesW |
| FileRemove / M11 | unlink/rmdir (x86-64) o unlinkat (ambas ISA) | unlink/rmdir | DeleteFileW/RemoveDirectoryW |
| ConsoleRead / M11 | read | read | GetStdHandle/GetConsoleMode/ReadConsoleW/ReadFile |
| ConsoleWrite / M11 | write | write | GetStdHandle/GetConsoleMode/WriteConsoleW/WriteFile |
| Args / M11 | Punto de entrada y getcwd | Punto de entrada y getcwd | GetCommandLineW/GetCurrentDirectoryW/GetEnvironmentStringsW/FreeEnvironmentStringsW |
| ClockMonotonic / M11 | clock_gettime | clock_gettime | QueryPerformanceCounter/QueryPerformanceFrequency |
| ClockWall / M11 | clock_gettime | clock_gettime | GetSystemTimePreciseAsFileTime |
| Entropy / M15 | getrandom | getentropy | BCryptGenRandom |
| C10.Abort / M05 | exit_group | _exit | ExitProcess |
| ProcessRun / N07a | pipe2/fcntl/ppoll/dup3/read/write/close/clone/execve/wait4/kill/exit | pipe/fcntl/poll/posix_spawn y acciones/read/close/waitpid/kill | CreateProcessW y auxiliares de canales/atributos/espera/terminación de su fila |
| SocketOpen / M15 | socket | socket | WSAStartup/socket |
| SocketBind / M15 | bind | bind | bind |
| SocketListen / M15 | listen | listen | listen |
| SocketAccept / M15 | accept | accept | accept |
| SocketConnect / M15 | connect | connect | connect |
| SocketRecv / M15 | recvfrom | recv | recv |
| SocketSend / M15 | sendto | send | send |
| SocketClose / M15 | shutdown/close | shutdown/close | shutdown/closesocket/WSACleanup |
| ImageCreate / N14a | memfd_create | Excluida: NativeImageOpen | Excluida: NativeImageOpen |
| ImageSeal / N14a | fcntl F_ADD_SEALS/F_GET_SEALS | Excluida: imagen inmutable firmada R45 | Excluida: imagen inmutable R45 |
| ImageReserve / N14a | mmap PROT_NONE | Excluida: reserva del cargador nativo | Excluida: reserva del cargador nativo |
| ImageMapFile / N14a | mmap de archivo | Excluida: mapping del cargador nativo | Excluida: mapping del cargador nativo |
| ImageEntry / N14a | Representación ABI, sin syscall | Excluida: NativeImageLookup | Excluida: NativeImageLookup |
| ImageProtect / N14a | mprotect RELRO RW→R | Excluida: permisos del cargador nativo | Excluida: permisos del cargador nativo |
| ImageUnmap / N14a | munmap | Excluida: NativeImageClose | Excluida: NativeImageClose |
| NativeImageOpen / N14a | Excluida: controlador ELF Nexo | dlopen/dlerror | LoadLibraryExW |
| NativeImageLookup / N14a | Excluida: controlador ELF Nexo | dlsym/dlerror | GetProcAddress |
| NativeImageClose / N14a | Excluida: controlador ELF Nexo | dlclose/dlerror | FreeLibrary |
| Tareas/sincronización/TLS de hilo / M15 | Sin selector activo; catálogo y símbolos pendientes de enmienda M15 | Sin selector activo; lista pthread condicional F5 | Sin selector activo; lista de hilos/SRW/TLS condicional F5 |

La auditoría de proveedores exige una regresión por capacidad y destino:
la receta positiva usa exclusivamente símbolos atribuidos a su dueño;
el negativo elimina solo esa atribución o introduce un símbolo fuera de
lista y debe bloquear R40 aunque la operación funcione. En Linux se incluyen
por separado FileOpen/openat, FileMakeDirectory/mkdirat, FileRename/renameat,
ClockMonotonic y ClockWall/clock_gettime, Entropy/getrandom y Abort/exit_group.
La lectura de imagen de N14a consume FileOpen de M11, sin apropiarse de él.

ImageEntry de Linux es representación ABI de N14a, sin símbolo del sistema.
C01/C02/C03/C05/C07 usan instrucciones o IR, sin imports externos propios;
ForeignCall de M13c solo acepta símbolos de manifiestos optativos declarados
y no habilita imports extra de std. C06 carece de operaciones y no admite proveedor.
La auditoría exige que cada símbolo importado/resuelto esté en la lista
permitida de su target **y** tenga atribución a una capacidad activa con
dueño; una fila futura pendiente no satisface esa condición.

**R41.** El adaptador experimental Cranelift de D04 se construye y mide en F2
como manda la base. Dos fallos reproducibles de puerta con ≥60% en generación/
enlace activan la comparación; si domina inferencia se corrige inferencia.
Solo puede promoverse antes de N13, con corpus completo y métricas aplicables
por destino; promoción significa habilitación transitoria de desarrollo,
nunca compromiso de conservar la crate tras autoalojamiento. T26 anticipada
no aprueba F6. Antes de activar N13 se reconstruyen los mismos destinos con
LLVM Nexo y se cierran R28/R30 frente a la referencia promovida, además de
la referencia Rust. Si LLVM no cubre esa semántica/rendimiento, se difiere
N13 conforme R51; no se retira el backend usado para afirmar la puerta.
N13 retira toda crate, wrapper y configuración Cranelift en su corte. Después de N13 hasta
E2E-F6 la contingencia es corregir LLVM o diferir bajo R53; no reintroduce
Rust ni adelanta el backend propio.
LLVM y el propio comparten IR/frontend/runtime; su convivencia temporal es
la excepción explícita requerida por acuerdos §9.2, no permiso para duplicar
el lenguaje ni para conservar adaptadores después de R39.

## 7. Recarga en caliente

**R42.** Se confirman H0 en F2; H1 después de E2E-F2 en intérprete de desarrollo;
H2 después de cerrar el backend nativo de desarrollo que lo ejecute; H3 solo
tras un ADR específico de migración. H2 puede usar LLVM antes del backend
propio. El runtime y la gestión de generaciones son Nexo; en Linux también
lo es el cargador ELF N14a, exigido antes de cualquier H2. En macOS/Windows
los selectores NativeImageOpen/Lookup/Close de R45, con proveedores R40, son la frontera permitida incluso en la
independencia final; no se exige reimplementar dyld/LoadLibraryExW. F0 no
implementa recarga. I04 aporta IR y FunctionId, no especifica ni acredita
identidad entre ediciones, body_hash/interface_hash/dependency_hash por función, tabla sustituible ni puntos
seguros. R09–R11 precisan esas obligaciones; N02 implementa tabla, DefKey/body_hash/interface_hash/dependency_hash/clave de artefacto,
layouts y ABI. M02/M03/M05 consumen sus descriptores; N14 extiende esa misma
tabla con generaciones H1/H2 según R45. D16 IR01/IR02 y D16 R37 son consumidores,
sin tabla adicional para biblioteca o recarga. Antes de H2 se exige una ampliación revisada del catálogo
D16 R22 para carga de imágenes/protecciones y coherencia de instrucciones
por target, concretada en R45; no habilita JIT anónimo. N14a posee esa ampliación antes de N14b (H2 LLVM) y N21 (H2 propio), con
pre/postcondiciones, efectos y evidencia nativa; no se supone incluido en
VmReserve ni se introduce una llamada extranjera para saltarse esa frontera.

**R43.** Para declaraciones existentes, H1/H2 aceptan únicamente cambio de
cuerpo o valor de constante conforme R47, con misma identidad,
firma, modos, contratos, procedencias, afinidad, layout, ABI, catálogo
requerido, datos públicos D13 §12 y resumen de efectos inferidos. Decide
interface_hash R10 confirmado por igualdad de registros,
incluido el resumen de efectos inferidos definido en R10; body_hash distinto solo es aplicable con interfaz
idéntica. Los efectos se recomputan antes de clasificar: cualquier cambio,
incluso reducción, exige contracts_changed y changed_fields=[effects] si es
la única diferencia. Cambiar un
contrato, aun debilitando efectos, requiere reinicio en esta primera versión.
Todos los llamadores afectados se revalidan antes de clasificar. Si la fuente
nueva es inválida, se rechaza con los códigos ordinarios y se conserva la
sesión anterior: reiniciar no convierte un programa inválido en válido.

**R44.** La instalación es una transacción en un punto seguro: se detienen
nuevas entradas al conjunto afectado y se espera quiescencia de sus frames,
préstamos y regiones dependientes. Los frames activos terminan con la versión
anterior, incluidas sus llamadas transitivas a esa generación; no se les
cambia la tabla a mitad de ejecución. La regla 3 de acuerdos §9.8 rige hasta
que un ADR propio defina la transformación de los bucles permanentes
elegibles (nivel H1b). El presupuesto es 1.000.000 pasos IR
instrumentados del conjunto activo, ordenados por (clave de tarea, número de
paso). La clave de tarea es el ordinal natural asignado por el planificador
de desarrollo al crear cada tarea, desde cero y sin reutilización durante
la sesión; el frame raíz recibe la primera clave. Las creaciones siguen el
orden determinista de despacho; número de paso es el ordinal natural desde
cero dentro de la tarea. Ningún ordinal admite wrap ni reutilización al
agotar su representación: se detiene la sesión. Cada operación IR, backedge y llamada
cuenta una vez, independientemente de optimización/reloj. El intento 1.000.001
sin quiescencia conserva la sesión y emite quiescence_budget_exhausted R47.
Un frame dentro de C08 ForeignCall sin prueba de retiro o de C09
SystemCall con efecto block (incluido ReadLine→ConsoleRead) fuerza
unsafe_boundary **antes de esperar**. Lo mismo ocurre si un handle
extranjero retiene un callback de la generación sin prueba de retiro, aunque
no haya frame ni tarea ejecutándolo (B23). Un frame detenido no consume pasos IR mientras está
bloqueado y no se puede esperar a que agote ese contador. El despacho de
C09 publica entrada/salida de frontera atómicamente con el protocolo de
quiescencia; si intenta entrar en block después de solicitar recarga,
se rechaza la recarga antes de ceder control al sistema. No se cambia la
operación de usuario ni se la cancela. El conjunto activo incluye el cierre
transitivo de dependencias de espera: si un frame afectado espera una tarea
no afectada detenida en C09 block o C08 sin retiro, también se clasifica
unsafe_boundary antes de esperar. live_kinds incluye frame, task y la
frontera encontrada. Tras descartar unsafe_boundary, el planificador toma
una instantánea atómica del cierre de espera y aplica este criterio operativo:
para cada tarea suspendida debe existir un camino de aristas de espera hasta
una tarea ejecutable o hasta un evento de reanudación ya encolado dirigido a
esa tarea o a una dependencia suya. Ejecutable significa que puede despachar
su próxima operación IR sin esperar otro recurso. Un evento admisible tiene
destino, orden y entrega finita fijados por el planificador determinista;
no depende de una futura acción externa, reloj de pared ni señal aún no
registrada. El planificador entrega los eventos así encolados en su orden
antes de declarar ausencia de progreso; puede avanzar su reloj lógico al
siguiente evento, sin esperar tiempo físico. Una tarea terminada satisface
su arista y se retira del cierre. Toda tarea suspendida sin ese camino,
incluida una SCC de espera sin salida ejecutable/evento, produce
live_dependency inmediatamente, con los kinds vivos del cierre testigo.
Una tarea ejecutable ajena a ese cierre no acredita progreso.
El chequeo se repite tras cada suspensión, consumo de evento o cambio de
arista; un bucle que sí ejecuta IR sigue sujeto al millón de pasos. Los
eventos no fabrican pasos IR, y su número pendiente debe ser finito en cada
instantánea. Entregar un evento solo consume su registro y habilita o
satisface su destino: no ejecuta código de usuario ni encadena eventos
nuevos sin ejecutar IR contabilizada. El planificador registra aristas, ejecutabilidad y eventos
atómicamente con el protocolo; nunca espera a que una tarea detenida consuma
el presupuesto. B37 fija el negativo y su evento positivo pareado.
live_kinds incluye system_call para
C09 y foreign_callback para C08, según el registro vivo. No se fuerza
cancelación ni se promete plazo de finalización del código de usuario.

Generadores/tareas suspendidos cuyo cuerpo o dependencias de estado cambien
cuentan como dependencia viva aunque el tamaño del frame coincida. H1/H2
requieren terminar/destruir explícitamente esos estados o reiniciar; nunca
reinterpreta locales ni migra estado automáticamente. H3 no está habilitado:
cualquier cambio de layout con objetos vivos requiere reinicio. El futuro
ADR de H3 deberá definir migración explícita fallable, limpieza, atomicidad
y recuperación antes de levantar ese rechazo.

**R45.** N14 es propietario del versionado de la tabla N02 para **H1 y H2**.
Cada generación inmutable contiene DefKey/instancia, body_hash/interface_hash/dependency_hash de R10, descriptor
ABI/layout, destino (función VerifiedIr en H1 o entrada nativa en H2) y
dependencias de código/datos. Hay una única raíz publicada por sesión.
Un frame fija su generación al entrar y la transmite a llamadas transitivas;
capturas/estados guardan esa referencia. Referencias activas cuentan frames,
préstamos, estados suspendidos, callbacks y handles de generación; no basta
un contador de llamadas superiores. Nueva generación se verifica completa
fuera de la raíz activa; en quiescencia R44 se intercambia la raíz una vez,
atómicamente. Un fallo conserva la anterior. Solo se descarga código con
cero referencias activas y cero dependientes; se invalida inlining del cierre.
N14a/N14b/N21 consumen este protocolo, sin otra tabla o runtime.

H2 exige contadores de pasos del IR lógico **instrumentados en los binarios
nativos**, con mapas de origen conservados tras optimización. Una instrucción
fusionada paga su secuencia lógica, sin contar dos veces llamada/backedge que
ya son una operación; B26/B27 se ejecutan en H1 y en cada destino H2 LLVM/propio,
con resultados idénticos al variar velocidad física. No existe timeout de 5 s
ni criterio temporal de clasificación. Producción no habilita esta indirección
ni instrumentación por defecto.

H2 carga una biblioteca nativa inmutable por generación: dyld en macOS,
LoadLibraryExW en Windows y cargador ELF escrito en Nexo en Linux x86-64/arm64,
entregado por N14a antes de habilitar H2; no es un JIT de memoria anónima.
En macOS arm64 se emite Mach-O dylib con secciones R-X/R--/RW-, se firma
externamente antes de dlopen y se conserva el producto sin firma para R24.
La firma usa ProcessRun bajo manifiesto R22.
El perfil de desarrollo admite firma ad hoc sin hardened runtime; si se usa
hardened runtime exige firma compatible con la validación de bibliotecas
del host. Firma o política incompatible rechaza la carga antes de instalar,
sin desactivar la protección. Esa limitación es explícita en N14b/N18/N21.
En macOS mmap/munmap sirven a Alloc/Free de C04, con permisos de datos/guardas
fijados al reservar según D16 R18; mprotect queda fuera de la lista macOS.
Windows tampoco permite VirtualProtect: Alloc reserva las guardas sin
comprometerlas. Ninguna de esas capacidades mapea código anónimo. **MAP_JIT y pthread_jit_write_protect_np quedan excluidos**:
su modelo de permiso de escritura por hilo no es el contrato elegido de
páginas nunca W+X. No se requiere entitlement allow-jit.
La ruta JIT distinta descrita por
[Apple](https://developer.apple.com/documentation/apple-silicon/porting-just-in-time-compilers-to-apple-silicon)
necesitaría otra decisión; no se presume soportada por H2.

Windows usa DLL por generación y LoadLibraryExW con ruta absoluta y búsqueda
cerrada R40. Linux usa ELF64 ET_DYN PIC del emisor Nexo/LLVM y manifiesto
cerrado; no invoca ld.so, dlopen ni una biblioteca libc. El cargador Nexo
valida identidad/ABI, tamaños, rangos, alineación, solapamientos y permisos
de segmentos antes de mapear. Lee/verifica el archivo completo con FileRead
hasta EOF y compara longitud y file_digest de la imagen registrados en el manifiesto R10/R45; no requiere lseek/fstat.
El controlador Nexo conserva los bytes completos ya verificados en almacenamiento
propietario y crea un ImageFile privado mediante ImageCreate. Escribe esos
bytes secuencialmente con FileWrite, sin otro escritor ni mapping. Tras cada
éxito parcial acumula únicamente la longitud confirmada como escrita y
continúa por el sufijo pendiente del búfer verificado. Error, longitud
devuelta mayor que la solicitada o cero bytes con datos pendientes aborta
la carga y cierra ImageFile una vez. Al terminar exige longitud acumulada
igual a la longitud del manifiesto. La evidencia de la copia es el contrato
de FileWrite y sus longitudes confirmadas sobre ese búfer inmutable; no
calcula otro SHA-256 del mismo búfer como supuesta observación del memfd.
Invoca ImageSeal y comprueba status y seals según su contrato inferior;
solo entonces libera el búfer y permite ImageReserve/ImageMapFile.
Conserva el descriptor sellado hasta la descarga. No intenta FileRead sobre
ese descriptor situado en EOF. La evidencia incluye escrituras parciales,
error intermedio, cero progreso, longitud imposible, file_digest distinto
del archivo original y cada fallo de sellado/observación; ninguno puede
alcanzar ImageMapFile ni publicar una generación.
No se vuelve a abrir la ruta original después de la comprobación. La copia
no se publica ni se ejecuta sin sello; una plataforma/política que no permita
memfd ejecutable sellado rechaza H2, sin fallback a archivo mutable ni libc.
Reserva espacio de direcciones sin ejecución y mapea segmentos de
archivo privados: texto R-X desde el inicio, datos R-- o RW- sin ejecución;
BSS anónimo solo RW-. No escribe texto ni crea alias escribible del código.
Las relocaciones solo afectan datos antes de publicar (RELRO se vuelve R--).
N14a implementa RELATIVE, GLOB_DAT, JUMP_SLOT y 64/ABS64 según x86-64/AArch64,
con binding inmediato a símbolos tipados del inventario; ninguna resolución
por PATH, búsqueda global o DT_NEEDED ajeno. TLS de hilo se referencia mediante
el runtime Nexo común, sin PT_TLS por generación. Se rechazan relocaciones de
texto, IFUNC, inicializadores/finalizadores, formatos o imports no declarados
antes de instalar; el emisor debe producir todas las operaciones del perfil
H2 mediante ese ABI, sin recortar la semántica del lenguaje.
N14a exige en ambos Linux carga/resolución/ejecución/descarga nativas, corpus
de objetos truncados, overflow, solapamiento, W+X, relocación/import inválidos,
fallos de cada reserva y limpieza sin fugas; cualquier fallo conserva la raíz
anterior. N14b/N21 consumen el mismo cargador, sin proveedor transitorio libc.
El emisor y enlazador prohíben secciones W+X y relocaciones que exijan escribir
texto ejecutable. N14a entrega las funciones Nexo LoadGeneration,
ResolveGenerationEntry y ReleaseGeneration, efectos io/block/alloc conservadores,
sin suspend ni callbacks de inicialización/finalización. Solo carga artefactos
Nexo verificados y su inventario, sin inicializadores extranjeros ocultos;
la resolución devuelve entrada tipada con ABI/generación, nunca dirección
convertible a entero. No son hojas irreductibles que oculten un cargador host.

N14a amplía explícitamente C09/D16 R22 con estas hojas Linux; sus firmas
son contratos IR, no sintaxis de fuente. C09.SystemCall es el nombre de la
familia de servicios de plataforma; ImageEntry es una hoja de representación
ABI sin syscall real, identificada así en la tabla, no un algoritmo oculto
de carga. Usan recursos opacos ImageFile,
ImageReservation e ImageMapping, sin conversión a entero. Las operaciones
del controlador (copiar, verificar ELF, resolver símbolos, aplicar relocaciones
de datos y contabilizar generaciones) son funciones Nexo ordinarias.

| ID de selector C09.SystemCall | Contrato crudo, efectos y proveedor Linux |
|---|---|
| [selector=ImageCreate] | Crea ImageFile vacío privado para imagen de archivo ejecutable, sin mapping; devuelve estado y recurso solo en éxito. memfd_create con sellado habilitado y política ejecutable explícita del manifiesto. io/block/alloc; FileWrite y FileClose admiten este handle con sus contratos existentes. |
| [selector=ImageSeal] | edit ImageFile sin mappings; fcntl F_ADD_SEALS añade en una operación F_SEAL_WRITE, F_SEAL_GROW, F_SEAL_SHRINK y F_SEAL_SEAL (máscara requerida 15), y tras su éxito ejecuta fcntl F_GET_SEALS. Devuelve {status:I64,seals:U32}: status=0 solo si ambas llamadas tienen éxito y (seals & 15)=15; seals es la máscara observada, o 0 si no hubo observación válida. Error de syscall devuelve su estado negativo; máscara observada incompleta devuelve -EIO. Solo éxito cambia el recurso a sellado y prohíbe FileWrite; cualquier fallo obliga al controlador a cerrar el recurso una vez y no autoriza ejecución. io/block; ambas llamadas y sus celdas de resultado se contabilizan. |
| [selector=ImageReserve] | Tamaño/alineación comprobados; mmap anónimo PROT_NONE devuelve estado y ImageReservation, sin permiso ejecutable. io/block/alloc; solo reserva direcciones para una imagen, no sustituye Alloc. |
| [selector=ImageMapFile] | edit ImageReservation, view ImageFile sellado, offset/longitud/destino y permiso R/RW/RX. mmap privado de archivo, MAP_FIXED solo dentro de la reserva propia y páginas aún no mapeadas. Devuelve estado y ImageMapping; solo RX mapea texto de archivo sin escritura, RW solo datos. Cola BSS usa páginas anónimas RW sin ejecución. io/block/alloc; rangos/alineación/overflow se validan antes del syscall. |
| [selector=ImageEntry] | view ImageMapping RX, offset comprobado y descriptor ABI verificado; devuelve estado y entrada nativa tipada ligada a generación/rango, sin dirección entera. Hoja de representación del backend, sin syscall, efectos vacíos; no ejecuta código ni acepta una ABI inventada por el programa. La comprobación de símbolo/exportación pertenece al controlador Nexo. |
| [selector=ImageProtect] | edit ImageMapping de datos, transición única RW→R mediante mprotect para RELRO. Estado nativo; io/block. No convierte memoria escrita en ejecutable ni crea alias escribible de RX. |
| [selector=ImageUnmap] | move ImageReservation sin referencias activas, munmap de toda la reserva y sus mappings; estado nativo, io/block. Consume el recurso una vez; error conserva el registro de limpieza pendiente y bloquea instalación/descarga, nunca permite reutilizar las direcciones como libres. |

Cada hoja cobra argumentos/llamada y una operación más las celdas del registro
crudo de estado/recurso; ImageSeal cobra dos operaciones si alcanza
F_GET_SEALS y una si falla F_ADD_SEALS. ImageEntry conserva el préstamo del mapping en la
entrada y ImageMapFile expone RawView<U8> solo de sus datos, con rango y
permiso R/RW, nunca de texto como lugar editable. Bucles, bytes copiados, tablas y wrappers pagan sus
recetas Nexo ordinarias. Mapeo y reservas se contabilizan como recursos del
controlador, no como slots T inicializados. El perfil H2 fija límites de bytes
por imagen y generaciones antes de activación; ningún rango sin límite se
acepta. N14a verifica sellos efectivos, fallo por política/permiso, intento
de escritura después de sellar, limpieza parcial, coherencia y retirada
solo con cero referencias. macOS/Windows usan los selectores
NativeImageOpen/Lookup/Close definidos abajo, con proveedores de R40;
el catálogo distingue ese proveedor del controlador ELF Nexo.
El manifiesto de imagen R45 registra generación, target/CPU,
ABI, clave de artefacto, ruta privada absoluta, longitud y file_digest de
cada archivo final que cargará el sistema, inventario cerrado de imports/
exports tipados, dependencias y permisos de segmentos. Se identifica por
manifest_digest de R10; este valor identifica el manifiesto, **no** los bytes
de la imagen. El producto macOS sin firma y su copia firmada tienen entradas
distintas con sus propios file_digest; el cargador compara el archivo final.
La lectura del archivo original exige longitud y file_digest iguales a su
entrada, no igualdad con manifest_digest. La copia privada memfd Linux se
acredita por la escritura completa del búfer verificado y la máscara de
sellos observada, conforme al procedimiento de esta R45.

N14a añade los siguientes selectores C09 para macOS/Windows, con la misma
identidad lógica y proveedor seleccionado por target. Solo el controlador
Nexo LoadGeneration los invoca tras validar imagen, manifiesto, política y
ausencia de inicializadores/finalizadores/callbacks extranjeros, incluidos
los de TLS de imagen; los recursos tienen generación y no son enteros.
No se habilitan para cargar bibliotecas arbitrarias desde std.

| ID de selector C09.SystemCall | Contrato crudo, efectos y proveedor macOS/Windows |
|---|---|
| [selector=NativeImageOpen] | Recibe ruta privada absoluta prestada y descriptor de imagen ya verificado con manifest_digest. La imagen y sus dependencias declaradas son inmutables durante su vida; no hay búsqueda fuera del inventario. Devuelve {status:I64,image:opcional NativeImage}, recurso presente solo en éxito. macOS usa dlopen con RTLD_NOW/RTLD_LOCAL/RTLD_FIRST y firma/política R45, entorno DYLD/LD sin redirecciones y dependencias absolutas del inventario; Windows LoadLibraryExW con flags R40. El controlador valida imports y todos los exports antes de publicar. Fallo no instala generación ni entrega recurso; éxito conserva la referencia del cargador. io/block/alloc, sin suspend ni callbacks Nexo. |
| [selector=NativeImageLookup] | Recibe view NativeImage y nombre de exportación del inventario con descriptor ABI exacto; devuelve {status:I64,entry:opcional NativeEntry}. macOS dlsym, Windows GetProcAddress. Solo éxito entrega entrada tipada ligada a imagen/generación y conservando su préstamo; ni dirección entera ni conversión de ABI. Ausencia, dirección nula o error del cargador son error. Antes de NativeImageOpen, el controlador verifica en las tablas del archivo final que cada export del inventario está definido por esa imagen; rechaza reexports Mach-O y forwarders PE. RTLD_FIRST y ese inventario impiden suplir un export por uno de una dependencia. No se comprueba un rango virtual de imagen a partir de la dirección devuelta ni se requieren APIs adicionales de introspección. io/block/alloc conservadores, sin ejecutar la entrada ni callbacks. |
| [selector=NativeImageClose] | Recibe move NativeImage con cero entradas prestadas, referencias activas y dependientes según R45; dlclose/FreeLibrary libera exactamente la referencia propia y devuelve {status:I64}. io/block/alloc conservadores, sin suspend. Fallo consume el recurso del llamador y conserva registro interno de limpieza pendiente que bloquea descarga/instalación; no declara libre una imagen todavía retenida. No garantiza desmapeo físico inmediato del sistema. |

dlerror limpia/consulta el estado de dlopen/dlsym/dlclose dentro de esas
mismas hojas macOS, sin devolver su puntero ni interpretar mensajes en libc:
status=0 en éxito, status=1 en error de cargador. Windows captura GetLastError
inmediatamente tras error. El controlador convierte estados a Result Nexo;
validación propia previa usa sus errores tipados, sin fingir estado nativo.
Cada hoja cobra argumentos/llamada, una operación y una celda por campo del
registro crudo (dos en Open/Lookup, una en Close), más una por el opcional
y una por el recurso/entrada presente; el wrapper Result cobra su propia
celda y cada valor escalar una sola vez. Leer archivos, verificar
file_digest/manifest_digest, validar tablas y mantener generaciones paga
los recorridos Nexo ordinarios; no se oculta ese trabajo en la hoja.
N14a prueba por target éxito, ausencia de símbolo, ABI/import/firma inválidos,
fallo de carga/descarga, rechazo con entrada viva y limpieza parcial.
Un export propio válido acepta; convertir solo ese export en reexport Mach-O
o forwarder PE rechaza antes de NativeImageOpen. Un nombre inventariado que
solo exporta una dependencia tampoco puede instalar la generación.
Un fallo mantiene la raíz anterior; el positivo usa imagen propia y export
válido, y la mutación aislada de cada condición no puede instalarla.

La coherencia de instrucciones precede a resolver entradas: en macOS la
carga de la dylib final se delega a dyld mediante dlopen (sin escritura
posterior de texto), y en Windows al cargador de LoadLibraryExW. No hay
una operación de caché Nexo sin proveedor; no se requiere sys_icache_invalidate
ni FlushInstructionCache para este contrato de imágenes inmutables. En Linux
N14a usa los mappings ejecutables de archivo y la coherencia del subsistema
VM del kernel, también en arm64; no genera ni modifica instrucciones en RAM.
La frontera sigue la [carga documentada de Apple](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/dlopen.3.html)
y las [obligaciones de caché de la VM Linux](https://docs.kernel.org/core-api/cachetlb.html).
Una evidencia nativa por destino comprueba permisos efectivos, ejecución de
cada generación con bytes distintos y descarga tardía antes de activar H2;
sin ella N14a no entrega el destino. No se parchean bytes ejecutables.

**R46.** El arnés diferencial toma un estado serializable de prueba en el
punto seguro, instala la edición y compara la continuación con un proceso
limpio con ese mismo estado y código nuevo. Solo se comparan efectos posteriores
al punto seguro; efectos previos no se deshacen. Los casos con recursos externos
usan un modelo de entorno/reapertura explícito, no pretenden clonar handles.
Se comprueban contadores de limpieza y préstamos, no solo salida visible.

## 8. Diagnósticos propuestos y matriz normativa

**R47.** Se propone integrar en [F0-CODIGOS](F0-CODIGOS.md) una sola entrada
nueva: `NX-E1701`, propietario D17/recarga, rechazo de una solicitud de recarga
válida como programa pero incompatible con la sesión. No se reutilizan
NX-E12xx–E15xx ni R05xx. Es un error de análisis de la edición, no un fallo
del programa en ejecución ni una excepción. Las puertas de bootstrap/backend
producen resultados de validación, no códigos NX-E ficticios de errores internos.

Subcausas cerradas, por orden de prioridad: `method_reload_unsupported`,
`trait_reload_unsupported`, `alias_reload_unsupported`,
`declaration_removed`, `identity_changed`,
`signature_changed`, `contracts_changed`, `frame_layout_changed`,
`unsafe_boundary`, `live_dependency`, `quiescence_budget_exhausted`.
La subcausa live_dependency cubre estados suspendidos y esperas que incumplen el criterio operativo
de R44, salvo frontera insegura prioritaria R44, sin retiro automático;
frames/préstamos ordinarios en progreso usan R44 sin rechazo previo por ella.
El agotamiento conserva el esquema data, changed_fields exactos (body si solo
cambió cuerpo) y live_kinds activos; límite/unidad son constantes del perfil
H1/H2 versionado. Dentro de la primera categoría aplicable
se elige la clave lexicográfica menor. Se propone ampliar el esquema de
diagnósticos con `stage=reload` y `operation=reload.classify`; solo en el
perfil de herramienta que implemente H1/H2, nunca en los perfiles F0 actuales.
La misma enmienda añade el rol relacionado `previous_definition`, referencias
de fuente con revisión y los campos data de esta entrada; no se insertan
valores nuevos en los vocabularios cerrados de F0 sin versionar el esquema.
El perfil de herramienta H1/H2 usa schema_version=2: todo Span de ese perfil
incluye revision=file_digest de los bytes de su fuente (R10),
además de file/start/end. El orden de related_spans añade revision después
de file y antes de start/end/role. Los perfiles históricos conservan versión 1.
La enmienda acumulativa y sus filas afectadas se registran en D17 §12.

La clasificación compara registros canónicos, no solo interface_hash. Esta tabla
cubre todos los componentes de interface_hash R10, incluidos los públicos
D13 §12; una diferencia no enumerada es un defecto del clasificador, nunca
permiso de recarga. Cada componente cambiado aporta exactamente el valor
indicado a changed_fields; los duplicados se eliminan y se ordena por ASCII.
Si cambian varios componentes se conserva su unión y gana la subcausa de
mayor prioridad anterior. signature subsume cambios mecánicos de ABI y
layout del descriptor de llamada derivados exclusivamente de esa misma
firma (por ejemplo I64→Bool en B16); no añade abi/frame_layout por esos
cambios derivados. La misma absorción se aplica al almacenamiento derivado
exclusivamente del tipo de una constante; B40 solo aporta signature.
Cambiar independientemente convención, representación o
layout de un tipo público o privado sí aporta esas claves aunque también cambie firma.
body se usa únicamente si cambia body_hash y no
cambió ningún componente de interfaz; no se añade a un cambio de firma como
B16. Las fronteras y dependencias vivas solo aportan live_kinds: no son
campos de interfaz ni cambios de fuente.

| Componente de interface_hash (R10) | Subcausa | changed_fields |
|---|---|---|
| Firmas, tipos de parámetros/resultado, tipo declarado de constante, requisitos de trait y destino de alias, modos, etiquetas y defaults | signature_changed; trait/alias usan sus subcausas prioritarias | signature |
| Procedencias y retención | contracts_changed | provenance |
| Afinidad | contracts_changed | affinity |
| Cotas escritas, incluidos requisitos genéricos | contracts_changed | contracts |
| Resumen de efectos inferidos de la función por operación invocable D11/D12 | contracts_changed | effects |
| Catálogo requerido: identidades, versiones, firmas y disponibilidad | contracts_changed | catalog |
| ABI | contracts_changed | abi |
| Descriptor físico y tipado de almacenamiento de cada tipo, público o privado, y de entornos/estados: slots con sus identidades de tipo, offsets, tamaño/alineación y tags/payloads; excluye nombres de miembros | frame_layout_changed | frame_layout |
| D13 §12: identidad (origen, nombre, revisión) del paquete, sin package_digest, exportaciones canónicas, traits/impl y extensiones | contracts_changed | public_interface |
| D13 §12: nombres, tipos declarados de miembros públicos y variantes de tipos, aunque conserven tamaño/layout físico (por ejemplo S.A→S.B o miembro I64→U64) | contracts_changed | public_interface |
| D13 §12: propiedades selladas de copia/independencia/transferencia, limpieza y supuestos extranjeros | contracts_changed | contracts |
| Dominio de interface_hash: versión de esquema y semántica | contracts_changed | hash_domain |

La matriz de clases siguiente completa la clasificación de altas, bajas y
cuerpos/valores; las filas de interfaz anteriores siguen aportando su unión
de changed_fields. Alta y baja aportan exclusivamente added y removed,
respectivamente, sin comparar una interfaz ausente. Una constante existente
con tipo idéntico y valor distinto aporta body; su valor canónico está en
body_hash. Todo consumidor del valor registra la arista comptime en
dependency_hash; se invalida, revalida y reconstruye el cierre transitivo
antes de publicar la generación. Errores ordinarios de ese cierre conservan
prioridad sobre NX-E1701. Un cambio de expresión con el mismo valor canónico
y los mismos registros no es un cambio semántico.

Traits y alias tienen DefKey propia y declaration_kind=trait/alias, incluso
si son privados. Cualquier alta, baja, cambio de identidad o edición
semántica suya requiere reinicio con trait_reload_unsupported o
alias_reload_unsupported, respectivamente, aunque no tengan consumidores.
Un requisito de trait (incluidos nombre, firma, cotas y presencia de default)
pertenece a su interfaz y aporta signature; el cuerpo de un default existente
con interfaz idéntica aporta body. Los defaults y sus cuerpos anónimos se
atribuyen al trait, sin crear una DefKey de método impl/extend para ellos.
El destino resuelto de un alias es su firma: cambiar su RHS aporta signature,
no body, aunque tenga el mismo layout. Un alias no tiene cuerpo ejecutable
ni valor separado. Los structs/enums tampoco tienen cuerpo ejecutable:
sus campos/variantes son interfaz/layout conforme a la tabla anterior.
La trivia de cualquier clase solo actualiza mapas y acepta con registros
semánticos iguales. Estas reglas se aplican a registros verificados, no
permiten aceptar una fuente inválida para después rechazar su recarga.

Una alta de función, tipo o constante acepta si es válida y no cambia
interfaces existentes; se recompila su cierre y aplica R44 a sus dependientes.
Si cambia una interfaz existente, se clasifica además esa declaración y
se rechaza por la tabla anterior. Una baja de cualquiera de esas tres clases
usa declaration_removed; la baja de method/trait/alias conserva su subcausa
prioritaria. No se deduce la clase por mayúsculas ni por layout físico.

| declaration_kind | Alta | Baja | Cambio de firma/interfaz | Cambio de cuerpo/valor |
|---|---|---|---|---|
| function | Acepta bajo regla de alta; B53 | declaration_removed/[removed]; B33 | signature_changed/[signature]; B16 | Compatible: R43/R44; B15/B16 positivo |
| type | Acepta bajo regla de alta; B51 | declaration_removed/[removed]; B52 | Tabla de interfaz: frame_layout_changed o contracts_changed; B32/B36/B38 | Sin cuerpo/valor separado: campos/variantes usan interfaz, B32; trivia B36 positivo |
| constant | Acepta bajo regla de alta; B43 | declaration_removed/[removed]; B42 | signature_changed/[signature]; B40 | body con revalidación por dependency_hash; B41 |
| method | method_reload_unsupported/[added]; fixtures de métodos | method_reload_unsupported/[removed]; fixtures de métodos | method_reload_unsupported/[signature]; B54 | method_reload_unsupported/[body]; fixtures de métodos |
| trait | trait_reload_unsupported/[added]; B46 | trait_reload_unsupported/[removed]; B47 | trait_reload_unsupported/[signature]; B44 | trait_reload_unsupported/[body] para default existente; B45 |
| alias | alias_reload_unsupported/[added]; B49 | alias_reload_unsupported/[removed]; B50 | alias_reload_unsupported/[signature]; B48 | RHS es firma, sin cuerpo/valor separado; B48, trivia en su positivo |

Los restantes datos D13 §12 usan las filas específicas anteriores: afinidad,
procedencias/retención, efectos, etiquetas/defaults y tipos. Un cambio de
DefKey se clasifica antes como identity_changed; la revisión de paquete no
cambia DefKey por sí sola. Una edición de desarrollo conserva la revisión
resuelta del paquete; cambiar esa revisión es cambio de public_interface,
no una edición de cuerpo compatible. Perfil, target/CPU, flags y versión de
runtime se fijan al iniciar sesión: cambiar esa configuración crea otra
sesión, nunca una solicitud H1/H2 de cuerpo. catalogo_resuelto y
layouts_concretos_usados son datos derivados que se recomputan en cada
edición, no configuración congelada: cambios de contratos requeridos o de
tipos existentes se clasifican por las filas anteriores, incluidos tipos
privados; una instancia local nueva sin interfaz ni estado previo afectado
solo invalida/reconstruye su cierre. Sustituir el proveedor externo del
catálogo, SDK o ABI de target requiere otra sesión; no se presenta como una
edición de cuerpo ni elude frame_layout_changed de un tipo existente.
Una nueva referencia de F a `C01.IntAdd[mode=wrapping,type=U32]` que amplía el
catálogo requerido, conservando los demás componentes, exige
contracts_changed, changed_fields=[catalog], aunque la edición fuente solo
toque el cuerpo. B34 es una mutación del artefacto tipado del arnés: se añade
una llamada resuelta a una función de std.core cuya hoja es esa identidad;
no se escribe un ID C01 como Name de fuente. El arnés recompone/verifica los
registros y body_hash/interface_hash/dependency_hash antes de pedir recarga, sin corrupción de integridad.
Si otra función no alcanzable desde F ya requería esa hoja no basta: se compara
el catálogo requerido de F, no una unión global que oculte el cambio.

Campos de NX-E1701: primario sobre el nombre de la declaración nueva;
relacionado `previous_definition` sobre el nombre anterior, con fuente y
revisión distinguibles; unexpected_token/parser_context/recognized_prefix/
expected_tokens_kind nulos; expected_tokens, cause_chain y repair vacíos;
feature_id nulo. `data` contiene exactamente `action="restart_required"`,
`declaration_key` (tupla R09), `declaration_kind` (function/type/constant/method/trait/alias), `old_generation`, `new_generation` (UInt),
`changed_fields` y `live_kinds` (listas de strings ordenadas sin duplicados).
No se inventa una edición automática que reinicie o pierda estado.
declaration_key identifica las seis clases de la matriz en la misma tabla de
identidades R09: un tipo con objetos vivos no necesita una función ficticia
para denunciar su layout. No se serializa un campo function_key adicional.
Cambiar únicamente el descriptor de almacenamiento de un tipo, público o
privado, conserva DefKey y usa declaration_kind=type,
subcause=frame_layout_changed, changed_fields=[frame_layout] y
live_kinds=[value] si solo hay instancias vivas. Se compara tanto su parte
física como las identidades de tipos de slots; I64 y U64 nunca son el mismo
slot tipado aunque tengan iguales tamaño, alineación y bits.
Para `pub struct S{pub A:I64;}` → `pub struct S{pub A:U64;}` cambian
descriptor tipado y miembro público: changed_fields=[frame_layout,public_interface],
subcause=contracts_changed por prioridad. Para `struct S{a:I64;}` →
`struct S{a:U64;}` solo cambia frame_layout, con frame_layout_changed.
No se añade public_interface por un miembro privado ni se omite frame_layout
porque el tipo no se exporte. Cambios físicos sin cambio de miembro público
usan solo frame_layout. Renombrar un miembro público conservando descriptor
físico y tipado usa exclusivamente contracts_changed/[public_interface],
no frame_layout ni live_dependency. La presencia de instancias solo cambia
live_kinds. En H1/H2 todo cambio de interfaz requiere reinicio incluso sin vivos.
Eliminar una función, tipo o constante usa declaration_removed, changed_fields=[removed]
y la clave/clase anterior; un renombrado explícito F→G en la solicitud usa
identity_changed, changed_fields=[identity] y la nueva clave; live_kinds
registra exactamente los vivos de la declaración anterior (vacío si no hay).
Sin correspondencia explícita se clasifica
como eliminación más adición, sin heurística por cuerpo. Una correspondencia
anónima dudosa R11 se clasifica identity_changed sobre la declaración superior
contenedora, changed_fields=[identity], con sus kinds vivos; no se inventa
una clave de closure estable por parecido de cuerpo.

method_reload_unsupported usa declaration_kind=method y la DefKey del método
nuevo; si se elimina, usa su clave y nombre de la fuente de la sesión.
El primario es el Name del método, también cuando cambia un cuerpo anónimo
contenido; previous_definition identifica ese Name en la sesión si existe.
Una adición no inventa previous_definition y aporta changed_fields=[added];
una eliminación aporta [removed], un cambio de identidad declarado [identity].
Para un método conservado se aplica la misma unión de campos de R47, incluido
[body] si solo cambia cuerpo; live_kinds enumera sus dependencias vivas reales.
Una correspondencia anónima dudosa dentro del método aporta [identity] sobre
él y mantiene method_reload_unsupported como subcausa prioritaria. Sin
correspondencia explícita entre claves distintas se trata como eliminación
y adición. Fuente inválida conserva primero sus diagnósticos ordinarios.

Para trait/alias se usan las mismas reglas de primario, previous_definition,
clave y generaciones: nombre nuevo si existe, nombre de sesión en una baja,
sin previous_definition en una alta. Identidad aporta [identity]; cambios
de firma o cuerpo usan la matriz y su unión de campos. Las subcausas de
clase no sustituyen esa evidencia por un campo genérico de rechazo.

Vocabularios: changed_fields ∈ {identity, signature, contracts, frame_layout,
body, effects, catalog, provenance, affinity, abi, public_interface, hash_domain,
removed, added}; live_kinds ∈
{frame, generator, task, loan, region, foreign_callback, system_call, value}. Los arrays pueden ser vacíos si no son causa. Una solicitud
que elimina una declaración usa su nombre anterior como primario; la petición
de edición identifica la revisión anterior, sin fabricar un token nuevo.

**R48.** PASS/FAIL en la tabla son resultados **esperados**, no veredictos
obtenidos. Las filas de herramienta/artefacto no son programas fuente y no
deben fallar por parser para acreditar la regla. Los informes de puertas
usan PASA/FALLA/NO EVALUADO/INCONCLUSO y un identificador de requisito.

| Caso | Entrada o mutación aislada | Esperado | Evidencia/diagnóstico exacto requerido |
|---|---|---|---|
| B01 | Mismas entradas stage2/3, objetos y binarios idénticos | PASS | R24, inventario y comparación de todos los bytes |
| B02 | Un byte de código de stage3 distinto, IR idéntica | FAIL | R24/FALLA; ruta, primer offset distinto, ambos file_digest; sin NX-E |
| B03 | Segunda construcción en otra ruta/usuario, entradas remapeadas iguales | PASS | Mismos bytes y manifiesto de diferencias de entorno |
| B04 | Se excluye una sección de datos de la comparación | FAIL | R24/FALLA incluso si file_digest del archivo mutilado coincide |
| B05 | Punto fijo correcto, D2 diverso difiere de stage2 | FAIL | R26/FALLA; no se retira C0 |
| B06 | V+1 usa una capacidad que V rechaza | FAIL | R25/FALLA, diagnóstico de alcance real de V; nunca parche por fuente |
| B07 | Binario Nexo carga helper Rust escondido en una biblioteca | FAIL | R29/R40/FALLA por inventario transitivo |
| B08 | Lexer Nexo y Rust ejecutan igual pero sus AST difieren en move | FAIL | R16/FALLA, primer nodo distinto y fuentes B |
| B09 | Formato dos veces cambia comentarios o AST | FAIL | R16/FALLA; CST/AST y bytes canónicos comparados |
| B10 | Cuota agotada se trata como verificación suficiente | FAIL | R06/FALLA; conserva NX-E0003 del perfil y no hay ejecutable |
| B11 | Backend omite cierre en salida Err | FAIL | R12/R38/FALLA, traza de limpieza diferente |
| B12 | Backend propio sin F6, aunque gana un microbenchmark | FAIL | R33/NO EVALUADO para puertas faltantes; no se habilita sustitución |
| B13 | Backend correcto pero incumple H-CS o T26 | FAIL | R38/R39/FALLA según modo; LLVM no se retira |
| B14 | Objetos de ABI distinta con mismo nombre de símbolo | FAIL | R08/FALLA antes de enlace ejecutable; no búsqueda de símbolo como permiso |
| B15 | Cambio solo de cuerpo compatible, sin dependencias vivas | PASS | R43–R46; aplica una generación completa; sin NX-E1701 |
| B16 | Firma cambia aunque todos los callers recompilen válidamente | FAIL | NX-E1701/signature_changed; ejemplo literal inferior |
| B17 | Cambia cota pública y un caller incumple no_alloc | FAIL | NX-E0412 conforme al catálogo; no NX-E1701 como sustituto; edición inválida |
| B18 | Cambia cuerpo de generador suspendido, mismo tamaño | FAIL | NX-E1701/live_dependency, changed_fields=[body], live_kinds=[generator] |
| B19 | Artefacto tipado de S público: cambia solo offset/alineación de un slot privado, conservando tipo de slot y toda su interfaz pública; hay objetos vivos | FAIL | NX-E1701/frame_layout_changed, declaration_key de S, declaration_kind=type, changed_fields=[frame_layout], live_kinds=[value]; primarios S nuevo/anterior; no migración implícita H3 |
| B20 | Insertar comentario antes de F en paquete de desarrollo de revisión fija, recomponiendo package_digest esperado | PASS | Misma DefKey, body_hash, interface_hash, dependency_hash y clave de artefacto; source_map_hash, file_digest de fuente y package_digest nuevos; construcción literal inferior. |
| B21 | Solicitud declara explícitamente renombrado F→G, sin vivos, tratando de conservar su slot | FAIL | NX-E1701/identity_changed, declaration_key nueva de G, changed_fields=[identity], live_kinds=[]; primario G nuevo/previous_definition F anterior; no correspondencia por body_hash |
| B22 | Préstamo compatible termina antes del punto seguro | PASS | No falso rechazo por un préstamo histórico ya extinguido |
| B23 | Edición solo de cuerpo de F con interfaz idéntica; solo queda un handle extranjero que retiene callback de F sin prueba de retiro, sin frame ni tarea ejecutándolo | FAIL | NX-E1701/unsafe_boundary, changed_fields=[body], live_kinds=[foreign_callback] |
| B24 | Cambia iteración de hash map, mismas claves | PASS | Mismos artefactos y diagnósticos ordenados; coste medido aparte |
| B25 | Solo cambian efectos inferidos de F, llamadores aún válidos | FAIL | NX-E1701/contracts_changed, changed_fields=[effects], live_kinds=[]; firma y contratos declarados idénticos |
| B26 | F activa no llega a quiescencia antes del paso 1.000.001, edición solo de cuerpo | FAIL | NX-E1701/quiescence_budget_exhausted, changed_fields=[body], live_kinds=[frame]; no instalación ni modificación de estado |
| B27 | F alcanza quiescencia exactamente al completar 1.000.000 pasos | PASS | Instalación completa, sin diagnóstico; reloj de pared irrelevante |
| B28 | Inyección aislada del arnés bajo f1-library-test-work-v1: L=A+W−1 fijado antes de ejecutar conforme D16 R32, W>=1 | FAIL | R0002/library_work, Diagnostic.profile_id=f1-library-test-work-v1, data={runtime_budget_kind:library_work,limit:L,observed:L+1}, span de llamada pública activa, limpieza única y ningún producto válido. Positivo L=A+W; demás cuotas de bootstrap-v1, A/W transcritos del certificado, sin alterar sus límites normales R50. |
| B29 | B1 intenta frame 1.025 | FAIL | R0002/call_frames, limit=1024, observed=1025, span de llamada, limpieza única; positivo pareado termina con 1.024 |
| B30 | Import macOS o Windows fuera de lista R40, aunque venga de una imagen del sistema; negativos separados mprotect en macOS y VirtualProtect en Windows | FAIL | Auditoría R40/FALLA con símbolo e imagen; no se anuncia independencia. Positivos de guardas: C04 conserva páginas de datos RW/guardas sin acceso mediante mmap/munmap o VirtualAlloc/VirtualFree, sin esos imports. |
| B31 | F activa detenida en ReadLine→C09/ConsoleRead block, edición de su cuerpo | FAIL | NX-E1701/unsafe_boundary inmediato, declaration_key de F, declaration_kind=function, changed_fields=[body], live_kinds=[frame,system_call]; cero espera por reloj/pasos y raíz anterior intacta. |
| B32 | Tipo S privado en package.net.http con instancia viva: a:I64→a:U64, tamaño/alineación/offsets idénticos, sin función F asociada | FAIL | NX-E1701/frame_layout_changed, declaration_key=[fixture,demo,package.net.http,S], declaration_kind=type, changed_fields=[frame_layout], live_kinds=[value], primarios nombre S nuevo/anterior [7,8); desarrollo literal inferior. |
| B33 | Se elimina F sin renombrado declarado | FAIL | NX-E1701/declaration_removed, declaration_key anterior, declaration_kind=function, changed_fields=[removed], live_kinds=[] si no hay vivos; primario nombre F anterior. |
| B34 | Artefacto tipado de F añade llamada std.core resuelta a C01.IntAdd[mode=wrapping,type=U32], nueva en su catálogo requerido, sin cambiar firma, efectos ni otros componentes de interfaz | FAIL | NX-E1701/contracts_changed, changed_fields=[catalog], live_kinds=[]; positivo cambia cuerpo usando solo hojas ya requeridas y acepta. |
| B35 | Edición solo de cuerpo; frame de F espera tarea no afectada detenida en C09 block | FAIL | NX-E1701/unsafe_boundary antes de esperar, changed_fields=[body], live_kinds=[frame,system_call,task], clave de F; raíz intacta y cero pasos de espera. Positivo termina tarea/frontera antes de solicitar recarga. |
| B36 | Renombrar solo miembro público A→B de S, conservando layout físico y llamadores válidos | FAIL | NX-E1701/contracts_changed, declaration_kind=type, changed_fields=[public_interface], live_kinds=[value] con una instancia; sin instancias, mismo diagnóstico con live_kinds=[]. No usa frame_layout_changed ni live_dependency. Positivo conserva el miembro A y añade solo trivia: acepta, con y sin instancia. |
| B37 | Fixture de arnés de registros verificados: edición de cuerpo de F; su frame espera T no afectada, suspendida, no ejecutable, sin evento pendiente ni salida ejecutable en el cierre, fuera de C08/C09 | FAIL | NX-E1701/live_dependency inmediato, changed_fields=[body], live_kinds=[frame,task], clave de F; cero pasos de espera, raíz anterior intacta. Un evento encolado que termina T y deja salir F acepta; fixture inferior. |
| B38 | S público en package.net.http: miembro público A:I64→A:U64, con layout físico idéntico | FAIL | NX-E1701/contracts_changed, changed_fields=[frame_layout,public_interface], live_kinds=[value], declaration_kind=type; positivo de trivia acepta. |
| B39 | Mover explícitamente F de package.a.bc a package.ab.c, mismo cuerpo y revisión, sin vivos | FAIL | NX-E1701/identity_changed, nueva declaration_key=[fixture,demo,package.ab.c,F], declaration_kind=function, changed_fields=[identity], live_kinds=[]; rutas distintas conforme R09. |
| B40 | Constante c: tipo I64→Bool, inicializador válido para cada tipo | FAIL | NX-E1701/signature_changed, declaration_kind=constant, changed_fields=[signature], c [6,7) en ambas revisiones; no body ni frame_layout derivado del tipo. |
| B41 | Valor de c: 1→2, mismo tipo; F consume c | PASS | c conserva interface_hash y cambia body_hash; F conserva body_hash/interface_hash y cambia dependency_hash, que obliga a revalidar/reconstruir su cierre. F devuelve 2 en generación 2. No NX-E1701, sin dependencias vivas. |
| B42 | Eliminar c sin consumidores | FAIL | NX-E1701/declaration_removed, declaration_kind=constant, changed_fields=[removed], c anterior [6,7). |
| B43 | Añadir c a biblioteca vacía | PASS | Alta válida, declaración constant, c nuevo [6,7), sin interfaz previa ni consumidores afectados. |
| B44 | Requisito de trait T: resultado I64→U64 | FAIL | NX-E1701/trait_reload_unsupported, declaration_kind=trait, changed_fields=[signature], T [6,7) en ambas revisiones. |
| B45 | Default existente de T: cuerpo return 1→return 2 | FAIL | NX-E1701/trait_reload_unsupported, declaration_kind=trait, changed_fields=[body], T [6,7) en ambas revisiones; no atribución al método F. |
| B46 | Añadir trait T | FAIL | NX-E1701/trait_reload_unsupported, declaration_kind=trait, changed_fields=[added], T nuevo [6,7), sin previous_definition. |
| B47 | Eliminar trait T | FAIL | NX-E1701/trait_reload_unsupported, declaration_kind=trait, changed_fields=[removed], T anterior [6,7). |
| B48 | Alias A: destino I64→U64, mismo layout | FAIL | NX-E1701/alias_reload_unsupported, declaration_kind=alias, changed_fields=[signature], A [5,6) en ambas revisiones; no body ni frame_layout de un nominal ficticio. |
| B49 | Añadir alias A | FAIL | NX-E1701/alias_reload_unsupported, declaration_kind=alias, changed_fields=[added], A nuevo [5,6), sin previous_definition. |
| B50 | Eliminar alias A | FAIL | NX-E1701/alias_reload_unsupported, declaration_kind=alias, changed_fields=[removed], A anterior [5,6). |
| B51 | Añadir struct S privado sin consumidores | PASS | Alta válida, declaration_kind=type, S nuevo [7,8); no cuerpo ejecutable ni interfaz previa que comparar. |
| B52 | Eliminar struct S privado sin consumidores | FAIL | NX-E1701/declaration_removed, declaration_kind=type, changed_fields=[removed], S anterior [7,8). |
| B53 | Añadir función F sin consumidores | PASS | Alta válida, declaration_kind=function, F nuevo [3,4); no interfaz previa que comparar. |
| B54 | Método extend F: resultado I64→Bool, retorno válido para cada tipo | FAIL | NX-E1701/method_reload_unsupported, declaration_kind=method, changed_fields=[signature], F [23,24) en ambas revisiones. |
| B55 | Proveedor ImageSeal del arnés: fallo F_ADD_SEALS, fallo F_GET_SEALS o máscara observada sin uno de los cuatro bits, negativos separados | FAIL | R45/FALLA de carga, no NX-E1701; no ImageMapFile ni publicación, cierre único. Sin F_GET_SEALS tras error de F_ADD_SEALS; status no exitoso y seals=0 si no hay observación válida. Cada máscara incompleta se conserva en seals y produce -EIO. Positivo: ambas llamadas exitosas y máscara 15, status=0; la carga continúa. |
| B56 | Vector del codificador manifest_digest con contenido=Bytes vacío: variar solo clase entre las Strings `construccion` e `imagen` | PASS | Dominio manifest_digest en ambos, contador U64=4; clase es primer campo propio, contenido el segundo, versiones comunes al final. Registros y digests distintos. Es un vector de codificación, no un manifiesto completo admisible por R22/R45. Negativos de esquema separados: usar clase como dominio, omitir clase, clase desconocida o contar solo los campos propios rechaza antes de aceptar artefacto; file_digest sigue SHA-256 crudo. |

Fuentes literales de B40–B54: ASCII, sin LF final, biblioteca root.n de
(fixture,demo,dev), módulo package, generaciones 1/2. ∅ representa cero
bytes, no un token de fuente. No hay consumidores ni vivos salvo F en B41,
que se ejecuta solo después de instalar la generación. Los spans son bytes
semiabiertos, calculados sobre las fuentes completas, sin sus delimitadores.

| Caso | Fuente de sesión | Fuente nueva | Name y span anterior/nuevo |
|---|---|---|---|
| B40 | `const c:I64=comptime 1;` | `const c:Bool=comptime true;` | c [6,7)/[6,7) |
| B41 | `const c:I64=comptime 1;fn F()->I64{return c;}` | `const c:I64=comptime 2;fn F()->I64{return c;}` | c [6,7)/[6,7); F [26,27)/[26,27) |
| B42 | `const c:I64=comptime 1;` | ∅ | c [6,7)/ausente |
| B43 | ∅ | `const c:I64=comptime 1;` | c ausente/[6,7) |
| B44 | `trait T{fn F(self:view)->I64;}` | `trait T{fn F(self:view)->U64;}` | T [6,7)/[6,7) |
| B45 | `trait T{fn F(self:view)->I64{return 1;}}` | `trait T{fn F(self:view)->I64{return 2;}}` | T [6,7)/[6,7) |
| B46 | ∅ | `trait T{}` | T ausente/[6,7) |
| B47 | `trait T{}` | ∅ | T [6,7)/ausente |
| B48 | `type A=I64;` | `type A=U64;` | A [5,6)/[5,6) |
| B49 | ∅ | `type A=I64;` | A ausente/[5,6) |
| B50 | `type A=I64;` | ∅ | A [5,6)/ausente |
| B51 | ∅ | `struct S{}` | S ausente/[7,8) |
| B52 | `struct S{}` | ∅ | S [7,8)/ausente |
| B53 | ∅ | `fn F()->I64{return 1;}` | F ausente/[3,4) |
| B54 | `struct S{} extend S{fn F(self:view)->I64{return 1;}}` | `struct S{} extend S{fn F(self:view)->Bool{return true;}}` | F [23,24)/[23,24) |

Todo negativo B40–B54 exige exactamente NX-E1701, stage=reload,
operation=reload.classify, subcausa y changed_fields de su fila,
action=restart_required, declaration_key=[fixture,demo,package,Name],
declaration_kind indicado, old_generation=1, new_generation=2, live_kinds=[].
En B54 el último componente de la clave es $method:extend:0:F.
El primario usa el Name nuevo salvo baja, que usa el anterior;
previous_definition usa ese mismo Name anterior si existe. Cada Span lleva
file=root.n y revision=file_digest de su fuente completa; los demás campos,
nulos y arrays vacíos son exactamente R47. No se añade token inesperado.
El positivo pareado de cada rechazo conserva la fuente de sesión y solo
antepone los bytes 2F 2F 78 0A (`//x\n`, LF); acepta sin NX-E1701.
Para cada regla de rechazo se elimina únicamente su condición: el test debe
detectar la aceptación indebida o el cambio de subcausa/campos exactos. En B40 se
omite solo el tipo de constante de la comparación de firmas; en B44/B45/
B46/B47 y B48/B49/B50 se desactiva únicamente la barrera de la clase y
operación examinadas. Las bajas B42/B52 omiten únicamente su rechazo.
En B54 se omite únicamente la barrera de método para cambios de firma.
B41 elimina solo la arista de valor de c en dependency_hash de F: conservar
el artefacto anterior o devolver 1 hace fallar la regresión. Su positivo
pareado cambia solo trivia y conserva los tres hashes semánticos.
El vector verificado N02 de B41 conserva en el CFG de F la referencia
resuelta a la DefKey de c, sin sustituirla por su literal; cuerpo e interfaz
de F son idénticos en ambas revisiones. El emisor consume el valor mediante
esa arista comptime y la caché contiene el artefacto de F para c=1. La prueba
exige que cambien dependency_hash y clave de artefacto de F y que se invalide
esa entrada; no basta reconstruir todo y acertar la salida por casualidad.
B43/B51/B53 exigen además detectar la mutación inversa que rechaza toda alta
válida. B55 omite por separado la consulta y cada bit requerido; B56 usa
un dominio dependiente de clase u omite el campo clase, por separado.
Los vectores de los siete registros R10 comprueban además etiqueta exacta,
orden/tipo de cada campo y contador igual a propios + 2. Sus negativos
aislados omiten uno de los comunes, cuentan solo los propios o repiten
esquema/semántica como campos propios de artifact_key. La lectura rechaza
bytes sobrantes o truncados, discriminante desconocido, clave duplicada o
desordenada y Bool/opcional distinto de 0/1; el positivo decodifica y
recodifica exactamente los mismos bytes. manifest_digest exige los prefijos
String 12/6 de clase, Bytes de contenido y las dos versiones exteriores;
duplicar clase/versión en contenido no cumple el esquema de manifiesto.
Cada mutación se restaura comprobando file_digest conforme R49.

Fixtures de R09/R11/R47 para métodos, en perfil H1/H2 con traits, impl y
extend habilitados. Paquete (fixture,demo,dev), módulo package, generaciones
1/2, sin callers ni estados vivos; bibliotecas ASCII sin LF final:

```nexo
struct S{} extend S{fn F(self:view)->I64{return 1;}}
```

```nexo
trait T{fn F(self:view)->I64;} struct S{} impl T for S{fn F(self:view)->I64{return 1;}}
```

Cada negativo cambia solo `return 1;` por `return 2;` y recompone los
registros verificados e integridad externa. Ambos programas son válidos:
NX-E1701/method_reload_unsupported, reload/reload.classify, primario F
nuevo y previous_definition F de la sesión en [23,24) para extend y
[58,59) para impl. Data exacta: action=restart_required,
declaration_key=[fixture,demo,package,$method:extend:0:F] o
[fixture,demo,package,$method:impl:0:F], respectivamente,
declaration_kind=method, old_generation=1, new_generation=2,
changed_fields=[body], live_kinds=[]; demás campos y revision de R47.
Cada positivo antepone solo `//x\n` (LF) a la fuente de la sesión, recompone
integridad/mapas y acepta sin NX-E1701, conservando clave y registros semánticos.
Eliminar únicamente la barrera de métodos debe hacer fallar ambos negativos;
tratar toda trivia como cambio de cuerpo debe hacer fallar los positivos.
Los vectores N02 añaden dos bloques extend de receptores distintos con F, un impl y un
extend con F, y una función superior F: todas sus claves son distintas.
La adición se aísla usando la fuente de sesión `struct S{}` y la fuente
extend completa como edición: misma clave de método y primario [23,24),
changed_fields=[added], live_kinds=[], sin previous_definition. La eliminación
invierte esas dos fuentes: clave de método de la sesión, primario y
previous_definition [23,24) de esa fuente, changed_fields=[removed],
live_kinds=[]. Ambas usan method_reload_unsupported y el resto de data R47;
el positivo conserva la fuente de la sesión sin añadir ni eliminar el método.

B20 de D17 R48 y V18 de D16 R45 usan paquete de desarrollo (origen=fixture, nombre=demo, revisión=dev),
módulo package, archivo root.n con `fn F()->I64{return 1;}` sin LF final.
La nueva fuente antepone bytes 2F 2F 78 0A (`//x\n`, donde \n representa LF).
Se recompilan interfaces y mapa fuente, se recompone package_digest y se
actualiza su valor esperado en el registro externo del arnés. Ningún
manifiesto/lockfile incluido contiene la integridad de sí mismo ni una
revisión generada desde los bytes. La prueba compara los registros R10:
cuerpo/interfaz/dependencias y clave de artefacto iguales, mapa y bytes
distintos. Mantener el package_digest esperado anterior da únicamente el
negativo de integridad V13 de D16 R45, no constituye la edición válida B20 de D17 R48.

Los vectores de R10 incluyen mutaciones independientes de proveedor resuelto,
descriptor privado usado, flags y parámetro de perfil: cada una cambia la
clave de artefacto. Proveedor de contrato idéntico, flags y perfil conservan
body_hash/interface_hash/dependency_hash; cambiar el descriptor privado
cambia además interface_hash del tipo y dependency_hash de sus consumidores.
Entrada de catálogo instalada
pero no usada, tipo privado no usado y opción cosmética de CLI son los
positivos que conservan la clave. Cambiar solo un dato de procedencia del
manifiesto cambia manifest_digest, sin cambiar productos ni clave; cambiar un byte
de imagen cambia file_digest y su entrada de manifiesto. Comparar la imagen
con manifest_digest en vez de con file_digest debe rechazar un positivo
válido y detecta esa mutación inversa del cargador.

B32 y B38 usan archivos de biblioteca sin LF final: root.n=`mod net;`,
net.n=`mod http;`; net/http.n contiene `struct S{a:I64;}` para B32 y
`pub struct S{pub A:I64;}` para B38. Las nuevas fuentes sustituyen solo
I64 por U64; Linux x86-64 tiene idénticos tamaños, alineaciones y offsets.
El arnés compila la expresión `S(a:1)` en B32 o `S(A:1)` en B38 dentro
del módulo http anterior,
mediante el frontend normal, y conserva un value propietario sin frame,
loan ni task. Así la privacidad no exige una importación externa inválida.
En B32 primario S nuevo y previous_definition anterior son [7,8);
en B38 ambos [11,12). Paquete (fixture,demo,dev), generaciones 1/2.
Data es exactamente R47: action=restart_required,
declaration_key=["fixture","demo","package.net.http","S"],
declaration_kind=type, old_generation=1, new_generation=2 y las dos listas
de cada fila. Los positivos anteponen solo `//x\n` al archivo de S,
actualizan integridad y mapas y aceptan aun con la instancia viva.
B39 usa `fn F()->I64{return 1;}` en a/bc.n anterior y ab/c.n nuevo,
con root.n=`mod a;` y a.n=`mod bc;` en la sesión, y root.n=`mod ab;`
y ab.n=`mod c;` en la edición; no cambia su cuerpo.
La solicitud declara la correspondencia entre las dos rutas. Primario y
previous_definition F son [3,4), generaciones 1/2 y campos R47; mantener
package.a.bc y cambiar solo trivia acepta. Comparar solamente el nombre F
o concatenar segmentos sin puntos debe hacer fallar esta regresión.

B37 pertenece a las fixtures de arnés de registros verificados del
planificador con tareas habilitadas. El arnés inyecta los registros
verificados de F y JoinT, sus identidades y el destino resuelto de la llamada;
no solicita resolución de nombres ni compilación de esa llamada al frontend.
F en package.net.http, generaciones 1/2, es un frame ordinario, no un estado
async/generador suspendido cuyo cuerpo cambie. La línea ilustrativa
`fn F()->I64{let x=JoinT();return x+1;}` y su variante con 2 en lugar de 1
son texto de mapa fuente del arnés, ASCII sin LF final, no entradas del
frontend. JoinT es el mismo registro de función Nexo de runtime en ambas
generaciones, firma ()→I64, efectos block; espera el resultado I64 de la tarea
T registrada por el arnés. Su espera está fuera de C08/C09. No hay otra tarea
F ni estado suspendido de F. Se conservan firma, efectos, catálogo e interfaz;
cambia body_hash de F. En ambos textos de 38 bytes, F ocupa [3,4), JoinT
[18,23), x declarado [16,17), x leído [33,34) y el literal editado [35,36).
El mapa usa file_digest de cada texto completo y asocia F con su registro.
La instantánea fija una arista de espera frame(F)→task(T), T sin operación
ejecutable, sin más aristas y cola de eventos vacía; no hay callback,
system_call, préstamo, región, generador ni value registrado como dependiente.
Data exacta: action=restart_required,
declaration_key=["fixture","demo","package.net.http","F"],
declaration_kind=function, old_generation=1, new_generation=2,
changed_fields=[body], live_kinds=[frame,task]; primario y
previous_definition usan [3,4) y los demás campos son R47.
El positivo cambia solo la cola: añade un evento determinista de terminación
de T con resultado 0 y orden 0; se entrega, satisface la espera y F completa
su suma/retorno en menos de 1.000.000 pasos IR, tras lo cual se instala la
generación 2. Una tarea ejecutable U sin arista desde F/T
no cambia el negativo; una SCC T→U→T sin eventos también lo conserva.

B36 usa el perfil de herramienta H1/H2 con módulos/structs públicos de F1
habilitados; no es una prueba del perfil F0. Usa fuentes ASCII de biblioteca
sin LF final: anterior
`pub struct S{pub A:I64;}` y nueva `pub struct S{pub B:I64;}`.
Paquete `(fixture,demo,dev)`, módulo package, generaciones 1/2; primario S nuevo
[11,12), previous_definition S anterior [11,12), data exacta de R47 con
action=restart_required, declaration_key=[fixture,demo,package,S] y los
campos de la fila. No hay callers ni otras diferencias de interfaz; la
instancia viva la crea el arnés: compila y evalúa `S(A:1)` mediante el
frontend normal en la generación 1, guarda el resultado como raíz propietaria
de la sesión y registra exactamente un value de S en esa generación. La
construcción ya terminó: no queda frame, préstamo ni tarea activos. La
variante sin vivos omite esa construcción. El positivo
sustituye la nueva fuente por `//x\npub struct S{pub A:I64;}` (\n es LF),
que conserva los registros semánticos y cambia el mapa fuente; el prefijo
son los bytes 2F 2F 78 0A. Se recompone package_digest y se actualiza su valor
esperado en el registro externo del arnés, conservando la revisión dev.
La carga de biblioteca no requiere Main.
B36 incluye además una prueba de prioridad (dos diferencias deliberadas,
no el negativo aislado anterior): la nueva fuente es
`pub struct S{pub B:Bool;}` sobre Linux x86-64, donde I64 y Bool tienen
layouts físicos distintos. Con el mismo valor viejo creado por el arnés,
exige NX-E1701/contracts_changed, changed_fields=[frame_layout,public_interface],
live_kinds=[value], declaration_kind=type y la misma clave, generaciones y
spans [11,12); sin valor viejo solo cambia live_kinds a []. Los demás campos
son los de R47. Esta interacción no sustituye los negativos aislados de
renombrado B36 y layout B19/B32.

Fixture literal B16, UTF-8 sin LF final en ambos archivos de revisión:

```nexo
fn F()->I64{return 1;}
```

```nexo
fn F()->Bool{return true;}
```

La sesión de prueba identifica paquete `(origen="fixture", nombre="demo", revisión="dev")`,
módulo `package`, declaración `F`, generaciones 1 y 2, sin callers ni estados
vivos. Ambas fuentes son bibliotecas válidas; no se solicita Main. Primario
en `new.n:[3,4)`, relacionado previous_definition en `old.n:[3,4)`;
ambos contienen exactamente `F`. `data` exacto:

```json
{"action":"restart_required","declaration_key":["fixture","demo","package","F"],"declaration_kind":"function","old_generation":1,"new_generation":2,"changed_fields":["signature"],"live_kinds":[]}
```

El positivo pareado sustituye únicamente el cuerpo de la primera por
`return 2;`: acepta recarga y F devuelve 2. La tabla no atribuye estos
diagnósticos a una implementación F0 existente.

B25–B27 usan las ubicaciones de F en las revisiones del arnés: primario
sobre el nombre nuevo y previous_definition sobre el anterior, con todos los
campos restantes de R47. B26/B27 fijan el mismo estado y secuencia de pasos;
cambiar únicamente la velocidad física no cambia clasificación. B28 usa
Diagnostic.profile_id=f1-library-test-work-v1 y L=A+W−1/L=A+W conforme
D16 R32, con ambos L dentro de su rango y A incluyendo argumentos y limpieza
reservada. No ejecuta el límite de R50 como condición de la regresión. La variante normal bootstrap-v1
certifica max Σ W_pre conforme R50 y cubre el caso de D16 R49: copias con
al menos cinco instrucciones administrativas por tick a los 200.000.000.000
ticks, sin agotar library_work mientras queda saldo lógico. Ese mínimo no
sustituye la cota superior certificada; no se rebaja library_work a T.

**R49.** Cada regla implementada requiere negativo aislado que cambie un solo
atributo, positivo pareado, campos/código/subcausa/span/token exactos cuando
haya diagnóstico, y mutación inversa que el test detecte. Se restaura por
file_digest (SHA-256 de cada archivo, R10); no se regeneran esperados desde el compilador bajo prueba. Fuzz de
fuente, IR y objetos debe terminar sin panic ni consumo fuera de límites.
El validador es distinto del implementador y contrasta también el HEAD
archivado con batería propia. Cada defecto corregido deja una regresión.
Las mutaciones nuevas se acotan a doce por ronda más las supervivientes, conforme D16 R48;
solo bloquean reglas explícitas. Ni el recuento ni el ejemplo R17 aprueban
un componente. Las puertas de dos revisores de F0 y aprobación de fases
por el propietario siguen vigentes.

## 9. Presupuestos, riesgos y pivotes

**R50.** N03 fija antes de N04/N05 un manifiesto de cargas y hardware para
medir B0/B1; N11 fija antes de N12 el de bootstrap. Límite operativo inicial de cada cadena
diversa: 24 horas de pared y 12 GiB de RSS máximo en la máquina fija de 16 GB
de RAM de referencia del roadmap §4.3; watchdog termina el proceso sin declarar resultado semántico.
Es una cota de investigación, no una garantía temporal del lenguaje. Se
registra trabajo, memoria, fallos y qué componente domina. El punto fijo
nativo debe cumplir además R30; pasar dentro de 24 horas no lo sustituye.

Se fija `bootstrap-v1`, perfil explícito por manifiesto para cualquier fuente
B1; N06 lo entrega antes de F2. Es una ampliación uniforme F1 de los límites
A2/J/F5/K, no un privilegio por nombre de programa ni una alteración retroactiva
de los perfiles F0. D16 §6 «Migración única de los cortes», R32, define la
contabilidad y la precarga que este perfil consume.

Una ejecución es una invocación completa del compilador/intérprete que produce
un stage, D1 o D2: desde entrada hasta salida, incluidos std, módulos, pasadas,
reintentos internos y limpieza. Todos los acumulados empiezan una vez y no
se reinician por archivo, llamada, fase, solicitud interna ni liberación.
R26 lanza invocaciones distintas para D1 y D2, cada una con su propio perfil;
sus 24 h son el presupuesto operativo conjunto. Una sesión REPL/LSP mantiene
los acumulados hasta salir; este perfil no promete una sesión ilimitada.
Cada nuevo check externo inicia sus cuotas de análisis, pero sigue consumiendo
trabajo/asignaciones del proceso que lo aloja. No se reinicia una ejecución
para ocultar que la prueba diversa excedió un límite.

| Contador de ejecución | Límite exacto |
|---|---:|
| Trabajo lógico A2/J/F5/K, T | 1.000.000.000.000 ticks |
| library_work, suma máxima W derivada abajo | 8.192.000.000.000.000 instrucciones |
| call_frames de usuario | 1.024 |
| library_frames administrativos | 4.096 |
| library_cells administrativas | 67.108.864 |
| live_value_cells | 67.108.864 |
| observation_bytes, incluidos cierres reservados | 268.435.456 |
| array_storage_bytes, pico vivo incluido crecimiento | 2.147.483.648 |
| array_allocation_bytes, acumulado de reservas | 64.000.000.000.000 |
| string_bytes, por String/constructor parcial | 67.108.864 |
| live_text_bytes | 1.073.741.824 |
| text_allocations, incluidos vacíos | 1.000.000.000.000 |

| Cuota de análisis de una compilación completa | Límite exacto |
|---|---:|
| source_bytes, suma del grafo en orden canónico de archivos | 67.108.864 |
| syntax_nesting | 256 |
| expression_tree_depth | 512 |
| array_type_depth (incluye List tras M09) | 256 |
| value_type_depth | 256 |
| function_count, incluye closures/gen/métodos | 262.144 |
| nominal_members | 4.194.304 |
| type_solver_steps global | 10.000.000.000 |
| ownership_flow_steps por invocable | 100.000.000 |
| ownership_flow_steps global | 100.000.000.000 |
| effect_fixpoint_steps global | 10.000.000.000 |
| ir_items global | 33.554.432 |
| trait_solver_steps global | 10.000.000.000 |
| generic_instantiations global | 1.048.576 |
| comptime_steps global | 1.000.000.000 |
| comptime_depth | 256 |
| comptime_memory_bytes vivos | 1.073.741.824 |

No quedan cuotas heredadas sin cifra. addition_chain_length/index_chain_length
siguen históricos sin emisiones independientes: no son dos límites ocultos.
Las unidades y prioridades son acumulativamente A2 §11, J §7, F5 §7 y K §8.
Source_bytes en este perfil cuenta cada entrada del grafo una vez, antes de
decodificar; el primario es el byte del archivo que excede el saldo global.
Tipos/propiedad/efectos/IR no reinician saldo al verificar transformaciones.
Trait_solver_steps cobra cada requisito/candidato/arista examinado, incluso
rechazado; generic_instantiations cada instancia distinta insertada, incluidas
derivaciones. Se activan los IDs ya reservados del catálogo, sin confundirlos
con runtime_budget_kind ni acreditar un solver incompleto.

**Derivación de reservas.** Cada receta r certifica W_r(q), incluidos
helpers, hooks concretos, fallos y limpieza, mediante D16 R32. Este perfil
exige además la cota amortizada `W_pre_r(q) <= 8192*t_r(q)`, donde t_r
son sus ticks lógicos públicos y W_pre incluye la reserva de destrucción
futura adquirida con cada propietario. Un destructor con cero ticks usa
ese crédito ya reservado; move lo transfiere, Pop lo entrega con su valor,
copia crea crédito nuevo y no se reinicia al liberar. Los helpers internos
se incluyen una vez en W_pre, no obtienen activaciones gratuitas separadas.

El certificado se obtiene del CFG de biblioteca y la receta lógica, no de
calibrar el intérprete: suma secuencias, máximo de ramas y cotas de bucles;
comprueba por inducción conservación de créditos de cada propietario.
La descomposición cubre copia/traslado/flags/barrido de capacidad y limpieza,
incluidos slots vacíos, bajo la cota global 8.192 instrucciones por tick.
En recorridos estructurales acotados a 256 niveles admite hasta 32 pasos
administrativos por nivel y unidad cobrada, **sumados todos los helpers**.
Formato escalar, crecimiento y validación UTF-8 presentan su desigualdad
directa con la misma cota, incluyendo casos vacío/error. La profundidad de
tipo no acota la altura de grafos indirectos: para ellos se exige medida
iterativa y prueba propia. Sin esa prueba no se activa la migración.
Comparadores, hashing/ordenación y hooks no certificados son código Nexo
ordinario con ticks; no se esconden recorridos cuadráticos como administración.

Para operaciones heredadas A2/J/F5/K no se añade trabajo lógico para fabricar
la desigualdad: si el cuerpo no la satisface, se corrige el cuerpo o se revisa
el perfil, conservando las pruebas de equivalencia. Las APIs nuevas de reserva
cruda/capacidad consumen la receta de D16 R32; así su barrido y destrucción
no se financian con una llamada de coste constante. Los bytes de bitmap y
metadatos conservan contabilidad administrativa separada.

Con T=1.000.000.000.000, toda secuencia admitida cumple
`max Σ W_pre_r(q_r) <= 8192*Σ t_r <= 8192*T =
8.192.000.000.000.000`. Esta es la derivación del límite library_work,
incluida la limpieza reservada, no un techo arbitrario igual a T. Para una copia
de cinco instrucciones administrativas por tick,
5·T = 5.000.000.000.000 < 8.192.000.000.000.000.
M09–M11a publican certificados de sus recetas; N06
los integra/verifica y publica su composición sobre R26/R28; una cota sin demostrar bloquea
antes de ejecutar. F/C se certifican por máximo de pila/ocupación, dentro
de 4.096/67.108.864, sin deducirlos de W ni del RSS.

La reserva acumulada Array/List también tiene una cota independiente del
reloj: literal/copia/JoinLists reservan 32*N bytes y pagan al menos N ticks
de celdas (más cabecera); crecer desde C=N>0 reserva 64*N bytes y paga N
ticks de traslado; primer Push reserva 32 bytes y paga al menos un tick.
Reservas de hijos tienen sus propios cargos; mover no reserva. Las APIs
nuevas que reservan capacidad explícita pagan los slots solicitados como
indica el párrafo anterior. Por tanto
`array_allocation_bytes <= 64*T = 64.000.000.000.000`.
Cada búfer String iniciado paga al menos un tick de cabecera:
`text_allocations <= T`. No se devuelve ningún acumulado al liberar.
Una ejecución que aún tiene saldo lógico no aborta por esos acumulados solo
por durar 24 h; podría agotar pico vivo u otra cuota independiente, con su
diagnóstico propio. No se usa el reloj para reiniciar ni ampliar saldos.

Todas las cifras y sus observed=limit+1 caben en el UInt JSON de A2 §13.6
(max=9.007.199.254.740.991). Los contadores usan U64 con aritmética comprobada;
comparan saldo antes de sumar y no dejan desbordar un producto intermedio.
No se amplía UInt, schema_version ni se serializa un entero inexacto.
Una solicitud estructural agregada mantiene observed=total candidato como
F5/K; el tamaño candidato exacto máximo de ese perfil también debe caber en
UInt antes de activar la receta. Un tamaño irrepresentable se rechaza en la
comprobación de tamaño previa, sin fabricar un observed truncado.

**Escala constructiva requerida.** El candidato se organiza por tablas de
IDs y buffers reutilizados de R14/R18, sin copiar toda la IR al insertar un
nodo. N06/N08–N11 deben acreditar el siguiente sobre de compilador completo,
incluidos std, frontend, checker, intérprete y emisor, antes de fijar C0:

| Magnitud del candidato de hasta 100.000 líneas funcionales | Cota de construcción que cabe en bootstrap-v1 |
|---|---|
| Fuente, invocables, miembros/instancias | <=16.777.216 bytes; <=65.536 invocables; <=1.048.576 miembros; <=262.144 instancias |
| AST/IR por IDs | <=1.048.576 nodos AST y <=16.777.216 registros IR; <=32.000.000 celdas lógicas vivas, incluidos campos, cabeceras, índices y temporales |
| Búferes y texto | <=1.073.741.824 bytes de slots vivos incluyendo coexistencia; <=268.435.456 bytes de texto y <=16.777.216 por String |
| Recorridos explícitos | <=512 frames de usuario, profundidad sintáctica/tipos <=128; expresiones <=256 |
| Análisis | <=5.000.000.000 pasos tipos/traits/efectos por contador; <=50.000.000 propiedad local y <=50.000.000.000 global; comptime <=500.000.000 pasos, profundidad <=128, memoria <=536.870.912 |
| Ejecución completa de compilación | <=500.000.000.000 ticks y <=134.217.728 bytes de observaciones; W/F/C certificados anteriores |

La capacidad es constructiva con tablas por columnas escalares/IDs:
16.000.000 slots escalares/IDs reservados requieren 512.000.000 bytes
lógicos (32 por slot de F5 §4.2), no 32 bytes por struct de tamaño arbitrario.
Un búfer que crece de 8.000.000 a 16.000.000 slots, coexistiendo con
8.000.000 slots de otras tablas, ocupa (8.000.000+16.000.000+8.000.000)*32
= 1.024.000.000 bytes, dentro del sobre de 1.073.741.824. Crecer de
16.000.000 a 32.000.000 slots no cabe en ese sobre; N11 debe acreditar
la distribución por tablas y sus máximos, sin usar ese crecimiento como
ejemplo admisible. Un ítem IR puede ser operando/referencia
inline sin nuevo slot; cada campo real y sus celdas sí se contabilizan.
AST, símbolos y estados se incluyen en ese total y en 32.000.000 celdas
vivas, no se suman como memoria gratuita. N11 debe publicar la distribución
por tabla y sus picos; el número de ítems no demuestra por sí solo memoria. Se liberan tablas de fase solo después de
su último consumidor; se emite objeto incrementalmente con buffers explícitos.
Cada fila tiene margen frente al límite correspondiente, y los acumulados
se deducen de T, sin el tope F0 de 67.108.864 bytes que agotaría las copias.
Esto fija una representación y un tamaño realizables para un compilador,
no prueba que las fuentes S todavía no implementadas ya satisfagan las cotas
de análisis/trabajo: N11 debe adjuntar contadores completos de S y cada
prueba diversa; sin esa evidencia R51 bloquea. Una fixture corta o contar
líneas no acredita la puerta.

La precarga conserva el orden acumulativo K §8.2: trabajo, frames, celdas,
array_storage_bytes, array_allocation_bytes, string_bytes, live_text_bytes,
text_allocations y observaciones; después reserva W/F/C antes de mutar.
Déficit lógico mantiene código, observed y span originales: R0001 para trabajo,
R0002 para las ocho cuotas de usuario. La ampliación acumulativa única de
R0002/runtime_budget_kind es D16 §12.1; library_work añade la novena.
Su déficit en precarga usa observed=limit+1 y llamada pública activa, sin
mutación ni producto válido. Con certificado conforme y saldo lógico ese
déficit no es alcanzable en el perfil normal; un desbordamiento de la cota
certificada es InvalidIr. Los negativos lo aíslan mediante inyección de
contador del arnés, sin atribuir a usuario una ejecución imposible.
Limpieza ya reservada se ejecuta una vez; no falla a mitad de traslado.

R28 usa este perfil bajo intérprete Rust candidato, R26 bajo el archivado y
las cadenas posteriores bajo intérprete Nexo anterior. El nativo lo aplica
en pruebas instrumentadas; R30 también mide sin instrumentación. El corpus
alojado conserva su perfil propio. Eliminar la instrumentación conserva cada
transición comprometida: el perfil solo puede abortar/rechazar antes de
compromiso, nunca cambiar un valor, orden, efecto o regla del lenguaje para
hacer caber la ejecución. El aborto controlado de cuota conserva limpieza
reservada de A2; no significa capturar C10/OOM. Watchdog 24 h/12 GiB termina
la validación como INCONCLUSA, sin diagnóstico semántico ni producto válido.

**R51.** Un fallo de seguridad, divergencia semántica, dependencia prohibida
o falta de punto fijo bloquea inmediatamente la retirada correspondiente.
Para N04/N05, dos ejecuciones reproducibles de la misma carga N03 que
excedan las cuotas del perfil disponible o el presupuesto operativo R50
permiten diferir el frontend Nexo; se registra la carga, cuota y evidencia,
y se mantienen los entregables Rust F1 y la obligación N04/N05 pendiente.
M09 migra todo lo existente y los entregables diferidos usan List, según
R19. Esa postergación no aprueba F1 ni exime de sus puertas originales; la
decisión de fase sigue siendo del propietario. N08 espera N05 aun diferido.
Tras dos ejecuciones reproducibles de la misma revisión que incumplan R30
o R50 se difiere la sustitución, conservando la última cadena verde. Se
permiten como máximo dos cortes de corrección de la causa medida antes de
registrar un pivote: simplificar representación/algoritmo dentro de la misma
semántica, o diferir autoalojamiento a una entrega posterior a F6. No se
mantienen dos compiladores de producción ni se promete independencia lograda.

**R52.** Se abandona el candidato concreto si después de esos dos cortes
sigue infringiendo los mismos límites o necesita relajar propiedad/efectos
para funcionar. Se conserva evidencia y se retira el candidato del árbol
activo. La directiva de autoalojamiento no se abandona por decisión técnica
implícita: renunciar al objetivo requiere enmienda explícita del propietario.
Puede continuar el roadmap original con Rust/LLVM y dependencias declaradas;
el retraso no cambia puertas F0–F6 ni permite declarar la independencia final.

**R53.** Para backend propio, si R38/R39 falla en dos mediciones repetibles,
se conserva LLVM y se corrige el cuello identificado. Después de dos cortes
sin cerrar la misma puerta se difiere su sustitución como extensión posterior,
sin rebajar rendimiento ni quitar plataformas. Cranelift no reemplaza
automáticamente ese objetivo. El riesgo de escribir optimización competitiva
se asume como alcance adicional, no como razón para degradar la primera 1.0.

**R54.** Riesgos específicos y respuesta obligatoria:

| Riesgo | Señal | Respuesta |
|---|---|---|
| Copias físicas B0 cuadráticas | R30 excedido por tablas/cadenas | Cambiar representación a IDs y buffers D16; medir antes de añadir CoW |
| Error común a Rust y Nexo | Oráculos acuerdan pero fixture normativa difiere | Reducir caso, corregir norma o ambos compiladores; repetir equivalencia |
| Semilla comprometida | manifest_digest/file_digest/firma o compilación diversa discrepa | Bloquear publicación, reconstruir desde último origen auditado |
| ABI/cargador propio incompleto | Falla solo una plataforma o Python | No distribuir ese backend ni retirar el anterior |
| dependency_hash omite una dependencia | Cambio de trait/default/layout reutiliza artefacto | Invalidar caché afectada, regresión T09; nunca desactivar checker |
| Puntos seguros no alcanzables | Agotamiento determinista R44 | Requiere reiniciar; conservar código/estado anteriores |
| Falta hardware/revisión | Evidencia nativa o independiente ausente | NO EVALUADO, no inferir PASS de emulación ni autoevaluación |

## 10. Subtareas y dependencias

**R55.** Los N son tareas futuras, no pending_scope F0. Consumir una M
significa verificar su entrega, no repetirla. Sufijos subdividen cortes;
el orden de filas es topológico y cada fecha sigue a sus prerrequisitos.

| Tarea | Depende de | Entrega y aceptación | Momento máximo |
|---|---|---|---|
| N01 | — | Conciliar enmiendas de D16 §12 (Enmiendas propuestas y compatibilidad) y D17 R58 (D17 §12, Enmiendas propuestas y alternativas rechazadas) | F0, antes de M02 y congelar F1 |
| N02 | N01; M01 | Implementar tabla de funciones, DefKey desde descriptores explícitos R09 (integración con resolvedor F1/D13 en N06), body_hash/interface_hash/dependency_hash y clave de artefacto IR01/IR02, layouts y ABI R07–R12; retirar representaciones anteriores y entregar vectores | Inicio F1, antes de M02/M03/M05 y N07a |
| N03 | N01; 0002→0003→0005→0004 completos | B0, corpus independiente, límites y manifiesto de cargas/hardware R50 | Antes de cerrar F1 |
| N04 | N03 | Lexer Nexo, tokens/spans/fronteras | Antes de cerrar F1, salvo R51 |
| N05 | N04 | Parser/CST/formateador Nexo R16 | Antes de cerrar F1, salvo R51 |
| N06 | N03; M07–M12 parte F1, incluidos M09a–c y M11a; F1/D08/D13 | Integrar/verificar B1/S0-std y mapas/ordenación/internado/SHA-256 de M09b/M09c; verificar migración B0→List de M09; conectar descriptores del resolvedor F1/D13 a DefKey N02; entregar perfil R50 y UTF-8 inválido | Antes de entrar F2 |
| N07a | N02,N06 | ProcessRun R13, incluido proveedor del intérprete Rust de R26, proveedores/manifiestos cuatro destinos, errores/efectos | F2, antes de N07b |
| N07b | N07a | LLVM Rust, emisión/enlace reproducible y driver; lockfile F2 | F2, antes de N07c |
| N07c | N07b; M12 nativo/M13a–c; restantes entregables F2 | Integración y dossier E2E-F2 R31 sin reimplementar F2 | Antes de C0 |
| N08 | N05,N06 | Checker Nexo, fila 2 R28 y fuzz | Antes de N09 |
| N09 | N02,N08 | Lowering/verificador Nexo, fila 3 | Antes de N10 |
| N10 | N09 | Intérprete Nexo sobre runtime D16, fila 4 | Antes de N11 o H1 |
| N11 | N07c,N10 | Emisor LLVM/driver/herramientas Nexo, R28/R21; manifiesto de cargas/hardware de bootstrap, distribución por tabla y picos, contadores completos de S y cada prueba diversa conforme R50; congelar C0 | Antes de N12 |
| N12 | N11 | Stage1–3, dos entornos/destino, R24/R26/R30 | Antes de N13 |
| N13 | N12 | Retirada Rust/Cranelift y cadena V→V+1 | Tras F2, antes de cerrar F3; diferible R51 |
| N14 | N02,N07c,N10 | Generaciones de la tabla única para H1/H2 y H1 solo en intérprete Nexo, R43–R47 y B15–B56 aplicables | Después F2, antes de distribuir H1 |
| N14a | N14 | Catálogo de carga H2 R45, ImageSeal con F_ADD_SEALS/F_GET_SEALS y máscara observada, y cargador ELF64 Nexo Linux x86-64/arm64 sin libc, corpus de segmentos/relocaciones/errores, permisos/coherencia y proveedores macOS/Windows R40; evidencia nativa antes de habilitar cada destino | Antes de N14b/N21 |
| N14b | N14a,N11 | H2 LLVM sobre runtime Nexo común, corpus y evidencia nativa | Después F2, antes de distribuir H2 LLVM |
| N15 | N07c; M14a–d,M15; F3–F6 completos | Dossier E2E-F6 R32 | Antes de N16 |
| N16 | N13,N15 | Propio Linux x86-64/ELF R38 | Después F6, antes de distribuirlo |
| N17 | N16 | Windows x86-64/PE, ABI/Python/limpieza/atomics | Antes de distribuirlo |
| N18 | N17 | macOS arm64/Mach-O en ARM real | Antes de distribuirlo |
| N19 | N18 | Linux arm64/ELF, matriz completa | Antes de retirar LLVM |
| N20 | N19 | Enlazador Nexo, retirada del enlazador externo y Wasm sin LLVM; LLVM nativo se retira en N22 | Antes de independencia |
| N21 | N14a,N19,N20 | H2 propio sobre runtime Nexo único R45 | Antes de distribuir H2 propio |
| N22 | N19,N20,N21 | Rendimiento/estudios, R27/R39 y retirada LLVM | Antes de sustitución de producción |
| N23 | N22 | Inventario R40 y construcción limpia desde V | Antes de independencia |
| N24 | N21 | ADR H3, migración fallable; mientras tanto reinicio | Antes de soportar H3 |

M12 tiene dos criterios explícitos, no dos implementaciones: su artefacto
fuente/S0-std F1 está completo para N06 sin ejecución nativa; N07b aporta
emisión y N07c verifica el criterio nativo F2 del mismo artefacto. La palabra
«parte F1» en N06 excluye ese criterio F2, sin arista M12→N07b para S0-std.
N06 no espera N05 ni M12 nativo: diferir frontend Nexo no bloquea S0-std/F2.
M16/M17 son agregados del mapa R04, nunca dependencias de vuelta. N14 espera
N10: H1 no se implementa en Rust para perderlo al retirarlo. N14b es una rama
que no bloquea N13; N21 reutiliza su infraestructura si existe, sin exigir
H2 LLVM. N22 espera N21: al retirar LLVM conserva H2 mediante el backend propio;
N23 hereda esa dependencia, N24 no es puerta F0–F6. Las capacidades F3–F6
se incorporan al compilador Nexo único tras N13 respetando R25.

## 11. Interacciones y alcance del prototipo

**R56.** Matriz de conservación:

| Autoridad | Consecuencia D17 |
|---|---|
| P01/P04 | Un lenguaje B, comparación por observaciones definidas; optimizar no borra fallos ni limpieza |
| P02/P03 | AST por IDs propietarios; semilla no dispensa copia, move, préstamos o exclusividad |
| P05/P06 | ABI y costes físicos separados de tipos/efectos y cuotas del simulador |
| P07/P08/P09 | Base de confianza explícita, sin servicio remoto obligatorio; diagnósticos causales únicos |
| D01/D14/D15 | Solo `.n`, modos B, retorno explícito, CST/AST y formato conservadores; ninguna sintaxis de bootstrap |
| D02 | Errores tipados y aborto del proceso; no se captura panic del compilador para continuar una cadena inválida |
| D03 | Comptime acotado, objetivo explícito; generación externa con entradas declaradas |
| D04 | Rust/LLVM iniciales, interfaz IR y Cranelift de contingencia; enmiendas temporales de D17 §12 |
| D05 | IDs sin reutilización bastan para B0; no se fingen arenas/regiones de F3 |
| D06 | Semillas/fuentes/licencias auditables dentro del repositorio privado; ninguna publicación implícita |
| D07 | Checked arithmetic, float estricto, evaluación y destrucción preservados por todos los emisores |
| D08 | Genéricos por requisitos, una impl canónica, límites de instanciación; backend no elige traits |
| D09 | CoW/publicación/atomics conservan identidad y coste; propio no baja atomics a accesos locales |
| D10 | Spans bytes, String/Bytes distintos, REPL versionada; no modifica layouts vivos |
| D11 | Capturas y cuatro resúmenes conservados en IR/ABI/interface_hash; ningún préstamo prolongado por portar a Nexo |
| D12 | Async no es requisito B1; sí obligación de backend final y restricción de recarga de estados |
| D13 | Clave nominal, firmas públicas, paquetes/lockfile e invalidación transitiva coherentes |
| 0001 | Antecedente experimental, nunca semilla C0 elegida por antigüedad |
| 0002 | IR/verificador/intérprete escalar y cuotas conservados; no deserialización ejecutable nueva |
| 0003 | Option/match/closures/generadores antes de colecciones; canon de exclusiones intacto |
| 0005 | Operadores/control/Array dinámico antes de texto; B0 usa esa representación provisional |
| 0004 | Texto/nominales/derivación cerrada, no Bytes, genéricos ni E/S real inventados |
| D16 | Una biblioteca Nexo sobre intrínsecas mínimas; migración única de colecciones, no catálogo paralelo |
| F4/roadmap §6.4/roadmap §6.6 | Perfiles C/Python son interoperabilidad voluntaria, no dependencia de std; Wasm se conserva |

**R57.** En F0 entran decisiones, vectores documentales y, cuando sus
prerrequisitos estén cerrados, experimentos B0 con el mismo intérprete y
límites; M01/N01 fijan inventario y diseño. N02 implementa tabla/layouts/IR01/IR02/ABI
al inicio F1; M02/M03/M05 consumen después, sin atribuir ABI nativa a F0. No entran
stage1–3 nativos, SDK, linker, recarga, runtime real ni resultados competitivos.
Una capacidad excluida del perfil completo F0 conserva NX-E0006, su
feature_id existente y `pending_scope=later_adr` conforme 0003 §2.2.
Solo una obligación incluida en un corte posterior F0 concreto puede usar
`implementation_limit` y el ámbito de ese corte (J/F5/K). Ningún N de esta
tabla se emite como pending_scope, ni se añaden feature_id self_host/backend.
Una opción de herramienta todavía inexistente no se disfraza de error de
sintaxis del programa Nexo.

## 12. Enmiendas propuestas y alternativas rechazadas

**R58.** Las citas de base siguientes son literales; las sustituciones se
aplican mediante este ADR y acuerdos, nunca editando la revisión 0.2.
D16 R27 remite calendario y cadena a D17; D17 R20 define S0-std/C0 y D17 R24
la igualdad de productos.

| Autoridad y texto exacto | Enmienda propuesta |
|---|---|
| D04: «**Decisión:** aceptar Rust y una representación intermedia tipada de Nexo.» | Rust se acepta como implementación de arranque C0; R20–R29 fijan sustitución por Nexo. La IR tipada permanece. |
| D04: «F2 añade **LLVM como backend nativo principal**, con configuraciones de desarrollo y optimización.» | Se conserva literalmente para F2; no se adelanta backend propio a esa puerta. |
| D04: «La producción conserva LLVM; un backend de desarrollo promovido debe pasar el corpus completo en los destinos donde se distribuya.» | Producción conserva LLVM hasta R39; después usa únicamente el propio. Se conserva la exigencia de corpus por destino. |
| D04: «F0 especifica la interfaz de backend, ABI interna y artefactos de depuración; F2 construye un pequeño adaptador experimental a Cranelift para escalares, llamadas y limpieza, con pruebas diferenciales.» | Se conserva la preparación F0/F2 y se añade retirada R41, no un adaptador permanente. |
| Roadmap §11.2, Compilador: «Backend propio desde cero, dos backends nativos completos obligatorios desde F2 y hot reload universal de objetos vivos.» | Backend propio se compromete como extensión después de E2E-F6, no como requisito inicial 1.0. Siguen excluidos dos backends completos obligatorios desde F2 y hot reload universal. |
| Roadmap §11.1, Herramientas: «intérprete, LLVM, servidor incremental, REPL, pruebas/doctests, formateador, LSP, diagnósticos estructurados y rayos X de costes.» | LLVM es la dependencia inicial de esa entrega y se sustituye bajo R39 conservando todas las herramientas. |
| F2: «**Fuera de alcance.** Backend rápido completo por defecto, hot reload avanzado, compatibilidad Python arbitraria, callbacks extranjeros, modificación in situ de arrays Python y gestor público de paquetes.» | Se conserva F2; H1/H2 se añaden después, H3 sigue sujeto a ADR. |
| F0-0002 §9.4: «Este corte no fija ABI nativa, layout portable de recursos, formato binario de módulos ni adaptadores LLVM/Cranelift; esas obligaciones D04 siguen pendientes.» | R07–R12 fijan los contratos de diseño faltantes; N02 implementa tabla de funciones, descriptores por objetivo, IR01/IR02 y ABI en F1 desde N01/M01; M02/M05 consumen después, antes de F2. No se atribuyen al alcance implementado de I04. |
| A2 §13.6: «`data` admite únicamente las claves registradas más abajo. Otros campos requieren revisar la versión del esquema.»; «Los doce valores de `RelatedSpan.role`, incluido `preceding_expression`, y sus reglas de emisión se enumeran exclusivamente en §13.15.» | En el perfil H1/H2, schema_version=2 amplía Diagnostic.stage con reload, registra operation=reload.classify y RelatedSpan.role=previous_definition, y añade revision: String (file_digest de bytes de fuente, R10) a Span; conserva file/start/end y ordena relaciones por file/revision/start/end/role. NX-E1701 exige exactamente siete claves data: action: String="restart_required", declaration_key: lista de cuatro String R09, declaration_kind: function/type/constant/method/trait/alias, old_generation/new_generation: UInt, changed_fields/live_kinds: listas cerradas R47. declaration_kind amplía su fila existente solo para NX-E1701; no sustituye sus valores/obligaciones para NX-E0104. Afecta schema_version, Diagnostic.stage/operation, Span, orden de relaciones, registro de roles A2 §13.15 y filas data indicadas; añade las seis claves restantes sin retirar ninguna de A2/J/F5/K/D16; esta fila es acumulativa y disjunta de la fila A2 §13.6 de D16 R55, incluido primitive_id y la unión de nueve runtime_budget_kind de D16 §12.1. Los perfiles históricos conservan versión 1. |
| F0_ACUERDOS §9.7: «sus niveles H1 a H3 siguen a F2» | R42–R46 concretan orden y rechazo; H3 no obtiene implementación ni plazo por esta frase. |
| D16 «4. Catálogo cerrado de confianza», R22: «Ampliar los selectores, por ejemplo para tareas o carga ejecutable, exige otra revisión del catálogo antes de F5/H2; no hay selector «otros».» | Ampliación propuesta: ProcessRun R13 en N07a antes del cierre F2 y carga ejecutable R42/R45 en N14a (ImageCreate/Seal/Reserve/MapFile/Entry/Protect/Unmap Linux y NativeImageOpen/Lookup/Close macOS/Windows) antes de N14b/N21; revisión de catálogo obligatoria, sin selector implícito. El contrato único de ImageSeal, incluido retorno, errores y costes, está definido en D17 R45. N14a posee sellado y observación en R04/R55, consumiendo el despacho M11; ninguna capacidad queda sin dueño. |

La ampliación de autoalojamiento no cambia licencias, hace público el repo,
aprueba fases ni elimina revisiones. F0-CODIGOS deberá integrar R47 antes
de emitir NX-E1701; mientras tanto es una reserva propuesta, no un catálogo
paralelo vigente.

Enmiendas adicionales propuestas; los títulos citados identifican las
secciones vigentes y sus propietarios se fijan en R04/R55:

| Autoridad y sección exacta | Enmienda propuesta |
|---|---|
| Roadmap «F2. Compilación, REPL y entrada temprana desde Python»: «Exportación explícita de funciones síncronas: booleanos, enteros de anchura declarada, flotantes, cadenas y búferes de lectura del conjunto soportado.» | Adelantar U8/U32/U64, conversiones y bits a F1/M05 para file_digest/package_digest y Bytes de arranque; exportación Python sigue F2. |
| Roadmap «4.2. Plataformas obligatorias y niveles de evidencia»: «F0 fija versiones mínimas de sistema, libc/CRT, SDK, objetivos de CPU y versiones de Python/LLVM/Rust.» | Conservar inventarios históricos; tras N23 quitar libc/CRT salvo libSystem mínimo R40; perfiles C/Python aparte. |
| Acuerdos «9.8. Recarga en caliente (hot reload)», regla 2 | H1/H2 exigen interfaz idéntica incluidos efectos inferidos; firma/efectos/contratos diferentes requieren reinicio aunque llamadores sigan válidos. R43/B16 endurecen expresamente esta regla. Fuente inválida conserva errores ordinarios. |
| Acuerdos «9.8. Recarga en caliente (hot reload)», regla 3: «La sustitución se aplica solo en puntos seguros (por ejemplo, entre iteraciones del bucle de eventos). Los marcos de llamada ya activos terminan con el código anterior.» | R44/R47 concretan quiescencia y agotamiento por trabajo determinista, sin timeout de pared; no atribuyen un timeout a la regla original. |
| Acuerdos §9.3(a): «sin pasar por libc ni por bibliotecas en C.»; D16 «4. Catálogo cerrado de confianza» R22 y «5. Distribución, identidad y arranque» R26 | Sustituir ese alcance por «sin pasar por libc ni por bibliotecas en C, salvo las APIs nativas de libSystem en macOS enumeradas en D17 R40». Lista cerrada y auditoría M11/M15/N23; no autoriza libc como std. |
| D16 «4. Catálogo cerrado de confianza», C01–C02; «11. Subtareas y dependencias», M05 | Catálogo cerrado D16 R18/C01–C02: U8/U32/U64, IntConvert y siete hojas de bits en M05/F1; R13 solo cita. SHA-256 Nexo pertenece a M09b, N06 integra. |
| D16 «6. Migración única de los cortes», R32 | bootstrap-v1 R50 publica todas las cifras de análisis/ejecución y deriva library_work y acumulados Array de T; conserva certificación/precarga W/F/C y permite R0002/library_work de inyección antes de mutar; N06 lo entrega antes F2 para R28/R26. |
| D16 «5. Distribución, identidad y arranque», R27; «11. Subtareas y dependencias», M12/M16/M17 | S0-std pertenece a M12, C0 a N11 tras F2; R23 define stage1 = C0.compilar(S, L, E). M16/M17 se descomponen según R04 sin duplicación ni aristas de vuelta; M12 nativo se consume en N07c; N07b entrega emisor/driver, sin apropiarse de M12. |
| D16 «7. Invariantes de IR, ABI y checker», R35–R37; «11. Subtareas y dependencias», M02/M03/M05 | N02 implementa tabla de funciones, layouts, IR01/IR02 y ABI con N01/M01 como únicas dependencias; M02/M03/M05 consumen después en F1. R10 define body_hash/interface_hash/dependency_hash y clave de artefacto; N14 añade generaciones solo tras N10. |
| D16 §9 (Diagnósticos propuestos y matriz de aceptación), R45/V18 | Enmienda propuesta, sin editar D16: el positivo V18 de D16 R45 exige también dependency_hash idéntico. La fixture compartida por B20 de D17 R48 y V18 de D16 R45 fija conjuntamente igualdad de body_hash/interface_hash/dependency_hash/clave de artefacto, conservando los negativos de integridad, incluido V13 de D16 R45, y sus campos. N02 entrega registros e invalidación; M03 audita su consumo. |

**R59.** Alternativas rechazadas:

1. Semilla basada solo en IR congelada: necesitaría un consumidor confiado
   nuevo y contradiría la frontera de 0002; binario más fuentes/manifiesto
   permite arrancar sin fingir que un dump es una prueba.
2. Conservar Rust indefinidamente como frontend, runtime o helper: incumple
   acuerdos §9.1 aunque la CLI esté escrita en Nexo. Se conserva únicamente evidencia
   histórica y una ruta de auditoría fuera de la construcción normal.
3. Reescribir todo antes de tener biblioteca/ABI: impide distinguir fallo de
   lenguaje, runtime y portado. Se fijan B0/B1 y equivalencia por componente.
4. Mantener dos frontends seleccionables o un puente permanente de AST:
   duplicaría reglas y fixtures. La integración del portado es atómica R29.
5. Declarar confianza por punto fijo: una cadena puede reproducir un defecto;
   se exige ruta diversa y se declaran sus partes comunes, sin prueba universal.
6. Backend propio antes de F2 o solo un x86-64 exitoso para retirar LLVM:
   desplaza las puertas de producto o elimina destinos. R31–R40 las conservan.
7. Transpilar a C/Rust o depender de ensamblador externo final: sustituye una
   dependencia por otra y puede perder contratos; se emiten objetos estándar.
8. Inventar formato ejecutable propio: añade cargador, depuración e interop
   sin necesidad; ELF/PE/Mach-O y Wasm mantienen las plataformas comprometidas.
9. Reducir comprobaciones para igualar tiempos: viola P01/P04. Si la causa es
   algoritmo/representación, se corrige ahí; si no cierra, se difiere la entrega.
10. H3 automático, parchear frames o casar funciones por offsets/body_hash:
    puede cambiar identidad/limpieza con estado vivo. Se rechaza y reinicia;
    el futuro ADR de migración debe aportar su propio modelo verificable.

**R60.** Quedan por obtener evidencias, no elecciones arquitectónicas abiertas:
file_digest/versiones reales de C0 y SDK; disponibilidad de hardware nativo;
resultados R16/R24/R26/R30/R38; descriptores y tabla implementados por N02 y consumidos por M02/M05; y conciliación
revisada con D16. Su ausencia bloquea las tareas indicadas. Ninguna se
sustituye por afirmar que la arquitectura ya está implementada o validada.
