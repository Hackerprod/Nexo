# I07c §8.3 — validación B (contexto limpio), ronda 1

Propuesta: `docs/correcciones/I07c-laguna-8-3.md` (base e6e5f82). Flujo A, solo lectura; binario de base compilado en `D:\nexo-work\b2` (target `D:/nexo-work/target-i07c-b2`).

**Primera línea (punto 5): la propuesta no cambia ninguna salida que el ADR fije ni contradice una regla.** Sí cambia lo que la base hace hoy con programas sin salida fijada por el ADR: `valid` pasa de NX-E0103 `[16,18)` a aceptado, y `b1a`/`neg` pasan de `internal_error` a su veredicto. Es la laguna que se cierra, no un motivo para detener.

## Veredicto global: FAIL (1 bloqueante de texto normativo, 1 bloqueante menor; el resto registrable)

## Por requisito

1. Decisión (aceptar, `r: Array<Bool>`): **PASA.** Coherente con §4.1.2.3 «Cierre de dominio indeciso» («Bool para `<`/`==` ... se compone a través de operaciones, bindings, grupos, arrays y uniones»), TD04/TD05 y RD102/RD103 (la igualdad fija el tipo, el dominio queda pendiente). `α=Bool` sale de la igualdad de asignación `Array<α>=Array<Bool>`; no hay `α=Array<α>`. El gemelo de §4.1.2.7 es compatible. La alternativa NX-E0103/NX-E0113 está bien refutada: NX-E0103 «al cierre» (§4.1.2.2, F0-CODIGOS L54) sería falso porque el RHS fija Bool; NX-E0113 es de constructores de valor (L61) y excluye expresamente el Array. No hay cita que haga más coherente la alternativa.
2. Texto normativo y no-cambio de filas: **FALLA (B1, B2).** Filas: escaneo de §§14.10–14.11 buscando `X=` con `X` leído en su propio RHS. Solo aparecen RD70 (`v1=v1;v2=[v2]`, tipos ya concretos: sin cambio) y ninguna TD/DG. En corpus/tests solo `tests/cli.rs:331` (`v` concreto): sin cambio. RD122/RD101/RD119/RD127 no tienen asignación que fije el elemento, y la regla dice que no cambian. La tabla de 127 filas RD (RD01–RD128 menos RD79) es completa, pero es una atestación sin método (R3). `r=[r]` sigue rechazado: binario base da NX-E0103 `[16,18)` (de acuerdo con §4.1.2.2, conflicto recursivo con tipos pendientes omitido).
3. Transcripción de offsets: **PASA con reserva (B2).** Recalculado a mano: `fn Main(){`=10 bytes; `var r=[];` 10–18; segunda sentencia desde 19, `[`=21, `(`=22, `)`=27, `true` 30–33, `]`=34, `;`=35 → RHS `[21,35)`, elemento `[22,34)`; `r=[1];` desde 36, `1`=39 → `[39,40)`. `[]` de b1a `[16,18)` coincide con la sonda de la base. code/subcause/data/found_type correctos por §13.15/§13.17 («Restantes contextos ... asignaciones»). El origen `expected_type=[21,35)` es defendible (§4.1.2.7: el origen de la asignación retirada corresponde al inicializador nuevo, y `var r=[true];r=[1]` da `[16,22)` el array completo; también §13.15 «siguiendo aliases hasta el inicializador»), pero la propuesta lo justifica con una sonda y no con el ADR.
4. Interinidad: **INCONCLUSO/no lícita como mecanismo.** Reconoce que no existe diagnóstico emitible, y NX-E0114 es un código no registrado ni autorizado; el único oráculo que deja es N(f), que ningún comportamiento implementable satisface salvo aceptar (la decisión misma). No acepta ni da InvalidIr en lo que describe, pero tampoco ofrece estado intermedio. Texto citado completo (la cita termina en «§4.1.2.2.»). Positivo/negativo aislados presentes (`r=[true]` vs `r=[1]`, un atributo).
5. Contradicciones: ninguna con filas fijadas (ver 2).

## Motivos del FAIL (accionables)

**Bloqueantes**
- **B1.** Párrafo propuesto: «ese resultado **puede** fijar un componente pendiente». N(a) exige aceptación independiente del solucionador; «puede» deja abierta la decisión. Sustituir por «fija».
- **B2.** El texto a insertar en §4.1.2.3 no enuncia el origen de la relación `expected_type`; solo lo hace la tabla de ejemplos, cuya ubicación en el ADR no se indica. Para un C fuera de RD, N(g) hace normativas esas ubicaciones (§13.15). Candidatos defendibles: `[21,35)` (array completo) o `[22,34)` (elemento, como RD29/RD93/RD106/RD89). Fijar en el texto o en una fila nueva del ADR `[21,35)` con su razón (§4.1.2.7/§13.15 inicializador) y declarar dónde van la fuente y el diagnóstico completos.

**Registrable (no bloquea)**
- R1. Quitar o separar la propuesta NX-E0114 (modifica F0-CODIGOS); indicar que, con el ADR enmendado, se implementa la aceptación directamente y no hay interinidad.
- R2. Añadir caso fijo `fn Main(){var r=[];r=[r];}` → NX-E0103 único `[16,18)` (§4.1.2.2) para amarrar que `r=[r]` no se acepta.
- R3. Añadir el negativo gemelo `fn Main(){var r=[true];r=[1];}` (M `[35,36)`, R `[16,22)`; sondeado en la base) y el método de comprobación de la tabla «Igual» (la base no puede ejecutarlo; indicar escaneo).
- R4. Base inconsistente: `r=[r==r];` sin posterior se acepta pero `r=[(r==r)==true];` sin posterior da NX-E0103; ambos deben aceptarse según la propuesta.
- R5. Errata «RD79 esté retirado»; el término «autorreferencia hipotética» no está definido en el ADR.
