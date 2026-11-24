# Corpus nuevos congelados para I07c

Congelados **antes de medir** el prototipo de restauración atómica (2026-11-24). Ambos generadores proceden del blob de `origin/i07c-estructural-r2:docs/validation/i07c-s5c-explor/`.

- `c1-nueva.json`: `python -B gen_family.py 2026112401 8000 c1-nueva.json`.
- `c2-nueva.json`: `python -B gen_family2.py 2026112402 8000 c2-nueva.json`.

Los SHA-256 de los bytes generados están en `corpus.sha256`. Son muestras de falsación; no sustituyen los corpus históricos ni extienden el alcance del oráculo a formas que este no modela.

Las tres muestras de B de 4.500 programas cada una, citadas en el brief §3.1, no aparecen como blobs en `origin/i07c-estructural-r2` ni en `origin/i07c-s5c-explor`. Se recuperaron de la evidencia local y se congelaron aquí, antes de medir la variante nueva, como `b-<semilla>.json`. Cada entrada conserva `form`, `flat` y `w`. Sus hashes verifican esta copia; **no hay un SHA publicado previo con que demostrar identidad byte a byte frente a la exploración anterior**.
