# Revisión de la implementación F0-0002 / I02b (Flujo B)

Alcance: sub-tarea I02b de docs/adr/F0-0002-second-slice.md (§17): parser, terminadores,
varias firmas, tipos escritos, formas reconocedoras e impresor, aislado de `compile`/`check`/`run`.
Implementación: Codex (gpt-6-luna al inicio y gpt-6-sol para el cierre). Validador: Sonnet 5.5,
instancia nueva y sin contexto en cada ronda, con oráculos propios derivados del ADR.
Coordinación: Claude (juez).

**Veredicto del corte: PASS tras cinco validaciones independientes** (cuatro FAIL con defectos cada
vez más acotados, y una quinta ronda cuyos dos bloqueantes se corrigieron y verificaron por el
coordinador con un crate propio y mutaciones en copias fuera del repo). Vale para I02b, no para
F0 ni F0-0002 completo. I03–I11 no iniciadas.

| Ronda | Veredicto | Defectos bloqueantes principales |
|---|---|---|
| 1 | FAIL | Escáner de características global y heurístico (1.797 falsos positivos en 2.500 programas válidos); terminadores como pre-pase; gramática aceptada de más; FIRST solo correctos con EOF; AST de `else` no normalizado; abort de pila y tiempo cuadrático; NX-E0003 sin emitir; 14 mutaciones supervivientes |
| 2 | FAIL | Pre-pase de terminadores no posicional; `?` como postfijo; FIRST con token erróneo; `function_value` por nombre; sobreconteo de `syntax_nesting`; abort con `consume` repetido |
| 3 | FAIL | `>=` partido con inserción en medio del Vec (tiempo cuadrático); `fn` como inicio de expresión; operando de `return`/`value:`; Name tras `with`; dos mutaciones |
| 4 | FAIL | Estado de `with {` cambiaba con el token erróneo; reparación E0005 sin vigilar con trivia; umbral del hilo ancho sin test de pila pequeña |
| 5 | FAIL | `recognized_prefix` de operador excluido incluía sentencias previas terminadas por salto; aborto de pila en debug bajo el umbral del hilo ancho |
| Cierre | PASS | Ambos corregidos y verificados; hueco de test de restauración de `statement_start` cerrado (3 tests) |

Evidencia independiente de las rondas 4–5: oráculo EOF frente a token erróneo con 5,3 M de pares
sobre 457 programas y 1,9 M sobre 2.229 prefijos de todas las producciones, sin discrepancias
bloqueantes; 72.000 fuentes válidas sin rechazos y round-trip estructural 100 %; 702 ejecuciones de
robustez hasta 1.000.000 de elementos; 78.000 casos de fuzz sin panics; 448–4.688 ejecuciones
`check`/`run` contra HEAD con 0 diferencias; linealidad por byte plana (32 familias).

## Decisiones del juez

- Se descartó el repro D2b (bloque como argumento) por no estar en la EBNF de §3.
- Las lagunas del ADR se mantienen con tests `adr_gap_*` y se documentan en las notas.
- El pendiente nominal de la enumeración exhaustiva de posiciones se resolvió con el oráculo
  independiente del validador, que no halló discrepancias bloqueantes.
- El parser se ejecuta siempre en un hilo ancho acotado (sin heurísticas de umbral).

## Límites

El parser está aislado del CLI. Pendiente: nombres y tipos (I03), propiedad, efectos, IR e intérprete
(I04–I11), y la fusión del addendum de códigos en F0-CODIGOS.md. La validación fue por ejecución,
oráculos propios y mutaciones en copias; sin red.
