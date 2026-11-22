# Validación B del plan I07c-S6 — ronda 1: **FAIL** (cuatro bloqueantes)

Base: HEAD `4b73f04`. Todo lo reproducido se comprobó con el CLI de `main` (`1bd010c`); no se modificó ningún fichero del repositorio. El plan es sólido en las 19 filas, las citas del ADR y la receta de cuota; los bloqueantes 2 a 4 dejan una decisión abierta que C tendría que improvisar.

## Bloqueantes

1. **K1-d2: el script de auditoría falla hoy y el plan exige que dé exit 0.**
   - Repro: `python -B scripts/i07c_span_audit.py` termina con exit 1 y el mensaje `unreviewed raw span at src/check/types.rs:7082: e.span().clone()`. Esa línea es `fn primary_span`, la misma función que el plan cita en `types.rs:7075–7083`.
   - El plan (§1 y §3.3) exige «status 0» del script en la puerta y prohíbe modificarlo; no está en la lista de ficheros editables. Tal como está, E fallaría desde el primer día por un defecto del script, no del checker.
   - Acción: decidir en el plan una de dos salidas: (a) excluir la invocación del script y cerrar K1-d2 solo con la prueba metamórfica de grupos; (b) autorizar un fichero concreto para corregir la clasificación del script. Mientras tanto la matriz no puede decir que la auditoría es puerta.

2. **P-8: el plan no fija perfil ni API, y con el perfil usado en el resto del plan el test falla.**
   - Repro: M05 literal con `check --format json --profile f0-0002-b-v2` da `NX-E0006` (`pending_scope: F0-0003`). Con `--profile f0-0003-b-v1` da el esperado `NX-E1325/constructor_type [44,49)`, con `scrutinee_type: Option<I64>`. La variante abierta `F(let l0=DV;let b:Option<I64>=l0;match l0{Ok(n)=>{},_=>{}})` da lo mismo en `[87,92)`.
   - La fila dice «M05 + variante abierta» y a la vez «sin duplicar» el test de `tests/j03_match.rs`, que ya cubre M05 con `ReducedProfile::JNormal` desde la biblioteca.
   - Acción: fijar el perfil o la API de P-8 y qué se añade realmente (solo la variante abierta, o M05 también). Escribir su deducción, porque esa variante no está en el ADR (en F0-0003 §3.2 solo se cita la oración general).

3. **N-a.3 y T-TD-ord: el test descrito no ejercita la cláusula del ADR y el plan no fija el estado de cierre.**
   - N-a.3 (§4.1.2 N(a), «Añadir o quitar cualquier D/V…»): el propio plan admite que no se puede alternar D/V desde fuera. El test `optional_diagnostics_do_not_decide_rd_acceptance` solo comprobaría que los C transcritos deciden la aceptación, lo que ya hace RD.
   - T-TD-ord (§14.10, «restricciones procesadas internamente en orden inverso, con otra elección de representante»): solo se puede reordenar dentro de `src/check/types.rs` (`:18719–18822`, tests internos con `checker.union`/`bind_independent`). Esos tests no reciben fuentes literales, y S6 no toca `src/`.
   - Decisión de B: la subcláusula de orden interno y de representante de T-TD-ord queda **INCONCLUSA** en S6. Las permutaciones de fuente verificadas (`w=4;v=3;`, alias `var w=DV;var v=w;…`, TD03 con `v=false`) se aceptan en `main`; valen como evidencia adicional, no como cobertura de esa subcláusula. N-a.3 solo puede cerrarse como parcial, o con una metamorfosis de fuente concreta que añada o quite hojas D, con aceptación transcrita, como el par `(if true then 1 else 1)` de I-c.
   - Acción: escribir en el plan el estado que tendrán estas dos filas al cerrar la matriz. El nombre del test no debe prometer más de lo que fija.

4. **Arnés RD: el mecanismo de clasificación C/D/V y la «evidencia de rol» están indefinidos.**
   - El plan manda «curar un inventario explícito de sitios D/V permitidos» por cada una de las 152 expansiones, sin decir de dónde sale. Se contradice con «un D/V conforme fuera de la instantánea puede anotarse I»: un extra fuera del inventario es C espurio y falla, así que cualquier D/V nuevo bloquearía. Eso lleva la presencia D/V (columna I, §4.1.2 I(a)) hacia N.
   - Dato a favor de una definición simple: el fixture ya lleva C/D/V por entrada, transcrito del ADR. Sobre `main`, la lista de primarios de las 152 filas coincide con el fixture (152 de 152 en la comparación de B).
   - N-i.2 pide «evidencia de rol en el AST/fuente», lo cual no se define ni cuenta con una API; choca además con «no comparar como N los roles y offsets de relación de RD».
   - Acción: definir (a) la fuente del inventario (propuesta: las entradas D/V del fixture), (b) qué hace el arnés ante un D/V nuevo con código/subcausa válidos de N(j), (c) qué es exactamente la prueba N de relaciones (propuesta: `expected_type != primario`, byte dentro de fuente y no vacío, sin comparar offsets), (d) cómo se detecta «variable pendiente» en `data`.

## Requisitos del encargo

| Req. | Veredicto | Evidencia |
|---|---|---|
| (1) 19 filas | PASA | 19 «sin cobertura» en la matriz (líneas 109, 117, 123, 126–128, 142, 195, 199, 210, 265, 266, 269, 274, 276, 293, 294, 307, 319), idénticas a las del plan. La sección 9 se atiende: N-j.2 y N-b (extras) se cubren con la regla de C espurio de §3.1, aunque el plan no los nombra por ID (no bloquea). P-8/M05 ya está cubierto en `j03_match.rs`; las 5 filas I (I-a/b/c, K4-f, RD-I) se cierran como instantánea por la partición de §4.1.2 y §14.10. |
| (2) Citas y esperados por fila | PASA salvo lo anterior | Las citas literales coinciden con el ADR. Los spans transcritos coinciden: L3-15, TD02 `[32,36)`/`[16,17)`, RD83 `[79,84)`/`[90,95)`, RD101/RD119, DG07 y DG08. El `[174,176)` de L3-19 se verificó contra `main` (`NX-E0102` en `[148,149)` más `NX-E0103` en `[174,176)`). Los positivos de T-F1p, T-F2p, TD04/05, RD87 y K4-f se aceptan en `main`; en RD87(c) `stdout=["[3]"]`. |
| (3) Receta Q | PASA | La receta del ADR da 93.307 bytes y la subcadena `y=a64;` aparece 15.401 veces (15.400 más la declaración). Sobre `main`: `NX-E0003`, `stage=types`, `operation=analysis.limit`, `budget_kind=type_solver_steps`, `budget_scope=compilation`, `limit=1000000`, `observed=1000001`; 3,9 s en release; el control con una asignación (913 bytes) se acepta. Es factible generarla dentro del test. |
| (4) Fortalecimiento RD | FALLA | Bloqueante 4. |
| (5) `src/`, `f0005_b1b.rs`, puertas | PASA | Ni `src/`, ni `f0005_b1b.rs`, ni `scripts/validate*.sh` figuran entre los editables. El único conflicto es el bloqueante 1. |
| (6) Puertas y mutación inversa | PASA con reservas | Ver «No bloquea». |
| (7) Contradicciones y decisiones abiertas | FALLA | Bloqueantes 1 a 4. |

## No bloquea

- El plan está basado en HEAD `4b73f04`, pero `origin/i07c-estructural` está en `e479d62` (adelantado). Hay que revalidar blobs y renumerar referencias de `types.rs` antes de que C empiece (el plan ya lo dice).
- La mutación inversa no se enumera por test (qué condición de `src/check/` se invierte para cada L/E/Q nuevo). Conviene dejar la tabla antes de C.
- Si un test nuevo falla en `main`, el plan prohíbe `#[ignore]` pero no dice dónde vive el test mientras la rama debe estar verde. Conviene fijar el destino (rama o corte de corrección).
- En RD87(c), `observations` incluye el evento de stdout. El ADR solo fija `stdout`; conviene que el test no fije `observations` más allá de eso.
- Los offsets de TD01/TD03 recalculados con la expansión de `DV` de F0-D15 (TD01 primario `[70,74)`; TD03 primario `[73,74)` y origen `[62,66)`) los calcula el test desde la fuente, que es lo correcto. El plan no los lista, pero es derivable.

Evidencia de apoyo (`gen.py`, `cmp.py`, `show.py`, `q.n`, `p8.n`) en `C:\nexo-work\tmp\validador-b-s6\`.
