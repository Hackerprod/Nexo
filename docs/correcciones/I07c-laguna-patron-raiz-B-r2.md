# I07c laguna «patrón constructor sobre raíz abierta» — informe B r2 (Flujo A)

Validado: `docs/correcciones/I07c-laguna-patron-raiz.md` corregido, contra F0-0002 §§4.1.1–4.1.2.3, 13.6, 13.15, 13.17, F0-0003 §§3.2, 3.3, 8.2 y el binario de `c719097` (sondas en `D:/nexo-work/i07c-bval`, ficheros `f1`–`f9`).

## Veredicto: PASS

## Comprobación de los 4 motivos de r1

| Motivo | Resultado |
|---|---|
| 1. Llave de `Main` en el ejemplo de familia | Corregido (`...{}}}`, 95 bytes). Sin la llave da `NX-E0002` `[94,94)` (parse), con ella `internal_error` (f3), como declara el texto. Offsets a mano correctos: `Ok(y)` `[78,83)`, llaves finales `[93,94)` y `[94,95)`. |
| 2. Forma concreta posterior | Definido: `NX-E0102/constructor_shape` si el tipo previo es parcial; `type_mismatch` si es completo. Offsets a mano: `Bool` `[90,94)`, RHS `[95,97)`; `Result<I64,I64>` `[90,105)`, RHS `[106,108)`; origen `[66,73)`. `related_spans` en orden de §13.6 (archivo, inicio, fin, rol). Roles `origin`, `expected_type`, `found_type` están en el vocabulario. Control: con tipo previo completo el binario ya da `type_mismatch` (f8, f9), coherente. Con la forma concreta antes del patrón da `constructor_type` (f6), coherente con el texto. Las fuentes nuevas dan hoy `internal_error` (f1, f2, f5, f7). |
| 3. Subpatrón sobre componente pendiente | Definido como recursivo (`Option<Option<α>>`, `Option<Result<α,β>>`) con negativo de familia anidada `[89,94)`: `Some(Some(x))` `[66,79)`, `,` 83, `Some(` `[84,89)`, `Ok(y)` `[89,94)`, correctos; hoy `internal_error` (f4). |
| 4. Ubicación única | Resuelto: bloque en §3.2 (resolución y cobertura) y bloque en §8.2 (filas, Data, mensajes, prioridad), con anclajes de inserción explícitos; ambos textos coinciden en campos y condiciones. Son subcausas de códigos existentes, no códigos nuevos, así que no contradice «exactamente tres códigos nuevos»; no cambia ninguna fila vigente. |

## Otras comprobaciones

- Ninguna salida fijada cambia: ninguna fuente de F0-0002/F0-0003 contiene `match` de constructor sobre raíz abierta; M01–M10 y los ejemplos de §3.3 tienen scrutinee concreto.
- La necesidad de `NX-E0102/constructor_shape` es verdadera: §13.17 y la lista de formatos no admiten serializar tipos con incógnitas, y `type_mismatch` exige `found_type` concreto.
- Texto normativo sin material de proceso, con válido e inválido por cada regla.

## No bloqueante

- Predicados de dominio refutados por una forma parcial (p. ej. `Len(l0)` con `l0` ya `Option<α>`) no se enumeran; la cláusula «otra igualdad» cubre `+`, pero conviene una línea explícita.
- `related_spans` del ejemplo se escribe como pares abreviados, no como objetos `RelatedSpan` de §13.6.
- Siguen sin fila de ejemplo `None=>{},Some(x)=>{}` y aridad en raíz abierta.
