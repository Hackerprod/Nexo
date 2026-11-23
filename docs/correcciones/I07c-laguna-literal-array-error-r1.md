# Literal de array con un elemento erróneo como hoja de una unión (texto para `F0-0002-lagunas.md`; r2 tras B)

Propuesta de A. Decisión del coordinador (revertible): lectura **N**. Base `origin/main` = `6c39ce0`; diseño `I07c-S5c-d3-diseno.md`. Este documento no modifica `docs/adr/`; los bloques de la sección 1 se pegarían tras la aclaración T5. Todo lo citado de `main` está medido con `check --format json` (perfil `f0-0002-b-v2`; `f0-0003-b-v1` donde hay `Some` o `match`). Σ = `fn Main(){let p=true;`; Π = `fn Main(){let p=true;let ar=[1];let aar=[[1]];`; Σ′ = `fn Main(){let p=true;let o:Option<I64>=Some(1);`. Fuentes ASCII sin LF final; spans en bytes `[inicio,fin)`.

## 1. Texto

### 1.1 Aclaración derivada (solo el literal heterogéneo)

```
<!-- Adopción (coordinador): ACLARACIÓN DERIVADA (no relleno) sobre el resultado de un literal de array heterogéneo. Origen:
     docs/correcciones/I07c-laguna-literal-array-error-r1.md. Solo deja por escrito lo que §4.1.1 y las cláusulas 4 y 5 de §4.1.2 fijan. -->

## Resultado de un literal de array heterogéneo (aclaración derivada de §4.1.1 y de las cláusulas 4 y 5 de §4.1.2)

Un literal de array sin contexto externo Array en el que un elemento posterior incumple la referencia de homogeneidad no ofrece un
tipo único a ningún consumidor, tampoco a una unión `if`: no ofrece testigo parcial, no establece la referencia de la unión, no es
incompatible con ella y no recibe NX-E1303. Su primer elemento válido sigue fijando la referencia del propio literal y cada elemento
incompatible conserva su NX-E0102 (§4.1.2.4); sus errores internos y los de las demás continuaciones se conservan, y estas se
contrastan entre sí como si el literal no estuviera. El testigo parcial de §4.1.2.5 procede solo de uniones y solo lo compara otra
unión.
```

Ejemplos (se añaden a la sección). **Ejemplo con una sola NX-E1303** (literal sin defecto, se contrasta): Π+`let a0=(if p then 1 else [ar,ar]);}` → NX-E1303 `[71,78)`, `expected_type=I64`, `found_type=Array<Array<I64>>`. **Inválido:** Π+`let a0=(if p then 1 else [ar,aar]);}` → único NX-E0102/type_mismatch `[75,78)` (`aar`), `Array<I64>`/`Array<Array<I64>>`, relaciones `expected_type=[72,74)` y `found_type=[75,78)`; sin NX-E1303 sobre `[71,79)`. **Contraste** (literal en la primera continuación): Π+`let a0=(if p then [ar,aar] else 1);}` → único NX-E0102 `[68,71)`; `1` no recibe NX-E1303. **Discriminador:** Π+`let a0=(if p then [ar,aar] else (if p then 1 else true));}` → NX-E0102 `[68,71)` y una sola NX-E1303 `[96,100)` (`true`, `I64`/`Bool`, origen esperado `1`).

### 1.2 Relleno de laguna, revertible (tres casos que el ADR no decide)

```
<!-- Adopción (coordinador, relleno de laguna REVERSIBLE): literal de array con defecto interno, con lectura contaminada o con un
     elemento unión errónea como hoja de una unión. Origen: docs/correcciones/I07c-laguna-literal-array-error-r1.md. Fija el
     comportamiento de `main`; no es una consecuencia directa de una cláusula. -->

## Literal de array con un elemento erróneo como continuación de una unión (relleno de laguna, revertible)

La regla de «Resultado de un literal de array heterogéneo» rige también cuando el literal es homogéneo pero algún elemento tiene un
defecto interno, es una lectura contaminada (§4.1.1) o es una unión con una continuación errónea: el literal no ofrece tipo ni
testigo parcial a una unión `if`, no es incompatible con su referencia y no recibe NX-E1303; sus errores internos se conservan. Rige
igual para el valor de un brazo de `match` con valor; cómo un `match`, o un brazo que es otro `match`, ofrece su testigo a otra unión
lo fija la sección «`match` con valor: unión de los valores de sus brazos y testigo parcial». Un literal sin defecto se contrasta.
```

| Caso | Con una sola NX-E1303 (literal sin defecto) | Inválido (sin NX-E1303 sobre el literal) | Discriminador |
|---|---|---|---|
| Defecto interno | Σ+`let a0=(if p then 1 else [true,true]);}` → `[46,57)`, `I64`/`Array<Bool>` | Σ+`let a0=(if p then 1 else [true+1,true]);}` → único NX-E0102 `[47,51)`, `I64`/`Bool` | Σ+`let a0=(if p then [true+1,true] else (if p then 1 else true));}` → NX-E0102 `[40,44)` y NX-E1303 `[76,80)` |
| Lectura contaminada | Σ+`let x=1+1;let a0=(if p then 1 else [x,x]);}` → `[56,61)`, `I64`/`Array<I64>` | Σ+`let x=true+1;let a0=(if p then 1 else [x,true]);}` → único NX-E0102 `[27,31)` | Σ+`let x=true+1;let a0=(if p then [x,true] else (if p then 1 else true));}` → NX-E0102 `[27,31)` y NX-E1303 `[84,88)` |

Nota de la tercera fila (fuentes completas): con una sola NX-E1303: Σ+`let a0=(if p then 1 else [(if p then [1] else [2]),[1]]);}` → `[46,76)`, `I64`/`Array<Array<I64>>`. Inválido: Σ+`let a0=(if p then 1 else [(if p then 1 else true),[1]]);}` → único NX-E1303 `[65,69)` (la de la unión interior), ninguna sobre el literal. Discriminador: Σ+`let a0=(if p then [(if p then 1 else true),[1]] else (if p then 1 else true));}` → NX-E1303 `[58,62)` y `[92,96)`, `I64`/`Bool`.

**`match`** (Σ′, `f0-0003-b-v1`). Brazo literal erróneo: Σ′+`let a0=match move o{Some(n)=>{value: [1,true];},None=>{value: 2;}};}` (115 B) → solo NX-E0102 `[87,91)`; el literal sin defecto se contrasta y fija la referencia: Σ′+`let a0=match move o{Some(n)=>{value: [1,2];},None=>{value: 2;}};}` (112 B) → NX-E1303 `[106,107)` (`2`, `Array<I64>`/`I64`).

## 2. Fundamento (no se pega)

**Qué fija el ADR:** solo el heterogéneo: §4.1.2.5 «Un array heterogéneo y una unión cuyas continuaciones válidas discrepan no ofrecen un tipo único a su consumidor»; el testigo parcial «solo permite comparar continuaciones en una unión exterior» y «no se atribuye al hijo erróneo un valor»; RD03/RD04 (la heterogeneidad no da un array válido a la condición), RD110 (una continuación errónea no se compara) y RD123 (una unión errónea no transmite testigo a un array); §4.1.2.4: una NX-E1303 compara una continuación con evidencia válida. T5 cubre operaciones con operando fallido, no el constructor de array: por eso 1.1 es derivada y 1.2 es relleno. **Argumento P** (§4.1.2.5 conserva la referencia de una unión discrepante como testigo hacia otra unión; por analogía un literal que falla conservaría `Array<T>` de su primer elemento válido): valoración: (i) el testigo es una excepción expresa de uniones y RD123 muestra que no se traslada entre constructores; (ii) el array heterogéneo comparte frase con la unión discrepante pero solo esta recibe la excepción; (iii) con error diferido P exigiría elegir una referencia no escrita; (iv) P cambiaría ≥149/24.000 salidas de `main`. P es defendible por analogía en 1.2; se elige N por menor cambio y por un único criterio («sin evidencia válida no hay NX-E1303»).

## 3. Efecto medido y límites de la afirmación «`main` = N»

- **0 cambios de aceptación y de salida de `main`** (no se toca `src/`).
- `main` aplica N en uniones **planas** (sin `if` anidado): con y sin envoltura `Some(…)` (p. ej. `(if p then Some(2) else [1,true])` solo NX-E0102 `[55,59)`), y en contextos sin unión (anotación escalar, asignación, elemento, `==`, `Print`, condición, `+`, indexado; B). Corpus 1: de 149 discrepancias entre lecturas P y N, `main`=N en 140, =P en 0; familia `0xD3A11` 1.316/1.539 y `0xB7C3E1` 1.352/1.697 (=P en 0). El «`Some` 80/80, `Ok` 252/252» de B vale para esa familia; **no lo reproduzco** para `Ok` (sin anotación da NX-E0113) ni es cierto con `if` anidado (abajo).
- **Defectos de `main` con constructor envolvente y unión anidada, distintos de la regla N** (barrido aproximado mío: 84 pares `Some`/sin `Some` de uniones anidadas, sin E0103/E0113, comparando los textos de los primarios de NX-E1303: **20 difieren, 6 de 36 sin hoja errónea y 14 de 48 con ella**; el gemelo sin `Some` da la hoja correcta). Corte: **d2/R1** (testigo parcial/ancla con constructor), no d3:
  1. *Primario en toda la unión anidada:* 91 B `fn Main(){let p=true;let a0=(if p then Some(2) else (if p then [1,true] else Some(true)));}` → `main` NX-E1303 `[53,87)` (`Option<I64>`/`Option<Bool>`); la regla da la hoja `[77,87)`; gemelo sin `Some`, 79 B, da `[71,75)`; con `Some([1,true])` (97 B) `[53,93)`.
  2. *NX-E1303 de menos:* 89 B `…(if p then Some(2) else (if p then true+1 else Some(true)));}` → solo NX-E0102 `[63,67)`; falta NX-E1303 sobre `Some(true)` `[75,85)` (gemelo sin `Some`, 77 B: `[69,73)`). Y 90 B `…(if p then (if p then Some(true) else Some(3)) else Some(2));}` → una NX-E1303 `[66,73)`; dos uniones incompatibles exigen dos (gemelo, 72 B: `[60,61)` y `[68,69)`).
  3. *`match` anidado con brazo literal erróneo:* Σ′+`let o2:Option<I64>=Some(2);let a0=match move o{Some(n)=>{value: 2;},None=>{value: match move o2{Some(m)=>{value: [1,true];},None=>{value: true;}};}};}` (197 B) → solo NX-E0102 `[163,167)`; sin el literal (`value: 3`) `main` da NX-E1303 `[178,182)` sobre `true`. Qué testigo ofrece el `match` anidado lo decide la laguna de `match` (M3; pieza d2-9): **no adjudicado aquí**.
- Los dos defectos previos de B (dos NX-E1303 en una misma unión; NX-E1303 de menos con `[w,aar]` pendiente) siguen asignados a d2/R1 con su repro como test.

## 4. Adopción

Pegar 1.1 y 1.2 (con sus ejemplos) tras T5; tests de regresión (uno por fila, con su positivo) en `tests/`, registrados en `tests/suite.rs`. Revertible: quitar 1.2 devuelve el estado anterior sin cambios de código.
