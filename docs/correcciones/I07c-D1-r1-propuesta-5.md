# D1 - motivo 2, cursores de visita (r11+r12, HEAD cbb4d64)

Veredicto: PASS.

- Verificable: lista cerrada de lecturas por funcion + rg de los 9 nombres + programas CLI. Reproduje en 9b8e03e los esperados de la seccion: TD `let x=true+1;let y=x;let z=false+1;` -> dos NX-E0102 [16,20) y [37,42), ninguno en y; `let a=true+1;let b:Bool=1;` [16,20)+[34,35); `let a=true+1;let b=[];` [16,20)+NX-E0103 [29,31). RD71 y el fixture ya coinciden con la base en mi diferencial.
- No es renombrar: sale de InferenceGraph/Checker, vida por drive, marcos del mismo padre, y la autoridad duradera queda en obligaciones/ocurrencias. Reclasificacion D->Q declarada en el plan (enmienda de tabla), no encubierta.
- `previous_taint || tainted`: es un agregado hacia el padre; sondas propias (bloque tainted como rama, closures, funciones con return) conservan los C independientes: no observable en ellas. Aceptable si se prohibe leerlo como hecho del hijo siguiente.
- Condicion: toda mutacion de fuga debe cambiar un CLI; si no, no cuenta (ya previsto).
- Sin contradiccion con solver unico: los hechos persistentes siguen en el grafo; solo cambian cursores locales.
