# Validación B-r1 de la propuesta I07c-laguna-b2d (Flujo A)

**Veredicto: FAIL** (3 bloqueantes de redacción/justificación; la decisión de fondo, opción A, es correcta). Ninguna salida fijada por el ADR cambia: verificado contra RD84(a), RD86(a-b), RD87(a-d), RD99, RD116, RD125, TD04, TD05. Las únicas filas con `Len` son RD86/87/125; ninguna contiene la arista circular.

## Por requisito
1. Decisión A vs B: PASA (con reserva del bloqueante 1). La base ya rechaza los controles directos (`Len(r)` con `[a0,r]`, `1+Len(r)`, `[1,r]`, `let r=3`); reproducido con el binario `D:\nexo-work\target-i07c-l\release\nexo.exe` (v2..v8: not_indexable en los primarios de la tabla, check y run). Aceptar b2d haría que una envoltura `DV+(DV-...)` cambiase la aceptación frente al control directo, contra N(a) (forma equivalente con las mismas restricciones). El estado «aceptar» de B no es punto fijo: con `Len(r)` válido el nominal está disponible (§4.1.2.3), `a0:I64`, y la igualdad de homogeneidad se acumula (cláusula 2), luego refuta el dominio. No hallé oscilación ni dependencia de orden con A.
2. Texto normativo: FALLA (bloqueantes 2 y 3).
3. No contradice salidas fijadas: PASA.
4. Tablas: PASA. Offsets recalculados a mano (DV=39 bytes; b2d `Len(r)` r=[148,149), último r=[162,163); controles [67,68), [69,70), [34,35), [79,80), [67,68), [68,69), [69,70]); la fila «Base» reproducida (b2d `checked/unit`, resto `rejected`, positivo `checked`). Código, `stage/operation` (`types`/`types.check`, ADR §13) y `data` coinciden con cláusula 3 y §13.15–13.17.
5. Mecanismo único: PASA. Es una arista del grafo (igualdad consumidora marcada como testigo), coherente con plan §4 (propagar invalidación solo a dependientes).

## Bloqueantes
1. **Argumento falso contra B (sección «Opción B»).** Afirma que, retirada la igualdad y conservado `I64` en `a0`, «el elemento `r` queda irresuelto y §4.1.2.2 exigiría NX-E0103». Falso: el elemento del constructor `[a0,r]` sería `I64` (referencia = `a0`); `r` es un binding pendiente, no un elemento, y el ADR acepta bindings pendientes sin constructor (cláusula 3, `Print(q)`). Solo la segunda rama (retirar también el nominal) produce NX-E0103. El plan §8.4 exigía explicar este punto. Corregir: rechazar B por (i) violar «las igualdades compatibles se acumulan» (cláusula 2) y la disponibilidad nominal de §4.1.2.3 (el estado aceptado no es estable) y (ii) N(a) frente al control directo; eliminar o acotar la rama NX-E0103.
2. **Los pares (1) y (3) del plan §8.4 no están en el texto a insertar.** El plan dice que «el ADR que cierre b2d fijará también su aceptación o rechazo, diagnóstico y primario» para `var r=DV;let a0=Len(r);let b=[a0,r];` ([67,68)) y `...1+Len(r)...` ([69,70)), con sus positivos. Solo aparecen en la tabla de variantes, fuera del bloque «Texto exacto». Incluirlos (con el positivo `1+Len(r)`; hoy falta) en la tabla normativa insertada, con campos de §13.15–13.17.
3. **Ambigüedades de redacción.** (a) «retira el resultado nominal ... para diagnósticos posteriores»: deja abierto qué ocurre con un consumidor ya comprobado antes. Repro (base): `fn Main(){var r=DV;let a0=Len(r);let c:Bool=a0;let b=[a0,r];}` emite un segundo `type_mismatch` en `[81,83)`; la regla debe decir «derivados, descubiertos antes o después» (V según §4.1.2.5, presencia I). (b) Ubicación: «después de la tabla "Ejemplos y diagnóstico exacto"» no es unívoca; tras ella vienen el párrafo de offsets y la tabla «Variantes de la misma regla». Fijar: tras la tabla «Variantes de la misma regla» (ADR línea 335 actual) y antes de la cláusula 4 (línea 337) o, mejor, inmediatamente tras el párrafo que acaba en «...(RD86). No hay NX-E0103 por q sin constructor...» (línea 307).

## Registrable (no bloquea)
- Los nombres de opción están invertidos respecto al plan §8.4 (allí A=retirar, R=retener); unificar al citar.
- Añadir variante: `...let b=[a0,r];r=[1];` (base: not_indexable en [67,68) más `type_mismatch` en [85,88)), que conserva el C del `r=[1]` posterior por orden fuente.
- La regla afecta en la práctica solo a `Len`; ningún otro intrínseco con nominal y dominio no unitario produce el ciclo (probé `q<q`: dominio unitario, ya cubierto por igualdad).

Inserción tras corregir: ADR §4.1.2.3, entre la tabla «Variantes de la misma regla» (o la línea 307) y la cláusula 4.
