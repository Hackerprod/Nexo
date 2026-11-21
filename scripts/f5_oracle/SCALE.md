# Medición y estado de escala del oráculo

Base de implementación de `nexo.exe`: `9af7d08`, etapa G1b con
`L01b,L02b,L03b` completas. Los binarios debug y release se construyeron con
Rust 1.99.1 MSVC y `CARGO_TARGET_DIR=C:/nexo-work/target-oracle`; cada
invocación lleva `--profile f0-0005-b-v1 --format json`.

- La construcción y ejecución **histórica, anterior a F5-O5, del modelo** con semillas 0..31,
  1000 válidos y 1000 negativos por semilla procesó 64.000 casos lógicos
  (32.000 + 32.000), 19 familias, en 17,3 s. La lista de SHA-256 por semilla
  está versionada en `tests/fixtures/f5_oracle_corpus_sha.json` y se comprueba
  en cada `selftest`. Esto acredita determinismo y consistencia
  del modelo con su parser/checker propio; no acredita el diferencial CLI.
- El piloto CLI G1b de semilla 0, índices 0..9 en ambos carriles, ejecutó 20
  fuentes con debug y release reales. La única familia discrepante fue el
  rango almacenado H03 (`internal_error` en los cuatro comandos), documentado
  en `HALLAZGOS.md`. S08 positivo dio `checked` en `check` y
  `NX-E0112/missing_main` en `run` con ambos binarios.
- La primera tanda del ejecutor (`batch.py`, semilla 0, 100+100) tomó y liberó
  `heavy-lock.py`: 200 casos lógicos, 93 respuestas de fuente reutilizadas,
  452 invocaciones físicas incluyendo H1/H2 y S08, 4,4 s de diferencial tras
  la autoprueba. Latencias p50/p95/p99: 8,379/10,542/35,283 ms. Las 40
  diferencias son las diez materializaciones de `range.stored` en los cuatro
  comandos; no hubo otra familia divergente. El proceso devolvió exit 1,
  como exige el hallazgo conocido, y no se convirtió en éxito.
- La campaña pública G6 0..31 y las fronteras de cuotas F5 siguen pendientes:
  esta base no tiene L08–L12 completas. Ejecutarla declarando esas capacidades
  falsamente convertiría pendientes conocidos en miles de divergencias
  artificiales. `batch.py` requiere el manifiesto G6 completo; cuando B6
  entregue esa base, correr el comando de `README.md` y registrar aquí
  casos, descartes, diferencias, invocaciones físicas y percentiles.

Las 19 familias aleatorias anteriores a F5-O5 cubrían escalares, control simple,
Array/Len/Join, fallos aritméticos y seis negativos estáticos aislados.
Const, efectos, préstamos complejos y fronteras de cuota están hoy en los
literales dirigidos y el modelo, sin generación aleatoria amplia. Esta brecha
de cobertura queda visible para la validación; el conteo 64.000 no la oculta.

La deduplicación del ejecutor reutiliza respuestas solo para bytes de fuente
idénticos, mismo perfil y mismos SHA-256 de binarios dentro de cada tanda. El
denominador anterior era 64.000 casos lógicos antes de deduplicación. La
puerta de pila 256/512 KiB es externa y sigue F5-r1; este documento no la
acredita.

## Corrección F5-O5

F5 §10 («Escala determinista», líneas 1317–1319) exige 32 semillas × 1000
fuentes válidas y 1000 mutaciones fuente por semilla. El generador actual
normaliza espacios e identificadores, deduplica por esa forma y registra en
`selftest` y `tests/fixtures/f5_oracle_corpus_sha.json` **1000 válidas y
1000 inválidas distintas para cada semilla 0…31**. El denominador comprobado
es 32.000 + 32.000 dentro de cada semilla; las coincidencias entre semillas
no se usan para inflar ese dato. Los SHA-256 se recalcularon sobre los casos
con observaciones. El máximo verificado es 30 nodos por fuente válida,
por debajo de los 32 de F5 §10. La versión Python queda fijada por el fixture.

Las 18 familias válidas incluyen Push/Pop anidado, orden de For Array,
break/continue, cortocircuito con efectos, función de usuario y const.
Las 10 familias inválidas incorporan una fuente válida variable antes de
mutar una sola regla. F5 §10 y el plan r2 §3 exigen además extremos y
profundidades; el generador de hasta 32 nodos no cubre por sí solo las
recetas largas de F5 §§9.4–9.5. Se enumeran brechas y pruebas dirigidas en
`COBERTURA.md`. La campaña CLI G6 queda pendiente de una base con L08–L12
activas; la proyección anterior conserva los pendientes de F5 §8.2.

## Segunda corrección F5-O6

`selftest` recalcula 32 SHA-256 del corpus con Python 3.13.1. En cada semilla
0…31 cuenta 1.000 fuentes válidas y 1.000 inválidas distintas después de
normalizar espacios y nombres. Abstrayendo también cada literal entero,
resultan **562–567 formas válidas y 896 inválidas** por semilla. El máximo
es 32 nodos válidos. Son 18 familias válidas y 14 inválidas; el incremento
de estructura incluye identidades aritméticas/lógicas en distintos árboles,
contextos heredados, ciclos e impureza const, préstamo vivo y FIRST truncado.
La frontera de cuota ±1 es dirigida (`check_charge_literals`, F5 §9.4), pues
el perfil reducido de L08 aún no está activo en G1b.

El contraste G1b con `d86cd26` debug y release, semillas 0:7 y 100+100,
ejecutó 1.600 casos. La primera versión de O5 había dado 594 divergencias;
la proyección corregida dio **0 divergencias sin clasificar** y conservó
160 observaciones del defecto `const k` como hallazgo conocido. `gate` publica
por semilla tanto los distintos normalizados como los estructurales. El
oráculo no deduce el esperado de esos binarios. El reintento único de timeout
se registra; un segundo timeout sigue siendo fallo.

## Tercera corrección F5-O7

Con payloads Array/Option de Pop y la familia `const.duplicate_name`, el
fixture recalculado mantiene 1.000+1.000 fuentes distintas por semilla 0…31.
Las formas estructurales son 564–569 válidas y 899 inválidas, con máximo
32 nodos válidos. Hay 18 etiquetas de familia válida y 15 inválida; ocho
de las válidas aportan una sola forma estructural y varían sus literales.
El mínimo de 400 se cumple por las otras formas; no se atribuye a esas ocho
familias una diversidad estructural que no aportan.
