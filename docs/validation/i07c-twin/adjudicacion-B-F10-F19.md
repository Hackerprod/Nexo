# Adjudicacion twin: casos F10-F19 (id de trabajo adjB)

Todos los programas llevan el preludio `fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];` y se cierran con `}`; abajo solo se da el cuerpo. F15 necesita ademas `fn H2(a:view Array<I64>)->Array<I64>{return [0];}` solo en la fuente completa (los minimos no la usan). Binario MAIN = `release.exe check --format json --profile f0-0002-b-v2`; BASE = `nexo.exe check` (texto). Ficheros auxiliares y scripts en `C:\nexo-work\tmp\twin-adj\adjB\` (`run.sh`, `full.py`). Todos los resultados de este informe se obtuvieron ejecutando los binarios; ningun fichero del repo fue modificado.

## Resumen ejecutivo

Los diez casos son la MISMA familia (llamada aqui **E**): el diferido emite `NX-E0103` sobre constructores `[]` / `[v]` que SI quedan resueltos por una asignacion tardia compatible, y a la vez cambia o pierde los C reales (E1303/E0102) segun el ORDEN de las asignaciones tardias. La condicion desencadenante, aislada en cada caso, es que antes de las asignaciones tardias exista un conflicto de forma sobre una variable aun pendiente (un `if p then p else q`, `s < true`, `if p then s else x`, etc.) y despues un constructor/union que relaciona dos variables pendientes. Quitando el conflicto previo (o poniendo las asignaciones en el otro orden) la salida cambia a una consistente con el ADR, lo que demuestra que no es una diferencia legitima de diagnostico sino una resolucion perdida. Veredicto para los 10: **DEFECTO DE PRODUCTO** (con el matiz de F18 abajo; en F18 el defecto es el E0103 espurio, la cuestion del C exacto es LAGUNA menor).

Los gemelos concretos NO son oraculo aqui (ADR §4.1.2.7: el adelanto del tipo solo conserva diagnosticos si T es compatible con las igualdades del prefijo y no vuelve determinado un contexto que era indeterminado; en estos programas el gemelo adelanta `r=[3]` antes de las uniones, lo que cambia que hoja da el conflicto). Por eso no se usa la diferencia con el gemelo como evidencia: el defecto se demuestra con (a) el E0103 sobre una variable que el propio programa resuelve (§4.1.2.2) y (b) la dependencia del orden de dos asignaciones a variables DISTINTAS, donde el orden fuente de las restricciones relevantes no cambia.

## Regla del ADR que decide (cita)

- §4.1.2.2 (cierre de funcion): "La obligacion de resolver el elemento de un constructor Array se comprueba siempre al cierre de la funcion, aunque no este vacio: si su elemento es una variable sin constructor que ninguna restriccion compatible resuelve, emite NX-E0103 sobre ese constructor completo." y "[] establece Array<alfa> ... un escalar incompatible con ese constructor no puede fijar alfa". En todos los minimos de abajo existe una asignacion compatible (`q=[3]`, `s=[[3]]`, ...) que resuelve la variable; luego E0103 no procede.
- §4.1.2.1: "Una igualdad entre tipos compuestos es atomica: un conflicto no conserva unificaciones parciales efectuadas al intentar esa misma igualdad. Las restricciones independientes de otros hijos si se conservan."
- §4.1.2.2: "Una igualdad incompatible se rechaza, sin sustituir el tipo establecido ... Nunca se usa una asignacion rechazada para cambiar el tipo de lecturas anteriores." y §4.1.2.3: "Al establecerse despues el constructor Array, esa restriccion anterior determina el elemento antes de comprobar las hojas de la restriccion posterior" (conflicto en la hoja posterior).
- N(a)/N(b)/N(d)/N(h): conservar todos los C con evidencia propia; E0103 solo sobre constructores irresueltos con obligacion propia; "La omision de un conflicto no serializable nunca permite aceptar una incompatibilidad segura: queda el C del constructor pendiente" (se omite el C solo si el componente pendiente emite E0103 al cierre; aqui el componente deja de estar pendiente).
- BASE b506d7d: no tenia inferencia diferida; todo `var x=[]` daba `NX-E0103` en su `[]` ya en la declaracion (en los diferidos de los 10 casos la base da E0103 en cada `[]` y solo en ellos mas el primer E0102 independiente), y en los gemelos concretos da los mismos C de tipo que MAIN (p. ej. F11/F13/F18 la base coincide con la lista de C de MAIN en el gemelo). Es decir, la base no es referencia para el diferido; solo confirma que los C del gemelo existian antes.

## Diagnostico por caso (programa completo, MAIN)

Notacion: `COD[/sub]@offset_rel(texto) esperado/encontrado`; offsets relativos al fin del preludio.

| Caso | Diferido (MAIN) | Gemelo concreto (MAIN) | Permutacion relevante (MAIN) |
|---|---|---|---|
| F10 | E0103@15 `[]`(q), E0102 true I64/Bool, E0103@83 `[r]`, E1303@102 r Array<Array<I64>>/Array<I64> | E0102 true, E1303@85 `[r]` I64/Array<Array<I64>> | [1,0]: igual que el diferido |
| F11 | E0103@6, E0103@15, E0102 x Array<I64>/I64, E0102@96 `<` I64/Bool | E0102 x, E1303 q Bool/Array<I64>, E0102 r I64/Array<I64>, E0102 q Bool/Array<I64> | [1,0]: E0103@6, E0103@15, E0102 x, E0102@98 q Bool/Array<I64> |
| F12 | E0103@6, E0103@15, E0102 r Array<Array<I64>>/Array<I64>, E0102 p Array<I64>/Bool | E0102 r, E1303 false Array<I64>/Bool, E0102 p | [1,0]: sin E0103; E0102 `[[q],r]`, E1303 false Array<Array<I64>>/Bool, E0102 p, E0102 r, E0102 `3` Array<I64>/I64 |
| F13 | E0103@6, E0103@15, E0102 q Bool/Array<I64>, E1303 `[q]` Array<I64>/Array<Array<I64>> | E0102 r, E1303 r, E0102 q, E1303 `[q]` | [1,0]: sin E0103; E0102 r Bool/Array<Array<I64>>, E1303 r Bool/Array<Array<I64>>, E0102 q, E0102 `3` Array<I64>/I64 |
| F14 | E0103@15, E0103@24, E0102 true, E0102 ar, E1303 q Array<Array<I64>>/Array<I64>, E0102 x | + E1303 br Array<I64>/Array<Bool> | [2,1,0]: sin E0103; E1303 br Array<Array<I64>>/Array<Bool>, E0102 `3`; [1,2,0]: igual que el diferido |
| F15 | E0103@6, E0103@24, E0102 true, E1303 x, E1303 `r[x]` Array<Array<I64>>/Bool, E0102 s | E0102 true, E1303 x, E1303 `r[x]`, E0102 s | [2,1,0] y [1,2,0]: sin E0103; E0102 true, E1303 x, E0102 s, E0102@142 `true` Array<Array<I64>>/Bool (no hay E1303 en `r[x]`) |
| F16 | E0103@15 `[]`, E0103@43 (literal completo `[q[1],(if..)][Len(..)]`) | E0102 `[q]` I64/Array<Array<I64>> (x2) | [1,0]: igual que el diferido |
| F17 | E0103@15, E0103@24, E0102 false, E0102 s Bool/Array<Array<I64>>, E1303 `[s]` Array<Bool>/Array<Array<Array<I64>>> | los tres ultimos C iguales, sin E0103 | [2,1,0]: sin E0103; E0102 s, E1303 `0` Array<Array<Array<I64>>>/I64, E0102 `true`; [1,2,0]: igual que el diferido |
| F18 | E0103@15, E0103@24, E0102 r Bool/Array<I64>, E1303 s I64/Array<I64>, E0102/not_indexable (if..) domain:array/I64 | los tres C iguales + E1303 q I64/Array<I64> | [2,1,0], [1,2,0]: igual que el diferido |
| F19 | E0103@15, E0103@24, E0102 true, E0102 `q[0]` Array<I64>/I64, E1303 true Array<I64>/Bool, E0102 r | los mismos 4 C, sin E0103 | [2,1,0]: sin E0103; E0102 `<` I64/Array<I64>, E1303 true, E0102 r, E0102 `3`; [1,2,0]: igual que el diferido |

(La base da, para cada diferido, `NX-E0103` en cada `[]` y despues los C de tipo independientes; para el gemelo concreto da los mismos C de tipo que MAIN, salvo que no pinta los D.)

## Programas minimos por caso (MAIN; cada uno verificado con el binario)

En cada par, "malo" es la salida de MAIN; "bueno" es la salida de MAIN al cambiar SOLO el orden de las dos asignaciones tardias (o quitar el conflicto previo), que es la que el ADR exige.

### F11 (representante de la familia; el mas pequeno)
Diferido minimo (los dos conflictos se aislan con `if p then p else q`):
`var r=[];var q=[];let a=(if p then p else q);let c=[q,[r]];q=[3];r=[3];`
- MAIN: `E0103@r []`, `E0103@q []`, `E0102 r I64/Array<I64>`.
- Con el orden `r=[3];q=[3];` o sin la sentencia `a`: no hay E0103; solo `E1303 q Bool/Array<Array<I64>>` y `E0102 '3' Array<I64>/I64` (o solo `E0102 '3'` sin `a`).
- Gemelo concreto minimo: `let r=[3];let q=[3];let a=(if p then p else q);let c=[q,[r]];` (no es oraculo, §4.1.2.7).
- Base: `NX-E0103` en ambos `[]`.
- ADR: §4.1.2.2/§4.1.2.1 (arriba). `r` y `q` quedan resueltos por `q=[3]`,`r=[3]`; E0103 no procede; el C se denuncia en la hoja posterior (`3`), no se permuta.
- **Veredicto: DEFECTO DE PRODUCTO.**

### F13
`var r=[];var q=[];let u=(if p then p else r);let b=(if p then r else [q]);r=[3];q=[3];`
- MAIN: `E0103@r`, `E0103@q`, `E1303 [q] Array<I64>/Array<Array<I64>>` (y no se denuncia el `r` del `if p then p else r`).
- Con `q=[3];r=[3];`: `E1303 r Bool/Array<Array<I64>>` y `E0102 '3' Array<I64>/I64` (sin E0103). Eso es lo que el ADR da (la union `b` precede a `r=[3]`; r queda Array<Array<alfa_q>>; el conflicto de forma de `u` se completa con `alfa_q=I64`).
- Gemelo: `let r=[3];let q=[3];let a=(if p then p else r);let b=(if p then r else [q]);` -> `E1303 r Bool/Array<I64>`, `E1303 [q]`.
- Base: E0103 en los `[]`.
- ADR: §4.1.2.2 ("se completan sus tipos con las igualdades compatibles posteriores"), §4.1.2.3, N(b)/(h).
- **Veredicto: DEFECTO DE PRODUCTO** (misma familia E; falta ademas el E1303 del `r` del primer `if`, que N(b) obliga a conservar).

### F12
`var r=[];var q=[];let u=[[q],r];let b=(if p then p else r);r=[3];q=[3];`
- MAIN: `E0103@r`, `E0103@q`, `E0102 r(en u) Array<Array<I64>>/Array<I64>`.
- Con `q=[3];r=[3];`: `E1303 r(en b) Bool/Array<Array<I64>>` y `E0102 '3' Array<I64>/I64`. Sin la sentencia `b` (ambos ordenes): solo `E0102 '3'`.
- Gemelo: `let r=[3];let q=[3];let u=[[q],r];let b=(if p then p else r);` -> E0102 r, E1303 r.
- Base: E0103 x2.
- ADR: §4.1.2.2 y §4.1.2.3 (el C se denuncia en la hoja posterior `3`, "Nunca se usa una asignacion rechazada para cambiar el tipo de lecturas anteriores": MAIN denuncia `r` en `u` usando `r=[3]`, que es posterior).
- **Veredicto: DEFECTO DE PRODUCTO.**

### F10
`var r=[];var q=[];let t=(if p then q[x] else (if p then [r] else r));r=[3];q=[3];`
- MAIN: `E0103@q []`, `E0103@[r]`, `E1303 r Array<Array<I64>>/Array<I64>`.
- Controles: con `q=[[[3]]]` en vez de `q=[3]`: solo `E1303 r`; sin la union anidada (`if p then q[x] else [r]`): `E0102 '3' Array<Array<I64>>/I64` (E0102 sobre la asignacion posterior incompatible de q) y sin E0103; con `let q=[[[7]]]` concreto: solo `E1303 r`. Salida esperada por consistencia con esos controles y §4.1.2.2/4.1.2.4: `E1303 r` y `E0102 '3' Array<Array<I64>>/I64` (de `q=[3]`), sin E0103 (r y q quedan resueltos: `r=[3]`, y q tiene elemento Array<Array<I64>>).
- Gemelo: `let r=[3];let q=[3];let t=(if p then q[x] else (if p then [r] else r));` -> `E1303 [r] I64/Array<Array<I64>>` (el gemelo da el C en otra hoja porque q[x] ya es I64; no oraculo).
- Base: E0103 en los `[]`.
- ADR: §4.1.2.2 (E0103 solo si "ninguna restriccion compatible resuelve"), §4.1.2.1 (atomicidad: la igualdad recursiva r vs [r] rechazada no debe dejar alfa_r=Array<alfa_r>), 4.7/4.1.2.4 (union anidada: una sola E1303 en la primera continuacion posterior incompatible, que es `r`).
- **Veredicto: DEFECTO DE PRODUCTO** (nota: aqui la salida esperada se deduce por consistencia con los controles; el E0103 espurio es el defecto firme).

### F14
`var q=[];var s=[];let t=(if p then [s] else q);let a2=(if p then q else br);q=[3];s=[3];`
- MAIN: `E0103@q`, `E0103@s`, `E1303 q Array<Array<I64>>/Array<I64>`.
- Con `s=[3];q=[3];`: `E1303 br Array<Array<I64>>/Array<Bool>` y `E0102 '3' Array<I64>/I64` (sin E0103). Sin `a2` (ambos ordenes): solo `E0102 '3'`.
- Gemelo: `let q=[3];let s=[3];let t=(if p then [s] else q);let a2=(if p then q else br);` -> E1303 q Array<Array<I64>>/Array<I64>, E1303 br Array<I64>/Array<Bool>.
- Base: E0103 x2.
- ADR: §4.1.2.2, §4.1.2.3, 4.1.2.4 (una E1303 por union, en la primera continuacion posterior incompatible; en `a2` esa es `br`, no `q`).
- **Veredicto: DEFECTO DE PRODUCTO.**

### F15
`var r=[];var s=[];let a=(if p then s else x);let b=(if p then s else r[x]);r=[true];s=[[3]];`
- MAIN: `E0103@r`, `E0103@s`, `E1303 x Array<Array<I64>>/I64`, `E1303 r[x] Array<Array<I64>>/Bool`.
- Con `s=[[3]];r=[true];`: `E1303 x ...`, `E0102 'true' Array<Array<I64>>/Bool` (sin E0103; el conflicto cae en la hoja posterior `true`, como §4.1.2.3 exige). Sin `a`: solo `E0102 'true'` en ambos ordenes.
- Gemelo: `let r=[true];let s=[[3]];...` -> E1303 x, E1303 `r[x]`.
- Base: E0103 x3 en el diferido.
- ADR: §4.1.2.3 ("Una hoja posterior incompatible se denuncia en esa hoja ... nunca se traslada ese conflicto al acceso anterior"); MAIN traslada el conflicto a `r[x]` (acceso anterior) y emite E0103.
- **Veredicto: DEFECTO DE PRODUCTO.**

### F16 (variante: una sola variable, sin permutacion; desencadenante = asignacion tardia rechazada)
`var q=[];let z:Array<Array<I64>>=[q[1],[q]][0];q=ar;`
- MAIN: `E0103@q []`, `E0103@[q[1],[q]]` (el literal completo).
- Controles: sin `q=ar;` -> `E0102 [q] Array<Array<I64>>/Array<Array<Array<Array<I64>>>>` y sin E0103; `let z:Array<Array<Array<I64>>>=q;q=ar;` -> `E0102 ar Array<Array<Array<I64>>>/Array<I64>`; `q=[[[1]]]` -> `E0102 q Array<I64>/Array<Array<Array<I64>>>`. Salida esperada: los dos C `E0102 [q]` y `E0102 ar` (el contexto anotado ya fija el elemento de q a Array<Array<Array<I64>>> y una asignacion rechazada no lo deshace: §4.1.2.2 "sin sustituir el tipo establecido"); ninguna variable queda sin constructor.
- Gemelo: `let q=ar;let z:Array<Array<I64>>=[q[1],[q]][0];` -> `E0102 [q] I64/Array<Array<I64>>` (el gemelo adelanta ar, que es incompatible con el contexto determinado; no oraculo).
- Base: E0103 en `var q=[]`.
- ADR: §4.1.2.2 (asignacion rechazada), N(b), N(h).
- **Veredicto: DEFECTO DE PRODUCTO** (E0103 espurio sobre `[]` y sobre el literal; pierde los dos C).

### F17
`var q=[];var s=[];let a1=(if p then {value: q;} else {value: (if s then {value: [s];} else {value: 0;});});q=[true];s=[ar];`
- MAIN: `E0103@q`, `E0103@s`, `E0102 s Bool/Array<Array<I64>>`, `E1303 [s] Array<Bool>/Array<Array<Array<I64>>>`.
- Con `s=[ar];q=[true];`: `E0102 s`, `E1303 0 Array<Array<Array<I64>>>/I64`, `E0102 true Array<Array<I64>>/Bool` (sin E0103). Sin los bloques `{value:}` (`if p then q else (if s then [s] else 0)`) no aparece E0103 en ningun orden.
- Gemelo: `let q=[true];let s=[ar];...` -> E0102 s, E1303 `[s]` Array<Bool>/... (C iguales, sin E0103).
- Base: E0103 x3.
- ADR: §4.1.2.4 / §4.7: la union se compara con la referencia de su primera continuacion en orden fuente (`q`, con elemento pendiente); la discrepancia es `0` (la primera incompatible posterior), y `q=[true]` es la restriccion POSTERIOR incompatible. MAIN usa `q=[true]` (posterior) para fijar la referencia antes de la union y desplaza la E1303 a `[s]`.
- **Veredicto: DEFECTO DE PRODUCTO.**

### F18
`var q=[];var s=[];let a=(if p then q[1] else s);let a1=Len((if p then x else q));q=[3];s=[3];`
- MAIN: `E0103@q`, `E0103@s`, `E1303 s I64/Array<I64>`, `E0102/not_indexable (if..) domain:array/I64`.
- Con `s=[3];q=[3];`: `E0102/not_indexable`, `E1303 q I64/Array<Array<I64>>`, `E0102 '3' Array<I64>/I64` (sin E0103). Sin `a1`: solo `E0102 '3'`.
- Gemelo: `let q=[3];let s=[3];...` -> `E1303 s I64/Array<I64>`, `not_indexable`, `E1303 q I64/Array<I64>` (el gemelo fija q[1]=I64 de antemano; como en F16, no es oraculo).
- Base: E0103 x3.
- ADR: la presencia de E0103 sobre `[]` resuelto contradice §4.1.2.2 sea cual sea la lectura del C exacto; el conjunto exacto de C esperado (E1303 en `s` con I64, o E0102 en `3` con Array<I64>) depende de si `q[1]` fija el elemento a una variable (ADR §4.1.2.3: raiz variable se relaciona sin fingir contexto concreto, que es la lectura de la salida "buena") o a un tipo ya determinado; el ADR §4.1.2.3 apunta a la primera. 
- **Veredicto: DEFECTO DE PRODUCTO** por los dos E0103 espurios (firme); la cuestion de en que hoja cae el C en la permutacion que NO trae E0103 no genera laguna adicional porque MAIN ya produce la salida coherente con §4.1.2.3 en ese orden.

### F19
`var q=[];var s=[];let t=(s < q[0]);let u=(if p then s else true)[x];q=[3];s=[3];`
- MAIN: `E0103@q`, `E0103@s`, `E0102 q[0] Array<I64>/I64`, `E1303 true Array<I64>/Bool`.
- Con `s=[3];q=[3];`: `E0102 '<' I64/Array<I64>`, `E1303 true`, `E0102 '3' Array<I64>/I64` (sin E0103). Sin la sentencia `u`: solo `E0102 '<'` + `E0102 '3'` en ambos ordenes.
- Gemelo: `let q=[3];let s=[3];...` -> `E0102 q[0] Array<I64>/I64`, `E1303 true` (sin E0103).
- Base: E0103 x3.
- ADR: §4.1.2.2/§4.1.2.3 como arriba. Ademas el sitio del C cambia segun el orden (`q[0]` vs operador `<`), lo que incumple el determinismo de N(a)/N(e) en el sentido de que dos asignaciones a variables distintas, sin relacion fuente entre ellas mas alla de las ya fijadas por la union anterior, cambian el primario.
- **Veredicto: DEFECTO DE PRODUCTO.**

## Tabla resumen

| ID | idx | veredicto | familia | una linea |
|---|---|---|---|---|
| F10 | 183938 | DEFECTO DE PRODUCTO | E (E0103 espurio; union anidada + q[x]) | `E0103` sobre q y `[r]` resueltos por `q=[3];r=[3]`; falta el C de `q=[3]` |
| F11 | 59334 | DEFECTO DE PRODUCTO | E | `var r,q;let a=(if p then p else q);let c=[q,[r]];q=[3];r=[3];` E0103 x2 y C desplazado |
| F12 | 125160 | DEFECTO DE PRODUCTO | E | `[[q],r]` + `if p then p else r`: E0103 x2, C trasladado al acceso anterior |
| F13 | 226338 | DEFECTO DE PRODUCTO | E | `if p then p else r` + `if p then r else [q]`: E0103 x2, pierde E1303 de r |
| F14 | 86140 | DEFECTO DE PRODUCTO | E | `if p then [s] else q` + `if p then q else br`: E0103 x2, E1303 en q en vez de br |
| F15 | 144704 | DEFECTO DE PRODUCTO | E | E1303 previo sobre s + `r[x]`: E0103 x2, conflicto movido a `r[x]` |
| F16 | 285671 | DEFECTO DE PRODUCTO | E (asignacion rechazada) | `q=ar` rechazada tras contexto anotado deja `[]` y el literal en E0103; pierde 2 C |
| F17 | 61075 | DEFECTO DE PRODUCTO | E | `{value:}` anidados: E0103 x2, E1303 en `[s]` en vez de en `0` |
| F18 | 54958 | DEFECTO DE PRODUCTO | E | `q[1]`/`s` + `Len(if..)`: E0103 x2 espurios (orden cambia la salida) |
| F19 | 93000 | DEFECTO DE PRODUCTO | E | `s<q[0]` + indexacion de union: E0103 x2, primario `q[0]` vs `<` segun orden |

## Defectos de producto (para quien decide)

**D-E (un solo defecto estructural, 10 repros).** `NX-E0103` espurio sobre constructores Array cuyo elemento queda resuelto por una asignacion tardia compatible, y C (`NX-E1303`/`NX-E0102`) perdidos o trasladados a otra hoja, segun el orden de dos asignaciones a variables distintas. Desencadenante: un conflicto de forma previo sobre una variable pendiente (rechazo con tipo pendiente) seguido de una relacion entre dos variables pendientes; tambien una asignacion tardia rechazada contra un contexto anotado (F16).
- Repro minimo recomendado (el mas corto, con su par "bueno"):
  - Malo: `fn Main(){let p=true;var r=[];var q=[];let a=(if p then p else q);let c=[q,[r]];q=[3];r=[3];}` -> MAIN: `NX-E0103` (r `[]`), `NX-E0103` (q `[]`), `NX-E0102/type_mismatch` en `r` (esperado I64, encontrado Array<I64>).
  - Bueno (solo cambia el orden `r=[3];q=[3];`): `NX-E1303` en `q` de `a` (Bool/Array<Array<I64>>) y `NX-E0102` en `3` (Array<I64>/I64), ningun E0103.
  - Salida esperada para ambos ordenes: la del "bueno" (ADR §4.1.2.2: E0103 solo si "ninguna restriccion compatible" resuelve el elemento, aqui `q=[3]` y `r=[3]` lo resuelven; §4.1.2.3: el conflicto se denuncia en la hoja posterior, no se traslada al acceso anterior).
- Otros repros minimos: F16 `var q=[];let z:Array<Array<I64>>=[q[1],[q]][0];q=ar;` (E0103 x2; esperado `E0102 [q]` y `E0102 ar`); F10 `var r=[];var q=[];let t=(if p then q[x] else (if p then [r] else r));r=[3];q=[3];`; F15 `var r=[];var s=[];let a=(if p then s else x);let b=(if p then s else r[x]);r=[true];s=[[3]];`; F17 y F19 en sus secciones.
- Es candidato a una sola causa raiz (consistente con §4.1.2.1: "un conflicto no conserva unificaciones parciales": una igualdad rechazada con tipo pendiente deja variables ligadas o desligadas indebidamente). No se ha leido el codigo del solucionador; esa hipotesis de causa no esta comprobada.

## Lagunas del ADR

Ninguna laguna independiente para estos diez casos. Observaciones menores, no bloqueantes:
1. F18: el ADR no fija con un ejemplo literal en que hoja cae el C cuando `q[1]` (raiz variable) se une con una continuacion `s` y la asignacion posterior fija q; la lectura de §4.1.2.3 ("una raiz todavia variable se relaciona con el resultado sintetizado, sin fingir un contexto concreto") es la que sigue la salida buena. Si quien decide quiere blindarla, conviene un ejemplo de ADR.
2. Los gemelos concretos de F10, F16, F18 no son oraculo por §4.1.2.7 (tipo adelantado que cambia el contexto); eso es DIFERENCIA LEGITIMA en la comparacion con el gemelo, pero no explica los E0103 del diferido.

## Resumen breve

Los 10 casos (F10-F19) son un unico defecto de producto (familia E): el diferido emite `NX-E0103` sobre `[]`/`[v]` que si se resuelven con una asignacion tardia compatible y mueve o pierde los C reales dependiendo del orden de dos asignaciones tardias; cada caso se reduce a 2-5 sentencias y su par bueno/malo se verifica cambiando solo el orden o quitando el conflicto previo. Reglas citadas: ADR §4.1.2.2 (cierre de E0103), §4.1.2.1 (igualdad compuesta atomica), §4.1.2.3 (conflicto en la hoja posterior), N(a)/(b)/(h). No hay lagunas ni diferencias legitimas que expliquen las salidas de MAIN en estos casos; los gemelos concretos no son oraculo por §4.1.2.7.
