# F0-0002 I07c — Adjudicación de la campaña `twin` de tipos diferidos

Estado: **borrador de adjudicación, sin PASS**. Un validador B debe confirmar los defectos contra el ADR antes de corregirlos (Flujo B). Evidencia reproducible en `docs/validation/i07c-twin/`.

## 1. Reproducción

- Arnés: `scripts/i07c_typed_campaign/main.rs` (modo `twin`), compilado sobre `main` `1bd010c` (`J16-B2-TWIN-HARNESS` queda cerrado: compila sin cambios de firma). Se añadió al arnés, como andamiaje, el listado de las `other_differences` (variable de entorno `LIST_OTHER`), que antes solo las contaba.
- Campaña de release: `twin 300000 10495373245766874321 4 3` (programas, semilla, hilos, profundidad): `flagged=19` y 8 `other_differences`, idéntica a `docs/validation/F0-0002-I10a-typed-campaign-results.json`. Salida completa: `i07c-twin/campana-release-300000.out`. La campaña de debug (100.000 programas, misma semilla) es un subconjunto: sus 10 señalados y sus 3 `other_differences` son exactamente los casos con índice menor que 100.000 (señalados 5871, 18584, 54958, 56146, 59334, 61075, 86140, 92249, 93000 y 98567; otros 29097, 39425 y 98859); el comportamiento de debug y de release es el mismo.
- Los 27 casos (19 + 8) están en `i07c-twin/casos.json` (`F01`–`F19` señalados, `O01`–`O08` otras diferencias) con su fuente reducida y las diferencias que reporta el arnés.
- Referencias comparadas: CLI de `main` (`1bd010c`, perfil `f0-0002-b-v2`) y CLI de la base `b506d7d` (anterior a la inferencia diferida de I07c: daba `NX-E0103` en todo `var x=[]` sin tipo inicial, por lo que no es oráculo para el diferido).
- Adjudicación: tres agentes independientes (informes `i07c-twin/adjudicacion-*.md`, instrucciones en `i07c-twin/instrucciones-adjudicacion.md`); cada caso se ejecutó con el binario de `main` sobre su fuente original, su gemelo concreto y sus permutaciones. Las conclusiones centrales (§3) las reprodujo además el coordinador del carril con el binario (`i07c-twin/repros/`).

## 2. Resultado por caso

| Veredicto | Casos | Recuento |
|---|---|---:|
| DEFECTO DE PRODUCTO | F01–F07, F09, F10–F19, O01–O08 | 26 |
| DIFERENCIA LEGÍTIMA | F08 | 1 |
| LAGUNA DEL ADR | ninguna que cambie un veredicto | 0 |

F08 es legítima: `if r {}` queda omitido porque su `found_type` contiene el elemento pendiente de `[DV]`, que emite `NX-E0103` (ADR §4.1.2.2, punto 2, y §4.2.2); con `r=[[3]]` el elemento se resuelve y `main` sí emite el `NX-E0102` de `if r {}`.

Familias (probablemente una causa de raíz común en la deferral de conflictos de forma sobre tipos pendientes; a confirmar en el plan de corrección):

- **FAM-E** (F01–F05, F07, F09–F19; 17 casos): una igualdad anterior relaciona incógnitas o fija un componente pendiente, y un conflicto con tipo esperado todavía pendiente (típicamente una unión con una hoja incompatible) precede a dos asignaciones tardías. `main` da resultados distintos según el orden de esas asignaciones: en un orden emite `NX-E0103` sobre un `[]` que una asignación posterior compatible resuelve y pierde o traslada diagnósticos reales de causa independiente (`NX-E1303`/`NX-E0102`); en el otro orden la salida es la que dan las reglas.
- **FAM-O** (O01–O08; 8 casos): `NX-E0103` sobre un `var x=[]` cuyo elemento sí queda resuelto por una asignación tardía compatible, cuando una unión o un literal de array sin contexto externo contrasta una referencia de elemento pendiente con una hoja posterior; en O01, O03, O07 y O08 además el `NX-E1303` se desplaza a otra hoja o cambia con el valor de la asignación tardía.
- **FAM-2** (F06): unión anidada con igualdad recursiva pendiente: `NX-E1303` en `ar` en vez de en la primera hoja incompatible (`r`), más un `NX-E0103` sobre un `[]` que `r=[3]` resuelve.

## 3. Repros mínimos verificados (binario de `main`, perfil `f0-0002-b-v2`)

Programas en `i07c-twin/repros/`; salida de `check --format json`.

1. `d.n` frente a `c.n` y `p.n` (FAM-O, O07/O03):
   - D (`d.n`): `fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));r=[3];}`
   - `main`: `NX-E0103` en el `[]` `[30,32)` y `NX-E1303` en `aar` `[74,77)` (esperado `Array<I64>`, hallado `Array<Array<I64>>`).
   - Gemelo concreto (`c.n`, `let r=[3];` antes) y permutación (`p.n`, `r=[3];` antes del uso): solo el `NX-E1303` en `aar`. Contraste sin anidar (`n.n`: `if true then aar else [r]`): aceptado.
   - Contradicción interna de la misma salida: el `expected_type` del `NX-E1303` ya se serializa completado (`Array<I64>`, resuelto por `r=[3]`) mientras `NX-E0103` declara el elemento irresuelto al cierre.
2. `e1.n`, `e2.n`, `e3.n` (FAM-E, F10 y relacionados): `fn Main(){let p=true;var r=[];var q=[];let a=(if p then p else q);let c=[q,[r]];` seguido de `q=[3];r=[3];}` (e1) o de `r=[3];q=[3];}` (e2); e3 es e1 sin la unión `a`.
   - e1 (`q=[3];r=[3];`): `NX-E0103` en `r` `[27,29)` y en `q` `[36,38)` y `NX-E0102/type_mismatch` en el `3` de `q`; **pierde** el `NX-E1303` de `q` en `if p then p else q` (conflicto de forma `Bool` frente a `Array<…>`).
   - e2 (`r=[3];q=[3];`): `NX-E1303` en `q` (esperado `Bool`, hallado `Array<Array<I64>>`) y `NX-E0102` en el `3` de `q`, sin `NX-E0103`.
   - e3: solo el `NX-E0102` de la asignación; sin la unión no hay nada que perder.

## 4. Qué dice el ADR (`docs/adr/F0-0002-second-slice.md`)

- §4.1 y la fila `E0103` (línea 1774): `NX-E0103` solo «cuando el elemento de un array vacío o constructor Array sigue sin resolver al cierre de la función».
- §4.1.2.2 (punto 2 de la tabla N, «Fijación y conflicto»): las igualdades compatibles se acumulan y completan los tipos de un conflicto ya conservado en su sitio; «nunca se usa una asignación rechazada para cambiar el tipo de lecturas anteriores»; un conflicto con `expected_type` pendiente «se omite solo si el componente pendiente emite NX-E0103/NX-E0113 al cierre»; «si ninguna restricción compatible lo completa, se omite al terminar porque el constructor `[]` emite NX-E0103». En los repros una restricción compatible (`r=[3]`) sí completa el elemento, así que no corresponde `NX-E0103` y el conflicto debe emitirse con su tipo completo.
- §4.1.2.1 (punto 1): igualdad compuesta atómica; las restricciones independientes de otros hijos se conservan; el orden normativo es el recorrido fuente.
- N(h): `NX-E0103`/`NX-E0113` solo sobre constructores irresueltos con obligación propia de §4.1.2.2.
- §4.1.2.7: el gemelo concreto y las permutaciones solo son oráculo de igualdad si el tipo adelantado no contradice una igualdad que el diferido ya había aceptado en el prefijo; por eso el gemelo no basta como oráculo en FAM-E y la decisión de qué orden es el correcto se apoya en las reglas por orden fuente. Los `perm_diff` con prefijo incompatible que `I07C-FUZZ` ya declaraba no oráculo siguen sin serlo; aquí el defecto es el `NX-E0103` espurio y el diagnóstico perdido, que no dependen de ese oráculo.

## 5. Límites y observaciones

- El esperado exacto de algunos casos (O03, O07, F10) lo derivó el agente por consistencia con controles y con las reglas por orden fuente, no con un ejemplo literal del ADR. El defecto firme, comprobado con el binario, es el `NX-E0103` espurio (elemento resuelto por una asignación compatible) y la dependencia de la salida del orden de asignaciones tardías. El plan de corrección debe fijar los esperados con el ADR y B validarlos.
- Que la causa de raíz esté en la deferral de conflictos de forma sobre tipos pendientes es una hipótesis: no se leyó el código del solucionador.
- Observación sin adjudicar (idéntica en la base): un array heterogéneo dentro del selector de una base contaminada (p. ej. `ar[true][[1,true]]`) no recibe diagnóstico. Candidata a laguna del ADR (§4.1.2.4 y §4.1.1: resultado contaminado y derivados), a decidir por Flujo A.
- Duda de lectura de §4.1.2.4: si el tipo pendiente de `r[1]` cuenta como evidencia válida para fijar la referencia de una unión (F06). `main` es contradictorio con cualquier lectura, así que no cambia el veredicto.
- Las campañas son de `main` `1bd010c`, anterior a F5-pila-r1; F5-pila solo extrae ayudantes sin cambiar comportamiento, pero conviene repetir los repros sobre el `main` que lo contenga.

## 6. Propuesta de trabajo

Defectos con programa reproducible (26 casos, 3 familias) → corrección por Flujo B (propuesta, B, C, D, D'), con regresión por familia (los repros 1 y 2 de §3, F06 y un gemelo permutado por familia). Después, repetir la campaña `twin` (release y debug, misma semilla) esperando `flagged` sin `NX-E0103` espurio y cero `other_differences`.
