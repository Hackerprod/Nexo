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
`for_range_break` detecta un break anulado. La segunda corrección añade orden
de raíz/índice antes de RHS, límites `for` izquierda a derecha e igualdad de
Array con longitud. `mutate.py` restaura cada archivo por SHA-256: 48 mutaciones
propias muertas de 48; 24 reconstrucciones de familias descritas por D, 20
muertas y cuatro no observables en esta superficie. Los parches exactos de D
no acompañaban el informe, de modo que esta campaña no se presenta como
reproducción byte a byte. El contrato de `run_compare` ahora se prueba con
ficticios para 18 familias válidas, cuatro formas de corrupción y `check` mal;
también se prueba cardinalidad y exit por separado. Detalle en `MUTACIONES.md`.

| Familias válidas | Reglas F5 | Observación |
|---|---|---|
| Escalares I64/Bool, Len, JoinArrays, if, while, compuesta, For rango y Array | §§2–4; §9.3 | Print del resultado y de una expresión dependiente; constantes variables por semilla/índice. |
| Push de Array anidado y Pop/Option | §4.1; §9.3 Push/Pop/JoinArrays | Copia independiente tras mutar origen; Some y None impresos. |
| For Array ordenado; For rango con break y continue | §§3.1–3.2; §9.3 | Secuencia de Print distingue inversión, break anulado y continue. |
| `and`/`or` con Right que imprime; función de usuario | §2.2; §9.3 | La salida prueba evaluación omitida/tomada y retorno. |
| Const escalar | §5; §8.2 H2 | Print del valor desde L10; antes se exige L09 o L10 pendiente según manifiesto. |
| Quince familias inválidas aisladas | §§2–5, 8–9, 12.2 | Incluyen ciclo y nombre duplicado const, impureza const, préstamo vivo y truncamiento FIRST. Cada una tiene reparación positiva; el contexto heredado no introduce un `for` pendiente en `types`. |

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

La frontera de cuota storage-96 ±1 se comprueba en `check_charge_literals`
según F5 §9.4. La generación CLI de cuotas sigue pendiente del perfil de
prueba activado por L08; no se atribuye esa cobertura al corpus G1b.

## Corrección O7: repros dirigidos

`model_pairs` fija el cierre inverso de dos recursos (F5 §4.2), `Some([1])`
y `Some(Some([1]))` para payloads transitivos (F5 §4.1). La familia válida
`array.pop_option` alterna escalares, Array y Option<Array>; la inválida
`const.duplicate_name` reproduce F5-N02 y fija NX-E0104/names antes de L09
(F5 §8.2 y §9.2.1). Una respuesta JSON literal de F5-O02 fija
`simulator_fault`/exit 1/R0502 para `run` (F5 §9.1); el mock no la construye
con `projection`. El contrato de CLI exige exit no cero ante fallos de `fuzz`
y divergencias de `gate`, y el comparador rechaza tipos JSON distintos.
La familia F5-N02 comienza después del índice 100 porque el binario
`d86cd26` aún emite dos barreras L09 en lugar de NX-E0104; el hallazgo y
la clasificación exacta están en `HALLAZGOS.md`.

Los mutantes de D N07, N18, N20 y N24 son equivalentes dentro del
subconjunto generado y del contrato observable actual:

- N07 elimina una copia de parámetro `view`; los casos generados no mutan el
  objeto original a través de otro alias mientras la vista está viva, por lo
  que stdout y eventos son iguales.
- N18 cambia la etapa de la barrera L10. Los manifiestos son prefijos y L09
  debe estar completa antes de L10; antes prevalece otra barrera.
- N20 omite la comprobación de aridad de una llamada en la VM. Los programas
  generados y los pares positivos tienen aridad válida; la comprobación es
  inalcanzable en ese conjunto.
- N24 elimina control sin efecto sobre la proyección de ese conjunto. El
  test dirigido no distingue salida, diagnóstico ni eventos en esos casos.

N04, avance de rango `+1` a `+2`, se aplica ahora con un parche único en la
rama `for` de `SourceVM` y se contrasta con `range_bounds_left_to_right`.
Los mutantes equivalentes quedan como brecha conocida de observación del
subconjunto, no como afirmación sobre programas Nexo fuera de él.

## Corrección O8: `while` en función no unitaria

El par literal `fn F()->I64{while false{}}fn Main(){}` fija
NX-E1317/ownership/flow.return en el cierre de `F`, con `expected_type`
en `I64` (F5 §2.2 y §9.2.1). La familia `control.while_return` genera
condiciones `false`, `true`, nombre Bool y `((true))`; `selftest` comprueba
también que un `return` final evita L04. N19 y sus tres variantes descritas
por D quedan discriminados por estos casos.

### Brechas conocidas, sin bloqueo

- N14b: el cambio `simulator_fault` → `rejected` en otras filas runtime de
  `literal_cases` no tiene todavía un esperado independiente; F5-O02 sí lo tiene.
- D6b/D6c: `_first_difference` carece de casos dirigidos para lista esperada
  frente a cadena de la misma longitud y para escalar `1` frente a `true` o
  `"1"`; el caso de diccionario sí está fijado.
- N18b: falta un par para la operación `ir.lower` de L10; requiere G5 con
  `--complete` hasta L09.
- DX-1..4: los cuatro mutantes ya clasificados como no observables en la
  superficie actual permanecen pendientes.
- No comprobado por D: G4+/L08 con binario real, `charges.py`, semillas fuera
  de 0:7 y captura independiente del exit literal de `sh validate.sh`.
