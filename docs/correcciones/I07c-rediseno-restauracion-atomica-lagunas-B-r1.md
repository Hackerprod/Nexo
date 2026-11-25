# I07c — propuestas de precisión normativa tras B r1

Estado: histórico de la propuesta r1, rechazada por B r2 en 1a/1b/1c. Las decisiones del coordinador D1–D4 y el contrato `I07c-rediseno-restauracion-atomica-R6-r2.md` la sustituyen; no tiene PASS de B.
Fuentes ASCII, sin BOM ni salto final; los spans son bytes `[inicio, fin)`.

## Condición fallida en una continuación posterior

**Corrección r2:** el texto de abajo solo rige **sin tipo esperado externo determinado**. Con tipo esperado externo determinado, cada hoja se comprueba contra ese contexto (§4.1.2.4, N(d)); una condición fallida no borra esos C. Las igualdades internas válidas de la `if` sí se comprometen aunque su resultado exterior esté contaminado. X-02 (67 B) exige E0102 en `0` `[45,46)` y `true` `[52,56)`, sin E1303 sobre `3`; SB (`ar` como condición) exige solo E0102 en `ar` `[72,74)`, sin E0103 espurio.

`F0-0002-lagunas.md`, T5, ya decide que el resultado de una operación con condición inválida queda contaminado. `F0-0002-second-slice.md` §4.1.2.4 exige contrastar las continuaciones válidas de una unión sin tipo esperado; §4.1.2.5 conserva los hijos independientes y suprime la comparación que necesita un resultado contaminado. Falta decir expresamente que una `if` anidada en una continuación **posterior** no aporta ninguna de sus hojas a la unión exterior cuando falla su condición, aunque sus hojas se comprueban entre sí y conservan errores propios.

Texto propuesto: «Si la condición de una `if` falla, sus continuaciones se comprueban para sus restricciones y diagnósticos independientes, incluida su propia comparación cuando hay evidencia válida. Su resultado no ofrece tipo ni testigo a un consumidor exterior. Si esta `if` ocupa una continuación posterior de otra unión, ninguna de sus hojas se aplana como continuación de la unión exterior; no se genera por ellas `NX-E1303` exterior ni se comprometen sus igualdades en el ancla exterior. Una condición válida restaura la regla ordinaria de aplanado de §4.1.2.4. La decisión depende de la validez del resultado que ofrece la operación, no de la forma o contención de spans.»

- Positivo válido, 63 bytes: `fn Main(){let x=(if true then 1 else (if true then 2 else 3));}`. Esperado: aceptación, `diagnostics=[]`.
- Negativo aislado, 60 bytes: `fn Main(){let x=(if true then 1 else (if 0 then 2 else 3));}`. Esperado: solo `NX-E0102/type_mismatch` sobre `0` `[41,42)`; ninguna `NX-E1303` exterior.
- Repro de B, 99 bytes: `fn Main(){let aar=[[1]];var r=[];let a0=[(if true then r[1] else (if 0 then r else aar)),1];r=aar;}`. Esperado de B: solo `NX-E0102` `[69,70)`. Su forma concreta se conserva como prueba de la regla, además del par aislado.

## Igualdad recursiva y constructor propio pendiente

**Decisión D2 del coordinador, r2:** se retira la propuesta de primario `[66,67)` de esta sección. El esperado normativo de §§4.1.2.2/13.17 es E0103 sobre el constructor `[v]` `[60,63)`; `main` tiene un defecto de primario. P3-06 y P3-14 son negativos adicionales. Los párrafos históricos siguientes explican la propuesta rechazada, no son instrucciones para C.

`F0-0002-second-slice.md` §4.1.2.2 contiene dos mandatos que no seleccionan de forma inequívoca el primario del siguiente caso: una igualdad recursiva sin solución finita conserva el conflicto en su sitio; cada constructor con elemento irresuelto conserva su `NX-E0103` sobre el constructor completo. Para `var r=[];r=[r];`, el ADR da `NX-E0103` en el `[]` inicial. Con un valor divergente inicial, el único constructor de valor es posterior y puede parecer dueño de una segunda obligación. B deriva `[v]` `[60,63)`, mientras `main` y el brief §3.4 piden el RHS `a` `[66,67)`.

Decisión propuesta: **el conflicto recursivo se denuncia en la igualdad que cierra el ciclo**, con primario en su RHS `[66,67)`; el constructor `[v]` no añade otro `NX-E0103` por la misma dependencia recursiva rechazada. Esta precisión requiere enmienda explícita del ADR; no se infiere del mero parecido con `var r=[];r=[r];`. La obligación propia del constructor sigue activa si permanece irresuelta por una causa independiente después de retirar la igualdad recursiva.

Texto propuesto: «Cuando una igualdad posterior crea el primer ciclo sin solución finita de una variable inicialmente sin constructor, se emite `NX-E0103` en el operando RHS de esa igualdad. Un constructor que participa únicamente en ese ciclo rechazado no duplica la denuncia por la misma dependencia. Si antes del ciclo existía un constructor irresuelto independiente, conserva su diagnóstico en el span completo de ese constructor, como en `var r=[];r=[r];`. Se conservan los diagnósticos de otros constructores independientes.»

- Positivo válido, 71 bytes: `fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=[1];}`. Esperado: aceptación, `diagnostics=[]`.
- Negativo, 69 bytes: `fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=a;}`. Esperado: solo `NX-E0103` `[66,67)`; ninguna segunda denuncia `[60,63)`.

El texto es una propuesta de arquitectura. Hasta que B valide la lectura, la fila recursiva y el test `cyclic_constructor_errors_keep_inference_code_at_each_site` permanecen INCONCLUSOS; el prototipo no acredita por sí mismo una enmienda normativa.

## Valor parcial de un brazo `match` cuya unión interior falló

**Decisión D1 del coordinador, r2:** se retira RM-M3; `match` conserva todas las 54 y 103 filas declaradas de `main`, incluidos siete R-C con E1303. No se abre aquí una regla nueva de brazo. Cualquier delta exige otro Flujo A. El repro histórico siguiente documenta el motivo de la retirada, no una laguna que R6 pueda decidir.

`F0-0003-third-slice.md` §3.3 exige comprobar todos los brazos y formar el tipo del `match` expresión; `F0-0002-second-slice.md` §4.1.2.5 distingue el testigo parcial de una primera continuación de un resultado ofrecido a un consumidor. Ninguna de esas cláusulas decide si el testigo parcial de una `if` fallida dentro del valor de un brazo se ofrece a la unión de brazos de `match`, ni si una incompatibilidad ya denunciada en la `if` puede volver a denunciarse al comparar los brazos. Esa decisión afecta las 52 filas declaradas de `match-conservacion-ext.json`; RM-M3 modifica 51 respecto de `main`. Las 51 filas de conservación siguen idénticas y son condición de cualquier resolución.

Repro de 181 bytes, ASCII sin LF: `fn Main(){let o:Option<I64>=Some(1);let p=true;let ar=[1];var q=[];var s=[];let a0=match move o{Some(n)=>{value: (if p then 1 else (if p then s else true));},None=>{value: true;}};}`. En perfil `f0-0003-b-v1`, `main` emite `NX-E0103` `[64,66)` y `[73,75)`, además de `NX-E1303` `[149,153)`; R5/RM-M3 emite solo los dos `NX-E0103`. Ambos rechazan. El span `[149,153)` es `true` del segundo brazo, cuyo contraste con el resultado parcial de la `if` del primero queda sin regla explícita en §3.3. Ejemplo positivo de 116 bytes: `fn Main(){let o:Option<I64>=Some(1);let a=match move o{Some(n)=>{value:(if true then 1 else 2);},None=>{value:3;}};}`; esperado `diagnostics=[]` en `check` y `run`.

Decisión normativa solicitada al Flujo A: precisar si la reserva de un error de la `if` interior deja un testigo parcial para el contraste de brazos de `match`, bajo qué dependencia puede usarse, y si un brazo con resultado contaminado se omite entero como consumidor. La respuesta debe fijar para el negativo el conjunto exacto de códigos y spans y debe aplicarse a las 52 filas declaradas, no solo al segundo brazo. Hasta esa decisión, las 51 diferencias de R5 son **INCONCLUSAS**, no mejoras, y RM-M3 no puede tratarse como pieza validada de una sola regla.
