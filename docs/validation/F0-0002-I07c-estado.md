# Estado medido de I07c

Generado con `python scripts/i07c_estado.py` desde resultados versionados y la referencia `b506d7d`.

| Magnitud | Debug | Release |
|---|---:|---:|
| Biblioteca activa | 969 | 971 |
| Biblioteca ignorada | 50 | 50 |
| Integración activa | 306 | 306 |
| RD normativo | 152/152 check/run pairs normativos | 152/152 check/run pairs normativos |

`sh scripts/validate.sh` terminó con código 0 en copia limpia del árbol y con target nuevo.

| Pruebas ignoradas en release | Completadas | Fallos |
|---:|---:|---:|
| 4 hilos | 53 | 0 |
| 1 hilo | 53 | 0 |

La biblioteca tiene 50 pruebas ignoradas; la escala de proyección, el oráculo RHS y los gemelos extensos son tres integraciones adicionales. Por eso cada campaña completa 53 pruebas. El oráculo RHS ejecuta 50000 fuentes y los gemelos extensos 300000, calculados desde sus bucles fuente y registrados en ambos modos de hilos.

| Razón máxima de escala | 4 hilos | 1 hilo |
|---|---:|---:|
| Pasos y asignaciones | 4.417 | 4.417 |
| Tiempo de reloj (informativo) | 4.352 | 4.522 |

El límite 4,6 se aplica a pasos y asignaciones; la duración no forma parte de esa puerta.

| Población de producción | Release |
|---|---:|
| Candidatas | 1174 |
| Detectadas | 693 |
| Citas detectadas revalidadas | 693 |
| No compilan | 172 |
| Supervivientes | 309 |
| Supervivientes informativos | 305 |
| Equivalencias normativas verificadas | 4 |
| Deuda de cuota I10a | 0 |

Cada resultado cita la sustitución y, si murió, la prueba que la detectó. `source-sha256.json` comprueba la restauración de las fuentes. Los supervivientes se clasifican según el nivel normativo del ADR en `docs/validation/i07c-mutations/`.

| Campaña medida | Programas | Comprobaciones | Nivel |
|---|---:|---:|---|
| groups_release | 300000 | 300000 | puerta |
| groups_debug | 40000 | 40000 | puerta |
| model_release | 250000 | 250000 | puerta |
| model_newseed_release | 120000 | 120000 | puerta |
| deferred_release | 417372 | 417372 | informativo |
| oracle_release | 2912268 | 2912268 | puerta |
| fuzz_release | 400000 | 800000 | puerta |
| fuzz_debug | 100000 | 200000 | puerta |
| types_release | 200000 | 400000 | puerta |
| stacks_release_128 | 312 | 624 | puerta |
| stacks_release_256 | 312 | 624 | puerta |
| stacks_release_512 | 312 | 624 | puerta |
| stacks_debug_128 | 312 | 624 | puerta |
| stacks_debug_256 | 312 | 624 | puerta |
| stacks_debug_512 | 312 | 624 | puerta |
| head_acceptance | 44 | 88 | puerta |
| quota_reduced_release | 4800 | 268800 | informativo |

| Gemelos tipados/libres | Programas | Firmas reducidas | Excepciones adjudicadas |
|---|---:|---:|---:|
| release | 300000 | 17 | 17 |
| debug | 300000 | 20 | 20 |
| release | 300000 | 21 | 21 |
| debug | 300000 | 16 | 16 |
| release | 300000 | 20 | 20 |
| debug | 300000 | 22 | 22 |

| Gemelos de dominios pendientes | Programas | Comparables |
|---|---:|---:|
| debug | 400000 | 327396 |
| release | 400000 | 327024 |

| Medición complementaria | Valor |
|---|---:|
| Fuentes de proyección normativa | 507000 |
| Fuentes aceptadas cotejadas con `run` | 46607 |
| Mutantes con variaciones D/V adjudicadas | 5 |
| Programas con variaciones D/V adjudicadas | 14079 |
| Gemelos informativos | 300000 |
| Gemelos comparables | 74405 |
| Gemelos exceptuados por resolución posterior | 86130 |
| Diferencias permitidas por resolución posterior | 14552 |
| `deferred_release` sin la misma hoja | 18 |
| `deferred_release` con diagnóstico adicional | 6 |

El arnés de gemelos comprueba campos de C con tipos completos y permutaciones; la campaña extensa de cuota reducida es informativa. La propiedad mínima de cuota reducida forma parte de la suite. Las puertas de fase siguen en `Pending.md`.
