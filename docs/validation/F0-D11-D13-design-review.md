# Revisión de diseño F0-D11/D12/D13 (Flujo A)

Alcance: docs/adr/F0-D11-closures.md, F0-D12-async.md, F0-D13-expresiones-modulos.md y
F0-CODIGOS.md. Autor del diseño: Codex (gpt-6-astra). Validador: Sonnet 5.5, instancia nueva
sin contexto previo en cada ronda. Coordinación: Claude (juez).

**Veredicto del corte: PASS en la iteración 7.** Vale para este corte de diseño. No aprueba F0,
no sustituye "dos revisores sin discrepancias" ni la aprobación del propietario. Ninguna fila
de las tablas se ha ejecutado: todas son expectativas de diseño (NO EVALUADO como evidencia).

## Historial

| Iter | Veredicto | Bloqueantes principales |
|---|---|---|
| 1 | FAIL | Códigos sin subcausa; tres códigos para "uso tras transferencia"; sin sintaxis de cota de efectos indirectos; ambigüedades copia/movimiento; ampliaciones sin declarar; filas no concretas; `region` sin atar a tareas; colisiones de catálogo |
| 2 | FAIL | E0004 del prototipo sin conciliar; reglas sin código; M17 pasaba por sintaxis accidental; `SuspendSafe` sin declarar; encabezado corrupto; sintaxis `with` dual |
| 3 | FAIL | Texto corrupto (`\n` literales); NX-E1317 contradecía `gen fn`/EX04; M04 y M17 no aislaban su causa; C37 indistinguible de error de sintaxis; `{ expr }` sin terminador sin diagnóstico |
| 4 | FAIL | `=> Expr` sin producción reconocedora (C25/E03); guardas y patrones con modo sin declarar como ampliación |
| 5 | FAIL | `value` reservada contradecía EX01/EX04 (base inmutable) |
| 6 | FAIL | `ModuleDeclStart` omitía `@` (EX02 `@derive`) |
| 7 | PASS | Ninguno |

## Decisiones de producto tomadas por el juez

- `gen fn -> T` declara el tipo del elemento; finalizar o `return;` termina la secuencia;
  `return valor;` es NX-E1324 (completa D10, conserva EX04).
- `NX-E0005` para terminadores ausentes con reparación comprobable (completa D01).
- Forma de valor de rama `value: expr;` con `:` obligatorio, `value` sigue siendo identificador
  ordinario (EX01/EX04 compilan verbatim).

## No bloqueantes abiertos de la iteración 7 (a resolver en el siguiente corte de diseño)

NB1 estatus de `drop(p)` (declararlo ampliación con grafía provisional); NB2 literales flotantes y
`0.5` vs `0..n` sin regla léxica; NB3 `BaseAttribute` sin producción; NB4 patrón tupla sin tipo
ni expresión tupla; NB5 "explicación consultable" (P09) ausente del esquema del catálogo;
NB6 si un `extend` del mismo módulo requiere `use extension`; NB7 negativos sin positivo
pareado; NB8 prioridad E0005 vs `missing_value_operand` en `{ value: }`.

## Límites

Validación por lectura y cálculo con scripts (spans recalculados, 66 códigos, conciliación con
los 13 E y 5 R del prototipo). No se ejecutó compilador. Pendiente: ejecutar fixtures, arneses
y corpus, y contrastar la gramática con un parser y formateador reales.
