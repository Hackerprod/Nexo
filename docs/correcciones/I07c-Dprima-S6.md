# D' I07c-S6 (commit 4d75e2c) - informe

Veredicto: **PASS** (ver al final).

1. Diff 2de688b..4d75e2c: solo docs/, scripts/ y tests/; nada en src/ ni en tests/f0005_b1b.rs; sin #[ignore] nuevos. Los tests nuevos escriben en std::env::temp_dir() (no en .scratch). En un `git archive` limpio (sin .scratch) los 19 tests de S6 + RD pasan (nextest default, 19/19).
2. 9 filas contrastadas con el ADR (L3-15, L3-19, T-TD-cli, T-TD04e, T-F1p, T-F2p, RD-ejec, P-8, Q-3, +RD83/RD101): esperados transcritos (offsets recalculados a mano para la expansion D15 de DV: TD01 3=[66,67), TD03 true=[62,66), RD83 [79,84), RD101 [16,18), b2d [148,149), M05-variante [87,92)); negativo+positivo pareado, nombres honestos. Los spans de algunos casos se derivan con rfind sobre la fuente (token del primario por regla del ADR), no del binario.
3. RD: 152/152, --self-test PASS. Perturbaciones propias (envoltorio del binario): quitar un C -> 4/152; duplicar sitio -> 4/152; cambiar codigo -> 4/152; tipo no serializable -> 4/152; diagnostico espurio en fila aceptada -> 148/152; diagnostico extra con estructura valida en otro sitio lexico -> 152/152 (nivel I, por diseno). No hay logica que convierta D/V en N.
4. Auditor: exit 0, --self-test PASS, 14 sitios, 3 pendiente_decision (8283, 12418, 14730). Inyectado un `expression.span().clone()` nuevo en copia -> exit 1 "unreviewed raw span". Leidos 7082, 8283, 12189, 12418, 13492: clasificacion coherente.
5. Q-3: receta del ADR 11.8.A reproducida por el test: 93.307 bytes, 15.401 `y=a64;`, NX-E0003 observed=1000001, limit 1000000, budget_kind type_solver_steps, control con 1 asignacion aceptado. PASA.
6. Mutacion inversa (sobre HEAD 4d75e2c, restauracion por SHA verificada; los parches aplican por cadena aunque types.rs de HEAD difiere del medido en b7e9b27): M14 -> muerta por el hermano grouped_domain_union_and_operator_sites_keep_inner_primary (el test original grouped_operands_keep_one_causal_primary pasa, como declara el doc); M03b, M04c, M07c, M15b -> muertas por su test propio. (D no habia probado M03b, M04c, M07c, M15b.)
7. Puerta: gate.sh --verify GREEN (debug 2146/2146, release 2148/2148, objetivo 15/29, RD 152/152); SHA de scripts/validate.sh = origin/main (7a1c7a19...).
8. Limites declarados con honestidad (N-a.3 PARCIAL, T-TD-ord INCONCLUSA, K1-d2 NO EVALUADO con 8283/12418/14730, supervivientes M01b/M03/M04/M09c sin medir, M06b/M09/M09b equivalentes por redundancia).

## No bloquea
- I07c-C-S6-informe.md y I07c-S6-mutaciones-inversas.md citan lineas y 12 sitios (8283, 12301, 14613) y un SHA de types.rs (d5046aed, arbol b7e9b27) distintos de HEAD (1050c84c, tras el merge de F5-pila-r1; 14 sitios, 12418/14730). Es la desfase declarado; la evidencia de mutacion se reprodujo en HEAD para 5 mutantes.
- Auditor y su test atados a numeros de linea de types.rs: cualquier edicion de ese fichero lo rompe (intencional, pero fragil para S5).
- Spans de tests derivados con rfind sobre la fuente en vez de literales fijos.
- RD87c tratado por id en el script RD (stdout [3]); es el unico caso normativo con salida.

## Bloquea
Ninguno.
