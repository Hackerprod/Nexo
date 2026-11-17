# I07c — validación B del plan, ronda r5

Plan: docs/correcciones/I07c-plan.md (registro r5). Ejecuciones con el binario release base (D:\nexo-work\target-i07c-c0), fuentes en D:\nexo-work\tmp.

## Veredicto global: FAIL (dos bloqueantes, ambos corregibles en el .md)

## 1. Regla general: PASA
La regla (aplicar las igualdades compatibles capaces de decidir el predicado antes de cerrar el dominio; refutar en el operando original y retirar solo lo dependiente) es la del ADR: §4.1.2.3 «El cierre no se declara mientras una restricción compatible pueda decidir el predicado», «no retira restricciones independientes», RD84(a), RD86(a), RD99. Es un único mecanismo (obligaciones separadas: predicado, nominal, igualdad consumidora, diagnóstico), no un parche de b2d. Coherente con los controles ejecutados en la base.

## 2. b2d como laguna de núcleo: CORRECTO
El ADR dice a la vez que un fallo de dominio «retira esa evidencia dependiente» y que no retira las independientes. La igualdad r=I64 de b2d procede del nominal del propio Len(r): retirarla devuelve el dominio a indeciso (aceptar), conservarla lo refuta (C en Len(r)). Dos puntos fijos estables; ninguna fila RD lo decide. Calificación correcta; no encuentro una cláusula que decida.

## 3. Suspensión de b2d: bloqueantes
B1. Los pares dirigidos (1) `var r=DV;let a0=Len(r);let b=[a0,r];` y (3) `var r=DV;let a0=1+Len(r);let b=[a0,r];` tienen exactamente la misma arista circular que b2d (r se fija vía a0, nominal del mismo Len(r)); solo les falta el envoltorio Never. El plan los fija como negativos normativos (not_indexable [67,68) y [69,70)) y a la vez dice que «no resuelven la arista circular»: prejuzgan la laguna (bajo la opción A se aceptarían, como acepta la base b2d). Solo (2) `let r=3` (concreto, independiente) y (4) `[1,r]` (independiente) son ancla legítima. Acción: sacar (1) y (3) de los oráculos normativos y tratarlos como parte de §8.4 (o reformularlos con fijación independiente).
B2. Estado de S1 incoherente. La fila S1 dice «Detenido hasta Flujo A de §8.4» (todo S1), mientras §7 dice «S1 queda detenido en b2d» y el encargo pide que no se bloquee lo demás. Además el cierre del grafo no puede implementarse para b2a/b2b/b2c/b2e/b2g/q4w sin definir la arista circular. Acción: decidir y escribir una sola cosa: o S1 avanza con b2d y los pares (1)(3) explícitamente excluidos y la arista representada sin política elegida (sin tests que la fijen), o S1 queda detenido entero hasta el ADR. Que S2 «tras S1» refleje la elección.

## 4. Pares y offsets
Verificados en la base: (1) [67,68), (2) [34,35), (3) [69,70), (4) [79,80); los cuatro positivos aceptan sin diagnósticos. Variantes `r=[1]` posterior (C en Len + type_mismatch [85,88)) y `r=3` posterior (C en [67,68)) consistentes con RD86(a).

## 5. §8.3 y b1a-false: CORRECTO
Inserción en 59836f1 reflejada como hecho; b1a-false etiquetado como derivado de la regla. Sin restos de «solo N(f)». La parada incluye la falta de decisión de §8.4 solo para el cierre, correcto.

## 6. No bloqueante
El título de la primera línea del plan es un aviso de estado; conviene quitarlo al validar. 
