# I07c Bloque I - D1 ronda 2 (ca64714; fd46264 motivo 1, ca64714 motivo 2)

Veredicto: PASS.

Entorno: clon D:\nexo-work\val-D1\repo2 (ca64714), binarios release/debug propios (target2), base ccf6df1 y 9b8e03e aparte. Arneses: h/oracle2.py, h/diff2.py, h/fuzz2.py (fuzz.py base SHA-256 328ba048...).

## Por requisito
1. Repros de ronda 1: PASA (release y debug, check y run).
   - b3x `let l0=DV;match l0{Some(x)=>{},_=>{}}`: checked/unit; variantes (None, Ok, anidados Some(Some)/Some(Ok), None primero, var, alias, match (l0), guarda, let k=x, k:Bool=x, k:I64=x): aceptan.
   - ADR 3.2/8.2 exactos: cobertura NX-E1304/non_exhaustive [57,62) missing_pattern None; NX-E1325/constructor_shape [78,83) y [89,94) con Data prelude:Ok/Option; NX-E0102/constructor_shape [95,97) con origin [66,73), expected [90,94), found [95,97]; Result: [106,108), expected [90,105); None bare: NX-E1304/unreachable_arm [75,76) con context [66,70).
   - Propiedad: NX-E0201 [94,95) y NX-E0202/definitely_consumed [109,110) (rechazados como en la base), positivos aceptan, recurso dentro de Some(x) rechazado, `match DV` directo checked. NX-E0113 vivo `let l0=None;...` rechaza; `Option<I64>` vivo acepta.
   - unreachable_resource_copy.n, RD71 [136,138), fixture independent_unresolved_constructor_in_branch.n, TD [16,20)/[37,42) y testigos [34,35), NX-E0103 [29,31): idénticos a la base en el diferencial.
   - Oraculos de S1/S2 (b1a..., b2a-e, b2g, q4w, pares, r=[1] [39,40)): siguen pasando.
2. Cursores: PASA. 0 coincidencias de `(self|checker).graph.(tainted|numeric_domain_owner)`; Checker (24) e InferenceGraph (26) sin campos equivalentes; VisitCursor vive en drive. Mutacion reproducida: quitar el mem::take en Task::StatementSite hace que TD pierda NX-E0102 [37,42) (CLI, release): discriminante.
3. Fidelidad por diff: PASA. Archivos: types.rs, diagnostic.rs, lower_match.rs (solo condiciones Analyze/Ty::Never en match_tests y bind_match_pattern; `selected.diverged()` intacto), tests i01/j01/j04/explain_and_print_output, i07c_*; F0-CODIGOS.md; todos en r7/r10/r11. Sin literales de fuente, sin allow(dead_code); helpers de prueba bajo cfg(test). Mutacion (2) reproducida: retirar la relajacion en match_tests hace fallar open_pattern_shape_accepts_... y unreachable_match_arms_still_check_resource_ownership.
4. Diferencial: PASA. 369 fuentes x check/run x release/debug, ccf6df1 vs ca64714: 0 diferencias; rel==dbg; check==run diagnosticos; RD 152/152 (rel y dbg), objetivo 15/29.
5. Fuzz semillas 1-5 (5000 programas, 4 ejecuciones): PASA. 0 panicos/InvalidIr/internal_error/cuelgues/divergencias rel-dbg o check-run (el internal_error de ronda 1, semilla 3, ya no existe). Frente a 9b8e03e cambian solo programas con match sobre raiz abierta: semilla 3 (internal_error -> checked) y semillas 4/5, donde la forma Option fijada por el match posterior rechaza un indexado/Len previo con NX-E0102/not_indexable (desvanece un NX-E0113): coherente con ADR 4.1.2.3 (base no indexable determinada despues, primario la base) y no es oraculo de N(f).

## No bloqueante
- Mutantes del motivo 2 no discriminantes que el informe C ya declara (reinicio de owner en check_against_context_one): sin programa que los exhiba.
- scripts/validate.sh completo no reejecutado por mi (el coordinador lo midio).
