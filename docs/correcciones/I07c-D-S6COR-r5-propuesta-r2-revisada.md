# Informe de D, revalidacion de docs/correcciones/I07c-S6COR-r2.md revisada (commit 8b95fad; codigo en 9e91715)

## VEREDICTO: PASS

## Motivos anteriores

1. Familia incompleta: RESUELTO. La propuesta reescribe el apartado 2 con `if` agrupado y `move` agrupado
   como destinos que llegan a efectos, y la tabla de los que se rechazan antes coincide con mis pruebas
   (`(f())()`, `(Some(f))()`, `(None)()`, `(())()`, `(1)()` NX-E0111; `(f.x)()`, `(g.Next)()` NX-E0006
   names.resolve [46,47); `([f])()`, `([f][0])()`, `(arr[0])()` NX-E0006 types.check [45,48); `({f})()`
   NX-E0002 [45,46); `(f+f)()` NX-E0102; `(!f)()` NX-E0001; `move f` en guarda NX-E1305/guard_transfer
   [83,87), comprobado).
2. Convencion sin declarar: RESUELTO. La regla (cabeza = primer token de la expresion interior desagrupada) esta
   enunciada con texto exacto de addendum, ejemplo valido, invalido y diagnostico por forma, y las filas se
   separan entre las derivables del control sin grupo y las que dependen de la regla.

## Comprobaciones con mi bateria

(a) Barrido propio mas amplio (2.952 programas, lista congelada SHA-256 f17c1533c0b28455065ed60f5156a8192f5563246cd4c1009ebb93cb355a5a66;
    55 formas de destino x 6 contextos x contratos no_io/no_alloc/no_block x grupos 1..3). Incluye `edit`/`view`/
    `move` anidados (`move (move f)`, `move ((f))`, `(move f)` dentro de `if`), lambdas anidadas (`fn (){(fn (){..})();}`,
    `fn [view f](){f();}`, `fn [move v]`), operadores (`f+f`, `-f`, `!f`, `1+1`, `()+f`), `if` en `if` por ambos
    brazos, `if (true)`, brazo `{value:f;}`, bloque con valor, `match` como destino, tuplas, `Some/Ok/None`,
    destinos con nombres de funcion/intrinseco. En el candidato 9e91715 los unicos primarios/pasos vacios son
    literal de closure, `if` y `move` (las tres formas ya reconocidas); no hay ninguna forma nueva que llegue a
    efectos con destino no nombre y primario vacio. Las formas con prefijo `edit`, `view`, bloque, `match`, `-`,
    `!`, tuplas se rechazan en parse/lex/names (el primer token de un destino que llega a efectos es siempre
    de identificador: `fn`, `if`, `move`, nombre).
(b) Mecanismo prototipado por mi (scope_sites indexa toda llamada con anclaje; head: nombre -> nombre, resto ->
    call_head(anclaje); starts por (archivo, inicio del anclaje) con minimo). Con el: 0 primarios o pasos
    vacios en las 2.952 y en las 3.492 del barrido anterior; los spans declarados salen exactos: `fn` agrupado
    [22,24), doble [23,25), E0412 [25,27), E0413 [25,27), guarda [41,43), literal anidado [22,24) con
    closure.call [29,31) e intrinsic.call [35,40), literal que llama a un nombre [45,47) con [51,52) y [33,38),
    `if` [45,47) y doble [46,48), `move` [45,49) y doble [46,50), E0412 con `if` [49,51), E0413 con `move`
    [65,69) (cadena value.drop [40,41) + resource.drop sin span), guarda con `if` [83,85) con intrinsic.call
    [39,44), `if` en `if` y `move (f)`/`move ((f))`: cabeza `if`/`move`. Todos coinciden con las tablas de la
    propuesta. check y run coinciden.
(c) Ninguna forma sin grupo cambia: diferencial base 1010938 vs prototipo sobre 6.444 fuentes (mis 2.952 + 3.492;
    77.328 comparaciones, 3 perfiles, check/run): 0 cambios de operacion, testigo, orden o data en programas
    sin grupo; solo movimientos permitidos. Lo mismo ya medido en la ronda anterior sobre 8.960 fuentes con el
    mismo mecanismo.
(d) Resto del texto: auditor (7 accesos), N1 (`move (y)` [37,38), control [36,37), `move ((y))` [38,39),
    positivo `y:move` aceptado), mutaciones S1..S5, orden de aplicacion: sin cambios respecto de lo ya validado.

## Observacion no bloqueante (fuera de esta propuesta; existe ya en 9e91715)
O1. `fn Main(){let f=fn (){Print(1);};(Ok(f))();}`: la base daba 2 diagnosticos (NX-E0111 non_callable_target
    [33,40) y NX-E0113 types.infer [34,39)); 9e91715 da 1 (NX-E0111 [34,39)), igual que la forma sin grupo
    `Ok(f)();` en base y candidato (1 diagnostico, NX-E0111). El primario normalizado coincide con el de
    NX-E0113 y desaparece la duplicacion del sitio. Es coherente con el control sin grupo y con el addendum
    (agrupar no cambia el numero), pero es un cambio de cardinalidad frente a la base: el coordinador puede
    anotarlo en Pending (ni D ni D prima lo habian listado).

Limpieza: copias y targets propios borrados; sin procesos propios vivos.
