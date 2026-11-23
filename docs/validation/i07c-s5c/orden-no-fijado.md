# I07c-S5c - patrones de orden (y de seleccion) que el ADR no determina

Autor: oraculo independiente (E). Fuente unica: `docs/adr/F0-0002-second-slice.md` (4.1, 4.1.1, 4.1.2 clausulas 1-7, 4.7, 13.9) y el addendum
`docs/adr/F0-0002-lagunas.md`. El oraculo es `scripts/i07c_s5c_oraculo.py`. Las citas son por seccion y clausula, no por numero de linea.

## Hallazgo principal: el orden de la lista esta fijado por 13.9; no hay patron de orden de la familia que el ADR deje abierto

El plan (§6 «Orden») preveia una lista de patrones de orden no fijados. Revisadas todas las clausulas, el resultado es **la lista nominal
de patrones de orden no fijados esta vacia para la familia, salvo el unico caso N-0 de la tabla siguiente, que el ADR excluye de una salida conforme**.
La razon es una sola regla: §13.9, vineta «Los resultados de una misma etapa se ordenan por `file`, `primary_span.start`, `primary_span.end`,
`code` y `subcause`, tratando `null` como cadena vacia para el desempate. No se ordenan por el texto traducido» (repetida para las cuotas en
§11: «se ordena por `primary_span.start` con los desempates de §13.9»). Todos los diagnosticos de la familia (`NX-E0103` operacion
`types.infer`, `NX-E1303` y `NX-E0102` operacion `types.check`) son de la etapa `types` y de un solo `file`; las clausulas 1-7 de 4.1.2 fijan
el **recorrido de comprobacion** (que hoja es «la primera» incompatible, que constraint fija un componente), y §13.9 («en estados convergidos, la
emision diferida aplica esta relacion causal antes de ordenar la lista») fija luego el orden de salida. Una emision diferida (E1303 completada al
cierre por una asignacion posterior) no cambia su posicion: se ordena por su primario.

Consecuencia para el diferencial: **la comparacion de orden es estricta para toda la familia**. Una diferencia de orden entre dos listas con el mismo
conjunto de diagnosticos es DEFECTO (la «cuarta clase informativa» del plan queda vacia). Si el verificador emite en otro orden, es un
defecto de §13.9 (determinismo, N(e)).

### Patrones revisados (lista nominal completa)

| Id | Patron (ejemplo) | Clausulas revisadas | Estado |
|---|---|---|---|
| O-1 | Diagnosticos con primarios de inicio distinto, descubiertos en orden distinto del fuente: E1303 diferida de la union y E0102 de una asignacion posterior; asignacion **antes** de la union (`q=[[ar]];let a0=(...)` con E0102 en una hoja posterior) | 13.9 vineta de orden, 4.1.2 cl.1 (orden de comprobacion), 13.9 «emision diferida ... antes de ordenar» | Fijado: por `start`; el instante de descubrimiento no cuenta |
| O-2 | Mismo inicio, fin distinto: hoja `r` frente a `r[0]`; `if` anidado `[60,78)` frente a su primera hoja `[60,..)`; literal `[r]` frente a su primer elemento solo si empezaran igual (no ocurre: `[` precede) | 13.9 (`primary_span.end`) | Fijado: por `end` ascendente |
| O-3 | Mismo primario, codigo distinto: `NX-E0103` sobre un `[]` que es a la vez hoja con otra denuncia | 13.9 (`code`); comparacion de cadenas ASCII: `NX-E0102` < `NX-E0103` < `NX-E1303` | Fijado |
| O-4 | Mismo primario y codigo, subcausa distinta (`type_mismatch` frente a `not_indexable`) | 13.9 (`subcause`, `null` = ``) | Fijado; ademas N(d) y cl.6 («como maximo un diagnostico de tipos por ocurrencia que actua como operando») lo excluyen para C |
| O-5 | Diagnosticos de funciones distintas en un mismo `file` | 13.9: la clave no incluye la funcion; «no obliga a abortar otras funciones independientes de la misma etapa» | Fijado por la clave; no afecta a un programa de una funcion |
| O-6 | Orden entre hijos al elegir *cual* de dos C conserva una misma ocurrencia | 4.1.1 («los empates siguen el orden de hijos»), 4.1.2 cl.6 («se conserva el primero segun el orden y la prioridad causal») | Fijado (es seleccion, no orden de lista) |
| O-7 | Etapas distintas (`types` frente a `ownership`/`effects`) | 13.9 orden de etapas y barrera | Fijado; fuera de la familia (solo `types`) |
| O-8 | Orden de `related_spans`/`cause_chain` dentro de un diagnostico | no es orden de diagnosticos; I(b) fija el contenido, no el orden interno | Fuera del alcance del oraculo (se compara por rol) |
| **N-0** | **Dos diagnosticos con la clave `(start, end, code, subcause)` identica** | 13.9 no da mas desempate; N(d), N(e) («tampoco estos se duplican por sitio») y cl.6 lo excluyen para C y D/V | **Unico patron que 13.9 no ordena. Excluido por el ADR de una salida conforme**: si el verificador lo emite, es un defecto de cardinalidad (duplicado por sitio), no una diferencia de orden. El diferencial no lo trata como orden informativo. |

Revisados y por que ninguna clausula fija N-0 como orden: 13.9 solo define la clave de cuatro campos; las clausulas 4.1.2 cl.1/cl.6 y
N(d)/N(e) prohiben el duplicado por sitio, de modo que no hay nada que ordenar.

## Puntos que el ADR no fija y que NO son de orden (el oraculo los marca, no los decide en silencio)

Aparecen en `no_fijado` de la salida JSON y en `d_optional`; el diferencial los compara como conjunto o los registra, y no bloquean por si solos.

| Id | Punto | Clausulas revisadas | Que hace el oraculo |
|---|---|---|---|
| T1 | Granularidad del testigo (relacion `expected_type`) de una variable que se fija por la igualdad de un **elemento hoja de un literal de array** que recibe la referencia de homogeneidad. Ejemplo B_qr: `let c=[q,[r]];` fija el elemento de `q` con la lectura `r` `[76,77)` (hoja) o con el elemento `[r]` `[75,78)` | 4.1.2 cl.4 («cada elemento de un literal de array que recibe un tipo esperado o una referencia» es hoja; «se comprueba la ocurrencia del valor final»), cl.6/l.371 (el origen es «la restriccion posterior que lo fijo»; para asignaciones es el RHS completo segun el ejemplo de `r=[(r==r)==true]`) | Elige la hoja `[76,77)` (coincide con la matriz). Si el ADR quisiera el elemento completo, esa relacion es I salvo fuera de RD (N(g)) |
| T2 | Origen de un esperado **compuesto** sin origen propio (p. ej. `Array<alfa>` de un `[]` cuyo `alfa` se fijo despues) en la relacion `expected_type` de una E0102 de asignacion | cl.6/l.371 «su origen concreto, siguiendo aliases hasta el inicializador o la restriccion posterior que lo fijo» | Desciende al componente: el origen es el de la variable ligada (operando de su primera igualdad compatible); si no hay, el inicializador completamente concreto; si no, se omite la relacion |
| T3 | Presencia de las hojas **D** (hojas incompatibles adicionales de una restriccion con referencia derivada de otra hoja de la misma union dentro de un array: segunda hoja de un `if` elemento de array) | 4.1.2 cl.4 («la implementacion de referencia tambien denuncia las hojas D ... solo su presencia es I»), cl.5 | No las emite en `diagnostics`; las lista en `d_optional`. Si el verificador las emite, no es defecto |
| T4 | Presencia de los **V** (derivados de una expresion contaminada o de una base no indexable) | 4.1.2 N(j), I(a), cl.5; 13.9 | No emite V; si el verificador los emite con codigo/subcausa de la operacion y primario existente, no es defecto (I) |
| T5 | Resultado nominal de una operacion cuyo operando esta contaminado (`+`, `<`, `==`): el oraculo lo trata como **sin evidencia** (4.1.2 cl.5: «una operacion con operandos incompatibles no ofrece su tipo nominal»), de modo que un elemento de array con esa operacion no establece la referencia. Es la lectura que reproduce O01 | 4.1.2 cl.5 y 4.1.2.3 «cierre de dominio indeciso» (que si ofrece el nominal cuando no ha fallado un hijo necesario) | Sin evidencia cuando algun operando falla; se registra como decision, no como laguna cerrada |

Ninguno de T1-T5 cambia la aceptacion ni el codigo/primario de un C; T1 y T2 solo mueven la relacion `expected_type` (I salvo en C de programas fuera
de RD, N(g)); T5 puede mover el primario de un C si el verificador la lee de otro modo, por lo que en caso de diferencia el diferencial debe
clasificarla contra esta tabla (y B decide si requiere Flujo A).
