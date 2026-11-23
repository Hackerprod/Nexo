# I07c-S6COR-GUARDA-CLOSURE — efecto de una llamada a closure en la guarda de `match`

**Estado: plan de A (contexto limpio), versión 2 (corrige las 4 omisiones documentales del FAIL de B r1), pendiente de B; NO autoriza implementación.** Corte de **núcleo** (aceptación de guardas y diagnóstico `NX-E1305/guard_effect`); andamiaje: corpus y script diferencial, fuzz, texto de documentos. Base: `origin/main` = `e7b1813` (`src/` idéntico a `68e0d2b`, la medida del defecto). Binario base medido: `git archive origin/main` compilado en debug (rustc 1.99.1) en `C:\nexo-work\val\a-guarda-closure-base`. Candidato de comprobación (no confirmado, se borra): la misma copia con los dos cambios de §5 en `src/ir/effects.rs` (el diff es el de §5; sin tocar tests ni ADR).

## 0. Resumen y cuenta

- **Defecto D1 (el reportado).** En una guarda de `match`, una llamada a una **closure** (variable, literal, enlace de patrón) se acepta con cualquier efecto; la llamada a una `fn` con nombre equivalente da `NX-E1305/guard_effect`. Causa raíz: el bucle de guardas de `ir/effects.rs::analyze` solo reconoce `Op::CallDirect` (l.1810); `Op::CallClosure` se descarta (§3).
- **Defecto D2 (hermano, hallado al medir la familia, distinta causa).** Las guardas que viven **dentro del cuerpo de una closure** no se comprueban nunca: ni siquiera `MockUnknown()` o `G()` directos se rechazan (`fn Main(){let f=fn (){match true{true if MockUnknown()==()=>{},_=>{}}};}` → `checked`). Causa: `CallSites::from_ast` descarta las guardas de las closures (`&mut Vec::new()`, l.133) y el bucle de guardas salta toda función sin declaración fuente (l.1789). D2 no es una familia de D1 (otro mecanismo) pero es la misma regla de aceptación y comparte la regresión de «closure llamada dentro de guarda dentro de closure».
- **Soluciones: 2 commits de producto, ambos en `src/ir/effects.rs`:** **S1** (D1) y **S2** (D2); mecanismo de cada una en §5. Se añade **C0** (andamiaje: corpus y script diferencial, sin tocar producto). La cadena de §8.
- **Efecto observable de S1 sobre tests existentes (medido):** con S1+S2 aplicadas la suite completa en debug (`cargo nextest run --profile default`, 2.205 tests) da 2.203 pasan y **2 fallan**, ambos por consecuencias previstas: (a) `i07c_group_primary::effect_diagnostics_report_the_target_name_of_a_grouped_call`, porque las 12 filas `head_*_guard` encodan el defecto (su fuente da ahora **dos** diagnósticos y su reparación se rechaza), y (b) `i07c_structural_invariants::grouped_operands_keep_one_causal_primary`, porque el auditor de spans fija `effects.rs` por número de línea y S2 desplaza una línea (§6). Ninguna otra salida protegida cambia.
- **Lagunas decididas por el coordinador (sin condiciones pendientes):** L1 = lectura A (`Entity` `function:<offset de fn>`); L2 = lectura 1 (S2 entra como commit propio, revertible, con puerta y mutación por separado); L3 = se adopta el texto de reemplazo de §10, que el coordinador incorpora al addendum antes de implementar. §10.

### 0.1 Clasificación núcleo / andamiaje (AGENTS.md «Rigor proporcional»)

| Requisito | Clase |
|---|---|
| Aceptar/rechazar una guarda según el efecto de la closure llamada (S1, S2): código `NX-E1305`, subcausa `guard_effect`, etapa `effects`, primario, `data` (`effect_evidence`, `required_effect`, `effect_model`) | **NÚCLEO** |
| Número y orden de diagnósticos (p. ej. `NX-E0415` + `NX-E1305` en el mismo sitio; dos llamadas en una guarda → dos diagnósticos); precedencia con `guard_control`/`guard_transfer`/`guard_mutation`; programas sin efecto siguen aceptados | **NÚCLEO** |
| Divergencia `check`/`run`, debug/release; pánico/`internal_error`/`InvalidIr`/cuelgue | **NÚCLEO** (bloquea) |
| `cause_chain` (operaciones y spans de los pasos), `Entity` de cada paso, `related_spans = []` | **ANDAMIAJE** (pruebas de ejemplo; sin campañas; `Entity` según L1 = A, incondicional) |
| Corpus congelado, script diferencial, fuzz, texto de `Pending.md`/defectos abiertos | **ANDAMIAJE** |

## 1. Alcance y autoridad (esperado derivado del ADR)

- **F0-0003 §3.3** («Cobertura, guardas y uniones»), párrafo «Tras casar estructura…»: «La guarda debe ser Bool, sin transferencia, mutación, alloc, block, suspend, io ni efectos desconocidos. Se aplica NX-E1305: primero guard_transfer, después guard_mutation, después guard_effect por sitio causal; un mismo sitio solo emite el primero aplicable. Esta precedencia no borra sitios independientes.» No hay excepción por tipo de invocable: una closure es un invocable cuyo efecto de llamada es el de su cuerpo.
- **F0-0003 §3.3**, párrafo «Una guarda no puede abandonar…»: «Los cuerpos de invocables anidados tienen su propio contexto; **sus efectos de llamada siguen sujetos a guard_effect**.» Es la cláusula que rige S1: la llamada a un invocable anidado (closure) desde una guarda está sujeta a `guard_effect`.
- **F0-0003 §3.3**, «Matriz de barreras»: «effects (guard_effect)»; «Q01 alcanza effects solo si ownership no falla.»
- **F0-0003 §8.1 (tabla de datos)**: `NX-E1305/guard_effect | effects / effects.check | effect_model, effect_evidence, required_effect; sin contract_origin`. **§8.1 (filas Q01)**: «alloc es el primero de los cuatro efectos desconocidos en el orden alloc,block,suspend,io: se emite un solo guard_effect por ese sitio»; cadena del testigo de efectos de A2 sin contrato escrito; «Cause.operation añade closure.create, closure.call, … con kind=call»; evidencia `verified` salvo la hoja de efecto desconocido (`unknown`): en T02 la cadena es `closure.call` (`verified`) > `intrinsic.call` (`unknown`), idéntica en evidencias a la de la `fn` (`function.call` verified > `intrinsic.call` unknown).
- **F0-0002 §8.2**: «Una causa transitiva señala exclusivamente el nombre del destino de la llamada del cuerpo comprobado, sin argumentos ni paréntesis, y enlaza cada llamada hasta la operación originaria.» Rige el primario (`g` en `g()`); selección del testigo: más corto, desempate por offsets (A2 §13.15).
- **F0-0002-lagunas.md «Sitio de la llamada con destino agrupado que no es un nombre»**: destino agrupado no nombre → la **cabeza** (primer token) de la expresión interior desagrupada; «La regla rige también para las llamadas situadas en una guarda de `match`.» Rige el primario de `(fn ()->Bool{…})()` (`fn`) y `(if …)()` (`if`). Un destino que es un nombre (incluidos `(g)()`, `((g))()`) usa el nombre `g`.
- **Efecto transitivo y captura:** el efecto de llamar a una closure es el resumen del cuerpo de su literal, transitivo por las llamadas de ese cuerpo (F0-0003 §8.1 fila «Efectos por operación»: «Literal cuyo cuerpo Print no incumple no_io hasta llamarlo»; resúmenes `create/copy/call/drop`, J09). La forma de captura (`let`, `var`, `copy`, `move`) no cambia el efecto de `call`; el literal `fn [edit x]()` exige llamada `edit`, y una llamada `edit g()` en guarda es `guard_mutation` (ownership, antes que effects).
- Fuera de alcance: `NX-E1305` de ownership (`guard_transfer`, `guard_mutation`, `guard_control`); contratos `with no_*` (no cambian); `for`; perfiles `f0-0002-*` (no admiten closures ni `match`: `NX-E0006` pending_scope `F0-0003`, sin cambio).

## 2. Estado medido y esperado exacto

Binario base (`main` `e7b1813`, debug). Perfiles `f0-0003-b-v1` y `f0-0005-b-v1` dan **resultados idénticos** salvo el valor de `effect_model` (`f0-0003-sim-v1` / `f0-0005-sim-v1`); `f0-0002-b-v2` da `NX-E0006` (`pending_scope=F0-0003`) en `fn`/`match` para toda fila con closure o `match` (sin cambio, sin relación con el defecto). `check` y `run` coinciden (el rechazo es de `effects`, previo a ejecutar). Offsets en bytes `[ini,fin)`; fuentes ASCII, sin BOM ni LF final; longitud entre paréntesis.

Convención de la columna «Esperado»: **un** diagnóstico `NX-E1305/guard_effect`, `stage=effects`, `operation=effects.check`, `related_spans=[]`, `data={effect_evidence, effect_model, required_effect}`, salvo que se indique otra cosa. «Cadena» = `cause_chain` (andamiaje; `#id` = `Entity` según L1 lectura A). `ev` = `effect_evidence`, `req` = `required_effect`.

### 2.1 Negativos (la closure con efecto llamada en guarda)

| # | Fuente (longitud) | Estado base (f0-0003/f0-0005) | Esperado | Cláusula | Clase |
|---|---|---|---|---|---|
| T01 | `fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true if g()=>{},_=>{}}}` (85) | `checked`, exit 0 (`run`: unit) | primario `[70,71)` (`g`); ev=`known_effect` req=`io`; cadena `closure.call[70,71)#function:16 > intrinsic.call[28,33)#prelude:print` | §3.3 «efectos de llamada siguen sujetos»; F0-0002 §8.2 | NÚCLEO |
| T02 | `fn Main(){let g=fn ()->Bool{MockUnknown();return true;};match true{true if g()=>{},_=>{}}}` (90) | `checked` | `[75,76)` (`g`); ev=`unknown_effect` req=`alloc`; cadena `closure.call[75,76)#function:16 > intrinsic.call[28,39)#prelude:mock_unknown` | §3.3 (efectos desconocidos); §8.1 Q01 (alloc primero) | NÚCLEO |
| T03 | `fn G()->Bool{Print(1);return true;}fn Main(){let g=fn ()->Bool{return G();};match true{true if g()=>{},_=>{}}}` (110) | `checked` | `[95,96)` (`g`); ev=`known_effect` req=`io`; cadena `closure.call[95,96)#function:51 > function.call[70,71)#function:3 > intrinsic.call[13,18)#prelude:print` | transitivo F0-0002 §8.2 | NÚCLEO |
| T04 | `fn Main(){let h=fn ()->Bool{Print(1);return true;};let g=fn ()->Bool{return h();};match true{true if g()=>{},_=>{}}}` (116) | `checked` | `[101,102)` (`g`); known/`io`; cadena `closure.call[101,102)#function:57 > closure.call[76,77)#function:16 > intrinsic.call[28,33)#prelude:print` | transitivo (closure→closure) | NÚCLEO |
| T05 | `fn Main(){let x=1;let g=fn ()->Bool{Print(x);return true;};match true{true if g()=>{},_=>{}}}` (93) | `checked` | `[78,79)`; known/`io`; cadena `closure.call[78,79)#function:24 > intrinsic.call[36,41)#prelude:print` (captura `let`, de lectura) | captura no cambia `call` | NÚCLEO |
| T06 | igual con `var x=1;` (93) | `checked` | idéntico a T05 (`[78,79)`, mismas cadenas) | ídem | NÚCLEO |
| T07 | `fn Main(){let a=[1];let g=fn [move a]()->Bool{Print(1);return true;};match true{true if g()=>{},_=>{}}}` (103) | `checked` | `[88,89)`; known/`io`; cadena `closure.call[88,89)#function:26 > intrinsic.call[46,51)#prelude:print` (captura `move`; la llamada `g()` es de lectura) | ídem | NÚCLEO |
| T08 | `fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true if (g)()=>{},_=>{}}}` (87) | `checked` | `[71,72)` (`g`, sin paréntesis); known/`io`; cadena `closure.call[71,72)#function:16 > intrinsic.call[28,33)` | lagunas «destino agrupado» (nombre) | NÚCLEO |
| T09 | igual con `((g))()` (89) | `checked` | `[72,73)` (`g`) | ídem | NÚCLEO |
| T10 | `fn Main(){match true{true if (fn ()->Bool{Print(1);return true;})()=>{},_=>{}}}` (79) | `checked` | `[30,32)` (`fn`, cabeza del literal agrupado); known/`io`; cadena `closure.call[30,32)#function:30 > intrinsic.call[42,47)` | lagunas «destino agrupado» (literal; «también en guarda») | NÚCLEO |
| T11 | `fn Main(){let g=fn ()->Bool{Print(1);return true;};match Some(g){Some(h) if h()=>{},_=>{}}}` (91) | `checked` | `[76,77)` (`h`, enlace de patrón); known/`io`; cadena `closure.call[76,77)#function:16 > intrinsic.call[28,33)` | §3.3: Option transporta closures; llamada por nombre | NÚCLEO |
| T12 | `fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true if (if g() then true else false)=>{},_=>{}}}` (111) | `checked` | `[74,75)` (`g`) | llamada anidada en la guarda | NÚCLEO |
| T13 | `…match true{true if g()==true=>{},_=>{}}` con T01 (91) | `checked` | `[70,71)` (`g`) | operando de guarda | NÚCLEO |
| T14 | `fn Main(){let g=fn ()->Bool{Print(1);return true;};let k=fn ()->Bool{MockUnknown();return true;};match true{true if g()==k()=>{},_=>{}}}` (136) | `checked` | **dos** diagnósticos, orden por primario: `[116,117)` (`g`) known/`io` cadena `closure.call#function:16 > intrinsic.call[28,33)`; `[121,122)` (`k`) `unknown_effect`/`alloc`, cadena `closure.call#function:57 > intrinsic.call[69,80)#prelude:mock_unknown` | §3.3 «Esta precedencia no borra sitios independientes» | NÚCLEO |
| T15 | `fn G()->Bool{Print(1);return true;}fn Main(){let k=fn ()->Bool{MockUnknown();return true;};match true{true if G()==k()=>{},_=>{}}}` (130) | solo el de `G` (`[110,111)`); `k` pasa | **dos**: `[110,111)` (`G`, `function.call`, known/`io`, **sin cambio**) y `[115,116)` (`k`, `unknown_effect`/`alloc`, cadena `closure.call[115,116)#function:51 > intrinsic.call[63,74)`) | ídem | NÚCLEO |
| T16 | `fn Main(){let g=fn ()->Bool{let r=MockOpen(1);return true;};match true{true if g()=>{},_=>{}}}` (94) | `checked` | `[79,80)`; known/`io`; cadena `closure.call#function:16 > intrinsic.call[34,42)#prelude:mock_open` | resumen de efectos del cuerpo | NÚCLEO |
| T17 | `fn Main(){let g=fn ()->Bool{let r=MockBlockingOpen(1);return true;};match true{true if g()=>{},_=>{}}}` (102) | `checked` | `[87,88)`; known/**`block`**; cadena `closure.call[87,88)#function:16 > value.drop[32,33)#binding:32 > resource.drop[sin span]` | req = primero aplicable | NÚCLEO |
| T18 | `fn Main(){let g=fn ()->Bool{let a=[1];let b=a;return true;};match true{true if g()=>{},_=>{}}}` (94) | `checked` | `[79,80)`; known/`alloc`; cadena `closure.call#function:16 > value.build_array[34,37)#Array` | idem | NÚCLEO |
| T19 | `gen fn S()->I64{yield 1;}fn Main(){let g=fn ()->Bool{var s=S();let n=Iterator.Next(edit s);return true;};match true{true if g()=>{},_=>{}}}` (139) | `checked` | `[124,125)`; known/**`suspend`**; cadena `closure.call[124,125)#function:41 > generator.next[78,82)#function:7 > generator.yield[16,21)#function:7` | idem | NÚCLEO |
| T20 | `fn Main(){let g=fn (x:view I64)->Bool{Print(x);return true;};match true{true if g(1)=>{},_=>{}}}` (96) | `checked` | `[80,81)` (`g`, sin argumentos); known/`io` | F0-0002 §8.2 «sin argumentos ni paréntesis» | NÚCLEO |
| T21 | `fn Main(){match true{true if (fn ()->Bool{Print(1);return true;})()==(fn ()->Bool{Print(2);return true;})()=>{},_=>{}}}` (119) | `checked` | **dos**: `[30,32)` y `[70,72)` (`fn` de cada literal); known/`io`; cadenas `closure.call[30,32)#function:30 > intrinsic.call[42,47)` y `closure.call[70,72)#function:70 > intrinsic.call[82,87)` | sitios independientes (§3.3) | NÚCLEO |
| T22 | `fn Main(){match true{true if (if true then {let h=fn ()->Bool{Print(1);return true;};value:h();} else {value:false;})=>{},_=>{}}}` (129) | `checked` | `[91,92)` (`h`); known/`io`; cadena `closure.call[91,92)#function:50 > intrinsic.call[62,67)` | closure creada y llamada dentro de la guarda | NÚCLEO |
| T23 | `fn Main(){let g=fn ()->Bool{let h=fn ()->Bool{Print(1);return true;};return h();};match true{true if g()=>{},_=>{}}}` (116) | `checked` | `[101,102)`; cadena `closure.call[101,102)#function:16 > closure.call[76,77)#function:34 > intrinsic.call[46,51)` | closure definida dentro de otra | NÚCLEO |

### 2.2 Guardas dentro del cuerpo de una closure (D2)

| # | Fuente (longitud) | Estado base | Esperado | Cláusula | Clase |
|---|---|---|---|---|---|
| U01 | `fn Main(){let f=fn (){match true{true if MockUnknown()==()=>{},_=>{}}};}` (72) | `checked` | `[41,52)` (`MockUnknown`); `unknown_effect`/`alloc`; cadena `intrinsic.call[41,52)#prelude:mock_unknown`. Paridad: la misma guarda en el cuerpo de una `fn` da `[29,40)` con la misma cadena (F0-0003 §8.1 Q01) | §3.3: la regla de la guarda es general | NÚCLEO |
| U02 | `fn G()->Bool{Print(1);return true;}fn Main(){let f=fn (){match true{true if G()=>{},_=>{}}};}` (93) | `checked` | `[76,77)` (`G`); known/`io`; cadena `function.call[76,77)#function:3 > intrinsic.call[13,18)` | ídem | NÚCLEO |
| U03 | `fn Main(){let g=fn ()->Bool{Print(1);return true;};let f=fn (){match true{true if g()=>{},_=>{}}};}` (99) | `checked` | `[82,83)` (`g`); known/`io`; cadena `closure.call[82,83)#function:16 > intrinsic.call[28,33)` (**requiere S1 y S2**) | ídem + §3.3 efectos de llamada | NÚCLEO |
| U04 | `fn Main(){let f=fn (){let g=fn ()->Bool{Print(1);return true;};match true{true if g()=>{},_=>{}}};}` (99) | `checked` | `[82,83)`; cadena `closure.call[82,83)#function:28 > intrinsic.call[40,45)` (S1+S2) | ídem | NÚCLEO |
| U05 | `fn Main(){}gen fn S()->I64{let g=fn ()->Bool{Print(1);return true;};match true{true if g()=>{},_=>{}}yield 1;}` (110) | `checked` | `[87,88)`; cadena `closure.call[87,88)#function:33 > intrinsic.call[45,50)` (closure en `gen fn`; S1; la guarda directa en `gen fn` ya se rechaza hoy: `G()` en `gen fn` → `NX-E1305`, **sin cambio**) | ídem | NÚCLEO |

### 2.3 Positivos (deben seguir ACEPTADOS; sin cambio)

| # | Fuente | Estado base y esperado |
|---|---|---|
| P01 | `fn Main(){let g=fn ()->Bool{return true;};match true{true if g()=>{},_=>{}}}` (76) | `checked`; `run` unit |
| P02 | `fn Main(){let x=1;let g=fn ()->Bool{return x==1;};match true{true if g()=>{},_=>{}}}` (84) | `checked` (captura sin efecto) |
| P03 | `fn Main(){let a=[1];let g=fn [move a]()->Bool{return true;};match true{true if g()=>{},_=>{}}}` (94) | `checked` |
| P04 | `fn Main(){match true{true if (fn ()->Bool{return true;})()=>{},_=>{}}}` (70) | `checked` |
| P05 | `fn Main(){let a=[1];match true{true if (fn [move a]()->Bool{return true;})()=>{},_=>{}}}` (88) | `checked` (crear/llamar sin efecto) |
| P06 | `fn Main(){let g=fn ()->Bool{Print(1);return true;};match g(){true if true=>{},_=>{}}}` (85) | `checked`: el efecto está en el **escrutinio**, no en la guarda (sin `guard_effect`) |
| P07 | `fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true=>{g();},_=>{}}}` | `checked`: el efecto está en el cuerpo del brazo |
| P08 | `fn G()->Bool{return true;}fn Main(){match true{true if G()=>{},_=>{}}}` | `checked` (control de `fn` pura; ya existe, `j05_match_guards.rs` l.483) |

### 2.4 Precedencia y casos que NO cambian (ownership precede a effects)

| # | Fuente | Estado base = esperado |
|---|---|---|
| O01 | `fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true if move g()=>{},_=>{}}}` (90) | `NX-E1305/guard_transfer` (ownership) `[70,74)` (`move`); **sin** `guard_effect` (Q01: effects solo si ownership no falla) |
| O02 | igual con `edit g()` (90) | `NX-E1305/guard_mutation` `[70,74)` (`edit`); sin `guard_effect` |
| O03 | `fn Main(){var x=1;let g=fn [edit x]()->Bool{x=2;Print(x);return true;};match true{true if g()=>{},_=>{}}}` (105) | `NX-E1107` ownership `[90,91)` (`g`; llamada de lectura a closure que exige `edit`), **sin** `guard_effect` |
| O04 | `fn Main() with no_io{let g=fn ()->Bool{Print(1);return true;};match true{true if g()=>{},_=>{}}}` (96) | **cambia (S1)**: hoy `NX-E0415` `[81,82)` solo; esperado **dos** diagnósticos con el mismo primario `[81,82)`, orden por código: `NX-E0415` (sin cambio: `contract_origin [15,20)`, `related contract[15,20)`, operación `closure.call`) y `NX-E1305/guard_effect` (known/`io`, mismas cadenas, sin relaciones). Paridad exacta con `fn G()->Bool{Print(1);return true;}fn Main() with no_io{match true{true if G()=>{},_=>{}}}`, que **hoy** emite ambos. |

### 2.5 Posiciones sin defecto (familia medida)

Con `with no_io` y una closure con `Print`, `NX-E0415` se emite en **todas** las posiciones medidas: sentencia de expresión `g();` `[62,63)`, inicializador `let x=g();` `[68,69)`, condición de `if` `[65,66)`, condición de `while` `[68,69)`, `return g();` `[75,76)`, escrutinio `match g(){…}` `[68,69)`, cuerpo de brazo `[80,81)`, destino agrupado `(g)();` `[63,64)`, guarda `[81,82)`. El resumen de efectos de la closure se propaga por `Op::CallClosure` en `summaries`/`call_components_by_rank`/`find_witness`; la pérdida es **exclusiva de la regla de guarda** (único consumidor que enumera instrucciones en busca de `CallDirect`). Posiciones «closure devuelta por función», «como argumento» y «como campo» **no existen** en los perfiles: `fn Mk(){return fn ()->Bool{…};}` → `NX-E0006/later_adr` en `Mk` `[3,5)`; parámetro `f:view Bool` llamado → `NX-E0111/non_callable_target`; array de closures `[g]` → `NX-E0006/later_adr`; `struct` → `NX-E0006/user_types`. Sin cambio.

## 3. Causa raíz comprobada (archivo:línea de `origin/main`)

**D1.** `src/ir/effects.rs`, `analyze`, bucle de guardas:
- l.1787–1858: por cada función y cada guarda se recopilan candidatos. Los testigos **locales** dentro de la guarda (`local[index]`, l.1797–1805: intrínsecos, copias, drops, creación de la closure) se convierten en diagnóstico (por eso `MockUnknown()` directo y la creación de una closure con captura `copy` de un array **sí** se rechazan hoy).
- l.1808–1852: para las **llamadas**, el bucle itera `Op` y exige `let Op::CallDirect { function, .. } = &instruction.op else { continue; };` (**l.1810**). `Op::CallClosure { site, .. }` cae en `continue`: la llamada a una closure en la guarda nunca produce candidato. El resumen del destino (`summaries[target]`, que sí incluye las closures, l.1494–1507) y el testigo (`find_witness`, que sí atraviesa `CallClosure`, l.4174) existen y no se consultan.
- El paso causal que se antepone (l.1832–1849) está fijado a `CauseOperation::FunctionCall` y a la entidad `function:<name_span.start de la fn fuente>`; una closure no tiene `name_span` (su identidad es el offset del token `fn`).

**D2.** `src/ir/effects.rs`:
- l.133: `scope_sites(&closure.body, &mut Vec::new(), &mut calls);` — las guardas halladas en el cuerpo de cada closure se vuelcan a un `Vec` temporal y se pierden (el comentario de `from_ast` lo declara: «Closure bodies are separate scopes: their guards are not their parent's», pero nadie las usa para ellas mismas).
- l.1789–1791: `let Some(source_function) = source_functions.get(f.name.as_str()).copied() else { continue; };` salta toda función cuyo nombre no es una `fn` fuente; las funciones de closure se llaman `$closure:<site>` (`lower.rs:1069`) y nunca llegan al bucle de guardas.

**Origen común:** el análisis de guardas se escribió para `fn` con nombre (único invocable con resolución de destino por `name_span`) y se amplió después a closures (F0-0003 J09) en el cálculo de resúmenes, sin llevar la ampliación a la regla de guarda.

## 4. Familia: barrido

| Posición / construcción | Medida base | Veredicto |
|---|---|---|
| Llamada a closure en guarda (variable, literal, agrupada, enlace de patrón, anidada en `if`/binario, creada dentro de la guarda) | acepta con cualquier efecto (T01–T23) | **D1** |
| Guarda dentro del cuerpo de closure (directa o por llamada) | acepta (U01–U05) | **D2** |
| Guarda dentro de `gen fn`, `fn` anidada en `match`, `match` expresión | ya rechaza llamadas a `fn` | sin defecto (la llamada a closure sí es D1) |
| Condición de `while`/`if`, inicializador, argumento, `return`, escrutinio, cuerpo de brazo con contrato `with no_*` | `NX-E0415` correcto en todas (§2.5) | sin defecto |
| Creación de closure con captura `copy` de array dentro de la guarda | rechaza `value.copy` `alloc` (`[44,50)` en `…(fn [copy a]()->Bool{return true;})()…`) | sin defecto |
| Generador: `Iterator.Next(edit s)` directo en la guarda | siempre `guard_mutation` (ownership) antes de effects | inalcanzable para `guard_effect` |
| `edit g()`/`move g()` en guarda | `guard_mutation`/`guard_transfer` (ownership) | sin defecto (precedencia, O01–O02) |
| `for` en guarda / closure devuelta / como argumento / en array / como campo | no alcanzan effects (`NX-E0006`/`NX-E0111`) | no aplica |

## 5. Mecanismo único por solución

Ambas soluciones son locales a `src/ir/effects.rs`; ninguna toca `types.rs` (regla de pila F5: no aplica; `scripts/f5-stack65.py` sigue en la puerta). Sin ayudante nuevo exportado; sin parche por patrón de fuente: el único dato que se usa es `Op::CallClosure { site }` ya existente, no el texto.

### S1 — el bucle de guardas resuelve también `Op::CallClosure` (D1)

En el bucle de instrucciones de la guarda (l.1808–1852), sustituir `let Op::CallDirect { function, .. } = &instruction.op else { continue; }` por una selección de la **arista de llamada** igual a la que `find_witness` ya construye para sus `callers` (l.4160–4176, mismos tres datos):

- `Op::CallDirect { function, .. }` → `(callee = function.clone(), operation = CauseOperation::FunctionCall, declaración = None → se mantiene `source_functions.get(name).name_span.start`)`.
- `Op::CallClosure { site, .. }` → `(callee = FunctionId(format!("$closure:{site}")), operation = CauseOperation::ClosureCall, declaración = Some(*site))`.
- cualquier otra `Op` → `continue`.

El resto del flujo **no cambia**: `indices.get(&callee)` → `summaries[target]` → bit prohibido (`EFFECTS`, orden alloc/block/suspend/io) → `find_witness(program, ast, &call_sites, &local, target, bit)`; `primary = call_sites.head(&instruction.span, &f.source_text)` (que ya resuelve nombre, agrupado y literal: lagunas «destino agrupado»); `witness.chain.insert(0, Cause { kind: Call, operation, span: Some(primary), entity: function:<declaración>, evidence: Verified })`. El paso antepuesto toma `operation` de la arista (ya no la constante `FunctionCall`) y el id de entidad de `declaración.unwrap_or(<name_span de la fn>)`. Deduplicación por sitio (`reported_guard_sites`) y orden `(site, rank)` sin cambio.

Por qué es el único mecanismo: la regla se evalúa por instrucción de la guarda en un solo lugar; todas las formas (variable, literal, agrupada, enlace de patrón, anidada) llegan como `Op::CallClosure` con el mismo `site`; el `site` identifica el literal por su token `fn`, no depende del enlace.

**Efectos laterales (S1):** (i) un programa con guarda que llama a closure con efecto deja de aceptarse: cambio de aceptación intencional (§2.1); (ii) una `fn … with no_*` con closure efectiva en guarda emite ahora **dos** diagnósticos en el mismo sitio (O04), como ya hace la `fn` (`B10f`); (iii) cuotas: `effect_fixpoint_steps` (`charge_effect`) no se toca; `find_witness` no cobra cuota (solo `count_cfg_work` de pruebas); sin cambio de cuotas ni de ticks de ejecución (el rechazo precede a la ejecución); (iv) orden de diagnósticos: `errors.sort_by(primario, código, subcausa)` final sin cambio; (v) no cambia ninguna salida de programa aceptado salvo los de (i).

### S2 — las guardas de cada closure se enumeran y se comprueban (D2)

- `CallSites::from_ast` (l.121–137): devolver además un mapa `closure_guards: HashMap<u64, Vec<Span>>` con las guardas de cada cuerpo de closure, clave `closure.fn_span.start` (igual a `site`, dato `inferred.closures[fn_span.start].site` y sufijo de `$closure:<site>`): sustituir `scope_sites(&closure.body, &mut Vec::new(), &mut calls)` por un `Vec` propio que se inserta en el mapa. El único llamador de producción es `analyze` (l.1421); el llamador de pruebas (`l.4405`, `CallSites::from_ast(ast).0`) ya usa solo el primer campo.
- Bucle de guardas (l.1788–1792): obtener `own_guards` de `function_guards[source_ranks[name]]` si `f.name` es una `fn` fuente, o de `closure_guards` por el sufijo numérico de `$closure:<site>` si es una closure; en cualquier otro caso, vacío. La obtención de `source_function` (necesaria solo para las restricciones `with`, l.1860 y siguientes) pasa **después** del bucle de guardas, conservando su `continue` para funciones sin declaración fuente.

Por qué es el único mecanismo: recolecta las guardas de cada invocable (que ya existían en `scope_sites`) y las pasa por el **mismo** bucle de S1; no se escribe una segunda comprobación para closures.

**Efectos laterales (S2):** (i) el bucle recorre las instrucciones de la closure solo si tiene guardas (cuerpo de coste nulo en el resto); (ii) dedupe por `(index, start, end)` con `index` de la función de closure: sin colisión con la `fn` envolvente; (iii) **desplazamiento de línea** en `effects.rs`: el auditor `scripts/i07c_span_audit.py` es **por número de línea** (todas sus entradas lo son; no tiene modo función+expresión) y fija `REVIEWED_EFFECTS` en `1223: guard.span().clone()`. S2 solo cambia **esa clave de línea**, conservando intacta la expresión, la clase (`identity`) y el contexto (`Frame::Guard(guard)`): la comprobación de vecindad del auditor (l.264) sigue exigiendo expresión y contexto, así que no se debilita. S1 no desplaza la línea (solo toca l.1810 en adelante). Pasar el auditor a clave función+expresión es **un corte aparte**, fuera de este plan; S2 inserta líneas antes de ella y C debe actualizar esa clave a la línea real tras el commit (medido con el candidato de A: 1226; C **recalcula**); (iv) ningún otro cambio de comportamiento (no se emiten diagnósticos nuevos fuera de guardas de closure).

## 6. Efectos laterales globales y salidas protegidas

- **Tests existentes que cambian (medidos, 2 de 2.205):**
  1. `tests/i07c_group_primary.rs::effect_diagnostics_report_the_target_name_of_a_grouped_call` (`run_family("effects")`): **12 filas** `head_no_io_{name,literal,if}_{1,2}_guard` y `head_no_suspend_{name,literal,if}_{1,2}_guard` (los `name:` consecutivos de `CASES`, cada uno con `family: "effects"`). Su `source`/`control` (closure con efecto en guarda + contrato) producen ahora **dos** diagnósticos (el de contrato, sin cambio, + `NX-E1305/guard_effect`) y su `repair` (la misma fuente sin `with`) se rechaza (la guarda sigue llamando a una closure con `Print`/`MockUnknown`). Medido por fila (f0-0003; `Print` → req `io`, `MockUnknown` → req `alloc`, evidencia `unknown_effect`): ver §7.2. **Cambio exigido (S1, mismo commit):** sacar esas 12 filas de `CASES` a una tabla propia `GUARD_CONTRACT_CASES` con un test `guard_calls_report_the_contract_and_the_guard_effect_at_one_site` (§7.2). Los otros 113 casos y `one_diagnostic` no cambian.
  2. `tests/i07c_structural_invariants.rs::grouped_operands_keep_one_causal_primary`: ejecuta `scripts/i07c_span_audit.py`; falla solo por el desplazamiento de línea de `REVIEWED_EFFECTS` (§5 S2 iii). Cambio: **solo** la clave de línea en el commit S2.
- Lista de **protegidos sin cambio de salida** (esperado actual = nuevo): `tests/j05_match_guards.rs` completo (incluye `transitive_guard_effect_uses_callee_summary_and_call_site`, `q01_*`, `independent_guard_effect_sites_*`), `tests/f0005_b1b.rs` l.624, `tests/j16_integrated.rs` l.661, `src/diagnostic.rs` l.2547, las filas `guard_intrinsic_call`, `guard_function_call` y `guard_{mutation,transfer}_*` de `i07c_group_primary.rs`, el corpus `corpus/` y `corpus/objetivo/` (ninguna fuente llama a una closure en una guarda: medido por `git grep` y por el diferencial de §8), las 152 filas RD y la batería de la puerta completa. El diferencial de §8 mide 840 fuentes con 0 cambios fuera de las clases esperadas.
- **PARAR y consultar** si, tras S1/S2, cambia algún otro test, fila RD, programa de `corpus/`/`corpus/objetivo/` o fixture (no ajustar esperados), o si el medidor de objetivos baja.
- **Pila:** no se toca `src/check/types.rs`; `scripts/f5-stack65.py` y el medidor F5 forman parte de `validate.sh` sin cambio esperado.
- **Documentos:** C no toca ningún ADR; el texto de L3 (§10) lo adopta el coordinador en el addendum antes de implementar. Al cerrar el corte C actualiza `Pending.md` (filas `S6COR-GUARDA-MATCH-AGRUPADA`/`S6COR-PENDIENTES`) y `docs/validation/F0-0002-I07c-defectos-abiertos.md` fila 33 (a «cerrada» con evidencia por ejecución), sin reescribir y sin afirmar PASS antes de tiempo.

## 7. Pruebas

### 7.1 Regresiones (esperados transcritos de §2; spans por bytes; no del binario)

Dos niveles, ambos en archivos ya registrados en `tests/suite.rs` (no se crea archivo de test):

1. **API de biblioteca, perfil `JNormal`** (`tests/j05_match_guards.rs`; helpers `rejected`, `rejected_sites`, `unit_events` existentes). Tests nuevos (nombres sin recuentos de pasada):
   - `closure_call_in_guard_reports_guard_effect_at_the_callee_name`: T01 (primario `span(70,71)`, `required_effect=io`, `effect_evidence=known_effect`, `Stage::Effects`, `Operation::EffectsCheck`, cadena `[ClosureCall, IntrinsicCall]`), T02 (`unknown_effect`/`alloc`), T08/T09 (agrupación no cambia el primario), T10 (`span(30,32)`).
   - `closure_call_in_guard_uses_transitive_and_independent_sites`: T03 (cadena de 3 pasos), T04, T14 y T15 con `rejected_sites` (lista de `(código, subcausa, inicio)` en orden: T14 `[116,121]`, T15 `[110,115]`), T21.
   - `closure_call_in_guard_keeps_ownership_precedence`: O01, O02 con `rejected` (códigos `guard_transfer`/`guard_mutation`, primario `[70,74)`) y O03 (`NX-E1107`).
   - `pure_closure_call_in_guard_is_accepted`: P01–P05 con `unit_events(…, &[])` (aceptan y ejecutan sin eventos).
   - `guard_inside_a_closure_body_is_checked`: U01, U02 (S2) y, tras S2, U03–U05.
2. **CLI en `f0-0003-b-v1` y `f0-0005-b-v1`, `check` y `run`** (`tests/i07c_group_primary.rs`, helper `invoke`, constantes `J`/`F5`, `span_of`/`section`): una tabla `GUARD_CLOSURE_CASES` con T01, T02, T03, T10, T11, T14, T16, T17, T18, T19, O04 (los dos diagnósticos) y los positivos P01–P07, que comprueba código, subcausa, etapa, operación, primario, `data` completo (con `effect_model` del perfil), `related_spans=[]`, cadena (operaciones y spans), exit code y que `check`/`run` coinciden. En `f0-0002-b-v2` una fila `NX-E0006` `pending_scope=F0-0003` en `match`/`fn` (sin cambio) como control de perfil. Test: `closure_calls_in_guards_report_guard_effect_in_every_profile`.

**Negativo aislado por regla con su positivo:** cada negativo rompe un solo atributo de la fuente (la presencia del efecto en el cuerpo de la closure) y su positivo es el mismo programa con el cuerpo `return true;` (T01↔P01, T05↔P02, T07↔P03, T10↔P04). Un negativo que falla por sintaxis accidental no cuenta: cada fuente de §2 se validó por ejecución con la salida citada.

**Oráculo de paridad (no sustituye al esperado literal):** para T01–T20 la fuente con `fn G()` equivalente produce el mismo conjunto `(código, subcausa, data, operaciones de cadena con closure.call↔function.call, texto del primario)`; el script de §8 lo comprueba para 180 pares del corpus.

### 7.2 Tabla de las 12 filas reubicadas (medida f0-0003; transcribir a `GUARD_CONTRACT_CASES`)

Cada fila produce **2 diagnósticos con el mismo primario**, ordenados por código (`NX-E0414` o `NX-E0415` < `NX-E1305`). El 1.º conserva **exactamente** los campos que la fila tiene hoy (código, `stage=effects`, `operation=closure.call`, primario, `data`, `related contract`, cadena). El 2.º es `NX-E1305/guard_effect`, `stage=effects`, `operation=effects.check`, mismo primario, `related_spans=[]`, `data={effect_evidence = el del 1.º, effect_model, required_effect}` y la **misma cadena** que el 1.º; `required_effect`: `io` en las filas `no_io` (`Print`), **`alloc`** en las `no_suspend` (`MockUnknown`: el primero de alloc,block,suspend,io, §8.1 Q01; el 1.º lleva `suspend`). `repair` de cada fila: el mismo programa **sin `with` y con el cuerpo de la closure `return true;`** (aceptado: P01/P04); las filas `if_*` no tienen `control` (cadena vacía en la tabla actual; se conserva). Verificar las 12 contra §2.4 O04.

| Fila (nombre actual) | Primario 1.º y 2.º (`source`) |
|---|---|
| `head_no_io_name_1_guard` | `[83,84)` `f` |
| `head_no_io_name_2_guard` | `[84,85)` `f` |
| `head_no_io_literal_1_guard` | `[41,43)` `fn` |
| `head_no_io_literal_2_guard` | `[42,44)` `fn` |
| `head_no_io_if_1_guard` | `[83,85)` `if` |
| `head_no_io_if_2_guard` | `[84,86)` `if` |
| `head_no_suspend_name_1_guard` | `[93,94)` `f` |
| `head_no_suspend_name_2_guard` | `[94,95)` `f` |
| `head_no_suspend_literal_1_guard` | `[46,48)` `fn` |
| `head_no_suspend_literal_2_guard` | `[47,49)` `fn` |
| `head_no_suspend_if_1_guard` | `[93,95)` `if` |
| `head_no_suspend_if_2_guard` | `[94,96)` `if` |

**Exigencias adicionales (medidas con el candidato; B r1 las reprodujo):** (i) el `control` de las filas con `control` no vacío (las 8 filas `name` y `literal`; las 4 `if_*` no tienen `control`) también produce **DOS** diagnósticos: el del contrato y `NX-E1305/guard_effect`, ambos con primario igual al `control_primary` de la fila (`[82,83)` `f` en `head_no_io_name_*`, `[40,42)` `fn` en `head_no_io_literal_*`, `[92,93)` `f` y `[45,47)` `fn` en las `no_suspend`), con el mismo reparto de campos que el `source`; (ii) el test nuevo `guard_calls_report_the_contract_and_the_guard_effect_at_one_site` **no reutiliza `one_diagnostic`** (que exige exactamente un diagnóstico, `i07c_group_primary.rs` l.2862–2867): usa un ayudante propio que exige **exactamente dos**, en orden de código, con el mismo primario, y comprueba cada uno por separado; (iii) `check` y `run` dan lo mismo en las tres fuentes de cada fila.

(Los primarios de `control` y la cadena siguen siendo los de la fila actual desplazados como hoy; C transcribe los del archivo y comprueba cada uno contra la salida del binario base **antes** de S1, que es el esperado «sin cambio» del 1.º diagnóstico, y contra §2.4 para el 2.º.)

### 7.3 Corpus congelado, diferencial y fuzz (ANDAMIAJE)

- **C0** crea `scripts/i07c_guarda_closure_diff.py` (§8, Apéndice A). Genera **840 fuentes** deterministas (sin azar; orden fijo de listas) y exige que el SHA-256 del JSON generado sea **`b82fa323f5d4c8ab0f0101017e02206b08857066aeeff1d06310e3a5ea97b860`** (calculado por A con `json.dumps(out, indent=0)`, sin LF final, UTF-8; si C obtiene otro valor, el generador del plan no se reprodujo y no se continúa).
- **Binario base** = caché de `origin/main` (`gate.sh <commit> --cached-bin debug|release` si el commit tiene registro verde; si no, `git archive` + build). Se ejecutan `check` **y** `run`, en `f0-0003-b-v1` y `f0-0005-b-v1`, debug y release.
- **Resultado medido por A (debug, prototipo S1+S2 frente a base, 840 fuentes × `check,run` × 2 perfiles):** cambian **exactamente** las clases `closure_guard_eff` (360), `literal_guard_eff` (24), `closure_param/transitive/chain/capture_read/capture_move/pattern_bound/two_calls_eff` (12 c/u), `guard_in_closure_{fn,cl,direct}_eff` (12 c/u); **0 diferencias** en las demás: `closure_guard_pure` (120), `fn_guard_{eff,pure}` (360+120), `contract_{cl,fn}_{eff,pure}`, `closure_outside_guard` (480), las `*_pure` y `ownership_or_other` (16). Los 180 pares `closure_guard_eff`/`fn_guard_eff` coinciden por paridad (0 discrepancias).
- **Criterio del script (falla si):** (a) una fuente de una clase «sin cambio» difiere entre base y candidato en `check` o en `run` (código de salida, `outcome.kind` o `diagnostics`); (b) una fuente de una clase «cambia» no queda en rechazo con el conjunto esperado (paridad con su par `fn`); (c) cualquier salida con `internal_error`, `InvalidIr`, pánico (código ≠ 0/1 o `stderr` no vacío) o tiempo > 60 s (salvo la nota de escalado); (a) se aplica también a las fuentes de `--gen2`.
- **Diferencial `fn+match` con las semillas del corte S5c (B r1 las ejecutó: 4.500 fuentes × `check`/`run`, base vs candidato y debug vs release, 0 diferencias, 0 pánicos).** Generador `gen2.py` = blob `11ac8537bda405ae60b97a444dea112d2e41a5dc` de `origin/i07c-estructural-r2:docs/validation/i07c-s5c/b-d2/gen2.py` (SHA-256 del contenido `11f10ff1cc3dab3bcbc7f8964951de23634b39549bdff1f8b9f23f477b357b90`; C lo extrae con `git show origin/i07c-estructural-r2:docs/validation/i07c-s5c/b-d2/gen2.py` y comprueba ambos hashes; no se copia al repositorio). Comando por semilla: `python gen2.py SEMILLA 1500 out.json`; SHA-256 del `out.json`: **6112023 = `48f6e809e10f701ae6b746ff21a523bf06c9cd8e9e1177da75b4b43b688213c4`; 1357911 = `9e4c4d9f96dbbe469d30f6aeb163bb69576b0cba3a8bb07b41674bb373ffe513`; 424242 = `f9952957f7093440140c2e831551fdb1e3ce11996bceb0fed57bc32eb96bab3e`**. El script de C0 acepta `--gen2 <semilla>` (ejecuta el generador, verifica el hash y corre las 1.500 fuentes). **Criterio:** 0 cambios de aceptación (aceptada/rechazada), de código, de subcausa ni de primario entre base y candidato, en `check` y `run`, **fuera de las clases con efecto** (closure llamada en guarda y guarda en closure con efecto); estas fuentes (`fn`+`match` sin closures) deben dar 0 cambios en todas.
- **Fuzz sin pánicos (generador congelado, Apéndice C):** `python fuzzgen.py 3000 fuzz.json` (semilla fija 20261123; `random.Random(20261123)`), SHA-256 del JSON generado **`a797ef463114cde39105012fd6f85e210ae7ab9b027ac5211ee87be092c90e01`** (calculado por A; 3.000 fuentes, 2.573 distintas). Se ejecuta en `check` y `run`, `f0-0003-b-v1` y `f0-0005-b-v1`, base y candidato, debug y release. Exige solo (c), que `check` y `run` coincidan en aceptación, que debug y release coincidan, y que **ninguna fuente rechazada por la base pase a aceptada** en el candidato (los rechazos solo pueden aumentar). B r1 ejecutó un generador propio equivalente (3.000 fuentes, 12.000 ejecuciones): 0 anomalías.
- **Nota de escalado (preexistente, no bloqueante):** el informe de `guard_effect` escala de forma superlineal con (guardas × llamadas × cadena). Medido por B r1 en debug: 400 guardas × 8 llamadas con cadena de 40 closures: base 14,7 s (acepta), candidato 39,7 s (3.200 diagnósticos); la `fn` equivalente en la **base** tarda 53 s (3.200 diagnósticos); 200×20 supera 60 s en el candidato (base 22,7 s). S1 no introduce el escalado (la `fn` ya lo tiene). **El criterio (c) de 60 s no se aplica a esos tamaños**; se registra en `Pending.md` (fila nueva, con fecha) y el corpus del plan no excede 2 guardas y 3 llamadas por programa.
- **Diferencial debug/release:** el mismo script con el binario release; las salidas deben ser idénticas a las de debug.

### 7.4 Mutación inversa por solución (núcleo)

Restauración por SHA-256 del archivo; suite completa, perfil `default`, debug y release; cada mutante debe MORIR (primer fallo basta) con el test indicado:

| Mutante | Cambio | Debe matar |
|---|---|---|
| M1 (S1) | volver a `let Op::CallDirect { function, .. } = &instruction.op else { continue; }` | `closure_call_in_guard_reports_guard_effect_at_the_callee_name` y `closure_calls_in_guards_report_guard_effect_in_every_profile` |
| M2 (S1) | el paso antepuesto vuelve a `CauseOperation::FunctionCall` | `…at_the_callee_name` (cadena `[ClosureCall, …]`) |
| M3 (S1) | `declaración` ignorada (entidad `function:0`, como hace la base cuando el destino no es una `fn` fuente) | la fila CLI de entidad (`function:<offset de fn>`, L1 = A; firme e incondicional) |
| M4 (S2) | volver a `scope_sites(&closure.body, &mut Vec::new(), …)` | `guard_inside_a_closure_body_is_checked` (U01/U02) |
| M5 (S2) | el salto `else { continue; }` antes del bucle de guardas (vuelve a omitir closures) | ídem |

Además, **una pasada por corte** (AGENTS «Mutaciones»; compensación obligatoria mientras el implementador C sea un agente Sonnet por agotamiento de créditos Codex): mutación de la región tocada (`analyze` guardas + `CallSites::from_ast`) con población congelada por SHA-256 antes de mutar; veredicto MUERTO por primer fallo, SUPERVIVIENTE con suite completa en debug y release; un superviviente sin programa que lo exhiba se registra en `Pending.md` con fecha, sin bloquear.

## 8. Orden, commits y puertas

Rama de trabajo del implementador: la que indique el coordinador (no `main`). **Un commit por solución, con su identificador en el mensaje**:

| Commit | Contenido | Archivos (matriz) |
|---|---|---|
| **C0** (andamiaje) | script diferencial + generador congelado + `--self-test` (el hash del corpus; que `base==base` da 0 diferencias; que un binario con un cambio de salida arbitrario hace fallar el criterio (a)) | `scripts/i07c_guarda_closure_diff.py` (nuevo). No toca producto ni tests |
| **S1** | mecanismo S1; regresiones §7.1 (API + CLI) salvo las de closure-en-guarda-de-closure (U03–U05); reubicación de las 12 filas (§7.2) | `src/ir/effects.rs`; `tests/j05_match_guards.rs`; `tests/i07c_group_primary.rs` |
| **S2** | mecanismo S2; filas U01–U05; clave de línea del auditor | `src/ir/effects.rs`; `tests/j05_match_guards.rs`; `scripts/i07c_span_audit.py` (clave de `REVIEWED_EFFECTS`) |
| **D1** (documental) | `Pending.md`, `docs/validation/F0-0002-I07c-defectos-abiertos.md` fila 33 | solo documentos (nunca ADR) |

Ningún otro archivo cambia. `tests/suite.rs` no cambia (ninguna prueba nueva en archivo nuevo). **Reversión:** S2 depende de S1 solo para U03–U05; U01/U02 pasan con S2 sola. Revertir S2 deja S1 y el auditor en verde (la clave de línea vuelve con el commit); revertir S1 con S2 presente rompe U03–U05: se revierte primero S2.

**Puerta mecánica antes de entregar a D (por C):** `sh scripts/validate.sh` en clon limpio (fmt, clippy `-D warnings`, tests debug y release, RD, `check-nextest-selection.py`, `f5-stack65.py`), con cerrojo `scripts/heavy-lock.py`; el diferencial de §7.3 sobre debug y release; el medidor de objetivos (`corpus/objetivo/`) sin pérdida; la pasada de mutación de §7.4. D/D'/integración usan siempre la puerta completa. La puerta rápida solo para iterar.

## 9. Alternativas descartadas

- **Filtrar en `lower`/ejecución** el efecto de la guarda: la regla es de la etapa `effects` (F0-0003 §3.3 matriz de barreras) y tendría dos rutas.
- **Sintetizar una `Op::CallDirect` para la closure** antes del bucle: duplica el mapa de llamadas y esconde la operación `closure.call` (F0-0003 §8.1 exige esa operación en la cadena).
- **Resolver por el texto del destino** (`g` es un enlace a un literal `fn`): parche por patrón de fuente y falla con enlaces de patrón (T11) y llamadas por `if` agrupado.
- **Tratar S2 como parte de S1:** distinta causa raíz (recolección vs. resolución); dos commits permiten revertir uno solo.
- **Rechazar toda llamada a closure en guarda:** F0-0003 §3.3 rechaza por **efecto**, no por forma (P01–P05 deben seguir aceptadas).
- **Cambiar los tests `head_*_guard` para que dejen de llamar a una closure efectiva:** el contrato `with no_*` solo se incumple con un efecto, que la guarda prohíbe a su vez; no existe fuente con una sola de las dos infracciones. Se conserva el escenario y se corrige la expectativa (§7.2).

## 10. Decisiones del coordinador sobre las lagunas (firmes; sin condiciones)

- **L1 — `Entity` del paso `closure.call` de `guard_effect`: lectura A.** `{kind: function, id: function:<offset del token fn del literal>}`, igual que ya emiten `NX-E0412..NX-E0415` y coherente con `function:7` de la fixture F04 (F0-0003 §8.1 solo dice «Entity se escribe completa»). Las filas de `Entity` de §2 y §7.1 son incondicionales; M3 es mutante firme.
- **L2 — alcance: lectura 1.** F0-0003 §3.3 prohíbe efectos en toda guarda y «propio contexto» de los invocables anidados no los exime. S2 entra en el corte como **commit propio, revertible**, con su puerta mecánica y su mutación por separado (§8, §7.4).
- **L3 — se adopta el texto de reemplazo** de la fila «guarda de `match`» de `docs/adr/F0-0002-lagunas.md` (relleno de laguna revertible, verificado por B r1 con `main` y con S1+S2); **lo incorpora el coordinador antes de implementar**; C no toca ningún ADR. Texto (fila de la tabla «Sitio de la llamada con destino agrupado que no es un nombre»):

| guarda de `match` | inválido: `fn Main() with no_io{match true{true if (fn ()->Bool{Print(1);return true;})()=>{},_=>{}}}` (90 bytes) | dos diagnósticos del mismo sitio, orden por código: `NX-E0415`, `closure.call`, `[41,43)` (`fn`), `contract_origin [15,20)`, pasos `[41,43)` e `intrinsic.call [53,58)`; y `NX-E1305/guard_effect` (effects/effects.check) `[41,43)`, `required_effect=io`, `effect_evidence=known_effect`, mismos pasos, sin relaciones (F0-0003 §3.3: la guarda prohíbe io; el contrato `no_io` es una regla independiente) | válido: `fn Main() with no_io{match true{true if (fn ()->Bool{return true;})()=>{},_=>{}}}` (81 bytes; `check` checked, `run` unit). Sin `with no_io` y con `Print(1)` sigue inválido: solo `NX-E1305/guard_effect` `[30,32)` (79 bytes). |

(Medido en `main` `e7b1813`: la fuente inválida da solo `NX-E0415` y la que no lleva `with` se acepta; con S1 dan lo anterior.) Las 12 filas `head_*_guard` de `tests/i07c_group_primary.rs` se corrigen en S1 (§7.2).

## 11. Riesgos y puntos que B debe vigilar

1. **Esperados.** Todas las cifras de §2 salen de ejecutar el binario base y el candidato de A; la **derivación** (cláusulas de §1) es independiente de esa ejecución: primario = nombre/cabeza del destino (F0-0002 §8.2 y lagunas), `req` = primer efecto prohibido por orden alloc/block/suspend/io (§8.1 Q01), cadena = testigo más corto. B debe contrastar al menos T01, T10, T14, O04 y U01 contra el ADR, no contra la ejecución.
2. **Doble diagnóstico con contrato (O04, §7.2).** Es la consecuencia de que la `fn` ya los emite ambos (`B10f`); B debe confirmar que el ADR no pide supresión causal entre `NX-E0415` y `NX-E1305` (F0-0003 §3.3 «no borra sitios independientes»; son el mismo sitio con dos reglas distintas, un contrato y la guarda).
3. **L1, L2, L3 decididas (§10).** B verifica que el plan no deja ninguna condición pendiente y que el texto de L3 coincide con lo medido (90/81/79 bytes y offsets).
4. **Filas reubicadas (12).** B debe comprobar que ninguna se pierde y que el 1.º diagnóstico de cada una conserva literalmente los campos actuales; la reubicación no debe rebajar el requisito (cada fila sigue comprobando primario, `data`, relación y cadena del contrato).
5. **Auditor por línea.** S2 desplaza una línea de `effects.rs`; C debe recalcular la clave y no cambiar la clasificación del sitio (`identity`, `guard.span().clone()`). Si la implementación de S2 inserta más o menos líneas que el candidato de A, la clave cambia: no es un defecto del plan.
6. **Rendimiento.** `find_witness` se invoca ahora por cada llamada a closure en guarda (como ya ocurre por llamada a `fn`); no cobra cuota. Un programa con muchas guardas y muchas llamadas puede tardar más: medir con el fuzz de 3.000 fuentes (criterio de 60 s, salvo la nota de escalado de §7.3) y registrar un cuelgue en tamaños pequeños como defecto bloqueante de andamiaje.
7. **Perfiles.** `f0-0005-b-v1` coincide con `f0-0003-b-v1` salvo `effect_model`; `f0-0002-b-v2` no admite closures ni `match`. B debe rechazar un plan de pruebas que solo cubra un perfil.
8. **Precedencia ownership.** O01–O03 dependen de que ownership falle antes que effects (F0-0003 matriz de barreras). Un cambio en S1 no debe alterar esa barrera; O01–O03 son los testigos.
9. **Mutante M3 firme** (L1 = A): debe morir por la fila de `Entity` `function:<offset de fn>`; si sobrevive, la fila de entidad no se ejecuta y es defecto del plan.
10. **Inalcanzable hoy:** cualquier otra forma de invocar una closure (campo, argumento, retorno, array) no existe en el perfil; si un corte posterior la habilita deberá revisar este mecanismo (`Op::CallClosure` es la única operación de llamada a closure en el IR).

## Apéndice A — generador del corpus congelado (sin azar)

```python
"""Corpus congelado de S6COR-GUARDA-CLOSURE (sin azar: el orden es el de las listas)."""
import hashlib
import json

BODIES = [
    ("pure", "return true;", False),
    ("cmp", "return 1==1;", False),
    ("print", "Print(1);return true;", True),
    ("unknown", "MockUnknown();return true;", True),
    ("open", "let r=MockOpen(1);return true;", True),
    ("array", "let a=[1];return true;", True),
    ("req", "let q=MockRequest();return true;", True),
    ("block", "let r=MockBlockingOpen(1);return true;", True),
]
CALLS = ["{n}()", "({n})()", "(({n}))()", "{n}()==true", "(if {n}() then true else false)", "(if true then {n}() else false)"]
CTX = [
    "match true{true if %s=>{},_=>{}}",
    "match false{true if %s=>{},_=>{}}",
    "let x=match true{true if %s=>{value:1;},_=>{value:2;}};",
    "match Some(1){Some(v) if %s=>{},_=>{}}",
    "if true{match true{true if %s=>{},_=>{}}}",
]
out = []


def emit(kind, tag, src):
    out.append({"kind": kind, "tag": tag, "src": src})


def eff(kind, e):
    return kind + ("_eff" if e else "_pure")


for bid, body, e in BODIES:
    for ci, call in enumerate(CALLS):
        for xi, ctx in enumerate(CTX):
            tag = "%s-c%d-x%d" % (bid, ci, xi)
            emit(eff("closure_guard", e), tag, "fn Main(){let g=fn ()->Bool{%s};%s}" % (body, ctx % call.format(n="g")))
            emit(eff("fn_guard", e), tag, "fn G()->Bool{%s}fn Main(){%s}" % (body, ctx % call.format(n="G")))
            emit("closure_outside_guard", tag, "fn Main(){let g=fn ()->Bool{%s};let y=g();%s}" % (body, ctx % "true"))
for bid, body, e in BODIES:
    for gi, form in enumerate(["(fn ()->Bool{%s})()", "((fn ()->Bool{%s}))()"]):
        emit(eff("literal_guard", e), "%s-l%d" % (bid, gi), "fn Main(){match true{true if %s=>{},_=>{}}}" % (form % body))
    emit(eff("guard_in_closure_fn", e), bid, "fn G()->Bool{%s}fn Main(){let f=fn (){match true{true if G()=>{},_=>{}}};}" % body)
    emit(eff("guard_in_closure_cl", e), bid, "fn Main(){let g=fn ()->Bool{%s};let f=fn (){match true{true if g()=>{},_=>{}}};}" % body)
    emit(eff("guard_in_closure_direct", e), bid, "fn Main(){let f=fn (){match true{true if (if true then {%s value:true;} else {value:false;})=>{},_=>{}}};}" % body.replace("return true;", ""))
    emit(eff("contract_cl", e), bid, "fn Main() with no_io{let g=fn ()->Bool{%s};let y=g();}" % body)
    emit(eff("contract_fn", e), bid, "fn G()->Bool{%s}fn Main() with no_io{let y=G();}" % body)
    emit(eff("closure_param", e), bid, "fn Main(){let g=fn (x:view I64)->Bool{%s};match true{true if g(1)=>{},_=>{}}}" % body)
    emit(eff("closure_transitive", e), bid, "fn G()->Bool{%s}fn Main(){let g=fn ()->Bool{return G();};match true{true if g()=>{},_=>{}}}" % body)
    emit(eff("closure_chain", e), bid, "fn Main(){let h=fn ()->Bool{%s};let g=fn ()->Bool{return h();};match true{true if g()=>{},_=>{}}}" % body)
    emit(eff("closure_capture_read", e), bid, "fn Main(){let x=1;let g=fn ()->Bool{let y=x;%s};match true{true if g()=>{},_=>{}}}" % body)
    emit(eff("closure_capture_move", e), bid, "fn Main(){let z=[1];let g=fn [move z]()->Bool{%s};match true{true if g()=>{},_=>{}}}" % body)
    emit(eff("closure_pattern_bound", e), bid, "fn Main(){let g=fn ()->Bool{%s};match Some(g){Some(h) if h()=>{},_=>{}}}" % body)
    emit(eff("closure_two_calls", e), bid, "fn Main(){let g=fn ()->Bool{%s};match true{true if g()==g()=>{},_=>{}}}" % body)
for tag, src in [
    ("edit", "fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true if edit g()=>{},_=>{}}}"),
    ("move", "fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true if move g()=>{},_=>{}}}"),
    ("capedit", "fn Main(){var x=1;let g=fn [edit x]()->Bool{x=2;return true;};match true{true if g()=>{},_=>{}}}"),
    ("capedit2", "fn Main(){var x=1;let g=fn [edit x]()->Bool{x=2;Print(x);return true;};match true{true if edit g()=>{},_=>{}}}"),
    ("capmove", "fn Main(){let a=[1];let g=fn [move a]()->Bool{return true;};match true{true if g()=>{},_=>{}}}"),
    ("drop", "fn Main(){let a=Some(1);match true{true if (if true then {Drop(a);value:true;} else {value:false;})=>{},_=>{}}}"),
    ("scrutinee", "fn Main(){let g=fn ()->Bool{Print(1);return true;};match g(){true if true=>{},_=>{}}}"),
    ("arm_body", "fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true=>{g();},_=>{}}}"),
]:
    emit("ownership_or_other", tag, src)
text = json.dumps(out, indent=0)
open("corpus2.json", "w", newline="").write(text)
print(len(out), hashlib.sha256(text.encode()).hexdigest())
```

Resultado de A: `840 b82fa323f5d4c8ab0f0101017e02206b08857066aeeff1d06310e3a5ea97b860`. Clases que **deben cambiar** (todas terminan en `_eff` y son de closure llamada en guarda o guarda en closure): `closure_guard_eff`, `literal_guard_eff`, `closure_param_eff`, `closure_transitive_eff`, `closure_chain_eff`, `closure_capture_read_eff`, `closure_capture_move_eff`, `closure_pattern_bound_eff`, `closure_two_calls_eff`, `guard_in_closure_fn_eff`, `guard_in_closure_cl_eff`, `guard_in_closure_direct_eff`. Clases que **no** deben cambiar: todas las demás.

## Apéndice B — reproducción de las medidas de §2

`nexo check <fuente> --profile f0-0003-b-v1 --format json` sobre el binario base (`origin/main`) y sobre el candidato; la columna «Estado base» de §2 sale de la primera, «Esperado» de §1 y de la paridad con la `fn` equivalente. Tabla del número de diagnósticos de §2.1 transcrita del JSON de cada fuente (`diagnostics[*].{code,subcause,primary_span,data,cause_chain}`).

## Apéndice C — generador del fuzz congelado

```python
"""Fuzz congelado de S6COR-GUARDA-CLOSURE: python fuzzgen.py 3000 salida.json (semilla fija 20261123)."""
import hashlib
import json
import random
import sys

BODIES = ["return true;", "return 1==1;", "Print(1);return true;", "MockUnknown();return true;",
          "let r=MockOpen(1);return true;", "let a=[1];return true;", "let q=MockRequest();return true;",
          "let r=MockBlockingOpen(1);return true;"]
CTX = ["match true{true if %s=>{},_=>{}}", "match Some(1){Some(v) if %s=>{},_=>{}}",
       "let x=match true{true if %s=>{value:1;},_=>{value:2;}};", "if true{match true{true if %s=>{},_=>{}}}",
       "match true{true=>{match true{true if %s=>{},_=>{}}},_=>{}}"]


def group(rng, text):
    for _ in range(rng.randint(0, 3)):
        text = "(" + text + ")"
    return text


def call(rng, names):
    n = rng.choice(names)
    if n.startswith("fn "):
        return "(%s)()" % n[3:]
    return group(rng, n) + "()" if rng.random() < 0.3 else n + "()"


def guard(rng, names):
    parts = [call(rng, names) for _ in range(rng.randint(1, 3))]
    g = "==".join(parts)
    if len(parts) > 1 or rng.random() < 0.4:
        g = "(if true then %s else false)" % parts[0] if len(parts) == 1 else g
    return g


def make(rng):
    defs, names = "", []
    for i in range(rng.randint(0, 3)):
        body = rng.choice(BODIES)
        if rng.random() < 0.35 and names:
            body = "return %s();" % rng.choice(names)
        if rng.random() < 0.8:
            defs += "let g%d=fn ()->Bool{%s};" % (i, body)
            names.append("g%d" % i)
    lit = "fn ()->Bool{%s}" % rng.choice(BODIES)
    names.append("fn (%s)" % lit)
    ctx = rng.choice(CTX) % guard(rng, names)
    if rng.random() < 0.3:
        ctx = "let f=fn (){%s};" % ctx
    pre = ""
    if rng.random() < 0.3:
        pre = "fn G()->Bool{%s}" % rng.choice(BODIES)
        ctx = ctx.replace("g0()", "G()") if rng.random() < 0.5 else ctx
    return "%sfn Main(){%s%s}" % (pre, defs, ctx)


n, out = int(sys.argv[1]), sys.argv[2]
rng = random.Random(20261123)
progs = [make(rng) for _ in range(n)]
text = json.dumps(progs, indent=0)
open(out, "w", newline="").write(text)
print(len(progs), hashlib.sha256(text.encode()).hexdigest())
```

Resultado de A: `3000 a797ef463114cde39105012fd6f85e210ae7ab9b027ac5211ee87be092c90e01` (`python fuzzgen.py 3000 fuzz.json`, `json.dumps(progs, indent=0)`, sin LF final). Si C obtiene otro hash, no se reprodujo y no se continúa.
