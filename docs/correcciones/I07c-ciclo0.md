# I07c — Ciclo 0 (solo medición)

Medición de la base `e6e5f82` (rama `i07c-estructural`) sin tocar `src/`, `tests/` ni el ADR. Clon limpio en
`D:\nexo-work\i07c-c0\base`, `CARGO_TARGET_DIR=D:\nexo-work\target-i07c-c0*`, `TEMP/TMP=D:\nexo-work\tmp`,
compilación `cargo build --locked --offline` en release y debug. Todos los programas de este documento están
guardados exactamente como se ejecutaron en `D:\nexo-work\i07c-c0\r\*.n`.

Convenciones. `DV` = `(if true then {return;} else {return;})`. Salidas CLI con `nexo <check|run> <f> --profile
f0-0003-b-v1 --format json`. «Biblioteca» = `nexo::ir::{check_scalar, compile_scalar}` desde una sonda externa
(`D:\nexo-work\i07c-c0\probe`), que expone `CompileError::InvalidIr`. Los gemelos concretos (`*_t`, `*_ok`) son
los de `Nexo\.scratch\val19-tipos\repros`. Nota de fuentes: la ruta `Nexo-robustez\.scratch\insumos\` no existe;
los repros y el informe `val19-tipos` están en `C:\Projects\Nexo\.scratch\val19-tipos\` (`repros\`, `informe.md`),
y `bucle-estado.md` sí está en `Nexo-robustez\.scratch\`.

## 1. Tabla resumen

| Ítem | Estado | Evidencia |
|---|---|---|
| a) B1 `InvalidIr`/`internal_error` en `(r==r)` sobre array pendiente | ABIERTO | b1a/b1b/b1c: `internal_error` (exit 2) en `check` y `run`, debug y release; biblioteca `InvalidIr`. Gemelos concretos aceptan. §2 |
| b) B2 aceptar lo que el ADR rechaza (b2a–b2e, b2g) | ABIERTO (6 de 6) | Los seis se aceptan en `check` y `run`, debug y release; el gemelo concreto rechaza con NX-E0102. Variantes nuevas y repro mínimo b2g (q4w). §3 |
| B3 `Some([])` en `match` (hallazgo J16, PENDIENTE §6) | ABIERTO | `internal_error`; repro mínimo más pequeño que el original (sin guarda). §4 |
| c) Campaña `twin` | Sin bug de núcleo en la muestra; adjudicación parcial | 28.000 programas: `bugs=0`, `dups=0`, `other_differences=0`; 6 firmas señaladas, 4 ya adjudicadas (§4.1.2.7), 2 `perm_diff` sin adjudicar. §5 |
| d) Mutantes M00017 / M00073 | M00073 CERRADO (muere en debug y release); M00017 INCONCLUSO (literal ya no existe; su análogo muere) | §6 |
| e) Lagunas ADR (`[DV][i]` vs `DV[i]`; unión exterior con `[]` primero) | ABIERTAS (comportamiento documentado, sin decidir) | §7 |
| f) ck1 (`check` NX-E0105 vs `run`) | NO REPRODUCIBLE en la base (check y run dan NX-E0105) | §8 |
| g) Requisito de solver sin recursión | CONFIRMADO ABIERTO (solo debug) | `Some(`×84/×126, `None[[…]]`, Array inválido×126 desbordan a 256 KiB en debug; release no. `Option<…>`×126 + `match` cabe en 256 KiB. §9 |
| h) Puertas de base | RD 152/152 (debug y release); objetivos 15/29; `validate.sh` exit 0 en 1.236 s (20 min 36 s) | §10 |

Familia común (detalle en §11): el resultado nominal de una operación cuyo dominio sigue indeciso, o cuyo
operando es `Never`/pendiente, no es un tipo pendiente del solucionador único; se representa como
`Type::Tainted` más conjuntos laterales por span. B1, B2 y B3 son tres síntomas (tipo no exportado, consumidor
descartado, evidencia contaminada) de esa misma representación.

## 2. B1 — `InvalidIr`/`internal_error` (ABIERTO)

Esperado (ADR §4.1.2 N(f), «Ausencia de errores internos, pánicos e `InvalidIr`»; N(a) y §4.1.1 «aceptación
independiente del orden»; el gemelo concreto se acepta). Obtenido, idéntico en debug y release y en `check` y `run`:

| Programa | CLI (check y run, debug y release) | Biblioteca (`check_scalar` y `compile_scalar`) | Gemelo concreto |
|---|---|---|---|
| b1a `fn Main(){var r=[];r=[(r==r)==true];r=[true];}` | exit 2, `driver_error/internal_error` | `InvalidIr(unresolved checker array type)` | `var r=[true];r=[(r==r)==true];` → checked / unit |
| b1b `fn Main(){let ar=[1];var r=[];let a=(if (r==r)==true then 1 else 2);r[0]=r[0];r=ar;}` | igual | `InvalidIr(write type)` | acepta |
| b1c `fn Main(){let p=true;let aar=[[1]];var r=[];let b=(if p==(r==r) then 1 else 2);let c=(r==aar);}` | igual | `InvalidIr(write type)` | acepta |

Reducciones medidas (biblioteca, release; debug idéntico en b1a): `var r=[];r=[(r==r)==true];r=[true];`,
`…r=[false];` y `…r=[1];` dan `InvalidIr(unresolved checker array type)`. Sin el `==true` externo
(`r=[(r==r)];r=[true];`) se acepta; con el array fuera de la asignación (`let a=[(r==r)==true];r=[true];`) se
acepta. `r=[(r==r)==true];r=[1];` devuelve `InvalidIr` en lugar de un diagnóstico (laguna de autorreferencia
hipotética: no se afirma qué diagnóstico corresponde; sí que `InvalidIr` viola N(f)). `var r=[];r=[(r==r)==true];`
solo da NX-E0103 `[16,18)` (comportamiento actual; el ADR no fija el caso autorreferente).

Causa raíz (leída en el código; no instrumentada): el literal de array llega a `lower.rs` con el tipo de su
elemento sin resolver. `src/ir/lower.rs` (rama `Task::ArrayNext`, línea 7344) devuelve
`InvalidIr("unresolved checker array type")` cuando `state.inferred == Ty::Never` en modo `Live`; el elemento
`(r==r)==true` llega sin tipo porque en el comprobador es `Type::Tainted` (devuelto con `pending_nominal_available`
falso en `types.rs::expr_inner` líneas 8889–8893, y en `finish_deferred_binary_right` 9616 y 9624), y la
exportación `types.rs` 11857–11868 solo recupera un tipo de un `Tainted` si existe una entrada lateral en
`conditional_result_types` (insertada en `check_against_context_one`, líneas 8436–8446, y solo cuando el sitio
está en `pending_domain_calls`) y `checker.errors.is_empty()`. Si el `Tainted` nace por otra ruta, se exporta
`ResolvedType::Unresolved` y el lowering lo ve como `Never`.

## 3. B2 — se aceptan programas que el ADR rechaza (ABIERTO)

Esperado: §4.1.1 («`Never` … no añade restricciones concretas»; «La aceptación se determina con todas las
restricciones locales, también en código hipotético») y §4.1.2.3 «Cierre de dominio indeciso» (el tipo nominal
independiente —`I64` de `+`/`Len`, `Bool` de `<`/`==`— sigue disponible para comprobar a sus consumidores aunque
el predicado de dominio de un argumento siga indeciso; una incompatibilidad de ese resultado con un contexto
concreto es C), N(a) (aceptación igual en la forma equivalente con contexto concreto), N(b)/N(c) (código, subcausa y
primario del C). El §14.10 «pares de fondo» (`DV+true`, `true+DV`) confirma que un operando `Never` no convierte la
operación en `Never`: el otro operando y el dominio de la operación se siguen comprobando. Los gemelos concretos
(`var q=ar` con `let ar=[1]`) rechazan en ambos perfiles y en `check` y `run`.

Obtenido en las seis fuentes originales, debug y release, `check` y `run`: `checked` / `unit`, exit 0, sin
diagnósticos.

| Fuente | Esperado (lo que da el gemelo concreto, debug y release) |
|---|---|
| b2a `fn Main(){var q=DV;let z:Bool=DV+Len(q);}` | NX-E0102/type_mismatch, primario `DV+Len(q)` [41,87), `expected_type=Bool`, `found_type=I64`, relación `expected_type` [36,40) |
| b2b `…let z:I64=DV<Len(q);` | NX-E0102/type_mismatch, primario `DV<Len(q)` [40,86) |
| b2c `…let z:I64=DV==(q==q);` | NX-E0102/type_mismatch, primario [40,87) |
| b2d `var r=DV;let a0=DV+(DV-Len(r));let b=[a0,r];` | NX-E0102/type_mismatch en `r` [136,137) |
| b2e `let br=[true];var q=DV;let a0=(DV-Len(q))<1;let a1=a0==br;` | NX-E0102/type_mismatch en `br` [113,115) |
| b2g `let p=true;let ar=[1];let aar=[[1]];var r=[DV];let a=(if (r==r)==p then ar else ar);let c=a==r;r=aar;` | NX-E0102/type_mismatch sobre `aar` de `r=aar`, `expected_type=Array<I64>`, `found_type=Array<Array<I64>>` (el gemelo `b2g_t` da [105,108); la variante `if (r==r) then` en la misma base lo rechaza correctamente) |

El enunciado del ciclo pide que b2g dé NX-E0102: se confirma que la base lo acepta (`checked`, exit 0).

Variantes medidas (release; las de debug coinciden en las que se ejecutaron: b2a–b2g y q4w):

- Operando `Never` con operando de dominio pendiente, aceptado: `var q=DV;let z:Bool=DV+Len(q);` (b2a),
  `…z:I64=DV<Len(q);` (b2b), **`…z:I64=Len(q)<DV;` (variante nueva: `Never` a la derecha de `<`)**, y
  `…z:Bool=DV+Len(q);q=[1];` (también aceptado cuando `q` se resuelve después a Array).
- Controles correctos en la misma base: `…Len(q)+DV` con `Bool` se rechaza (primario [67,113)); `…Len(q)<2` con
  `I64` (RD) se rechaza; `1+Len(q)`, `Len(q)+1`, `DV+1` y `DV<1` con anotación incompatible se rechazan.
  Por tanto la pérdida es específica de «`Never` como operando + operando con dominio pendiente», en `+`/`-` con
  `Never` a la izquierda y en `<`/`==` con `Never` en cualquier lado.
- Repro mínimo para b2g (el mecanismo no es el `Never` aritmético): `fn Main(){let p=true;let ar=[1];let
  aar=[[1]];var r=[DV];let c=(if (r==r)==p then 1 else 2);let d:Bool=c;r=aar;}` se acepta (debug y release,
  `check` y `run`). Esperado: NX-E0102 sobre `c` [137,138), `expected_type=Bool`, `found_type=I64`. Gemelos que
  rechazan bien: `var r=aar;` (q4t, primario `c` [102,103)), `var r=[];` (q4v), y `if (r==r) then` sin `==p` (q4u).
  Con `let d:Bool=c;` ausente y sin `r=aar;` el programa da NX-E0103 sobre `[DV]`. Es decir, `c` es `if … then 1
  else 2`, de tipo `I64` independiente de `r`, pero pierde su evidencia (queda como binding contaminado).
  Otras variantes aceptadas por la misma causa: `let a=(if (r==r)==p then ar else ar);let c=a==r;let
  d:Array<Bool>=r;` (z6), y `(if (r==r)==p then ar else ar)==r` directo (q1).

Causa raíz (leída en el código; las variantes anteriores la delimitan, sin instrumentación):

1. b2a/x13 (`+`/`-` con `Never` a la izquierda). `types.rs::finish_deferred_binary_right`, bloque
   `pending_domain_calls.contains(right)` (líneas ~9588–9617): inserta la operación en `pending_domain_calls`
   (9608), lo que hace que `check_against_context_one` omita la comparación contextual inmediata (condición
   `!self.pending_domain_calls.contains(...)`, ~8465), pero para `+`/`-` no registra la obligación diferida del
   consumidor (`defer_nominal_domain(... expected, expected_span ...)`), que sí registra la ruta no diferida en
   `expr_inner` 8877. El `Bool` de `let z:Bool` no se compara nunca con el `I64` nominal.
2. b2b/b2c/b2e y `Len(q)<DV` (`<`/`==` con `Never` en un lado). El mismo bloque (9588–9606) y `expr_inner` 8846–8873
   encolan un `PendingGuardedComparison` con `consumer_expected`; su cierre en `types.rs::function`
   (~5636–5675) exige `resolved_join_side(expected)` y `resolved_join_side(found)` para comprobar nada, también
   el consumidor. `resolved_join_side` devuelve `None` para `Type::Never` (línea 3695). El lado `Never` hace que se
   descarte la comprobación del consumidor completa (el `Bool` nominal frente a `I64`), aunque el consumidor no
   depende de él.
3. b2g/q4w (condición con dominio pendiente sobre array de elemento `Never`). `expr_inner` 8828–8893: con el
   operando izquierdo en `pending_domain_calls`, `pending_nominal_available(left)` (2866) es falso y la operación
   devuelve `Type::Tainted` (8893). Ese `Tainted` activa `self.tainted` y, vía `finish_statement_at_site`
   (5695–5760), marca el binding `c`/`a` como inválido, que ya no aporta evidencia (`a==r` no fija `r`; `d:Bool=c`
   no se comprueba). El ADR distingue explícitamente «predicado indeciso» de contaminación (§4.1.2.3: «no es un
   error ni contaminación»). La diferencia con `var r=[]` (que sí rechaza) no se ha aislado; queda INCONCLUSA
   la razón exacta por la que `[DV]` y `[]` difieren en `pending_nominal_available`.

## 4. B3 — `internal_error` con `Some([])` en `match` (ABIERTO)

Programa del hallazgo (PENDIENTE §6): `fn Main(){match Some([]){Some(v) if false=>{let a:Array<I64>=v;},Some(w)=>{let
b:Array<I64>=w;},None=>{}}}`. CLI, check y run, debug y release: exit 2, `driver_error/internal_error`. En
biblioteca (perfil escalar) `Option` da NX-E0006, por eso solo se reproduce por la CLI.

Reducción propia: `fn Main(){match Some([]){Some(v)=>{let a:Array<I64>=v;},_=>{}}}` da el mismo `internal_error`
(no hace falta la guarda). Controles: `Some([1])` en el mismo `match` se acepta; `let x=Some([]);match x{Some(v)=>{
let a:Array<I64>=v;},None=>{}}` da `internal_error`; `match Some([]){Some(v)=>{},None=>{}}` da NX-E0103 `[]` [21,23) (el
elemento queda sin resolver); `match Some([]){Some(v) if false=>{…Array<I64>…},Some(w)=>{…Array<Bool>…},None=>{}}`
da NX-E0102 [93,94). Esperado: aceptación, con `[]` resuelto a `Array<I64>` por la anotación (§4.1.1, §4.1.2.2),
o, si el ADR se interpreta de otro modo, un diagnóstico; no `internal_error` (N(f)).

Causa raíz: no localizada por instrumentación. Candidata de la misma familia: el tipo de la variable ligada por el
patrón (`Checker::pattern_type`, 9930) se fija después de resolver el `[]` pendiente, y la exportación a
`InferredTypes` (11857–11868) o el lowering de la variable de patrón ve un tipo sin resolver (el origen concreto de
`internal_error`, `InvalidIr` o pánico, no se ha medido: la CLI colapsa los tres en `internal_error`). INCONCLUSO.

## 5. Campaña `twin` de tipos diferidos (c)

Arnés: `scripts/i07c_typed_campaign/main.rs` (modo `twin N SEMILLA HILOS PROFUNDIDAD`, generador `gen_prog`). No
compila en `e6e5f82` porque `nexo::interp::run_scalar` ahora recibe `MockEnvironment`; para medir se usó una copia
fuera del repo (`D:\nexo-work\i07c-c0\tw`) con una única línea cambiada
(`run_scalar(&p, nexo::interp::MockEnvironment::empty())`); no se tocó el repo. Esto es un hallazgo de andamiaje:
el arnés está roto en la base.

| Corrida | Programas | Bugs | Duplicados | `other_differences` (nonA_diff) | Señalados | Reducidos |
|---|---:|---:|---:|---:|---:|---:|
| release, semilla 17021, profundidad 3, 4 hilos | 20.000 | 0 | 0 | 0 | 1 | 1 |
| debug, semilla 17022, profundidad 3, 4 hilos | 8.000 | 0 | 0 | 0 | 3 | 3 |
| release, semilla 10495373245766874321 (la de `F0-0002-I10a-typed-campaign-results.json`), profundidad 3, 4 hilos | 20.000 | 0 | 0 | 0 | 2 | 2 |

Clasificación (cada firma comprobada contra `docs/validation/i07c-typed-exceptions.json`, que adjudica según
§4.1.2.7 las firmas de las semillas 17021, 17022, 17401, 17402, 28501 y 28502):

- Semilla 17021, idx 5784: adjudicada («r is required as Bool by earlier if conditions…»). RESUELTA por adjudicación.
- Semilla 17022, idx 649, 6263 y 7839: las tres adjudicadas en el manifiesto. RESUELTAS por adjudicación.
- Semilla de I10a, idx 18584 (`perm_diff [1,0]`) e idx 5871 (`perm_diff [2,1,0]`): la semilla de I10a no figura en
  el manifiesto; SIN ADJUDICAR. Son los casos que `Pending.md` cuenta como «19 señalados + 8 `other_differences`
  (release) y 10 + 3 (debug)» sobre 300.000 programas; esas cifras no se han reproducido (se midieron 20.000, no
  300.000), por lo que el estado de los 8+3 `other_differences` es INCONCLUSO a escala completa; en la muestra
  de 28.000 programas no apareció ninguno.
  Ejemplo medido (idx 18584 reducido, prefijo `let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];`):
  `var r=[];var q=[];AssertEq((if ar then r[0] else q),(if 1 then r else ar));r=[3];q=[3];` da cinco
  diagnósticos (NX-E0103 [74,76], NX-E0103 [83,85], NX-E0102 [99,101], NX-E1303 [117,118], NX-E0102 [124,125]);
  el mismo programa con `q=[3];r=[3];` da cuatro y sin NX-E0103 (NX-E0102 [99,101], NX-E0102 [124,125], NX-E1303
  [138,140], NX-E0102 [152,153]). Ambos rechazan; difiere el conjunto de diagnósticos al permutar dos asignaciones
  que resuelven `r` y `q`. No se adjudica aquí si el NX-E0103 con `r=[3]` posterior es un defecto (N(b)/N(d)) o
  una consecuencia legítima de §4.1.2.7: INCONCLUSO, requiere adjudicación contra el ADR.

Lo abierto frente a lo cerrado desde F0-0007/J14: en las muestras no hay `bugs`, `dups` ni diferencias de aceptación
entre gemelos; todo lo señalado es una diferencia de primer diagnóstico o de permutación. Lo cerrado: las firmas de
las semillas del manifiesto. Lo abierto: las firmas `perm_diff` de la semilla de I10a (0 adjudicadas) y la escala
completa (300.000).

## 6. Mutantes M00017 y M00073 (d)

Resultado registrado antes (`docs/validation/i07c-mutations/results.jsonl`): ambos `detectada` en release y
`superviviente` en debug, con testigos `array_assignment_keeps_first_element_origin_after_conflict` (M00017) e
`independent_numeric_error_has_no_inferred_expected_origin` (M00073), de `tests/i07c_mutation_regressions.rs`.

Medición en copia desechable `D:\nexo-work\i07c-c0\mut` (`git archive e6e5f82`, `CARGO_TARGET_DIR` propio, SHA-256
de `src/check/types.rs` verificado antes y tras cada mutación: `RESTORED_OK`). Solo se midieron estos dos; no se
corrió el arnés `scripts/i07c_mutaciones.py` ni la población de 1.174.

- M00073 (`drop_state`, `self.concrete_plan.insert(site, concrete);` en la rama `else if !has_deferred_join`;
  línea original 961, hoy 2118): el literal existe y se aplicó. El testigo falla (el mutante muere) en debug y en
  release; el otro test del par pasa. CERRADO. Base sin mutar: ambos tests pasan en debug y release.
- M00017 (`eq`: `.all(|item| self.intrinsic_assignment_type(item).as_ref() == Some(&first))`, línea original 736):
  el literal ya no existe en `e6e5f82` (J14/J16 convirtieron `intrinsic_assignment_type` en recorrido iterativo con
  `Frame::Array`). La mutación tal como está registrada no es aplicable: INCONCLUSO para el mutante literal.
  Se midió el análogo directo en el código actual (`if value != first {` → `if value == first {`, en
  `Frame::Array`, `types.rs` ~1883): el testigo falla (muere) en debug y en release. Es un análogo escrito a mano,
  no el mutante registrado.

## 7. Lagunas del ADR (e): comportamiento actual, sin decidir

`DV` = `(if true then {return;} else {return;})`. Debug y release coinciden; `check` y `run` coinciden.

- `DV[i]` frente a `[DV][i]`. `fn Main(){let a=DV[0];}` → acepta. `fn Main(){let c=true;let a=(if c then [DV][0] else
  1);}` → acepta. `fn Main(){let a=[DV][0];}` → NX-E0103 sobre `[DV]` [16,57), sin más diagnósticos. El ADR §4.1.2.3
  fija la proyección condicional, pero no el tipo del resultado de una proyección de `Never` ni de `[Never]`.
  Test que congela esto: `adr_gap_never_index_projection_keeps_current_resolution` (`src/ir/i07c_tests.rs`).
- Unión exterior con `[]` primero. `fn Main(){let c=true;let br=1;let m0=(if c then [] else (if c then [1] else
  br));}` → NX-E0103 sobre el `[]` [48,50) y NX-E1303 sobre `br` [76,78). §4.1.2.4 no decide si el testigo parcial de
  la unión interior fallida debe resolver el elemento del `[]` de la rama anterior. Test que lo congela:
  `adr_gap_nested_union_and_array_element_keep_current_causal_sites` (`tests/i07c_causal_obligations.rs`).
- Laguna adicional observada (no listada en el pendiente): autorreferencia hipotética `var r=[];r=[(r==r)==true];`
  → NX-E0103 [16,18) (el elemento tiene tipo nominal `Bool`, pero su evaluación depende de `r`); y con `r=[true];`
  después, `InvalidIr` (B1).

## 8. ck1 (f)

La deuda documentada es: `check` da NX-E0105 en una escritura a elemento inmutable mientras `run` del módulo sin
`Main` da solo NX-E0112. En `e6e5f82` no se reproduce. Fixtures `tests/fixtures/i07c/
immutable_array_element_write_without_entry.n` y `missing_entry_preempts_immutable_element_write.n`
(`fn F(){let a=[1];a[0]=2;}`): `nexo check` y `nexo run` dan solo NX-E0105 ([125,127) y [17,18)), en debug y
release; en biblioteca `check_scalar` y `compile_scalar` dan NX-E0105. Un módulo sin `Main` y sin error
(`fn Other(){}`) da `check` aceptado y `run` NX-E0112/missing_main [12,12), como debe. Ningún test de
`tests/` referencia esos dos fixtures (`grep` vacío). Estado: no reproducible; el texto de `Pending.md` líneas
26–27 está desactualizado respecto de la base (prioridad de propiedad ya precede a la entrada). Sin deuda
observable de `check`/`run` aquí.

## 9. Requisito del solver sin recursión (g)

La CLI usa un hilo de 16 MiB en debug (8192 KiB × 2) y 2 MiB en release. Para medir el contrato de 256 KiB se usa
el control de pruebas existente de `src/lib.rs::frontend_stack_size`:
`NEXO_STACK_MINIMUM_CASE=1 NEXO_PROBE_CHECKER_KIB=<n>` (mínimo aceptado 256). No se midió a 204 KiB: el
filtro `>= 256` lo impide por la CLI, y las sondas unitarias internas con 204 KiB (p. ej. `j16_direct_pattern_type_child`,
`types.rs` 12209) no se ejecutaron.

Programas (generados por script, guardados en `D:\nexo-work\i07c-c0\r\g*.n`): `g1_some126` =
`fn Main(){let x=` + `Some(`×126 + `1` + `)`×126 + `;}` (775 B); `g1_some84` igual con 84; `g2_optmatch126` =
`fn F(x:view Option<…×126 I64…>){match x{_=>{}}}fn Main(){}`; `g2_optmatch_some126` con el patrón `Some(`×126 `_`
`)`×126 y brazo `_`; `g3_none126` = `fn Main(){let x=None[` + `[`×125 + `1` + `]`×126 + `;}`; `g4_arrbad126` =
`fn Main(){let x:Array<I64>=` + `[`×126 + `true` + `]`×126 + `;}`.

| Programa (`nexo check`) | debug, pila por defecto | debug, 256 KiB | debug, 512 KiB | release, 256 KiB |
|---|---|---|---|---|
| g1_some126 | checked | **desborda la pila** (aborto, sin JSON) | checked | checked |
| g1_some84 | checked | **desborda** | checked | checked |
| g3_none126 | NX-E0113 [16,20) | **desborda** | NX-E0113 | NX-E0113 |
| g4_arrbad126 | NX-E0102 [28,282) | **desborda** | NX-E0102 | NX-E0102 |
| g2_optmatch126, g2_optmatch_some126 | checked | checked | checked | checked |

Resultado: el solver recursivo (`Checker::bind`, 3321; `unify`, 3702; `selection_type`, 3276;
`resolved_syntax_type`, 3253; `concrete_with_depth`, 3150) desborda a 256 KiB en debug con `Some(`×84 y ×126,
`None[[…]]` y Array inválido ×126; no en release. `Option<…>`×126 con `match` cabe en 256 KiB en esta medición
(la fila `J16-S1C-MATCH-TYPE-DEPTH` registra 193 KiB, por debajo del contrato; no se midió a 204 KiB). Solo se
registra el resultado.

## 10. Puertas de línea base (h)

- `sh scripts/validate.sh` en clon limpio (`git clone` + checkout `e6e5f82`, target nuevo
  `D:\nexo-work\target-i07c-c0-val`, tras la compilación): exit 0 (`VALIDATE_EXIT=0`), duración 1.236 s (20 min 36 s) con campañas y mutaciones corriendo a la vez en la misma máquina (por tanto, tiempo orientativo). fmt y clippy `-D warnings` sin salida de error; pruebas debug y release: suma de las líneas `test result` de ambos perfiles = 3.796 pasadas, 0 falladas, 145 ignoradas (los `ignored` incluyen las campañas extensas que `validate.sh` no ejecuta).
- RD (`scripts/i07c_rd_conformidad.py --verify <nexo.exe>`): `RD normative: 152/152 check/run pairs`,
  `RD snapshot differences: 0`, en release y en debug.
- Objetivos (`python scripts/objetivo.py`, binario release): 15/29 ejecutan (01–15 ejecutan; 16–29 no: NX-E0006 en
  16–19 y 21–29, NX-E0002 en 20).

## 11. Causa raíz y familia común

Familia: **restricciones y resultados nominales sobre tipos pendientes que no pasan por un único solucionador**.
En `src/check/types.rs` (14.432 líneas) conviven cuatro mecanismos que deciden por separado qué sabe el
comprobador de una expresión cuyo operando está pendiente o es `Never`:

1. Síntesis inline en `expr_inner` (8570–9190) y `finish_binary_right`/`finish_deferred_binary_right`
   (9547–9840), que según la forma devuelven `Type::I64`/`Bool`, `Type::Tainted` o `Type::Never`.
2. La cola `self.pending: Vec<PendingConstraint>` (variantes `DomainNominal`, `BinaryDomain`, `GuardedComparison`,
   `Projection`, …), resuelta en `resolve_pending_projections` (2498) y en el cierre de `function` (~5560–5680).
3. Conjuntos y banderas laterales por span: `pending_domain_calls`, `failed_join_exprs`, `deferred_exprs`,
   `divergent_exprs`, `conditional_result_types`, `context_rejections`, `failed_constructors` y el booleano
   `tainted`, que `finish_statement_at_site` (5695) convierte en «binding inválido».
4. La exportación a `InferredTypes` (11857–11868), que reinterpreta `Type::Tainted` mediante
   `conditional_result_types`.

El resultado nominal de una operación con dominio indeciso (`Len(q)<2`, `DV+Len(q)`, `(r==r)==p`) es una
propiedad que el ADR (§4.1.2.3, «Cierre de dominio indeciso») exige conservar; el código la representa como un
`Type::Tainted` (contaminación) más una entrada lateral que algunas rutas crean y otras no. De ahí los tres
síntomas medidos: (i) se omite la obligación del consumidor (b2a/b2b/b2c/b2d/b2e: 9588–9617, 5636–5675, 3695);
(ii) un estado indeciso se trata como contaminación y retira evidencia independiente (b2g, q4w: 8828–8893,
5695–5760); (iii) el tipo del elemento o de la variable pendiente no llega al lowering (B1, B3: 8436–8446,
11857–11868, `lower.rs` 7344). El mecanismo único a plantear es un único estado por ocurrencia («resultado nominal
condicionado a un dominio pendiente», distinto de `Tainted` y de `Never`) que consumidores, cierre y exportación lean de
la misma estructura. Esto no es un parche local: los tres sitios están en rutas diferentes de la misma tabla de
estados.

Confianza: los síntomas y su delimitación son reproducibles; los números de línea y la lectura causal salen de
leer el código y de variantes de programas, sin instrumentación; los puntos señalados como INCONCLUSO (razón de
la diferencia entre `[DV]` y `[]` en b2g; origen exacto del `internal_error` de B3; adjudicación de los `perm_diff`)
siguen sin cerrar.

## 12. Lo que no se pudo medir

- Frecuencia de B1 (22 de ~760.000 programas válidos en la ronda 19): no se re-midió; solo los tres repros y
  reducciones propias.
- `twin` a escala completa (300.000 programas por perfil, con las cifras 19/8 y 10/3 de `Pending.md`): solo
  se corrieron 20.000 (release, dos semillas) y 8.000 (debug); los `perm_diff` de la semilla de I10a no están en el
  manifiesto de adjudicaciones y no se adjudicaron.
- Población de mutación (1.174 sustituciones, 309 supervivientes registrados): no se ejecutó; solo M00017 (análogo)
  y M00073.
- Pila a 204 KiB: el control del repositorio impone un mínimo de 256 KiB fuera de pruebas unitarias; se midió a 256 y
  512 KiB y con la pila por defecto.
- Criterio de parada de 1.000.000 de programas en release y debug, generador congelado, y la revisión independiente:
  fuera del alcance de un ciclo 0.
- Instrumentación de la causa raíz de B1, B3 y de la diferencia `[DV]`/`[]` en b2g: no se modificó el código; las
  causas son lecturas del código con variantes de programa.
- `check` frente a `run` solo se comparó por CLI y por biblioteca para los repros de este documento; NB-I11a-08
  (cobertura de `explain`/`Print`) no se midió.
- Tiempos de `validate.sh` son con carga concurrente (campañas y mutaciones en la misma máquina).
