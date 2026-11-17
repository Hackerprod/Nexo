# Validación B-r2 de I07c-laguna-b2d

**Veredicto: PASS.** Ninguna salida fijada por el ADR cambia (RD84(a), RD86, RD87, RD99, RD116, RD125, TD04, TD05 sin cambio; la única novedad es la omisión del V del consumidor anterior, que el ADR deja como presencia I y la primera línea declara).

## Bloqueantes de B-r1
1. Argumento contra B: CORREGIDO. Ahora se apoya en acumulación de igualdades (cláusulas 1-2), estado de cierre no estable y N(a) frente al control directo; reconoce que retirar solo la igualdad no exigiría NX-E0103 (elemento `I64` por `a0`; `r` pendiente admitido en cláusula 3).
2. Pares (1) y (3) y sus positivos: CORREGIDO, en el bloque normativo. Offsets recalculados a mano: par (1) Len [63,66), r [67,68), segundo elemento [80,81); par (3) r [69,70), segundo elemento [82,83); `let c:Bool=a0;` [70,84), `a0` [81,83). Positivos (b2d, (1), (3)) cambian solo el segundo elemento `r` por `1`; en el binario base `check`=checked y `run`=unit.
3. Redacción y ubicación: CORREGIDO. «derivados, antes o después»; ubicación única tras la tabla «Variantes de la misma regla» y antes de la cláusula 4.

## Verificaciones
- Supresión del V coherente con N(d): el `c:Bool=a0` solo es V cuando `Len` falla (el control `let c:Bool=a0;let b=[a0,1];` sigue dando C en [81,83) en la base, correcto por §4.1.2.3). Precedente en RD86(a), donde la comparación anterior al fallo ya se omite. No suprime ningún C independiente.
- Sin material de proceso dentro del bloque a insertar («PROPUESTA/REVERTIBLE», B-r1 y rutas quedan fuera del bloque).
- Sin defectos nuevos bloqueantes.

## Registrable (no bloquea)
- El texto dice «antes o después» pero la tabla solo tiene el caso «antes»; añadir `...let b=[a0,r];let c:Bool=a0;` (base: type_mismatch extra en [94,96)) si se quiere oráculo explícito.
- Citar RD86(a) como precedente de la omisión.

**Inserción confirmada:** `docs/adr/F0-0002-second-slice.md` §4.1.2.3, justo después de la última fila («Negativo aislado») de la tabla «Variantes de la misma regla» (hoy línea 335) y antes de la cláusula 4 (línea 337).
