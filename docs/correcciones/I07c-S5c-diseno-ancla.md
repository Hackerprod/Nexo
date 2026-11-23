# I07c-S5c — Diseño del ancla de unión bajo retracción (propuesta A, paso 1 de Flujo A/B; sin código)

Rama `i07c-estructural`. Referencias: `main` = `2875943` (`base-release.exe`), V6 = `2296dc8` (`cand-release.exe`), C2 = V6 + `docs/validation/i07c-s5a/r1-variante-C2.diff` compilado en copia desechable (release, `rustc` 1.99.1, perfil `f0-0002-b-v2`, `check`). Todo esperado de la matriz está derivado del ADR (`F0-0002-second-slice.md` §4.1.2 cláusulas 1–7, §4.7; `F0-0002-lagunas.md` «Ancla con componente pendiente y asignación posterior», D-REF) y comparado después con los tres binarios por script (esperado escrito antes de ver la salida). Nivel: unión, denuncia única, aceptación y primario son N (cláusulas 2 y 4); la relación `expected_type` es I (cláusula 2, 6).

## 1. Hallazgo central (medido)

El defecto que V6 destapa **ya existe en `main` sin siembra**. Con la asignación posterior suprimida, `main`, V6 y C2 dan lo mismo y los tres violan el ADR:

| Programa (sin asignación posterior) | Esperado (ADR) | main = V6 = C2 |
|---|---|---|
| `X_R1na`: `let r=[[[1]]];var q=[];let a0=(if p then q else (if p then 1 else r));` | E1303 `[91,92)` (`1`), `A³`/`I64`, rel. `q` `[73,74)`; sin E0103 (`r` compatible fija `α`) | E0103 `[52,54)` + E1303 `[98,99)` (`r`), `I64`/`A³`, rel. `1`: el `1` anidado actúa como referencia |
| `X_R1ord` (`then r else 1`) | E1303 `[98,99)` rel. `q`, sin E0103 | E0103 `[52,54)`; rel. `r` `[91,92)` |
| `Y_pend_if`: `var r=[];(if p then r else (if p then 1 else true))` | solo E0103 `[27,29)` (el conflicto de `1` queda pendiente y se omite por E0103, l.301; `true` no recibe E1303 propia, l.353) | E0103 + E1303 `[73,77)` en `true` con referencia `1` |

Causa en el código (lectura de `if_else_context`, `if_after_first`, `if_finish_join`, `seed_planned_shape`): `if_else_context` **descarta el ancla** del `if` anidado cuando sus dos ramas discrepan por forma **y** `resolved_join_side(ancla)` es `None` (ancla no totalmente concreta). Entonces la unión anidada sintetiza su propia referencia (segunda referencia), y las igualdades compatibles de sus hojas con la referencia real se pierden. En `main` casi nunca se llega ahí cuando hay asignación posterior porque `seed_planned_shape` inyecta `concrete_plan` (evidencia **adelantada** de la asignación) y el ancla aparece concreta al entrar: es la anticipación que el addendum y §4.1.2.7 prohíben para decidir disponibilidad. V6 retira esa anticipación (cambios 1 y 3) y por tanto deja al descubierto el camino de ancla pendiente: R1, R2, R3 son ese camino, no un fallo de la retracción en sí. Corolario: la retracción hace que el ancla refleje el estado real (pendiente); lo incorrecto es lo que la unión hace con un ancla pendiente.

Segundo hallazgo: con ancla pendiente `main` ya viola «denuncia única» en profundidad 3 (`X_flat3`: `if p then q else (if p then 1 else (if p then 2 else 3))` da 2 E1303; V6 da 3; C2 2; ADR: 1 en el `1`) y pierde igualdades compatibles de una hoja posterior a una incompatible (`R4`: E0103 espurio en el `[]` de `r`; l.353 «las restricciones compatibles de todos los hijos se conservan siempre»).

## 2. Modelo: qué es el ancla

1. **Identidad (solo orden fuente).** El ancla de una unión sin contexto externo determinado es la pareja (tipo, origen) de su **primera continuación normal con evidencia válida** en el orden de comprobación (l.299, l.353; D-REF: una variable pendiente `r[0]` o `r` vale; el tipo puede contener incógnitas). Es **inmutable** una vez fijada: ni una hoja posterior, ni una unión anidada, ni una asignación posterior la sustituyen (l.353: «no se sintetiza otra referencia incompatible dentro de ellas»; «sin cambiar esa referencia»). Un `if` en la primera continuación tiene su propia unión y aporta el testigo parcial (l.353, final).
2. **Aplanado.** Cada hoja posterior, incluidas las primeras de los `if` anidados, se contrasta con el ancla **en orden fuente**, siempre contra la misma referencia.
3. **Igualdad por hoja.** Compatible: la igualdad se acumula y fija el componente «la primera igualdad compatible que, junto con las precedentes, determina su tipo» (l.301); se conserva su procedencia. Incompatible: se rechaza atómicamente (l.299) sin deshacer las anteriores.
4. **Denuncia única.** La primera hoja incompatible es la **única** que puede recibir E1303 (l.353; N). Las posteriores se siguen contrastando solo para acumular igualdades compatibles; sus incompatibilidades no se denuncian.
5. **Estado «pendiente».** Si en la entrada el ancla no es concreta, el conflicto se **difiere** y se completa al cierre con las igualdades compatibles posteriores, asignaciones incluidas; se omite solo si el componente pendiente emite E0103 (l.301). La decisión de disponibilidad en la entrada no se rehace desde una asignación posterior (l.355, §4.1.2.7: «no existe una regla de sustituir retrospectivamente…»).
6. **Asignación posterior.** Se comprueba contra el tipo ya fijado; si es incompatible emite su propio E0102 en su RHS con origen en la primera igualdad compatible (addendum) y **no** retipa lecturas anteriores (l.301).

Retracción de V6 (cambio 1): decide qué capas **sembradas** sobreviven; no puede formar parte del modelo del ancla. El requisito de diseño es que (1)–(6) no dependan de si hay siembra.

## 3. Matriz (esperado derivado del ADR; OK/NO por binario)

`NO(I)` = solo la relación `expected_type` difiere (I). Cardinalidad = nº de diagnósticos esperados. `A³` = `Array<Array<Array<I64>>>`. Fuente literal en `docs/validation/i07c-s5a/{d-repros,limites}/*.n` y `.scratch/a-s5c/m/*.n` (regenerable; no versionado), twin en `docs/validation/i07c-twin/casos.json` (O01, O03, O07, O08) y plan S5 §7.

| Caso | Esperado (código @primario, tipos exp/hallado, rel. `expected_type`; card.) | main | V6 | C2 |
|---|---|---|---|---|
| R1 | E1303 `[91,92)` `A³`/`I64`, `q` `[73,74)`; 1 | OK | NO (E1303 `[98,99)`, ref. `1`) | OK |
| R1twin (q ya `[[ar]]`) | E1303 `[95,96)` `A³`/`I64`, `[77,78)`; 1 | OK | OK | OK |
| R2 | E1303 `[93,94)` `A³`/`I64`, `q` `[68,69)`; sin E0103; 1 | OK | NO (solo E0103 `[38,40)`) | OK |
| R2b (`r` concreto) | E1303 `[98,99)`, rel. `q` `[73,74)`; 1 | OK | NO (rel. `r`) | OK |
| R3 | E1303 `[99,104)` (`[q,r]`) `I64`/`A⁴`, `[81,82)`; 1 | OK | NO (2: +`[110,112)`) | OK |
| R3b (`then 1 else 2`) | E1303 `[77,78)` solo en `1`; 1 | OK | NO (2) | OK |
| R3c (todo concreto) | E1303 `[82,83)`; 1 | OK | OK | OK |
| R4 (`1` antes de `r`) | E1303 `[94,95)`, `q` `[76,77)`; sin E0103 (`r` compatible); 1 | NO (E0103 `[46,48)` espurio) | NO (solo E0103) | NO (= main) |
| R5 | NO DERIVABLE: uniones anidadas con tres referencias; D lo derivó con confianza media. Obtenido: main 1 E1303 en `1`; V6=C2 2 E1303 (`[[r]]`, `1`) | — | — | — |
| X_R1na / X_R1ord / Y_pend_if / Y_pend_if2 / X_flat3 | ver §1 | NO | NO | NO |
| X_R1inc (`q=[3]` incompatible) | E1303 `[91,92)` `A³`/`I64` rel. `q` + E0102 en el `3` `A²`/`I64`, rel. `r` `[98,99)`; 2 | NO (E1303 con `A¹`; sin E0102: asignación rechazada retipa `q`) | NO | NO |
| X_R1pre, X_R2na, X_R4na, Y_conc_if, Y_pend_if_asg | controles (ver script): E1303/E0103 coherentes con §2 | OK | OK | OK |
| X_R4asg_r (`r=[[ar]]` además) | E1303 `[86,87)`, `q` `[68,69)`; 1 | OK | NO (ref. `r`) | OK |
| A (96 B) | E1303 `[83,86)` `A²`/`A³` `[53,54)`; E0102 `[92,93)` `A¹`/`I64` `[74,77)`; 2 | NO | OK | OK |
| A control `r=[[3]]` | solo la E1303; 1 | OK | OK | OK |
| A sin asignación | E1303 `[83,86)` `[53,54)`; 1 | NO (solo E0103 `[30,32)`) | OK | **NO** |
| A reordenado (`[r]` antes de `aar`) | E1303 `[74,77)`; E0102 `[92,93)` rel. `[83,86)`; 2 | NO | OK | OK |
| B q→r / r→q | E1303 `[63,64)` `Bool`/`A²` `[56,57)`; E0102 en el `3` de `q`; 2 | NO / OK | OK / OK | OK / OK |
| F06 mín. | E1303 `[76,77)` `A¹`/`A²` `[51,55)`; E0102 `[91,92)` `[83,85)`; 2 | NO | OK | OK |
| M2, D-REF inválido | E1303 `[77,78)` `A²`/`A³` `[53,57)`; E0102 `[92,95)` `[84,87)`; 2 | NO | OK | OK |
| M2 control / sin asignación / D-REF válido | 1 / 1 / 0 | OK | OK | OK |
| O01, O03, O07, O08 | tablas de `I07c-laguna-esperado-diferido-r1.md` (6, 3, 2, 6 diagnósticos) | NO | OK | OK |
| P1, P2, c3 | N igual a main; rel. del E0102 de `ar` = asignación que fija el componente (`[93,101)`, `[119,124)`, `[93,99)`) | OK | NO(I) | OK |
| P3 | E0102 `[99,100)` rel. `[88,94)` | NO(I) | OK | NO(I) |
| w1 | sin E0102 `[87,88)` (asignación rechazada no retipa); rel. `[93,100)` | NO (espurio, N) | NO(I) | NO (= main, N) |
| w3 | E1303 `[108,114)`, E0102 `[129,131)` rel. `[119,126)`, E0102 `[135,136)`; sin E0103 | NO (3 espurios, N) | NO(I) | NO (= main, N) |
| P4, P4pos, generated1411/1681 | NO DERIVADO (B: sin cierre; P4pos defecto previo) | — | — | — |

Totales sobre los 44 casos derivables ejecutados: main 22 OK, V6 26, C2 33. Los 11 casos en que C2 falla son R4, X_R1na, X_R1ord, X_R1inc, X_flat3, Y_pend_if, Y_pend_if2, A sin asignación, P3 (I), w1 y w3. Las dos regresiones de C2 que D halló (E0103 con primario en un nombre, `g02426`; segunda E1303 en unión plana, `g03010`) no se reproducen en esta matriz: se rastrean con el barrido de §5 (aparecen 1/4000).

## 4. Alternativas

Notación: «terminación» = el bucle de planning (≤ 129 pasadas, `seed_cap` solo decrece) no cambia; «orden» = independencia respecto al orden de las asignaciones.

**ALT-1 — No retraer capas con asignación posterior compatible (C1/C2, medida).** Terminación: igual que V6 (menos retracciones). Orden: depende del veredicto por binding `rejected_assignments` (empírica). Efecto medido en la matriz: arregla R1–R3b y X_R4asg_r (vuelve a `main`), pierde A sin asignación, P3, w1, w3, deja R4, X_*, Y_*, `X_flat3`, y D halla 2 regresiones residuales. Fundamento débil: conserva la anticipación (`concrete_plan`) como lo que sostiene al ancla; cada caso sin siembra sigue roto. Descartada.

**ALT-2 — Registro de ancla por unión, independiente de la siembra (recomendada).** Mecanismo único: una unión crea al fijar su referencia un registro en el grafo (clave: ocurrencia de la unión raíz del aplanado; vaciado por pasada junto a `rejected_assignments`) con `{tipo, origen, denunciada: bool}`. (a) `if_else_context` propaga **siempre** el ancla a los `if` anidados posteriores: se elimina la rama «descartar si forma en conflicto y ancla no resuelta». (b) Cada hoja aplanada aplica su igualdad con el ancla **aunque la unión ya tenga denuncia** (invariante: igualdad compatible aplicada exactamente una vez; arregla R4 y la pérdida de R2). (c) La primera incompatible marca `denunciada` (inmediata o diferida); las siguientes no registran `PendingDeferredJoin` ni E1303. (d) La retracción de V6 (cambios 1–3 íntegros, sin C2) se conserva: ya no afecta a la unión porque (a)–(c) no consultan `resolved_join_side` del ancla ni evidencia sembrada.
- Terminación: el registro se escribe y consume dentro de una comprobación de la unión (una visita por hoja); no es leído por `collect_planned_shapes`/`retract_unconsumed_seeds`; no hay realimentación al planning. Orden: la identidad del ancla y la denuncia dependen solo del orden fuente interno; las asignaciones solo completan tipos al cierre y a través de grafo (commutativo). Es más fuerte que V6/C2.
- Efecto previsto (razonado; **no ejecutado**: no se ha escrito código): R1–R3c, R4, X_R1na, X_R1ord, X_R4asg_r, Y_pend_if/2, X_flat3 pasan a OK; A, A sin asignación, A reord., B, F06, M2, O01–O08 conservan lo de V6; P1, P2, P3, c3, w1, w3 quedan como V6 (relación I y E0102 espurios de main retirados). X_R1inc: previsto OK (hipótesis: planning acepta `q=[3]` porque la hoja `r` compatible se perdió; con (b) el planning la rechaza y el sitio entra en `rejected_assignments`); verificar.

**ALT-3 — Separar siembra de resolución.** `resolved_join_side` y la disponibilidad al entrar se calculan sobre una vista sin evidencia sembrada (marca `from_plan` en nodos). Terminación y orden: neutros. Efecto: equivale a V6 en R1–R3 (ya parte de un ancla no concreta); **no arregla** X_R1na ni Y_pend_if, que no tienen siembra. Necesaria como higiene (coherencia con l.355), insuficiente como mecanismo. Descartada como mecanismo único; ALT-2 la hace redundante en la unión.

**ALT-4 — Ancla y relación por procedencia (orden de creación de igualdades).** Un registro append-only de igualdades compatibles con índice de creación; el ancla y la relación `expected_type` se leen de la primera que fija el componente (addendum), no de la hoja/capa sembrada. Terminación: lista acotada por las igualdades de la pasada. Orden: el índice sigue el orden fuente. Efecto: arregla P1, P2, c3, w1, w3 (relación, I) y P3 sin tocar la unión; no arregla R1–R4 ni X_*. Es el límite de procedencia (S5a.5) y puede ir después de ALT-2 sin interferir. Descartada como solución del ancla.

(Reconstruir la unión tras retraer —una pasada final adicional— no ataca la causa: X_R1na falla sin siembra; se descarta sin variante.)

## 5. Recomendación, riesgos y verificación

**ALT-2**, aplicada sobre V6 completa (no sobre C2), como un único cambio en el camino de unión de `Checker` (`if_else_context`, `if_after_first`, `if_finish_join`).

Riesgos: (1) la cláusula de descarte pudo existir por un caso protegido: ninguna de las 152 filas RD ni los 13 RD con `if` y `[]` la ejercen (inspeccionado), pero TD/DG, `tests/` y las 27 fuentes twin pueden hacerlo; se exige correr toda la puerta antes de aceptar. (2) Cambia salidas de `main` en uniones con ancla pendiente (`Y_pend_if*` quitan una E1303 de `main`): son consecuencia de l.301/l.353, no del azar; cada una debe quedar listada y derivada. (3) `X_nest_first` (`(if p then (if p then q else 1) else 2)`) da hoy 2 E1303 con la relación del segundo en la `if` interior `[60,78)`; el ADR fija «dos uniones, dos E1303» pero no el origen relacionado: no se toca (I).

Verificación (la puerta completa es la única que fundamenta PASS):
1. **Diferencial exhaustivo acotado contra `main` y V6**: familia de `familia-oraculo.py` ampliada a profundidad 3, hojas `{q, r, 1, 2, ar, aar, r3, [q,r], r[0]}`, `var []` pendiente y concreto, asignación {ninguna, compatible, incompatible, antes de la unión}, ambos órdenes y 3 perfiles. Se enumera **cada** diferencia con su cláusula; 0 sin clasificar. No se acepta un muestreo: las 2 regresiones de C2 aparecieron a 1/4000.
2. **Oráculo independiente** (≤ 150 líneas, sin importar el verificador): referencia = primera hoja válida; aplanado completo; una E1303 sobre la primera incompatible; omisión por E0103; igualdad atómica con occurs-check. Se valida primero contra esta matriz y la tabla de la validación D; sirve solo donde modela (sin condiciones ni contextos externos).
3. **Detectores estructurales** en todo barrido: a) E0103 con primario no constructor; b) >1 E1303 con la misma relación `expected_type` en una unión; c) E1303 perdida con más E0103; d) cambio de aceptación.
4. Corpus: 3.674 fuentes de C, 27 twin (E0103 espurio ≤ F08, F10), 152 RD, diferencial debug/release y `check`/`run`, fuzz 4000 (semilla 7391045882, SHA 60cf15df…).
5. **Regresiones** (negativo aislado + positivo): R1, R2, R2b, R3, R3b, R4, X_R1na, X_R1ord, Y_pend_if, X_flat3, A sin asignación, X_R1inc, con esperado transcrito de §3; R3c y X_R1pre como positivos.
6. **Mutación inversa por regla** (restauración SHA-256): a) reintroducir el descarte del ancla; b) `denunciada` nunca se marca; c) igualdad no aplicada tras la denuncia; d) referencia = última hoja; e) retracción desactivada; cada una debe morir por un test nombrado.

Decisiones del ADR que faltan (registrar; no bloquean ALT-2):
- **Laguna L1 (Flujo A solo si se discute):** denuncia única cuando la primera incompatible se omite por E0103 pendiente: decido la lectura literal (l.301 + l.353: la siguiente hoja no recibe E1303; `Y_pend_if` = solo E0103). Hoy lo contradicen `main`, V6 y C2.
- **NO DERIVABLE → laguna → Flujo A:** (L2) origen relacionado de la E1303 cuando la referencia de la unión externa viene de un `if` anidado en la primera continuación (`X_nest_first`, I); (L3) R5, P4, P4pos, generated1411/1681 (derivación no cerrada; no se prueba que sea laguna); (L4) origen relacionado cuando la primera igualdad compatible es un nodo de unión entre dos pendientes (P1/P2, I).

## 6. Pila de `Checker::drive`

Reserva medida 5.944 B (`drive` 27.536 B, `resume_non_match` 33.080 B, pareja 60.616 B, techo 66.560 B de 65 KiB). ALT-2 toca solo funciones de unión llamadas desde el conductor; no añade ramas a `drive` ni a `resume_non_match`. El registro vive en `graph` (mapa), no en el marco; el acceso va en dos ayudantes `#[inline(never)]` (abrir/consumir), con `FactId` y progreso explícitos, sin campos nuevos en `IfProgress` más allá de un booleano ya presente (`anchor_pending`). Margen exigido ≥ +1.000 B, es decir, crecimiento máximo de la pareja 4.944 B; medir con `scripts/f5-frame-sizes.py` y `scripts/f5-stack65.py` (parte de `validate.sh`) con rustc 1.99.1 y perfil de test.

## 7. Binarios, fuentes nuevas y limpieza

Usados: `.scratch/cand-bin/base-release.exe` (`main`), `cand-release.exe` (V6) y C2 compilado en copia (release, 27 s, `target` propio ya borrado). No se usaron los mutantes `mut-*.exe`. Fuentes nuevas (todas con prefijo `fn Main(){let p=true;let ar=[1];`, ASCII, sin LF final; `rN`/`qN` según el caso):
- `X_R1na`: `let r=[[[1]]];var q=[];let a0=(if p then q else (if p then 1 else r));`; `X_R1ord`: igual con `then r else 1`; `X_R1pre`: `...var q=[];q=[[ar]];let a0=(...then 1 else r));`; `X_R1inc`: R1 + `q=[3];`.
- `X_R2na`: `var r=[];var q=[];let a0=(if p then q else (if p then r else 1));`; `X_R4na`: igual con `then 1 else r`; `X_R4asg_r`: R4 + `q=[[ar]];r=[[ar]];`.
- `X_flat3`: `var q=[];let a0=(if p then q else (if p then 1 else (if p then 2 else 3)));q=[[ar]];`; `X_nest_first`: `var q=[];let a0=(if p then (if p then q else 1) else 2);q=[[ar]];`.
- `Y_pend_if`: `var r=[];let a0=(if p then r else (if p then 1 else true));`; `Y_pend_if2`: `then true else 1`; `Y_pend_if_asg`: Y_pend_if + `r=[2];`; `Y_conc_if`: `let r=[1];` en lugar de `var r=[];` (sin `ar`).
