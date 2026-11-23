# I07c-S5c — Diseño del ancla de unión bajo retracción (propuesta A, r2; sin código)

Rama `i07c-estructural`. `main` = `2875943` (`base-release.exe`), V6 = `2296dc8` (`cand-release.exe`), C2 = V6 + `docs/validation/i07c-s5a/r1-variante-C2.diff` (compilado en copia desechable, borrada). Perfil `f0-0002-b-v2`, `check`. Esperados derivados del ADR (`F0-0002-second-slice.md` §4.1.2 cláusulas 1–7, §4.7; `F0-0002-lagunas.md` «Ancla con componente pendiente…», D-REF) antes de ver la salida y comparados por script. N = aceptación, código, subcausa, primario, cardinalidad (cláusulas 2 y 4); I = relación `expected_type`, datos (cláusula 2, 6). Esta r2 atiende los seis motivos de `I07c-S5c-diseno-B-r1.md`.

## 1. Hallazgo central (confirmado por B)

El defecto que V6 destapa ya existe en `main` sin siembra. Prefijo común `fn Main(){let p=true;let ar=[1];`:

| Programa | Esperado (ADR) | main = V6 = C2 |
|---|---|---|
| `X_R1na`: `let r=[[[1]]];var q=[];let a0=(if p then q else (if p then 1 else r));` | E1303 `[91,92)` (`1`), `A³`/`I64`, rel. `q` `[73,74)`; sin E0103 (`r` compatible fija `α`) | E0103 `[52,54)` + E1303 `[98,99)` con el `1` anidado como referencia |
| `X_R1ord` (`then r else 1`) | E1303 `[98,99)`, rel. `q` `[73,74)`; sin E0103 | E0103 `[52,54)`; rel. `r` `[91,92)` |
| `Y_pend_if`: `var r=[];let a0=(if p then r else (if p then 1 else true));` | solo E0103 `[38,40)` | E0103 `[38,40)` + E1303 `[84,88)` (`true`) |
| `X_flat3`: `var q=[];(if p then q else (if p then 1 else (if p then 2 else 3)));q=[[ar]];` | 1 E1303 en el `1` `[77,78)` | main 2, V6 3, C2 2 |
| `X_mix3`: `var q=[];var r=[];(if p then q else (if p then r else (if p then 1 else 2)));q=[[ar]];` | 1 E1303 en el `1` `[104,105)`, rel. `q` `[68,69)` | main 1, V6 2 (+`2` `[111,112)`), C2 1 |
| `R4` (`1` antes de `r`, ambos `var`, `q=[[ar]]`) | E1303 `[94,95)`; sin E0103 de `r` (l.353: «las restricciones compatibles de todos los hijos se conservan siempre») | E0103 `[46,48)` espurio en main y C2; V6 solo E0103 |

Causa (lectura de `if_else_context`, `if_after_first`, `if_finish_join`, `seed_planned_shape`): `if_else_context` **descarta el ancla** del `if` anidado cuando sus ramas discrepan por forma y `resolved_join_side(ancla)` es `None`; la unión anidada sintetiza una segunda referencia y se pierden las igualdades compatibles de sus hojas con la real. `main` lo disimula cuando hay asignación posterior: `seed_planned_shape` inyecta `concrete_plan` (evidencia **adelantada** de la asignación) y el ancla aparece concreta al entrar, la anticipación que l.355 y §4.1.2.7 vedan para decidir disponibilidad. V6 la retira y deja al descubierto el camino de ancla pendiente: R1–R3b son ese camino, no un fallo de la retracción en sí.

Defecto previo adicional, **sin cubrir por ALT-2 salvo que (b) lo arregle**: `W0` (repro de B) `fn Main(){let ar=[1];var r=[];var q=[];let a=(if true then true else q);Print([r,q]);q=[[1<2]];r=[];}` da en los tres binarios E0103 `[27,29)` (el `[]` de `r`) y E1303 `[69,70)` (`exp=[59,63)`). Derivado: `[r,q]` iguala `r` con `q` (l.353, homogeneidad) y `q=[[1<2]]` fija el elemento; el addendum dice que el `[]` «no lo recibe cuando esa igualdad compatible ya lo fijó»; esperado: solo la E1303. Sin la unión (`let a=1;`, `W0nounion`) se acepta. Hipótesis (no medida): `q` queda marcada como fallida por la denuncia y `[r,q]` no aplica la igualdad (misma familia que (b)); si ALT-2 no lo corrige, se registra como defecto de núcleo previo con fecha y no bloquea S5c. P1, w1 y c3 heredan este E0103: **no** son OK en main, V6 ni C2.

## 2. Modelo: qué es el ancla

1. **Identidad (solo orden fuente).** Pareja (tipo, origen) de la primera continuación normal con evidencia válida en el orden de comprobación (l.299, l.353; D-REF: una variable pendiente vale, con incógnitas). Inmutable una vez fijada: ni hojas posteriores, ni uniones anidadas, ni asignaciones posteriores la sustituyen. Un `if` en la primera continuación tiene su unión propia y aporta el testigo parcial (l.353).
2. **Aplanado.** Cada hoja posterior, incluidas las primeras de los `if` anidados, se contrasta con el ancla en orden fuente.
3. **Igualdad por hoja.** Compatible: se acumula y fija el componente (l.301: «la primera igualdad compatible que, junto con las precedentes, determina su tipo»), con procedencia. Incompatible: se rechaza atómicamente (l.299) sin deshacer lo anterior.
4. **Denuncia única.** Solo la primera hoja incompatible puede recibir E1303 (l.353; N). Las posteriores siguen contrastándose para acumular igualdades compatibles; sus incompatibilidades no se denuncian. Si esa denuncia se omite por pendiente (componente que emite E0103 al cierre, l.301), cuenta como ya hecha (l.367: «un conflicto pendiente de serialización cuenta como ya denunciado»): la hoja siguiente no la hereda.
5. **Pendiente.** Con ancla no concreta el conflicto se difiere y se completa al cierre con las igualdades compatibles posteriores, asignaciones incluidas. La disponibilidad al entrar no se rehace desde una asignación posterior (l.355, §4.1.2.7).
6. **Asignación posterior.** Se comprueba contra el tipo ya fijado; si es incompatible emite su E0102 en el RHS con origen en la primera igualdad compatible y no retipa lecturas anteriores (l.301).
7. **Relación `expected_type` (I; regla fijada, l.371).** Para una unión, el operando de la primera continuación que la establece (en `X_nest_first` es el `if` anidado `[60,78)`, ya coincidente en main y V6); para una asignación, su origen concreto, siguiendo aliases hasta el inicializador o la restricción posterior que lo fijó (P1, P2, c3, w1: el RHS `q=[[…]]`). Ninguna de las dos necesita Flujo A.

## 3. Matriz (45 casos derivables; fuentes: `d-repros/`, `limites/`, `casos.json` O01/O03/O07/O08, §7)

`NO(I)` = solo difiere la relación. Cardinalidad entre paréntesis.

| Caso | Esperado | main | V6 | C2 |
|---|---|---|---|---|
| R1 | E1303 `[91,92)` `A³`/`I64` rel. `q` (1) | OK | NO | OK |
| R1twin, R3c | E1303 con `q` ya `[[ar]]` / todo concreto (1) | OK | OK | OK |
| R2 | E1303 `[93,94)` rel. `[68,69)`, sin E0103 (1) | OK | NO (solo E0103) | OK |
| R2b | E1303 `[98,99)` rel. `q` `[73,74)` (1) | OK | NO(I) | OK |
| R3 | E1303 `[99,104)` `I64`/`A⁴` (1) | OK | NO (2) | OK |
| R3b | E1303 `[77,78)` solo en `1` (1) | OK | NO (2) | OK |
| R4, X_R1na, X_R1ord, X_flat3, Y_pend_if, Y_pend_if2 | §1 | NO | NO | NO |
| X_mix3 | 1 E1303 en el `1` | OK | NO (2) | OK |
| X_R4asg_r (R4 + `r=[[ar]]`) | E1303 `[86,87)` rel. `q` (1) | OK | NO | OK |
| X_R1inc (`q=[3]`) | E1303 `[91,92)` `A³`/`I64` + E0102 en el `3` `A²`/`I64` rel. `r` `[98,99)` (2) | NO (retipa `q` a `A¹`, sin E0102) | NO | NO |
| X_R1pre, X_R2na, X_R4na, X_nest_first, Y_conc_if, Y_pend_if_asg, W0nounion | controles (1–2 diag.; ver script) | OK | OK | OK |
| W0 | solo E1303 `[69,70)` (1) | NO (+E0103) | NO | NO |
| A / A reord. | E1303 + E0102 de `r=[3]` (2) | NO | OK | OK |
| A control, A sin asignación | E1303 `[83,86)` `[53,54)` (1) | OK / NO (solo E0103) | OK / OK | OK / NO |
| B q→r / r→q | E1303 `[63,64)` + E0102 en el `3` de `q` (2) | NO / OK | OK / OK | OK / OK |
| F06, M2, O01, O03, O07, O08 | tablas de `I07c-laguna-esperado-diferido-r1.md` (2, 2, 4, 3, 2, 6) | NO | OK | OK |
| M2 control, M2 sin asig., D-REF válido | (1, 1, 0) | OK | OK | OK |
| P1, c3, w1 | sin E0103 (`W0`); E1303 + E0102 de `ar` rel. `q=[[…]]` | NO (E0103; w1 además E0102 `[87,88)`) | NO (E0103; rel. `[85,86)`) | NO (E0103) |
| P2 | única E1303 sobre `r` `[101,102)` `A³`/`A²` rel. `[75,80)`; E0102 de `q=ar` `[127,129)` `A²`/`A¹` rel. `[119,124)`; E0102 de `r=[r]` `[133,134)` `A¹`/`A²` rel. `[119,124)` (3; adjudicado: derivación del oráculo, B `I07c-S5c-oraculo-B-r1.md` §2b) | NO (E1303 en `[true]` `[108,114)`; E0102 `[133,134)` `A²`/`A³`; la relación de `q=ar` `[119,124)` acierta) | NO (lo mismo, y relación de `q=ar` `[76,77)`: regresión propia de V6) | NO (= main) |
| P3 | E0102 `[99,100)` rel. `[88,94)` | NO(I) | OK | NO(I) |
| w3 | única E1303 sobre `r` `[101,102)` `A³`/`A²` rel. `[75,80)`; E0102 `[129,131)` `A²`/`A¹` rel. `[119,126)`; E0102 `[135,136)` `A¹`/`A²` rel. `[119,126)` (adjudicado como P2) | NO (espurios N) | NO (E1303 en `[108,114)`, relación `[76,77)`, E0102 `[135,136)` `A²`/`A³`) | NO (= main) |
| R5, P4, P4pos, generated1411/1681 | NO DERIVABLE (derivación no cerrada; P4pos defecto previo); no entran en el recuento | — | — | — |

Recuento por fila (45): `main` NO en A, A sin asig., A reord., B q→r, F06, M2, O01, O03, O07, O08, P1, P2, P3(I), R4, W0, X_R1inc, X_R1na, X_R1ord, X_flat3, Y_pend_if, Y_pend_if2, c3, w1, w3 = 24, **OK 21** (P2 deja de ser OK tras la adjudicación de B). V6 NO en P1, P2, R1, R2, R2b(I), R3, R3b, R4, W0, X_R1inc, X_R1na, X_R1ord, X_R4asg_r, X_flat3, X_mix3, Y_pend_if, Y_pend_if2, c3, w1, w3 = 20, **OK 25** (P2 y w3 pasan de NO(I) a NO de núcleo). C2 NO en A sin asig., P1, P2, P3(I), R4, W0, X_R1inc, X_R1na, X_R1ord, X_flat3, Y_pend_if, Y_pend_if2, c3, w1, w3 = 15, **OK 30**. Contando `NO(I)` como correctos en N: 22 / 26 / 31. w1 en V6 es defecto N (E0103 espurio), no solo I. Las dos regresiones residuales de C2 (`g02426`, `g03010`) no se reproducen aquí.

## 4. Alternativas

**ALT-1 — No retraer con asignación posterior compatible (C1/C2, medida).** Arregla R1–R3b, X_R4asg_r, X_mix3; pierde A sin asignación; deja los 6 casos de §1, X_R1inc, W0 y conserva `concrete_plan` como soporte del ancla. Descartada.

**ALT-2 — Registro de ancla por unión (recomendada).** Al fijar su referencia, la unión crea en el grafo un registro `{tipo, origen, denunciada}` (clave: ocurrencia de la unión raíz del aplanado; vaciado por pasada). (a) `if_else_context` propaga siempre el ancla a los `if` anidados posteriores: desaparece la rama de descarte. (b) Cada hoja aplica su igualdad compatible exactamente una vez, aunque la unión ya haya denunciado. (c) La primera incompatible marca `denunciada` (inmediata, diferida u omitida por pendiente); las siguientes no registran `PendingDeferredJoin` ni E1303. (d) La retracción de V6 (cambios 1–3) se aplica después, en un segundo bloque (§5).

*Terminación (corregida).* El registro vive solo dentro de la comprobación de una unión, pero **sí hay realimentación**: (c) cambia cuándo se crea un `PendingDeferredJoin`; `note_assignment_verdict` (V6 l.8603–8622) inserta el sitio en `rejected_assignments` cuando `pending_join_count()` crece durante una asignación (uniones en su RHS) y `read_plan` lo lee (V6 l.3258); además (b) añade igualdades, lo que cambia las componentes del union-find que decide qué capa atestigua `retract_unconsumed_seeds`. Camino: unión → igualdades/`PendingDeferredJoin` → veredicto por binding y testigos → `seed_cap` → siguiente pasada de planning. `seed_cap` solo decrece, `rejected_assignments` y el registro se vacían por pasada y el bucle tiene tope 129: **para siempre**, pero el tope no garantiza que el resultado sea correcto ni que `changed`/`retracted` converjan pronto. Se exige: contador de pasadas por función en copia instrumentada sobre toda la matriz, los 3.674 y el barrido (V6 medía ≤ 3), y que ningún programa alcance 129.

*Independencia de orden (corregida).* La identidad del ancla y la denuncia dependen solo del orden fuente interno de la unión. Las asignaciones influyen por el camino anterior, así que no se afirma independencia: se exige el **barrido de permutaciones de orden de asignaciones** (todas las de 2 y 3 asignaciones, compatibles e incompatibles, antes y después de la unión; V6 tenía 49 pares) con salida idéntica salvo las diferencias que el ADR deriva del orden de hojas.

*Efecto previsto (razonado, no ejecutado).* Sobre `main`: R4, X_R1na, X_R1ord, X_flat3, Y_pend_if/2 pasan a OK sin perder ningún OK de `main`; X_R1inc previsto OK (hipótesis: el planning acepta `q=[3]` solo porque la hoja `r` compatible se perdió) y W0 queda abierto; P1, P2, P3, c3, w1, w3 no cambian por ALT-2.

**ALT-3 — Separar siembra de resolución** (`resolved_join_side` sobre una vista sin evidencia sembrada). Equivale a V6 en R1–R3 y no arregla los casos sin siembra. Necesaria como higiene, insuficiente.

**ALT-4 — Procedencia por registro append-only de igualdades** (relación §2.7). Arregla relaciones (P1, P2, c3, P3, w1, w3 en I); no toca R1–R4 ni X_*. Es S5a.5; puede ir después. (Reconstruir la unión tras retraer se descarta sin variante: X_R1na falla sin siembra.)

## 5. Plan por bloques, riesgos y verificación

- **B1 — ALT-2 sobre `main` sin V6.** Debe conservar los 21 OK de `main` y arreglar R4, X_R1na, X_R1ord, X_flat3, Y_pend_if, Y_pend_if2 (X_mix3 ya es OK en main y debe seguirlo); prueba de independencia de la siembra.
- **B2 — retracción de V6 encima de B1.** Debe dejar OK R1–R3b, X_R4asg_r, X_mix3 y A…O08 como V6, sin las regresiones de §3.
- Riesgos: (1) la cláusula de descarte pudo existir por un caso protegido: las 152 filas RD y los 13 RD con `if` y `[]` no la ejercen (inspeccionado); TD/DG, `tests/` y las 27 fuentes twin sí podrían; se exige la puerta completa. (2) `Y_pend_if*` quitan una E1303 de `main` por l.353 + l.367, no por azar; cada diferencia queda listada y derivada. (3) W0 puede requerir un camino distinto (elemento de array con variable ya denunciada); si es así, se separa.

Verificación (solo la puerta completa fundamenta PASS):
1. Diferencial contra `main` **y** V6 con **todas** las diferencias, en ambos órdenes de asignación, 3 perfiles, debug/release, `check`/`run`: familia de `familia-oraculo.py` ampliada a profundidad 3, hojas `{q, r, 1, 2, ar, aar, r3, [q,r], r[0]}`, `var []` pendiente y concreto, asignación {ninguna, compatible, incompatible, antes de la unión}; 0 sin clasificar, sin muestreo (C2 falló a 1/4000).
2. Oráculo independiente (sin importar el verificador) que modele pendiente, omisión por E0103, resolución por asignación posterior y aplanado; se valida **antes** contra esta matriz.
3. Detectores en todo barrido: E0103 con primario no constructor; >1 E1303 con la misma relación en una unión; E1303 perdida con más E0103; cambio de aceptación.
4. Corpus: 3.674 fuentes de C, 27 twin, 152 RD, fuzz 4000 (semilla 7391045882, SHA 60cf15df…), pasadas de planning y permutaciones (§4).
5. Regresiones negativas aisladas: R1, R2, R2b, R3, R3b, R4, X_R1na, X_R1ord, Y_pend_if, X_flat3, X_mix3, X_R1inc; positivos R3c y X_R1pre; esperados transcritos de §3.
6. Mutación inversa por regla (restauración SHA-256): reintroducir el descarte del ancla; `denunciada` sin marcar; igualdad omitida tras la denuncia; referencia = última hoja; cada una debe morir por un test nombrado.

## 6. Decisiones del ADR

- **Derivada, no laguna (L1, antes mal encuadrada):** l.353 («una sola NX-E1303 sobre la primera continuación posterior incompatible… las demás… no reciben NX-E1303 propia») + l.367 (el conflicto pendiente cuenta como ya denunciado) + l.301 (omisión por E0103). Para `F0-0002-lagunas.md`. Inválido: `fn Main(){let p=true;let ar=[1];var r=[];let a0=(if p then r else (if p then 1 else true));}` (`check`/`run`: rechazado; única `NX-E0103` `[38,40)`, sin E1303 en `1` ni en `true`). Válido: `…var r=[];let a0=(if p then r else (if p then r else r));r=[2];}` (aceptado, `checked`, sin diagnósticos). Contraste: con `r=[2];` y sin `let ar=[1];` (`Y_pend_if_asg`) el componente queda resuelto y hay una sola E1303 `[66,67)` en `1`.
- **Regla de relación fijada (l.371):** §2.7. L2 (`X_nest_first`) y L4 (P1/P2) dejan de ser lagunas; el diferencial no admite diferencias de relación sin clasificar.
- NO DERIVABLE por ahora, sin probar que sea laguna: R5, P4, P4pos, generated1411/1681 (derivación no cerrada). Pasan a Flujo A solo si tras B1/B2 persisten y B no las cierra.

## 7. Pila de `Checker::drive`

Techo, no previsión: 66.560 − 60.616 = 5.944 B de reserva menos el margen obligatorio de 1.000 B = 4.944 B de crecimiento máximo permitido de la pareja `drive`+`resume_non_match`. `if_finish_join`, `if_advance`, `if_after_first` e `if_else_context` no son hoy `#[inline(never)]`, y `if_advance` se llama desde el conductor (V6 l.10600–10631): inlinear código nuevo ahí puede inflar `drive`. Reglas: las funciones de unión que se toquen (`if_finish_join`, `if_after_first`, `if_else_context`) pasan a `#[inline(never)]`; **no se toca `if_advance`**; el registro vive en `graph` y su acceso en dos ayudantes `#[inline(never)]` (abrir/consumir) con `FactId` y progreso explícitos. Previsión: la rama de descarte que se elimina reduce código; lo añadido son un `HashMap` y un booleano fuera de marco; objetivo ≤ 1.500 B de crecimiento de la pareja; **si pasa de 2.500 B se detiene el bloque y se vuelve al diseño**. Medir `scripts/f5-frame-sizes.py` (rustc 1.99.1, perfil de test) en cada commit y `scripts/f5-stack65.py` (parte de `validate.sh`).

## 8. Binarios y limpieza

`.scratch/cand-bin/base-release.exe` (`main`), `cand-release.exe` (V6) y C2 compilado en copia (release, 27 s, bajo `scripts/heavy-lock.py`; copia y `target` borrados). Mutantes `mut-*.exe` no usados. Fuentes nuevas: las `X_*`/`Y_*`/`W0` de §1 con el prefijo indicado (más `A_*`, `B_*`, `M2_*` de `I07c-S5-plan.md` §7 y `I07c-laguna-esperado-diferido-r1.md`).
