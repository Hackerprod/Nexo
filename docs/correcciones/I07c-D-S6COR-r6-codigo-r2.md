# Informe de D: validacion del codigo r2 de S6-COR, commit f8c148faa95ef7f5b304dc8c65e3c7e49dc3e93d

## VEREDICTO: FAIL (un unico motivo, de andamiaje del corte: el auditor no detecta un .span() crudo nuevo en parte de effects.rs); el codigo de producto PASA

## Motivo del FAIL (reproducible)

1. scripts/i07c_span_audit.py (ampliado a effects.rs) NO falla ante un acceso crudo nuevo en la zona de
   produccion de src/ir/effects.rs posterior a la linea del primer `mod tests {` (linea 2490), que incluye
   `find_witness` (seleccion y desempate de testigos, el codigo de este corte). audit_plain toma
   tests_begin = primera linea `mod tests {` y trata TODO lo que sigue como linea de test (la cuenta, no falla).
   Repro en copia desechable de f8c148f (solo src/ y scripts/):
   - insertar `fn rogue2(e: &Expr) -> Span { e.span().clone() }` justo antes de `fn find_witness(`;
     `python -B scripts/i07c_span_audit.py` da exit 0 (esperado exit 1, "unreviewed raw span", como pide la
     propuesta r2 apartado 7: .span() crudo nuevo en effects.rs -> exit 1).
   - el mismo .span() antes de `fn call_head(` (antes del marcador) da exit 1; en names.rs (antes de
     `mod tests {`, ultimo bloque del fichero) da exit 1. El --self-test solo prueba la insercion previa al
     marcador, por eso pasa.
   Correccion: delimitar la zona de test por los limites reales de los modulos cfg(test) (emparejando llaves
   desde `mod tests {` y desde `mod witness_differential_tests {`) y tratar como produccion el resto; anadir al
   --self-test un caso rojo dentro de find_witness (region posterior a `mod tests {`), que sirve de regresion.

## Lo que PASA

(1) Defectos de D prima y B: PASA (check y run, J, debug y release; debug==release y check==run en todos):
    `(fn (){Print(1);})()` [22,24) primario y primer paso (control [21,23)); `((fn..))()` [23,25); E0412 [25,27);
    E0413 [25,27); guarda de match [41,43); `(if true then f else f)()` `if` [45,47); doble [46,48);
    `(move f)()` `move` [45,49); doble [46,50); anidado `(fn (){(fn (){Print(1);})();})();` [22,24), [29,31),
    [35,40); trivia `(  f)()` [47,48), `(  fn..)()` [47,49), `(  (move f))()` [48,52).
(2) Barridos: PASA. 0 primarios o pasos vacios en el candidato en mis 2.952 (55 formas) y 3.492 programas. Mi
    diferencial congelado contra la base 1010938 (6.920 fuentes: 2.781+288+537+2.952+236 fixtures JSON+217 .n y los
    152 RD; 83.040 ejecuciones, 3 perfiles, check/run, debug y release): 17.240 movimientos, todos de primario o
    paso de llamada permitidos por el addendum; 0 cambios de operacion, testigo, orden, data o codigo; 0 en
    programas sin grupo; 0 panicos, internal_error o InvalidIr; 0 divergencias debug/release; RD 152/152 en
    debug y release. Otra diferencia: la conocida O1 de mi informe r4 (`(Ok(f))();` 2 diagnosticos en la base,
    1 en el candidato, igual que la forma sin grupo `Ok(f)();`), presente desde 9e91715; coherente con el addendum.
(3) Fidelidad: PASA. git diff 9e91715 f8c148f -- src solo toca src/ir/effects.rs y es la propuesta: scope_sites
    indexa toda llamada (Site::Name o Site::Anchor con expression_primary_span del destino), written_head aplica
    call_head desde el anclaje, order_start por (archivo, inicio del sitio) con minimo; sin condicionales por
    codigo ni patron. 111 filas en i07c_group_primary.rs; verifique por CLI las de temporales (`F((G)())`
    primario [92,93) con control [91,92) y paso value.drop [91,96); `(G)();` [63,64) con control [62,63)) y las de
    closure, if y move: coinciden con la regla y el control, no con el binario. N1 `move (y)` [37,38), control
    [36,37), positivo `y:move` aceptado (sin cambios desde la ronda previa).
(4) Mutacion inversa propia (parche exacto, SHA-256 inicial, mutado y restaurado verificado, --profile default):
    Q1 anclaje con el span de la llamada: MUERTO (grouped_non_name_targets_report_their_head_and_order_by_the_call);
    Q2 destino no nombre indexado como Name: MUERTO (idem); Q3 order_start identidad: MUERTO
    (order_start_is_the_call_start_for_every_target_shape); Q5 temporal soltado sin productor: MUERTO
    (i07c_group_primary::effect_diagnostics_report_the_target_name_of_a_grouped_call); Q6 anclaje con
    target.span(): MUERTO (grouped_resource_constructor_keeps_the_witness_of_the_bare_call).
    SUPERVIVIENTES en la suite completa debug (2.168 tests) y sin programa que los exhiba tras comparar mutante
    contra candidato en 6.622 programas (3 perfiles, check/run, 64 dirigidos): Q4 (fallback de head a
    span.clone() en vez de call_head) y Q7 (se conserva el mayor inicio de llamada cuando dos llamadas comparten
    sitio, el S2e de C). No bloquean.
    Sobre S2e: NO estoy de acuerdo con el argumento de C (dos llamadas con el mismo sitio exigen un destino que es
    una llamada, rechazado antes con NX-E0111): es falso cuando el destino devuelve un closure. Ejemplo que llega
    a efectos: `fn Main() with no_io{let f=fn (){return fn (){Print(1);};};(f())();}` da NX-E0415 closure.call
    [60,61) en base y candidato (y `f()()` sin grupo [59,60)); ahi la llamada exterior y la interior comparten
    sitio. No pude construir un programa que distinga min de max, asi que queda como superviviente sin programa
    (no bloquea), pero la justificacion de C debe decir no observable en los programas probados, no inalcanzable.
    Esa forma (destino = llamada que devuelve un closure) llega a efectos y no estaba en el barrido de C ni en el
    mio anterior; su resultado es correcto (cabeza f del destino).
(5) Auditor: exit 0, reviewed 20/20 (types 14, names 5, effects 1), sin pendiente_decision; --self-test PASS
    (10 rechazos). Ver motivo 1.
(6) Pila: PASA. f5-frame-sizes.py: base y candidato drive 27.536, resume_non_match 33.080, pareja 60.616, margen
    +5.944; f5-stack65.py propio: control desborda, option_match126 y result_match126 65 KiB 5/5. RD 152/152.
    gate.sh f8c148faa95ef7f5b304dc8c65e3c7e49dc3e93d --verify: GREEN (arbol 51a716a7f1c1ca47a674b3b44e86974e0eb4b4b8);
    log 2168/2168 debug y 2170/2170 release (coinciden con el JSON), f5-stack65 5/5 + 5/5; SHA-256 de
    scripts/validate.sh = origin/main:scripts/validate.sh =
    7a1c7a199e4bcb09356bc6d56df8bb80556a5403f59a57cf502343deded130df.
(7) Ajustes del comparador de C: no afectan a la validez de la evidencia que fundamenta este veredicto. Permiten
    trivia (espacio, tab, CR, LF) entre el parentesis y la cabeza en el caso vacio -> nombre y tratan () como
    unidad; siguen exigiendo que el texto nuevo sea un nombre o el antiguo sin grupos. Mi diferencial usa un
    comparador propio e independiente y da los mismos resultados. Las tandas t0..t6 con el comparador anterior
    solo podian marcar MAS diferencias, no menos.

## No bloqueante
N1. Q4 y Q7 sobreviven (ver 4). N2. Corregir la justificacion de S2e.
N3. La mezcla de main trae cambios a scripts/f5-*.py y heavy-lock.py ajenos al corte.

Limpieza: copias y targets propios borrados; sin procesos propios vivos.
