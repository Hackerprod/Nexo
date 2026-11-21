# Validador B - verificacion de la tabla de FAMILIA (§2) de I07c-S4-r1.md (paso 4.2)

Veredicto: PASS (la tabla es completa en sitios; sin sitio omitido que cambie la salida). Tres imprecisiones no bloqueantes que conviene corregir en el texto antes de que C implemente.

## Metodo
- Inventario independiente desde `git show be8c47c:src/check/types.rs` (identico a 8aba06f) frente a `git show 8cd5afe:src/check/types.rs`: 139 lineas de limpieza/guardado/restauracion de owner/taint en las regiones de drive, resume, helpers y closure (take(), `= None`/`= false`, `previous_*`, `prior_taint`, `inherited_numeric_owner`), cotejadas con los 33 `child_fact` de S4. Lectura completa de S4 para Statement/Assignment, Array, Index, Binary (los 5 marcos), Call, UnionCall, CallTarget, IntrinsicOne, TryAdd, AssertEq, closure_body_advance, if_advance y branch_accept.
- Dinamico (solo ejecutables ya compilados, sin recompilar): B1b `C:\nexo-work\bin\3100a29\release` frente a (a) S4 `C:\nexo-work\target-s4\release` (reproduce el defecto de D; su mtime 03:07 es anterior al commit 8cd5afe 03:37, por lo que lo uso solo como detector) y (b) el candidato de C `C:\nexo-work\val\c-s4r1\candidate-release-nexo.exe` (SHA-256 d0dfa506..., el declarado en §5). Perfil `f0-0005-b-v1`, JSON completo.
  - Barrido 1: 360 fuentes (izquierdo true/TryAdd(p,1)/(true+1) x operadores + - * and < x 12 operandos derechos con `r`/`[r]`/`r[0]`/`or`/`and`/`if`/Some/Len/... x 2 colas de asignacion): S4 da 24 diferencias; candidato 0.
  - Barrido 2: 66 fuentes con los mismos nucleos envueltos en `[ ]`, Some, TryAdd, AssertEq, `if`, Len, unario, indexacion, suma: S4 22 diferencias; candidato 0.
  - Barrido 3 (taint): 300 fuentes (5 primeras sentencias contaminadas x 20 consumidores x 3 niveles de bloque): 0 diferencias en S4 y candidato.

## Clasificacion independiente de la base (la raiz de la familia)
Restauracion del owner ANTES de la comparacion contextual (la heredada): solo `Frame::Array` (8805 antes de `ArrayComplete`) y `Frame::Index` (8822, a la entrada del marco). Restauracion DESPUES de `check_against_context_one`: Binary (9150, 9171, 9194, 9243, 9288, owner None durante toda la comprobacion), Call (9348), CallTarget/ClosureCall (9407, 9430, 9460), UnionCall (9484 `= None` y 9499), IntrinsicOne (9526), TryAdd (9535/9572), AssertEq (9577/9600/9611). Por eso el helper de S4 (hijo owner vacio hasta el final) solo cambia el resultado cuando se llama con owner heredado no nulo: Array completo, Index completo y el `Whole` de `and`/`or` (LogicalRight, que no limpia owner), mas cualquier cosa que llegue ahi bajo un `+ - * / %` (F5). Para Call/UnionCall/IntrinsicOne/TryAdd/AssertEq/Binary el owner ya es None en base, y el hijo vacio del helper no cambia nada.

## Comprobaciones de la tabla de C
- Fila `check_against_context_one`: correcta y suficiente; la correccion propuesta (cerrar el hijo al terminar `expr_inner`/`Whole`) elimina las 46 diferencias halladas en los barridos 1 y 2.
- Binary: la ordenacion de S4 reproduce la de base (owner None en `Frame::Binary`, `BinaryRight/DeferredRight/Independent*`: `fact_mut(binary_fact).numeric_domain_owner = None` tras fusionar el hijo RHS, comprobacion bajo ese hecho, `merge_to_parent` despues); RHS de rutas independientes hereda `Some(op)` (child Empty/Inherit) como base 8955/9031; rutas normales Inherit/Empty como base. Sin sitio omitido.
- Statement/closure/assignment/branch value/If cond: (taint Empty, owner Inherit) coincide con base (take de taint sin tocar owner); fusiones por `merge_child`/escritura explicita del taint del padre (StatementSite, closure ~2886-2942, if_advance condicion, branch_accept 13537 con `merge_taint=false` y taint combinado a mano) equivalen a 7795-7798, 8011/8161, 13000-13002, 13396-13397.
- Retornos anticipados (AnalysisLimit): S4 fusiona/retira el hijo antes de `Resume::Next` en Call/TryAdd/AssertEq; en base el cursor no importa tras un AnalysisLimit. Sin hallazgo.

## No bloquean (correcciones de texto/pruebas)
1. La fila "If y Match" del §2 no recoge que un valor de rama que sea Array/Index/`and`-`or` cae en la misma familia: `TryAdd(p,1) - (if p then [r] else r)` y `true * ...` (barrido 1, p. ej. a139, a163, a187; barrido 2, b53/b9) difieren en S4 y se resuelven con la correccion del helper. Anadir esa ruta a la fila y a la matriz de §5.4 (hoy solo `or`, `and`, `+`, `-`, `<`, `==`; faltan `*` `/` `%` de F5 y el envoltorio por `[ ]`, Some, TryAdd, AssertEq, `if`, Len, indexacion).
2. Las filas "Some/Ok/Err y payload", "TryAdd" e "Intrinseco de un argumento" dicen que su comparacion queda "expuesta al mismo limite incorrecto": en base el owner ya es None en esos tramos (9484/10302/10331), asi que el hijo vacio no altera nada. La exposicion real es de Array, Index y el Whole logico. Corregir la clasificacion; el repro con `TryAdd` es TryAdd como operando izquierdo del `+`, no un sitio distinto.
3. El criterio de la raiz conviene escribirlo en el §2: "restauracion antes del consumidor" (Array, Index) frente a "despues" (resto); asi D y D' saben que cualquier otro llamador con owner heredado (p. ej. un futuro marco F0-0004/B2b) cae en el mismo defecto.

## Limites
- No audite linea por linea los 33 `child_fact`; revise los de las familias anteriores y el barrido dinamico es una muestra (448 fuentes), no exhaustivo. Un barrido de taint sin diferencias no prueba ausencia total en esa familia.
- El ejecutable `target-s4` es anterior al commit 8cd5afe: cualquier diferencia entre ambos no esta cubierta.
