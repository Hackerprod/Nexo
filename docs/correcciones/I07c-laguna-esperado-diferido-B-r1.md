# Validación B de `I07c-laguna-esperado-diferido-r1.md` — ronda 1: FAIL (un bloqueante acotado a M2/O01/O08)

Repro A, M1, O03 y O07 quedan validados y pueden adoptarse. Solo lectura; auxiliares (`chk.py`, `sweep*.py`, `rdcmp.py`) en `C:\nexo-work\tmp\validador-b-lag2\`.

## Derivación propia de repro A

Programa: `fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));r=[3];}`.

**Salida actual de `main`** (`check` y `run`): NX-E0103 `[30,32)` y NX-E1303 en `aar` `[74,77)` (esperado `Array<I64>`, hallado `Array<Array<I64>>`).

**Derivación del ADR.**
- La primera continuación `r` establece la referencia `Array<α>` (l.353, §4.1.2.4).
- `aar`, aplanada, es compatible y fija `α:=Array<I64>` (l.353 «esperado pendiente que se unifica… no es un error»; l.301 «primera igualdad compatible»).
- `[r]` es la primera continuación posterior incompatible, así que lleva la única NX-E1303 `[83,86)`.
- `r=[3]` se rechaza sin cambiar el tipo ya fijado (l.301 «nunca se usa una asignación rechazada…»; l.361 «una asignación fallida tampoco invalida el tipo ya establecido del destino»): NX-E0102/`type_mismatch` en el `3` `[92,93)`, con origen `aar`.
- No hay NX-E0103, porque `α` queda resuelto antes del cierre.

Coincide con la propuesta (orden, `data`, relaciones). `main` produce cada mitad por separado: con `r=[[3]]` da solo la E1303 de `[r]` (`expected_type=[53,54)`); con `else r` y `r=[3]` da solo la E0102 del `3` con origen `aar`. Los spans por bytes de A, M2, O01, O03, O07 y O08 coinciden con la fuente.

**El gemelo concreto no es oráculo**: falla la condición 1 de l.373 (`T=Array<I64>` contradice la igualdad `aar~α` ya aceptada en el prefijo). `main` depende del valor escrito después (con `r=[[3]]` da E1303 en `[r]`; con `r=[[[3]]]` da E1303 en `aar`): es la sustitución retrospectiva que l.373 y l.355 prohíben. La condición 2 (contexto externo) no hace falta: aquí no hay anotación. **La lectura 2 es la única sostenible: repro A no es laguna, es defecto con esperado transcribible.** NX-E0103 es incorrecto en cualquier lectura (§4.1 y fila l.1774).

## Motivo del FAIL

1. **M2, O01 y O08 dependen de una decisión que la propuesta no registra.**
   - Qué hay que decidir: que una variable pendiente sin constructor (`r[0]`) como primera continuación establezca la referencia de la unión, y que la igualdad recursiva de `r` se conserve y se complete después con `aar`.
   - Apoyo: l.353 (referencia de array «aún variable»), l.301, y `main` sin asignación o con asignación compatible, que ya sigue esa lectura.
   - Falta: ninguna frase del ADR lo dice para uniones; la adjudicación §5 lo dejó abierto (duda F06); el addendum tampoco lo enuncia.
   - Corregir: añadir al addendum esa regla con un ejemplo válido, uno inválido y su diagnóstico, o marcar esos tres esperados como pendientes de decisión.
   - Con esa decisión, el rebatimiento del gemelo vale: `T=Array<Array<I64>>` contradice el prefijo, porque `aar` fija `α:=Array<Array<I64>>`; por eso aparece el NX-E0102 extra en `r=aar`.

## No bloquea

- **Barrido de salidas protegidas, sin discrepancias:** las 152 filas RD con `main` sin desvíos en los C; 217 `.n` de fixtures y `corpus/` y 59 literales de `tests/*.rs`: ninguna salida protegida cambia con la regla; ninguna tiene la forma de repro A; RD19 es coherente.
- **Test existente que cambia:** `i07c_pending_nominal_domain::incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin` espera dos NX-E0103 que por el ADR son espurios (`r=[3]` fija `r` y `q` queda `Array<Array<I64>>`): su esperado se corrige en COR tras revisión; la propuesta ya lo advierte.
- **Límite sobre O02, O04, O05 y O06 (legítimo):** O02 es derivable como M2 con la misma decisión (J en `r` y M en el RHS `aar`, no J en `[ar]` como `main`); O05 da en la derivación de B J en `aar` y M en `s=ar`, no el M en `s` de `main` (decisión pendiente); O04 y O06 dependen de l.357 sobre el contexto I64 del segundo operando de `+` (decisión pendiente y posible defecto aparte).
- **Regresiones extra en COR:** repro A sin asignación tardía (donde `main` hoy pierde la E1303 de `[r]`); orden con `[r]` antes de `aar`; gemelo marcado como no oráculo.
