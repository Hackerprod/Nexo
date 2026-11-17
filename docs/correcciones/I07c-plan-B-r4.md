# I07c — validación B del plan, ronda r4 (retorno de C-S1)

Plan: docs/correcciones/I07c-plan.md (registro r4, sin commit). Ejecuciones con el binario release base de D:\nexo-work\target-i07c-c0 sobre fuentes escritas en D:\nexo-work\tmp.

## Veredicto global: FAIL (un bloqueante de mecanismo, uno documental menor)

## 1. Oráculo de b2d
- Offsets recalculados a mano (DV mide 39 bytes): `r` de `Len(r)` = [148,149), último `r` = [162,163). Correctos.
- Controles en la base: `var r=DV;let a0=Len(r);let b=[a0,r];` NX-E0102/not_indexable [67,68), solo relación found_type; `var r=DV;let a0=1+Len(r);let b=[a0,r];` mismo C [69,70); `let r=3;let a0=1+(1-Len(r));let b=[a0,r];` mismo C [34,35); `var r=DV;let b=[1,r];let a0=Len(r);` mismo C [79,80) (fijación independiente, análogo a RD84(a)/RD86(a)). La familia es coherente: igualdad compatible primero, C en el operando del dominio.
- Punto débil: en b2d la evidencia que fija r=I64 (el nominal de `Len(r)` vía `-`, `+`, a0) depende de la propia operación cuyo dominio refuta. §4.1.2.3/§4.1.2.5 dicen que un fallo de dominio «retira la evidencia dependiente»; aplicado a r=I64 retiraría la refutación y daría aceptación (que es justo el comportamiento actual de b2d). El plan retira el nominal de a0 para suprimir el C en el último `r`, pero conserva r=I64 sin decir por qué. La derivación «no hay nueva laguna» no argumenta este punto circular, y §4 («propaga invalidación solo a dependientes») no contiene la regla que lo resuelve. Es exactamente donde C se detuvo.
  Acción: añadir a §4 y al registro r4 la regla explícita (la igualdad fijada desde el nominal provisional se conserva frente al dominio de la misma operación porque «el cierre no se declara mientras una restricción compatible pueda decidir el predicado» y el orden de RD84(a)/RD86(a)/RD99: igualdad antes del dominio, C en el operando del dominio), citar la ancla independiente `[1,r]` + `Len(r)` y el gemelo concreto `let r=3`, y exigir esos tres como pares de regresión. Si A considera que hay dos respuestas razonables, Flujo A.

## 2. Resto de oráculos B2
Revisados sin copiar la salida del mecanismo: derivación propia y controles sin Never ejecutados en la base.
- b2a 1+Len(q)->Bool: C [67,75) I64/Bool; b2b 1<Len(q)->I64: C [66,74); b2c true==(q==q)->I64: C [66,78); b2e con 1-Len(q): C en `br` [101,103) Bool/Array<Bool>; b2g con [1]: C en `aar` [106,109); q4w con []: C en `c` [101,102) Bool/I64; Len(q)<1, Len(q)+DV y los controles de posición: mismos C.
- Ninguno de ellos tiene la circularidad de b2d: nada fija el argumento pendiente desde el propio resultado nominal de la operación cuyo dominio queda indeciso (en b2g `a==r` fija r desde `ar`, independiente; en q4w el dominio de `r==r` se satisface). Los seis restantes PASAN. Los offsets del plan coinciden con los de la expansión literal.

## 3. Incorporación de §8.3
Los oráculos de §2.1 (b1a/ir1, b1a-false, ir1c/ir1d, sin asignación posterior, ir1_t, recursivo `r=[r]`, negativo `r=[1]` [39,40)) coinciden con el ADR insertado en 59836f1; b1a-false es derivado de la regla (no fila literal del ADR) y debe etiquetarse así. No quedan restos de «solo N(f)» ni «excluidos del oráculo» fuera de las filas históricas del registro r2/r3.
Bloqueante menor (contradicción documental): el encabezado, §7 y §8.3 siguen diciendo que el coordinador «inserta» el texto en el ADR como precondición futura, y ya está insertado (59836f1). Reescribir como hecho cumplido.

## 4. Mecanismo y S1–S3
S2 y S3 no se ven afectados. Sobre S1: el plan afirma que b2d no altera §4; no es cierto mientras no exista la regla del punto 1 (ver acción).

## No bloqueante
- Las filas históricas del registro r2/r3 (que hablan de «laguna abierta») deberían marcarse como sustituidas por r4.
