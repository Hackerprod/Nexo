# Aclaración derivada: resultado de un literal de array con un elemento erróneo (texto final para `F0-0002-lagunas.md`)

Propuesta de A, para adopción por el coordinador por el mismo cauce que T5 (`lagunas.md`, «Resultado de una operación con un operando fallido o contaminado»). Decisión del coordinador (revertible): lectura **N**. Base `origin/main` = `6c39ce0`; informe de B: `I07c-S5c-d3-diseno-B-r1.md`; diseño: `I07c-S5c-d3-diseno.md`. Este documento no modifica `docs/adr/`; el bloque de la sección 1 es el texto exacto que se pegaría.

## 1. Texto para `lagunas.md`

```
<!-- Adopción (coordinador): ACLARACIÓN DERIVADA (no relleno) sobre el resultado de un literal de array con un elemento erróneo.
     Origen: docs/correcciones/I07c-laguna-literal-array-error-r1.md. Deja por escrito lo que §4.1.1 y las cláusulas 4 y 5 de §4.1.2
     fijan para el literal heterogéneo (l.361) y lo extiende, como laguna decidida, a los casos vecinos. -->

## Resultado de un literal de array con un elemento erróneo (aclaración derivada de §4.1.1 y de las cláusulas 4 y 5 de §4.1.2)

Un literal de array sin contexto externo Array en el que algún elemento incumple la referencia de homogeneidad, tiene un defecto
interno, es una lectura contaminada (§4.1.1) o es una unión con una continuación errónea no ofrece tipo a ningún consumidor, tampoco
si el consumidor es una unión `if`: no ofrece testigo parcial, no establece la referencia de la unión, no es incompatible con ella y
no recibe NX-E1303 por ese resultado. Sus errores internos independientes y los de las demás continuaciones se conservan, y las
continuaciones restantes se contrastan entre sí como si ese literal no estuviera. El testigo parcial de §4.1.2.5 procede solo de
uniones y solo lo compara otra unión. Un literal sin defecto es una continuación con tipo y se contrasta. Sin contexto Array
determinado, el primer elemento válido sigue fijando la referencia de homogeneidad del literal y cada elemento posterior
incompatible conserva su NX-E0102 (l.353); lo que el literal no hace es ofrecer a otra expresión el tipo `Array<T>` así obtenido.
```

Los ejemplos siguientes se añaden a la misma sección. Σ = `fn Main(){let p=true;`; Π = `fn Main(){let p=true;let ar=[1];let aar=[[1]];`. Fuentes ASCII sin LF final; spans en bytes `[inicio,fin)` desde cero; `check` de `main`, perfil `f0-0002-b-v2`.

```
Válido (el literal es homogéneo y se contrasta): Π+`let a0=(if p then 1 else [ar,ar]);}` (rechazado por una sola NX-E1303)
primario `[71,78)` (`[ar,ar]`), `expected_type=I64`, `found_type=Array<Array<I64>>`.

Inválido: Π+`let a0=(if p then 1 else [ar,aar]);}`: único NX-E0102/type_mismatch, primario `[75,78)` (`aar`),
`expected_type=Array<I64>`, `found_type=Array<Array<I64>>`, relaciones `expected_type=[72,74)` y `found_type=[75,78)`.
Sin NX-E1303 sobre `[71,79)`: el literal heterogéneo no ofrece tipo a la unión.

Contraste (primera continuación): Π+`let a0=(if p then [ar,aar] else 1);}`: único NX-E0102 `[68,71)`; el literal no establece la
referencia, así que `1` no recibe NX-E1303.

Discriminador (la referencia es la primera continuación con evidencia válida):
Π+`let a0=(if p then [ar,aar] else (if p then 1 else true));}`: NX-E0102 `[68,71)` y una sola NX-E1303 `[96,100)` (`true`,
`expected_type=I64`, `found_type=Bool`), origen esperado `1`. Una NX-E1303 sobre `1` o sobre el literal incumple esta regla.
```

Variantes de la misma regla (cada una: válido que se contrasta / inválido sin NX-E1303 sobre el literal / discriminador):

| Variante | Válido | Inválido | Discriminador |
|---|---|---|---|
| Homogéneo con defecto interno | Σ+`let a0=(if p then 1 else [true,true]);}`: NX-E1303 `[46,57)`, `I64`/`Array<Bool>` | Σ+`let a0=(if p then 1 else [true+1,true]);}`: único NX-E0102 `[47,51)`, `I64`/`Bool` | Σ+`let a0=(if p then [true+1,true] else (if p then 1 else true));}`: NX-E0102 `[40,44)` y NX-E1303 `[76,80)`, `I64`/`Bool` |
| Lectura contaminada (T5/§4.1.1) | Σ+`let x=1+1;let a0=(if p then 1 else [x,x]);}`: NX-E1303 `[56,61)`, `I64`/`Array<I64>` | Σ+`let x=true+1;let a0=(if p then 1 else [x,true]);}`: único NX-E0102 `[27,31)`, `I64`/`Bool` | Σ+`let x=true+1;let a0=(if p then [x,true] else (if p then 1 else true));}`: NX-E0102 `[27,31)` y NX-E1303 `[84,88)` |
| Elemento unión errónea (RD123) | Σ+`let a0=(if p then 1 else [(if p then [1] else [2]),[1]]);}`: NX-E1303 `[46,76)`, `I64`/`Array<Array<I64>>` | Σ+`let a0=(if p then 1 else [(if p then 1 else true),[1]]);}`: único NX-E1303 `[65,69)` (la de la unión interior), ninguna sobre el literal | Σ+`let a0=(if p then [(if p then 1 else true),[1]] else (if p then 1 else true));}`: NX-E1303 `[58,62)` y `[92,96)`, `I64`/`Bool` |

En todos los casos `check` y `run` coinciden en rechazo y diagnósticos (`run` no ejecuta primitivas).

## 2. Fundamento (no se pega en `lagunas.md`)

**Qué fija el ADR.** l.361: «Un array heterogéneo y una unión cuyas continuaciones válidas discrepan no ofrecen un tipo único a su consumidor». l.363: el testigo parcial «solo permite comparar continuaciones en una unión exterior»; «No se atribuye al hijo erróneo un valor»; una unión errónea «no transmite su testigo parcial al array». l.353: una NX-E1303 compara una continuación **con evidencia válida** con la referencia; las incompatibilidades de hojas posteriores no reciben otra. RD03/RD04: «la heterogeneidad no proporciona un array válido a la condición». RD110: una continuación `true+1` no se compara; solo vale el testigo de la válida. RD123: la unión errónea no establece la homogeneidad del array. T5: el resultado de una operación con operando fallido o contaminado «no es incompatible con [la referencia] y no recibe NX-E1303».

**Qué no fija.** l.361 nombra solo el literal heterogéneo. El homogéneo con defecto interno (`[true+1,true]`), el de lectura contaminada (`[x,true]` con `x=true+1`) y el de elemento unión errónea (RD123) como hoja de una unión no tienen fila RD; T5 no nombra el constructor de array. Es una laguna con dos lecturas razonables y decide el coordinador (nivel N: afecta a la emisión de una C).

**Argumento a favor de la lectura contraria (P).** l.363 dice que una unión que discrepa «conserva únicamente su referencia procedente de la primera continuación válida como testigo parcial para una unión exterior»; por analogía, un literal cuya homogeneidad falla conservaría `Array<T>` (de su primer elemento válido, que l.353 sí mantiene como referencia) y lo ofrecería como testigo a la unión. **Valoración:** (i) l.363 limita el testigo a uniones y lo llama excepción expresa; ampliarlo a otro constructor es crear una regla, y RD123 muestra que el ADR no traslada testigos entre constructores distintos; (ii) l.361 pone al array heterogéneo junto a la unión discrepante y niega a ambos el tipo para su consumidor: la unión recibe la excepción escrita, el array no; (iii) con error diferido (variable pendiente) P exigiría decidir qué elemento es la referencia, regla no escrita; (iv) P es coherente con RD03/RD04 solo si el array deja de ser «no válido» para las uniones pero lo sigue siendo para condición, `==`, `+`, indexado, anotación y asignación, que es donde `main` ya aplica N. P es defendible por analogía; se elige N por menor cambio (no toca `main`), coherencia con RD03/RD04/RD110/RD123/T5 y un único criterio («sin evidencia válida no hay NX-E1303») para todos los consumidores.

## 3. Efecto medido

- **Aceptación y salida de `main`: 0 cambios.** La aclaración describe el comportamiento vigente y no requiere cambio de `src/`; los ejemplos de la sección 1 se comprobaron en `main` (`check`, byte a byte).
- `main` = N, medido: corpus 1 (24.000): de 149 discrepancias entre una lectura P y una N, `main` coincide con N en 140 y con P en 0; familia nueva `0xD3A11` (2.024): 1.316 de 1.539 con N, 0 con P; B, familia `0xB7C3E1` (1.949; SHA-256 `8fc5b165…392cb6`): 1.352 de 1.697 con N, 0 con P; contextos sin unión (anotación escalar, asignación, elemento de otro array, `==`, `Print`, condición, `+`, indexado) y envolventes `Some(…)` 80/80 y `Ok(…)` 252/252 iguales a la unión sin envolver.
- Los restantes de esas familias (6 en la de B, 9 en el corpus 1) no son de esta regla: son dos defectos reales de `main` que **van a d2 (R1)** con su repro como test y no se tratan aquí: (a) dos NX-E1303 en una misma unión, `fn Main(){let ar=[1];let q=[ar];let a0=(if true then q else (if true then ar else (if true then 2 else ar)));}` da `[74,76)` y `[96,97)` (l.353 exige una); (b) NX-E1303 de menos con variable pendiente, `fn Main(){let p=true;let aar=[[1]];var w=[];let a0=(if p then (if p then [w,aar] else 2) else 1);}` da solo `[86,87)` y debe dar también `[94,95)`.
- Ya fuera de esta aclaración, el oráculo `scripts/i07c_s5c_oraculo.py` necesita su propia corrección (diseño d3, commits d3-2a/2b); no forma parte del texto normativo.

## 4. Adopción

Pegar el bloque de la sección 1 en `lagunas.md` tras la aclaración T5; tests de regresión (uno por fila de la sección 1, con su positivo) en `tests/`, registrados en `tests/suite.rs`; la puerta completa los ejecuta. Revertible: quitar la sección y los tests devuelve el estado anterior sin cambios de código.
