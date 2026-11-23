# Validación B, confirmación final dirigida del plan I07c-S5c (139 líneas, commit 2bff6ae): **PASS**

Solo lectura del plan; sin compilar, sin procesos.

## 1. Contradicción resuelta, sin otra nueva
- §3.3 (l.39): con clave `Absent` y conflicto, `anchor_contrast` devuelve `Reported(error)` si hay error y `Pending` si solo creció la cola; no marca registro ni trunca; informa siempre del crecimiento y decide el llamador.
- Tabla de §3.4: `if_after_first`/`Absent`: error inmediato a `anchor_error`, `Pending` a `anchor_pending` (como `main`, l.14111); `second_leaf_anchor`/`Absent`: guarda `anchor_error` de `main` y `Pending` ignorado (como `main`, que no calcula pendiente en la hoja `else`).
- §3.6 (l.62): describe lo mismo (primera hoja con `anchor_error` o `anchor_pending` locales, de modo que `if_finish_join` ve lo mismo que en `main`; hoja `else` con guarda solo para clave ausente, `Pending` ignorado, sin truncado).
- Las filas `Open` y `Denounced` no cambian y siguen siendo coherentes con §3.3. La PARADA de §9 incluye la guarda `match`. Sin contradicciones nuevas entre §3.3, §3.4, §3.6 y el resto.

## 2. `8 bis` recoge mis condiciones
Sí, las seis: documentos del oráculo (`orden-no-fijado.md`, `oraculo-vs-matriz.md`) validados por B antes de B1-S1 con los tres negativos y la guarda X_two_unions en la matriz; B1-M8 con registro diferido/inmediato y sustitución del programa si sobrevive; B1-M5, M6 y M7 con sus killers y restauración por SHA-256; B2 contra B1 con P2 y la observación de A (main una E1303 en `1`, V6 añade otra en `3`, ya reproducida en la ronda anterior); D reproduce con su binario (`check` y `run`, debug y release), confirma guarda `match`/`Absent` y la pila contra la línea base de S0. La eliminación de `rows["mutation-grouped-array"]` (condicionada a `rg` y, si algo la cita, nota fechada en `Pending.md`) está bien decidida.

## 3. Tamaño y decisiones abiertas
139 líneas (≤ 220). Sin «B decide»; los condicionales (X_R1inc, W0, mutation-grouped-array) tienen salida definida.

## Resultado
PASS. El plan queda validado por B y puede implementarlo C (Sonnet) cuando el orquestador lo apruebe y E haya escrito el oráculo y validado `orden-no-fijado.md` y `oraculo-vs-matriz.md`. Reservas que ya son condiciones de `8 bis`, no defectos: B1-M8 por verificar al compilar y pila contra la línea base medida (60.616 B según los informes S6COR, no 60.920).
