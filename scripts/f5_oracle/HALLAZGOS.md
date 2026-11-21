# Hallazgos del diferencial F5

## Rango almacenado antes de B2b

Fuente exacta de `case.n`, ASCII y **sin LF final** (23 bytes):

```nexo
fn Main(){let r=0..<3;}
```

Invocaciones con el ejecutable debug de `9af7d08`, construido en
`C:/nexo-work/target-oracle` con Rust 1.99.1 MSVC:

```text
nexo.exe check case.n --profile f0-0005-b-v1 --format json
nexo.exe run case.n --profile f0-0005-b-v1 --format json
```

Ambas devuelven exit 2 y exactamente el siguiente JSON, salvo la ruta de archivo
del argumento:

```json
{"schema_version":1,"profile_id":"f0-0005-b-v1","outcome":{"kind":"driver_error","reason":"internal_error"},"stdout":[],"events":[],"observations":[],"observation_bytes":0,"reserved_observation_bytes":0,"diagnostics":[]}
```

F5 §3.1 y §8.2 («Rango fuera de for directo»), concretado por F5-H03 y
F5-D01, exige en `check` y `run` `rejected`, exit 1 y un único diagnóstico
`NX-E0006`, `feature_id=range`, `data={"pending_scope":"later_adr"}`,
`types/types.check`, primario `[17,20)` sobre `..<`. El JSON literal F5-H03
incluye los demás campos contractuales. El arnés debe registrar la diferencia
como divergencia conocida hasta que B2b corrija la causa; nunca convertir
`internal_error` en esperado ni excluir silenciosamente el caso.

El piloto del oráculo sobre `9af7d08` reprodujo la misma familia con la fuente
generada `fn Main(){let answer=0..<1;}` (sin LF). Ambos verbos en debug y
release dieron `driver_error/internal_error`; los otros 19 casos del lote
0 (10 válidos y 9 negativos aislados) coincidieron con su proyección G1b.
El reporte JSONL es reconstruible con el comando de `README.md`; el fixture versionado es
`tests/fixtures/f5_oracle_regressions.jsonl`.

## Const escalar reconocido pero nombre no resuelto en `d86cd26`

Fuente ASCII sin LF final (43 bytes):

```nexo
const k:I64=comptime 1;fn Main(){Print(k);}
```

Con `C:/nexo-work/bin/d86cd26/debug/nexo.exe` y el release del mismo commit,
`check` y `run`, `--profile f0-0005-b-v1 --format json`, devuelven exit 1,
`rejected` y un único `NX-E0101`, `names/names.resolve`, primario `[39,40)`
(`k`), `data={"use_kind":"value"}`. F5 §8.2, «Const escalar antes de L09 y
L10», dice que mientras L09 esté pendiente la forma escalar incluida recibe
**exclusivamente** `NX-E0006/implementation_limit`, `types/types.check`,
`pending_scope=L09` sobre `const` `[0,5)`. La fuente contiene esa forma y
no usa ninguna regla excluida. La misma falla de nombres se reprodujo con
`fn Main(){let x=k;Print(x);}` tras la declaración const, así que el hallazgo
alcanza al uso de const como valor, no solo a argumentos de llamada.

En G1b, semillas 0:7, 100+100 por semilla, el arnés conserva 160 diferencias
de esta familia (40 fuentes × dos verbos × debug/release) como
`known_issue=F5-const-name-use` en el JSONL; no cambia el esperado del ADR.
El coordinador puede trasladar esta repro al carril B1b.

## F5-N02: nombre duplicado antes de L09

Fuente ASCII sin LF final:

```nexo
const k:I64=comptime 1;const k:I64=comptime 2;fn Main(){}
```

F5 §9.2.1, fila F5-N02, fija NX-E0104 en `names/names.declare`, primario
`[29,30)`, `declaration_kind=constant`, relacionado
`previous_declaration=[6,7)`. El binario `d86cd26` debug con `check`,
`--profile f0-0005-b-v1 --format json`, devuelve exit 1/rejected y dos
NX-E0006/types con `pending_scope=L09` en `[0,5)` y `[23,28)`; no emite
NX-E0104. El arnés registra solo esa respuesta exacta como
`known_issue=F5-const-duplicate-name` para la familia generada. Esta familia
comienza después de los primeros 100 índices, de modo que la tanda G1b
100+100 conserva las 160 observaciones previas de `const.scalar`.
