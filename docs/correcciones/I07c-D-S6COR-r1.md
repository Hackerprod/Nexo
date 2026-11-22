# Informe de D: I07c-S6-COR, commit 1552e81806e1d32f7eecb15d1cca454f6eadd5b3 (base 1010938)

## VEREDICTO: FAIL (un defecto de nucleo reproducible; el resto PASA)

## Motivos del FAIL (accionables)

1. NX-E0415, operacion y testigo equivocados cuando la llamada agrupada produce un temporal soltado
   (nucleo: operation y cause_chain; el addendum dice "No cambia ... el orden causal").
   Perfil f0-0003-b-v1, check y run, debug y release. Fuente ASCII:
   fn Main() with no_io{(MockOpen)(1);}
   - Control sin grupo `fn Main() with no_io{MockOpen(1);}`: NX-E0415, operation=intrinsic.call,
     primario [21,29) MockOpen, cause_chain=[intrinsic.call [21,29)].
   - Esperado con grupo (addendum F0-0002-lagunas.md, "Primario de una expresion bajo agrupaciones":
     "No cambia el codigo, la subcausa, la barrera, la aceptacion, el orden causal ni el numero de
     diagnosticos"; plan S6COR s5.1 "No reordenar pasos ni testigos"; F0-0002 s8.2 nombre del destino):
     NX-E0415, operation=intrinsic.call, primario [22,30) MockOpen, cause_chain=[intrinsic.call [22,30)],
     contract [15,20). Es lo que daba la base salvo el primario vacio [21,21).
   - Obtenido (1552e81): operation=value.drop, primario [22,30), cause_chain=[value.drop [21,34)].
     Cambia de testigo (otra operacion, otro paso; primario y paso ademas incoherentes entre si).
   - Mismo defecto: `fn Main() with no_io{((MockOpen))(1);}` (value.drop, paso [21,36)),
     `fn Main() with no_io{(MockBlockingOpen)(1);}` (value.drop [22,38), paso [21,42)),
     `fn F(r:view MockResource){}fn Main() with no_io{F((MockOpen)(1));}` (value.drop [51,59), paso [50,63))
     y su doble grupo. Son las 5 fuentes de C; mi diferencial independiente no encuentra otra diferencia
     fuera del addendum (ver req. 5).
   - Causa: el desempate de testigos (F0-0002 s13: "los empates se resuelven por la secuencia de offsets de
     sitios fuente") pasa a usar el inicio del nombre (22) en lugar del inicio de la llamada (21) que usaba la
     base; el value.drop del temporal (paso [21,..)) gana. Adjudicacion: el addendum adoptado impone que
     agrupar no cambie el orden causal; la lectura literal del desempate por offsets daria value.drop, pero
     entonces el addendum seria contradictorio consigo mismo. Con el texto adoptado es defecto.
     Correccion esperada: el desempate no depende de los grupos (clave de orden igual a la de la base, por
     ejemplo el inicio de la llamada); el span emitido sigue siendo el nombre.
   - Los casos de tests/i07c_group_primary.rs de esta familia (lineas ~1019-1070 y su doble grupo) fijan el
     resultado erroneo (value.drop); el plan s5.7 exigia PARAR ante un cambio de salida protegida distinto del
     primario, y AGENTS prohibe tests que fijan comportamiento no decidido. Deben pasar a intrinsic.call
     [22,30) / [23,31) con paso igual.
   - Si el coordinador prefiere la lectura literal del desempate, es una laguna de nucleo: Flujo A (el
     addendum tendria que decir que el temporal soltado puede ganar). Mientras diga lo que dice, bloquea.

## Requisitos

(1) FIDELIDAD: PASA. Un solo bucle de primario `while let Expr::Group`: expression_primary y
    #[inline(never)] expression_primary_span en src/frontend/parse.rs:819-833; Checker::primary_span solo
    delega (types.rs:7078); names, types, parse, lower, lower_match y effects lo invocan. Sin `if code ==`,
    sin patrones de fuente. Los otros bucles de Group (names.rs:414,1012; types.rs:7041,7059,7157...) no
    eligen primario. Archivos dentro del plan (mas i07c_structural_invariants.rs, derivado del inventario del
    auditor, e i07c_s5b_selector.rs, regresion pedida). No se tocaron ownership_flow, interp, ir/model ni
    verify. Sin ramas nuevas en drive/resume_non_match.
(2) PRIMARIOS: PASA salvo (3). Mi bateria (base git archive 1010938 vs candidato): 27 filas de la tabla del
    plan con antes -> despues exactos (la fila NX-E0105 trae la fuente sin la llave final; con la del addendum
    `fn Main(){let a=Some(1);match edit (a){_=>{}}}` [35,38) -> [36,37)). Doble grupo: for ((...)) [21,24),
    ((x))(2) [20,21), ((F))() [30,31), Drop(((x[0]))) [27,31), F(1,((2))) [34,35), ((f))() F5 [36,37),
    move ((a[0])) [27,31), match move ((a[0])) [33,37), (move (a[0])) [27,31). Los dos defectos de D de S6:
    `fn Main(){for x in ([1]){}}` con f0-0003-b-v1 -> [20,23) y `fn Main(){let x=1;(x)(2);}` -> NX-E0111
    [19,20). Controles: `(a)=2` sobre let sigue [19,20); duplicate_argument `F(x:1,x:(2))` sigue en la
    etiqueta [45,46); R0102 intacto (`[((5))]` igual que la base); E0415 ((Print))(1) [23,28), ((F))() [40,41),
    ((f))() [58,59) con primer paso igual.
(3) HALLAZGO DE C: adjudicado, FALLA (motivo 1).
(4) Fila `move (a[0]);` de migrated_parser_contracts.rs (needle `(a[0])` -> `a[0]`): PASA. Deriva de la fila J
    del plan ([25,31) -> [26,30)) y del addendum; reproducido por CLI; las filas hermanas no cambian.
(5) DIFERENCIAL: PASA salvo (3). Generador propio gen.py (148 plantillas, huecos con 0..3 grupos, semilla
    90210417): 2.781 fuentes distintas, SHA-256 de la lista
    b4bb0ca0e71dffb4712784c5e276c659981469206a2d85226fb89eda830135d7 (congelada en FROZEN.txt antes de
    ejecutar); mas 288 dirigidas a lower_match, 217 .n de corpus/ y tests/fixtures/, 236 fuentes de fixtures
    JSON, y los 152 RD (i07c_rd_conformidad.py --verify en debug y release: 152/152, diferencias de snapshot
    0). 3 perfiles, check y run, debug y release, base vs candidato (unas 42.000 comparaciones). Todas las
    diferencias son de primario o primer paso de llamada de NX-E0006, E0105, E0111 (5 subcausas), E0202,
    E0412/E0413/E0415 (cabeza vacia -> nombre), E1104, E1105 (3 subcausas), E1107 y E1305, y cada una se
    comprobo mecanicamente (el texto anterior sin parentesis exteriores es el nuevo, o vacio -> nombre).
    Unicas diferencias en otros campos (operation, cause_chain): las fuentes del motivo 1. 0 divergencias
    debug/release, 0 panicos, 0 internal_error/InvalidIr, 0 cuelgues, 0 primarios candidatos que sean un
    grupo puro. Los fixtures JSON y el corpus quedan identicos salvo primarios.
(6) PILA: PASA. f5-frame-sizes.py (rustc 1.99.1, /MAP, dumpbin 14.44): base y candidato drive 27.536,
    resume_non_match 33.080, pareja 60.616, margen +5.944; expr_inner 12.752 -> 12.720; rutas 36.440 /
    31.968 / 50.224 -> 50.192 / 80.152 -> 80.120. f5-stack65.py propio: control 16 KiB desborda;
    option_match126 y result_match126 a 65 KiB 5/5 cada uno.
(7) MUTACION INVERSA PROPIA: PASA con huecos no bloqueantes. 10 mutantes distintos de los 36 de C (parche
    exacto, SHA-256 inicial/mutado/restaurado verificado, --profile default, parada en el primer fallo; los
    supervivientes con la suite completa debug, 2165 tests):
    M1 `while let`->`if let` en el ayudante: MUERTO (parse::tests::expression_primary_removes_every_outer_group_and_only_those)
    M2 sobre-desagrupar Unary (move/edit): MUERTO (expression_primary_keeps_the_prefix_of_move_and_edit)
    M3 indice de efectos con el span crudo del destino: MUERTO (written_calls_resolve_to_the_target_name_under_groups)
    M5 pierna conflict de Drop((a)) cruda: MUERTO (i07c_group_primary::ownership_diagnostics_report_the_inner_place_or_value)
    M7 pierna conflict de match move en lower_match cruda: MUERTO (misma prueba)
    M9 paso Next con sites.head en vez de next_head: MUERTO (generator_resume_anchors_ignore_the_group_around_the_target)
    M14 relacion de declaracion de non_callable sin desagrupar: MUERTO (call_binding_diagnostics_report_the_inner_target_and_argument)
    S1 taint heredado en index_failed_base: MUERTO en la suite completa 1680/2165
        (i07c_s5b_selector::nested_selector_of_failed_base_chain_keeps_its_own_error)
    M6 y M11: SUPERVIVIENTES, exhibidos por programa (ver N1, N2); el candidato es correcto en esos programas.
(8) AUDITOR: PASA. i07c_span_audit.py exit 0, `reviewed: 14/14`, sin pendiente_decision; --self-test PASS
    (6 reversiones); con `fn rogue(e:&Expr)->Span{ e.span().clone() }` insertada en una copia: exit 1
    ("unreviewed raw span").
(9) REGRESION DE TAINT (S5b.1): PASA. fn Main(){let ar=[1];let b=true;let x=1;var r=[3];var s=[];let t=ar[true];
    (ar[b][t][[1,2][true]]);} da NX-E0103 [56,58), NX-E0102 [68,72), [78,79) y [90,94) `true` (v2, J y F5); el
    mutante que hereda el taint en el selector de index_failed_base falla ese test y solo ese.
(10) PUERTA: PASA. `sh scripts/gate.sh 1552e81806e1d32f7eecb15d1cca454f6eadd5b3 --verify`: registro GREEN
    (arbol 9be0f316cdc96f8cfa0e041cd631c5a9c4be79bd). Log: 2165/2165 debug y 2167/2167 release (coinciden con
    el JSON), f5-stack65 5/5 + 5/5. SHA-256 de scripts/validate.sh del commit =
    origin/main:scripts/validate.sh = 7a1c7a199e4bcb09356bc6d56df8bb80556a5403f59a57cf502343deded130df.
(11) LIMITES DE C: PASA como NO EVALUADO honesto. NX-E1111 de prepare_closure_call: no hay forma de escribir
    un tipo closure fuera de su funcion (`fn G(f:view fn()...)` y retorno de closure dan NX-E0002, `[f]`
    NX-E0006, `Some(f)` conserva el borrador). `for` conflict del iterable de Array (lower.rs): inalcanzable
    (J rechaza en types con NX-E0006 sobre el iterable, F5 sobre `for`); generadores y rangos no pasan por
    esa ruta. Raices de check_place_access/asignacion: siempre un Name. Ramas edit/lectura de lower_match.rs
    (363-374): mis 288 programas dirigidos (edit, lectura, move, parametros view/edit/move, closures con
    captura, indexados, anidados) y el resto del corpus no producen ningun primario que sea un grupo puro; no
    pude alcanzarlas.

## No bloqueante

N1. Hueco de regresion con programa (M6, suite completa): la pierna conflict de `move (a)` (lower.rs:10531) no
    tiene prueba. El mutante `self.conflict(id, value.span().clone(), "consume")` cambia
    fn Main(){var a=Some(1);match edit a{Some(x)=>{let b=move (a);},None=>{}}} (f0-0003-b-v1, check) de
    primario `a` a `(a)`; el candidato esta bien. Anadir fila (y `move ((a))`).
N2. Hueco de regresion con programa (M11, suite completa): brazo `sig.intrinsic && formal == Inout` de
    names.rs:1208. El mutante `a.value.span().clone()` cambia fn Main(){var a=[1];Push((a),2);}
    (f0-0005-b-v1; candidato NX-E0111/argument_mode_mismatch [26,27), mutante [25,28)), `Pop(((a)))` e
    `Iterator.Next((g))`. Anadir fila.
N3. El plan s5.6 pedia ampliar i07c_mutaciones.py e i07c_mut_freeze.py a parse.rs y congelar la poblacion; C
    hizo 36 mutantes dirigidos propios y no entrego esa ampliacion (andamiaje).
N4. docs/validation/i07c-s6cor/__pycache__/*.pyc quedaron versionados; la fila NX-E0105 de la tabla del plan
    omite la llave final de la fuente (typo documental).
N5. tests/i07c_structural_invariants.rs fija numeros de linea de types.rs (7078, 8279, 12484, 14797, 14811):
    fragil como antes; el siguiente corte que edite types.rs debe reconciliarlos.

## Limpieza
Copias y targets propios borrados; arbol C:\projects\Nexo-i07c sin modificar; sin procesos propios vivos.
Scripts y salidas de D en C:\nexo-work\tmp\validador-d-s6cor (gen.py, templates.txt, FROZEN.txt, diff.py,
mut*.py, logs).
