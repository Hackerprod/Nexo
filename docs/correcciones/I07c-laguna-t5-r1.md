# I07c — laguna «T5»: resultado nominal de una operación con un operando fallido (propuesta A, r1)

Documento de propuesta (solo documentación). Veredicto de A: **T5 no es una laguna del ADR: F0-0002 fija la respuesta.** No se propone una
regla de relleno; se propone una **aclaración derivada** opcional (§3) y reclasificar T5 en `docs/validation/i07c-s5c/orden-no-fijado.md`
de «decisión no fijada» a «fijado por el ADR» (§5). `main` y V6 ya la cumplen (§1.3): no hay defecto de producción que corregir por T5.

## 1. Derivación

### 1.1 Lo que el ADR decide (F0-0002 §4.1.1, §4.1.2 cl. 4/5, §13.9, §14.10)
- §4.1.1, tabla: «Resultado contaminado: **Depende de un error previo.** Conserva la causa y suprime solo diagnósticos derivados; **no
  demuestra igualdad** ni borra errores independientes.» Una operación cuyo operando falló (o está contaminado) depende de un error previo.
- Cl. 5: «Fallar una obligación contamina el resultado de la expresión fallida […] **Una operación con operandos incompatibles no ofrece su
  tipo nominal de resultado** para denunciar después la anotación, condición, igualdad o llamada que la contiene.» La lista es de consumidores
  de ejemplo: el criterio es que el resultado no ofrece su nominal; no es una lista cerrada.
- Cl. 5, 2.º párrafo, sobre elementos y uniones: «una unión errónea como primer elemento de un array no transmite su testigo parcial […]; esta
  [la referencia] procede del primer elemento **no divergente con evidencia válida** (§4.1.2.4). **No se atribuye al hijo erróneo un valor ni se
  utiliza el tipo nominal de su operación fallida.**» Y: «El testigo parcial […] procede de una continuación válida independiente, no del tipo
  o valor de la expresión fallida.»
- Cl. 4 (elemento de array y hoja de `if`): sin contexto externo, «el primer elemento no divergente y **con evidencia válida** establece la
  referencia»; en un `if`, «la primera continuación **con evidencia válida** establece la referencia de la unión». Un hijo contaminado no tiene
  evidencia válida (§4.1.1).
- Cl. 4, `<`/`==`/`AssertEq` con un operando contaminado: «No se atribuye un tipo común **ni un resultado válido a la operación**»; para
  `AssertEq`: «no se atribuye a la llamada un resultado válido». Cl. 5, RD21: «No se usa el supuesto resultado Bool de esa operación inválida
  para aceptar el programa ni para reemplazar el C»; un NX-E0102 adicional contra la anotación es V (presencia I).
- El «cierre de dominio indeciso» (cl. 2) ofrece el nominal solo «siempre que **no haya fallado una obligación de la operación ni un hijo
  necesario** para obtener su resultado»; distingue *indeciso* («no es un error ni contaminación») de *fallido*. Por tanto lo único que sostiene
  el nominal es la ausencia de fallo, no la ausencia de tipo del operando.

### 1.2 Lo que el texto calla, y por qué no abre una segunda lectura
Cl. 5 (1.er párrafo) no nombra elementos de array ni hojas de unión; las nombran el 2.º párrafo y cl. 4 (arriba). Tampoco distingue «operando
incompatible» de «operando contaminado sin incompatibilidad propia» (p. ej. `x+1` con `let x=true+1;`, o un operando unión con error interno),
pero §4.1.1 («depende de un error previo») y cl. 4 (`<`/`==`) lo cubren; el único sentido de «hijo necesario» compatible con RD65/RD110 es
cualquier operando evaluado de la operación.

### 1.3 Comportamiento actual (`check --format json --profile f0-0002-b-v2`; `main` y V6 idénticos en todos)
Fuentes ASCII sin BOM ni LF final; offsets en bytes `[ini,fin)`; todos `rejected`.
| fuente | salida de `main` y V6 | lectura B (nominal ofrecido) añadiría |
|---|---|---|
| `fn Main(){let a=[true+1,true];}` | único NX-E0102/type_mismatch `[17,21)` (`true`), `I64`/`Bool`, relación `found_type=[17,21)` | NX-E0102 en el 2.º `true` `[22,26)` (referencia `I64`) |
| `fn Main(){let a=[true,true+1];}` | único NX-E0102 `[22,26)` | NX-E0102 sobre `true+1` |
| `fn Main(){let c=true;let a=if c then true+1 else true;}` | único NX-E0102 `[37,41)` | NX-E1303 sobre el `true` final |
| `fn Main(){let x=true+1;let a=[x+1,true];}` | único NX-E0102 `[16,20)` | NX-E0102 en `true` final |
| `fn Main(){let a=[[true]<1,1];}` | único NX-E0102 `[24,25)` (`1`), `Bool`/`I64` | NX-E0102 en el 2.º `1` `[26,27)` |
| `fn Main(){let c=true;let a=[if c then true+1 else 1,true];}` | único NX-E0102 `[38,42)`; el `true` final fija la referencia | NX-E0102 en el `true` final |
Los demás consumidores (anotación `a:Bool=true+1`, argumento `F(true+1)` con `F(p:view Bool)`, `return`, selector `a[true+1]`, `AssertEq(true+1,true)`,
elemento de `Array<Bool>` con contexto externo) emiten solo el C del operando fallido, sin denuncia de la operación.

## 2. Lecturas razonables y efecto
- **A (sin evidencia; contaminado).** Una operación con un operando fallido o contaminado no tiene tipo nominal ni evidencia para ningún
  consumidor (elemento, hoja, argumento, contexto externo). Solo mantiene sus C internos. Efecto: la tabla 1.3, columna central.
- **B (nominal por operador).** El resultado es `I64` (`+`,`-`) o `Bool` (`<`,`==`) y alimenta restricciones. Efecto: las diferencias de la columna
  derecha; mueve presencia de C (el elemento posterior pasa a ser C frente a una referencia fabricada por un error previo) y, con contexto
  externo, convierte la hoja en C en vez de V (RD21 dice V y presencia I), lo que contradice cl. 5 y RD21.
- **C (híbrida por operador: `==`/`<` sí ofrecen Bool, `+`/`-` no).** Sin apoyo en el texto: cl. 4 niega el resultado válido a `<`/`==` con operando
  contaminado y cl. 5 no distingue operador. Descartada.
Solo A es coherente con §4.1.1 (contaminación y no cascada), N(d) (un error no fabrica sitios causales ajenos) y con las filas RD21, RD65, RD110.

## 3. Recomendación
No relleno. Si el orquestador desea fijarlo en el addendum para evitar reaperturas, el texto es una **aclaración derivada** (no relleno), análoga
a la de `+`/`-` ya adoptada:

> **Resultado de una operación con un operando fallido o contaminado (aclaración derivada de §4.1.1 y de las cláusulas 4 y 5 de §4.1.2).** El
> resultado de `+`, `-`, `<`, `==`, `AssertEq`, de un acceso o de una llamada con un operando que ha fallado una obligación de la operación, o
> que está contaminado, está contaminado: carece de tipo nominal y de evidencia válida para cualquier consumidor, también si el consumidor es un
> elemento de array, una hoja de `if` o de bloque con valor, un argumento, un selector, un `return` o un contexto externo. Un elemento o una
> continuación así no establece la referencia de homogeneidad ni la de la unión, no es incompatible con ella y no recibe NX-E1303 ni
> NX-E0102 por ese resultado; sus errores internos independientes y los de los demás elementos se conservan. La indecisión de un dominio no es
> fallo: el nominal solo se ofrece cuando ni la operación ni un operando necesario han fallado (cierre de dominio indeciso).

Válido (`checked`, sin diagnósticos): `fn Main(){let a=[1+2,3];}` y `fn Main(){var q=[];let a=[q[0]<1,true];q=[1];}`. Contraste por indecisión
(el nominal sí se ofrece): `fn Main(){var q=[];let a=[q[0]<1,1];q=[1];}` da único NX-E0102 `[33,34)` (`1`), `Bool`/`I64`. Inválido: `fn Main(){let a=[true+1,true];}` → único NX-E0102/type_mismatch `[17,21)`, `expected_type=I64`,
`found_type=Bool`, relación `found_type=[17,21)`, sin relación `expected_type`, sin diagnóstico en `[22,26)`.
Corregido (`1+1`): `fn Main(){let a=[1+1,true];}` falla por homogeneidad con su propio primario (`true` `[21,25)`, `I64`/`Bool`): control sin operación fallida.

## 4. Salidas protegidas
La regla no cambia ninguna. `check` de `main` y de V6 sobre 369 fuentes (las variantes de las 152 RD, `corpus/{pass,fail,objetivo}`,
`tests/fixtures/i07c*`; todas con el perfil `f0-0002-b-v2`): 0 diferencias entre ambos; `python -B scripts/i07c_rd_conformidad.py --verify`
(`base-release.exe` y `cand-release.exe`): 152/152 pares check/run en ambos. Una barrida textual (operador `+ - < ==` y un NX-E0102 en el mismo
segmento) marca 88 candidatos, entre ellos RD21/22/76/77, RD65, RD94(a-d), RD95(a-d), RD105, RD110, RD126, todos ya conformes con A. TD/DG y tests
existentes no fijan una salida B. La barrida no es una prueba de ausencia: A es lo que producen hoy ambos binarios.

## 5. Efectos y riesgos
- Casos del oráculo S5c que dejan de ser «T5 pendiente»: **O01** (casos.json; la primera componente `[2][0] + false[x]` de `a1` no establece la
  referencia). El oráculo (`scripts/i07c_s5c_oraculo.py`, rama de `+`/`-` «cl.5») ya aplica A; no añade `T5` a `no_fijado` y solo la tabla de
  `orden-no-fijado.md` lo cuenta: se reclasifica a «fijado, ADR cl.5 y §4.1.1» y se retira el aviso «puede mover el primario».
- Cualquier salida futura que añada NX-E0102/NX-E1303 por el nominal de una operación fallida es un defecto de núcleo con esperado transcrito
  (la tabla 1.3), no una decisión.
- Riesgo: la lectura «hijo necesario» de §1.2 es la del cierre indeciso; si se quisiera que un operando *contaminado* no impida ofrecer `I64`
  a `x+1`, habría que reabrir RD94/RD95 por Flujo A (hoy no hay base textual).
