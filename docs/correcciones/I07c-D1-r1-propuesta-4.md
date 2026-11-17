# D1 - propuesta r1 con mecanismo r10 (HEAD 1165a1b)

Veredicto: PASS.

- Repros de D: r10 mantiene `selected.diverged()` y recorre brazos (MatchNext/finish_match_arm) en Analyze; solo relaja descriptor en match_tests (lower_match.rs:982, descriptor en 1038) y bind_match_pattern (1090, 1105). Regresiones exigidas: NX-E0201 [94,95) y NX-E0202/definitely_consumed [109,110), sus positivos, unreachable_resource_copy.n, control `match DV` directo, y recurso dentro de `Some(x)`. Mutacion (3) restaura el atajo r9 y debe fallar ambos negativos.
- Valor vivo: Analyze solo se activa tras divergencia (lower.rs 1606, 2267, ...), asi que la relajacion no alcanza codigo vivo; Live conserva InvalidIr; par vivo NX-E0113 vs checked/unit y su variante tras `return;` lo discriminan.
- Obligaciones al implementar: `None` frente a binding en bind_match_pattern (hoy 1105+ trata `None` como variante solo si ty es Option; con Ty::Never debe decidir por InferredTypes.bindings), `x` exportado en anidados/guardas, `None` primero, `let k=x`. Estan en la lista de pruebas; D las verificara.
- Archivos (r7 + lower_match.rs + i07c_tests.rs + explain_and_print_output.rs 4->5, verificado en linea 152): justificados.
No objecion adicional.
