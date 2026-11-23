# I07c-S5a.1 — enmienda r2 al plan (§3, mecanismo V6, punto 2): la fuente de `read_plan` es una asignación aceptada del sitio

**Origen:** hallazgo de C (`I07c-C-S5a1-hallazgo.md`) y FAIL de B a la r1 (`I07c-S5a1-enmienda-B-r1.md`; w1/w3 en `docs/validation/i07c-s5/s5a1-b/`). Medido con binarios de `main` (HEAD de la base, `types.rs` `54c9dfed…`), del candidato (`types.rs` sin confirmar, solo leído) y de variantes mías en copias desechables (borradas). Perfil `f0-0002-b-v2`. Sustituye a la r1; la derivación validada de P1–P3 (`[93,101)`, `[119,124)`, `[88,94)`) no cambia. **No exige Flujo A.**

## 1. Causa raíz de w1/w3 (comprobada)

`assignment_type_plan[site]` (`types.rs`, relleno con `or_insert`) solo recibe la PRIMERA asignación cuyo RHS es `intrinsic_assignment_type` (enteros, `true`/`false`, `()`, nombres de locales concretos y arrays de ellos). `1<2`, `x+1`, `Len(ar)` o `(1<2)` dan `None`: `q=[[1<2]]` no entra nunca. En w1/w3 la única asignación intrínseca de `q` es `q=ar`, justo la rechazada (`Err`). Con la clave por RHS de la r1 la entrada rechazada bloquea `read_plan` y NO hay otra fuente: la asignación aceptada `q=[[1<2]]` no puede suministrar el tipo. (La r1 lo describía como «sitio sin entrada»; en w1 `q` sí tiene entrada, pero rechazada.) Medido: variante r1 = candidato en w1/w3 (`[85,86)` y `[76,77)`).
Alternativas descartadas: caer SIEMPRE a `concrete_evidence` cuando la entrada se rechaza arregla w1/w3 pero reintroduce el `NX-E0103` espurio en F01, F02, F04, F05, F09 y O05 (35 diferencias con el candidato): la evidencia puede venir de OTRO sitio (`[r,[q]]` + `q=[3]`). Quitar `failed` (M2d) devuelve el bug de `main` (E0102 espurio `[87,88)` en w1 y E0103/E0102 espurios en w3).

## 2. Decisión: texto exacto que sustituye al punto 2 de §3

> 2. **`read_plan` solo con una asignación aceptada del sitio como fuente.** `note_assignment_verdict(place, rhs_start, failed)` (en planning; `rhs_start = Self::primary_span(value).start`) anota `accepted_assignments: HashSet<(site, rhs_start)>` si la asignación a un local NO tuvo RHS `Err` ni dejó un `PendingDeferredJoin` nuevo (se conserva `failed`), y `rejected_sites: HashSet<site>` si lo tuvo. Se eliminan `rejected_assignments` por binding. La fuente del tipo leído, en este orden: (a) `assignment_type_plan[site]` si `(site, su RHS)` está aceptada; (b) si el sitio tiene entrada pero rechazada, `concrete_evidence` solo si su `origin.start` es el RHS de una asignación aceptada del MISMO sitio; (c) si el sitio no tiene entrada, `concrete_evidence` salvo que `site ∈ rejected_sites` (criterio del candidato, sin cambio). Sin fuente no hay `read_plan`. El resto de la compuerta (`!seed_cap`, `!conflicts_prefix`, `assigned_bases`, uniones diferidas) queda como en el plan.

La regla es una: «la lectura adelantada toma el tipo de la primera asignación aceptada del sitio»; (b) es la extensión mínima para RHS no intrínsecos y (c) conserva intacta la rama que ya existía. **Residuo declarado:** (c) mantiene un criterio por binding para sitios sin entrada. Unificarlo con (b) (exigir origen aceptado del sitio también ahí) cambia salidas de núcleo no derivadas (en P1 desaparece el E0103 `[27,29)`, en P2 el primario de la E1303 pasa de `[108,114)` a `[101,102)`): no se hace; límite con fecha (§5).
- **Terminación:** inalterada. Los conjuntos se llenan solo en planning y se vacían por pasada; `read_plan` solo lo lee la pasada final y el planning no lo aplica; `seed_cap` solo decrece (tope 129). No hay realimentación ni estado nuevo que el planning lea.
- **Independencia de orden:** la entrada sigue siendo la primera intrínseca por orden fuente; (b) elige por RHS aceptado del sitio, no por posición. Medido: w1 con `q=ar` antes y después de `q=[[1<2]]` (todas las permutaciones de `q=[[1<2]];r=[];q=ar;`) = `main` o correcto en el orden donde el candidato divergía; las 3.674 fuentes del diferencial (con permutaciones) sin diferencias de orden. Sigue siendo empírica.

## 3. Evidencia de la variante (copia mía de 12 líneas, no el código de C)

Diferencial de C (3.674 fuentes, `check`+`run`, `f0-0002-b-v2`): difiere del candidato solo en `generated:1411` y `generated:1681`. Repros A, A_control, B, F06, M1/M2, D-REF, constructor, protecciones, O01–O08 y F01–F19: iguales al candidato (esperados de §7 intactos). Batería metamórfica `q=[[<e>]]` sobre P1 y P2 con `e ∈ {true, 1<2, x+1, 3, (1<2), x, Len(ar), (x), 0-x, x<1, true],[false, 1<2],[x<1, x+1,3}` (28 programas): el `expected_type` del E0102 de `ar` es el RHS de `q=[[e]]` en los 28, y el resto de diagnósticos = candidato. Positivos (sin `q=ar;`) = `main`. `check` y `run` rechazan igual (exit 1).

## 4. Regresiones nuevas (C; esperados derivados, no del binario). Todas `check` y `run`, exit 1, `rejected`

1. **P1** (113 B) E0102 `[109,111)` `ar`, `Array<Array<Bool>>`/`Array<I64>`, `expected_type=[93,101)`; resto de P1 como `main` (`NX-E0103 [27,29)`, `NX-E1303 [70,71)` con `[60,64)`). Positivo: sin `q=ar;`.
2. **P2** (137 B) E0102 `[127,129)`, `Array<Array<I64>>`/`Array<I64>`, `[119,124)`. Positivo: sin `q=ar;`.
3. **P3** (104 B) E0102 `[99,100)` `3`, `Array<Array<I64>>`/`I64`, `[88,94)`. Positivo: sin `r=[[3]];`.
4. **w1** (112 B, `s5a1-b/w1.nx`) E0102 `[108,110)` `ar`, `Array<Array<Bool>>`/`Array<I64>`, `expected_type=[93,100)` (`[[1<2]]`); sin el E0102 `[87,88)` de `main`. Positivo: sin `q=ar;` (queda E0103 `[27,29)` y E1303 `[70,71)`). **Control de `failed`:** sin esa rama w1 reintroduce el E0102 `[87,88)`.
5. **w3** (139 B, `s5a1-b/w3.nx`) E1303 `[108,114)` (`[75,80)`), E0102 `[129,131)` `ar` con `[119,126)` (`[[x+1]]`), E0102 `[135,136)` (`[75,80)`). Positivo: sin `q=ar;`. Sin `failed` aparecen E0103 `[46,48)`/`[119,126)` espurios.
6. Barrido: 12 de los 28 casos de §3 como tabla (P1 y P2, `e` variado) con el RHS esperado calculado por bytes del propio fuente.
Mutantes: «clave por binding» (candidato) muere con P1/P2/w1/w3; sin `failed` muere con w1/w3 (P1 solo no lo mata); sin rama (b) muere con w1/w3; sin `site` en la clave de (b) muere con F01 (E0103 espurio).

## 5. Límites con fecha (a `Pending.md`, revisar en la siguiente campaña sobre el código vigente)

- **P4 deja de ser control:** con esta variante P4 coincide con `main` (el candidato lo acierta porque bloquea `read_plan`, no por regla). Su positivo (sin `r=ar;`) da un E0102 `[166,167)` en `r=[q]` idéntico en `main`, candidato y variante: defecto previo, independiente del veredicto. Tampoco se fija la relación `expected_type` de las E1303 de P4 (el addendum no la fija).
- `generated:1411` (derivación no cerrada), `generated:1681` (E1303 adicional de clase D, informativa), 4 aleatorias más con RHS no intrínsecos (variante = `main`, candidato distinto; sin derivar), P5 (117 B) y el `[133,134)` de P2: no entran a `tests/`.
- **Capas ≥ 3:** `q=[[ar]]`, `q=[[[true]]]` conservan `[85,86)`/`[93,94)` porque `seed_cap` (V6 punto 1) bloquea `read_plan`; sin esa compuerta darían el RHS correcto pero cambiarían diagnósticos en ~4 de 12.000 aleatorios sin derivar. Límite.
- Sitio sin entrada (c): criterio por binding; ¿el E0103 `[27,29)` de P1 es espurio (§4.1.2.2 «no lo recibe cuando esa igualdad compatible ya lo fijó»)? Lo emite `main`; se registra.
- Reserva de pila: `note_assignment_verdict` sigue `#[inline(never)]`; medir al cierre. Regenerar `i07c_span_audit.py` y el diferencial.
