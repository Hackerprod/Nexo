# Validador B, revalidacion: addendum «Sitio de la llamada con destino agrupado que no es un nombre» (I07c-S6COR-r2, commit ba4ff77)

Veredicto: PASS.

## Discrepancia de spans NX-E0414: tiene razon C, mi esperado estaba desplazado un byte
Por bytes: `fn Main() with no_suspend{` mide 9 (`fn Main()`) + 5 (` with`) + 11 (` no_suspend`) + 1 (`{`) = 26. El `(` esta en 26; el `fn` agrupado en [27,29); el control sin grupo `fn (){..}()` en [26,28); con doble grupo el `fn` en [28,30). Comprobado con indexacion de la cadena fuente (`s.index('(fn')=26`, `s.index('fn',26)=27`; doble: 28). Mis valores `[26,28)` y `[27,29)` del informe anterior eran erroneos (los calcule restando mal el prefijo); los de la propuesta (`[27,29)`, `[28,30)`) son los correctos. Retiro ese dato.

## Reverificacion de mis dos motivos
1. NX-E0414 en la regla: PASA. La regla cita los cuatro contratos de §8.2 (E0412 a E0415), hay filas E0414 en el addendum y en §6 (literal, doble, guarda, nombre, `if`, `move`, guarda con `if`), y el barrido es 2.784 = 4 x 696 (coherente con los 2.088 = 3 x 696 anteriores). NX-E0416 (`no_heap_alloc`) figura en el catalogo como propuesto y no es contrato de §8.2; el perfil lo rechaza antes con NX-E0006 `syntax.parse` (`fn Main() with no_heap_alloc{(fn (){let a=[1];})();}`), sin alcanzar efectos.
2. a) Todo paso de llamada lleva su cabeza: PASA. Texto, ejemplo anidado (`[22,24)`, `[29,31)`, `[35,40)`) y §5 coinciden con mi calculo por bytes; la base da `[21,21)`, `[28,28)`, `[35,40)`.
   b) NX-E1305 retirado de la regla y de la cita: PASA; la regla dice «incluidas las llamadas situadas en una guarda de match» y §2 explica que guard_effect no se produce con destino agrupado no nombre y que `(move f)()` en guarda se rechaza antes por guard_transfer.
   Ademas: trivia (frase y ejemplo `(  f)()` [47,48)), clausula de mecanismo retirada, alternativa literal de §8.2 para `(move f)()` declarada: presentes.

## Calculo por bytes de las filas nuevas (cadena fuente indexada, no el binario) contra la propuesta y los pasos siguientes del binario de 1010938
Todas coinciden: E0414 literal [27,29) con intrinsic.call [33,44); doble [28,30) con [34,45); control sin grupo [26,28) con [32,43); guarda E0414 [46,48) con [58,69); control de guarda [45,47) con [57,68); nombre E0414 [55,56) con [38,49); `if` [55,57) y doble [56,58); `move` [55,59) y doble [56,60); guarda con `if` bajo no_suspend [93,95) con [44,55); anidado [22,24), [29,31), [35,40); trivia [47,48) con [33,38). La base solo difiere en los pasos de llamada vacios, que es lo que corrige la regla. En guarda bajo no_alloc/no_block el defecto de la base es el mismo (primario vacio [43,43)).

## Contradicciones nuevas
Ninguna encontrada: §5 (cadenas), §2 (guardas) y el addendum son consistentes entre si; (4) salidas protegidas sin cambios respecto a mi barrido anterior (solo vacios con destino nombre).

## No bloqueante
- Una closure con MockUnknown() llamada en una guarda se acepta sin NX-E1305 (ya registrado en la propuesta para Pending.md).
- La fila de guarda con `if` bajo no_suspend aparece entre las filas de `move` en §6 (orden de tabla, sin efecto).
