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
