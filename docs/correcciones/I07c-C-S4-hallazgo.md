# I07c S4.1 — bloqueo por aumento de pila

El árbol de implementación de S4.1 **no está listo para D ni para commit**. Se detiene el trabajo conforme a la instrucción de volver a A/B si el mecanismo del plan no basta. No se modifica la prueba ni el umbral para aceptar el resultado.

## Reproducción

- Fuente `J16-S1C-option_match126` del arnés versionado `scripts/i07c_stack_probe/probe.py` (`sha256=9d702f634956ba899926253718fce6f6e47c99c35ef48367d975996d611c7312`). Ejecutable de pruebas debug S4 `C:/nexo-work/target-s4/debug/deps/nexo-ae65c584bd3eb2e6.exe`, modo `TYPES`: `--kib 65 --case option_match126` termina con `3221225725` (desbordamiento de pila). La misma fuente pasa a 129 y 204 KiB. En la serie `--minimum --case j16`, 65 KiB falló 5/5 y 129 KiB pasó 5/5.
- Fuente `J16-S1C-result_match126` (`sha256=c15e432a269f65c625d6b17b613039ee004839a8328febd698f3442d8144b45c`), mismo ejecutable/modo: 65 KiB termina con `3221225725`; 129 y 204 KiB pasan. La serie dio 65 KiB fallido 5/5 y 129 KiB verde 5/5.
- Esperado por `docs/correcciones/I07c-S4-plan.md` §9 y `docs/correcciones/I07c-C-S3-informe.md`: ambas filas deben conservar el umbral observado S3 de **65 KiB** en TYPES y CHECKER. El plan prohíbe empeorar cualquier fila S3. Las ocho filas sí pasan a 204 KiB debug y 256 KiB debug/release en ambos modos; eso no subsana el aumento del umbral.

Se descartó como causa suficiente la comprobación adicional de parentesco añadida durante esta sesión a `InferenceGraph::fact` y una llamada de `fact_mut` a `fact`: se retiraron y el desbordamiento de `option_match126` a 65 KiB persistió. La causa concreta del crecimiento de marco o de ruta de llamada aún no está medida. Resolverla puede exigir ajustar el mecanismo o el reparto de marcos; C no introduce esa decisión fuera del plan validado.

## Estado de la implementación al detenerse

Los cambios sin commit están solo en `src/check/types.rs` y `tests/i07c_deferred_constraints.rs`. Se migraron los cursores a hechos con `FactId` y generaciones del `InferenceGraph`; las regresiones internas de reentrada, seis sitios de `Bind`, rollback y causas independientes pasan individualmente. El binario S4 dio RD **152/152** en debug y release. El diferencial versionado de **369 fuentes** frente a `C:/nexo-work/bin/3100a29/{debug,release}/nexo.exe` dio cero diferencias en `check`/`run`; SHA-256 de referencias: debug `6edf41c63b45fbd00bd7d0acf43901f5539d463a60fa5cfd10f2c68ae0930169`, release `e8aabcf7b94d3948cdad0d15ae74e8d847f8df68899b52c41bab58f9a9d6b112`.

No se ejecutó la puerta final ni se realizó mutación inversa/campaña: el incumplimiento de pila bloquea la entrega. El código queda en el worktree para inspección de A/B; no se hizo commit, push ni operación mutante de git.

## Resolución (2026-11-21)

El diagnóstico medido (`I07c-C-S4-diagnostico-pila.md`) muestra que, a perfil idéntico, `be8c47c` y S4 tienen el mismo umbral; el plan se enmendó (registro r4 de `I07c-S4-plan.md`, validado por B en `I07c-plan-B-S4-r4.md`) para fijar el perfil de referencia y declarar el crecimiento de marcos como coste medido. Este hallazgo queda como historial; C reanuda la puerta final.
