# Oráculo independiente de F0-0004 (texto UTF-8 y tipos de usuario)

Herramienta de validación (andamiaje con esperados de núcleo) que decide, para fuentes Nexo con `String`, `struct`/`enum`, métodos, `derive`, visibilidad, entorno simulado v2 y la aplicación de referencia, qué resultado o diagnóstico exige el ADR `docs/adr/F0-0004-text-and-user-types.md` (K, pin `1010938`, blob `8bf37f56…`). No cambia K ni autoriza aceptar fuentes F4 antes de su K. Plan: `docs/correcciones/F4-oraculo-r1.md` (r3). Python 3.11+ sin dependencias externas (medido con 3.13.1).

## Independencia

Los esperados salen solo de K, de los fragmentos heredados que el plan ratifica (A2 §§4.2, 12.4, 13.6, 13.12, F0-0003 §3.1, D15 §2.4, transcritos con hash en `tests/fixtures/f4_oracle_pins.json`) y de literales de K §§2–12. Nunca de `nexo.exe`, de `src/`, del prototipo ni de `scripts/f5_oracle` (de ahí solo se copian, con pin AST, las piezas genéricas de `generic.py`). Un esperado jamás se calcula observando el binario; lo que K no fija va a `LAGUNAS.md` y no se compara. `audit` lo comprueba por lista blanca (clases de módulo y grafo de imports), por patrones de ruta vetados y, dinámicamente, con `sys.addaudithook` durante `selftest`.

| Clase | Módulos | Regla |
|---|---|---|
| MODELO | `utf8ref`, `model`, `literal`, `diagnostics`, `generate`, `barrier`, `app`, `charges` | sin E/S, sin `json`, sin `os`; importan solo la biblioteca de la lista blanca y otros MODELO o el LECTOR |
| LECTOR | `fixtures_io` | única superficie de lectura (fixtures `f4_oracle_*` y los cinco ADR pinados) |
| ARNÉS | `generic`, `runner`, `batch` | solo lee JSONL ya generado; no importa ningún MODELO; puede usar `json` y archivos (la restricción del lector rige para el MODELO) |
| ORQUESTA | `cli`, `selftest`, `mutate`, `audit`, `minimize` | unen modelo y arnés |

## Uso

```
python -B -m scripts.f4_oracle.cli selftest [--fast] [--groups G0,G9,...] [--fail-fast]
python -B -m scripts.f4_oracle.cli audit
python -B -m scripts.f4_oracle.cli generate|fuzz-corpus --profile P --phase G9 --complete K01,...,K10 --seeds 0:7 \
    --valid-per-seed 500 --invalid-per-seed 500 --output f.jsonl [--no-literals] [--final]
python -B -m scripts.f4_oracle.cli compare|gate|fuzz --binary-debug D --binary-release R --profile P --phase G9 \
    --complete ... --seeds ... --valid-per-seed N --invalid-per-seed N --output out.jsonl
python -B -m scripts.f4_oracle.cli minimize|batch|corpus-sha ...
python -B -m scripts.f4_oracle.mutate F4-Ok [--only T01,N03] [--workers 2] [--full] [--check]
```

- `selftest` (≈ 2–4 min; `--fast`: `audit`, pines, literales y `utf8ref`, < 1 s) corre antes de comparar con ningún binario: filas de K reproducidas por el modelo (G0–G10), determinismo y SHA de corpus, mínimos de distinción, ejecutables ficticios con una corrupción por campo del contrato, la matriz C01–C11 y la propia campaña de mutación. `gate` ejecuta `audit` y `selftest`, registra exit 0 y el SHA-256 de su log y solo entonces `compare`.
- Fases `G0`–`G10` y `--complete`: conjunto de unidades K cerradas (no un prefijo), validado contra las dependencias de K §12 y las listas de cada fase (`barrier.PHASES`); una combinación imposible es error de arnés (exit 2). `--final` solo vale con `--phase G10`: añade las recetas largas de cuotas de la clase Q (65.537 bytes, 4.097 asignaciones, 16.385 miembros, cinco cadenas de 60.000 bytes) e imprime la matriz C01–C11 de K §12.1.
- Clases de caso: V válidas, N una mutación aislada de una V con su positivo reparado, L gramática del Hole, B barrera de capacidad pendiente, A la aplicación de K §11 con entorno aleatorio, E entornos válidos o con un solo defecto, Q recetas de cuotas (perfil normal y los cuatro reducidos de K §8.3). Cada registro lleva `expected` y `expected_sha256` antes de la primera invocación; el arnés no recalcula nada.
- Exit de `compare`/`gate`/`fuzz`: 0 sin divergencias; 1 ante divergencia, fallo de arnés, `internal_error`, pánico, `InvalidIr` o timeout confirmado (un timeout nunca es PASS); 2 ante error de configuración o `known_issue` caducada. Una divergencia cubierta por `tests/fixtures/f4_oracle_known_issues.json` (familia, código, ruta, fase máxima, cita de K, id de `HALLAZGOS.md`, revisor distinto de C) se cuenta aparte como `known`.
- Semillas: G0–G9 `0:7`; G10 `0:31` (C), `32:63` (D), `64:95` (D'); fuzz `96:127`. `--seeds A:B` incluye ambos extremos. `corpus-sha` fija el SHA del corpus por semilla y clase (`tests/fixtures/f4_oracle_corpus_sha.json`; regenerar solo tras revisar un cambio del generador).

## Reglas de máquina

Los binarios se copian a `C:/nexo-work/tmp/f4-oracle-<PID>/bin/`, se cotejan por SHA-256 y se borran; cada invocación escribe la fuente (y el entorno) en su propio directorio; solo se terminan PID propios (nunca `taskkill /IM`). Las campañas largas se lanzan con `scripts/heavy-lock.py`, 2 workers como máximo por carril y un hueco de 60–90 s entre lotes (`batch`).

## Archivos

`COBERTURA.md` (qué cubre cada tramo y su contraste con el binario), `MUTACIONES.md` (campaña sobre el propio oráculo), `LAGUNAS.md` (lo que K no fija), `HALLAZGOS.md` (defectos del producto, de K y del oráculo), `DESVIOS.md` (diferencias con el plan).
