# F0-0007 — Inferencia diferida de constructores Result

Estado: **decisión de arquitectura revisada de forma independiente; sin implementación ni aprobación de fase**.
Fecha: 2026-11-13. Flujo A; análisis documental y lectura de código, sin ejecución.
Base inspeccionada: rama `i07c-robustez`, HEAD `db58092267f407de5001ad05f9b715c36e7741f9`.

## 1. Autoridad, problema y alcance

Se decide NB-I11b-11 de `Pending.md` y su relación con A/T01. A2 designa
[F0-0002](F0-0002-second-slice.md); J, [F0-0003](F0-0003-third-slice.md);
F5, [F0-0005](F0-0005-operators-control-collections.md). La superficie es
[F0-D15](F0-D15-sintaxis-oficial.md). No se modifica la base 0.2.

| Autoridad vigente | Lo que exige y frontera de esta decisión |
|---|---|
| A2 §4.1 | `Ok` necesita E y `Err` necesita T; ningún tipo por defecto; NX-E0113 si falta contexto local suficiente. |
| A2 §4.1.1 | Variables monomórficas y aliases persistentes; anotaciones, inicialización, asignaciones, argumentos, operadores y retornos aportan restricciones transitivas, también hipotéticas. Never, incógnita y contaminación son distintos. |
| A2 §4.1.2, puntos 1–2 | Orden de comprobación fuente, primera igualdad compatible que determina cada componente, rechazo sin cambiar el prefijo aceptado; igualdad compuesta atómica. Un conflicto puede completar sus datos después sin cambiar de sitio ni volverse válido. |
| A2 §4.1.2, puntos 3–5 | Un dominio no unitario no elige tipo. El contexto se distribuye según su disponibilidad al entrar en cada nodo; no se redistribuye retrospectivamente. En AssertEq, igualdad antes del dominio y conflicto sobre el segundo argumento completo. |
| A2 §13.17; §14.1, F2-S09/F2-S10 | Indeterminación tras **todo** el contexto local; rutas por posición del tipo del constructor. Ok sin contexto da NX-E0113; Array irresuelto conserva NX-E0103. |
| A2 §4.1.2, punto 2; A2 §14.10, TD01–TD07; `Pending.md`, registro de NB-I11b-11 | La fijación en orden de fuente preserva el prefijo compatible y las procedencias (A2 §4.1.2, punto 2; TD01–TD07 las protegen). Que no autorice a congelar incógnitas en el primer uso es una deducción de este ADR: el registro de `Pending.md` plantea esa cuestión como pregunta abierta y esta decisión la cierra. |
| J §3.1; F5 §4.1 | None reutiliza la inferencia I07c/I08; Array reúne también restricciones de usos posteriores. |

No hay contradicción normativa entre «todo el contexto local» y la fijación
monótona: se incorporan igualdades posteriores compatibles, sin reemplazar las
anteriores ni conservar partes de una igualdad rechazada. Sí contradice estas
reglas emitir NX-E0113 al visitar un constructor sin contexto inmediato.
El hueco operativo es cómo conservar sus componentes pendientes, ligarlos a usos
posteriores y cerrar sus obligaciones con las mismas reglas que Array. A/T01
ya tiene respuesta normativa en A2 §4.1.2, punto 1; su test registra un defecto,
no una excepción autorizada.

Esta decisión favorece inferencia local predecible y diagnósticos estables para
personas y agentes (`docs/VISION.md`, «Qué es Nexo»). No añade genéricos de usuario,
conversiones, sobrecargas, tipos por defecto ni inferencia entre cuerpos de funciones.

## 2. Causa raíz verificada por lectura

En `src/check/types.rs`:

- `Checker::check_intrinsic`, rama `Ok | Err`, comprueba el payload con el
  contexto Result inmediato. Si no lo recibe, descarta el tipo sintetizado del
  payload (`let _ = payload`) y devuelve NX-E0113 directamente; no crea variable
  para el componente opuesto, enlace de expresión ni obligación de cierre.
  Así el inicializador falla antes de que `statement` pueda conservar un binding
  pendiente válido; las restricciones de sus usos posteriores no lo recuperan.
- La rama `Expr::Array` de `expr_inner` crea para `[]` variables de raíz y elemento,
  registra `array_child`, `array_origin`, `array_constructed`, `expr_vars`,
  `deferred_exprs` y `PendingConstraint::Constructor`. `statement` enlaza el
  binding por `unify`; `bind`/`union` conservan relaciones y `concrete` resuelve
  el elemento. `function` consulta `pending_constructors` y emite NX-E0103 solo
  al cierre. `check_with_partial_limit` añade planificación de formas sin emisión
  y después comprobación; no basta repetir ese plan para recuperar el Result perdido.
- `unify` delega en `unify_with_expected_origin`. Su rama `(Result,Result)`
  unifica `.ok` y luego `.err` mediante llamadas mutantes consecutivas, sin
  transacción ni restauración si falla la segunda. Existe representación
  `Type::Result` con componentes `Type::Deferred`, pero construirla por sí solo
  destaparía este segundo defecto: A/T01 podría fijar T=Bool desde una igualdad
  cuyo E=Bool ya contradice el I64 del payload.
- `check_intrinsic`, rama AssertEq, usa `result_constructor` y
  `expected_from_first`: reconoce sintácticamente Ok/Err en el segundo argumento
  y le entrega el Result concreto del primero. Explica la aceptación de
  `AssertEq(r,Ok(1))` y de su variante con TryAdd, cuyo resultado es concreto;
  el primer constructor sigue fallando antes de llegar a la igualdad común.

La reparación pertenece al grafo de restricciones y sus obligaciones de cierre,
no a nuevas inspecciones de patrones de fuente. El análisis no ejecutó los repros;
el estado observado está registrado en NB-I11b-11 y en
`tests/i08.rs::deferred_result_alias_conflict_pins_current_t01_diagnostic`.

## 3. Decisión única (requisitos NÚCLEO)

**R1 — Construcción parcial.** Ok(p) establece `Result<P,α>` y Err(p)
`Result<α,P>`, con P sintetizado del payload y α fresca monomórfica. Se conserva
la identidad de cada incógnita a través de bindings, aliases, componentes y usos;
no se instancia otra por lectura. Un contexto Result ya disponible restringe
sus componentes conforme a A2 §§4.1–4.1.2. Never no aporta un tipo concreto y
la contaminación conserva las prioridades causales existentes.

**R2 — Contexto posterior.** Anotación de un alias, argumento con parámetro
anotado, retorno, asignación, elemento de Array o igualdad de operandos añade
una igualdad estructural al mismo grafo. Relacionar dos incógnitas es útil aunque
no determine ninguna. Se fija un componente en la primera igualdad compatible
que, junto al prefijo aceptado, determina su tipo; fijar otro componente después
no cambia el primero. Los dominios, incluido `domain:copyable_data`, no escogen
un tipo. Sin restricción suficiente la incógnita permanece hasta el cierre.

**R3 — Orden y atomicidad.** Rige A2 §4.1.2, puntos 1–4: contexto heredado,
hijos en orden escrito y relación del resultado sintetizado con su consumidor.
Una igualdad entre tipos compuestos se confirma completa o no cambia ninguna
de sus unificaciones, enlaces ni procedencias. Una implementación puede usar
una transacción o preparación sin cambios y confirmación; el resultado debe
ser independiente del representante y del orden interno del solucionador.
Las restricciones independientes del payload u otros hijos se conservan.
La igualdad incompatible queda rechazada en su sitio y se continúa con los
usos independientes; nunca contamina por sí sola el binding válido utilizado.
Completar datos del conflicto posteriormente no incorpora esa igualdad al grafo.

**R4 — Hermanos y evaluación.** AssertEq sintetiza sus argumentos en orden
escrito, conserva sus tipos parciales y exige su igualdad antes del dominio
(A2 §4.1.2, punto 4). Así el segundo puede completar el componente del primero;
no necesita volver a comprobar ni evaluar ese constructor. La orientación del
diagnóstico sigue siendo esperado=primero, encontrado=segundo y primario sobre
el segundo argumento completo. La igualdad posterior no se redistribuye como
contexto heredado en hojas ya visitadas. Se aceptan Ok/Err y TryAdd en las dos
posiciones de AssertEq por el mecanismo común (hoy solo se acepta con el
constructor en segundo lugar), retirando la excepción sintáctica de AssertEq. No cambia el orden de evaluación, efectos, movimientos
ni limpieza: inferir no ejecuta, duplica ni reordena operandos.

Nota sobre N10 (decisión de este ADR): A2 §13.17 suprime solo el NX-E0113 exterior que depende de un Array
irresuelto. Este ADR lo lee así: `Ok([])` sin anotación emite únicamente el diagnóstico del Array (una causa, un
diagnóstico) y no un NX-E0113 adicional por `$.err`. Si una revisión posterior concluye que `$.err` debe
denunciarse por separado, se resuelve por enmienda de este ADR.

**R5 — Cierre y diagnóstico.** Tras todas las restricciones locales compatibles,
un constructor Result que conserve componentes indeterminados emite
NX-E0113, `subcause=null`, `stage=types`, `operation=types.infer`, sobre su
expresión completa sin grupos. `unresolved_components` usa las posiciones y
el preorden de A2 §13.17, sin variables internas ni tipos inventados.
Se mantienen la obligación por constructor, la clasificación causal y supresión
de derivados de A2 §4.1.2, puntos 2 y 5: no un error por alias o uso; un Array
irresuelto conserva NX-E0103 y no provoca además NX-E0113 exterior dependiente.
Un conflicto de tipos serializable usa NX-E0102/type_mismatch, types/types.check,
con el operando y contexto de la restricción original. Si falta evidencia
serializable, rige la omisión condicionada al diagnóstico del constructor de
A2 §4.1.2, punto 2; no se rechaza anticipadamente el constructor ni se inventa
un componente para poder denunciar el conflicto.

**R6 — Un mecanismo, mismo alcance.** Array conserva su regla y NX-E0103;
no se cambia su semántica resuelta. J02 aplica el mecanismo a None/Some según
J §3.1, incluida la ruta `$.some` ya definida allí. F5-L07 lo reutiliza para
las igualdades de T de sus firmas selladas (F5 §4.1). Estas capacidades mantienen
sus puertas y dependencias: este ADR no las activa ni amplía el subconjunto.

## 4. Ejemplos normativos y pares

Expectativas de diseño **NO EVALUADAS**. Son instancias de A2 §§4.1–4.1.2,
13.17 y 14.1, F2-S09/F2-S10, en superficie F0-D15; A/T01 conserva su fuente
registrada. `⟦texto⟧` delimita exactamente el primario: se eliminan solo los
marcadores y se cuentan bytes UTF-8 desde cero, intervalo semiabierto, como
en A2 §14. No son sintaxis Nexo. Cada fila negativa exige un único diagnóstico.

| ID / regla | Fuente válida completa; `check` acepta y `run` termina en unidad |
|---|---|
| V1 / R1–R2 | `fn Main(){let r=Ok(1);let s:Result<I64,Bool>=r;}` |
| V2 / R1–R2 | `fn Main(){let r=Err(7);let s:Result<Bool,I64>=r;}` |
| V3 / R2 | `fn F(r:view Result<I64,Bool>){} fn Main(){let r=Ok(1);F(r);}` |
| V4 / R2 | `fn F()->Result<I64,I64>{let x=Ok(1);return x;} fn Main(){let y=F();}` |
| V5 / R2–R3 | `fn Main(){var r=Ok(1);r=Err(true);let s:Result<I64,Bool>=r;}` |
| V6 / R4 | `fn Main(){let r:Result<I64,I64>=Ok(1);AssertEq(Ok(1),r);}` |
| V7 / R4 | `fn Main(){let r:Result<I64,I64>=Ok(1);AssertEq(r,Ok(1));}` |
| V8 / R4 | `fn Main(){let r=TryAdd(1,2);AssertEq(Ok(3),r);AssertEq(r,Ok(3));}` |
| V9 / R2 | `fn Main(){let r=Ok(1);let s=r;let t:Result<I64,Bool>=s;}` |
| V10 / R1–R2 | `fn Main(){let r=Ok(Ok(1));let s:Result<Result<I64,Bool>,I64>=r;}` |
| V11 / R6 | `fn Main(){let a=[];let b:Array<I64>=a;AssertEq(a,[]);}` |
| V12 / R2 | `fn Main(){let r=Ok(1);let a:Array<Result<I64,Bool>>=[r];}` |

En la tabla siguiente U = NX-E0113/null (types/types.infer),
A = NX-E0103/null (types/types.infer), M = NX-E0102/type_mismatch
(types/types.check). Las columnas de tipos muestran esperado / encontrado
completos conforme a A2 §13.17; los primarios están marcados en la fuente.

| ID / regla | Fuente inválida completa | Diagnóstico y datos | Positivo pareado |
|---|---|---|---|
| N1 / R1,R5 | `fn Main(){let r=⟦Ok(1)⟧;}` | U; `unresolved_components=["$.err"]` | Anotar `r:Result<I64,I64>` (F2-S09). |
| N2 / R1,R5 | `fn Main(){let r=⟦Err(7)⟧;}` | U; `unresolved_components=["$.ok"]` | Anotar `r:Result<Bool,I64>` (§13.17). |
| N3 / R3, A/T01 | `fn Main(){let r=Err(1);let s:Result<Bool,Bool>=⟦r⟧;let t:Result<I64,I64>=r;}` | M; `Result<Bool,Bool>` / `Result<I64,I64>` | Quitar solo la declaración de `s`; `t` debe seguir siendo compatible en ambos análisis. |
| N4 / R2–R3 | `fn Main(){let r=Ok(1);let s:Result<I64,Bool>=r;let t:Result<I64,I64>=⟦r⟧;}` | M; `Result<I64,I64>` / `Result<I64,Bool>` | Cambiar solo el E de `t` a Bool. |
| N5 / R2 | `fn F(r:view Result<Bool,I64>){} fn Main(){let r=Ok(1);let s:Result<I64,I64>=r;F(⟦r⟧);}` | M; `Result<Bool,I64>` / `Result<I64,I64>` | Cambiar solo el T del parámetro a I64. |
| N6 / R2 | `fn F()->Result<Bool,I64>{let x=Ok(1);let s:Result<I64,I64>=x;return ⟦x⟧;} fn Main(){}` | M; `Result<Bool,I64>` / `Result<I64,I64>` | Cambiar solo el T del retorno a I64. |
| N7 / R4 | `fn Main(){let r:Result<I64,I64>=Ok(1);let s:Result<I64,Bool>=Ok(1);AssertEq(r,⟦s⟧);}` | M; `Result<I64,I64>` / `Result<I64,Bool>` | Cambiar solo el E de `s` a I64; la igualdad también se cumple al ejecutar. |
| N8 / R2,R5 | `fn Main(){let r=⟦Ok(1)⟧;Print(r);}` | U; `unresolved_components=["$.err"]`; el dominio de Print no elige E. | Anotar `r:Result<I64,Bool>`. |
| N9 / R6 | `fn Main(){let a=⟦[]⟧;}` | A; `data={}` | Anotar `a:Array<I64>` (F2-S10). |
| N10 / R5–R6 | `fn Main(){let r=Ok(⟦[]⟧);}` | A; `data={}`; sin NX-E0113 exterior dependiente. | Anotar `r:Result<Array<I64>,Bool>`. |

En A/T01, `Err(1)` fija E=I64. La igualdad de `s` intenta T=Bool y E=Bool:
se rechaza entera, sin fijar T. La de `t` fija T=I64; completa la evidencia
del conflicto anterior, cuyo primario sigue siendo el `r` de `s`. El programa
se rechaza, pero no hay error en `t` ni NX-E0113 en `Err(1)`. No se adopta el
NX-E0113 actual del test como esperado normativo.

## 5. Alternativas descartadas

| Alternativa razonable | Motivo de descarte |
|---|---|
| Exigir anotación en cada constructor | Simple de implementar, pero contradice el contexto local transitivo y exige escritura redundante; requeriría enmienda de A2 §§4.1–4.1.1 y 13.17. |
| Fijar o rechazar en el primer uso | Confunde el primer uso con la primera igualdad suficiente; un alias o dominio puede no determinar E/T. Rechazaría V9 y el positivo de N8 sin apoyo normativo. |
| Propagar solo desde contextos previos/inmediatos | Conserva el atajo actual, pero rechaza V1–V4/V6 y crea dos políticas para Result y Array. |
| Resolver globalmente y redistribuir el tipo final hacia atrás | Puede aceptar positivos, pero cambia primarios y prioridades de hojas; infringe disponibilidad por nodo y el prefijo monótono de A2 §4.1.2. |
| Conservar componentes compatibles de una igualdad fallida | Reduce trabajo de rollback, pero fija T=Bool en A/T01 y rechaza indebidamente `t`; infringe atomicidad. |

## 6. Implementación absorbida y aceptación

| Requisito / clase | Dueño existente y evidencia exigida |
|---|---|
| R1–R5, NÚCLEO | Corrección de I07c con integración I08, antes de J01: grafo compartido, obligaciones de cierre y unificación compuesta atómica; sustituir la expectativa defectuosa A/T01. A2 §§17.1–17.2 y J §12.1. El grueso del trabajo es dotar a la representación de tipos diferidos, a `bind` y a `union` de hijos de Result análogos a los de Array. Las cargas de A2 §§11.1 y 11.8 sobre los pasos del solucionador se mantienen. |
| R6, NÚCLEO | I07c conserva Array; J02 reutiliza para Option, J03 para sus uniones; F5-L07 para Array y firmas selladas. J §§3.1,12.2; F5 §§4.1,10. Sin adelantar sus capacidades. |
| Conservación, NÚCLEO | TD01–TD07, DG07/DG08 y columnas N del corpus RD; sitios causales, procedencias exigidas, Never/contaminación, propiedades del tipo completo y orden de ejecución. J16 y F5-L11/F5-L12 repiten las regresiones de sus perfiles. |
| Transporte, formato, scripts, cuotas y contadores, ANDAMIAJE | Integrar en arneses existentes de I07c/I08 y después J/F5; ejemplos de flujos normativos y fuzz sin pánicos. No se fijan cifras nuevas ni se exige campaña de mutación u oráculo independiente para esta clase. |

Para cada regla de núcleo: negativo aislado y positivo pareado, esperados
transcritos de este ADR y de las secciones citadas, código/subcausa/primario y
campos exactos aplicables; falsos positivos, aliases, anidación, ambas variantes
y posiciones, contexto inmediato/posterior y continuación tras conflicto.
La mutación inversa por regla conserva restauración verificada por SHA-256;
una sola campaña final sobre núcleo por el implementador. Fuzz sin pánicos,
internal_error, InvalidIr ni cuelgues; diferencial debug/release y `check`/`run`
(aceptación y diagnósticos iguales para estas fuentes con Main válido; los
negativos no ejecutan). El validador distinto contrasta también su batería
contra HEAD archivado; máximo dos rondas completas y luego verificaciones
dirigidas, conforme a `AGENTS.md` y `docs/F0_ACUERDOS.md` §10.

El corte informará programas objetivo que ejecutan sobre el total y justificará
si no habilita ninguno nuevo; actualizará Pending/HANDOFF/RECOVERY al cerrarse.
Esta propuesta no declara implementación, PASS, cierre de I11b ni aprobación de fase.

## 7. Naturaleza de la decisión

**Concreta el hueco; no modifica una regla normativa revisada.** Aceptar el
contexto posterior y restaurar atomicidad ya se deduce de A2 §§4.1.1–4.1.2;
R1–R6 eligen su realización coherente frente a alternativas que sí requerirían
enmienda. No cambia TD/RD, clasificación C/D/V, catálogo, spans ni prioridades.
Tras validación, bastaría registrar esta precisión y su referencia en A2 §20
y enlazarla desde §§4.1.1–4.1.2 y 13.17 mediante el proceso documental autorizado;
no se requiere enmienda semántica. J §3.1 y F5 §4.1 ya remiten al mismo motor.
Aquí no se modifica ningún ADR revisado ni la enmienda previa de IR pendiente.
