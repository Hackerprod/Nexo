# Adjudicacion twin adjC: casos O01-O08 (`other_differences`)

Binarios: MAIN = `release.exe check --format json --profile f0-0002-b-v2`; BASE b506d7d = `nexo.exe check` (texto).
Auxiliares en `C:\nexo-work\tmp\twin-adj\adjC\` (`*.n`, `run.sh`, `bis.sh`). Cada caso original (O01..O08, con el preludio comun) se ejecuto en MAIN y en su gemelo concreto; cada reduccion se verifico con MAIN.

## Resumen de la causa (los 8 casos son UNA familia)

En los ocho, MAIN emite **NX-E0103 sobre un `[]` de un `var x=[]` cuyo elemento SI queda resuelto** por una asignacion posterior compatible (`r=[3]`, `r=aar`, `s=ar`, `q=[x]`...), y el gemelo concreto no emite NX-E0103. El disparador es siempre una **union (`if`) o literal de array sin contexto externo en la que una referencia con elemento pendiente se contrasta con una continuacion/elemento posterior anidado** (la referencia es `r[i]`/`q[0]`, o `r`/`[r]`/`[s]` junto a `aar`). Un `if` sin anidar de la misma forma NO produce E0103 (ver verificaciones), asi que la causa depende del anidamiento/aplanado de §4.1.2.4.

Contradiccion interna de MAIN (evidencia mas fuerte): en la misma salida, el `expected_type` del C se serializa ya completado (p. ej. `Array<I64>`, resuelto por la asignacion tardia) mientras que NX-E0103 declara el elemento irresuelto al cierre. Ambas cosas no pueden ser ciertas.

Regla del ADR (`F0-0002-second-slice.md`):
- §4.1: «Un array vacio o constructor Array conserva `NX-E0103` si su elemento sigue sin resolver al cierre de la funcion.»
- §4.1.2.2: «Si ninguna restriccion compatible lo completa, se omite al terminar porque el constructor `[]` emite NX-E0103» (el E0103 solo existe si ninguna igualdad compatible completa el elemento); «Un conflicto de forma o una igualdad recursiva ... puede conocerse antes que los tipos completos de sus operandos. Se conserva el conflicto en su sitio y se completan **sus** tipos con las igualdades compatibles posteriores»; ejemplo de E0103 legitimo: `var r=[];r=[r];` (sin solucion finita) y `let r=[];let a=[r,0];` (sin asignacion que lo complete).
- N(h): E0103/E0113 solo sobre constructores irresueltos con obligacion propia de §4.1.2.2.
- §4.1.2.7 (gemelo con tipo conocido) para los sitios C.

## Familia y repros minimos (verificados con MAIN)

Notacion: D = diferido, C = gemelo concreto, P = permutacion (asignacion movida antes del uso; no hay tipo diferido).

### Repro M1 (O07, O03)
- D: `fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));r=[3];}`
  MAIN: `NX-E0103` @[30,32) `[]`; `NX-E1303` @[74,77) `aar` (esperado `Array<I64>`, hallado `Array<Array<I64>>`).
- C: `...let r=[3];let a=(if true then r else (if true then aar else [r]));}` MAIN: solo `NX-E1303` @`aar` (esperado `Array<I64>`).
- P: `var r=[];r=[3];let a=...` MAIN: solo `NX-E1303` @`aar` (igual que C).
- BASE: D -> `NX-E0103` @31 (la base no tenia inferencia diferida: todo `var r=[]` sin tipo inicial daba E0103); C -> `NX-E1303` @`aar`; P -> `NX-E0103` @31.
- Contraste (misma forma sin anidar `r`/`[r]`): `var r=[];let a=(if true then aar else [r]);r=[3];` -> aceptado (sin E0103).

### Repro M2 (O01, O08)
- D: `fn Main(){let aar=[[1]];var r=[];let a=(if true then r[0] else (if true then r else aar));r=aar;}`
  MAIN: `NX-E0103` @[30,32); `NX-E1303` @[84,87) `aar` (esperado `Array<I64>`).
- C: `let r=aar;let a=(if true then r[0] else (if true then r else aar));` MAIN: solo `NX-E1303` @[78,79) `r` (esperado `Array<I64>`, hallado `Array<Array<I64>>`).
- P: `var r=[];r=aar;let a=...` MAIN: solo `NX-E1303` @`r` (igual que C).
- BASE: D -> `NX-E0103` @31; C -> `NX-E1303` @79 (`r`).
- Diferencia adicional: el C pasa de la primera hoja incompatible (`r`) a la segunda (`aar`). Sin anidar (`if p then r[1] else r;r=aar;`) MAIN si da `NX-E1303` @`r` sin E0103.

### Repro M3 (O04, O06)
- D: `fn Main(){let ar=[1];var q=[];let a=(ar + (if true then q[0] else (if true then [2] else q)));q=[3];}`
  MAIN: `NX-E0103` @[27,29); `NX-E0102/type_mismatch` @[37,39) `ar` (I64/`Array<I64>`); `NX-E1303` @`[2]` (I64/`Array<I64>`).
- C (`let q=[3];`) y P (`var q=[];q=[3];` antes): mismos dos C (E0102 @`ar`, E1303 @`[2]`), sin E0103.
- BASE: D -> `NX-E0103` @28 y `NX-E0102` @38; C -> `NX-E0102` @39.
- (El operando izquierdo `ar` contaminado deja el derecho sin contexto I64; es igual en el gemelo.)

### Repro M4 (O05)
- D: `fn Main(){let ar=[1];let aar=[[1]];var r=[];var s=[];let a=[[r],s];let b=(if true then [s] else aar);s=ar;r=ar;}`
  MAIN: `NX-E0103` @[50,52) (el `[]` de `s`); `NX-E0102/type_mismatch` @[64,..) `s`.
- C (`let r=ar;let s=ar;`) y P (`s=ar;r=ar;` antes): solo el `NX-E0102` @`s`; sin E0103.
- BASE: D -> E0103 @42 y @51; C -> `NX-E0102` @65.

### Repro M5 (O02)
- D: `fn Main(){let ar=[1];let aar=[[1]];var r=[];let a=(if true then r[0] else (if true then [ar] else r));r=aar;}`
  MAIN: `NX-E0103` @[41,43); `NX-E1303` @[88,..) `[ar]` (esperado `Array<I64>`, hallado `Array<Array<I64>>`).
- C y P: solo `NX-E1303` @`[ar]` (mismo sitio y datos). BASE: D -> E0103 @42; C -> E1303 @90.

## Veredicto por caso (cada uno verificado con MAIN sobre su fuente original)

Fuentes minimas completas en los repro de arriba; el preludio de cada original solo anade ruido (no cambia la diferencia).

| ID | idx | Observado en MAIN (original) | Reduccion |
|---|---|---|---|
| O01 | 29097 | E0103 @`[]` de r, + C E1303 en `[[2]]` en vez de `r` (D) | M2 |
| O02 | 39425 | E0103 @`[]` de r; resto de C iguales al gemelo | M5 (`r[0]` + `[ar]`) |
| O03 | 98859 | E0103 @`[]` de r; resto igual | M1 (`r` + `aar`/`[r]`, `r=[x]`) |
| O04 | 136136 | E0103 @`[]` de q; resto igual | M3 |
| O05 | 166953 | E0103 @`[]` de s; resto igual | M4 |
| O06 | 191390 | E0103 @`[]` de r; resto igual | M3 (`r[0]`, hoja `q`/`r`) |
| O07 | 241894 | E0103 @`[]` de r + E1303 @`aar` | M1 |
| O08 | 283013 | E0103 @`[]` de r, + E1303 @`aar` en vez de @`r` (D) | M2 |

En todos, el E0103 desaparece en el gemelo y en la permutacion con la asignacion adelantada, y los C coinciden salvo en O01/O08 (sitio del E1303).

## Lo que dice el ADR y veredicto

**Veredicto (los 8): DEFECTO DE PRODUCTO.**

1. NX-E0103 espurio (O01-O08). Salida esperada: ninguna NX-E0103, porque la asignacion tardia (o, segun el orden fuente, una hoja de la union) resuelve el elemento del `[]` antes del cierre (§4.1 y §4.1.2.2 citados arriba; N(h)); el `[]` solo debe dar E0103 si "ninguna restriccion compatible lo completa" (p. ej. `var r=[];let a=(if true then r[0] else (if true then r else aar));` sin asignacion: ahi si procede E0103, y MAIN lo da). §4.1.2.7: no hay contexto externo (`let a=...` sin anotacion), por tanto el diferido debe conservar los C de los usos comunes del gemelo cuando T es compatible con el prefijo.
   - O02, O04, O05, O06: los C (codigo, subcausa, primario) de D coinciden con los del gemelo y la permutacion; la unica diferencia es el E0103. Esperado: salida identica al gemelo (modulo offsets). Falla N(h)/N(a): se rechaza con un C extra de otro codigo sin obligacion propia.
   - O03, O07 (M1): ademas, el C de MAIN (E1303 @`aar`, esperado `Array<I64>`) usa el valor de la asignacion tardia (`r=[3]`) para tipar una hoja que, en orden fuente, fijo antes el elemento (`aar` fija `Array<I64>`). Con `r=[[3]]` MAIN da E1303 @`[r]` (esperado `Array<Array<I64>>`) sin E0103, es decir, el resultado cambia segun el valor posterior. §4.1.2.7: «No existe una regla de sustituir retrospectivamente cada variable pendiente por el tipo escrito en su ultima asignacion» y «Si una igualdad anterior fijo otro tipo, el diferido denuncia la restriccion posterior incompatible» (aqui el gemelo NO es oraculo del C porque `Array<I64>` es incompatible con la igualdad `aar ~ Array<b>` del prefijo). Mi derivacion de lo esperado para M1: E1303 @`[r]` + E0102 @`3` (esperado `Array<I64>`, hallado `I64`), sin E0103; el punto firme e indiscutible es la ausencia de E0103. El C exacto de este sub-caso lo debe confirmar quien decida (ver nota).
   - O01, O08 (M2): el gemelo es oraculo (T=`Array<Array<I64>>` es compatible con el prefijo, no hay contexto externo): esperado E1303 @`r` (primera continuacion posterior incompatible, §4.1.2.4: «una sola NX-E1303, sobre la primera continuacion posterior incompatible»; la hoja `r` es `Array<b>` frente a la referencia `b`, un conflicto recursivo conocido antes de completar tipos, que §4.1.2.2 manda conservar en su sitio). MAIN lo mueve a la segunda hoja (`aar`/`[[2]]`) y anade E0103. Confirmado por el no anidado (`if p then r[1] else r`), que MAIN resuelve bien en `r`.

No es LAGUNA el E0103: el ADR lo decide (solo si el elemento sigue irresuelto al cierre). No es DIFERENCIA LEGITIMA: ni el envenenamiento ni la regla de constructor sin resolver (§4.1.2.2) la justifican, porque el elemento esta resuelto y el propio `expected_type` serializado lo prueba.

Nota de honestidad: no he podido derivar con certeza, solo desde el ADR, el conjunto exacto de C de M1 (la interaccion entre la fijacion por orden fuente de la hoja `aar` y la asignacion tardia depende de leer juntos §4.1.2.2/4.1.2.4 y §4.1.2.7); lo que si es inequivoco para los 8 casos es el E0103 espurio. Si se prefiere decidir el sub-caso M1 como laguna (fijar si `aar` en una union con referencia pendiente fija el elemento antes que una asignacion posterior), seria una laguna ADICIONAL, no sustituye al defecto del E0103.

## Tabla resumen

| ID | idx | veredicto | familia | una linea |
|---|---|---|---|---|
| O01 | 29097 | DEFECTO DE PRODUCTO | E0103 espurio con union anidada (M2) | E0103 en `[]` ya resuelto + E1303 movido de `r` a la segunda hoja |
| O02 | 39425 | DEFECTO DE PRODUCTO | E0103 espurio (M5) | solo E0103 extra; C iguales al gemelo |
| O03 | 98859 | DEFECTO DE PRODUCTO | E0103 espurio (M1) | E0103 extra y C dependiente del valor de la asignacion tardia |
| O04 | 136136 | DEFECTO DE PRODUCTO | E0103 espurio (M3) | solo E0103 extra; C iguales al gemelo |
| O05 | 166953 | DEFECTO DE PRODUCTO | E0103 espurio (M4) | solo E0103 extra en `s`; C iguales al gemelo |
| O06 | 191390 | DEFECTO DE PRODUCTO | E0103 espurio (M3) | solo E0103 extra; C iguales al gemelo |
| O07 | 241894 | DEFECTO DE PRODUCTO | E0103 espurio (M1) | E0103 extra y C dependiente del valor de la asignacion tardia |
| O08 | 283013 | DEFECTO DE PRODUCTO | E0103 espurio con union anidada (M2) | E0103 extra + E1303 en `aar` en vez de en `r` |

## DEFECTOS DE PRODUCTO para quien decide (repro minimo y esperado)

1. **E0103 espurio con referencia pendiente en union anidada** (cubre O02, O04, O05, O06 y la parte E0103 de los demas). Repros: M3, M4, M5. Esperado: salida identica a la del gemelo/permutacion (sin NX-E0103), por §4.1 y §4.1.2.2.
2. **E0103 espurio + C dependiente de la asignacion tardia** (O03, O07; repro M1). Esperado: sin E0103; los C no deben cambiar segun el valor asignado despues (§4.1.2.7: sin sustitucion retroactiva); mi derivacion: E1303 @`[r]` + E0102 @`3` (a confirmar).
3. **E1303 en la hoja equivocada + E0103** (O01, O08; repro M2). Esperado: E1303 @`r` (primera continuacion incompatible), como el gemelo y la permutacion; sin E0103.

LAGUNAS: ninguna obligatoria; posible laguna adicional solo en el orden de fijacion del sub-caso M1 (ver nota).

Observacion de familia: las tres variantes comparten el mecanismo (E0103 depende del anidamiento de la union); la permutacion con la asignacion adelantada produce siempre el resultado correcto, lo que apunta a que el defecto esta en el tratamiento de la union/array con referencia pendiente cuando la asignacion llega despues, no en la regla de E0103.
