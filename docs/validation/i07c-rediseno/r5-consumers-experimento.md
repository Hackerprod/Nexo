# R5, identidad de consumidor activo: experimento fallido

Binario release SHA-256 `893fee7577889ae12774fca32df00d40f3778c7d9b58d69f178bb50fa61fe8f7`, rustc 1.99.1, flags `pendfa,retfa,retract,nojoin,atomic0,e0103rule,retfail,rmm3b,rmm3e,rmm3f,nojoinr,skipv3,r5prefix,r1t5,r5t5`.

La proyección y la igualdad guardan la cadena de `FactId` de consumidores de expresión activos. La historia de una igualdad retirada solo participa en el plan si ambos comparten un consumidor. M1/M2 de B coinciden con `main` y sus esperados en los tres perfiles (6/6). La matriz T5/N da 45/45. Las sondas dirigidas dan **15/16**: `planned` pierde `NX-E1303` `[69,70)` y añade `NX-E0102` `[69,70)`, `[83,84)` y `[90,91)` respecto de la regla esperada. `planned_full` sí conserva su `NX-E1303` `[169,170)`.

La causa es representacional: el driver abre visitas separadas para expresiones hermanas dentro del consumidor, por lo que sus cadenas de `FactId` activos no comparten el nodo que impone la restricción de homogeneidad. Una identidad de sentencia por offset había conservado `planned`, pero era un proxy de forma y produjo 22 A de núcleo en el corpus T5. Este experimento no satisface la sonda de entrada al barrido completo y no constituye el prototipo de los cinco dominios del diseño. Su código se conserva solo para reproducir el fallo; ninguna cifra declara PASS.
