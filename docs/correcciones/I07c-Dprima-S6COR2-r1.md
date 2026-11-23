# D prima, I07c-S6-COR-2, commit c75e704a57bea51ead8690d35cd4bbaadc228c24

Veredicto global: FAIL (1 motivo bloqueante segun el encargo: superviviente de la mutacion de regiones exhibido con programa).

## Motivo del FAIL
1. M00029 (src/ir/lower.rs, prepare_expr, find de token con start == span.start cambiado a !=), declarado por C sin programa, SE EXHIBE.
   Programa (perfil f0-0003-b-v1, run, debug): fn Main(){{}{}{}{}{}{}{}while true{let a=1;let y=move a;}}
   Esperado (F0-0002 s11, origen del tick del prefijo move): R0001 con primario = token move. Candidato: R0001 sobre move (correcto).
   Mutante (sha_initial f8fcbfde7a21, mutado f4bccbc57a30, restaurado f8fcbfde7a21): R0001 sobre fn (primer token del archivo).
   Ninguna prueba de la suite lo detecta. No es un defecto de comportamiento del candidato sino un hueco de prueba en region obligatoria,
   bloqueante porque el encargo lo manda. Correccion: regresion con ese programa (barrido de pad para move/edit como origen de tick) que falle con el mutante.

## Requisitos
- (1) Fidelidad: PASA. Produccion solo lower.rs y lower_match.rs. Un unico ayudante (parse::expression_primary[_span], ya en main). Sin condicionales por codigo/patron/constante. 7 sitios + espejo de S4 presentes. 40aba0b no compila las pruebas (falta use accepted; lo arregla 937f246): afecta solo a bisect, no a la revertibilidad de S1..S7.
- (2) 7 sitios con CLI propia, base y candidato, release y debug, perfiles 0003/0005 (0002 no admite closures): PASA. S1-S4: primario = interior en barrido de pad 0..11 con 4 envolturas. S5: Id(1) primer fallo N=43.478 en base y cand; (Id)(1) y ((Id))(1) base 45.454, cand 43.478; closure f() 52.631, (f)() base 55.555, cand 52.631. S6: (x)=2 en guarda, NX-E1305/guard_mutation primario x [53,54) (base [52,55)). S7: (Drop)(a) en guarda, NX-E1305/guard_transfer en Drop [59,63) (base aceptaba).
- (3) Diferencial: PASA. Semilla nueva 8814407, 3.600 fuentes, SHA-256 del manifiesto 3036c480d6fa863c56ee955d22de3a511e94f66265233631a143ebc0a27ab8a0 congelado antes de ejecutar. 3 perfiles x check/run x debug/release x base/cand, 21.600 celdas por binario, debug completo. Diferencias base a cand: 632 solo span R0001 (S1-S4), 1.332 solo span guard_mutation (S6), 476 guard_transfer nuevo en Drop (S7), 996 R0001 con destino agrupado (S5), 88 sin clasificar por mi heuristica, todas con destino agrupado y todas iguales al gemelo sin grupos. Oraculo independiente: gemelo SIN grupos (mismas extracciones aleatorias): cand(con grupos) == cand(sin grupos) en codigo, subcausa, outcome, data y eventos (spans enmascarados) en 21.600 celdas, 0 diferencias; textos de primario iguales salvo parentesis internos, ninguno es un grupo completo; base(sin grupos) == cand(sin grupos) en 21.600 celdas. corpus/ (44, con objetivo), tests/fixtures (173), 152 RD y 294 fuentes de tests: 0 diferencias salvo las S6/S7 previstas. check/run coherentes. 0 panicos, internal_error, InvalidIr, cuelgues, divergencias debug/release. Cifras protegidas: i10b_tests.rs y work_oracle_tests.rs sin tocar; corpus/ intacto.
- (4) Pila: PASA. /MAP + dumpbin: drive 27.536, resume_non_match 33.080, pareja 60.616, margen +5.944 (sin cambio). f5-stack65 5/5 a 65 KiB. RD 152/152 en debug y release. Auditor exit 0, self-test PASS, un span crudo nuevo en lower_match.rs (copia) da exit 1.
- (5) Mutacion inversa propia S1..S7 (sustitucion del ayudante por acceso crudo): las 7 se exhiben con mi bateria CLI (sha inicial lower.rs f8fcbfde7a219c61, lower_match.rs 609641cea276fc4d; mutados 08bf6429, b11a37f5, 2fa7d25c, 949e6fb7, 763ac7db, a5a62850, c3ecc734; restauracion verificada): PASA. Regiones: 12 de los 69 reproducidos MUERTOS con nextest (su test pasa sin mutar): M00001, M00008, M00014, M00015, M00022, M00026, M00027, M00028, M00031, M00032, M00033, M00034. Supervivientes: M00013, M00051, M00054 no exhibidos; M00021 y M00023 inalcanzables (InvalidIr / Statement::Value rechazado antes); M00029 exhibido (motivo 1).
- (6) gate.sh --verify: GREEN (arbol 636554b9...), 2193 debug y 2195 release ejecutados y pasados, RD 152/152, objetivo 15/29; SHA-256 de scripts/validate.sh igual a origin/main (7a1c7a19...). PASA.
- (7) Honestidad de C: el limite de debug parcial es cierto y no afecta (lo cubri completo); las desviaciones declaradas son ciertas y aceptables; la lista de supervivientes sin programa es incorrecta para M00029.

## No bloquea
- 40aba0b no compila las pruebas (bisect), corregido en 937f246.
- M00013, M00051, M00054, M00021, M00023 sin programa (deuda con fecha 2026-11-23 de C).
- f5-frame-sizes necesita --dumpbin explicito si dumpbin no esta en PATH.
Scripts: C:\nexo-work\tmp\validador-dprima-s6cor2\scripts\
