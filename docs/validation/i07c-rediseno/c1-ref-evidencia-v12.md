# R6 v12: evidencia de la referencia C1 aislada

Estado: en validación por B r18; este registro no declara PASS.

## Árbol y alcance

- Base: `origin/i07c-rediseno-impl` `96d564496b1072d09a4a169c3be801d69aa3af96` (C0, derivado de `main` `31d91dbebd31c35b3cd09095c2d8e65b96c80548`).
- Rama publicada: `origin/i07c-rediseno-c1-ref` `88205510cdfe8f9df3eef1236a33a0487e9474cf`; árbol `0f6aa10427685a106e21ac68fa5d026b4863bb54`. `git diff --numstat 96d5644 8820551`: `src/check/types.rs` +111/−13, `src/ir/i07c_tests.rs` +1/−1. El segundo archivo porta únicamente el primario E3 `[104,107)` aprobado en G3.
- La búsqueda del diff por `NEXO_XV`, `union_denounced`, `EqualityRecord`, `ReadContext` y R1–R5 no devuelve mecanismo de esas etapas. C1 añade la selección del origen causal del constructor en el camino de igualdad recursiva y lo conserva en `PendingDeferredJoin` separado del sitio `found`.

## Reproducción y salidas observadas

Compilar `main` y el commit de referencia en clones limpios de `C:`, con `CARGO_BUILD_JOBS=4` y targets propios. Ejecutar desde el árbol de diseño v12, usando una ruta estable en `--case` para ambos binarios:

```text
python -B docs/validation/i07c-rediseno/verify_r9_contract.py
python -B docs/validation/i07c-rediseno/owned_rows_gate.py --main-exe <main-release.exe> --candidate-exe <ref-release.exe> --case <caso.n> --commit C1 --output <owned.json>
python -B docs/validation/i07c-rediseno/candidate_gate.py --main-exe <main-release.exe> --candidate-exe <ref-release.exe> --case <caso.n> --commit C1 --limit 500 --output <corpus.json>
```

El verificador informó `R6 v12 contract structure OK` y `matrix oracle 45`. `owned_rows_gate` terminó con `complete=true`, 3/74 dirigidas, 0/45 matriz, 0/13 congeladas, F06=`main`, `failures=[]`; salida íntegra en `c1-ref-owned-v12.json` (SHA-256 `4fda59a68fb39204e0c572768ce20f2db92eb9922d676e7b801ca00e356b208b`). Los tres primarios son `recursive_69` `[60,63)`, `recursive_78_equivalent` `[16,18)` y `[69,72)`, `recursive_127_equivalent` `[60,63)` y `[118,121)`.

El gate cruzado con `--limit 500` recorrió los 20 corpus/perfiles, 9.560 filas, con `failures=[]`: `OK=3911`, `C=1357`, `U=4292`; salida íntegra en `c1-ref-corpus-500-v12.json` (SHA-256 `cb35ad248a79fe871ea36d6cd0802654c7a4dd1ebb4b96dafc822f9bc1ac9c5`). `complete=false`: esta muestra **no acredita** la puerta cruzada de 151.188 filas. La comparación separada de las 103 fuentes `match` contra `main` dio 103/103 sin diferencias.

La puerta completa `sh scripts/validate.sh` corrió en clon limpio del commit de referencia, bajo `python -B scripts/heavy-lock.py run --session i07c-rediseno`, con `CARGO_BUILD_JOBS=4` y `CARGO_TARGET_DIR` en `C:`. Terminó con exit 0: fmt, clippy `-D warnings`, nextest `--profile default` debug 2218/2218 y release 2220/2220, y sondas TYPES `option_match126` y `result_match126` 5/5 a 65 KiB. El binario release de esa puerta tiene SHA-256 `d709920e71b24cb975928a117bd55cf1a6f25401f989a786e396e612d201fd2b`; el `main` release limpio, `28f25c2e5762722afec07bf63f1497a1bfa8dc6d5d5b10a7e93dfeffa7f27cbd`. Las comparaciones dirigidas de los tres recursivos y F06 dieron el mismo JSON en debug/release y en `check`/`run`; F06 conservó íntegros E0103 `[27,29)` y E1303 `[83,85)` de `main`.

`atomic_78` y W0 conservan el E0103 adicional de `main` en la referencia aislada. La ablación de B r17 ya mostró que restaurar solo `type_nodes` no lo elimina. La propiedad de ambas y del positivo W0nounion se trasladó a C3, donde se introduce `JoinRecord`; C1 no reclama resolverlas. B debe volver a comprobar el contrato y el árbol de referencia independientemente.
