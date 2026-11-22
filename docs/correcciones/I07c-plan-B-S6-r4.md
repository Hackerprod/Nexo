# Validacion B (ronda 4) del plan I07c-S6 - veredicto PASS

Base HEAD 4b73f04, solo lectura. Plan de 128 lineas, revisado frente al unico bloqueante de la ronda 3.

## Bloqueante de la ronda 3: corregido
- Mismo criterio para 8283, 12301 y 14613: tabla §3.3 (lineas 66, 69, 75) los marca los tres `pendiente_decision`; ninguno se llama defecto probado ni conformidad. 8283 y 12301 documentan el repro observado (`for x in ([1])` => [19,24); `(x)(2)` => [18,21), ambos reproducidos por B en la ronda 3); 14613 declara expresamente que no hay repro de esa rama ni se afirma defecto.
- Esperado `[20,23)` presentado como DERIVADO por analogia con F0-0003 l.91 (NX-E0102/type_mismatch sobre la misma expresion iterada; `for x in (1){}` => [20,21), comprobado), "NO literal ni normativo todavia"; se reconoce que l.90 no decide grupos y que ambas lecturas son razonables (§3.3 parrafo "Laguna", fila K1-d2).
- §4.1.2.1 ya no se cita como decisorio: se restringe a "primarios de restricciones de tipo" y se dice que la barrera `later_adr` no es una restriccion (fila K1-d2, tabla, §3.3).
- K1-d2 = NO EVALUADO/pendiente de decision para esos primarios (cabecera, fila K1-d2, §3.3, §3.5, §4.1, §4.4, §4.7, §5); el exit 0 del auditor y las metamorfosis de restricciones no lo cierran.
- Sin test rojo en `tests/suite.rs` (fila K1-d2, §3.3, §4.7). Destino: si el coordinador decide por Flujo A "sin grupos" y el comportamiento permanece, defecto + corte `I07c-S6-COR-<id>` con regresion (§3.3, §4.1, §4.7, §5). Si decide conservar grupos, se registra la regla y no se llama defecto.
- Coherencia del resto: no quedan `defecto_conocido`, `diagnostic_target` ni "K1-d2 FALLA" (los dos hits de "defecto confirmado/probado" son negaciones). La lista de clases del auditor (linea 78) coincide con la tabla de 12 sitios: primary_unwrapped, pendiente_decision, deferred_result, pattern_primary, pattern_relation, quota_charge, argument_target. La fila de mutacion de `grouped_operands_keep_one_causal_primary` aclara que no acredita 8283/12301/14613. Las filas ya validadas (19 filas, P-8, N-a.3 PARCIAL, T-TD-ord INCONCLUSA, filtro RD decidible, Q) no cambian.

## No bloquea
- Antes de C: revalidar blobs y lineas de types.rs tras actualizar la rama remota (e479d62), segun el propio plan.
- La decision de Flujo A sobre primarios con grupos queda fuera de S6; K1-d2 no puede cerrarse en la matriz hasta entonces (el plan lo establece).
