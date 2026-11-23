# INFORME-7 — Ronda 7: variante de `atomic0` o retirada

EXPLORACION, no integrable. Inicio 2026-11-23 04:49. Resultado: **no hay variante de `atomic0` con 0 regresiones de core que repare `planned_projection…`; `atomic0` se retira
(`candidato-7.diff` = candidato-5 sin `atomic0` en el conjunto por defecto de interruptores)**. Pero la retirada tiene un coste medido (sec. 2): el candidato sin `atomic0` da 56 regresiones de core frente a `main`.

## 1. Variantes de `atomic0` (3, medidas)

Mecanismo: en `anchor_contrast`, con la union registrada y aun no denunciada, `atomic0` guarda `type_nodes` y los restaura si el contraste da error o crea una union pendiente.
Defecto: en el programa minimo `fn Main(){let x=1;var q=[];var s=[];let a=[s[x],(if 0 then true else q),(if x then q else s)];q=[true];s=[[true]];}` (y en `planned_projection…`) el candidato da E0102 en `q` en vez de la E1303 de `main`/oraculo y anade E0102 en `q` y `s`.

| variante | idea | `planned_projection` / `incompatible_prefix` | otros programas | resultado |
|---|---|---|---|---|
| `a0np` | no restaurar en la pasada de planificacion si no hay error | = `main` / = `main` | pierde `failed_nested` (E0103 nuevos, donde el oraculo respalda al candidato) y la ganancia de c1 2159 | RECHAZADA (CLI) |
| `a0sc` | no restaurar si la ancla es escalar concreto y lo hallado es una incognita, sin error | = `main` / = `main` | **matriz 40/45 core** (fallan W0, B_qr, P1, c3, w1; con atomic0 es 45/45) | RECHAZADA (matriz) |
| `a0de` | no restaurar cuando la union es elemento de array pendiente (`deferred_element`) | sin efecto (la bandera es falsa en ese punto) | — | RECHAZADA (CLI) |
Antes (ronda 6) `atomic0e` y `atomic0e+atomic0p`: 56 regresiones de core. Los casos que `atomic0` protege son uniones `(if true then true else q)` sin contexto esperado (W0, B_qr, P1, c3, w1); la diferencia con el defecto es solo que en este ultimo la union es elemento de un array con primer elemento pendiente `s[x]`, propiedad que la pasada de planificacion no ve.

## 2. Retirada de `atomic0`: ablacion completa (candidato-5 sin `atomic0`, todos los corpus)

Tabla A-D `eval3.py table main na` (nivel core; `na` = candidato sin `atomic0`): c1 A=4 (B 2737, C 148, D 24), b A=0, perm A=0, t A=50 (D 94), s7021993 A=1, s9908255 A=1, s4417061 A=0:
**56 regresiones de core frente a `main`** (con `atomic0`: 0). Son exactamente las 56 que `atomic0` evita. Indices: c1 [2159, 5498, 10272, 19152], s7021993 [745], s9908255 [3976], t (50, p.ej. 38, 48, 56, 66, 68, 128).
Programas que difieren entre candidato-5 y sin `atomic0`: 765 en total (c1 93, b 24, perm 33, t 489, c2 34, B 30/32/30); lista de indices en `atomic0-diffs.json`. Fuente y salidas de `main`/candidato-5/sin `atomic0` de los 6 de c1 y B: `atomic0-perdidas-muestra.txt`
(p.ej. c1 2159: `main` y candidato-5 solo E0103 `[67,69)`; sin `atomic0` anade E0103 `[76,78)`). No he extraido el oraculo por programa (en c1 el oraculo = `main` en estos casos, ver `minimos-ablacion.txt`); los 50 de `t` no se listan con fuente (corpus-t carece de oraculo).
Nota: no pude reproducir la cifra «26 arreglos» de la ronda 4 sobre este conjunto; los numeros anteriores son los medidos hoy.

## 3. Suite real sin `atomic0` (debug, `tests-candidato7.log`)

2167 ejecutados, 2152 pasan, **15 fallan** (frente a 17 con `atomic0`): ya **pasan `incompatible_prefix…` y `planned_projection…`**. Fallan: quota (2: `deferred_if_join_charges_the_reduced_boundary`, `failed_branch_join_keeps_reduced_quota_error_prefix`);
causal_obligations (6: adr_gap_nested…, failed_outer_array…, failed_outer_condition…, mixed_nested…, pending_inner…, unresolved_constructor_survives…); mutation_regressions (3: anchored_first_branch…, cyclic…, linked_array…);
normative_survivors (2: deferred_projection…, independent_causal_sites…); pending_nominal_domain (1: failed_nested_union…); structural_invariants (1: grouped_operands…). Los 2 de `ir::i07c_tests` siguen arreglados por `skipv3`.
`incompatible_prefix`: sin `atomic0` el candidato da la salida de `main`; la derivacion a favor del candidato (INFORME-6 sec. 4.2) deja de aplicarse, y el test pasa.

## 4. Decision

Elegir entre (a) retirar `atomic0` (arregla planned_projection e incompatible_prefix, pero 56 regresiones de core en corpus: 50 en `t`, 4 en c1, 2 en B) o (b) conservarla (0 regresiones en corpus, 1 defecto abierto en una familia estrecha, hallado solo por el test). Por la regla «ninguna pieza empeora lo que `main` acierta» estricta
se aplicaria (a); pero medido, (a) empeora mas programas de los corpus que (b). Decide el coordinador; aqui `candidato-7.diff` entrega (a) como pidio.

## 5. Limites (no medido)

No se ejecutaron en la ronda 7: matriz de 45 y aceptacion check/run para la configuracion sin `atomic0` (solo la matriz del candidato-5 y de `a0sc`); apilado; el oraculo por programa de las 56 regresiones; fuentes de las 50 de `t`. La suite se ejecuto solo en debug. El codigo de `atomic0`, `atomic0e/p` permanece como interruptor inerte en `types.rs` (solo cambia la lista por defecto).
