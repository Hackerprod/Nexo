# Validación B, confirmación final dirigida del plan I07c-S5c (140 líneas, commit 783415e): **PASS**

> Nota posterior: T5 se resolvió (no es laguna; la fija el ADR, aclaración derivada en `F0-0002-lagunas.md`); la clase T5 del plan se retiró. Lo que sigue es histórico.

Solo lectura del plan; sin compilar, sin procesos.

## Motivo 1 (cifra 21): resuelto
La fila B1-S0 (l.18) dice ahora «guardas de los 21 OK verdes». No queda otra aparición de «22 OK» en el plan (las demás «22» son la fecha 2026-11-22). Coherente con §1 y §2 y con el recuento corregido del Diseño §3 (main 21, V6 25, C2 30).

## Motivo 2 (condición de B2 por multiconjunto): resuelto y bien definido
§8.3 (l.124) y §8 bis (l.132): la comparación es de multiconjunto por fila contra el oráculo corregido (`--matrix`; cada diagnóstico = código, subcausa, primario, tipos, relación; la cardinalidad es el tamaño del multiconjunto). B2 no se integra si en alguna fila (a) pierde un diagnóstico que B1 acierta, incluido el E0102 de `q=ar` de P2 con relación `[119,124)`; (b) añade uno que el oráculo no prevé (E1303 extra en `3` de la observación de A, E0103 espurio de P12, las dos E1303 extra de X_two_unions); (c) aumenta los diagnósticos erróneos que B1 tenía. Una fila ya NO en B1 puede mejorar sin contar como regresión. Cubre justo lo que faltaba: adiciones espurias y empeoramiento de filas ya NO. Las filas extra (X_two_unions y la observación de A) son filas de `PLAN_ROWS` del oráculo, así que el mismo `--matrix` las incluye. No hay contradicción con §2 (el test `#[ignore]` de P2 en la rama no cuenta como verde y la comparación va por script) ni con la PARADA de §9.

## Sin contradicciones nuevas
Las cifras, el orden estricto, la clase T5, el test de P2 limitado a `q=ar` y los candidatos P03–P05 y P15–P17 siguen coherentes. Tamaño 140 ≤ 220; no queda decisión abierta.

## Notas no bloqueantes
- «Medido ahora» (l.82) cita en rojo solo P03, P15, P16 y P17; P04 y P05 (rojos en mi medición con main y V6) entran por la regla «candidatos si C los mide rojos en `main` y verdes tras S1», así que no hay contradicción.
- El criterio (b) debe leerse con la excepción del propio oráculo: los diagnósticos D/V (presencia de nivel I, `d_optional`) y T5 no cuentan como «no previstos»; aclararlo en el script `i07c_s5c_filas.py` evita rechazos de B2 por falsos positivos, no por falsos negativos.

## Resultado
PASS. El plan queda validado por B; C de B1 puede arrancar (tras la aprobación del orquestador y con las condiciones de `8 bis`).
