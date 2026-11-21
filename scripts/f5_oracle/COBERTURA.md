# Cobertura del generador F5-O5

Autoridad: F5 §§2–5, 8.2, 9.3 y 10; plan del oráculo r2 §§2–4.
`selftest` compara 70 pares de diagnóstico/sintaxis transcritos, ocho
regresiones de ejecución con stdout literal y los casos generados con el
modelo. En `G1b`–`G3b` el diferencial compara el diagnóstico pendiente de
F5 §8.2; en `G4` con `L08` completa compara también stdout, observaciones,
bytes, eventos, error y outcome. La prueba con ejecutables ficticios detecta
un resultado equivocado de `47%1`.

La revisión de los mutantes descritos por D se reproduce en
`tests/fixtures/f5_oracle_model_pairs.json`: `push_copy_nested` distingue
el Push que no copia, `for_array_order` detecta recorrido inverso y
`for_range_break` detecta un break anulado. `mutate.py` restaura cada archivo
por SHA-256; sus 32 inversiones previas y ocho adicionales se ejecutaron
aisladamente, con 40 muertas y ninguna superviviente. El informe de D no
incluía los parches exactos de sus 24 variantes, por lo que ese conteo no
se atribuye a una reproducción byte a byte de su campaña.

| Familias válidas | Reglas F5 | Observación |
|---|---|---|
| Escalares I64/Bool, Len, JoinArrays, if, while, compuesta, For rango y Array | §§2–4; §9.3 | Print del resultado y de una expresión dependiente; constantes variables por semilla/índice. |
| Push de Array anidado y Pop/Option | §4.1; §9.3 Push/Pop/JoinArrays | Copia independiente tras mutar origen; Some y None impresos. |
| For Array ordenado; For rango con break y continue | §§3.1–3.2; §9.3 | Secuencia de Print distingue inversión, break anulado y continue. |
| `and`/`or` con Right que imprime; función de usuario | §2.2; §9.3 | La salida prueba evaluación omitida/tomada y retorno. |
| Const escalar | §5; §8.2 H2 | Print del valor desde L10; antes se exige L09 o L10 pendiente según manifiesto. |
| Diez mutaciones inválidas de un contexto válido | §§2–4, 8–9 | Un solo error por fuente, reparación positiva y comprobación de código, subcausa y primario cuando la regla está activa. |

Huecos actuales: la distribución aleatoria no fuerza MIN/MAX, tamaños
0/1/2/3/4/7/8/9/1024 ni profundidades 32/127/128/129 de F5 §10; esos
extremos requieren recetas dirigidas y puertas de pila de F5-r1 («Pila y
cuotas en cada G»). Tampoco genera fronteras de cuotas ni limpieza de
MockOpen a escala: F5 §§9.4–9.5 fija programas largos fuera del límite de
32 nodos por fuente aleatoria, y `SourceVM.observation_bytes` hoy cuenta
solo stdout. Los literales del ADR ya cubren varias reglas de esos grupos;
no se atribuye a la campaña aleatoria cobertura de los restantes. El
modelo de `for edit`, List y const no escalar permanece fuera del dominio
positivo por F5 §§3.2, 4.1 y 5.
