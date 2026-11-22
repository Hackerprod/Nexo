# Informe B - plan I07c-S5, ronda 3 dirigida
Veredicto: FAIL (un bloqueante de precision en el positivo del caso iv). Todo lo demas PASA.
Evidencia: fuentes en C:\nexo-work\tmp\validador-b-s5\r\ (i1-i4, p1-p5, t2), CLI de main 1bd010c, show.py.
(1) Casos extra: i1 47 B, i2 45 B, i3 50 B, i4 55 B; offsets por bytes: i1 [30,34) true, [39,43) true, rel [37,38) '1'; i2 [30,34), [38,42); i3 [30,34), [42,46), rel [40,41) '1'; i4 [30,34), [47,51), rel [45,46) '1'. Todos coinciden.
 main: i1-i3 emiten solo [30,34) (omite el segundo, confirmado); i4 emite ambos (control sin cambio, confirmado); run exit 1 en los cuatro.
 Positivos con 2 interior: p1 (1+true)[[1,2]], p2 ar[true][2+1], p3 ar[true][0][[1,2]] -> solo el error de base en main (y tras S5b, sin I64 al selector). 
 FALLA: p4 `let t=ar[true];let z=t[[1,2]];` (positivo de iv) emite DOS: [30,34) y E0102 [44,49) `[1,2]` (I64/Array<I64>); el plan dice que cada positivo "deja solo el error de la base". 
 Con base Tainted main impone I64 al selector; con base Err (S5b) el plan dice no imponerlo: la afirmacion es falsa para iv. Precisar (p.ej. positivo iv = `t[2]` -> solo [30,34), o declarar [44,49) como V no exigido).
(2) PASA: t2 (69 B) en main: E0102 [22,26) rel [18,21), E0102 [34,38) solo found_type, E1303 [63,65) (a desaparecer por Y'); bad_binary: E0102 [16,20) + E1303 [42,47) (a desaparecer). Spans por bytes correctos.
(3) PASA: repro 85 B: main da E1303 [60,66) `[true]` esperado Array<I64>; derivacion en orden fuente: r:Array<a>; [true] fija a=Bool; [1] incompatible -> E1303 [72,75) `[1]`; r=[1] -> E0102 [81,82) `1`. Spans correctos.
 index_selector: el plan exige informe validado por B o D antes de editar el test y cita l.361: PASA. +272 B: ayudante #[inline(never)] y condicion "si no vuelve a 6.784 B, S5a.3 no pasa de C": PASA. Operadores del barrido: 13 binarios (+ - * / % < <= > >= == != and or), 2 unarios, 5 compuestas, operandos y posiciones, SHA antes de ejecutar: PASA.
