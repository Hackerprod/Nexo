# I07c-S5c B1 — hallazgo de C: el plan no basta para R4, P15 y P16 sin contrastar la hoja `else` que es una variable pendiente

Fecha 2026-11-22. Subagente C (Sonnet, desvío por presupuesto, `docs/F0_ACUERDOS.md` §10). Rama local `i07c-s5c-b1`. No se ha integrado ni publicado nada: la desviación que sigue vive solo en el commit B1-S1 local y necesita la validación de A/B (plan) antes de que D la juzgue.

## 1. Qué no basta

El plan §3.4 pasa a `second_leaf_anchor(visit, progress, block, second, failed, anchored)` el contraste de la hoja `Else::Block` con las precondiciones de `main`, entre ellas «ancla y hoja presentes»: `(Some(base), Some(found)) = (progress.anchor, second)`. Cuando la hoja `else` es **una variable pendiente** (`var r=[]`), `second` es `None` (su tipo es interno; `if_finish_join` la reconstruye como `second_var.map(Type::Deferred)`), de modo que el bloque de `main` y el ayudante de la letra del plan **no la contrastan** con el ancla: su igualdad compatible con la referencia no se acumula y, tras la denuncia de una hoja anterior, el `join` local que hoy la cubría está suprimido (§3.4, `if_finish_join`). Efecto medido:

| Fuente (prefijo `fn Main(){let p=true;let ar=[1];let aar=[[1]];`; R4 con `let x=1;`) | Esperado (oráculo de E / matriz) | Binario con la letra del plan | Con la desviación |
|---|---|---|---|
| R4 `var r=[];var q=[];let a0=(if p then q else (if p then 1 else r));q=[[ar]];` | solo E1303 `[94,95)` | E0103 `[46,48)` espurio + E1303 (= `main`) | solo E1303 |
| P15 `var r=[];let a=(if p then ar else (if p then aar else r));` | solo E1303 en `aar`, sin E0103 | E0103 espurio + E1303 (= `main`) | solo E1303 |
| P16 = P15 con `r=[true];` | E1303 en `aar` y E0102 en `true` | pierde el E0102 (= `main`) | E1303 + E0102 |

El plan §5 declara R4 como negativo de B1-S1 que debe pasar a verde y P15/P16 como candidatos; con la letra del plan siguen rojos y R4 no se arregla.

## 2. Qué se hizo (desviación mínima, para validar)

En `second_leaf_anchor`, el tipo de la hoja es `second.or(Deferred(variable de la hoja))`, **solo cuando la unión está registrada** (`union_denounced` contiene la clave): es el mismo criterio que `first_for_anchor` aplica a la primera hoja en `if_after_first` y lo que el Diseño §2.2–§2.3 y D-REF fijan (una variable pendiente es una continuación válida con incógnitas). Con la clave **ausente** (ancla sin registro, p. ej. el `if` de valor de un brazo de `match`, plan §3.6) el comportamiento sigue siendo idéntico a `main` (la hoja pendiente no se contrasta). No hay otro mecanismo ni campo nuevo.

Se mantuvo el binario con la letra del plan (`.scratch/lit-src`, no versionado) para poder comparar: el resto de la matriz (los 21 OK de `main`, X_R1na, X_R1ord, X_flat3, Y_pend_if, Y_pend_if2, X_R1inc, X_join_local, X_compat_then_absorbed, X_pend_second_leaf, P03, P04, P05, P17, A, A_reord, A_noasg, F06, M2, O01, O03, O07, O08) coincide con el oráculo con y sin la desviación.

## 3. Hallazgos adicionales (sin decidir, a adjudicar)

- Filas fuera de la matriz de 45 («NO DERIVABLES» del plan): `generated1681`, `P4`, `P4pos` y `R5` **cambian** con B1 (también con la letra del plan; no es efecto de la desviación). El oráculo no valida esas filas (sintaxis fuera del subconjunto o derivación no cerrada); en `generated1681` el oráculo (no validado) coincide con `main` en los tres primeros diagnósticos y B1 los cambia. Se listan en el informe con su salida antes/después y quedan para adjudicación de D (`Pending.md` con fecha).
- W0, P1, P2, P3(I), c3, w1, w3 y B_qr no cambian con B1 (defectos previos de `main`, igual que en el diseño §3).
