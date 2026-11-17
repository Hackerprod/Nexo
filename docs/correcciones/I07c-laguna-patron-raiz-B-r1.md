# I07c laguna «patrón constructor sobre raíz abierta» — informe B r1 (Flujo A)

Validado: `docs/correcciones/I07c-laguna-patron-raiz.md` contra F0-0002 §§4.1.1–4.1.2.3, F0-0003 §§3.2, 3.3, 8.2 y el binario de `c719097` (release, `D:/nexo-work/target-i07c-l`; sondas en `D:/nexo-work/i07c-bval`).

## Veredicto: FAIL

La decisión (el patrón aporta la forma Option/Result a la raíz) es la más coherente con el ADR y el argumento contra el rechazo es verdadero. El texto a insertar, tal como está, tiene un ejemplo roto y deja tres decisiones abiertas.

## Motivos (accionables)

1. **El ejemplo inválido de familia no tiene cerrada la llave de `Main`.** El texto normativo da `fn Main(){let l0=DV;match l0{Some(x)=>{},Ok(y)=>{},_=>{}}` (una `}` menos que el ejemplo aceptado). Con `DV` expandido, `check` da `rejected` con un único `NX-E0002` (parse) en `[94,94)`, no `NX-E1325/constructor_shape` en `[78,83)`. Es un negativo que falla por sintaxis accidental. Con `}}` final el binario da hoy `internal_error`, lo esperado antes de la corrección. Los offsets calculados a mano `[78,83)` y `[57,62)` son correctos.
2. **Forma concreta posterior incompatible con un patrón previo: decisión abierta.** El texto solo cubre la forma concreta «previa» (`constructor_type`). Para `let l0=DV;match l0{Some(x)=>{},_=>{}}let b:Bool=l0;` y para `...let b:Result<I64,I64>=l0;` el binario da `internal_error`. Con la forma concreta antes del `match` da `NX-E1325/constructor_type`, `scrutinee_type=Bool`, sobre el patrón. §4.1.1 manda denunciar la primera igualdad incompatible en orden fuente, que aquí sería el `let` posterior con `found` pendiente (no serializable, N(j)). El texto debe fijar código, subcausa y primario (o declarar que el patrón se denuncia con `constructor_type` por N(a)) y añadir el par válido/inválido.
3. **Subpatrón constructor sobre un componente pendiente: decisión abierta.** La regla se enuncia para «lectura de un binding cuyo tipo aún no tiene constructor». `let l0=DV;match l0{Some(Some(x))=>{},_=>{}}` (el componente `α` no es lectura de binding) da `internal_error`. Debe decir que la regla es recursiva sobre componentes pendientes y dar su cobertura (testigo), o excluirlos con un C.
4. **Ubicación única no inequívoca de la nueva subcausa.** La fila `constructor_shape` (Data, mensaje canónico, precedencia) se define en §3.2, pero §8.2 es un catálogo cerrado («exactamente tres códigos nuevos», filas, lista de mensajes, «Constructor_type precede a constructor_arity»). Un implementador que lea §8.2 no la ve. O la fila y el mensaje van en §8.2 (y §3.2 remite), o se declara que §8.2 se enmienda en el mismo acto, con un único texto autoritativo.

## Tabla de comprobaciones

| Punto | Resultado |
|---|---|
| `match DV{Some(x)=>{let k:Bool=x;},_=>{}}` | `checked`. Confirma. |
| `let l0:Option<I64>=DV;...match l0{Some(x)=>{},_=>{}}` | `checked`. Confirma. |
| `let l0=DV;match l0{None=>{},_=>{}}` | `NX-E1304/unreachable_arm` `[75,76)`. Confirma. |
| Repro `b3x.n` y variantes `Ok`, `Some`+`None`, `Some(x)`+`_` | `check` y `run`: `internal_error`. Confirma. |
| `Some(x)` solo | `NX-E1304/non_exhaustive` `[57,62)`; hoy `missing_pattern=_`; el testigo de §3.3 (None antes que Some) da `None`: la propuesta es coherente. |
| Cobertura `Some(x)`+`_`, `Some(x)`+`None`, `Ok` no cubre `Err`, guardas sin cobertura | Coherente con §3.3 y con M01–M10 (scrutinee concreto sin cambio). |
| `None` sin evidencia es binding; bajo Result es binding | Coherente con §3.2. `None` primero con evidencia posterior (hoy `None=>{},Some(x)=>{}` da `unreachable_arm` `[75,82)`) pasaría a aceptado; N(a) lo permite; falta fila de ejemplo (no bloqueante). |
| Subcausa nueva vs §8.2 | Necesaria: `constructor_type` exige tipo completo y no se puede serializar `Option<α>`. No hay opción más pequeña honesta. No contradice filas existentes (ubicación: motivo 4). |
| Cambio de salidas fijadas (RD/TD/DG, T01–T14, M/Q) | Ninguna fuente fijada de F0-0002/F0-0003 contiene `match` sobre raíz abierta (revisión por búsqueda). Sin cambio. |
| Texto normativo sin proceso, con válido e inválido | Sí, salvo motivo 1. |

## No bloqueante

Faltan filas de ejemplo para `None=>{},Some(x)=>{}`, para evidencia independiente posterior a un `None` (`let o:Option<I64>=l0`) y para aridad en raíz abierta (`Some(a,b)`).
