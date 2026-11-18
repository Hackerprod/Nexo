# Validación B del plan r16 (S3) — ronda 4

Base: `git archive origin/i07c-estructural-r2` (c18f229), binario de la base y una copia desechable instrumentada
(D:\nexo-work\val-B-S3\mut; no toca el worktree).

## Veredicto: FAIL (2 defectos de plan comprobables)

### Reproducido (PASA)
- Las tres fuentes del hallazgo y RD71 dan en `check`/`run` exactamente lo que dice el hallazgo
  (checked/unit; dos `NX-E0102/type_mismatch` `[106,107)` y `[133,134)`; checked/unit; RD71 único `[136,138)`).
  El SHA-256 de la tercera fuente está truncado en el hallazgo (48 hex); el de la fuente reproducida
  empieza por `32292988640e56c0`. No bloquea.
- Mutante propio que pasa el owner del padre a los 5 sitios con `VisitCursor::default()` (3423, 4747/4757/4771/4787)
  y a las entradas `union`: JSON idéntico en las 3 fuentes, RD71 y 21 programas míos (`check` y `run`). Coherente con AGENTS
  «Rigor proporcional» (superviviente sin programa no bloquea). Líneas 1498–1505, 3846, 4120–4167, 5861–5940
  existen; `bind` no consulta `tainted`. No se relajan RD71/TD/ADR; r16 no cambia comportamiento (conserva cursor vacío).

### Motivos
1. **La evidencia de la tabla de r16 no ejercita los arcos que dice evidenciar.** Instrumenté cada sitio con
   cursor vacío (3423 en `require_union`; 4747/4757/4771/4787 en `union_inner`) con un registro a archivo.
   Resultado: `c2` (fuente `5c200ae1…`) llega a `bind → require_union` (3729, owner `(66,154)`, 4 veces) pero
   **ningún** sitio de cursor vacío se ejecuta; `c3` (`32292988…`) no ejecuta ninguno; tampoco las 217 fuentes de
   `corpus/`+`tests/`, RD71 ni 21 programas dirigidos (0 impactos en total). Los `bind` de payload que C
   observó «bajo `require_union`» son los de 3732 (ya heredan `visit`), no los de 3422–3429. Por tanto
   «mutación aislada ... JSON idéntico» y «A pasó el cursor padre por ... sus cuatro bind hijos» no prueban
   «herencia no observable» en esos arcos: la mutación se aplicó a código no alcanzado. Corregir r16 y la fila
   `I07C-S3-CURSOR-CLI`: decir que ninguna fuente conocida alcanza esos cinco sitios (alcanzabilidad
   desconocida), no que fueron trazados y mutados con JSON idéntico.
2. **La prueba interna exigida no es comprobable tal como está escrita.** r16 la hace la única garantía y pide
   «cada bind hijo empieza con ambos campos vacíos», pero no define cómo se observa el cursor de un hijo
   (`bind` recibe `&VisitCursor` y lo consume internamente; tras S3 es un valor de marco), y deja escape
   «documenta los no alcanzables» sin criterio, de modo que los cuatro sitios de `union_inner` (y 3423)
   podrían quedar sin ninguna prueba. Con el punto 1, esos sitios pueden ser inalcanzables desde fuente.
   Fijar: (a) el punto de observación (función/constructor de marco que recibe arco y padre y devuelve el cursor
   hijo, o estado de grafo construido a mano que alcance cada sitio); (b) para cada sitio, o prueba que falla con
   un mutante que hereda, o demostración de inalcanzabilidad desde el grafo de llamadas; (c) que la ausencia de
   mutante discriminante se registre por sitio.

### No bloqueante
- Truncamiento del SHA-256 en el hallazgo.
