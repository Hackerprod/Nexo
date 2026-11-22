# Validacion B de I07c-laguna-span-agrupaciones-r1.md  -> FAIL (4 motivos; la decision de fondo es coherente)
Binario: release.exe de main (cache 9d7ed0a...). Fuentes en blob 4b73f04. Sondas: pp.py, sweep.py (esta carpeta).

## Pasa
1 Citas literales verificadas: F0-0002 l.299 (4.1.2.1 punto 1), l.1038, l.1155 (bajo 11 "Origen de los ticks"), 13.13 "Convencion de spans", F0-0003 l.90-91, 14.10 "Los grupos que rodean una expresion no forman parte de su primario ni de sus relaciones de tipo" (l.3258). Apoyo adicional no citado: F0-0002 l.359 ("sin etiqueta ni grupos") y l.602 (drop((a[0])) sobre la proyeccion completa), 3.6 l.230 ("Despues de quitar agrupaciones").
  No contradice 13.13 ni F0-0003 l.468 (argumento completo = con prefijo move/edit, no grupo).
2 Ejemplos: los 12 "antes" de la tabla reproducen exactamente con el CLI (offsets y subcausas); los "despues" son coherentes con la regla (offsets recomputados a mano). run coincide con check.
3 Salidas protegidas: 152 RD + tests/fixtures (json/jsonl/.n) + corpus = 758 fuentes, 1481 diagnosticos, 0 primarios que sean un grupo balanceado. Tests Rust revisados: ninguno fija un primario con parentesis (los de types.rs:16618, i07c_mutation_regressions:1331/1631 fijan tablas/relaciones y primarios ya interiores). docs/baseline sin spans.

## Bloquea
B1. Inventario incompleto: sitios de produccion que hoy emiten primario agrupado y no figuran (repros, perfil f0-0003-b-v1, check):
 - lower.rs:9194 (Drop(arg) check_available) y 10524 (Unary Consume): `fn Main(){let a=Some(1);Drop(a);Drop((a));}` -> NX-E0202 [37,40) `(a)` ; `...Drop(a);let b=move (a);}` -> [43,46) `(a)`.
 - lower.rs:8705/8722 (scrutinee de match no rooteado/rooteado): `fn Main(){let a=Some(1);Drop(a);match (a){_=>{}}}` -> NX-E0202 [38,41) `(a)`; con `match move (a)`/`match edit (a)` [43,46).
 - NX-E1105 (conflict()): `fn Main(){var a=Some(1);match edit a{Some(x)=>{Drop((a));},None=>{}}}` -> [52,55) `(a)`; idem `match edit (a)`, `match (a)` y `F(edit (a))` (f0-0005-b-v1).
 - lower.rs:9033-9135 (NX-E1107, target.span() crudo) no esta en la tabla.
 - NX-E1104: `fn Main(){let f=if true then {let x=1;value:(fn [view x]()->I64{return x;});}else{return;};Drop(f);}` -> [44,75) con parentesis (el sin grupo es [44,73)). La fila lower.rs:4810 lo dice "condicionado": hay repro.
 La fila effects.rs dice "comprobar"; hay repro (B2).
B2. Defecto de grupo en efectos, no cubierto: `fn Main() with no_io{(Print)(1);}` -> NX-E0415 primario VACIO [21,21) (sin grupo: [21,26) `Print`); `fn F(){Print(1);}fn Main() with no_io{(F)();}` -> NX-E0415 [38,38) (sin grupo [38,39)); `((Print))(1)` igual. Un primario vacio de una regla de efectos es erroneo bajo 13.13 y l.702 (primario = nombre del destino sin parentesis). Esperado tras la regla: `Print` [22,27) y `F` [39,40). Debe aparecer en inventario, en ejemplos del addendum y en regresiones del COR.
B3. Regresiones del COR omiten las familias NX-E0202, NX-E1105, NX-E1104, NX-E1107 y NX-E0415 (arriba); la lista actual solo nombra E0006/E0111/E0105. Anadir al menos un ejemplo valido/invalido/diagnostico por familia al addendum o declarar expresamente que el plan COR las enumera.
B4. El texto del addendum dice "ninguna regla publicada denuncia los parentesis" correcto, pero el documento afirma en "Inventario" que la tabla clasifica el flujo; con B1-B2 esa afirmacion es falsa. Corregir tras B1-B3.

## No bloquea
- Citar F0-0002 l.359 y l.602 como apoyo de la regla; precisar que "tabla N" de 4.1.2.1 es la clausula 1 de la lista N.
- Aclarar que un prefijo move/edit no es grupo (F0-0003 l.468 "argumento completo").
- Inconsistencia preexistente fuera de regla: NX-E0202 sobre `(a)[0]` da `a` en `let b=(a)[0]` y `(a)[0]` completo en `match (a)[0]`; la regla no la cambia (Index con hijo agrupado), anotarlo.
- E0003/expression_tree_depth: la altura no cuenta grupos (l.2456) y syntax_nesting salta antes; sin riesgo observado.
- Mecanismo unico (funcion primario-sin-grupos, primary_span ya existente types.rs:7075-7082) realista; riesgo de reordenar diagnosticos empatados por offset correctamente anotado.
- Coherencia con I07c-S6-plan 3.3: consistente (8283/12301/14613 pendiente_decision -> COR si se acepta la decision).
