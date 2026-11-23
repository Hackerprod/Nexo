# Validación B del diseño I07c-S5c-d2 (2g') — ronda 2: FAIL parcial (R1 y RE validadas; `matchpartial` se separa)

B compiló 2g' (release y debug) desde `origin/i07c-s5c-explor` @ aa3b505 y lo comparó con `main` y el oráculo. Informe de trabajo en `C:\nexo-work\tmp\validador-b-d2\informe-2.md`.

## Fuera de muestra (tres corpus de semillas nuevas, 4.500 programas cada uno, congelados con SHA antes de ejecutar)
| Semilla | SHA-256 |
|---|---|
| 7021993 | `611f34e21b71c06ba255a308a3e01f97cd0f3f0c9de7563ddb2f0961962c0f10` |
| 4417061 | `0976fb60d2f494ce5b3beca958da149210c416835c27bcb717e13f26c2cd3c48` |
| 9908255 | `d41babb25b6e6b55e5d9832f1ddcff2d2ae12cd31ba2b7100173940e524c586e` |

Formas: planas, `fn`, `fnparam`, `while`, `if`, `match`, `matchexpr`, bloques `value`, `annot`, `ret`. **0 regresiones de core en los 13.500 programas planos modelados** (perfil `f0-0002-b-v2`): 2.446 arreglos, 115 C, 15 D. R-A y R-B de la ronda 1 corregidos; permutaciones de la ronda 1: 0 regresiones. Ninguna pieza de R1 ni RE se retira: cada ablación rompe programas de las semillas nuevas donde 2g' acierta con el oráculo y no hay casos al revés.

| Pieza | Programas que la exigen |
|---|---|
| `pendfa` | 443 |
| `retfail` | 131 |
| `retfa` | 74 |
| `atomic0` | 26 |
| `retract` | 19 |
| `nojoin` | 17 |
| `arrx` sola | 90 |
| `skip` sola | 34 |
| `e0103rule` completa | 124 |

Mínimos (prefijo Π, esperado = oráculo): `retract`: Π+`let a0=(if p then [r] else (if p then q else (if p then 1 else (if p then r else ar))));q=ar;` → solo E0103 `[76,78)`; sin `retract` añade E0103 `[67,69)`. `nojoin`: Π+`let a0=(if p then true else (if p then (if p then r else q) else (if p then ar else [q])));q=aar;` → solo E0103 `[76,78)`; sin `nojoin` una E1303 espuria `[129,130)`. Citas ADR: existen y fundan las piezas (matiz: «las siguientes sin diagnóstico propio» es de l.353, no de l.367). `run`/debug/release/tres perfiles: 500 programas por perfil, `check` release = debug; `run` release = debug en 1.500 casos; aceptación de `main` y 2g' coincide. Pasadas de planning (1.500): máx. 3. Pila: pareja 60.616 B, margen 5.944 B, `f5-stack65.py` 5/5. PASA.

## Motivos del FAIL
1. **`matchpartial` implementa M1, no M3.** El diseño afirma M3 y que no cambia `main`; el ejemplo «Denunciada» del propio diseño (160 B; perfil `f0-0003-b-v1`; Σ=`fn Main(){let o:Option<I64>=Some(1);let p=true;let ar=[1];var q=[];var s=[];`; cuerpo `let a0=match move o{Some(n)=>{value: (if p then ar else 1);},None=>{value: true;}};}`) lo refuta: esperado (diseño y `main`) E0103 `[64,66)` y `[73,75)` y únicamente E1303 `[132,133)`; 2g' añade E1303 `[151,155)`; sin `matchpartial` coincide con `main`. El ejemplo «Pendiente» sí funciona con `matchpartial` (pierde su E1303 sin ella). En los tres corpus hay 695 diferencias propias del `match`: 685 desaparecen sin `matchpartial` y 588 añaden diagnósticos a los de `main`. Implementar M3 de verdad o cambiar la laguna a M1/M2 declarándolo cambio deliberado de `main`.
2. **Cifra errónea en la laguna de procedencia:** el ejemplo «Válido» (`s=[[4]];`) mide 84 B, no 87 B; los otros dos (115 B y 87 B) coinciden con `main` y el oráculo.
3. **M3 contradice ADR l.367** (un conflicto pendiente de serialización cuenta como ya denunciado; M3 trata distinto pendiente y denunciada): las lecturas principiadas son M1 o M2; el oráculo no modela `match`: va por Flujo A y el corpus no la adjudica.
4. **Intercambio de diagnósticos sin nombrar (familia d3):** en 5 de 13.500 programas planos 2g' gana la E0102 interna correcta de un literal-hoja pero pierde la E1303 sobre el literal que `main` daba; el oráculo da las dos. Repro (semilla 4417061, idx 2348), tras `fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];`: `var s=[];r=ar;let a0=(if p then (if p then (if p then [[q]] else (if p then 1 else s)) else (if p then [[q]] else q)) else [s,q]);q=ar;}`: `main` y el oráculo E1303 `[202,207)`; 2g' no. Nombrarlo y asignarlo a d3.
5. **Mejora sin clasificar:** en `annot` y `ret`, 2g' retira un E0102 cuyo primario es la envoltura de una unión interna que `main` emitía además de la hoja (semilla 7021993, idx 373, `[175,196)`): coherente con l.353 («sin duplicar el sitio por su envoltura»): listarlo como mejora clasificada.

## Qué puede avanzar
R1 y RE (d2-1 a d2-8) validadas por B: pueden pasar a plan por bloques si d2-9 (`matchpartial`) se separa hasta resolver 1 y 3.
Condiciones del plan: diferencial completo clasificado (incluidas las 695 diferencias de `match`), mutación por pieza = su ablación con población congelada, pila medida en cada commit (aborto > 2.500 B), D y D' distintos, corpus nuevos adicionales con SHA, 0 regresiones de core.
