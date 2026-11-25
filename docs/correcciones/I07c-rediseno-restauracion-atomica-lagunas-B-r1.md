# I07c — propuestas de precisión normativa tras B r1

Estado: propuestas para el Flujo A; no modifican `docs/adr/` y no tienen PASS de B.
Fuentes ASCII, sin BOM ni salto final; los spans son bytes `[inicio, fin)`.

## Condición fallida en una continuación posterior

`F0-0002-lagunas.md`, T5, ya decide que el resultado de una operación con condición inválida queda contaminado. `F0-0002-second-slice.md` §4.1.2.4 exige contrastar las continuaciones válidas de una unión sin tipo esperado; §4.1.2.5 conserva los hijos independientes y suprime la comparación que necesita un resultado contaminado. Falta decir expresamente que una `if` anidada en una continuación **posterior** no aporta ninguna de sus hojas a la unión exterior cuando falla su condición, aunque sus hojas se comprueban entre sí y conservan errores propios.

Texto propuesto: «Si la condición de una `if` falla, sus continuaciones se comprueban para sus restricciones y diagnósticos independientes, incluida su propia comparación cuando hay evidencia válida. Su resultado no ofrece tipo ni testigo a un consumidor exterior. Si esta `if` ocupa una continuación posterior de otra unión, ninguna de sus hojas se aplana como continuación de la unión exterior; no se genera por ellas `NX-E1303` exterior ni se comprometen sus igualdades en el ancla exterior. Una condición válida restaura la regla ordinaria de aplanado de §4.1.2.4. La decisión depende de la validez del resultado que ofrece la operación, no de la forma o contención de spans.»

- Positivo válido, 63 bytes: `fn Main(){let x=(if true then 1 else (if true then 2 else 3));}`. Esperado: aceptación, `diagnostics=[]`.
- Negativo aislado, 60 bytes: `fn Main(){let x=(if true then 1 else (if 0 then 2 else 3));}`. Esperado: solo `NX-E0102/type_mismatch` sobre `0` `[41,42)`; ninguna `NX-E1303` exterior.
- Repro de B, 99 bytes: `fn Main(){let aar=[[1]];var r=[];let a0=[(if true then r[1] else (if 0 then r else aar)),1];r=aar;}`. Esperado de B: solo `NX-E0102` `[69,70)`. Su forma concreta se conserva como prueba de la regla, además del par aislado.

## Igualdad recursiva y constructor propio pendiente

`F0-0002-second-slice.md` §4.1.2.2 contiene dos mandatos que no seleccionan de forma inequívoca el primario del siguiente caso: una igualdad recursiva sin solución finita conserva el conflicto en su sitio; cada constructor con elemento irresuelto conserva su `NX-E0103` sobre el constructor completo. Para `var r=[];r=[r];`, el ADR da `NX-E0103` en el `[]` inicial. Con un valor divergente inicial, el único constructor de valor es posterior y puede parecer dueño de una segunda obligación. B deriva `[v]` `[60,63)`, mientras `main` y el brief §3.4 piden el RHS `a` `[66,67)`.

Decisión propuesta: **el conflicto recursivo se denuncia en la igualdad que cierra el ciclo**, con primario en su RHS `[66,67)`; el constructor `[v]` no añade otro `NX-E0103` por la misma dependencia recursiva rechazada. Esta precisión requiere enmienda explícita del ADR; no se infiere del mero parecido con `var r=[];r=[r];`. La obligación propia del constructor sigue activa si permanece irresuelta por una causa independiente después de retirar la igualdad recursiva.

Texto propuesto: «Cuando una igualdad posterior crea el primer ciclo sin solución finita de una variable inicialmente sin constructor, se emite `NX-E0103` en el operando RHS de esa igualdad. Un constructor que participa únicamente en ese ciclo rechazado no duplica la denuncia por la misma dependencia. Si antes del ciclo existía un constructor irresuelto independiente, conserva su diagnóstico en el span completo de ese constructor, como en `var r=[];r=[r];`. Se conservan los diagnósticos de otros constructores independientes.»

- Positivo válido, 71 bytes: `fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=[1];}`. Esperado: aceptación, `diagnostics=[]`.
- Negativo, 69 bytes: `fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=a;}`. Esperado: solo `NX-E0103` `[66,67)`; ninguna segunda denuncia `[60,63)`.

El texto es una propuesta de arquitectura. Hasta que B valide la lectura, la fila recursiva y el test `cyclic_constructor_errors_keep_inference_code_at_each_site` permanecen INCONCLUSOS; el prototipo no acredita por sí mismo una enmienda normativa.
