# Validación B del plan r17 (S3) — ronda 5 (verificación dirigida)

Base: `git archive origin/i07c-estructural-r2` (60c54e6; types.rs sin cambios, solo documentación). Reproducción en copia
instrumentada (D:\nexo-work\val-B-S3\mut) con registro a archivo en los cinco sitios `VisitCursor::default()`.

## Veredicto: PASS

1. Tabla por sitio y fila `I07C-S3-CURSOR-CLI` verdaderas.
   - Fuentes: 0 impactos en los cinco sitios con `c2` (`5c200ae1…`), `c3` (`32292988…`), RD71, 217 fuentes de
     `corpus/`+`tests/` y 30 programas dirigidos míos. `c2` sí entra en `bind → require_union` (3729, owner `(66,154)`)
     sin pasar por 3423, como dice r17.
   - Alcanzabilidad interna reproducida con prueba temporal propia (`empty_checker`, `new_variable`, `bind`,
     `require_array`, `require_union`, `union`): marcas en 3423, 4748, 4758, 4772 y 4788 (1 vez cada una). Además,
     entrando por `bind` con cursor padre `tainted=true`, owner `(1,2)`, se alcanzan 3423 (vía 3729) y 4772.
   - Inalcanzabilidad desde fuente de 4748/4758: las únicas escrituras de `concrete` son 2406 (tras quitar envoltorios
     Array), 3885 (el bloque Array de 3761–3821 retorna siempre antes) y 4893 (copia de uno existente), y 3418 la borra.
     La inducción de r17 se sostiene. Líneas citadas verificadas.
2. Prueba interna: define el punto de observación (registro `cfg(test)` en el despacho de producción, antes de
   entrar al `bind` hijo, con `(sitio, padre, hijo)`), criterio por sitio (una entrada real, padre `(true,Some)`, hijo
   `(false,None)`, mutante aislado que hereda), contraste con 3732 (hereda) y prohíbe «documentar no alcanzables»
   (si un sitio se elimina, C debe probarlo con diferencial). Es comprobable.
3. No relaja RD71/TD/DG/fixtures, diferencial, «sin recursión por profundidad» ni el ADR; conserva el cursor vacío
   actual (sin cambio de salida).

### No bloqueante
- Para los cuatro sitios de `union_inner` r17 dice «unir con el orden indicado»; `union` no tiene parámetro de cursor,
  así que el «padre» solo existe si el test entra por `bind` (3704) o `unify` (4307), como pide el arco de r14. Verificado
  que esa entrada funciona (4772 alcanzado con padre `(true,Some((1,2)))`). Conviene decirlo en el fixture.
