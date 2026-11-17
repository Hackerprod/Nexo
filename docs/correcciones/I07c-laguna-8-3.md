# I07c §8.3 — propuesta para la asignación que lee su destino (Flujo A)

Estado: **PROPUESTA; REVERTIBLE**. No es un PASS ni modifica todavía el ADR. Base `e6e5f82`, rama `i07c-estructural`. La decisión corresponde al coordinador al llenar una laguna que el ADR no resolvía; una revisión independiente debe validarla antes de incorporarla. **Solo cambia programas sin salida fijada por el ADR; ninguno con salida fijada cambia:** el válido sin asignación posterior pasa de NX-E0103 `[16,18)` a aceptado, y `b1a=ir1` y el negativo con `r=[1]` pasan de `driver_error/internal_error` a sus veredictos propuestos. Véase «Interinidad» para el oráculo anterior al PASS.

## Decisión y razón

**Aceptar la solución monomórfica finita.** En `var r=[];r=[(r==r)==true];`, el primer constructor impone `r: Array<α>`. Las dos lecturas de `r` en `r==r` exigen el mismo tipo, sin imponer `α=Array<α>`: ninguna de las dos lecturas es el elemento del array. `==` devuelve `Bool` nominal; su dominio `copyable_data` queda pendiente hasta resolver `α`. La igualdad exterior compara `Bool` con `Bool`. El único elemento del RHS es por ello `Bool`, y la asignación compatible unifica `α=Bool`. El resultado es `r: Array<Bool>`, finito y copiable. La restricción de dominio pendiente queda satisfecha al cierre. La asignación `r=[true]` posterior es compatible y **no es necesaria** para obtener el tipo.

Esto aplica §4.1.1 (monomorfismo y restricciones hipotéticas), §4.1.2.1 (igualdad compuesta atómica), §4.1.2.2 (acumular igualdades compatibles y resolver el elemento del constructor), §4.1.2.3 (el dominio indeciso no es error y el resultado nominal `Bool` de `==` alcanza grupos y arrays) y §4.1.2.4 (igualdad antes del dominio de `==`). TD04 acepta el resultado `Bool` de igualdad hipotética; TD05 deja que un elemento `Bool` determine el array. §4.1.2.7 conserva el gemelo que adelanta `Array<Bool>` al inicializador porque el adelanto es compatible y no cambia un contexto externo distribuido. La visión de un lenguaje fácil de usar con control progresivo de la máquina favorece aceptar una solución finita comprobable sin exigir anotación redundante.

**Alternativa descartada:** exigir que el elemento del array esté determinado antes de admitir `r==r`, con NX-E0103 sobre `[]` o NX-E0113 sobre la lectura. Daría un rechazo sensible al momento interno de resolución; NX-E0103 sería falso al cierre porque el RHS fija `Bool`, incluso sin asignación posterior. NX-E0113 corresponde a un constructor de valor indeterminado, no a una lectura de variable hipotética (§4.1.2.3 y catálogo). Esta alternativa rompería la disponibilidad nominal explícita de §4.1.2.3, el tratamiento de TD04/TD05 y el gemelo de §4.1.2.7. No se propone un tipo recursivo ni se relaja la prueba de finitud: `r=[r]` exige `α=Array<α>` y sigue bajo la regla existente de conflicto recursivo.

## Texto exacto propuesto para el ADR

**Ubicación de la enmienda completa:** `docs/adr/F0-0002-second-slice.md`, §4.1.2.3, inmediatamente después del párrafo de «Cierre de dominio indeciso» y sus ejemplos, antes de la cláusula 4. Se insertan allí, en este orden y como texto normativo, el párrafo entrecomillado siguiente y la tabla de «Ejemplos y diagnóstico exacto» de esta propuesta, con sus fuentes, offsets y campos. La tabla forma parte de la enmienda, no es un mero oráculo externo: para estos C fuera de RD, §4.1.2 N(g) hace normativas las ubicaciones de §13.15.

> **Resultado nominal en una asignación que lee su destino (precisión normativa).** Una lectura de un binding dentro del RHS de su propia asignación conserva el mismo tipo monomórfico del binding; no crea por sí sola una igualdad entre ese tipo y el tipo del resultado del RHS. Si una operación válida del RHS tiene resultado nominal independiente de un dominio aún indeciso, ese resultado **fija** un componente pendiente del tipo del destino a través de un constructor de valor y de la igualdad de asignación. Se resuelven después los dominios pendientes con el tipo completo. La aceptación exige una solución finita de todas las igualdades compatibles y la satisfacción de los dominios decididos; el carácter hipotético de la evaluación no permite posponer ni perder esas restricciones. En `fn Main(){var r=[];r=[(r==r)==true];}`, `r==r` devuelve `Bool` y el RHS fija `r: Array<Bool>`; `check` acepta sin diagnósticos y `run` devuelve unidad sin observaciones. Una restricción posterior incompatible conserva su sitio: `fn Main(){var r=[];r=[(r==r)==true];r=[1];}` se rechaza con un único `NX-E0102/type_mismatch` sobre el `1` final, esperado `Bool` y encontrado `I64`. Su relación `expected_type` señala el **array completo** `[(r==r)==true]` `[21,35)`, primera expresión de asignación que determina `r: Array<Bool>`; la relación `found_type` señala `1` `[39,40)`. El elemento interior `(r==r)==true` `[22,34)` aporta `Bool`, pero no es la expresión cuyo tipo completo iguala el tipo del destino. Según §§4.1.2.6–7 y 13.15, se sigue la procedencia hasta la expresión que fija el tipo esperado de la asignación completa; §§13.17 y 4.1.2.6 reservan `Bool`/`I64` y el primario para la hoja incompatible, sin sustituir el origen de `Array<Bool>` por su componente. RD29, RD89, RD93 y RD106 relacionan el literal `true` porque allí ese literal fija directamente el elemento; aquí el elemento lo fija `(r==r)==true`, mientras que el array completo fija el tipo del destino. No se emite NX-E0103 por el `[]` ya resuelto ni se inventa un tipo recursivo. Un fallo real de igualdad, dominio o hijo necesario sigue contaminando solo sus resultados dependientes conforme a §4.1.2.5; una ecuación sin solución finita sigue las reglas existentes de §4.1.2.2.

No se añade código ni subcausa al catálogo. La primera sentencia de la precisión no autoriza `r=[r]`: allí el RHS sí intenta igualar el elemento `α` con el array completo `Array<α>`.

## Ejemplos y diagnóstico exacto

Fuentes ASCII, sin BOM ni LF final, en superficie oficial F0-D15.

| Caso | Fuente | Esperado tras el PASS de la enmienda; reglas existentes donde se indica |
|---|---|---|
| Válido | `fn Main(){var r=[];r=[(r==r)==true];}` | `check`: salida 0, `outcome.kind=checked`, `diagnostics=[]`. `run`: salida 0, `outcome.kind=unit`, `diagnostics=[]`, `stdout=[]`, `events=[]`, `observations=[]`. Al ejecutar, se comparan los dos arrays vacíos, la igualdad interior y la exterior dan `true`, y `r` pasa a `[true]`. |
| Inválido, una regla | `fn Main(){var r=[];r=[(r==r)==true];r=[1];}` | `check` y `run`: salida 1, `outcome.kind=rejected`, ningún evento, salida ni observación, un C: `code=NX-E0102`, `subcause=type_mismatch`, `stage=types`, `operation=types.check`, primario `[39,40)` (token `1`), `data={"expected_type":"Bool","found_type":"I64"}`. `related_spans`: `expected_type=[21,35)` (primer RHS `[(r==r)==true]`, origen concreto del tipo del destino), `found_type=[39,40)`; sin otras relaciones. `rule=explanation_id=NX-E0102/type_mismatch`, `cause_chain=[]`, `repair=[]`, `feature_id=null`, `unexpected_token=null`, `expected_tokens=[]`, `expected_tokens_kind=null`, `parser_context=null`, `recognized_prefix=null`. Reemplazar solo `1` por `true` da el positivo. |
| Recursivo sin solución finita | `fn Main(){var r=[];r=[r];}` | `check` y `run`: salida 1, `outcome.kind=rejected`, sin ejecución; único C `NX-E0103`, `subcause=null`, `stage=types`, `operation=types.infer`, primario `[16,18)` (el `[]` inicial), porque `α=Array<α>` no resuelve finitamente su elemento (§§4.1.2.2 y 13.17). |
| Gemelo negativo con tipo adelantado | `fn Main(){var r=[true];r=[1];}` | `check` y `run`: salida 1, `outcome.kind=rejected`, ningún evento, salida ni observación; único C `code=NX-E0102`, `subcause=type_mismatch`, `stage=types`, `operation=types.check`, primario `[26,27)` (token `1`), `data={"expected_type":"Bool","found_type":"I64"}`, `related_spans`: `expected_type=[16,22)` (array inicial `[true]`), `found_type=[26,27)`; sin otras relaciones. `rule=explanation_id=NX-E0102/type_mismatch`, `cause_chain=[]`, `repair=[]`, `feature_id=null`, `unexpected_token=null`, `expected_tokens=[]`, `expected_tokens_kind=null`, `parser_context=null`, `recognized_prefix=null`. `var r=[true]` fija el Array desde el inicializador; §§4.1.2.7, 13.15 y 13.17 dan el C en la hoja posterior y el origen en el array completo. |

Offsets calculados por bytes ASCII desde cero, sin usar el binario como oráculo: `fn Main(){` ocupa `[0,10)`; `var r=[];` ocupa `[10,19)` y su `[]` `[16,18)`; el primer RHS ocupa `[21,35)`, su elemento `[22,34)` y el `1` final `[39,40)`. En el gemelo, `var r=[true];` ocupa `[10,23)`, el inicializador `[16,22)` y el `1` posterior `[26,27)`. El primario del negativo es la **hoja** incompatible de la segunda asignación (§§4.1.2.2, 4.1.2.4 y 13.15), no el operador `==`, el array completo ni el primer `[]`. La primera expresión RHS completa establece el tipo de la raíz; §13.15 da `expected_type` a esa expresión fuente, de la misma forma que al inicializador completo `[true]` del gemelo (§4.1.2.7). El negativo gemelo contrasta una sola hoja con el válido `fn Main(){var r=[true];r=[true];}`.

## Variantes y oráculo para I07c

| ID | Fuente exacta | `check` / `run` propuestos |
|---|---|---|
| `b1a = ir1` | `fn Main(){var r=[];r=[(r==r)==true];r=[true];}` | Aceptado / unidad, `diagnostics=[]`, sin eventos ni observaciones. La última asignación es compatible. |
| `ir1c` | `fn Main(){var r=[];r=[r==r];r=[true];}` | Aceptado / unidad, `diagnostics=[]`, sin eventos ni observaciones. El resultado `Bool` de la igualdad fija el elemento. |
| `ir1d` | `fn Main(){var r=[];var w=[];r=[w==w];w=[true];}` | Aceptado / unidad, `diagnostics=[]`, sin eventos ni observaciones. `r` se fija a `Array<Bool>` por su RHS y `w` por su asignación posterior. |
| Sin asignación posterior | `fn Main(){var r=[];r=[(r==r)==true];}` | Aceptado / unidad, `diagnostics=[]`, sin eventos ni observaciones. Prueba que la posterior no aporta la solución. |
| `ir1_t`, tipo adelantado | `fn Main(){var r=[true];r=[(r==r)==true];}` | Aceptado / unidad, `diagnostics=[]`, sin eventos ni observaciones. |
| Negativo aislado | `fn Main(){var r=[];r=[(r==r)==true];r=[1];}` | Rechazo descrito arriba: solo C sobre `1` `[39,40)`; `run` no ejecuta primitivas. |

**Sondas en clon desechable** `D:/nexo-work/i07c-laguna-probe` de `e6e5f82`, `CARGO_TARGET_DIR=D:/nexo-work/target-i07c-l`, `TEMP/TMP=D:/nexo-work/tmp`: `check` y `run` actuales dan `driver_error/internal_error` para `b1a=ir1` y el negativo con `r=[1]`; `ir1c`, `ir1d` y `ir1_t` ya pasan. Sin asignación posterior, el comprobador actual da NX-E0103 sobre `[]` `[16,18)`, discrepancia con esta propuesta. `fn Main(){var r=[];r=[true];r=[1];}` ya produce NX-E0102/type_mismatch en el último `1`, con origen `expected_type` en el primer RHS `[true]`. Las sondas describen la implementación, **no** fijan el oráculo.

## Compatibilidad con las filas publicadas

La precisión añade una relación de restricciones que faltaba especificar solo cuando el RHS lee el mismo binding cuyo elemento Array completa con el resultado nominal de una operación. No reinterpreta `Never`, una igualdad recursiva de tipos, una unión fallida ni un dominio refutado. Método de comprobación de «Igual»: inspección textual de todas las fuentes y variantes TD/RD/DG de §§14.10–14.11 para hallar un destino leído en su propio RHS y una asignación que fije mediante resultado nominal un elemento Array aún pendiente; comparación de los hallazgos con las restricciones y esperados escritos de cada fila, incluidas variantes, positivos, C, primarios, orden, salida y columnas I. En RD solo RD70 contiene asignaciones a bindings leídos en su propio RHS (`v1=v1;v2=[v2]`), ambos con tipos ya concretos; no hay ese patrón en TD ni DG. Las demás familias citadas abajo no aportan el RHS nominal que determina el elemento pendiente. Es una comprobación documental estática: el binario base no implementa la regla nueva y no acredita estas filas. Según esa inspección, ninguna fila TD, RD ni DG cambia; «Igual» significa **todos los resultados y variantes publicados para esa fila**, no solo el código principal. RD79 está retirado y no es una fila.

| Fila TD | Estado | Fila DG | Estado |
|---|---|---|---|
| TD01 | Igual | DG01 | Igual |
| TD02 | Igual | DG02 | Igual |
| TD03 | Igual | DG03 | Igual |
| TD04 | Igual | DG04 | Igual |
| TD05 | Igual | DG05 | Igual |
| TD06 | Igual | DG06 | Igual |
| TD07 | Igual | DG07 | Igual |
| — | — | DG08 | Igual |

TD04 conserva ambos órdenes de igualdad hipotética; TD05 conserva ambos órdenes de array con `DV`. TD01–TD03, TD06–TD07 y DG01–DG08 conservan también sus positivos, gemelos y campos exactos.

| Fila RD | Estado | Fila RD | Estado | Fila RD | Estado | Fila RD | Estado |
|---|---|---|---|---|---|---|---|
| RD01 | Igual | RD02 | Igual | RD03 | Igual | RD04 | Igual |
| RD05 | Igual | RD06 | Igual | RD07 | Igual | RD08 | Igual |
| RD09 | Igual | RD10 | Igual | RD11 | Igual | RD12 | Igual |
| RD13 | Igual | RD14 | Igual | RD15 | Igual | RD16 | Igual |
| RD17 | Igual | RD18 | Igual | RD19 | Igual | RD20 | Igual |
| RD21 | Igual | RD22 | Igual | RD23 | Igual | RD24 | Igual |
| RD25 | Igual | RD26 | Igual | RD27 | Igual | RD28 | Igual |
| RD29 | Igual | RD30 | Igual | RD31 | Igual | RD32 | Igual |
| RD33 | Igual | RD34 | Igual | RD35 | Igual | RD36 | Igual |
| RD37 | Igual | RD38 | Igual | RD39 | Igual | RD40 | Igual |
| RD41 | Igual | RD42 | Igual | RD43 | Igual | RD44 | Igual |
| RD45 | Igual | RD46 | Igual | RD47 | Igual | RD48 | Igual |
| RD49 | Igual | RD50 | Igual | RD51 | Igual | RD52 | Igual |
| RD53 | Igual | RD54 | Igual | RD55 | Igual | RD56 | Igual |
| RD57 | Igual | RD58 | Igual | RD59 | Igual | RD60 | Igual |
| RD61 | Igual | RD62 | Igual | RD63 | Igual | RD64 | Igual |
| RD65 | Igual | RD66 | Igual | RD67 | Igual | RD68 | Igual |
| RD69 | Igual | RD70 | Igual | RD71 | Igual | RD72 | Igual |
| RD73 | Igual | RD74 | Igual | RD75 | Igual | RD76 | Igual |
| RD77 | Igual | RD78 | Igual | RD80 | Igual | RD81 | Igual |
| RD82 | Igual | RD83 | Igual | RD84 | Igual | RD85 | Igual |
| RD86 | Igual | RD87 | Igual | RD88 | Igual | RD89 | Igual |
| RD90 | Igual | RD91 | Igual | RD92 | Igual | RD93 | Igual |
| RD94 | Igual | RD95 | Igual | RD96 | Igual | RD97 | Igual |
| RD98 | Igual | RD99 | Igual | RD100 | Igual | RD101 | Igual |
| RD102 | Igual | RD103 | Igual | RD104 | Igual | RD105 | Igual |
| RD106 | Igual | RD107 | Igual | RD108 | Igual | RD109 | Igual |
| RD110 | Igual | RD111 | Igual | RD112 | Igual | RD113 | Igual |
| RD114 | Igual | RD115 | Igual | RD116 | Igual | RD117 | Igual |
| RD118 | Igual | RD119 | Igual | RD120 | Igual | RD121 | Igual |
| RD122 | Igual | RD123 | Igual | RD124 | Igual | RD125 | Igual |
| RD126 | Igual | RD127 | Igual | RD128 | Igual | — | — |

En particular, RD23/RD29 conservan el elemento fijado por un contexto o referencia anterior; RD87(a–d) sigue aceptado; RD101/RD119 siguen rechazando constructores cuyos elementos no se resuelven; RD102/RD103 conservan la fijación por igualdad con `true`; RD109 conserva el dominio refutado; RD116 conserva la igualdad compatible y el dominio fallido; RD122 conserva dos NX-E0103 porque **ninguna** igualdad aporta un elemento concreto; RD127(a–b) sigue rechazando `[q]`/`[DV]` irresueltos. El nuevo caso tiene un RHS `Bool` válido que RD101/RD119/RD122/RD127 no tienen. En ninguna fila se cambia el dominio de `==` ni se permite una ecuación `α=Array<α>`.

## Interinidad

Hasta que el ADR enmendado tenga PASS, `b1a=ir1`, el válido sin asignación posterior, el negativo con `r=[1]` y las variantes de la nueva regla quedan **EXCLUIDOS del oráculo normativo de aceptación, rechazo y diagnóstico**; para esos programas solo se exige §4.1.2 N(f): ningún pánico, `InvalidIr` ni `internal_error` en `check` o `run`. Sus filas en «Ejemplos y diagnóstico exacto» y «Variantes y oráculo para I07c» expresan la decisión arquitectónica propuesta y adquieren fuerza normativa tras ese PASS. Con el catálogo y el ADR vigentes no existe un diagnóstico emitible para rechazar esta forma: NX-E0103 exige elemento Array irresuelto al cierre, NX-E0113 no sustituye el código propio del Array y NX-E0006 exige una capacidad excluida registrada. Si la implementación solo cumple N(f) aceptando el válido, `b1a=ir1`, `ir1c`, `ir1d`, el caso sin asignación posterior o `ir1_t` durante la interinidad, esa aceptación coincide con la decisión propuesta; no se impone otra salida transitoria para ellos. Para el negativo con `r=[1]`, el gemelo `var r=[true];r=[1];` y el recursivo `r=[r]`, N(f) es solo la exigencia mecánica: el único veredicto válido para cada uno es el rechazo de su propia fila (NX-E0102, NX-E0102 y NX-E0103, respectivamente), nunca su aceptación. Los `internal_error` actuales de `b1a=ir1` y del negativo incumplen N(f). El recursivo y el gemelo conservan por separado sus reglas ya vigentes.
