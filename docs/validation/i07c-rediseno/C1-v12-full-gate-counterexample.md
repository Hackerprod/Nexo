# C1 v12: contraejemplo de la puerta completa (continuación 25)

Estado: **FAIL reproducible**, sin PASS de C1 ni propuesta v13 validada. Base `main` `31d91db`; C0 `96d5644`; referencia C1 `88205510`. La salida de la puerta completa de Luna está en `C:/nexo-work/evidence/luna-c1-corpus.json` (SHA-256 `e92a2eece30cab90af4415cdaf8ebfc904c9dc71ca268922afe61c060842b756`): 20 corpus, 151.188 filas, D=7, A-unmodeled=3, H=2 históricas. No se modificaron corpus, esperados ni gate.

## Reproducción independiente

Se usaron los binarios release congelados de `main` (SHA-256 `83654d652d59d2b4e1ee03a2702c46c3c0a8a6831fd7a4a02bde7b85a693fc53`) y de C1 (SHA-256 `2a16eee103bd2e7602a734096d990c7aed724b9c09195335033613941d2b62c2`). Para cada fila se extrajo la fuente del corpus congelado mediante `verify_corpora.FILE` y `audit_families.source`, y se ejecutó `candidate_gate.execute` con perfil `f0-0002-b-v2`. Los diez casos reprodujeron su clasificación. La tabla da solo los primarios E0103; los E0102 y demás campos se cotejaron con la salida JSON de la CLI.

| Fila | Clase | `main` E0103 | Referencia E0103 |
|---|---|---|---|
| `b-projections-2026112401:2071` | D | 61–63, 210–214 | 61–63, 134–137 |
| `b-projections-2026112401:2373` | D | 61–63, 223–247 | 61–63, 244–247 |
| `b-projections-t5-2026112402:1182` | D | 52–54, 255–256, 274–275 | 52–54, 106–109, 274–275 |
| `b-projections-2026112403:1736` | D | 52–54, 61–63, 232–233, 261–262 | 52–54, 61–63, 148–153 |
| `dirigido5:2069` | A-unmodeled | 73–75, 135–136 | 73–75, 129–132 |
| `dirigido5:2741` | D | 130–133 | 119–121, 124–127, 130–133 |
| `dirigido5:2963` | D | 122–124, 156–157, 161–164 | 73–114, 122–124, 128–131, 161–164 |
| `dirigido5:4425` | A-unmodeled | 84–86, 166–167 | 84–86, 109–150, 153–156 |
| `dirigido5:5287` | A-unmodeled | 101–102, 110–113 | 73–75, 86–89, 110–113 |
| `dirigido5:5531` | D | 114–117 | 27–29, 108–111, 114–117 |

Mínimo de 78 bytes, SHA-256 `cf0279fa3fc51b9498120c6c2c4d2f2f1f23050b5d5a1089ff7e3fc8a5a0d69f`:

```nexo
fn Main(){var v=(if true then {return;} else {return;});var a=[];a=[v];v=[a];}
```

`main` da E0103 `[73,76)`; C1 v12 da tres E0103 `[62,64)`, `[67,70)`, `[73,76)`; el oráculo R6 da dos `[62,64)`, `[67,70)`. Rige F0-0002 §4.1.2.1 (igualdad compuesta atómica y ocurrencia fuente), §4.1.2.2 (deber propio de constructor y rechazo sin alterar igualdades previas) y §13.9 (sitios causales independientes y no duplicación). El repro contiene dos constructores con testigo y una igualdad recursiva posterior; un único span no identifica las obligaciones que pertenecen a cada igualdad.

## Causa comprobada y límite del corte

En `recursive_equality_diagnostic`, la referencia busca un origen por el grafo global de `ObligationKind::Constructor`; después guarda ese span en `PendingDeferredJoin.recursive_origin` y lo sustituye por `join.found` al drenar el `Join`. No se conserva la identidad de la igualdad que creó la obligación. Así el constructor de un intento distinto puede reclamar el E0103 de otro `Join`, desplazarlo o duplicarlo. En el mínimo, la traza muestra los constructores `[67,70)` y `[73,76)` en el mismo componente cuando se procesa la igualdad de `[73,76)`; la búsqueda elige `[67,70)`.

Dos ablaciones compiladas con `CARGO_BUILD_JOBS=4` bajo `heavy-lock --session i07c-rediseno` delimitan el fallo. Si `recursive_origin` se fija a `request.found` (comportamiento C0), las diez salidas vuelven a `main`, pero `owned_rows_gate.py --commit C1` falla en sus tres recursivos: el primario pasa de `[60,63)` a `[66,67)` en `recursive_69`, y análogamente en los otros dos. Si se usa el origen previo solo cuando la igualdad actual no aporta un constructor propio, los tres recursivos pasan, pero ocho de las diez filas siguen distintas de `main`. Estas ablaciones son diagnóstico, no solución; se retiraron del worktree de referencia.

La fila C1 de R6-final v12 §5 prohíbe `EqualityRecord`/journal; C2 los introduce. La puerta de §5 exige además que todas las filas no propias de C1 sean `main` u oráculo. El mínimo y las dos ablaciones muestran que elegir el primario por el grafo o por la presencia de un constructor en el span no satisface ambas obligaciones. Antes de v13 hay que asignar `Join` y `ConstructorDuty` por `EqualityId`/`ConsumerId` y comprobar por ejecución completa cada frontera C1–C6; si un hito intermedio no cumple, fusionarlo con el siguiente. No hay prototipo ejecutable de C2–C6 que permita afirmar ahora esa comprobación.
