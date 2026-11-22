# Informe B - ronda 4 (confirmacion)
PASA. Linea 62 del plan: positivo de (iv) = `fn Main(){let ar=[1];let t=ar[true];let z=t[2];}`; con el CLI de main emite solo E0102 [30,34), rejected, run exit 1 (el rechazo es el del propio error de base).
Se explica y se declara no exigido el [44,49) de `t[[1,2]]`. (i)-(iii) conservan `2` por el `true` interior (p1-p3 de la ronda 3: solo error de base).
Unica referencia al positivo de (iv) en el plan: linea 62 (grep de `t[2]`, `t[[1,2]]`, `44,49`). Sin otros cambios de fondo comprobados en lo verificado en rondas 2-3.
Veredicto: PASS.
