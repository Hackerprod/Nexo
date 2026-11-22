# Adjudicacion twin, lote adjA (F01-F09, todos `flagged`)

Binarios: MAIN = `release.exe` de main (perfil f0-0002-b-v2; repetido en f0-0003-b-v1 con el mismo resultado en los representantes), BASE = `nexo.exe` de b506d7d (solo texto).
Todos los programas de abajo se ejecutaron uno a uno con ambos CLIs (auxiliares `G0..H4` omitidas: no cambian nada; los casos F01..F09 completos tambien se ejecutaron con el preludio de variables `x,p,ar,br,aar` y dan lo que dice `reduce_show`).
Notacion de salida: `CODIGO/subcausa [inicio,fin) 'texto' esperado<-encontrado`. Los offsets son de la fuente minima escrita en una linea tal como se cita. `D` = diferido, `G` = gemelo concreto, `P[...]` = permutacion de las asignaciones tardias.

## 0. Reglas del ADR usadas (F0-0002-second-slice.md)

- §4.1.1: «la primera igualdad incompatible con las igualdades precedentes se denuncia en su operando ... "Primera" sigue el recorrido de comprobacion de §4.1.2.1».
- §4.1.2 N(a)-N(d), N(h): aceptacion; existencia/codigo/subcausa de cada C; span primario del C; un diagnostico por sitio; NX-E0103/0113 solo sobre constructores irresueltos.
- §4.1.2.1 (orden normativo = orden fuente; «Una igualdad entre tipos compuestos es atomica»).
- §4.1.2.2: «Las igualdades compatibles se acumulan ... Una igualdad incompatible se rechaza, sin sustituir el tipo establecido ... Se continua con los sitios independientes posteriores ... Nunca se usa una asignacion rechazada para cambiar el tipo de lecturas anteriores. Si al terminar la resolucion `expected_type` sigue pendiente o no se consigue evidencia concreta serializable para `found_type`, se omite ese conflicto solo si el componente pendiente emite NX-E0103/NX-E0113 al cierre ... Si ninguna restriccion compatible lo completa, se omite al terminar porque el constructor `[]` emite NX-E0103». Tambien: «Un conflicto de forma o una igualdad recursiva sin solucion finita ... Se conserva el conflicto en su sitio y se completan **sus** tipos con las igualdades compatibles posteriores».
- §4.1.2.4: union sin contexto externo: «la primera continuacion con evidencia valida establece la referencia ... La union emite una sola NX-E1303, sobre la primera continuacion posterior incompatible»; arrays: referencia = primer elemento con evidencia valida.
- §4.1.2.7 (gemelo con tipo conocido): el gemelo conserva los diagnosticos solo si «`T` es compatible con todas las igualdades aceptadas en el prefijo anterior a esa asignacion» y el adelanto no vuelve determinado un contexto externo; «Si una igualdad anterior fijo otro tipo, el diferido denuncia la restriccion posterior incompatible; el gemelo denuncia los usos incompatibles con su inicializador, sin exigir igualdad de cardinalidad ni de ubicacion entre ambos». Y: **«No existe una regla de sustituir retrospectivamente cada variable pendiente por el tipo escrito en su ultima asignacion.»**

## 1. Familia FAM-1: una asignacion tardia se aplica ANTES que una igualdad anterior en orden fuente, cuando hay un conflicto pendiente (F01, F02, F04, F05, F07, F09)

### Sintoma comun y causa observable
Con dos incognitas `r`,`q` (o `r`,`s`) cuyo tipo se relaciona por una igualdad anterior (`AssertEq(r[1],q)`, `[r,[q]]`, `[[q],r]`, union anidada, `[s,r[1]]`) y un **conflicto cuyo tipo esperado aun esta pendiente** (una union con una hoja incompatible, o un `if` sobre un array), el resultado de MAIN **depende del orden de las dos asignaciones tardias**:

- orden `r=[3];q=[3];` (r primero): MAIN se comporta como si `r=[3]` hubiera fijado `r: Array<I64>` ANTES que la igualdad previa; denuncia la igualdad previa (su operando `q`, `r[x]`, ...) y **pierde C**; ademas emite **NX-E0103 sobre dos `[]` que `q=[3]`/`s=[3]` resuelven** (el propio diagnostico lleva `found_type: Array<I64>` para `q`, es decir MAIN sabe que q esta resuelto; contradiccion interna);
- orden `q=[3];r=[3];` (q primero): MAIN da el resultado que exigen las reglas por orden fuente (conflicto de la union/condicion con tipos completados + NX-E0102 sobre el `3` de `r=[3]`).

Esto contradice §4.1.1 («primera igualdad incompatible con las precedentes»), §4.1.2.2 («Nunca se usa una asignacion rechazada para cambiar el tipo de lecturas anteriores»; E0103 solo «si ninguna restriccion compatible lo completa»), §4.1.2.7 («No existe una regla de sustituir retrospectivamente ...») y las obligaciones N(b)/N(c)/N(h). La aceptacion no cambia (todos los programas se rechazan en ambos ordenes), por lo que no es N(a); si cambian la existencia, el primario y el codigo de C (N(b),(c)), y hay NX-E0103 espurios (N(h)).

El gemelo concreto NO es oraculo en esta familia (§4.1.2.7, condicion 1): al adelantar `r: Array<I64>` el gemelo contradice la igualdad previa `α=Array<β>` que el diferido ya habia aceptado antes de la asignacion. Por eso la diferencia `diferido vs gemelo` en si es legitima; el defecto es que MAIN, en un orden de asignaciones, **imita al gemelo** (que es la sustitucion retrospectiva prohibida) y en el otro orden da lo correcto. La referencia correcta es la permutacion `q` primero (coincide con la salida que predicen las reglas, calculada abajo a mano).

Derivacion comun a mano (F02 como ejemplo): `r:Array<α>`, `q:Array<β>`; `AssertEq(r[1],q)` acepta `α=Array<β>` (compatible, primera en orden fuente). La union `(if true then r else ar)` compara `Array<Array<β>>` con `Array<I64>`: conflicto de forma, `expected_type` pendiente (β); `q=[3]` fija β=I64 y el conflicto se emite al cierre con `Array<Array<I64>> <- Array<I64>` en `ar`; `r=[3]`: `3` frente a `Array<β>` = NX-E0102 en `3`, esperado `Array<I64>`. Ningun `[]` queda irresuelto: NX-E0103 no procede.

### Representante minimo (F02)
```
D (r,q): fn Main(){let ar=[1];var r=[];var q=[];AssertEq(r[1],q);let z=(if true then r else ar);r=[3];q=[3];}
G      : fn Main(){let ar=[1];let r=[3];let q=[3];AssertEq(r[1],q);let z=(if true then r else ar);}
P[1,0] : fn Main(){let ar=[1];var r=[];var q=[];AssertEq(r[1],q);let z=(if true then r else ar);q=[3];r=[3];}
```
- MAIN D: E0103 [27,29) `[]`; E0103 [36,38) `[]`; E0102/type_mismatch [53,54) `q` I64<-Array<I64>.
- MAIN G: E0102/type_mismatch [55,56) `q` I64<-Array<I64>.
- MAIN P[1,0]: E1303 [83,85) `ar` Array<Array<I64>><-Array<I64>; E0102/type_mismatch [96,97) `3` Array<I64><-I64.
- BASE (cualquiera de los tres diferidos): E0103 [27,29) y E0103 [36,38) (la base no tenia inferencia diferida; el G da E0102 en `q`).
- Control: sin la union (`AssertEq(r[1],q);r=[3];q=[3];`) MAIN da E0102 en el `3` de `r=[3]` en ambos ordenes; con la union solo en un orden. Variando solo el orden de `r=[3];q=[3];`: el resultado cambia.

Veredicto F02: **DEFECTO DE PRODUCTO** (esperado: el de P[1,0], tambien para el orden r,q; sin NX-E0103).

### F01 (idx 202130)
```
D : fn Main(){var r=[];var q=[];let a0=[r,[q]];let a1=(if false then r else 0);r=[3];q=[3];}
G : fn Main(){let r=[3];let q=[3];let a0=[r,[q]];let a1=(if false then r else 0);}
P : ...;q=[3];r=[3];}
```
- MAIN D: E0103 [16,18), E0103 [25,27), E0102/type_mismatch [39,40) `q` I64<-Array<I64>.
- MAIN G: E0102 [41,42) `q` I64<-Array<I64>; E1303 [74,75) `0` Array<I64><-I64.
- MAIN P[1,0]: E1303 [72,73) `0` Array<Array<I64>><-I64; E0102/type_mismatch [84,85) `3` Array<I64><-I64.
- BASE D: E0103 [16,18), E0103 [25,27). BASE G: E0102 q + E1303 0 (igual que MAIN G).
- ADR: `[r,[q]]` fija `α=Array<β>` (primer elemento r = referencia; `[q]`: `Array<Array<β>>`), antes que `r=[3]` en orden fuente; la union da E1303 en `0` con esperado completado `Array<Array<I64>>` y `r=[3]` da E0102 en `3`. Es exactamente P[1,0]. Sin la union el caso es correcto en ambos ordenes.
- Veredicto: **DEFECTO DE PRODUCTO** (FAM-1). Diferencia con G legitima (§4.1.2.7 cond. 1); defecto = salida de D en orden (r,q).

### F04 (idx 18584)
```
D : fn Main(){let ar=[1];var r=[];var q=[];AssertEq((if true then r[0] else q),(if true then r else ar));r=[3];q=[3];}
G : fn Main(){let ar=[1];let r=[3];let q=[3];AssertEq((if true then r[0] else q),(if true then r else ar));}
P : ...;q=[3];r=[3];}
```
(el caso completo con `if ar then`, `if 1 then` da ademas E0102/type_mismatch en `ar` y en `1`, Bool<-Array<I64> / Bool<-I64, iguales en D, G y P; son sitios limpios y coinciden.)
- MAIN D: E0103 [27,29), E0103 [36,38), E1303 [72,73) `q` I64<-Array<I64>.
- MAIN G: E1303 [74,75) `q` I64<-Array<I64>.
- MAIN P[1,0]: E1303 [96,98) `ar` Array<Array<I64>><-Array<I64>; E0102/type_mismatch [110,111) `3` Array<I64><-I64.
- BASE D: E0103 [27,29), [36,38).
- ADR: en la union izquierda `r[0]` (α pendiente) y `q` (`Array<β>`) son compatibles en orden fuente (`α=Array<β>`); la union derecha `r`/`ar` es el conflicto (E1303 en `ar`); `r=[3]` choca con `Array<β>`. MAIN D coincide con G (E1303 en `q`) = sustitucion retrospectiva; el `extra E0103` marcado por el arnes es el sintoma, no la causa.
- Veredicto: **DEFECTO DE PRODUCTO** (FAM-1).

### F05 (idx 217284)
```
D : fn Main(){var r=[];var q=[];let u=(if true then 0 else r);let w=[[q],r];r=[3];q=[3];}
G : fn Main(){let r=[3];let q=[3];let u=(if true then 0 else r);let w=[[q],r];}
P : ...;q=[3];r=[3];}
```
(caso completo con `AssertEq(false,(if ar then 0 else r));AssertEq(x,[[q],r]);` da lo mismo mas E0102 `ar` Bool<-Array<I64>, igual en D/G/P.)
- MAIN D: E0103 [16,18), E0103 [25,27), E0102/type_mismatch [69,70) `r` Array<Array<I64>><-Array<I64>.
- MAIN G: E1303 [57,58) `r` I64<-Array<I64>; E0102/type_mismatch [71,72) `r` Array<Array<I64>><-Array<I64>.
- MAIN P[1,0]: E1303 [55,56) `r` I64<-Array<Array<I64>>; E0102/type_mismatch [81,82) `3` Array<I64><-I64.
- BASE D: E0103 [16,18), [25,27).
- ADR: la union `(if true then 0 else r)` da E1303 en `r` (esperado I64, `r:Array<α>`; tipos completados al cierre: `Array<Array<I64>>`); `[[q],r]` fija `α=Array<β>`; `r=[3]` choca en `3`. MAIN D pierde el E1303 de la union (C perdido, N(b)) y mueve el conflicto a `r` de `[[q],r]`.
- Los controles: solo la union (`var r=[];let u=(if true then 0 else r);r=[3];`) da E1303 en `r` correcto; solo el array (`let w=[[q],r];...`) da E0102 en `3` correcto en ambos ordenes. Falla solo la combinacion.
- Veredicto: **DEFECTO DE PRODUCTO** (FAM-1). Aqui se pierde un C (E1303 de la union).

### F07 (idx 5871)
```
D : fn Main(){var r=[];var s=[];let a1=(if true then (if true then s else 1) else r[1]);r=[3];s=[3];}
G : fn Main(){let r=[3];let s=[3];let a1=(if true then (if true then s else 1) else r[1]);}
P : ...;s=[3];r=[3];}
```
(el original lleva ademas `var q=[];G4([q]);`; `G4([q])` fija `q` por el tipo del parametro y no interviene. El original completo con el preludio de funciones, ejecutado desde `twin-cases\F07.txt`, da D = E0103 [455,457), E0103 [473,475), E1303 `1` [523,524) Array<I64><-I64, E1303 `r[1]` [531,535) Array<I64><-I64; G = los dos E1303 [526,527) y [534,538); igual que la forma minima.)
- MAIN D: E0103 [16,18), E0103 [25,27), E1303 [70,71) `1` Array<I64><-I64, E1303 [78,82) `r[1]` Array<I64><-I64.
- MAIN G: E1303 [72,73) `1` Array<I64><-I64; E1303 [80,84) `r[1]` Array<I64><-I64.
- MAIN P[s,r]: E1303 [70,71) `1` Array<I64><-I64; E0102/type_mismatch [93,94) `3` Array<I64><-I64.
- BASE D: E0103 [16,18), [25,27).
- ADR: la union interior da E1303 en `1` (esperado `Array<σ>`, completado `Array<I64>`); su primera continuacion valida es testigo parcial (§4.1.2.5) y la union exterior lo compara con `r[1]` (α pendiente, compatible: `α=Array<σ>`); `r=[3]` choca en `3`. MAIN D da un E1303 en `r[1]` que el orden fuente no justifica y dos E0103 espurios.
- Veredicto: **DEFECTO DE PRODUCTO** (FAM-1).

### F09 (idx 56146)
```
D : fn Main(){var r=[];var s=[];if [s,r[1]] {}let u=(if true then r else true);r=[3];s=[3];}
G : fn Main(){let r=[3];let s=[3];if [s,r[1]] {}let u=(if true then r else true);}
P : ...;s=[3];r=[3];}
```
(el caso completo del arnes con `if (if [s,r[x]] then r else (if x then r else true)[ar]) {}` da lo mismo en esencia.)
- MAIN D: E0103 [16,18), E0103 [25,27), E0102/type_mismatch [34,38) `r[1]` Array<I64><-I64.
- MAIN G: E0102/type_mismatch [36,40) `r[1]` Array<I64><-I64; E1303 [71,75) `true` Array<I64><-Bool.
- MAIN P[s,r]: E0102/type_mismatch [31,39) `[s,r[1]]` Bool<-Array<Array<I64>>; E1303 [69,73) `true` Array<Array<I64>><-Bool; E0102/type_mismatch [84,85) `3` Array<I64><-I64.
- BASE D: E0103 [16,18), [25,27).
- ADR: `[s,r[1]]` (referencia `Array<σ>`, `α=Array<σ>`) frente al contexto Bool de la condicion = C en el array completo (esperado Bool; encontrado `Array<Array<I64>>` tras completar σ), la union da E1303 en `true`, y `r=[3]` choca en `3`. MAIN D pierde **dos C** (el de la condicion y el E1303 de `true`) y mueve el conflicto a `r[1]`.
- Veredicto: **DEFECTO DE PRODUCTO** (FAM-1).

## 2. F06 (idx 98567): union anidada con igualdad recursiva pendiente (familia FAM-2, emparentada con FAM-1)

```
D : fn Main(){let ar=[1];var r=[];let a0=(if true then r[1] else (if false then r else ar));r=[3];}
G : fn Main(){let ar=[1];let r=[3];let a0=(if true then r[1] else (if false then r else ar));}
```
(el caso completo del arnes, con `{value: r;}`/`{value: ar;}` y `< 1`, da el mismo patron: D = E0103 [455,457)(r) + E1303 en `ar` I64<-Array<I64>; G = E1303 en `r` I64<-Array<I64>.)
- MAIN D: E0103 [27,29) `[]`; E1303 [83,85) `ar` I64<-Array<I64>.
- MAIN G: E1303 [77,78) `r` I64<-Array<I64>.
- BASE D: E0103 [27,29). BASE G: E1303 [77,78) `r`.
- ADR: la union exterior (sin contexto externo, `let a0=` sin anotacion) toma como referencia `r[1]` (tipo α); §4.1.2.4 aplana las continuaciones del `if` anidado en orden fuente: la primera es `r : Array<α>`, que frente a la referencia α es una igualdad recursiva (`α=Array<α>`). §4.1.2.2: «Una igualdad recursiva sin solucion finita ... Se conserva el conflicto en su sitio y se completan sus tipos con las igualdades compatibles posteriores». Es decir, el conflicto es la hoja `r` (la primera posterior incompatible, §4.1.2.4 «una sola NX-E1303, sobre la primera continuacion posterior incompatible»), como en G; no `ar`. El ejemplo `r=[r]` (E0103 solo) es el caso en que *ninguna* restriccion posterior completa α.
- Contradiccion interna de MAIN D, valida con cualquier lectura: emite NX-E0103 en `[]` (α irresuelto) y a la vez un E1303 con `expected_type=I64` (α resuelto a I64, por `r=[3]`). §4.1.2.2: E0103 solo si «ninguna restriccion compatible lo completa» (N(h)). Y el primario del E1303 (`ar`) no es la primera continuacion incompatible en orden fuente (N(c)).
- Pistas de la misma causa que FAM-1: variando solo la posicion de las hojas (`else (if false then ar else r)`) MAIN D da E0103 + E1303 en `ar` I64<-Array<I64>, mientras que por orden fuente `ar` fija `α=Array<I64>` y el conflicto seria E1303 en `r` + E0102 en el `3` (MAIN tambien aplica `r=[3]` antes). Sin la asignacion tardia (`...;}` sin `r=[3];`) MAIN da E1303 en `r` Array<I64><-Array<Array<I64>> **mas** un E0103 en `[]`, contradictorio tambien.
- Nota de incertidumbre: los campos exactos `expected_type/found_type` del E1303 correcto dependen de como se complete α (I64 por `r=[3]` o `Array<I64>` por la hoja `ar`); eso es columna I (§4.1.2.6). Lo normativo es el primario `r` (N(c)), la ausencia de E0103 en un `[]` completado, y que exista un C en la primera hoja incompatible.
- Veredicto: **DEFECTO DE PRODUCTO** (FAM-2: E0103 espurio + primario de E1303 movido a `ar`).

## 3. F03 (idx 92249): FAM-1 (variante con selector, sin NX-E0103)

```
D (r,q) : fn Main(){let p=true;let ar=[1];var r=[];var q=[];let a0=ar[r][[q,r]];let u=(if true then q else p);r=[3];q=[true];}
P (q,r) : ...;q=[true];r=[3];}
G       : fn Main(){let p=true;let ar=[1];let r=[3];let q=[true];let a0=ar[r][[q,r]];let u=(if true then q else p);}
Dsu     : fn Main(){let p=true;let ar=[1];var r=[];var q=[];let a0=ar[r][[q,r]];r=[3];q=[true];}   (sin la union)
```
(el caso completo del arnes lleva `if [(if aar then q else p)] {}` en lugar de la union simple; D/G/P dan los mismos C comunes: E0102 `r` del `ar[r]`, E0102 `aar` Bool<-Array<Array<I64>>, E1303 `p` Array<Bool><-Bool.)
- MAIN D (r,q): E0102/type_mismatch [60,61) `r` I64<-Array<I64>; E1303 [97,98) `p` Array<Bool><-Bool.
- MAIN G: E0102 [65,66) `r` I64<-Array<I64>; E1303 [102,103) `p` Array<Bool><-Bool.
- MAIN P (q,r): E0102 [60,61) `r` I64<-Array<Bool>; E0102 [63,68) `[q,r]` I64<-Array<Array<Bool>>; E1303 [97,98) `p` Array<Bool><-Bool; E0102/type_mismatch [112,113) `3` Bool<-I64.
- MAIN Dsu (sin union, r,q): E0102 [60,61) `r` I64<-Array<I64>; E0102 [79,83) `true` I64<-Bool. (sin union, q,r): E0102 `r` I64<-Array<Bool>; E0102 `3` Bool<-I64.
- BASE D/Dsu/P: E0103 en los dos `[]` (la base no tenia inferencia diferida). BASE G: E0102 `r` + E1303 `p` (igual que MAIN G).
- Analisis: `[q,r]` impone `α=β` (primer elemento `q` = referencia; `r: Array<α>` compatible mientras ambos pendientes), igualdad previa a ambas asignaciones. Si la igualdad vale, de las dos asignaciones incompatibles entre si la primera fija el tipo y la segunda es el conflicto (§4.1.2.2, §4.1.2.7): en (r,q) `r=[3]` fija α=β=I64, el C es el `true` de `q=[true]` (esperado I64, encontrado Bool) y la union `q`/`p` da E1303 en `p` con esperado completado `Array<I64>`; en (q,r) el C es el `3` y el E1303 de `p` es `Array<Bool>`.
- MAIN es coherente con esa lectura en (q,r) y sin la union (Dsu), pero en (r,q) con la union **pierde el C de `true`** y da el E1303 de `p` con esperado `Array<Bool>` (β=Bool a la vez que α=I64 en el `r` del `ar[r]`: ignora la igualdad `α=β`). El resultado de D depende, pues, de que exista el conflicto pendiente de la union y del orden de las asignaciones: mismo disparador que FAM-1 (igualdad previa + conflicto pendiente + orden de asignaciones). Si en cambio se sostuviera que `[q,r]` (selector de una base contaminada) no impone `α=β`, entonces MAIN (q,r) estaria mal (`E0102 r ...Array<Bool>` y `3` Bool<-I64 sobrarian). Con cualquiera de las dos lecturas MAIN es incoherente consigo mismo entre los ordenes, y con la union sustituida por un array llano (`let w=[q,r];` en vez de `a0`) MAIN es coherente en ambos ordenes (E1303 `p` con el tipo de la primera asignacion y conflicto en la segunda), lo que confirma que la incoherencia no es de la regla sino del tratamiento del conflicto pendiente.
- El gemelo no es oraculo (§4.1.2.7 cond. 1): `q=[true]` adelantado contradice la igualdad `α=β` aceptada en el prefijo.
- El E0102 sobre el selector `[q,r]` (I64<-Array<Array<Bool>>), presente solo en (q,r), es un derivado V: «El requisito `I64` del selector depende de que la base sea indexable» y la base `ar[r]` esta contaminada (§4.1.2.3, §4.1.2.5/RD84); su presencia es columna I (§4.1.2 I(a)): no bloquea.
- Veredicto: **DEFECTO DE PRODUCTO** (FAM-1; aqui el sintoma es C perdido y esperado inconsistente del E1303, sin NX-E0103).
- Observacion fuera de alcance (no adjudicada, identica en la BASE): en `fn Main(){let ar=[1];let a0=ar[true][[1,true]];}` el array heterogeneo del selector exterior no recibe diagnostico (MAIN y BASE: solo el del primer selector). §4.1.2.3 dice que «los errores internos de la expresion del selector no dependen de ello»; si se quiere exigirlo, es una cuestion del corpus concreto, no de la inferencia diferida. Afecta a la lectura de la igualdad `[q,r]` arriba.

## 4. F08 (idx 145869): `if r {}` con constructor `[DV]` sin resolver

```
D : fn Main(){var r=[];let a0=(if true then r[1] else [(if true then {return;} else {return;})]);if r {}r=[3];}
G : fn Main(){let r=[3];let a0=(if true then r[1] else [(if true then {return;} else {return;})]);if r {}}
```
- MAIN D: E0103 [50,91) `[(if true then {return;} else {return;})]`.
- MAIN G: E0103 [51,92) (el mismo constructor) y E0102/type_mismatch [97,98) `r` Bool<-Array<I64>.
- BASE D: E0103 [16,18) `[]` de `var r`. BASE G: E0102 [97,98) `r` (y sin E0103 del constructor `[DV]`, que la base no exigia).
- ADR: `r[1]` (α) y `[DV]` (`Array<δ>`) estan en la misma union: `α=Array<δ>` queda aceptado antes de `if r {}` y de `r=[3]`. `if r {}` es un conflicto de forma con `expected Bool` y `found Array<Array<δ>>`: δ es el elemento del `[DV]`. §4.1.2.2: «Si al terminar la resolucion ... no se consigue evidencia concreta serializable para `found_type`, se omite ese conflicto solo si el componente pendiente emite NX-E0103 ... al cierre»: δ emite NX-E0103 sobre `[DV]` (`[DV]` «sin otro contexto que determine el elemento» emite NX-E0103 sobre el constructor completo; N(h)). Tampoco se acepta el programa. `r=[3]` es otro conflicto con esperado pendiente (`Array<δ>`), omitido por la misma razon.
- Control que confirma que la regla se aplica y no es un parche: si el elemento se completa (`r=[[3]]`), δ=I64, no hay E0103 sobre `[DV]` y MAIN emite `E0102 [96,97) r Bool<-Array<Array<I64>>`: `var r=[];let a0=(if true then r[1] else [DV]);if r {}r=[[3]];}`. Con `[1]` en lugar de `[DV]` tambien emite ese E0102.
- El gemelo (`r: Array<I64>`) no es oraculo: contradice `α=Array<δ>` aceptado en el prefijo (§4.1.2.7 cond. 1).
- Veredicto: **DIFERENCIA LEGITIMA** (§4.1.2.2 omision + N(h); §4.1.2.7 cond. 1).

## 5. Tabla resumen

| ID | idx | veredicto | familia | una linea |
|---|---|---|---|---|
| F01 | 202130 | DEFECTO DE PRODUCTO | FAM-1 | `[r,[q]]` + union: orden (r,q) aplica `r=[3]` antes que `α=Array<β>`; E0103 espurios y C perdidos; (q,r) correcto |
| F02 | 223705 | DEFECTO DE PRODUCTO | FAM-1 | `AssertEq(r[1],q)` + union: mismo sintoma; MAIN D imita al gemelo (sustitucion retrospectiva prohibida) |
| F03 | 92249 | DEFECTO DE PRODUCTO | FAM-1 (variante) | con selector `[q,r]` + union: en (r,q) se pierde el C de `true` y el E1303 de `p` usa `Array<Bool>`; sin la union o en (q,r) es coherente |
| F04 | 18584 | DEFECTO DE PRODUCTO | FAM-1 | E1303 movido de `ar` a `q` + E0103 espurios segun el orden |
| F05 | 217284 | DEFECTO DE PRODUCTO | FAM-1 | se pierde el E1303 de la union; conflicto movido al array; E0103 espurios |
| F06 | 98567 | DEFECTO DE PRODUCTO | FAM-2 | E1303 en `ar` en vez de la primera hoja incompatible `r`; E0103 sobre `[]` resuelto |
| F07 | 5871 | DEFECTO DE PRODUCTO | FAM-1 | E1303 espurio en `r[1]` + E0103 espurios; (s,r) correcto |
| F08 | 145869 | DIFERENCIA LEGITIMA | N(h)/omision pendiente | `if r {}` omitido: `found_type` no serializable y `[DV]` emite E0103 |
| F09 | 56146 | DEFECTO DE PRODUCTO | FAM-1 | se pierden dos C (condicion Bool sobre el array y E1303 de `true`) en el orden (r,s) |

## 6. Defectos de producto para quien decide

**DP-A (FAM-1; F01, F02, F03, F04, F05, F07, F09).** Programa minimo y orden correcto:
```
fn Main(){let ar=[1];var r=[];var q=[];AssertEq(r[1],q);let z=(if true then r else ar);r=[3];q=[3];}
```
Salida de MAIN: `NX-E0103 [27,29)`, `NX-E0103 [36,38)`, `NX-E0102/type_mismatch [53,54) q I64<-Array<I64>`.
Esperado (§4.1.1, §4.1.2.1, §4.1.2.2, §4.1.2.7, N(b)(c)(h); coincide con la misma fuente con `q=[3];r=[3];` que MAIN resuelve bien): `NX-E1303 [83,85) ar Array<Array<I64>><-Array<I64>` y `NX-E0102/type_mismatch [96,97)`-equivalente en el `3` de `r=[3]` (esperado `Array<I64>`, encontrado `I64`); sin NX-E0103. Otros repros: F03-min (`let p=true;let ar=[1];var r=[];var q=[];let a0=ar[r][[q,r]];let u=(if true then q else p);r=[3];q=[true];`; falta `E0102 [..] true I64<-Bool`, que aparece al quitar la union o permutar), F05-min (`var r=[];var q=[];let u=(if true then 0 else r);let w=[[q],r];r=[3];q=[3];`; se pierde el E1303 de la union), F09-min (`var r=[];var s=[];if [s,r[1]] {}let u=(if true then r else true);r=[3];s=[3];`; se pierden E0102 de la condicion y E1303 de `true`).
Disparador observado: hace falta (i) una igualdad previa que relacione la forma de `r` con otra incognita y (ii) un conflicto previo con tipo esperado aun pendiente; con solo uno de los dos MAIN es correcto en ambos ordenes. El resultado depende del orden de dos asignaciones tardias que no interactuan entre si.
Las salidas de MAIN en el orden bueno coinciden con el calculo a mano en todos los casos; no hay discrepancia de aceptacion.

**DP-B (FAM-2; F06).**
```
fn Main(){let ar=[1];var r=[];let a0=(if true then r[1] else (if false then r else ar));r=[3];}
```
MAIN: `NX-E0103 [27,29)` y `NX-E1303 [83,85) ar I64<-Array<I64>`. Esperado: C en la primera hoja incompatible en orden fuente, `r` (`[77,78)`, como el gemelo y como `r[1] else r`, que MAIN resuelve bien: `NX-E1303 [50,51)`), sin NX-E0103 en un `[]` que `r=[3]` completa (§4.1.2.2, §4.1.2.4, N(b)(c)(h)). Posible causa comun con DP-A: el tratamiento de conflictos pendientes cuyo esperado depende de una asignacion posterior.

## 7. Lagunas del ADR

Ningun caso lo es. Puntos que podrian discutirse, sin bloquear mi veredicto:
1. F06: si el tipo pendiente de `r[1]` cuenta como «evidencia valida» que establece la referencia de la union (§4.1.2.4 lo admite explicitamente para arrays: «Una raiz todavia variable se relaciona con el resultado sintetizado»; para uniones solo por analogia). Con cualquier lectura MAIN D es contradictorio (E0103 + expected I64), de modo que el veredicto no depende de ello.
2. Observacion de F03 (heterogeneidad en el selector de una base contaminada): si se exige, es decision sobre el corpus concreto y afecta a la BASE por igual; no cambia el veredicto de F03 (incoherencia entre ordenes).

## 8. Verificacion

- Cada caso se ejecuto: (a) completo con el preludio de variables (`x,p,ar,br,aar`) y las funciones auxiliares via `twin-cases\<ID>.txt` (salidas de MAIN idénticas a las del `reduce_show` salvo offsets), (b) en su forma minima citada, con MAIN y con BASE, y (c) las permutaciones relevantes.
- Nota: sin las funciones auxiliares `G0..H4`, un programa que llame a `G4` da NX-E0101 (nombre no resuelto) en ambos CLIs; las formas minimas citadas no usan auxiliares.
- Auxiliares en `C:\nexo-work\tmp\twin-adj\adjA\` (`run.sh`, `fmt.py`, `t.n`). No se modifico nada del repositorio.
