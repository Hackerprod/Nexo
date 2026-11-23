# Revalidacion dirigida B r2 de I07c-laguna-t5-r1 (commit 7470afb) - veredicto: FAIL (1 motivo)

Metodo: script `r2/chk.py` (en C:\nexo-work\tmp\validador-b-t5\r2) recalcula por bytes y ejecuta `main` (base-release) y V6 (cand-release) sobre las 16 fuentes
de los ejemplos (tabla 1.3 x7, discriminantes x4, texto del addendum, valido, contraste, corregido, O08).

## Resultado
- Spans: TODOS correctos por bytes y por CLI (17 spans de primario + 8 spans de la columna B/relaciones: `[17,21)`, `[24,28)`, `[22,26)`, `[37,41)`, `[16,20)`, `[34,38)`,
  `[24,25)`, `[26,27)`, `[38,42)`, `[52,56)`, `[51,55)`, `[29,30)`, `[39,40)`, `[16,18)`, `[21,22)`, `[43,44)`, `[33,34)`, `[21,25)`, `[48,54)`); relaciones citadas
  (`expected_type=[24,28)`/`[34,38)`/`[38,42)`, `found_type` correspondientes) coinciden. `main` == V6 en todas.
- Motivos 1, 2 y 3 de r1: corregidos (span `[24,28)`; O01 y O08 y lista de documentos; V/I y cuatro discriminantes verificados; bloque con valor verificado).
- Texto del addendum: exacto, sin decision abierta (NX-E0103 e `if` con condicion fallida con ejemplos correctos).

## Motivo del FAIL
1. Fila 5 de la tabla 1.3 (linea 43): `fn Main(){let a=[[true]<1,1];}` -> la propuesta dice «`[24,25)` (`1`), `Bool`/`I64`». La salida real de `main` y V6 es
   `data={"expected_type":"Array<Bool>","found_type":"I64"}` (relaciones `expected_type=[17,23)`, `found_type=[24,25)`). Corregir a `Array<Bool>`/`I64`
   (la igualdad de `<` compara `[true]` con `1`; el esperado es el tipo completo del operando izquierdo). La tabla 1.3 se declara en §5 esperado transcrito de un defecto de nucleo.

## No bloqueante
Ninguno nuevo.
