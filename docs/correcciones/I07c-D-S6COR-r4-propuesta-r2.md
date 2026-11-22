# Informe de D, paso 4.2 ronda 2: validacion de docs/correcciones/I07c-S6COR-r2.md (codigo en 9e91715)

## VEREDICTO: FAIL (propuesta casi correcta: una afirmacion de familia falsa y una decision sin declarar)

## Motivos del FAIL (exactos)

1. La afirmacion de la familia es FALSA: "solo el nombre y el literal de closure llegan a la etapa de efectos".
   Con binarios de base 1010938 y candidato 9e91715 (identicos), perfiles f0-0003-b-v1 y f0-0005-b-v1, check:
   - `fn Main() with no_io{let f=fn (){Print(1);};(if true then f else f)();}`: NX-E0415 closure.call,
     primario [45,45) VACIO (cadena [closure.call [45,45), intrinsic.call [33,38)]); sin grupo no parsea.
   - `fn Main() with no_io{let f=fn (){Print(1);};(move f)();}`: NX-E0415 closure.call, primario [45,45) VACIO.
   El barrido de C no incluyo destinos `if` ni con prefijo `move`. Ambos alcanzan efectos y reproducen el
   defecto que el addendum prohibe ("nunca un primario vacio por empezar la llamada en `(`").
   Correccion: reescribir el apartado 2 de la propuesta: alcanzan efectos nombre, literal de closure, `if`
   entre parentesis y destino con prefijo `move`; el resto SI se rechaza antes (comprobado: `(f())()`,
   `(arr[0])()`, `(())()`, `(1)()`, `(None)()` NX-E0111; `(a.b)()`, `(g.Next)()`, `(f.x)()` NX-E0006
   names.resolve; `(match..)` y `({f})` NX-E0002; `(!f)` NX-E0001; `(f+f)`, `(-f)` NX-E0102). Ampliar el
   barrido con esas formas.
2. Decision de convencion sin declarar para esos destinos. El mecanismo propuesto (anclaje = expresion interior
   desagrupada; head = call_head desde el anclaje), prototipado por mi tal cual, da para `(if true then f
   else f)()` el primario `if` [45,47) y para `(move f)()` el primario `move` [45,49). No hay control sin
   grupo ni referencia en la base (daba vacio): esos spans no se derivan de un control, son la extension de
   la convencion de cabeza `fn`. La propuesta solo justifica la cabeza para el literal de closure y envia a
   Flujo A "el literal completo".
   Correccion: (a) declarar en el apartado 3 que la regla es "cabeza (racha alfanumerica) desde el inicio de
   la expresion interior" tambien para `if` y para destinos con prefijo y pedir al coordinador que la registre
   (laguna de nucleo con mas de una respuesta razonable: expresion completa, cabeza, o el nombre `f` tras
   `move`) antes de fijarla en tests; o (b) fijar solo lo derivable (literal `fn`, control [21,23)) y declarar
   `if`/`move` como limite NO EVALUADO con su repro. En ambos casos anadir filas para las dos fuentes (check y
   run, 1 diagnostico, primario no vacio) al corpus de regresion.

## Comprobaciones con mi bateria (resto: PASA)

(1) Causa raiz: reproducida. `fn Main() with no_io{(fn (){Print(1);})();}`: base y candidato NX-E0415
    closure.call [21,21), cadena [closure.call [21,21), intrinsic.call [28,33)]; control sin grupo [21,23);
    E0412 `(fn (){let a=[1];})()` [24,24); E0413 con MockBlockingOpen [24,24); guarda [40,40). Causa: scope_sites
    solo indexa Expr::Name y el resto cae en call_head (texto desde `(`), segun el codigo.
(2) Familia: mi barrido (3.492 programas: 25 destinos x 9 contextos x 4 contratos x grupos 0..3, J y F5, SHA-256
    de la lista 01ce01079f91ae2ed97d2596290daedbe03543d700a43a83636c6ae90d60a7d6): vacios en 9e91715 en
    closure literal (E0415, E0413) y en las formas `if`/`move` del motivo 1. Metodos/campos, resultado de
    llamada, indice, unidad, entero: rechazados antes (ver arriba). Next y yield no regresan (Iterator.Next es
    nombre; yield no admite grupo).
(3) Mecanismo: PROTOTIPADO por mi en copia desechable (scope_sites indexa toda llamada con anclaje =
    expression_primary_span(target); head: nombre -> nombre, resto -> call_head(anclaje); starts por (archivo,
    inicio del anclaje) con minimo; sin condicionales por codigo ni patron). Resultado: 0 primarios o pasos
    vacios en los 3.492; las 6 fuentes de D prima dan [22,24), [23,25), [25,27), [25,27), [41,43) con primer paso
    igual y pasos siguientes intactos; iguales a la tabla del apartado 5; control [21,23); check y run
    coinciden. Diferencial base 1010938 vs prototipo (8.960 fuentes: mi corpus congelado 2.781+288+537, el de C
    1.500, 3.492 nuevas, fixtures JSON y .n; 107.520 comparaciones, 3 perfiles, check/run, debug): 0 cambios de
    testigo, operacion, codigo, data u orden y 0 diferencias en programas sin grupo; solo movimientos permitidos
    (mas `(())()` -> `()`, falso positivo de mi comparador: `()` es la unidad). Sin panicos ni internal_error.
    No se reabre el desempate de r1.
(4) Pruebas: filas del apartado 5 coherentes con el prototipo y con la aritmetica de bytes; positivos pareados
    validos. N1: `fn G(y:view Option<I64>){let z=move (y);}fn Main(){}` base [36,39), candidato NX-E1105
    permission_escalation [37,38) con paso borrow.read [5,6); control `move y` [36,37); `move ((y))` [38,39);
    positivo `y:move` aceptado; igual en J y F5. Mutaciones propuestas adecuadas; S4 y S5 pueden ser
    equivalentes (aceptable si se argumenta).
(5) Auditor: los 7 accesos estan bien clasificados: names.rs:191 cota de busqueda de token (boundary); 795
    clave de entrada/cuota (identity); 1085 relacion Context del destino completo (relation); 1186 primario de
    duplicate_argument sin etiqueta, inalcanzable (un duplicado posicional no ocupa slot; los nombrados usan
    label_span); 1193 span ocupado, solo relacion Context del siguiente duplicado (relation); 1516 test;
    effects.rs:1200 region de guarda (identity). Ampliar el auditor a names.rs y effects.rs con REVIEWED por
    contexto y `else: raise` es coherente; el --self-test debe incluir el caso rojo en cada fichero. No lo pude
    ejecutar (no esta implementado).
(6) Lectura del ADR: de acuerdo en que el primario no puede ser vacio y en que la cabeza `fn` del literal sin
    grupo ([21,23)) es la convencion vigente; el literal completo seria decision nueva (Flujo A). NO esta
    cubierta (motivo 2) la convencion para destinos `if` y con prefijo.

## Correcciones para pasar
a. Corregir el apartado 2 (motivo 1) y anadir `(if true then f else f)()` y `(move f)()` (y dobles grupos) al
   barrido y a las filas de regresion, con check y run.
b. Resolver el motivo 2 (declarar la regla de cabeza y registrarla, o limitar a lo derivable y declarar NO
   EVALUADO). El mecanismo del apartado 3 no necesita cambios.
Tras ello, mecanismo, N1, auditor y mutaciones se validan como estan.

Limpieza: copias y targets propios borrados; sin procesos propios vivos.
