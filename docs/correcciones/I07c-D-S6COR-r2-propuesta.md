# Informe de D, paso 4.2: validacion de docs/correcciones/I07c-S6COR-r1.md (codigo sigue en 1552e81)

## VEREDICTO: PASS

## Comprobaciones con mi bateria

(1) Causa raiz: CONFIRMADA. src/ir/effects.rs, find_witness: los testigos locales de Main (hojas sin arista) se
    comparan con compare_paths (src/ir/effects.rs ~4055-4099): (unknown, n causas), luego atajo de aristas, luego
    el vector de cause.span.start de toda la cadena, y solo despues la identidad. En `MockOpen(1);` W_i y W_d
    empatan en [21] y gana la identidad (resultado de la base: intrinsic.call [21,29)); con grupo el paso de
    llamada pasa a [22,30) y el value.drop sigue en [21,34): [21] < [22]. Reproducido con binarios de base y
    candidato: base intrinsic.call [21,21), candidato value.drop primario [22,30), paso [21,34).
(2) Familia: CONFIRMADA y sin formas nuevas fuera del mismo cambio. Reproduje los casos de C
    (`let f=fn (){(MockOpen)(1);};f();`: base [closure.call [49,50), intrinsic.call [33,33)], candidato
    [closure.call [49,50), value.drop [33,46)]). Mi barrido propio (plantillas distintas, 537 fuentes, 3
    perfiles; MockOpen, MockBlockingOpen, MockReadRecords/MockRequest/MockWriteI64/MockReply, `with no_io`,
    `no_block`, `no_suspend`, `no_alloc`, `with {call(no_io)}`, temporales en sentencia, argumento, `let`,
    `Some(..)`, array, `match`, `if`, `while`, bloque anidado, brazos de match, guardias, `for`/generadores,
    `Next`, funcion que devuelve MockResource, transitivo via K/L, closure, grupos 0..3, destino de llamada
    agrupado): 99 fuentes con cambio de testigo, TODAS del mismo tipo (NX-E0415; ultimo paso intrinsic.call ->
    value.drop; solo con constructor de recurso agrupado como temporal soltado, directo, transitivo
    (function.call) o via closure (closure.call)). Sin cambio de testigo en NX-E0413, NX-E1305 (guardias),
    `Mk()`/`(Mk)()` (value.drop ya en la base) ni `Next`.
(3) Mecanismo unico: CUBRE los sitios. Los unicos sitios que desempatan testigos por offset son los 3 que lista
    la propuesta (atajo, vectores a_offsets/b_offsets de compare_paths, oraculo find_witness_enumerated);
    las ordenaciones de guardia (effects.rs:1806) y de errores (1949) ordenan diagnosticos por primario y
    no cambian (guardias sin diferencias). PROTOTIPE el mecanismo exacto en una copia mia (CallSites con segundo
    mapa nombre -> inicio de la llamada, order_start, usado en esos 3 sitios; sin tocar Cause/Witness/Op/JSON,
    sin condicionales por codigo): 47.616 comparaciones base vs prototipo (3 perfiles, check/run, debug y
    release no; debug) sobre mis 2.781 + 288 + 537 dirigidas, 236 fixtures JSON y corpus/fixtures .n: 0
    diferencias de testigo, 0 diferencias en programas sin grupo, solo movimientos de primario/paso de
    llamada permitidos, 0 internal_error/panico. Las 5 fuentes del defecto y la fila de closure dan exactamente
    los esperados de la propuesta (intrinsic.call [22,30), [23,31), [22,38), [52,60); closure.call [49,50) +
    intrinsic.call [34,42), y con doble grupo [53,54) y [35,43)).
(4) Pruebas: PASA. Verifique por aritmetica de bytes y por CLI de la base que los esperados salen del control
    sin grupo mas el desplazamiento de parentesis, no del binario corregido: closure control [47,48) +
    [33,41), grupo [49,50) + [34,42); F(MockOpen(1)) [50,58) -> [51,59)/[52,60); N1: control `move a` [58,59),
    `move ((a))` [60,61), positivo `let b=move (a);` valido; N2: Push((a),2) [26,27) con control [25,26),
    Pop(((a))) [32,33) con control [30,31), Iterator.Next((g)) [60,61) con control [59,60). Positivo
    pareado de Push/Pop: confirmado que no existe en F5 (`Push(edit a,2)` da NX-E0006 sobre `Push` [20,24));
    `Iterator.Next(edit g)` es aceptado en J y F5 (checked). Positivos `fn Main(){(MockOpen)(1);}` y
    `F((MockOpen)(1))` sin `with no_io` verificados validos. Mutacion inversa (R1..R4): discrimina; R1 y R4
    se solapan (starts vacio equivale a identidad) y R2 (vectores sin order_start con el atajo) muere por las
    filas porque la comparacion de hojas locales no pasa por el atajo.
(5) Lectura del ADR: DE ACUERDO. El texto adoptado del addendum ("No cambia ... el orden causal", y el
    nombre del destino solo fija el span emitido, F0-0002 s8.2) fija la lectura A; con la B el mismo programa
    daria testigos distintos con y sin grupo y el addendum se contradiria. No hay laguna ni Flujo A.
(6) El resto del plan sigue vigente: primarios, pila, auditor y puerta no se tocan (types.rs/drive intactos).

## Notas (no bloquean)

- El atajo de aristas con order_start es equivalente al orden por inicio del nombre entre llamadas hermanas;
  unificarlo es correcto, pero ningun mutante lo discrimina (equivalente); no exigir prueba.
- Tras 4.3, repetir mi barrido y el de C debe dar 0 diferencias de testigo; el diferencial completo de D
  (2.781 + 288 + 537 + fixtures + 152 RD) se reejecutara en la validacion del codigo.
- Mantener en las filas de closure la comprobacion de `run` y de 1 diagnostico.
