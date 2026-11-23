# F0-0002 I07c — Defectos abiertos de núcleo (registro del 2026-11-22)

Estado: **registro de trabajo, sin PASS ni aprobación de fase**. Autor: agente de documentación del carril I07c (no implementa ni valida). Base de la medición: `origin/main` `1e2e393`. Desde `2875943` (binario `base-release.exe`) `origin/main` no cambia nada de `src/` ni de `tests/` salvo ficheros del oráculo F4 (`git diff 2875943 origin/main --stat -- src` vacío), así que `base-release.exe` es el `main` actual para estos repros; S5b.1, S6 y S6-COR están dentro de ese binario. No hay otro binario de comparación: V6 (`cand-release.exe`) es la rama `i07c-estructural` (commit `2296dc8`, no integrado: `git merge-base --is-ancestor 2296dc8 origin/main` = NOT).

Método: cada fuente se ejecutó el 2026-11-22 con `base-release.exe check --format json --profile f0-0002-b-v2 <fuente>` (otro perfil donde se indica; `run` solo en S6-COR-2). Primarios en bytes `[inicio,fin)`; tipos `A<..>` = `Array<..>`. El **esperado** de cada fila procede de una de tres fuentes, indicadas como (O) oráculo independiente `scripts/i07c_s5c_oraculo.py` (solo ADR, no lee el verificador; B lo validó en `docs/correcciones/I07c-S5c-oraculo-B-r1.md`), (B) derivación de un validador B/D con cláusula del ADR, (ADJ) adjudicación del coordinador del carril (`docs/validation/F0-0002-I07c-adjudicacion-twin.md`). Para las filas del oráculo, «actual = documentado» significa que la salida de hoy coincide con la que registran la matriz del diseño S5c y las adjudicaciones. Lo no comprobado se marca NO VERIFICADO.

## 1. Resumen

58 filas abiertas de núcleo, en 9 grupos. Ninguna está cerrada en `main`: los mecanismos que las cierran (S5a V6, S5a.2, S5a.3, S5a.5, S5c, S6-COR-2) no están integrados. Clasificación de gravedad por grupo (no por fila):

| Grupo | Filas | Gravedad dominante | Corte que lo cierra | Cuándo |
|---|---:|---|---|---|
| FAM-E (twin) | 17 | `NX-E0103` añadido sobre un `[]` que una asignación compatible resuelve; diagnóstico `NX-E1303`/`NX-E0102` perdido o trasladado; salida dependiente del orden de asignaciones | S5a.1 (V6) | sin fecha; V6 en rama, sin PASS |
| FAM-O (twin) | 8 | `NX-E0103` añadido; `NX-E1303` desplazado de hoja; O04/O05/O06 además `+`/`-` (S5a.2) | S5a.1 y S5a.2 | ídem |
| FAM-2 (twin) | 1 | `NX-E1303` en hoja equivocada más `NX-E0103` añadido | S5a.1 | ídem |
| S5c-ancla | 12 | `NX-E0103` espurio y/o `NX-E1303` extra o perdido por descarte del ancla de unión (también sin siembra) | S5c bloque B1 (ALT-2) | rediseño tras 2026-11-24 04:43 |
| S5c-otros | 10 | E0103/E0102 espurios, primario de C movido, retipado por `read_plan`, relación | S5c B1/B2, S5a.5 | ídem |
| F5-operand-domain | 1 | `InvalidIr` (`internal_error`, exit 2) | S5a.3 | sin fecha |
| S6-COR-2 | 7 | primario/origen de tick con grupo exterior, conteo de cuota, aceptación errónea | S6-COR-2 (plan con PASS de B r5 en la rama; sin implementar) | sin fecha; C Sonnet, re-validable con Codex tras 2026-11-24 |
| Guarda de `match` agrupada | 1 | aceptación errónea (falta NX-E1305) | tras S6-COR-2, Flujo B | sin fecha |
| Asignación a destino agrupado | 1 | recuento de `NX-E0102` cambia | tras S6-COR-2, Flujo B | sin fecha |

Por clase: `InvalidIr` 1 (F5); aceptación errónea 2 (guarda de `match`, `(Drop)(a)` de S6-COR-2); diagnóstico añadido o perdido y código/primario erróneo: el resto, con la relación `expected_type` equivocada como único defecto en 1 fila (P3) y como defecto adicional en varias. Un defecto de núcleo bloquea siempre (AGENTS.md); este documento no decide qué hacer con I07c, solo lista lo que existe.

## 2. Tabla de defectos abiertos

Las filas 1 a 6 de §2.2 son los mínimos de las familias twin de §2.1 (A, B, F06, M2): ilustran, no suman al recuento. Fuentes exactas: las citadas por fichero son byte a byte las del repositorio (campo `reduced` de `docs/validation/i07c-twin/casos.json`, `docs/validation/i07c-s5c/fuentes-recuperadas.json`, `docs/validation/i07c-s5a/{limites,d-repros}/*.n`); las demás se transcriben aquí. Prefijos: `PF=fn Main(){let p=true;let ar=[1];let aar=[[1]];` y `PX=fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];`.

### 2.1 Familias de la campaña twin (26 defectos; F08 es legítima)

Fecha de registro de cada fila: 2026-11-22 (la adjudicación es de 2026-11-20/21). Esperado firme común (ADJ, ADR §4.1 fila E0103, §4.1.2.2 punto 2): **ningún `NX-E0103` sobre un `[]` que una asignación compatible posterior fija** y ningún diagnóstico real de causa independiente perdido o trasladado. Esperados exactos de O03/O07/F10 los derivó el agente de adjudicación por consistencia (`§5` de la adjudicación), no con un ejemplo literal; los de O01–O08 están además derivados en `docs/correcciones/I07c-laguna-esperado-diferido-r1.md` y `I07c-laguna-o04-o05-o06-r1.md` (B: PASS).

Salida actual de `main` (códigos con primario) por caso, sobre la fuente `reduced` (bytes):

| Id (bytes) | Fam. | Salida actual de `main` | Estado |
|---|---|---|---|
| O01 (214) | O | E0103[74,76) E0102[93,99) E0102[139,144) E1303[198,203) | coincide con lo documentado (E0103 espurio; E1303 en hoja distinta) |
| O02 (356) | O | E0103[74,76) E0102[83,100) E1303[196,207) E0102[268,269) | E0103 espurio persiste |
| O03 (367) | O | E0103[74,76) E1303[203,206) E0102[277,278) | persiste; O: E1303 `[212,215)` y E0102 `[358,359)` |
| O04 (394) | O | E0103[83,85) E0102[122,123) E0102[130,135) E1303[181,184) E0102[196,197) E0102[259,263) E1303[332,333) E1303[352,354) E1303[363,365) | persiste |
| O05 (279) | O | E0103[92,94) E0102[155,156) E0102[157,158) E0102[265,266) | E0103 espurio; esperado completo no derivado (solo la ausencia del E0103) |
| O06 (213) | O | E0103[74,76) E0102[105,108) E0102[133,135) E1303[141,142) E0102[159,160) E0102[166,167) E0102[173,174) E0102[182,184) E1303[198,202) | persiste; esperado de 8 C en la propuesta O04/O05/O06 §B.4 (E1303 `[148,149)`) |
| O07 (163) | O | E0103[74,76) E1303[140,143) | persiste; O: E1303 `[149,152)` + E0102 `[159,160)` |
| O08 (160) | O | E0103[74,76) E0102[93,94) E0102[100,101) E0102[107,109) E0102[130,131) E1303[144,147) | persiste |
| F01 (146) | E | E0103[74,76) E0103[83,85) E0102[97,98) | persiste (ADJ: sin E0103) |
| F02 (148) | E | E0103[74,76) E0103[83,85) E0102[100,101) | persiste |
| F03 (152) | E | E0102[96,97) E0102[114,117) E1303[130,131) E0102[145,149) | la fuente original ya no muestra E0103 (cambió con S5b.1); el mínimo F03-min sigue dependiente del orden (ver abajo). El original: NO VERIFICADO |
| F04 (156) | E | E0103[74,76) E0103[83,85) E0102[99,101) E1303[117,118) E0102[124,125) | persiste |
| F05 (157) | E | E0103[74,76) E0103[83,85) E0102[105,107) E0102[140,141) | persiste |
| F07 (175) | E | E0101[95,97) (la fuente `reduced` omite el preludio con `G4`) | usar F07-min (abajo): persiste |
| F09 (182) | E | E0103[74,76) E0103[92,94) E0102[105,109) E0102[116,117) E0102[127,128) | persiste |
| F10 (189) | E | E0103[83,85) E0102[93,97) E0103[151,154) E1303[170,171) | persiste (residuo F10/F11 de V6) |
| F11 (182) | E | E0102[104,105) E1303[136,137) E0102[166,167) E0102[178,179) | el original ya no muestra E0103 (S5b.1); el mínimo (`B_qr`/e1, §2.2 fila 3) persiste. Original: NO VERIFICADO |
| F12 (199) | E | E0103[74,76) E0103[83,85) E0102[102,103) E0102[176,177) | persiste |
| F13 (190) | E | E0103[74,76) E0103[83,85) E0102[157,158) E1303[171,174) | persiste |
| F14 (213) | E | E0103[83,85) E0103[92,94) E0102[102,106) E0102[116,118) E1303[133,134) E0102[156,157) | persiste |
| F15 (217) | E | E0101[179,181) (falta preludio `H2`) | usar F15-min (abajo): persiste |
| F16 (206) | E | E0103[83,85) E0103[111,178) | persiste (ADJ: esperado E0102 en `[q]` y en `ar`) |
| F17 (231) | E | E0103[83,85) E0103[92,94) E0102[106,111) E0102[171,172) E1303[187,190) | persiste |
| F18 (227) | E | E0103[83,85) E0103[92,94) E0102[108,109) E1303[143,144) E0102[169,211) | persiste; el primer C de la unión de F18 lo debe derivar C desde §4.1.2.3–5 (plan S5 §7) |
| F19 (240) | E | E0103[83,85) E0103[92,94) E0102[102,106) E0102[121,125) E1303[180,184) E0102[217,218) | persiste |
| F06 (168) | 2 | E0103[74,76) E1303[149,151) | persiste; esperado (B, D-REF): E1303 en `r` y E0102 del `3`, sin E0103 y sin E1303 en `ar` (ver F06 mínimo, §2.2) |

Repros mínimos de la familia (verificados hoy; esperado (O) salvo indicación):
- F07-min (97 B): `fn Main(){var r=[];var s=[];let a1=(if true then (if true then s else 1) else r[1]);r=[3];s=[3];}` actual: E0103[16,18) E0103[25,27) E1303[70,71) E1303[78,82). Esperado (O): E1303[70,71) y E0102[87,88) (el `3` de `r=[3]`), sin E0103; el gemelo concreto (`let r=[3];let s=[3];`) da solo los dos E1303 (control, no oráculo: §4.1.2.7).
- F15-min (161 B): `PX` + `var r=[];var s=[];let a=(if p then s else x);let b=(if p then s else r[x]);r=[true];s=[[3]];}` actual: E0103[74,76) E0103[83,85) E1303[110,111) E1303[137,141). Esperado (O): E1303[110,111) y E0102[146,150) (`true`, §4.1.2.3: la hoja posterior se denuncia en la hoja). Con `s=[[3]];r=[true];` `main` da lo esperado (dependencia del orden).
- F10-min (150 B): `PX` + `var r=[];var q=[];let t=(if p then q[x] else (if p then [r] else r));r=[3];q=[3];}` actual: E0103[83,85) E0103[124,127) E1303[133,134). Esperado (O): E1303[133,134) y E0102[146,147), sin E0103.
- F16-min (121 B): `PX` + `var q=[];let z:Array<Array<I64>>=[q[1],[q]][0];q=ar;}` actual: E0103[74,76) E0103[101,114); esperado (ADJ): E0102 en `[q]` y E0102 en `ar`, sin E0103 (el oráculo no modela esta sintaxis).
- F03-min (116 B): `fn Main(){let p=true;let ar=[1];var r=[];var q=[];let a0=ar[r][[q,r]];let u=(if true then q else p);r=[3];q=[true];}`: con `r=[3];q=[true];` da E0102[60,61) E1303[97,98) A<I64>/Bool E0102[109,113); con `q=[true];r=[3];` da E0102[60,61) E0102[63,68) E1303[97,98) A<Bool>/Bool E0102[112,113). Salida dependiente del orden, comprobada; esperado exacto: NO DERIVADO (el oráculo añade un `not_indexable` en `ar[r]` que no figura en la adjudicación).
- Observación de adjudicación (heterogeneidad en el selector de base contaminada, `ar[true][[1,true]]`): cerrada por S5b.1 (ver §4).
- F08 (legítima, no se cuenta): `main` sigue dando E0103[108,149) (constructor genuinamente irresuelto); control sin cambio.

### 2.2 Ancla de unión, retipado y relaciones (sin siembra: también en `main`)

| # | Id (bytes) y fuente exacta (sin `check` de otros perfiles) | Salida actual de `main` | Esperado (quién) | Estado |
|---|---|---|---|---|
| 1 | A (96): `fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));r=[3];}` | E0103[30,32) E1303[74,77) A<I64>/A<A<I64>> exp=[53,54) | E1303[83,86) A<A<I64>>/A<A<A<I64>>> exp=[53,54) y E0102[92,93) `3` A<I64>/I64 exp=[74,77), sin E0103 (O; plan S5 §7, D-REF) | coincide con lo documentado; V6 acierta; control `r=[[3]]` (A_ctrl) correcto |
| 2 | A_noasg (90): A sin `r=[3];` | E0103[30,32) | E1303[83,86) (O) | ídem (V6 acierta) |
| 3 | `B_qr`=e1 (94): `fn Main(){let p=true;var r=[];var q=[];let a=(if p then p else q);let c=[q,[r]];q=[3];r=[3];}` | E0103[27,29) E0103[36,38) E0102[76,77) | E1303[63,64) Bool/A<A<I64>> y E0102[83,84) (O; ADJ) | persiste; con `r=[3];q=[3];` (`B_rq`=e2) `main` da el esperado: dependencia del orden |
| 4 | A_reord (96): A con `[r]` antes que `aar` | E0103[30,32) E1303[83,86) | E1303[74,77) y E0102[92,93) exp=[83,86) (O) | persiste |
| 5 | F06 mínimo (95): `fn Main(){let ar=[1];var r=[];let a0=(if true then r[1] else (if false then r else ar));r=[3];}` | E0103[27,29) E1303[83,85) I64/A<I64> | E1303[76,77) `r` y E0102[91,92), sin E0103 (O; B) | persiste |
| 6 | M2 (97): `fn Main(){let aar=[[1]];var r=[];let a=(if true then r[0] else (if true then r else aar));r=aar;}` | E0103[30,32) E1303[84,87) | E1303[77,78) y E0102[92,95) (O) | persiste |
| 7 | R4 (115): `fn Main(){let x=1;let p=true;let ar=[1];var r=[];var q=[];let a0=(if p then q else (if p then 1 else r));q=[[ar]];}` | E0103[46,48) E1303[94,95) | solo E1303[94,95) A<A<A<I64>>>/I64 exp=[76,77) (O; B diseño §1) | persiste |
| 8 | X_R1na (103): `fn Main(){let p=true;let ar=[1];let r=[[[1]]];var q=[];let a0=(if p then q else (if p then 1 else r));}` | E0103[52,54) E1303[98,99) | solo E1303[91,92) exp=[73,74) (O) | persiste |
| 9 | X_R1ord (103): como 8 con `then r else 1` | E0103[52,54) E1303[98,99) exp=[91,92) | solo E1303[98,99) exp=[73,74) (O) | persiste (primario correcto, E0103 y relación erróneos) |
| 10 | X_flat3 (117): `fn Main(){let p=true;let ar=[1];var q=[];let a0=(if p then q else (if p then 1 else (if p then 2 else 3)));q=[[ar]];}` | E1303[77,78) y E1303[95,96) | una E1303[77,78) (O; ADR l.353) | persiste (2 en lugar de 1) |
| 11 | Y_pend_if (92): `fn Main(){let p=true;let ar=[1];var r=[];let a0=(if p then r else (if p then 1 else true));}` | E0103[38,40) E1303[84,88) | solo E0103[38,40) (O; ADR l.353 y l.367) | persiste |
| 12 | Y_pend_if2 (92): como 11 con `then true else 1` | E0103[38,40) E1303[87,88) | solo E0103[38,40) (O) | persiste |
| 13 | X_R1inc (109): X_R1na + `q=[3];` | E1303[91,92) A<I64>/I64 | E1303[91,92) A<A<A<I64>>>/I64 y E0102[105,106) `3` exp=[98,99) (O) | persiste (retipa `q` a A<I64>, sin E0102) |
| 14 | X_P03 (103) `PF`+`var r=[];let a=(if p then r else (if p then ar else 1));}` | E0103[52,54) E1303[98,99) | solo E1303[98,99) A<I64>/I64 (O; B oráculo) | persiste |
| 15 | X_P04 (103) `…then 1 else ar…` | E0103[52,54) E1303[97,99) | solo E1303[90,91) (O) | persiste |
| 16 | X_P05 (105) `…then 1 else true…` sin asignación | E0103[52,54) E1303[97,101) | solo E0103[52,54) (O) | persiste |
| 17 | X_P15 (105) `PF`+`var r=[];let a=(if p then ar else (if p then aar else r));}` | E0103[52,54) E1303[91,94) | solo E1303[91,94) (O) | persiste |
| 18 | X_P16 (114) como 17 + `r=[true];` | E1303[91,94) | E1303[91,94) y E0102[107,111) `true` I64/Bool (O) | persiste (pierde el E0102) |
| 19 | X_P17 (105) `PF`+`var r=[];let a=(if p then r else (if p then ar else aar));}` | E0103[52,54) E1303[98,101) | solo E1303[98,101) (O) | persiste |
| 20 | W0 (101): `fn Main(){let ar=[1];var r=[];var q=[];let a=(if true then true else q);Print([r,q]);q=[[1<2]];r=[];}` | E0103[27,29) E1303[69,70) exp=[59,63) | solo E1303[69,70) Bool/A<A<Bool>> exp=[59,63) (O; B adjudicó a favor del oráculo) | persiste; sin la unión (W0nounion) se acepta; causa NO MEDIDA |
| 21 | P1 (113), c3 (111), w1 (112): `docs/validation/i07c-s5a/limites/{P1,c3,w1}.n` | E0103[27,29) (los tres) y E1303[70,71); w1 además E0102[87,88) | sin E0103 (hereda W0); w1 sin E0102 `[87,88)` (una asignación rechazada no retipa lecturas anteriores, l.301) (O; B) | persiste; las relaciones de `main` en P1/c3 (`[93,101)`, `[93,99)`) son correctas |
| 22 | P2 (137), w3 (139): `limites/{P2,w3}.n` | P2: E1303[108,114) exp=[75,80), E0102[127,129), E0102[133,134) A<A<I64>>/A<A<A<I64>>>; w3: E0103[46,48) E0102[78,79) E0103[119,126) E0102[129,131) | E1303 sobre `r` `[101,102)` A<A<A<I64>>>/A<A<I64>>; E0102[133,134) A<I64>/A<A<I64>> exp=[119,124) (P2), E0102[135,136) (w3) (O; B `I07c-S5c-oraculo-B-r1.md` §2b) | persiste; defecto previo de núcleo (dos asignaciones rechazadas mueven el primario de una C) |
| 23 | P3 (104): `limites/P3.n` | E0102[81,82) y E0102[99,100) exp=[57,58) | igual salvo exp=[88,94) en el segundo (derivado del ADR l.301/l.309; B) | persiste; solo relación |
| 24 | P4pos (170): `limites/P4pos.n` | incluye E0102[166,167) A<A<A<A<I64>>>>/A<A<I64>> exp=[102,107) | no debe existir (`r=[q]` es compatible, l.301) (B de S5a.1) | persiste; defecto previo |
| 25 | readplan (85): `fn Main(){var r=[];let a=(if true then r else (if true then [true] else [1]));r=[1];}` | E1303[60,66) A<I64>/A<Bool> exp=[39,40) | E1303[72,75) `[1]` y E0102[81,82) (orden fuente, ADR l.373) (O; Pending `S5-LIMITE-READPLAN`) | persiste; V6 tampoco lo cambia |

Todos los de este bloque son C o U de núcleo (código, primario o cardinalidad); los de la relación `expected_type` (columna I, ADR I(b)) no bloquean por sí solas, pero acompañan a filas con defecto N. Sin derivación cerrada (no se cuentan): R5 (294 B), P4 (175 B), `generated:1411`, `generated:1681` (`limites/` y `d-repros/`); la salida de `main` en cada una se conserva en la matriz del diseño S5c.

### 2.3 Operandos de `+`/`-` (S5a.2) y dominio lógico (S5a.3)

Los casos O04, O05-M4 y O06 de la tabla 2.1 son sus manifestaciones; mínimos verificados:
- Y' (101 B): `fn Main(){let ar=[1];var q=[];let a=(ar + (if true then q[0] else (if true then [2] else q)));q=[3];}` actual: E0103[27,29) E0102[37,39) E1303[80,83). Esperado (B, `I07c-laguna-o04-o05-o06-r1.md` §B.4): **un solo** E0102/type_mismatch[37,39) (`ar`, I64/A<I64>), sin E0103 ni E1303. Con `1` en lugar de `ar` (100 B) `main` da dos E0102 `[79,82)` y `[88,89)`, que es lo esperado.
- M4 (112 B): `fn Main(){let ar=[1];let aar=[[1]];var r=[];var s=[];let a=[[r],s];let b=(if true then [s] else aar);s=ar;r=ar;}` actual: E0103[50,52) E0102[64,65) A<A<I64>>/A<I64>. Esperado (B, §A): E1303[96,99) y E0102[103,105) exp=[60,63), sin E0103.
- F5-operand-domain: `fn Main(){let w=1+(true or false);}` con `--profile f0-0005-b-v1`: `outcome=driver_error`, `reason=internal_error` (`InvalidIr`, exit 2). Esperado (analogía ADR F5 §§2.1, 8.1 y filas F5-O06/O08): `NX-E0102/type_mismatch` en el operando sin paréntesis, `{expected_type:I64, found_type:Bool}`. Reproduce lo documentado. Cierra S5a.3 (plan S5; intento `ac91e09` revertido por regresión en `index_selector`).

### 2.4 Primario y recuento bajo agrupaciones (S6-COR-2 y pendientes de S6-COR)

Perfil `f0-0003-b-v1` salvo la última fila; `run` donde se indica. Esperado (B/D del corte; addendum `F0-0002-lagunas.md` «Primario de una expresión bajo agrupaciones»): el primario es la expresión sin las agrupaciones exteriores.

| # | Fuente (control entre paréntesis) | Actual | Esperado | Estado |
|---|---|---|---|---|
| 26 | `fn Main(){{}{}{}var n=0;while (n<9999999){}}` (`run`) | R0001[30,41) (control sin grupo [30,39)) | `[30,39)` | reproduce; plan S6-COR-2 S2 |
| 27 | `fn Main(){{}{}var n=0;while true{(n+1);}}` (`run`) | R0001[33,38) (control [33,36)) | `[33,36)` | reproduce; S1 |
| 28 | `…while true{(n)=1;}` (`run`, mismo prefijo) | R0001[33,36) (control [33,34)) | `[33,34)` | reproduce; S3 |
| 29 | `fn Main(){{}{}{}{}{}{}{}{}var n=0;while true{let y=if true then (n) else 2;}}` (`run`) | R0001[64,67) (control [64,65)) | `[64,65)` | reproduce; S4 |
| 30 | bucle `n=n+1;(Id)(1);` frente a `Id(1);`: falta un tick por iteración con destino agrupado | NO VERIFICADO hoy (se mide con bisección de N; documentado: 43.477 frente a 45.453) | mismo N que el control | plan S6-COR-2 §2 punto 5; S5 |
| 31 | `fn Main(){var x=1;match true{true if (if true then {(x)=2;value:true;} else {value:false;}) =>{},_=>{}}}` | NX-E1305/guard_mutation[52,55) `(x)` (control `x` [52,53)) | `[52,53)` | reproduce; S6 |
| 32 | `fn Main(){let a=Some(1);match true{true if (if true then {(Drop)(a);value:true;} else {value:false;}) =>{},_=>{}}}` | **aceptado** (`checked`); `Drop(a)` da NX-E1305/guard_transfer[58,62) | NX-E1305/guard_transfer en `Drop` | reproduce; S7 |
| 33 | `fn Main(){match true{true if (fn ()->Bool{MockUnknown();return true;})()=>{},_=>{}}}` | aceptado (`checked`) | NX-E1305/guard_effect (Pending `S6COR-GUARDA-MATCH-AGRUPADA`) | la aceptación reproduce; el control con nombre (`let g=fn …;match true{true if g()=>…}`) **también** se acepta en este perfil: control y esperado NO VERIFICADOS |
| 34 | `fn Main(){let x=1;var q1=[];(q1)=[[1],[true]];q1[0][0]=[x];q1=[[1]];}` (`f0-0002-b-v2`) | 3 NX-E0102: [35,36) [39,43) [64,65) | 2 (la forma sin grupo da E0102[37,41) y [53,56)) | reproduce; `S6COR-ASIGNACION-DESTINO-AGRUPADO`; preexistente e idéntico a la base `1010938` |

Observaciones de S6-COR que no son defecto: `(Ok(f))()` pasa de 2 a 1 diagnóstico, igual que la forma sin grupo (hoy `f0-0003-b-v1`: solo NX-E0111[34,39)); la fila de NX-E0113 de `(Ok(1))()` sin test es hueco de cobertura (§5).

## 3. Diseño S5c

Estado documentado en la rama `i07c-estructural` (no en `main`; HEAD `fd78f3f`):
- El defecto de ancla de unión existe en `main` sin siembra (§2.2 filas 7 a 19). Diseño `docs/correcciones/I07c-S5c-diseno-ancla.md`: ALT-2 (registro de ancla por unión) en dos bloques, B1 sobre `main` sin V6 y B2 con la retracción de V6; B dio PASS al diseño (r2) y al plan de implementación (r6), con el oráculo `scripts/i07c_s5c_oraculo.py` adjudicado (42/45 filas de la matriz coinciden; W0, P2 y w3 se adjudicaron a favor del oráculo; matriz corregida: OK en `main` 21 de 45, 24 con defecto).
- La enmienda r1 del plan (contraste de la hoja `else` pendiente con unión registrada, para R4, P15 y P16) obtuvo **FAIL de B** (`I07c-S5c-plan-enmienda-B-r1.md`): ALT-2, tanto la letra del plan como la desviación, **regresiona 15 programas de la familia que `main` y el oráculo aciertan** (unión anidada en primera posición con ancla de testigo parcial; p. ej. con prefijo `fn Main(){let p=true;let ar=[1];var q=[];var r=[];` el cuerpo `let a0=(if p then (if p then r else 1) else q);q=[[ar]];}`: `main` da solo E1303 en `1`, ALT-2 da E0103 y E1303 en `q`), y la desviación añade 12 regresiones frente a la letra (N1 a N4). **Esto no es un defecto de `main`: es un límite del diseño**; activa la PARADA del plan §9 («diferencial deja un DEFECTO»).
- Estado de trabajo: exploración acotada en curso; **rediseño por Codex tras el 2026-11-24 04:43** (fin del límite de uso del carril). Hasta entonces no hay mecanismo propuesto que el diferencial acepte.
- **Criterio de éxito objetivo** (B, `I07c-S5c-diseno-B-r2.md` y enmienda B r1; plan S5c §8 y 9): (1) diferencial sin muestreo frente a `main` y V6, ambos órdenes de asignación, tres perfiles, debug/release, `check`/`run`, con cero diferencias sin clasificar; (2) **ninguna fila en la que `main` coincida con el oráculo y B1 no** (regresión por diagnóstico, no por fila); (3) R4, P15 y P16 verdes con sus positivos; los 6 de B1 (R4, X_R1na, X_R1ord, X_flat3, Y_pend_if, Y_pend_if2) correctos; (4) matriz de 45 filas corregida validada contra el oráculo; (5) pasadas de planning contadas (ninguna llega a 129) y permutaciones de 2 y 3 asignaciones idénticas; (6) pila: crecimiento de la pareja `drive`+`resume_non_match` ≤ 2.500 B, margen final ≥ +1.000 B, `scripts/f5-stack65.py` verde; (7) mutación inversa por regla (descarte del ancla, `denunciada` sin marcar, igualdad omitida tras la denuncia, referencia = última hoja); (8) orden estricto por §13.9: toda diferencia de orden es defecto.
- T5 (resultado nominal de una operación con operando fallido): resuelta, no es laguna; aclaración derivada adoptada en `main` (`1e2e393`).

## 4. Cerrado en este tramo (solo lo que consta en `origin/main`)

- **S4** (contexto decidido por nodo; hijo vacío único): integrado (cierre `1bd010c`); D y D' PASS sobre `6394b67`. Residuo: mutación con 47 supervivientes sin programa (Pending `I07C-S4-MUT-SIN-PROGRAMA`).
- **S5b.1** (FAM-3: el selector de un índice se evalúa aunque su base falle): integrado (`6e9623e`, `1010938`). Verificado hoy: `fn Main(){let ar=[1];let z=ar[true][[1,true]];}` da E0102[30,34) y E0102[39,43) exp=[37,38), como exige el plan S5 §5. Cierra la observación de la adjudicación sobre `ar[true][[1,true]]`.
- **S6** (pruebas que cierran los huecos sin cobertura de la matriz): integrado (`1ad7b30`); D y D' PASS sobre `4d75e2c` (el PASS vale para el corte, no para la fase).
- **S6-COR** (primario de una expresión bajo agrupaciones en tipos, nombres, propiedad, efectos): integrado (`2a7891d`); D' PASS sobre `aa4da68`; implementado por un agente Sonnet por desvío de presupuesto, re-validable con Codex tras 2026-11-24.
- **S6-COR-2**: **no está cerrado**. En `origin/main` solo constan el plan (`docs/correcciones/I07c-S6COR2-plan.md`; el informe de B r5 con PASS está en la rama `i07c-estructural`, no en `origin/main`) y la adopción del texto en `F0-0002-lagunas.md` (`a24a1a1`); el código no está (las 7 manifestaciones de §2.4 persisten).
- **T5**: resuelta por el ADR; adoptada la aclaración (`1e2e393`).
- **S5a.1 (V6)**: **no integrado** (rama `i07c-estructural`; `2296dc8`). Límites documentados en `docs/validation/i07c-s5a/limites-conocidos.md`: V6 elimina el E0103 espurio en 21 de 23 casos de `casos.json` y deja F10/F11 con 2 E0103 cada uno, pero cambia la relación `expected_type` en P1, P2, w1, w3, c3 (procedencia), añade E0103 espurio en P12 y no corrige P2/w3, W0, R4, etc. S5a.2, S5a.3 y S5a.5 tampoco están implementados.

## 5. Límites declarados

- **Procedencia (relación `expected_type`)**: V6 (no `main`) devuelve la hoja de la unión en lugar de la asignación que fijó el componente en P1, P2, w1, w3 y c3 (`limites-conocidos.md`); la lectura de «qué asignación fija» está fijada por la regla de relación (ADR l.371), pero ALT-4 (registro append-only) y el plan `I07c-S5a5-procedencia-requisitos.md` quedan sin hacer. En `main` solo P3 tiene la relación equivocada (§2.2 fila 23).
- **Residuos F10/F11 de V6** (2 E0103 espurios cada uno, idénticos a `main`): plan propio S5a.5 «testigo causal», sin escribir.
- **NO DERIVADO** (sin esperado cerrado; no se cuentan): R5, P4, `generated:1411`, `generated:1681`, el O05 original de 279 B (solo se sabe que su E0103[92,94) es espurio), P5 y cuatro aleatorias del plan S5a.1 (fuentes no conservadas); F18 y el esperado exacto de O03/O07/F10 por consistencia, no por ejemplo literal.
- **Auditor de spans `scripts/i07c_span_audit.py` (NO EVALUADO por este registro)**: H1 (`#[cfg(test)]` sobre variante de enum o campo no se delimita), H2 (lógica antigua tras `mod quota_tests` en `types.rs`) y `Expr::span(e)` no se detecta (Pending `S6COR-PENDIENTES`). El auditor es andamiaje: no fundamenta PASS.
- **Deuda con fecha de `for_iteration`** (Pending `S6COR2-FOR-ITERATION`, 2026-11-22): aplicar `parse::expression_primary_span` en el origen del cargo `ArrayIterationLength` (`lower.rs` ~3160) y en el primario de la barrera de la iteración al activar `for_iteration`; hoy inalcanzable (`for` es NX-E0006 o NX-E0002 antes del lowering). Se revisa en el corte que active `for`, o antes en la siguiente campaña de mutación.
- **Supervivientes de mutación sin programa** (no bloquean sin programa; deuda con fecha: revisión en la campaña extendida nueva sobre el código vigente): M06b, M09, M09b (S6), Q4 y Q7 con equivalentes S2e y M8 (S6-COR), 47 de S4, 95 de ccf (`I07C-MUT-SIN-PROGRAMA`). Los identificadores M06b, M09 y M09b proceden de la lista del encargo y **no se localizaron en `Pending.md` de `origin/main`**: NO VERIFICADO su alcance; el informe de mutaciones inversas es `docs/correcciones/I07c-S6-mutaciones-inversas.md`.
- **Otros** (Pending): `I07C-S4-VS-BASE` (`let z=(true+([r] or r))` con `r` irresuelto: 3 diagnósticos frente a un posible cuarto según N(b)/N(d); prioridad baja, Flujo A tras F5; NO VERIFICADO hoy); campaña `I07C-MUT-EXT` histórica pausada; 41 tests `adr_gap_*` que no son oráculo; filas «sin cobertura» de la matriz de cierre (19 en el borrador: p. ej. L3-15, L3-19, T-F1p, T-F2p, Q-3) sin tratar.
- **Esta lista no sustituye** el contrato de cierre ni el informe de D' de I07c: solo registra defectos con programa. No se afirma PASS de I07c ni de ninguna fase.
