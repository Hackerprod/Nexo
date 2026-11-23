# Validación B dirigida del plan I07c-S5c actualizado (140 líneas, commit 3f1e6e1) y de la matriz corregida: **FAIL por dos precisiones menores**

Alcance: mis seis condiciones del informe del oráculo, coherencia de cifras, matriz corregida y decisiones abiertas. Ejecuté `--matrix` (45/45) y `--self-test` (15/15), reobtuve P2 y W0 del oráculo y comparé con mi derivación. Sin compilar, sin procesos.

## Condiciones del informe del oráculo

| # | Condición | Estado |
|---|---|---|
| 1 | Corregir matriz (W0, P2, w3), Diseño §3 y recuento | RECOGIDA. `rec_all.py` y `matriz-esperados.json` cambian solo W0, P2 y w3; el oráculo da P2 = E1303 `[101,102)` `A³`/`A²` rel `[75,80)`, E0102 `[127,129)` `A²`/`A¹` rel `[119,124)`, E0102 `[133,134)` `A¹`/`A²` rel `[119,124)`, y W0 rel `[59,63)`: es mi derivación. «Adjudicación de B» añadida a `oraculo-vs-matriz.md` |
| 2 | `Pending.md` con fecha y medición en B1-S1 | RECOGIDA (plan §5: se MIDE en B1-S1; mejora derivada con test o `Pending.md` 2026-11-22 con la prueba reproducible) |
| 3 | Orden estricto, B2 por diagnóstico, test P2 acotado | PARCIAL: orden estricto y test `p2_relation_is_the_assignment_origin` limitado a `q=ar` rel `[119,124)`: OK. B2 por diagnóstico: ver motivo 2 |
| 4 | T5 | RECOGIDA con coherencia (ver abajo) |
| 5 | Candidatos P03–P05, P12, P15–P17 y `unmodeled` | RECOGIDA (con esperados que coinciden con mi `mine2.py`; mi medición de main/V6 incluye también P04 y P05 como rojos, A solo cita P03, P15, P16 y P17 en «Medido ahora»: no bloquea, los candidatos entran por medición de C) |
| 6 | Mutación del rollback equivalente | RECOGIDA (plan §7) |

## Cifras 21/25/30 (22/26/31)
Recontadas: main NO = 23 + P2 = 24, OK 21; V6 mantiene 20 NO (P2 y w3 pasan de NO(I) a NO), OK 25; C2 NO = 14 + P2 = 15, OK 30. Contando NO(I) como correcto: main 24 − P3(I) = 23 NO, OK 22; V6 20 − R2b(I) = 19, OK 26; C2 15 − P3(I) = 14, OK 31. Coincide con el Diseño §3 y con el plan (§1, §2, y el Diseño §5). **Excepción:** `I07c-S5c-plan.md` l.18 (fila B1-S0) sigue diciendo «guardas de los **22** OK verdes» (motivo 1).

## T5 frente a «0 sin clasificar»
Coherente: T5 es una clase explícita de las cuatro («T5 PENDIENTE DE FLUJO A»), con lista nominal en el informe de C; no se cuenta como IGUAL AL ORÁCULO ni como DEFECTO y se reclasifica con la regla del addendum antes de D. Está fuera de la familia (solo O01 y O08, que son filas de B2), así que el único riesgo es de calendario: el plan dice «no bloquea B1» y a la vez «se reclasifica antes de que B1 llegue a D». Aceptable; condición para el coordinador: D de B1 no arranca con T5 sin reclasificar.

## Motivos del FAIL
1. **Cifra obsoleta en el plan.** l.18 (fila B1-S0): «guardas de los 22 OK verdes». Tras adjudicar P2 son 21 (plan §1 y §2 ya lo dicen). Corregir a 21; AGENTS exige no dejar cifras erróneas aunque sean menores.
2. **La condición de B2 «por diagnóstico» (§8.3) no cubre la cardinalidad ni el empeoramiento de filas ya NO en B1.** Dice que B2 solo se integra si ningún diagnóstico que B1 acierta deja de acertarse y que una fila ya NO en B1 puede mejorar sin contar como regresión. Pero la regresión más habitual de V6 es la inversa: AÑADE diagnósticos (la E1303 extra en `3` de la observación de A, la E0103 espuria de P12, las dos extra de X_two_unions). Un diagnóstico espurio añadido no es «un diagnóstico acertado que deja de acertarse»; y una fila ya NO en B1 podría empeorar (ganar espurios) sin que la regla lo vea. Definir la comparación como multiconjunto por fila contra el oráculo corregido: B2 no se integra si, en cualquier fila, (i) pierde un diagnóstico que B1 acierta (código, subcausa, primario, tipos, relación) o (ii) añade un diagnóstico que el oráculo no prevé o aumenta el número de diagnósticos erróneos que tenía B1. Con eso, «cardinalidad» deja de ser un atributo suelto y la observación de A y P12 quedan cubiertas por la regla y no solo por las filas nombradas en 8 bis.

## Veredicto
FAIL por 1 y 2 (ediciones de dos líneas). Todo lo demás está resuelto: matriz corregida y 45/45, orden estricto, T5 coherente, cifras coherentes salvo l.18, 140 líneas (≤ 220) y ninguna decisión abierta distinta de las del motivo 2. Con esas dos correcciones el plan queda validado y C de B1 puede arrancar.
