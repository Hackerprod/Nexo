# Oráculo independiente F0-0005

Python 3.13.1. El modelo, generador y diagnósticos se derivan del ADR F5 y de
las reglas A2/J citadas allí. No importan el compilador Rust ni usan su salida
para crear esperados. `selftest` comprueba 52 filas semánticas con reparación,
18 pares S y los programas publicados antes de que `gate` actúe como juez.
F5-S08 conserva su biblioteca de 18 bytes: `check` acepta y `run` exige
`NX-E0112/missing_main` (véase `LAGUNAS.md`).

```powershell
python -B -m scripts.f5_oracle.cli selftest
python -B -m scripts.f5_oracle.mutate F5-O1
python -B -m scripts.f5_oracle.mutate F5-O2
python -B -m scripts.f5_oracle.mutate F5-O3
python -B -m scripts.f5_oracle.mutate F5-O4
python -B -m scripts.f5_oracle.mutate F5-O5
```

La CLI recibe **rutas de dos ejecutables inmutables** y perfil explícito. El
manifiesto `--complete` es un prefijo ordenado de las subtareas reales de
F5-r1: `L01b,L02b,L03b,L04b,L05b,L07b,L06b,L08,L09,L10,L11,L12`.
`--phase` identifica G1b, G2b, G3b, G4, G5 o G6, pero no infiere capacidades
del binario. Un G6 de escala exige las doce completas. La CLI copia los
ejecutables a un directorio privado `C:/nexo-work/tmp`, coteja SHA-256 y
elimina esa copia al terminar. Solo termina procesos que lanzó. Normaliza
exclusivamente campos `file` a `case.n`; conserva orden y cardinalidad de
diagnósticos, eventos y observaciones. El log JSONL guarda fuente, hashes,
esperado, obtenido, primera diferencia y comando. Reanudar exige el mismo
hash de juez, ADR, binarios, perfil y manifiesto.

Ejemplo de piloto sobre la base G1b actual:

```powershell
python -B -m scripts.f5_oracle.cli gate --binary-debug C:/nexo-work/target-oracle/debug/nexo.exe --binary-release C:/nexo-work/target-oracle/release/nexo.exe --profile f0-0005-b-v1 --phase G1b --complete L01b,L02b,L03b --seeds 0 --valid-per-seed 10 --invalid-per-seed 10 --output C:/nexo-work/tmp/f5-oracle-pilot.jsonl
```

Para el cierre G6, ejecutar la escala 0..31 con 1000 fuentes válidas y 1000
mutaciones por semilla. `batch` toma `heavy-lock.py` en cada tanda, deja 75 s
entre tandas, y genera `summary.json` y un JSONL reanudable por tanda:

```powershell
python -B -m scripts.f5_oracle.batch --binary-debug C:/nexo-work/tmp/f5-bin/debug-nexo.exe --binary-release C:/nexo-work/tmp/f5-bin/release-nexo.exe --profile f0-0005-b-v1 --phase G6 --complete L01b,L02b,L03b,L04b,L05b,L07b,L06b,L08,L09,L10,L11,L12 --seeds 0:31 --valid-per-seed 1000 --invalid-per-seed 1000 --batch-seeds 4 --output-dir C:/nexo-work/tmp/f5-oracle-g6
```

`compare`/`gate` ejecutan hasta cuatro invocaciones por fuente, pero reutilizan
respuestas de fuentes idénticas dentro de una tanda, conservando los 64.000
casos distintos por semilla antes de deduplicar entre semillas. `selftest`
imprime el recuento normalizado y el máximo de nodos por semilla; también
comprueba con dos ejecutables ficticios que `run` rechaza un resultado
incorrecto de `47%1`. Desde G4/L08, el diferencial compara el stdout, los
eventos, las observaciones y sus bytes calculados por el modelo. Antes de
L08 exige solo el diagnóstico pendiente correspondiente a F5 §8.2.
El informe separa casos, invocaciones
físicas y percentiles de latencia. `fuzz` prueba bytes arbitrarios y prefijos
truncados buscando pánico, `internal_error`, `InvalidIr` o timeout; no presume
semántica de bytes arbitrarios. El timeout es fallo, no se reintenta en silencio.

El JSON público no expone ticks de éxito ni bytes vivos/acumulados de Array.
Esos cargos se acreditan con los arneses Rust del corte. Las pruebas de pila
se ejecutan con `scripts/j16_stack_cases.py --closefix` y las recetas F5-r1,
fuera de este oráculo CLI. En el HEAD G1b actual sigue abierto el defecto de
rango almacenado descrito en `HALLAZGOS.md`; no se cambia su esperado.
