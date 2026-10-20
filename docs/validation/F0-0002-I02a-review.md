# Revisión de la implementación F0-0002 / I02a (Flujo B)

Alcance: sub-tarea I02a de docs/adr/F0-0002-second-slice.md (§17): léxico, tokens y spans
(negativos LX01–LX05). Implementación: Codex (gpt-6-luna). Validador: Sonnet 5.5, instancia nueva
y sin contexto en cada ronda. Coordinación: Claude (juez).

**Veredicto del corte: PASS tras 3 iteraciones** (2 FAIL y una verificación final de las dos
mutaciones pendientes, ejecutada por el coordinador en una copia fuera del repo). Vale para I02a, no
para F0 ni F0-0002 completo. I02b–I11 no iniciadas. El lexer queda aislado en `src/frontend/lex.rs`
y no está conectado a `compile`, `check` ni `run`.

| Iter | Veredicto | Bloqueantes |
|---|---|---|
| 1 | FAIL | Siete mutaciones de la frontera léxica sin test; documentos que decían falsamente que I01 seguía pendiente de revisión |
| 2 | FAIL | Faltaban tests de "`-` es un token separado" (§3.2/§3.5) y de "`/*...*/` no es comentario" (§3.2) |
| 3 | PASS | Ninguno: ambas mutaciones fallan en su test específico (`minus_is_always_a_separate_symbol_token`, `block_comment_syntax_is_rejected_at_its_first_slash`) |

Evidencia independiente de la iteración 1–2: oráculo derivado de §3 con 9.723 entradas sin
discrepancias; LX01–LX05 byte a byte; 65 casos de `check`/`run` idénticos a HEAD; fuzz de 70.000
iteraciones sin panics; tiempo lineal (×4 al pasar de 1 MiB a 4 MiB).

## Lagunas del ADR registradas (comportamiento actual fijado con tests `adr_gap_*`)

El conjunto de "espacios" (el lexer usa `char::is_whitespace`), `1a` (Integer + Name), la política de
cadenas (escapes, cierre antes de LF y saltos de línea dentro de cadenas) y `\r` aislado (se emite
como Newline aunque §13.8 cuenta líneas solo por LF).

## Límites

Validación por ejecución de la suite, oráculo independiente y mutaciones en copias; sin red. El
parser, el AST y el impresor (I02b) y el resto del §14 no se cierran en este corte.
