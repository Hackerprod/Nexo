# C2 referencia, continuación 33 — bloqueo comprobado (sin PASS)

Base `af19a34` (C1 de Luna). Rama `i07c-rediseno-c2-ref`: `4e63e33` porta y extiende el journal de nodo del prototipo R6; `e333273` incorpora los artefactos v15b; `64ad2d4` añade los discriminadores de D/D'. Ningún commit afirma C2.

## Lo que sí está medido

- `sh scripts/validate.sh`, rustc 1.99.1, JOBS=4 y heavy-lock `i07c-rediseno`: debug 2.220/2.220, release 2.222/2.222; `option_match126` y `result_match126` 5/5 cada una a 65 KiB. Aún falta medir la pareja de marcos `/MAP` del C2 final.
- Release `nexo.exe` SHA-256 `592733c38598545d4d767637d8817b1123a197851fc850e26ff8f722e30ce8de`: `candidate_gate.py --commit C2` completo, 20 corpus/151.188 filas, OK=62.755, C=16.678, U=71.753, H históricas=2; A=0, D=0, A-unmodeled=0, H-divergent=0. El mapa coincide con C1 release en 151.188/151.188 filas. Informe SHA-256 `1953881ef834c5b5fcb2a5aed0090dc08caa19ab343c080f9902734c7a12412f`.
- `owned_rows_gate.py --commit C1`: 3/74, 0/45, 0/13 y F06=`main`, cero fallos. `c2_discriminator_gate.py`: 14 combinaciones perfil/programa con `check` y `run`, cero fallos. En `q[0]=MockOpen(1);` seguido de 4.000 pares `q=Some(q);q=[];`, `main` tardó 1,312 s y la rama 2,187 s (ambos release, mismo programa y perfil `f0-0003-b-v1`); la regresión de ~80× de C1 no persiste.

## Por qué no es C2

`owned_rows_gate.py --commit C2` con el manifiesto v15b comprueba solo 10/74, 0/45, 3/13 y falla en `RC1_read_context_variable` (también su copia congelada) y `ST1_equivalent`. En `RC1`, `fn Main(){var s=[];let a=[s[0],(if true then 1 else true)];s=[1];}`, el ADR exige solo NX-E1303 `[52,56)` y el candidato emite NX-E0102/type_mismatch `[52,56)`; el `I64` de la asignación posterior se ha usado al entrar en la `if`. En `ST1`, `fn Main(){var q=[];let a=[q[0],(if true then [q] else (if true then [1] else q))];q=[1];}`, se exige solo NX-E1303 `[68,71)` y el candidato emite tres NX-E0102 en `[45,48)`, `[68,71)` y `[77,78)`. Son programas reproducibles, no huecos de cobertura.

Hay además una contradicción de propiedad en v15b que esta continuación obliga a resolver en v15c: `owned_corpus_ids.C2` asigna a C2 **11/74, 2/45, 3/13** (incluye `atomic_78`, W0 y W0nounion); `commits.C2` y `verify_r9_contract.py` fijan **10/74, 0/45, 3/13**, y `owner_directed`/`owner_matrix` asignan esas tres filas a C3. Bajo la tabla real, `atomic_78` todavía emite un E0103 `[16,18)` extra ante el único E1303 esperado `[52,53)`; W0 emite E0103 `[27,29)` extra ante E1303 `[69,70)`; W0nounion ya da `checked`. F06 conserva la salida completa de `main`, como exige la puerta cruzada hasta C3. No se ha cambiado el manifiesto para aparentar 11/2 verde.

## Experimento descartado y mecanismo que falta

Se portó experimentalmente la vista R2 del prototipo como snapshot por componente y ordinal AST, sin publicarla. Corrigió RC1/ST1, pero `planned` perdió el E1303 `[69,70)` y el NX-E0102 `[106,112)` y produjo un pánico en la fila congelada `b-projections-t5-2026112402:3` (SHA de fuente `4e04abd430606267415d28076c010ec1d13a052bd1c94c69f8f346f4ec66aadf`): `translation[&node.parent]` no tenía la raíz del hijo. En las primeras 50 filas del primer corpus produjo A=3, D=3 y A-unmodeled=2. La fuente publicada de la rama se restauró a `64ad2d4`; el binario experimental no fundamenta un veredicto.

El snapshot R2 del prototipo copia nodos sin un libro causal por componente; ampliar su clausura de padres evitaría ese pánico, pero no acredita el prefijo ni el origen de `planned`. C2 necesita la sustitución ya prescrita en R6-final §7: `ReadContext` de entrada/epoch, `OriginFact` de componente con ordinal reservado, `Completion` que llena solo huecos, y dependencias de igualdades publicadas para reactivar `Waiting`; sus escrituras deben compartir el journal anidado. `ResultStatus` y la unión final de `if` siguen fuera de C2. El reparto 11/2 de esta continuación debe quedar explícito en la nueva referencia y en v15c antes de B r22. D7/C4 continúa pendiente, sin alteración aquí.

No se publica v15c ni se solicita validación B r22: faltan una referencia C2 que pase esas filas, ambos perfiles del gate completo sobre ese árbol, la medición `/MAP`, y el SHA contractual final.
