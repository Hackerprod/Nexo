# F0-0002 I07c — Defectos abiertos de núcleo (registro del 2026-11-22)

Estado: **registro de trabajo, sin PASS ni aprobación de fase**. Autor: agente de documentación del carril I07c (no implementa ni valida). Base de la medición: `origin/main` `1e2e393`. Desde `2875943` (binario `base-release.exe`) `origin/main` no cambia nada de `src/` ni de `tests/` salvo ficheros del oráculo F4 (`git diff 2875943 origin/main --stat -- src` vacío), así que `base-release.exe` es el `main` actual para estos repros; S5b.1, S6 y S6-COR están dentro de ese binario. No hay otro binario de comparación: V6 (`cand-release.exe`) es la rama `i07c-estructural` (commit `2296dc8`, no integrado: `git merge-base --is-ancestor 2296dc8 origin/main` = NOT).

Actualización 2026-11-23 (`origin/main` `e6c1291`): desde `1e2e393` solo cambian `src/ir/lower.rs` y `src/ir/lower_match.rs` (`git diff 1e2e393 origin/main --stat -- src`), por S6-COR-2 (integrado, `9dbcc99`) y F5-L04b; `src/check/` no cambia, de modo que las medidas de tipos de §2.1 a §2.3 siguen siendo las del 2026-11-22 con `base-release.exe`. **No se han re-ejecutado hoy** con un binario de `e6c1291`: su vigencia se infiere de que `src/check/` no cambia (NO VERIFICADO por ejecución). Las filas de S6-COR-2 de §2.4 pasan a «cerrado» según la evidencia citada en esa sección; la exploración del rediseño de S5c y su resultado están en §6.

Método: cada fuente se ejecutó el 2026-11-22 con `base-release.exe check --format json --profile f0-0002-b-v2 <fuente>` (otro perfil donde se indica; `run` solo en S6-COR-2). Primarios en bytes `[inicio,fin)`; tipos `A<..>` = `Array<..>`. El **esperado** de cada fila procede de una de tres fuentes, indicadas como (O) oráculo independiente `scripts/i07c_s5c_oraculo.py` (solo ADR, no lee el verificador; B lo validó en `docs/correcciones/I07c-S5c-oraculo-B-r1.md`), (B) derivación de un validador B/D con cláusula del ADR, (ADJ) adjudicación del coordinador del carril (`docs/validation/F0-0002-I07c-adjudicacion-twin.md`). Para las filas del oráculo, «actual = documentado» significa que la salida de hoy coincide con la que registran la matriz del diseño S5c y las adjudicaciones. Lo no comprobado se marca NO VERIFICADO.

## 1. Resumen

51 filas abiertas de núcleo, en 8 grupos (el registro del 2026-11-22 contaba 58 en 9; las 7 de S6-COR-2 están cerradas en `main` desde `9dbcc99`, ver §2.4 y §4). Las demás siguen presentes en `main`: los mecanismos que las cierran (S5a V6, S5a.2, S5a.3, S5a.5, S5c) no están integrados y, tras la exploración fallida de S5c (§6), quedan al rediseño por Flujo A. Clasificación de gravedad por grupo (no por fila):

| Grupo | Filas | Gravedad dominante | Corte que lo cierra | Cuándo |
|---|---:|---|---|---|
| FAM-E (twin) | 17 | `NX-E0103` añadido sobre un `[]` que una asignación compatible resuelve; diagnóstico `NX-E1303`/`NX-E0102` perdido o trasladado; salida dependiente del orden de asignaciones | S5a.1 (V6) | sin fecha propia; al rediseño de §6 (V6 en rama, sin PASS) |
| FAM-O (twin) | 8 | `NX-E0103` añadido; `NX-E1303` desplazado de hoja; O04/O05/O06 además `+`/`-` (S5a.2) | S5a.1 y S5a.2 | al rediseño de §6 |
| FAM-2 (twin) | 1 | `NX-E1303` en hoja equivocada más `NX-E0103` añadido | S5a.1 | al rediseño de §6 |
| S5c-ancla | 12 | `NX-E0103` espurio y/o `NX-E1303` extra o perdido por descarte del ancla de unión (también sin siembra) | S5c bloque B1 (ALT-2; la exploración de §6 no produjo mecanismo integrable) | rediseño por Flujo A (Codex, desde 2026-11-24 04:43; §6) |
| S5c-otros | 10 | E0103/E0102 espurios, primario de C movido, retipado por `read_plan`, relación | S5c B1/B2, S5a.5 | al rediseño de §6 |
| F5-operand-domain | 1 | `InvalidIr` (`internal_error`, exit 2) | S5a.3 | al rediseño de §6 |
| S6-COR-2 | 7 (cerradas; no cuentan entre las 51) | primario/origen de tick con grupo exterior, conteo de cuota, aceptación errónea | S6-COR-2 | **cerrado**: integrado en `main` (`9dbcc99`, 2026-11-23); implementado por un agente Sonnet por desvío de presupuesto, re-validable con Codex tras 2026-11-24 (deuda de mutación en Pending `I07C-S6COR2-DEUDA-MUTACION`) |
| Guarda de `match` agrupada | 1 | aceptación errónea (falta NX-E1305) | S6-COR-2 ya integrado; Flujo B con regresión | sin fecha |
| Asignación a destino agrupado | 1 | recuento de `NX-E0102` cambia | S6-COR-2 ya integrado; Flujo B con regresión | sin fecha |

Por clase: `InvalidIr` 1 (F5); aceptación errónea 1 (guarda de `match` agrupada; `(Drop)(a)` quedó cerrado por S6-COR-2); diagnóstico añadido o perdido y código/primario erróneo: el resto, con la relación `expected_type` equivocada como único defecto en 1 fila (P3) y como defecto adicional en varias. Un defecto de núcleo bloquea siempre (AGENTS.md); este documento no decide qué hacer con I07c, solo lista lo que existe.

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

Las filas 26 a 32 (S6-COR-2) están **cerradas** en `main`: la columna «Actual» conserva la medida del 2026-11-22 previa al corte. Evidencia de cierre: integración `9dbcc99` con D PASS dirigido y D' PASS de confirmación según su mensaje y la fila `I07C-S6COR2-DEUDA-MUTACION` de `Pending.md` (los informes de D y D' no están en `origin/main`: su contenido NO VERIFICADO aquí), informe de C `docs/correcciones/I07c-C-S6COR2-informe.md` (diferencial sobre 5.685 fuentes) y regresiones en `tests/i07c_group_primary.rs`; **no se han re-ejecutado hoy** con un binario de `e6c1291`. Las filas 33 y 34 **siguen abiertas**. Perfil `f0-0003-b-v1` salvo la última fila; `run` donde se indica. Esperado (B/D del corte; addendum `F0-0002-lagunas.md` «Primario de una expresión bajo agrupaciones»): el primario es la expresión sin las agrupaciones exteriores.

| # | Fuente (control entre paréntesis) | Actual | Esperado | Estado |
|---|---|---|---|---|
| 26 | `fn Main(){{}{}{}var n=0;while (n<9999999){}}` (`run`) | R0001[30,41) (control sin grupo [30,39)) | `[30,39)` | **cerrado por S6-COR-2 S2** (`c07b8f0`, integrado `9dbcc99`); test `while_condition_tick_reports_the_inner_comparison_of_a_large_bound` en `tests/i07c_group_primary.rs` |
| 27 | `fn Main(){{}{}var n=0;while true{(n+1);}}` (`run`) | R0001[33,38) (control [33,36)) | `[33,36)` | **cerrado por S6-COR-2 S1** (`049b48f`); tests `expression_statement_tick_*` |
| 28 | `…while true{(n)=1;}` (`run`, mismo prefijo) | R0001[33,36) (control [33,34)) | `[33,34)` | **cerrado por S6-COR-2 S3** (`2d47cc2`); test `assignment_tick_reports_the_inner_place` |
| 29 | `fn Main(){{}{}{}{}{}{}{}{}var n=0;while true{let y=if true then (n) else 2;}}` (`run`) | R0001[64,67) (control [64,65)) | `[64,65)` | **cerrado por S6-COR-2 S4** (`f7a5a9d`); tests `then_operand_tick_*`, `else_operand_tick_*`, `branch_operand_tick_*` |
| 30 | bucle `n=n+1;(Id)(1);` frente a `Id(1);`: falta un tick por iteración con destino agrupado | NO VERIFICADO hoy (se mide con bisección de N; documentado: 43.477 frente a 45.453) | mismo N que el control | **cerrado por S6-COR-2 S5** (`542d5b1`); tests `grouped_call_target_charges_the_ticks_of_the_bare_call` y `grouped_closure_call_charges_the_ticks_of_the_bare_call` (relacionales sobre la frontera de cuota; el informe de C da N=43.477 para `Id(1)` sin fijar la cifra) |
| 31 | `fn Main(){var x=1;match true{true if (if true then {(x)=2;value:true;} else {value:false;}) =>{},_=>{}}}` | NX-E1305/guard_mutation[52,55) `(x)` (control `x` [52,53)) | `[52,53)` | **cerrado por S6-COR-2 S6** (`8002952`); casos `guard_mutation_*` de `CASES` y test `guard_mutation_reports_the_inner_place` |
| 32 | `fn Main(){let a=Some(1);match true{true if (if true then {(Drop)(a);value:true;} else {value:false;}) =>{},_=>{}}}` | **aceptado** (`checked`); `Drop(a)` da NX-E1305/guard_transfer[58,62) | NX-E1305/guard_transfer en `Drop` | **cerrado por S6-COR-2 S7** (`33046c4`); tests `guard_transfer_reports_the_inner_callee` y `grouped_callee_other_than_drop_is_accepted_in_a_guard` |
| 33 | `fn Main(){match true{true if (fn ()->Bool{MockUnknown();return true;})()=>{},_=>{}}}` | aceptado (`checked`) | NX-E1305/guard_effect (Pending `S6COR-GUARDA-MATCH-AGRUPADA`) | la aceptación reproduce; el control con nombre (`let g=fn …;match true{true if g()=>…}`) **también** se acepta en este perfil: control y esperado NO VERIFICADOS |
| 34 | `fn Main(){let x=1;var q1=[];(q1)=[[1],[true]];q1[0][0]=[x];q1=[[1]];}` (`f0-0002-b-v2`) | 3 NX-E0102: [35,36) [39,43) [64,65) | 2 (la forma sin grupo da E0102[37,41) y [53,56)) | reproduce; `S6COR-ASIGNACION-DESTINO-AGRUPADO`; preexistente e idéntico a la base `1010938` |

Observaciones de S6-COR que no son defecto: `(Ok(f))()` pasa de 2 a 1 diagnóstico, igual que la forma sin grupo (hoy `f0-0003-b-v1`: solo NX-E0111[34,39)); la fila de NX-E0113 de `(Ok(1))()` sin test es hueco de cobertura (§5).

## 3. Diseño S5c

Estado documentado en la rama `i07c-estructural` (no en `main`; HEAD `fd78f3f`):
- El defecto de ancla de unión existe en `main` sin siembra (§2.2 filas 7 a 19). Diseño `docs/correcciones/I07c-S5c-diseno-ancla.md`: ALT-2 (registro de ancla por unión) en dos bloques, B1 sobre `main` sin V6 y B2 con la retracción de V6; B dio PASS al diseño (r2) y al plan de implementación (r6), con el oráculo `scripts/i07c_s5c_oraculo.py` adjudicado (42/45 filas de la matriz coinciden; W0, P2 y w3 se adjudicaron a favor del oráculo; matriz corregida: OK en `main` 21 de 45, 24 con defecto).
- La enmienda r1 del plan (contraste de la hoja `else` pendiente con unión registrada, para R4, P15 y P16) obtuvo **FAIL de B** (`I07c-S5c-plan-enmienda-B-r1.md`): ALT-2, tanto la letra del plan como la desviación, **regresiona 15 programas de la familia que `main` y el oráculo aciertan** (unión anidada en primera posición con ancla de testigo parcial; p. ej. con prefijo `fn Main(){let p=true;let ar=[1];var q=[];var r=[];` el cuerpo `let a0=(if p then (if p then r else 1) else q);q=[[ar]];}`: `main` da solo E1303 en `1`, ALT-2 da E0103 y E1303 en `q`), y la desviación añade 12 regresiones frente a la letra (N1 a N4). **Esto no es un defecto de `main`: es un límite del diseño**; activa la PARADA del plan §9 («diferencial deja un DEFECTO»).
- Estado de trabajo (actualizado 2026-11-23): la exploración acotada terminó sin mecanismo integrable (§6); **rediseño por Flujo A con Codex desde el 2026-11-24 04:43** (fin del límite de uso del carril), antes de F5 B2b+. Hasta entonces no hay mecanismo propuesto que el diferencial acepte.
- **Criterio de éxito objetivo** (B, `I07c-S5c-diseno-B-r2.md` y enmienda B r1; plan S5c §8 y 9): (1) diferencial sin muestreo frente a `main` y V6, ambos órdenes de asignación, tres perfiles, debug/release, `check`/`run`, con cero diferencias sin clasificar; (2) **ninguna fila en la que `main` coincida con el oráculo y B1 no** (regresión por diagnóstico, no por fila); (3) R4, P15 y P16 verdes con sus positivos; los 6 de B1 (R4, X_R1na, X_R1ord, X_flat3, Y_pend_if, Y_pend_if2) correctos; (4) matriz de 45 filas corregida validada contra el oráculo; (5) pasadas de planning contadas (ninguna llega a 129) y permutaciones de 2 y 3 asignaciones idénticas; (6) pila: crecimiento de la pareja `drive`+`resume_non_match` ≤ 2.500 B, margen final ≥ +1.000 B, `scripts/f5-stack65.py` verde; (7) mutación inversa por regla (descarte del ancla, `denunciada` sin marcar, igualdad omitida tras la denuncia, referencia = última hoja); (8) orden estricto por §13.9: toda diferencia de orden es defecto.
- T5 (resultado nominal de una operación con operando fallido): resuelta, no es laguna; aclaración derivada adoptada en `main` (`1e2e393`).

## 4. Cerrado en este tramo (solo lo que consta en `origin/main`)

- **S4** (contexto decidido por nodo; hijo vacío único): integrado (cierre `1bd010c`); D y D' PASS sobre `6394b67`. Residuo: mutación con 47 supervivientes sin programa (Pending `I07C-S4-MUT-SIN-PROGRAMA`).
- **S5b.1** (FAM-3: el selector de un índice se evalúa aunque su base falle): integrado (`6e9623e`, `1010938`). Verificado hoy: `fn Main(){let ar=[1];let z=ar[true][[1,true]];}` da E0102[30,34) y E0102[39,43) exp=[37,38), como exige el plan S5 §5. Cierra la observación de la adjudicación sobre `ar[true][[1,true]]`.
- **S6** (pruebas que cierran los huecos sin cobertura de la matriz): integrado (`1ad7b30`); D y D' PASS sobre `4d75e2c` (el PASS vale para el corte, no para la fase).
- **S6-COR** (primario de una expresión bajo agrupaciones en tipos, nombres, propiedad, efectos): integrado (`2a7891d`); D' PASS sobre `aa4da68`; implementado por un agente Sonnet por desvío de presupuesto, re-validable con Codex tras 2026-11-24.
- **S6-COR-2**: integrado en `origin/main` (`9dbcc99`, 2026-11-23; `Pending.md` lo registra en `I07C-S6COR2-DEUDA-MUTACION` y el commit `c3be7cc` añade esa fila). Según el mensaje de integración: D PASS dirigido (`I07c-D-S6COR2-r2`) y D' PASS de confirmación (`I07c-Dprima-S6COR2-r2`), puerta GREEN sobre `8e6f777`; esos dos informes no están en `origin/main` (NO VERIFICADO su contenido). En `origin/main` constan el plan `docs/correcciones/I07c-S6COR2-plan.md`, el informe de C `docs/correcciones/I07c-C-S6COR2-informe.md` y la evidencia `docs/validation/i07c-s6cor2/`. Cierra las 7 manifestaciones de §2.4 (filas 26 a 32); **no** cierra las filas 33 y 34 (guarda de `match` agrupada y asignación a destino agrupado). Desvío por presupuesto: implementador Sonnet, re-validable con Codex tras 2026-11-24; deuda de mutación declarada en Pending (7 supervivientes de región: M00013, M00021, M00023, M00051, M00054, M00047, M00048).
- **T5**: resuelta por el ADR; adoptada la aclaración (`1e2e393`).
- **Rellenos de laguna adoptados en `docs/adr/F0-0002-lagunas.md`** (decisión del coordinador, revertible): literal de array con elemento erróneo (N), estado de `match` con uniones (conservación) y procedencia de `expected_type` (A) en `eeb3c9c`; continuación posterior con constructor pendiente en una unión (`pending_inner`, lectura B) en `6b6f7c5`.
- **S5a.1 (V6)**: **no integrado** (rama `i07c-estructural`; `2296dc8`). Límites documentados en `docs/validation/i07c-s5a/limites-conocidos.md`: V6 elimina el E0103 espurio en 21 de 23 casos de `casos.json` y deja F10/F11 con 2 E0103 cada uno, pero cambia la relación `expected_type` en P1, P2, w1, w3, c3 (procedencia), añade E0103 espurio en P12 y no corrige P2/w3, W0, R4, etc. S5a.2, S5a.3 y S5a.5 tampoco están implementados y quedan al rediseño de §6.

## 5. Límites declarados

- **Procedencia (relación `expected_type`)**: V6 (no `main`) devuelve la hoja de la unión en lugar de la asignación que fijó el componente en P1, P2, w1, w3 y c3 (`limites-conocidos.md`); la lectura de «qué asignación fija» está fijada por la regla de relación (ADR l.371), pero ALT-4 (registro append-only) y el plan `I07c-S5a5-procedencia-requisitos.md` quedan sin hacer. En `main` solo P3 tiene la relación equivocada (§2.2 fila 23).
- **Residuos F10/F11 de V6** (2 E0103 espurios cada uno, idénticos a `main`): plan propio S5a.5 «testigo causal», sin escribir.
- **NO DERIVADO** (sin esperado cerrado; no se cuentan): R5, P4, `generated:1411`, `generated:1681`, el O05 original de 279 B (solo se sabe que su E0103[92,94) es espurio), P5 y cuatro aleatorias del plan S5a.1 (fuentes no conservadas); F18 y el esperado exacto de O03/O07/F10 por consistencia, no por ejemplo literal.
- **Auditor de spans `scripts/i07c_span_audit.py` (NO EVALUADO por este registro)**: H1 (`#[cfg(test)]` sobre variante de enum o campo no se delimita), H2 (lógica antigua tras `mod quota_tests` en `types.rs`) y `Expr::span(e)` no se detecta (Pending `S6COR-PENDIENTES`). El auditor es andamiaje: no fundamenta PASS.
- **Deuda con fecha de `for_iteration`** (Pending `S6COR2-FOR-ITERATION`, 2026-11-22): aplicar `parse::expression_primary_span` en el origen del cargo `ArrayIterationLength` (`lower.rs` ~3160) y en el primario de la barrera de la iteración al activar `for_iteration`; hoy inalcanzable (`for` es NX-E0006 o NX-E0002 antes del lowering). Se revisa en el corte que active `for`, o antes en la siguiente campaña de mutación.
- **Supervivientes de mutación sin programa** (no bloquean sin programa; deuda con fecha: revisión en la campaña extendida nueva sobre el código vigente): los 7 de S6-COR-2 (M00013, M00021, M00023, M00051, M00054; M00047 y M00048 por `for` inalcanzable en el perfil; Pending `I07C-S6COR2-DEUDA-MUTACION`), M06b, M09, M09b (S6), Q4 y Q7 con equivalentes S2e y M8 (S6-COR), 47 de S4, 95 de ccf (`I07C-MUT-SIN-PROGRAMA`). Los identificadores M06b, M09 y M09b proceden de la lista del encargo y **no se localizaron en `Pending.md` de `origin/main`**: NO VERIFICADO su alcance; el informe de mutaciones inversas es `docs/correcciones/I07c-S6-mutaciones-inversas.md`.
- **Otros** (Pending): `I07C-S4-VS-BASE` (`let z=(true+([r] or r))` con `r` irresuelto: 3 diagnósticos frente a un posible cuarto según N(b)/N(d); prioridad baja, Flujo A tras F5; NO VERIFICADO hoy); campaña `I07C-MUT-EXT` histórica pausada; 41 tests `adr_gap_*` que no son oráculo; filas «sin cobertura» de la matriz de cierre (19 en el borrador: p. ej. L3-15, L3-19, T-F1p, T-F2p, Q-3) sin tratar.
- **Esta lista no sustituye** el contrato de cierre ni el informe de D' de I07c: solo registra defectos con programa. No se afirma PASS de I07c ni de ninguna fase.

## 6. Estado de I07c tras la exploración (2026-11-23)

Fuente única: la rama `origin/i07c-estructural-r2` (informes de la exploración de S5c y planes de B). Las cifras de esta sección están copiadas de esos informes (se indica cuál); no se han vuelto a medir. La exploración es **desechable y no integrable**: prototipos con interruptores `NEXO_XV` sobre `origin/main` `eeb3c9c`, sin tests de producción ni puerta completa.

### 6.1 Resultado

Ninguna variante del mecanismo de ancla de unión (ALT-2 con las piezas R1/RE/RM-M3) cumple a la vez «0 regresiones de core frente a `main`» y «ningún defecto nuevo». Los barridos llegaron a 0 regresiones de core en todos los corpus solo con la pieza `atomic0` (INFORME-4 a INFORME-6, candidato-5); `atomic0` deja un defecto propio (test `planned_projection_child_does_not_use_failed_branch_as_witness`, INFORME-6 §3) y, retirada o variada, reintroduce regresiones (INFORME-7, INFORME-8). **Nada de d2 se integra.** El diseño d2 validado por B para R1 y RE (`docs/correcciones/I07c-S5c-d2-diseno.md` y `docs/correcciones/I07c-S5c-d2-diseno-B-r2.md`) y el plan (`I07c-S5c-d2-plan.md`, con su FAIL `I07c-S5c-d2-plan-B-r1.md`) quedan como **entrada** del rediseño, no como plan ejecutable.

### 6.2 Causa estructural común

**Restauración atómica del tipo de elemento al fallar un contraste.** ALT-2 (registro de ancla por unión) y las piezas R1 y RE se apoyan en `atomic0` (el primer contraste incompatible restaura los nodos de tipo previos: igualdad atómica; cita del diseño d2: línea 299 de `docs/adr/F0-0002-second-slice.md`, «una igualdad entre tipos compuestos es atómica»). Sin `atomic0`, la base ALT-2 sola da **1.007 regresiones de core** frente a `main`; con `atomic0` da 0, pero `atomic0` tiene su propio defecto en `planned_projection…` (programa mínimo de INFORME-6 §3: `fn Main(){let x=1;var q=[];var s=[];let a=[s[x],(if 0 then true else q),(if x then q else s)];q=[true];s=[[true]];}`: el candidato da `NX-E0102` en `q` donde `main` y el oráculo dan `NX-E1303`).

Mecanismo común de las 56 y de las 1.007 regresiones (INFORME-8 hallazgo 1, `ejemplos-regresiones-base.txt`): tras denunciar la unión, una asignación posterior (`r=q;q=[[ar]]`) produce un `NX-E0102` de más. Ejemplo (corpus t, índice 3584, `f0-0002-b-v2`): `fn Main(){let p=true;let ar=[1];let aar=[[1]];let r3=[[[1]]];var q=[];var r=[];var s=[];let a0=(if p then [q] else (if p then ar else q));r=q;q=[[ar]];}` → `main` y oráculo: `NX-E0103` `[85,87)` y `NX-E1303` `[126,128)`; sin `atomic0`: además `NX-E0102` `[146,148)`.

### 6.3 Cifras de la ablación (INFORME-8)

Regresiones de core (A: `main` acierta frente al oráculo y el candidato no) en todos los corpus (c1, c2, corpus-b, perm, corpus-t y las 3 semillas de B), todo **sin `atomic0`** (hallazgo 1):

| conjunto sin `atomic0` | regresiones de core frente a `main` |
|---|---|
| base ALT-2 sola (todas las piezas apagadas) | 1.007 (c1 516, t 61, b 67, perm 51, B 13+10+8, c2 ~230) |
| solo RM-M3 (`rmm3b,e,f`) | 1.004 |
| solo RE (`e0103rule,skipv3`) | 990 |
| solo R1 (`pendfa,retfa,retract,nojoin,nojoinr,retfail`) | 73 |
| R1+RE (sin `match`) | 63 |
| R1+RE+RM-M3 (candidato-7) | 56 en c1/t/B (+7 en c2 según la tabla por forma) |
| R1+RE+RM-M3 **con** `atomic0` (candidato-5) | 0 |

De las 56 del candidato-7, 54 ya aparecen con todas las piezas apagadas; quitar una sola pieza restituye `main` en 3 de 56; añadir `atomic`, `defmark` o `noseed` no restituye ninguna; `atomic0` restituye `main` en las 56 (`clasificacion-56.json`).

Pieza a pieza, con el conjunto completo del candidato-5 (hallazgo 2; cifras no aditivas):

| pieza | arreglos perdidos al quitarla | regresiones que aparecen |
|---|---:|---:|
| `pendfa` | 316 | 868 |
| `retfa` | 142 | 26 |
| `retract` | 49 | 3 |
| `nojoin` (+`nojoinr`) | 178 | 1 |
| `atomic0` | 247 | 56 |
| `e0103rule` | 290 | 11 |
| `retfail` | 207 | 12 |
| RM-M3 (`rmm3b/e/f`) | 7 | 7 |
| `skipv3` | 0 en corpus (efecto de aceptación) | 0 |

### 6.4 Variantes de `atomic0` medidas (INFORME-6 §3, INFORME-7 §1)

| variante | resultado |
|---|---|
| `atomic0e` (restaurar solo con error) | 56 regresiones de core (c1 4, s7021993 1, s9908255 1, t 50); rechazada |
| `atomic0p` | sin efecto |
| `atomic0e`+`atomic0p` | 56 regresiones, mismos índices; rechazada |
| `a0np` (no restaurar en la pasada de planificación si no hay error) | iguala `main` en `planned_projection…` e `incompatible_prefix…`, pero pierde `failed_nested…`; rechazada |
| `a0sc` (no restaurar si el ancla es escalar concreto y lo hallado una incógnita) | matriz 40/45 core (fallan W0, B_qr, P1, c3, w1; con `atomic0` 45/45); rechazada |
| `a0de` (no restaurar si la unión es elemento de array pendiente) | sin efecto (la bandera es falsa en ese punto); rechazada |
| retirar `atomic0` (candidato-7) | 56 regresiones de core frente a `main` (50 en corpus t, 4 en c1, 2 en B); arregla `planned_projection…` e `incompatible_prefix…` |

Ninguna variante cumple «ninguna pieza empeora lo que `main` acierta». Resto de la exploración (INFORME a INFORME-5): 2f/2g' (R1+RE) y RM-M3 sí llegan a 0 regresiones de core, pero solo con `atomic0`; V6 original da 53 regresiones de core y 78 arreglos sobre c1 (INFORME §2) y su retracción de capas sembradas «daña la procedencia» (INFORME §7.4). Matriz de 45 filas: `main` 22/45 en core y 21/45 en full; 2g' y candidato-5 45/45 en core y 38/45 en full (INFORME §2, INFORME-3 §5).

### 6.5 Decisión

1. No se integra nada de d2, ni R1/RE/RM-M3 ni `atomic0`. Los defectos de §2.1 y §2.2 permanecen en esta lista.
2. **Disparador con fecha: Codex 2026-11-24 04:43: rediseño de la restauración atómica por Flujo A, primero, antes de F5 B2b+; dueño S5c-d3.** El brief del rediseño está en `docs/correcciones/I07c-rediseno-restauracion-atomica-brief.md`.
3. S5a.2, S5a.3 y S5a.5 quedan **al rediseño**.
4. Ninguna fila abierta por la exploración fallida se elimina: el rediseño las cubre.

### 6.6 Decisiones de lectura del coordinador (revertibles; Flujo A para cambiarlas)

- **T5 literal**, también para continuaciones con condición fallida: una `if` con condición fallida es una unidad contaminada que no establece referencia. `main` diverge en `fn Main(){let q=(if true then (if 1 then 1 else 2) else true);}`: da `NX-E0102` `[34,35)` y `NX-E1303` `[56,60)` (`true`), igual que el prototipo (B, `I07c-S5c-tests-B-r1.md` fila 9). Es **defecto de `main` con repro**; el esperado exacto de diagnósticos NO está derivado en las fuentes (el ejemplo de T5 del ADR es de array). Dueño: S5c-d3.
- **`pending_inner`: lectura B**, adoptada en `docs/adr/F0-0002-lagunas.md` (`6b6f7c5`); el test `pending_inner_constructor_preserves_outer_branch_comparison` cambia de esperado con ella.
- **Literal de array con elemento erróneo (N)**: adoptado (`eeb3c9c`).
- **`match` con valor y uniones**: conservación pura del comportamiento de `main`, sin regla del ADR; datos de referencia: la tabla de 54 filas (`docs/validation/i07c-s5c/match-conservacion.json`) y la de 103 filas declaradas (`docs/validation/i07c-s5c-explor/match-conservacion-ext.json`: 51 de conservación y 52 diferencias declaradas, 3 + 28 + 21; INFORME-6 §5).
- **`cyclic_constructor_errors_keep_inference_code_at_each_site`**: inconsistencia entre formas equivalentes (§4.1.2.7), **defecto con repro de las dos formas** (`I07c-S5c-tests-B-r1.md` fila 6): `fn Main(){var m1=[];AssertEq([true],[m1[7],m1]);}` (forma inline) da `NX-E0103` `[17,19)` y `[36,46)` (candidato) o además `[43,45)` (`main`); `fn Main(){var m1=[];let a=[m1[7],m1];AssertEq([true],a);}` (forma `let`) da en ambos solo `NX-E0102` `[33,35)`. Solo está respaldado retirar `[43,45)` (una lectura, ADR §4.1.2.2); el esperado de la forma inline NO queda derivado del ADR.
- **Test eliminado por el candidato** `check::types::quota_tests::i07c_survivor2_nested_shape_hints_reach_the_leaf` (prueba `ShapeHint`; matador registrado de M00123 y M00125): **no se elimina sin sustituto**.

### 6.7 Informes enlazados (todos en `origin/i07c-estructural-r2`)

- Exploración: `docs/validation/i07c-s5c-explor/INFORME.md`, `INFORME-2.md`, `INFORME-3.md`, `INFORME-4.md`, `INFORME-5.md`, `INFORME-6.md`, `INFORME-7.md`, `INFORME-8.md` (con `piezas-excluidas-cifras.json`, `ejemplos-regresiones-base.txt`, `clasificacion-56.json`, candidatos `candidato-*.diff`).
- Diseño y plan d2: `docs/correcciones/I07c-S5c-d2-diseno.md`, `I07c-S5c-d2-diseno-B-r2.md`, `I07c-S5c-d2-plan.md`, `I07c-S5c-d2-plan-B-r1.md`.
- Tests: `docs/correcciones/I07c-S5c-tests-B-r1.md`.
- Evidencia de B y corpus: `docs/validation/i07c-s5c/` (`b-d2/`, `b-d2plan/`, `b-tests/`, `match-conservacion.json`).

No verificado en esta sección: ninguna cifra se re-midió; el contenido de los informes de D y D' de S6-COR-2 no está en `origin/main`.
