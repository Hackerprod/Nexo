# Corpus de proyección independiente para el rediseño I07c

La receta `b` de `scripts/i07c_rd_projection_corpus.py` transcribe `b-repro/fuzz.py` de B r1. Los archivos originales de B (SHA abreviados `35031349…`, `d39c9c47…`, `d37a21fb…`) y sus semillas no se entregaron con los scripts; estas son **muestras nuevas** congeladas antes del barrido corregido, no una reconstrucción byte a byte. El oráculo `scripts/i07c_rd_oracle.py` filtra los programas modelables antes de congelar fuentes; el binario candidato no genera esperados.

| archivo | receta | semilla | programas | con proyección previa en sentencia independiente | SHA-256 del archivo |
|---|---|---:|---:|---:|---|
| `b-projections-2026112401.json` | B | 2026112401 | 3.000 | 1.502 | `7094da96764a6c47bea4178aebb66f7276a3ca1c673df003b5ec07f75312d2d8` |
| `b-projections-t5-2026112402.json` | B con condición fallida | 2026112402 | 3.000 | 1.551 | `eee0155cdaa9549113cc346a480a0ae6b9d8d7f0f9690a55d436a3521e075fc6` |
| `b-projections-2026112403.json` | B | 2026112403 | 3.000 | 1.545 | `cf3049333b04b0bbce64d540ca283a3fe0398791d50789c1aa8e42f398be6714` |
| `owner-projections-771412.json` | propia: `let`, `if` y elemento de array previos | 771412 | 60 | 60 | `f6d6fd8b0b3635dc8bbd873c72724a5755df712b36bd84866e95dc992d48a121` |

La salida del arnés se ata al SHA-256 de ambos ejecutables y separa A/B/C/D en core y full. `scripts/i07c_rd_b_minima.py` fija además M1/M2 por bytes en tres perfiles; `scripts/i07c_rd_b_metamorphic.py` reproduce los gemelos de B. Un gemelo suyo con `let b={value:q[0];};` es sintácticamente inválido como bloque aislado y no cuenta como negativo de inferencia. Ninguna cifra de este archivo declara PASS.

La variante de contexto por sitio de sentencia (SHA del ejecutable `82729b1f2b6c21300974a455614c2c12ad0d20a48c4f3da00456ae9cfc4890eb`) eliminó las regresiones A en los corpus B sin condición fallida y en el corpus propio, pero produjo **22 A de núcleo** en el conjunto T5. Los cuatro resúmenes `r5owner-summary-*.json` registran 9.060 comparaciones, todas modeladas. Los 22 índices constan en `r5owner-summary-b-projections-t5-2026112402.json`; por ejemplo 172 omite el error `[125,129)` y añade `[146,147)`, 416 añade `NX-E0103` `[70,72)`, 537 añade `NX-E0103` `[52,54)`. Son defectos reproducibles: ni la corrección de M1/M2 ni las 16 sondas dan PASS. La ablación de `r1t5` o `r5t5` restaura esos tres resultados, pero también pierde las sondas T5 que motivaron esas piezas; se necesita otra representación.

En los 22 A, la aceptación no cambia; 9 añaden al menos un `NX-E0103`, 7 añaden al menos un `NX-E1303` y 2 pierden al menos un `NX-E0103`. Son recuentos solapados por código, derivados de los diagnósticos completos guardados por índice; un defecto de código o span primario bloquea igualmente.
