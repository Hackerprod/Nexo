# I07c R6 v8: familias de diferencias frente a `main` (para B r13)

Base: `main` `31d91dbebd31c35b3cd09095c2d8e65b96c80548`; cada corpus usa el perfil congelado en `r7-frozen-batteries.json` (c1: `f0-0002-b-v2`). Los testigos c1 son fuente ASCII sin BOM ni LF final; intervalos `[inicio,fin)` de bytes UTF-8. Los 24.000 casos de `c1` dan 23.994 filas modelables, 20.649 iguales y **3.345 distintas**. La clasificación siguiente es una partición cerrada de las proyecciones ordenadas `(code, subcause, primary, expected_type, found_type, related.expected_type, related.found_type)`: se aplica primero cardinalidad, después igualdad de código y primario, después tipos y relaciones. Los IDs nominales de todas las filas y los recuentos de los otros 19 corpus/perfiles están en `r13-family-audit.json`; los programas fuente y las dos salidas observadas de los testigos están en `r12-c1-witnesses.json`. La salida observada del oráculo **no es la fuente del esperado ADR de los testigos**: la derivación por cláusula se transcribe abajo para que B la contraste independientemente.

| Familia cerrada (predicado de pertenencia) | c1 | Clase | Cláusula ADR que decide el esperado | ¿`main` = oráculo dentro de la familia? |
|---|---:|---|---|---|
| F1. Diagnóstico adicional de `main` (`len(main)>len(oráculo)`) | 2.277 | Arreglo núcleo | F0-0002 §§4.1.2.2, 4.1.2.4–5, 13.9; addendum «Continuacion posterior con constructor pendiente en una union» | No: todas son filas discrepantes; si el candidato difiere de `main` cuando estos coinciden, cuenta como A. |
| F2. Diagnóstico causal omitido por `main` (`len(main)<len(oráculo)`) | 247 | Arreglo núcleo | F0-0002 §§4.1.2.1–2, 4.1.2.5, 13.9 y 13.17 | No, por el mismo criterio. |
| F3. Igual cardinalidad, cambia código, subcausa o primario | 243 | Arreglo núcleo | F0-0002 §§4.1.2.1–4, 4.1.2.6, 13.9 y 13.17 | No, por el mismo criterio. |
| F4. Igual código y primario, cambia tipo o una relación distinta de solo `expected_type` | 54 | Arreglo núcleo | F0-0002 §§4.1.2.4–6 y 13.15–17: el testigo parcial no es el tipo nominal del resultado inválido | No, por el mismo criterio. |
| F5. Solo cambia la relación `expected_type`, incluida ausencia en `main` | 524 | Procedencia I | F0-0002 §4.1.2.6 y §13.15; addendum «Origen de la relación `expected_type` en la NX-E0102 de una asignación» cuando corresponda | No, por el mismo criterio. |
| **Subtotal núcleo F1–F4** | **2.821** | Arreglo núcleo | 2.277 + 247 + 243 + 54 | |
| **Total distinto** | **3.345** | | 2.821 + 524 | |

La partición por firma observable permite cotejar los 3.345 IDs sin una causa inferida del código. Para **autorizar** una fila, B deriva del ADR su diagnóstico y su clase C/D/V; una firma F1–F4 no convierte una presencia D/V opcional en arreglo de núcleo. Si una derivación no respalda la clase, se corrige la tabla y el oráculo antes de implementar esa fila. B muestrea al menos 20 diferencias por familia, con fuentes y esperados propios. Una fila de igualdad `main`–oráculo jamás puede entrar en F1–F5; una diferencia del candidato en ella es A y bloquea.

## Recuento por corpus/perfil

Los perfiles repetidos del mismo blob son entradas distintas de la puerta. `Sin oráculo` exige conservación exacta de `main`; `Igual` es la población de guarda A. Cada fila del corpus aparece en exactamente una columna de resultados.

| Corpus/perfil | Filas | Igual | Sin oráculo | F1 | F2 | F3 | F4 | F5 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| b-projections-2026112401 | 3000 | 1783 | 325 | 462 | 106 | 95 | 4 | 225 |
| b-projections-t5-2026112402 | 3000 | 1386 | 341 | 647 | 153 | 272 | 3 | 198 |
| b-projections-2026112403 | 3000 | 1822 | 326 | 447 | 101 | 100 | 5 | 199 |
| owner-projections-771412 | 60 | 43 | 0 | 1 | 16 | 0 | 0 | 0 |
| c1 | 24000 | 20649 | 6 | 2277 | 247 | 243 | 54 | 524 |
| c2 | 16000 | 1897 | 13869 | 165 | 16 | 14 | 2 | 37 |
| c2@f0-0003-b-v1 | 16000 | 1897 | 13869 | 165 | 16 | 14 | 2 | 37 |
| b | 4000 | 2980 | 347 | 472 | 45 | 39 | 8 | 109 |
| perm | 4044 | 3220 | 226 | 245 | 59 | 40 | 6 | 248 |
| t | 10584 | 5774 | 128 | 2930 | 1030 | 456 | 2 | 264 |
| s7021993 | 4500 | 852 | 3403 | 155 | 33 | 19 | 7 | 31 |
| s4417061 | 4500 | 880 | 3362 | 181 | 23 | 20 | 4 | 30 |
| s9908255 | 4500 | 892 | 3392 | 160 | 7 | 12 | 5 | 32 |
| dirigido5 | 6000 | 921 | 4456 | 379 | 35 | 27 | 0 | 182 |
| c1-nueva | 8000 | 6995 | 0 | 676 | 78 | 68 | 25 | 158 |
| c2-nueva | 8000 | 937 | 6943 | 80 | 10 | 5 | 1 | 24 |
| c2-nueva@f0-0003-b-v1 | 8000 | 937 | 6943 | 80 | 10 | 5 | 1 | 24 |
| c1-nueva2 | 8000 | 6966 | 1 | 693 | 84 | 75 | 15 | 166 |
| c2-nueva2 | 8000 | 962 | 6909 | 82 | 8 | 9 | 4 | 26 |
| c2-nueva2@f0-0003-b-v1 | 8000 | 962 | 6909 | 82 | 8 | 9 | 4 | 26 |
| **Total** | **151188** | **62755** | **71755** | **10379** | **2085** | **1522** | **152** | **2540** |

## Testigos de c1 derivados por cláusula

Los tres primeros IDs de cada familia son las fuentes más cortas en bytes dentro de c1 (desempate por ID). La columna «esperado ADR» fija los campos decisivos; los diagnósticos comunes no mencionados se conservan en el orden de `r12-c1-witnesses.json`. Esta transcripción es manual; el JSON separa las salidas **observadas** para detectar un desacuerdo con la derivación.

| Familia | Testigo y bytes | Esperado ADR y derivación independiente |
|---|---|---|
| F1 | `c1:44` (111 B) | Dos NX-E0103 sobre los constructores irresueltos `q=[]` `[67,69)` y `r=[]` `[76,78)`. La lectura `r` en `r[0]` `[107,108)` no es un tercer constructor con obligación propia (§4.1.2.2); su consecuencia queda contaminada (§4.1.2.5). |
| F1 | `c1:10707` (111 B) | Los mismos dos NX-E0103 `[67,69)` y `[76,78)`; `Print` no convierte la lectura `r` `[106,107)` en constructor independiente (§§4.1.2.2–3). |
| F1 | `c1:5703` (113 B) | Los mismos dos NX-E0103 `[67,69)` y `[76,78)`; el literal exterior no añade NX-E0103 sobre la lectura `r` `[108,109)` (§§4.1.2.2, 4.1.2.5). |
| F1 | **`c1:12348` (132 B)** | Dos NX-E0103 `[67,69)` y `[76,78)`, sin E1303 en `1` `[127,128)`. `[q]` es la primera incompatibilidad exterior con raíz pendiente: consume la única E1303 de esa unión aunque no pueda serializarse. Lo fija el addendum «Continuacion posterior con constructor pendiente en una union», con ejemplo inválido propio de 103 B. `main` añade aquí la E1303 de más. |
| F2 | `c1:13202` (135 B) | NX-E0103 `[67,69)`, E1303 `[117,118)` al comparar la continuación `r` con `aar`, y NX-E0102/type_mismatch `[131,132)` sobre `3` (`Array<I64>`/`I64`): la igualdad anterior que fijó el elemento de `r` persiste; la asignación posterior incompatible se denuncia en su RHS (§4.1.2.2). |
| F2 | `c1:3434` (137 B) | NX-E0103 `[76,78)`, E1303 `[125,127)` (`Array<Array<I64>>`/`Array<I64>`), y NX-E0102 `[133,135)` (`Array<Array<I64>>`/`Array<I64>`). La posterior `q=ar` no sustituye la igualdad compatible anterior de `q` (§§4.1.2.2, 4.1.2.5). |
| F2 | `c1:4081` (137 B) | NX-E0103 `[67,69)`, E1303 `[114,115)` (`Array<I64>`/`I64`), y NX-E0102 `[130,134)` (`I64`/`Bool`) en el elemento de la asignación posterior: la restricción rechazada no cambia el tipo establecido (§4.1.2.2). |
| F3 | `c1:6193` (130 B) | NX-E0103 `[76,78)` para `r=[]` y E1303 `[123,126)` (`Array<I64>`/`Array<Array<I64>>`) para `[q]` frente a la referencia `ar`: la comparación de continuaciones se hace en la hoja y `q=[]` adquiere contexto compatible; no se desplaza el conflicto al constructor inicial (§§4.1.2.1, 4.1.2.4). |
| F3 | `c1:12890` (130 B) | NX-E0103 `[67,69)` para `q=[]`; E1303 `[118,119)` (`I64`/`Array<I64>`) en la continuación `r`, cuyo elemento queda restringido por `r[0]` y `2` (§§4.1.2.2–4). |
| F3 | `c1:1425` (131 B) | NX-E0103 `[67,69)` para `q=[]`; E1303 `[126,127)` (`Array<I64>`/`Array<Array<I64>>`) en la continuación `r`: la lectura indexada conserva su contexto y no crea NX-E0103 en el constructor ya restringido (§§4.1.2.2–4). |
| F4 | `c1:8476` (137 B) | Dos E1303, `[119,120)` `Array<Array<I64>>`/`I64` y `[127,128)` `Array<Array<I64>>`/`Array<I64>`. El segundo esperado sigue siendo la primera continuación válida `[q,r]` como testigo parcial, no el `I64` de la continuación rechazada (§§4.1.2.4–5). |
| F4 | `c1:6934` (139 B) | Dos E1303, `[123,124)` `Array<Array<I64>>`/`Array<I64>` y `[131,135)` `Array<Array<I64>>`/`Bool`. El primer conflicto no reemplaza la referencia de la unión exterior con el tipo de `r` (§§4.1.2.4–5). |
| F4 | `c1:22309` (139 B) | Dos E1303, `[124,125)` `Array<Array<I64>>`/`Array<I64>` y `[132,136)` `Array<Array<I64>>`/`I64`. El `r[0]` exterior se compara con el testigo parcial de `[q,r]`, no con la hoja incompatible interior (§§4.1.2.4–5). |
| F5 | `c1:8381` (118 B) | NX-E0102 `[114,116)` `Array<Array<I64>>`/`Array<I64>`, relación `expected_type=[40,45)` del inicializador `aar=[[1]]`; la expresión `if` `[108,111)` no es el origen concreto del esperado (§4.1.2.6, §13.15). |
| F5 | `c1:10345` (118 B) | El mismo NX-E0102 `[114,116)` y relación `expected_type=[40,45)`; la segunda continuación `q` no sustituye el origen de `aar` (§4.1.2.6, §13.15). |
| F5 | `c1:16811` (118 B) | El mismo NX-E0102 `[114,116)` y relación `expected_type=[40,45)`; la primera continuación `q` no sustituye el origen de `aar` (§4.1.2.6, §13.15). |

El testigo solicitado `c1:12348` es exactamente `fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];let a0=(if p then true else (if p then [q] else 1));}` (132 bytes). La fuente completa de los otros quince testigos está en `r12-c1-witnesses.json`; cada entrada guarda su longitud en bytes, la salida observada del oráculo y la de `main` para contrastarlas con el esperado ADR de esta tabla.

## Reglas de aceptación de la tabla

Por cada corpus/perfil, `r13-family-audit.json` fija denominador, SHA, recuentos F1–F5 y los IDs discrepantes; la suma de las cinco familias más las filas iguales y sin oráculo debe ser el total. En C1–C5, una fila no propia puede seguir en `main` o alcanzar el oráculo, pero no volver atrás; toda fila donde `main` ya coincide con el oráculo exige A=0. En C6, **toda** fila modelable coincide con el oráculo, y cada diferencia frente a `main` pertenece a exactamente una familia; sin oráculo coincide con `main`. Las 103 filas de `match` coinciden con `main` en C0–C2; desde C3, 88 siguen exactamente a `main` y las 15 D6 siguen el esperado ADR por fila de `r13-match-d6.json`. Las demás filas de conservación D5 coinciden exactamente con `main` en todos los commits. Ninguna familia concede una dispensa contra estas igualdades.
