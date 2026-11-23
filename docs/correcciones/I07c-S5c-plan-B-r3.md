# Validación B dirigida, 3.ª ronda, del plan I07c-S5c (`I07c-S5c-plan.md` 131 líneas, commit 74409e2): **FAIL por un único motivo de precisión**

Alcance: comprobar los 7 motivos de `I07c-S5c-plan-B-r2.md`, reproducir los programas nuevos y buscar contradicciones nuevas. Binarios: `.scratch\cand-bin\base-release.exe` (main) y `cand-release.exe` (V6), `check --format json --profile f0-0002-b-v2`, prefijo `fn Main(){let p=true;let ar=[1];`. No se compiló nada, sin procesos; copia de trabajo borrada.

## Reproducción de los programas nuevos (coincide con el plan en todos)

| Programa | main | V6 | Esperado del plan |
|---|---|---|---|
| X_join_local (`var q=[];let a0=(if p then q else (if p then 1 else (if p then true else 3)));q=[[ar]];`) | E1303 `[77,78)` rel `[59,60)` y E1303 `[95,99)` | E1303 `[77,78)` y `[105,106)` rel `[95,99)` | 1 E1303 en `1`: rojo en ambos |
| X_compat_then_absorbed (con `(if p then q else 3)` al final) | `[77,78)` y `[102,103)` rel `[59,60)` | `[77,78)` y `[102,103)` rel `[95,96)` | 1 en `1`: rojo en ambos |
| X_pend_second_leaf (`var r=[];let a0=(if p then r else (if p then (if p then r else 1) else true));r=[2];`) | E1303 `[95,96)` y `[103,107)` | ídem | 1 en `1` `[95,96)` rel `[59,60)`: rojo en ambos |
| mismo sin `r=[2];` | solo E0103 `[38,40)` | ídem | solo E0103 (control) |
| `(if p then r else (if p then r else 1))` (1 última) | solo E0103 `[38,40)` | ídem | guarda verde |
| X_two_unions (con `r=[[ar]]`) | E1303 `[86,87)` rel `[68,69)` y `[133,137)` rel `[115,116)` | 3: añade `[93,94)` | guarda: 2 E1303 exactos en main |
| Observación de A (`(if p then q else (if p then q else (if p then 1 else 3)));q=[[ar]];`) | solo `[95,96)` | `[95,96)` y `[102,103)` | V6 añade una E1303 en `3` (confirmado) |

Esperados derivables del ADR (l.353: una sola E1303 sobre la primera hoja incompatible; l.367: el conflicto pendiente cuenta como denunciado; `q=[[ar]]` fija `q` como `A³`, `r=[2]` fija `r` como `A<I64>`): sí en los tres negativos y en la guarda.

## Estado de los 7 motivos de r2

| r2 | Estado |
|---|---|
| 1 P2 contra §2 y §8.3 | RESUELTO: test propio `p2_relation_is_the_assignment_origin` verde en B1, `#[ignore]` solo en la rama no integrada `i07c-s5c-b2`, el ignorado no cuenta como verde y la comparación va por script |
| 2 clave ausente | RESUELTO en la segunda hoja (guarda `anchor_error.is_none()` conservada solo con `Absent`, prueba unitaria del primer error y guarda `match` vía `JNormal`); queda una incoherencia en la primera hoja (motivo único, abajo) |
| 3 `Absorbed` y `Reported` en la segunda hoja | RESUELTO: tabla completa por llamador y estado; `second_leaf_anchor` fija `anchor_absorbed`; negativo X_compat_then_absorbed reproducido |
| 4 X_pend_second_leaf | RESUELTO en lo reproducible (ya no es verde en main). La afirmación de que B1-M8 muere con este programa no está verificada: si el conflicto de la `1` fuera inmediato y no diferido, se marcaría `Denounced` por `Reported` y B1-M8 sobreviviría. Es condición para C (abajo), no defecto del plan |
| 5 X_two_unions | RESUELTO: guarda con fuente y esperado exactos, verificados |
| 6 orden | RESUELTO: cuarta clase y `orden-no-fijado.md` validado por B antes de B1-S1; fuera de la lista es DEFECTO, lista incompleta es PARADA |
| 7 `T0-group-shape` | RESUELTO (l.80–82 y l.130 de `scripts/j16_mutations.py`, históricos intactos). Matiz: el script no «solo escribe»: `--replay` (l.177) lee y reescribe el JSON histórico sin consultar `MUTATIONS` ni `PAIRED_FIXTURES`, así que borrar las referencias no lo rompe. Queda `rows["mutation-grouped-array"]` (l.55) sin mutación que lo use; no bloquea |

## Motivo del FAIL

1. **Contradicción entre §3.3 y la tabla de §3.4 para `Absent` en la primera hoja.** §3.3 dice que con `Absent` y conflicto solo diferido (la cola creció) `anchor_contrast` devuelve `Compatible` («no se marca ni se calcula pendiente»), mientras que la tabla de §3.4 para `if_after_first` con `Absent` pone `anchor_pending = true` «como `main`» ante un conflicto diferido (en `main`, l.14111, la primera hoja sí calcula `anchor_pending`). Con `Compatible` el llamador no puede saber que la cola creció, así que `if_finish_join` rehace el `join` local y un `if` bajo un ancla de `match` (clave `Absent`, l.14423) dejaría de ser idéntico a `main`, contra la guarda de §3.6 y la PARADA. Resolver con una regla única: o `anchor_contrast` devuelve `Pending` también en `Absent` y `second_leaf_anchor` lo ignora en ese estado (la tabla ya dice «ninguna»), o `if_after_first` calcula por su cuenta el crecimiento de cola; y corregir la línea de §3.3. Es una edición de una línea; el resto queda resuelto.

## Respuestas a las preguntas del coordinador

- Mecanismo único y no parche: sí. `Absent`, `Open` y `Denounced` están en un solo ayudante y `anchor_absorbed` es su resultado `Absorbed`.
- Contradicciones nuevas: solo la de arriba. §2 y §8.3 son coherentes: el test de P2 ignorado no cuenta como verde, B2 se integra solo si ninguna fila pasa de OK en B1 a NO o NO(I), y P2 lo impide mientras no se corrija, lo que el plan asume y registra.
- Decisiones abiertas: ninguna (X_R1inc y W0 tienen salida definida; 131 ≤ 220 líneas; sin «B decide»).

## Condiciones que exijo a C y D cuando el plan pase (si algo de esto falla: PARADA)

1. C: antes de B1-S1, `orden-no-fijado.md` y `oraculo-vs-matriz.md` presentes y validados por B; la matriz incluye los tres negativos nuevos, X_compat_then_absorbed y la guarda X_two_unions.
2. C: demostrar que B1-M8 muere con X_pend_second_leaf y registrar si el conflicto de la `1` es diferido o inmediato (cuenta de cola antes y después); si sobrevive, sustituir el programa, no quitar el mutante.
3. C: B1-M5 debe poner roja X_two_unions; B1-M6 la prueba unitaria de cola; B1-M7 X_join_local y X_compat_then_absorbed (restauración por SHA-256).
4. C y D: la comparación B2 contra B1 incluye como fila la observación de A y P2; B2 no se integra si alguna pasa de OK a NO.
5. D: reproduce con su propio binario los tres negativos y la guarda; compara `check` y `run`, debug y release; confirma que la guarda `match` de §3.6 (JNormal) da la salida de `main` y que con clave `Absent` el primer error no se sobrescribe.
6. D: confirma la pila en cada commit de `types.rs` contra la línea base medida en S0, no contra 60.920.
