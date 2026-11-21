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

## Hallazgo de prueba en S4-r1, paso 4.3 (2026-11-21)

La instrucción general de §5.3 de `I07c-S4-r1.md` exige iniciar **cada** prueba interna con owner heredado `Some((26,41))` y taint del padre `true`, y exige que la inversión del helper `check_against_context_one` falle por la comparación contextual hecha con owner incorrecto. La primera variante de la prueba trató ambas exigencias como si pidieran el mismo diagnóstico: con `tainted=true` en el hecho padre, el helper suprime la comparación contextual. Reproducción interna: en `boundary_context_owner_closes_constructor_before_comparison`, fijar `checker.graph.fact_mut(parent).tainted = true` antes de llamar al helper con `PrecomputedType::CallComplete(Ok(Type::I64))` frente a `Bool`; el resultado obtenido es `Ok(I64)` y la aserción del esperado `NX-E0102` falla por `unwrap_err()` sobre `Ok`. Con taint `false`, la prueba da `NX-E0102`, observa owner heredado tras la síntesis y discrimina la inversión del helper.

No se cambió la semántica del taint: se separó la observación de la comparación (taint falso) de la observación del cierre del hecho (taint verdadero), como se registra abajo.

Antes de este hallazgo, el blob `20227004ec8fb24741dc634aa770799db81c1b0d9b3da983f29ab5a87f499c89` pasó `validate-fast.sh` (2.106 debug / 2.108 release) y las 20 inversiones de §5.3 murieron por `cargo nextest run --profile default` en copia desechable, con 20 fallos dirigidos y restauración SHA-256. Después se mejoraron las pruebas `#[cfg(test)]` para sembrar owner y taint verdaderos en las demás fronteras; esas pruebas internas pasan, pero el hash previo, los veredictos y el fuzz no se trasladan automáticamente al árbol actual. Una pasada provisional del fuzz se detuvo voluntariamente tras 768 fuentes, cero diferencias y cero fallos hasta ese punto. Los veredictos, el diferencial protegido y la comparación A/B de pila del blob final se registran en `I07c-C-S4-informe-r1.md`. Ninguno de estos resultados se declara PASS del corte.

### Resolución en el mismo paso 4.3

El hallazgo anterior afectaba al diseño de la **prueba**, no a la suficiencia de la propuesta: §5.3 permite observar la frontera interna aunque el taint verdadero suprima el diagnóstico contextual. `boundary_context_owner_closes_constructor_before_comparison` ejecuta ahora tres estados del padre: owner heredado con taint falso, sin owner con taint falso, y owner heredado con taint verdadero. Los dos primeros comprueban el diagnóstico y el owner en la comparación; el tercero comprueba que el hijo owner vacío se fusiona antes de retornar y que el padre conserva owner y taint verdadero. La prueba pasó en el corregido. Se reanuda la validación del hash actualizado, sin cambiar la semántica ni la propuesta.
