# F0-0002 I07c — Las 58 filas abiertas clasificadas por estructura y coste del arreglo (2026-11-22)

Estado: **registro de trabajo, sin PASS ni aprobación de fase**. Análisis y documentación; no se implementó ni validó nada. Fuente de las 58 filas: `docs/validation/F0-0002-I07c-defectos-abiertos.md`. Binario: `.scratch/cand-bin/base-release.exe` (= `main`; sin cambios de `src/` desde `2875943` según ese registro), `check --format json --profile f0-0002-b-v2` salvo indicación. Oráculo: `scripts/i07c_s5c_oraculo.py --source` (solo arrays/uniones; **no modela `AssertEq`** ni algunas formas: donde se cita, se dice).

Criterio del propietario (vía el orquestador): no terminar con una lista anotada cuya corrección exija luego «mover el motor completo»; se cierra bien ahora y lo fácil puede esperar. Eje de este documento: **L = LOCAL** (uno o pocos sitios, sin rediseñar un mecanismo) frente a **E = ESTRUCTURAL** (exige rediseñar o reescribir un mecanismo del solver/checker). La plausibilidad del programa (columna «Real.») es dato complementario para priorizar dentro de cada clase.

## 1. Resumen

| | Filas |
|---|---:|
| **L** (local) | **10** |
| **E** (estructural) | **48** |
| Total | 58 |

Mecanismos estructurales (fila asignada a su mecanismo principal; sin doble cuenta): **M-A** siembra/planning 26, **M-B** ancla de unión 16, **M-C** procedencia y plan de lecturas 6. Los 10 L: S6-COR-2 7 filas (7 sitios, 8 líneas), guarda de `match` 1, asignación a destino agrupado 1, F5-operand-domain 1; estimación total en §5: **~12–15 sitios en ~10–11 commits**.

Hallazgo que condiciona la prioridad: **ninguna de las 58 filas rechaza un programa correcto.** Las 48 E son programas que ya contienen un error de tipos real; el defecto es un `NX-E0103` espurio sobre un `[]` que una asignación posterior resuelve, y/o un E1303/E0102 perdido, movido o contado de más. Hay 2 aceptaciones indebidas (filas 32 y 33, ambas L) y 1 `internal_error` (F5, L). Detalle de qué ve el usuario en §4.

## 2. Mecanismos estructurales: tabla mecanismo → filas

| Mecanismo (`src/check/types.rs`) | Filas (principal) | n | ¿Bien acotado? | Cierre previsto |
|---|---|---:|---|---|
| **M-A. Siembra de capas por planning y compuerta de orden**: `collect_planned_shapes` (~3143), `seed_planned_shape` (~3409), `retract_unconsumed_seeds` (~3377, solo en V6), mapas `shape_plan`/`concrete_plan` del `InferenceGraph`, compuerta `conflicts_prefix`/`pre_assignment_snapshot`, cierre en `defer_shape_conflict` (~5114) y `unresolved_constructor_shape_conflict` (~4956) | FAM-E 17 (F01–F05, F07, F09–F19), FAM-O 8 (O01–O08), FAM-2 F06 (E0103; su E1303 es de M-B) | 26 | **Una estructura, pero cruza dos capas** (la pasada de planning y la final no comparten semántica) y su corrección depende de M-C | S5a.1 (V6, rama; sin PASS) + S5a.5 «testigo causal» (F10/F11) + S5a.2 para O04/O05/O06 |
| **M-B. Ancla de unión y retracción**: `if_else_context` (~14219), `if_after_first` (~14159), `if_finish_join` (~13775), `anchor_error`; el diseño añade un registro de ancla por unión en el grafo | ancla 12 (R4, X_R1na, X_R1ord, X_flat3, Y_pend_if, Y_pend_if2, P03, P04, P05, P15, P16, P17) + F06 (E1303 de `ar`) + W0 + P1, c3, w1 (su E0103 hereda de W0) | 16 | **No: toca varias capas** (tres funciones `if_*`, un registro nuevo, y realimentación con `note_assignment_verdict` ~8607/`rejected_assignments`/`retract_unconsumed_seeds`; diseño S5c §4 «Terminación») | S5c-d2: rediseño con Codex el 2026-11-24 |
| **M-C. Procedencia y plan de lecturas**: `read_plan`, `assignment_type_plan` (guarda solo el primer RHS intrínseco por sitio), `rejected_assignments` (veredicto binario por binding, no por asignación) | X_R1inc, P2, w3, P3, P4pos, readplan | 6 | **Acotado en datos** (tres mapas), pero el arreglo previsto (ALT-4, registro append-only de procedencia; plan `I07c-S5a5-procedencia-requisitos.md`) reescribe qué se guarda y lo consume M-A | S5a.5 / S5c B2 |

Secundarios (no suman): P1, c3, w1 también dependen de M-C (relación `expected_type` que V6 deriva a la hoja sembrada); O04/O05/O06 también de S5a.2 (consumo de la denuncia de dominio de `+`/`-`, `pending_operation_guard`/`pending_operand_evidence` ~4084/4103, función acotada pero con coste de pila en `drive`); F06 también de M-B.

**Por qué un parche no basta (evidencia medida, no opinión):**
- M-A: la causa es que el planning calcula capas con una semántica distinta de la pasada final y el punto fijo se auto-sostiene sin retractación (`docs/correcciones/I07c-S5-causa-raiz.md` §4). «Quitar la siembra» rompe 21 tests protegidos y 2 fixtures (§0.6). V6 (`2296dc8`) elimina el E0103 espurio en 21 de 23 casos del twin con 0 B de pila, pero deriva la relación `expected_type` en P1, P2, w1, w3, c3, añade un E0103 espurio en P12 y deja F10/F11 con 2 E0103 cada uno (`docs/validation/i07c-s5a/limites-conocidos.md`; `I07c-S5-plan.md` §9). La variante C2 arregla R1–R3b pero pierde «A sin asignación» y deja X_R1inc, W0 y los 6 de §1 del diseño (ALT-1 descartada, `I07c-S5c-diseno-ancla.md` §4).
- M-B: el defecto ya existe en `main` sin siembra (diseño S5c §1): `if_else_context` descarta el ancla del `if` anidado. ALT-2 (registro por unión) **regresiona 15 programas que `main` y el oráculo aciertan** (unión anidada en primera posición con ancla de testigo parcial) y la desviación añade 12 más (N1–N4) (`I07c-S5c-plan-enmienda-B-r1.md` §3–4): el ancla exterior es el testigo parcial de la interior, de modo que una corrección local de `second_leaf_anchor` no basta (B, §4). Activa la PARADA del plan S5c §9.
- M-C: la deriva «aceptada previa + rechazada posterior» sobre un binding solo se corrige si el veredicto y el ancla pasan a ser por asignación (`limites-conocidos.md` «Causa»); eso cambia lo que M-A lee.

## 3. Las 58 filas

`Real.`: **R** ejemplo realista verificado que reproduce; **P** patrón reproducido con programa verosímil pero algo forzado; **F** no se encontró uno realista tras ≤3 intentos (o el programa es fabricado). `Ej.` remite a §4. «Esp.» = esperado del registro (oráculo O / adjudicación ADJ).

| # | Fila | Cl. | Mecanismo → corte | Real. | Qué cambia (en una línea) |
|---|---|---|---|---|---|
| 1 | F01 | E | M-A → S5a.1 | P (X1) | E0103 ×2 espurios; depende del orden de asignaciones |
| 2 | F02 | E | M-A | P (X2) | E0103 ×2 espurios |
| 3 | F03 | E | M-A | F | salida dependiente del orden; el programa usa `ar[r][[q,r]]`; esperado NO DERIVADO |
| 4 | F04 | E | M-A | P (X3) | E0103 ×2 espurios, E1303 movido |
| 5 | F05 | E | M-A | F | condiciones array y `AssertEq(x,[[q],r])` fabricadas |
| 6 | F07 | E | M-A | P (X4) | E0103 ×2 y E1303 en hoja extra; falta E0102 de `r=[3]` |
| 7 | F09 | E | M-A | F | `if [s,r[x]] {}` fabricado |
| 8 | F10 | E | M-A + S5a.5 | P (X4) | E0103 espurio; residuo que V6 no cierra |
| 9 | F11 | E | M-A + S5a.5 | P (X1) | original ya sin E0103 (S5b.1); el mínimo B_qr persiste; V6 deja 2 E0103 |
| 10 | F12 | E | M-A | F | `s1=(if ((if false then r else false)==r) …)` |
| 11 | F13 | E | M-A | F | condición array, `==` entre mixtos |
| 12 | F14 | E | M-A | F | `true+1` y condición array (análogo realista no reproduce) |
| 13 | F15 | E | M-A | P (X4) | E0103 ×2; E1303 de hoja posterior |
| 14 | F16 | E | M-A | F | `[q[1],[q]][0]` anotado; reproduce el E0103×2 pero el programa es forzado |
| 15 | F17 | E | M-A | F | `let a0:I64=false;` y ramas con bloques |
| 16 | F18 | E | M-A | F | reproduce E0103 ×2 pero con `Len(if … k else q)` fabricado; primer C derivable por §4.1.2.3–5 |
| 17 | F19 | E | M-A | F | `true+1`, `while r {}` |
| 18 | O01 | E | M-A | P (X6) | E0103 espurio; E1303 en hoja distinta |
| 19 | O02 | E | M-A | F | condiciones array; bucle `while [..]` |
| 20 | O03 | E | M-A | P (X6) | E0103 espurio; E1303 desplazado; E0102 de `r=[x]` cambia de sitio |
| 21 | O04 | E | M-A + S5a.2 | P (X6) | E0103 y E1303 además de un único E0102 de `+` |
| 22 | O05 | E | M-A + S5a.2 | F | esperado completo NO DERIVADO; análogo no reproduce |
| 23 | O06 | E | M-A + S5a.2 | F | operaciones sobre arrays y condiciones array |
| 24 | O07 | E | M-A | P (X6) | E0103 espurio; E1303 en `aar` en vez de `[r]`; pierde E0102 |
| 25 | O08 | E | M-A | F | análogo no reproduce |
| 26 | F06 | E | M-A + M-B | P (X7) | E0103 espurio; E1303 en `ar` en vez de `r`; pierde E0102 |
| 27 | R4 | E | M-B → S5c-d2 | P (X8) | E0103 espurio |
| 28 | X_R1na | E | M-B | P (X8) | E0103 espurio; primario y relación equivocados |
| 29 | X_R1ord | E | M-B | P (X8) | E0103 espurio; relación equivocada |
| 30 | X_flat3 | E | M-B | P (X8) | 2 E1303 en lugar de 1 (conteo) |
| 31 | Y_pend_if | E | M-B | P (X8) | E1303 de más junto al E0103 esperado |
| 32 | Y_pend_if2 | E | M-B | P (X8) | ídem |
| 33 | X_P03 | E | M-B | **R** (X8) | E0103 espurio sobre `var acc=[]` en cadena else-if |
| 34 | X_P04 | E | M-B | **R** (X8) | E0103 espurio y E1303 en `nums` en vez de en `1` |
| 35 | X_P05 | E | M-B | P (X8) | E1303 de más junto al E0103 esperado |
| 36 | X_P15 | E | M-B | **R** (X8) | E0103 espurio |
| 37 | X_P16 | E | M-B | **R** (X8) | pierde el E0102 de `acc=[true]` |
| 38 | X_P17 | E | M-B | **R** (X8) | E0103 espurio |
| 39 | W0 | E | M-B (causa NO MEDIDA) | P (X9) | E0103 espurio junto al E1303 correcto |
| 40 | P1 | E | M-B + M-C | P (X9) | E0103 espurio (hereda W0) |
| 41 | c3 | E | M-B + M-C | P (X9) | ídem, capas ≥ 3; análogo no verificado aparte |
| 42 | w1 | E | M-B + M-C | P (X9) | ídem y E0102 espurio `[87,88)`; análogo no verificado aparte |
| 43 | X_R1inc | E | M-C | P (X8) | retipa `q` y pierde el E0102 del `3` |
| 44 | P2 | E | M-C (+M-B) | F | `r=[r]` recursivo y dos asignaciones rechazadas mueven el primario |
| 45 | w3 | E | M-C | F | ídem |
| 46 | P3 | E | M-C → S5a.5 | P (X10) | solo la relación `expected_type` (rel. a `q` en lugar de `q=[[nums]]`) |
| 47 | P4pos | E | M-C (causa NO MEDIDA) | F | E0102 espurio en `r=[q]`; programa fabricado (`[]` como condición); DISCUTIBLE |
| 48 | readplan | E | M-C (`S5-LIMITE-READPLAN`) | **R** (X11) | primario en `[true]` en vez de `[1]`; pierde E0102 |
| 49 | F5-operand-domain | **L** | S5a.3 | **R** (X12) | `internal_error` (exit 2) en lugar de `NX-E0102` |
| 50 | S6 R0001 while `(cond)` | **L** | S6-COR-2 S2 | P (X13) | primario con paréntesis |
| 51 | S6 sentencia `(n+1);` | **L** | S6-COR-2 S1 | F | sentencia entre paréntesis |
| 52 | S6 destino `(n)=1` | **L** | S6-COR-2 S3 | F | destino entre paréntesis |
| 53 | S6 `let y=if … then (n)` | **L** | S6-COR-2 S4 | F | operando de rama agrupado |
| 54 | S6 conteo `(Id)(1);` | **L** | S6-COR-2 S5 | F | falta un tick por iteración; NO VERIFICADO hoy |
| 55 | S6 guarda `(x)=2` | **L** | S6-COR-2 S6 | F | primario `(x)` en lugar de `x` |
| 56 | S6 guarda `(Drop)(a)` | **L** | S6-COR-2 S7 | F | **aceptación indebida**; verificada (X14) |
| 57 | Guarda de `match` agrupada | **L** | corte nuevo tras S6-COR-2 | F | aceptación sin E1305; DISCUTIBLE (X14) |
| 58 | Asignación a destino agrupado | **L** | corte nuevo tras S6-COR-2 | F | 3 E0102 en lugar de 2; solo con `var q1=[]` pendiente |

Las filas 1–26 son M-A (con el E1303 de la 26 en M-B); 27–42 M-B; 43–48 M-C; 49–58 L. Recuentos: E = 48, L = 10. Realistas **R** 7 (33, 34, 36, 37, 38, 48, 49), **P** 26, **F** 25.

## 4. Ejemplos realistas verificados (salida real de `base-release.exe`; esperado en cursiva = oráculo salvo indicación)

Todos rechazados también por el esperado: lo erróneo es el conjunto de diagnósticos. Spans en bytes `[inicio,fin)`.

**X1 (filas 1, 9)** — `fn Main(){var rows=[];var row=[];let ready=true;let label=(if ready then ready else rows);let table=[rows,[row]];rows=[1,2];row=[3];Print(table);}`
`main`: E0103[19,21) E0103[30,32) E0102[107,110). *Esperado*: E1303[84,88) `Bool`/`Array<Array<I64>>` y dos E0102 (`1`,`2`... del literal), sin E0103. Con `row=[3];rows=[[1,2]];` en el otro orden `main` da E1303[84,88) correcto: dependencia del orden. **Rechazo correcto, códigos erróneos (E0103 ×2 espurios, E1303 perdido).**
**X2 (fila 2)** — `…var rows=[];var row=[];let nums=[1,2];let ok=true;AssertEq(rows[1],row);Print((if ok then rows else nums));rows=[3];row=[3];}`: `main` E0103[19,21) E0103[30,32) E0102[77,80). Esperado (ADJ): sin E0103; el oráculo no modela `AssertEq` (no sirve de contraste).
**X3 (fila 4)** — `AssertEq((if ok then rows[0] else row),(if ok then rows else nums));rows=[3];row=[3];`: `main` E0103 ×2 + E1303[94,97). Esperado (ADJ): sin E0103.
**X4 (filas 6, 8, 13)** — F07: `var r=[];var s=[];let ok=true;let a=(if ok then (if ok then s else 1) else r[1]);r=[3];s=[3];` → `main` E0103[16,18) E0103[25,27) E1303[77,78) E1303[85,89); *E1303[77,78) y E0102[94,95)*. F10: `…let t=(if ok then q[k] else (if ok then [r] else r));r=[3];q=[3];` → E0103[25,27) E0103[88,91) E1303[97,98); *E1303[97,98) y E0102[110,111)*. F15: `let a=(if ok then s else k);let b=(if ok then s else r[k]);r=[true];s=[[3]];` → E0103 ×2 + E1303[73,74) + E1303[101,105); *E1303[73,74) y E0102[110,114)*.
**X6 (filas 18, 20, 21, 24)** — O07: `fn Main(){var acc=[];let grid=[[1]];let a=1;let pick=(if a<1 then acc else (if a<2 then grid else [acc]));acc=[3];Print(pick);}`: `main` E0103[18,20) E1303[88,92); *E1303[98,103) y E0102[111,112)*. Con `acc=[[3]]` `main` acierta (control). O01 (`acc[0]`/`[[2]]`, `acc=grid`): E0103 + E1303[97,102) frente a *E1303[88,91)+E0102[109,113)*. O04: `let s=(nums + (if a<1 then q[0] else (if a<2 then [2] else q)));q=[3];` → E0103[31,33) E0102[49,53) E1303[92,95); *solo E0102[49,53)*.
**X7 (fila 26)** — `fn Main(){let nums=[1,2];var r=[];let a=1;let v=(if a<1 then r[0] else (if a<2 then r else nums));r=[3];Print(v);}`: E0103[31,33) E1303[91,95) (`nums`); *E1303[84,85) (`r`) y E0102[101,102)*.
**X8 (filas 27–38, 43)** — cadenas else-if con un acumulador sin anotar. Prefijo `fn Main(){let ok=true;let nums=[1,2];let grid=[[1]];`.
- P03: `var acc=[];let sel=(if ok then acc else (if ok then nums else 1));Print(sel);}` → E0103[60,62) E1303[114,115); *solo E1303[114,115)*. P17 (`… nums else grid`) → E0103[60,62) E1303[114,118); *solo E1303*. P15 (`if ok then nums else (if ok then grid else acc)`) → E0103[60,62) E1303[105,109); *solo E1303*.
- P04 (`… then 1 else nums`): `main` E0103[60,62) E1303[111,115) (`nums`); *E1303[104,105)* (`1`): primario en la hoja equivocada. P16 (P15 + `acc=[true];`) → solo E1303[105,109); *además E0102[126,130)*.
- R4: `var buf=[];var spare=[];let sel=(if ok then buf else (if ok then 1 else spare));buf=[[nums]];` → E0103[73,75) E1303[117,118); *solo E1303*. X_R1na (`cube=[[[1]]]`, `if ok then buf else (if ok then 1 else cube)`) → E0103 + E1303[98,102); *E1303[91,92)*. X_flat3 (`… else (if ok then 1 else (if ok then 2 else 3)))`, `buf=[[nums]]`) → 2 E1303 [104,105) [123,124); *1*. Y_pend_if (`… 1 else true`, sin asignación) → E0103[60,62)+E1303[111,115); *solo E0103*. X_R1inc (`buf=[3]`) → solo E1303[91,92) con `Array<I64>`; *E1303 con `Array³` y E0102[110,111)*.
**X9 (filas 39–42)** — `fn Main(){let ok=true;var found=[];var seen=[];let flag=(if ok then true else seen);Print([found,seen]);seen=[[ok]];found=[];}`: E0103[32,34) E1303[78,82); *solo E1303[78,82)*. Variante P1 (`==nums`, `seen=nums` al final): igual más E0102[153,157) correcto.
**X10 (fila 46)** — `fn Main(){let ok=true;let nums=[1];var q=[];var r=[];let a=[q,(if ok then r[0] else [r])];q=[[nums]];r=[[3]];}`: mismos códigos y primarios que el oráculo, pero el segundo E0102 `[105,106)` tiene `expected_type` = `q` `[60,61)`; *`[92,100)` (`q=[[nums]]`)*. Solo relación.
**X11 (fila 48)** — `fn Main(){let ok=true;var out=[];let sel=(if ok then out else (if ok then [true] else [1]));out=[1];Print(sel);}`: E1303[74,80) (`[true]`); *E1303[86,89) (`[1]`) y E0102[97,98)*.
**X12 (fila 49)** — perfil `f0-0005-b-v1`: `fn Main(){let a=true;let b=false;let n=1+(a or b);}` y `let total=2*(ok and ok);` → `outcome=driver_error` (internal_error, `InvalidIr`). Sin paréntesis (`1+a or b`) da NX-E0102[41,42) `I64`/`Bool`. *Esperado*: NX-E0102/type_mismatch en el operando.
**X13 (fila 50)** — `run`, `f0-0003-b-v1`: `fn Main(){let flag=true;{}{}{}while (flag){}}` → R0001[36,42) (con paréntesis); sin grupo (`while flag{}`) [36,40); sin los `{}` previos el primario ya sale sin paréntesis. El relleno `{}` solo rota qué tick agota la cuota; el paréntesis de la condición sí es natural.
**X14 (filas 56, 57)** — `check`, `f0-0003-b-v1`: `let a=Some(1);match true{true if (if true then {(Drop)(a);value:true;} else {value:false;}) =>{},_=>{}}` → `checked`; con `Drop(a)` sin grupo da NX-E1305/guard_transfer[58,62). Fila 57: la closure `fn ()->Bool{MockUnknown();return true;}` en la guarda, tanto agrupada como con nombre (`g()`), se acepta en `f0-0003-b-v1` y en `f0-0005-b-v1`; el control con nombre debería rechazarse: **esperado NO VERIFICADO** (el efecto `MockUnknown` puede no contar en estos perfiles). Fila 58: `(grid)=[[1],[true]]` con `var grid=[[1],[2]]` no reproduce (1 E0102 igual que sin grupo); solo con `var q1=[]` pendiente.

### 4.1 Cuántos de los 23 programas del twin son realistas

Los 23 con E0103 espurio son F01, F02, F04, F05, F06, F09, F10, F11, F12, F13, F14, F16, F17, F18, F19 y O01–O08 (F11 solo en el mínimo; F07 y F15 se añaden como mínimos con su E0101 original; F08 es legítima, F03 no tiene E0103 hoy).
- **Realistas tal como están escritos: 0 de 23.** Los 27 programas del fuzzer inyectan 3–6 errores no relacionados (`true+1`, arrays como condición de `if`, `{return;}`, `[[3]]` forzados).
- **Patrón central reproducible con un programa verosímil: 10 de 23.** F01 y F11 (X1), F02 (X2), F04 (X3), F10 (X4), F06 (X7), O01, O03, O04, O07 (X6). Nota: en O03 y O04 la reproducción es la del patrón (unión anidada con `[acc]` o `+` con una unión), no del programa entero.
- **Fabricados: 13 de 23** (F05, F09, F12, F13, F14, F16, F17, F18, F19, O02, O05, O06, O08); en F12, F14, O06, O08 el análogo realista dio la salida correcta; F05 y O05 no reproducen con oráculo concluyente.

## 5. Estimación de las filas L

| Fila(s) | Sitios | Commits | Corte | Riesgo / interacción |
|---|---|---:|---|---|
| S6-COR-2, filas 50–56 | 7 sitios semánticos, 8 líneas (6 en `src/ir/lower.rs`, 2 en `src/ir/lower_match.rs`): `ExpressionStatement`, `WhileCondition`, `Assignment`, `ValueStatement`, `prepare_expr`, `guard_violations` ×2 | 7 (uno por solución S1–S7) | S6-COR-2 (plan con PASS de B r5 en la rama; sin implementar) | mecanismo único ya existente (`parse::expression_primary_span`/`expression_primary`); el plan declara además la deuda con fecha de `for_iteration` (inalcanzable hoy) |
| Guarda de `match` agrupada (57) | 1 en `src/ir/effects.rs` (el testigo de efecto ~1867 no ve el destino agrupado no nombre) | 1 | corte nuevo tras S6-COR-2, Flujo B | esperado NO VERIFICADO; confirmar primero el esperado |
| Asignación a destino agrupado (58) | 3–6 de los 7 emparejamientos `Statement::Assign{place,..}` en `types.rs` (~449, 1526, 1701, 7049, 9010, 9510, 15272) para desagrupar el destino | 1–2 | corte nuevo tras S6-COR-2 | **DISCUTIBLE como L**: solo ocurre con variable pendiente, es decir, roza M-C (`assignment_type_plan`); si el sitio es compartido con M-C, pasa a E |
| F5-operand-domain (49) | 1 (`LogicalComplete` en `check_against_context_one`, + ayudante `#[inline(never)]` por pila) | 1 | S5a.3 | **DISCUTIBLE como L**: el intento `ac91e09` se revirtió por regresión en `index_selector` y hay límite de pila (6.784 B); plan en `I07c-S5-plan.md` §6 |
| **Total** | **~12–15 sitios** | **~10–11** | | |

## 6. Reglas nuevas (del orquestador)

1. **Ninguna fila nueva en `Pending.md` sin un CORTE con dueño y orden asignados.** La nota «pendiente» queda solo para lo que espera una decisión de diseño.
2. **Los defectos LOCALES que aparezcan en un corte se corrigen en ese corte** (o en el inmediato con commit propio); no se anotan.
3. **Los ESTRUCTURALES se resuelven bien.** El rediseño del ancla de unión (S5c-d2) es lo primero con Codex el 2026-11-24, antes de B2b..B6 de F5 y de F0-0004. Mientras tanto una exploración (agente X) busca un candidato con 0 regresiones frente a `main` y V6 (criterio de éxito en el registro de defectos §3, punto 2: ninguna fila donde `main` coincida con el oráculo y el candidato no).

## 7. Sesgos y límites

- **Plausibilidad**: juzgué «realista» por criterio propio: un programa corto con nombres de uso normal, una cadena else-if o una lista pendiente rellenada luego (`var acc=[]`), y no más de 1–2 errores de tipos. Descarté arrays como condición, `true+1`, bloques `{return;}`, índices por arrays y `r=[r]`. Un lector que tolere menos errores por programa clasificaría menos **P** como realistas; uno que acepte código ofuscado, más.
- **R solo en 7 filas** y todas con una mejor cobertura de patrón que de programa: la cadena `if a then acc else (if b then nums else 1)` es natural, pero el usuario real suele anotar `var acc:Array<I64>=[]` (el corpus `corpus/objetivo/` lo hace siempre), lo que evita el patrón. No medí frecuencia de uso.
- **El oráculo no es el ADR**: es independiente y B lo validó, pero solo modela un subconjunto. Donde difiere de `main` en la elección de hoja (P04, readplan, O07, F06) cito su esperado; la regla de «primera hoja incompatible» y la relación `expected_type` vienen de la adjudicación del carril. Los esperados de F02, F04, F16 y F18 son ADJ (oráculo sin `AssertEq`).
- **Clasificación L/E**: el eje es estructural y lo apoyo en los informes citados, no en un prototipo mío. **DISCUTIBLE**: (a) filas 57 y 58 y F5 (49) como L: ver §5; (b) W0 (39) y P4pos (47): «causa NO MEDIDA» en el registro, asigno W0 a M-B por herencia en P1/c3/w1 y P4pos a M-C por semejanza; (c) F06 y P2 en dos mecanismos; (d) O04/O05/O06 dependen además de S5a.2, que es acotado pero con coste de pila; (e) no medí si M-A queda cerrado en V6 con S5a.5, solo cito `limites-conocidos.md`.
- **No verificado**: fila 54 (conteo de ticks: no se midió; el registro dice 43.477 frente a 45.453), c3 y w1 como programas realistas aparte, el esperado de la fila 57, el efecto de `MockUnknown` en guardas.
- **Ejemplos**: P (X1–X13) usan nombres y estructura de uso normal pero son pequeños; los 7 R no son programas completos de aplicación. Las cifras de span son de mis fuentes, no de los repros mínimos del registro. Sin compilación ni procesos en segundo plano; temporales en `.scratch/a-impacto/` (borrado al terminar).
