# D' (contexto limpio) sobre I07c-S6-COR, commit ddf551e4f47338df6a494f50badf797b6466978b

**Veredicto global: FAIL.**

Un defecto bloqueante reproducible, introducido por esta implementación y no declarado en el addendum
(motivo 1). Todo lo demás pasa: fidelidad, primarios, regla (a), pila, auditor, puerta y mutación inversa.
Hay además puntos que el coordinador debe decidir y que no bloquean (sección «No bloquea»).

Evidencia propia reproducible en esta carpeta (gen.py, gen2.py, runner*.py, diffs2.py, meta*.py, mut.py,
frozen.txt). Copias y targets borrados; ningún proceso residual; no se tocó C:\Projects\Nexo ni
C:\projects\Nexo-robustez.

## Motivos del FAIL (accionables)

1. **`for x in (G([])){}` pierde un diagnóstico: cambia el número de diagnósticos (el addendum dice que no cambia).**
   - Repro mínima: `fn Main(){for x in ([]){}}` con `--profile f0-0003-b-v1`, `check` y `run`.
   - Base 1010938: `NX-E0006 [19,23)` más `NX-E0103 types.infer [20,22)` (2 diagnósticos).
   - Candidato ddf551e: solo `NX-E0006 [20,22)` (1 diagnóstico). El control sin grupo `fn Main(){for x in []{}}`
     da 1 diagnóstico en base y en candidato.
   - Causa: `Checker::record_error` (src/check/types.rs, `graph.emitted_sites`) descarta una segunda denuncia con el
     mismo primario. Al pasar el primario de NX-E0006 de `([])` a `[]`, coincide con el sitio de NX-E0103 y este se suprime.
   - Regla incumplida: docs/adr/F0-0002-lagunas.md, sección «Primario de una expresión bajo agrupaciones»: «No cambia el
     código, la subcausa, la barrera, la aceptación, el orden causal ni el número de diagnósticos». El informe de C no lo
     declara y tests/i07c_group_primary.rs no tiene fila para `([])`.
   - También con `(([]))` y `( // c` + salto + `[])` (5 fuentes del corpus 2, 6 del corpus 1; solo esta familia).
   - Remedio razonable: el resultado nuevo coincide con el control sin grupo. Declararlo en el addendum (fe de erratas del
     coordinador: la supresión por sitio de §4.1.1/§4.7 rige sobre el primario desagrupado) y fijar una regresión de `check`
     y `run` con `([])` y `(([]))` que exija 1 diagnóstico. Si se prefiere conservar el recuento hay que tocar `record_error`.
     Mientras la contradicción exista el corte no cierra (AGENTS.md: contradicción entre documentos y defecto de núcleo bloquean).

## Requisitos

| # | Requisito | Veredicto | Evidencia |
|---|---|---|---|
| 1 | Un solo ayudante usado por todas las familias | PASA | src/frontend/parse.rs: `expression_primary` (único bucle `while let Expr::Group` de elección de primario) y `expression_primary_span` (`#[inline(never)]`). Delegan `Checker::primary_span`, names.rs (bind_call_target, bind, modo), lower.rs, lower_match.rs, effects.rs (`scope_sites`) y parse.rs (dos emisores de partial_transfer). Los otros bucles Group de types.rs/names.rs clasifican variables o sitios, no eligen primario. |
| 1b | Sin cambios fuera de plan, patrones, duplicación ni código muerto | PASA | Producción: parse, names, types, effects, lower, lower_match y fila de migrated_parser_contracts; ownership_flow intacto. `find_witness_enumerated` es oráculo `#[cfg(test)]`. |
| 1c | Ramas nuevas en drive/resume_non_match | PASA | No hay ramas nuevas; el diff de types.rs son 3 hunks de primario. |
| 2 | Primarios de la tabla del plan y del addendum | PASA | 17 filas literales del addendum con CLI propio, check y run, debug y release: 0 desvíos. 12 positivos válidos: checked sin diagnósticos. `([1])` [20,23), `(x)(2)` [19,20), `((x))(2)` [20,21). |
| 3 | Regla (a), destino agrupado no nombre | PASA | 9 filas de la tabla con fuente literal (fn [22,24), if, move, no_suspend, anidado, trivia, doble, guarda): primario y pasos exactos en dbg/rel x check/run. Filas «no llega a efectos» correctas. Probados además move agrupado, if con grupos internos, trivia con salto de línea y comentario UTF-8, closures anidados, closure invocada desde closure, guardas, `Next` agrupado (sin cambio). Ningún primario ni paso vacío en el candidato. |
| 4 | Diferencial base->candidato | FALLA (motivo 1) | Ver sección siguiente; salvo el motivo 1 todo cambio es el permitido. |
| 5 | Pila | PASA | f5-frame-sizes.py con /MAP propio: drive 27.536, resume_non_match 33.080, pareja 60.616, margen +5.944 (coincide con C). f5-stack65.py limpio: 65 KiB 5/5 en option_match126 y result_match126; control 16 KiB desborda. |
| 6 | Mutación inversa propia | PASA | 16 mutantes distintos con SHA-256 inicial/mutado/restaurado; 15 mueren con i07c_group_primary (nextest --profile default): M1 ayudante exterior, M2 ancla de scope_sites, M3 head Anchor, M4 order_start, M5 productor del temporal, M6 partial_transfer en parse, M7 None en names, M9 for_iteration, M10 primary_span de types, M11 CallSites vacío, M12 first_f5_capability, M13 non_callable_target en types, M14 Drop en lower, M15 match move en lower_match, M16 excess_positional_argument. M8 (clave de orden normalizada en first_f5_capability) sobrevive y es equivalente: el único b con a <= b < a_norm caería dentro de `(` o trivia, que no inicia candidato. |
| 7 | Auditor | PASA | i07c_span_audit.py exit 0, reviewed 20/20; --self-test PASS. `.span()` crudo insertado en producto de names.rs, effects.rs (línea 800 y dentro de find_witness, 4084) y types.rs (dos zonas): exit 1 en los cinco. Una cadena con `#[cfg(test)]`, `#[cfg(not(test))]` y `cfg(any(test,..))` no engañan al delimitador de zona de test. |
| 8 | Puerta | PASA | `sh scripts/gate.sh ddf551e... --verify`: registro GREEN verificado (árbol ce9081ec...). Log: debug 2168/2168, release 2170/2170 (coinciden con el JSON), RD 152/152, objetivo 15/29. SHA-256 de scripts/validate.sh = 7a1c7a19... igual a origin/main:scripts/validate.sh. RD propios con mis binarios dbg y rel: 152/152. |
| 9 | Honestidad de los límites de C | PASA con matiz | `for` de Array en lower.rs (~3108): inalcanzable, el `for` de Array es NX-E0006 pendiente en los tres perfiles. NX-E1111 de prepare_closure_call: sin repro tras 15 intentos. lower_match.rs 352/363/367/374/473: con un binario centinela [0,1) en esas líneas, 23.421 ejecuciones de mis corpus no dan ningún primario centinela (solo relaciones y pasos de origen de préstamo, que deben conservar el grupo): coherente con NO EVALUADO. Q4/Q7/S2e: no pude exhibirlos con grupos. |

## Diferencial (punto 4)

Generadores propios con semilla congelada antes de ejecutar (frozen.txt):
- Corpus 1: semilla 20261122771, 3.307 fuentes, SHA-256 bd3eaecca1219f9e0dced4d7a09399dc1b2d13292a945f3e1c7d063925e33b6a.
  Tuvo trivia `/*c*/` inválida en este lenguaje (no es comentario, da NX-E0002); lo reemplacé por el corpus 2.
- Corpus 2: semilla 20261123417, 4.500 fuentes, SHA-256 ed2d1670759d6bddb4c86e6d0b868e4121662a29c4daae88c2f3be2670a9ed9b
  (trivia de espacios, saltos de línea y `//` con UTF-8; mayor peso en efectos).
- Ejecuciones: 79.368 y 108.000 (3 perfiles x check/run x base/cand x debug/release), más `corpus/` y `tests/fixtures/`
  (217 .n, 5.208 ejecuciones) y las 152 RD.
- Anomalías: 0 pánicos, internal_error, InvalidIr o cuelgues; 0 divergencias debug/release (base y candidato);
  check/run difieren en 228 y 441 comparaciones, idénticas en base y candidato (fallos de ejecución de `run`, p. ej. R0102).
- `corpus/` y `tests/fixtures/`: base == candidato en las 5.208 ejecuciones.
- Cambios base->candidato por categoría, todos verificados automáticamente contra la regla (grupo exterior retirado, con
  trivia; o, para vacío, el primer token de la cabeza: nombre, `fn`, `if`, `move`): primario de NX-E0006, E0111, E0105, E0202,
  E1104, E1105, E1107; primario y pasos closure.call, function.call e intrinsic.call de NX-E0412, E0413, E0414, E0415.
  Corpus 2: E0006 1.034, E0111 902, E0105 4, E0202 38, E1104 168, E1105 26, E1107 56, E0412 170 (+202 pasos), E0413 178 (+220),
  E0414 80 (+100), E0415 408 (+488). Ningún cambio de código, subcausa, operación, data, relaciones, orden, outcome ni salida de
  `run`, salvo el motivo 1 (2 -> 1 diagnósticos).
- Sin primario vacío en el candidato; sin primario que siga siendo un grupo completo salvo `()` unidad, R0001 y orígenes de
  préstamo (ver «No bloquea»).
- Metamorfismo adicional (agrupar al azar un token, una llamada completa, un destino, o dos veces; 369 programas de corpus/,
  tests/fixtures/ y RD; semillas 7001, 7002, 7003, 8101): 0 neutralidades rotas en el candidato que no estén ya en la base;
  detector de fuga (primario igual al grupo cuyo interior era el sitio original): 0 en 5.152 mutantes.
- Profundidad: grupos 1..5.000 en 9 formas (for, destino, argumento, Drop, match, `+=`, efectos): sin pánico, debug == release,
  mismos códigos que la base, frontera NX-E0003/syntax_nesting coherente.

## No bloquea (decisión del coordinador o preexistente)

A. **R0001 con la condición de `while` agrupada.** `fn Main(){let x=1;var n=0;while (n<9999999) {}}`, `run`,
   f0-0003-b-v1: primario [32,43) `(n<9999999)`; el control `while n<9999999 {}` da [32,41). Igual en base y candidato.
   El origen del tick es `condition.span()` (src/ir/lower.rs, `WorkEvent::WhileCondition`, ~9631). Pasa igual con las
   sentencias `(n);`, `(n)=1;`, `(F());`. F0-0002 §11 «Origen de los ticks» fija «expresión de condición», «expresión completa»
   y «lugar destino completo» y dice que las agrupaciones conservan span; el addendum dice que esa frase habla del nodo y que la
   regla rige también para «ejecución». Hay ambigüedad real y el resultado depende del tick en que cae el límite. Lo registro como
   laguna de cuotas (andamiaje): el coordinador decide con ejemplo válido e inválido. Si decide que el addendum cubre R0001 pasa a
   defecto de núcleo y es un sitio de lowering.
B. **Neutralidad del destino de asignación agrupado (preexistente, igual en base).**
   `fn Main(){let x=1;var q1=[];q1=[[1],[true]];q1[0][0]=[x];q1=[[1]];}` frente a la misma fuente con `(q1)=[[1],[true]];`,
   `--profile f0-0002-b-v2`: 2 NX-E0102 frente a 3, con tipos esperados y relaciones distintos. Agrupar el lugar de una asignación
   cambia la inferencia pendiente. Fuera del alcance (no es primario); va a Pending.md.
C. Cobertura: sin test para `([])`; sin programa para lower_match.rs 363/367/374 ni NX-E1111 de prepare_closure_call (declarado por C).
D. Mi corpus 1 usó `/*c*/` como trivia; es defecto mío, no del producto.

## Archivos relevantes
- C:\nexo-work\tmp\validador-dprima2-s6cor\informe.md y scripts de la batería en la misma carpeta.
- Producción citada: C:\projects\Nexo-i07c\src\check\types.rs (`record_error`, `emitted_sites`), src\ir\lower.rs (~9631),
  src\frontend\parse.rs, src\ir\effects.rs.
