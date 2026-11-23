# Validación B del diseño I07c-S5c (ancla de unión) — ronda 2: **PASS**

Los seis motivos de r1 están resueltos; sin contradicciones nuevas en la matriz ni en el argumento de realimentación. La parte de C2 de las cifras queda INCONCLUSA (B no recompiló C2). Matriz: 45 filas (OK `main` 22, V6 25, C2 31), cuadran fila por fila; `main`/V6 comprobadas con binarios en ~33 de 45 filas. 12 casos nuevos rederivados por bytes desde el ADR (W0, W0nounion, X_mix3, X_nest_first, F06, A, A sin asignación, A control, D-REF inválido y válido, Y_pend_if_asg, P2): todos coinciden con la matriz y con V6.

## ¿Puede pasar a plan de implementación por bloques? Sí. Condiciones
1. **B1 sobre `main` sin V6:** conserva los 22 OK de `main` y arregla R4, X_R1na, X_R1ord, X_flat3, Y_pend_if e Y_pend_if2; después B2 con V6 encima.
2. **W0:** medirlo en B1; si ALT-2 no lo arregla, registrarlo en `Pending.md` con fecha y causa antes de cerrar I07c (directriz 2026-11-21: no se cierra el tramo con un caso señalado sin adjudicar).
3. **Diferencial sin muestreo:** todas las diferencias frente a `main` y V6, en ambos órdenes de asignación, tres perfiles, debug/release, `check`/`run`; cada diferencia clasificada contra la matriz, 0 sin clasificar.
4. **Oráculo independiente** validado antes contra la matriz de 45 filas (incl. W0 y X_mix3).
5. **Pasadas de planning:** contarlas sobre matriz y corpus (ninguna debe llegar a 129) y barrido de permutaciones de asignaciones (todas las de 2 y 3, compatibles e incompatibles, antes y después de la unión).
6. **Pila:** medir `scripts/f5-frame-sizes.py` en cada commit con aborto por encima de 2.500 B de crecimiento de la pareja; `scripts/f5-stack65.py` dentro de `validate.sh`.
7. **Regresiones y mutación inversa** como en §5 del diseño, incl. el negativo aislado de `X_mix3` y los positivos R3c y X_R1pre.

Higiene: B sobrescribió `C:\nexo-work\tmp\adr.md` con el ADR de HEAD de este worktree (ya existía un `adr.md` ahí que pudo ser de otro agente; no se pudo comprobar).
