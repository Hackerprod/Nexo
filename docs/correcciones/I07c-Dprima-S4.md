# D' I07c-S4, commit 6394b67 (rama i07c-estructural)

Veredicto: PASS (sin bloqueantes).

(1) Diff be8c47c..6394b67: produccion solo src/check/types.rs; pruebas en tests/i07c_deferred_constraints.rs y tests/i07c_s4_context_regressions.rs (registrado en tests/suite.rs); tests/f0005_b1b.rs sin tocar; resto docs/andamiaje. Ganchos de prueba bajo cfg(test). PASA.
(2) grep: VisitCursor, solver_snapshot, visit.tainted, visit.numeric_domain_owner, parent_visit = 0 en src y tests. Sin alias: prior_taint/previous_owner/lhs/left_tainted desaparecen; right_tainted es un local leido del hecho y consumido en el mismo paso. PendingDeferredJoin.numeric_domain_owner preexiste y esta declarado en el plan. drive: un unico end_visit sin retornos intermedios; with_independent_fact begin/end emparejados; AtomicEnd comparte relojes por Rc. PASA.
(3) Diferencial be8c47c vs 6394b67, debug y release, check y run, JSON/exit/stdout/stderr, ademas debug-vs-release del nuevo: corpus+fixtures (217), 1.302 mutaciones del corpus, ~14.900 programas de 5 generadores propios (semillas 20261121, 424242, 9090909, 31337, 5550123; incl. anidamientos 5-200 niveles, closures, match con payloads, Option/Result, llamadas/Len/TryAdd/AssertEq), en perfiles f0-0002, f0-0003 y f0-0005: 0 diferencias, 0 panicos/internal_error/timeouts. RD: 152/152 en ambos. Repro de D: 3 NX-E0102/type_mismatch en [26,30), [32,35), [45,46) en base y nuevo, debug y release. PASA.
(4) Pila (lib test, 5 ejecuciones, 65/129/193/257 KiB, con y sin debuginfo, TYPES y CHECKER, 8 filas J16-S1C): minimo (biseccion) identico en las 32 combinaciones (129 o 193; sin debuginfo option/result/some_pattern 65). Unica celda distinta: sin debuginfo CHECKER array126 a 65 KiB base 1/5, nuevo 0/5 (ruido; umbral 129 en ambos). PASA.
(5) gate.sh --verify: "registro GREEN verificado". SHA-256 de scripts/validate.sh igual en HEAD y origin/main (51a23f2f...). PASA.
(6) Mutacion inversa por muestra (cargo test --lib, restauracion por SHA-256 verificada): M1 merge_child sin OR de taint, M2 owner Empty->heredado, M3 end_visit sin retirar hechos, M4 sitios de union solver Empty->Inherit, M5 taint siempre heredado: los cinco detectados (M4 por solver_bind_dispatch_applies_each_fact_boundary; los demas por varios tests). PASA.

No bloquea: los expect/assert de fact()/fact_mut() siguen en produccion como invariantes internas; ningun programa los alcanzo.
Limpieza: copias y targets propios borrados.
