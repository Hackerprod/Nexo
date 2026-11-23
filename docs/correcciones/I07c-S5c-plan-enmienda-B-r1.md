# Validación B de la enmienda r1 al plan S5c (hoja `else` que es variable pendiente): **FAIL**

Enmienda `docs/correcciones/I07c-S5c-plan-enmienda-r1.md` (origen `origin/i07c-estructural-r2`), hallazgo de C y parche local (`Nexo-i07c-c`, solo lectura: leí `second_leaf_anchor` y ejecuté sus binarios `lit-release.exe` y `b1-release.exe`). Comparé con `base-release.exe` (main), `cand-release.exe` (V6) y el oráculo de E (`--source`, `derive`). Perfil `f0-0002-b-v2`, `check`. Sin procesos; copias borradas.

## 1. R4, P15 y P16 (reproducidos; derivación mía por bytes antes de comparar)
Derivé del ADR (l.353, l.301): R4 solo E1303 en `1` `[94,95)` `A³`/`I64` rel `q` `[76,77)` sin E0103; P15 solo E1303 en `aar` `[91,94)` `A¹`/`A²` sin E0103; P16 además E0102 en `true` `[107,111)` `I64`/`Bool`. Resultado ejecutado:

| | oráculo | main | V6 | lit (letra del plan) | b1 (desviación) |
|---|---|---|---|---|---|
| R4 | E1303 `[94,95)` | E0103 `[46,48)` + E1303 | solo E0103 | E0103 + E1303 | solo E1303 |
| P15 | E1303 `[91,94)` | E0103 `[52,54)` + E1303 | ídem | ídem | solo E1303 |
| P16 | E1303 + E0102 `[107,111)` | solo E1303 | ídem | ídem | las dos |

Coincide con la derivación manual, con el oráculo y con lo que afirma A. La causa que identifica C es correcta: `second` es `None` para una variable pendiente y la letra del plan no la contrasta.

## 2. Matriz de 45 filas
Ejecuté las 45 fuentes contra el oráculo (código y primario): main 22, lit 37, b1 38. La única fila donde difieren lit y b1 es R4 (lit E0103 + E1303; b1 solo E1303): confirma «solo cambia R4».

## 3. Contraejemplos (búsqueda sobre 4.940 programas: 512 con hoja `else` pendiente de profundidad 2, y 4.428 con anidados en primera posición, `flat3`, `nest2`, asignaciones previas y posteriores, hojas `q, r, 1, ar, aar, r3, r[0], [q,r]`)
- En 512 programas con la forma de la enmienda: b1 = oráculo en 506 (lit 427, main 404); 79 difieren entre lit y b1 y los 79 los acierta b1; 0 regresiones; los 6 restantes son un defecto previo idéntico en lit y main (E0103 con primario en un nombre, `[118,119)`, con `r[0]`/`r`).
- En los 4.428 programas: b1 = oráculo 3.965, lit 3.683, main 2.628; **12 regresiones de b1 frente a lit** (lit acierta y es igual a main y al oráculo; b1 no). Todas con la misma forma: unión anidada en PRIMERA posición con hojas `r`/`r[0]` (D-REF) y hoja `else` externa `q` pendiente.

Repro mínima de las regresiones (prefijo `fn Main(){let p=true;let ar=[1];var q=[];var r=[];`):

| Programa | oráculo = main = lit | b1 |
|---|---|---|
| N1 `let a0=(if p then (if p then r[0] else r) else q);` | solo E0103 `[38,40)` | E0103 `[38,40)`, E0103 `[47,49)` y E0103 `[89,90)` (primario en el nombre `q`) |
| N2 N1 + `q=[[ar]];` | solo E1303 `[89,90)` `A³`/`A⁴` | E0103 `[47,49)` y E0103 `[89,90)`; pierde la E1303 |
| N3 `(if p then (if p then r else r[0]) else q);q=[[ar]];` | solo E1303 `[86,90)` `A³`/`A²` | solo E0103 `[47,49)`; pierde la E1303 |
| N4 N1 + `r=[true];` | solo E0103 `[38,40)` | añade E1303 `[89,90)` `Bool`/`Array<Bool>` |

Derivación a mano de N2: `q=[[ar]]` fija γ=`A²` en `q:A<γ>`; la unión interior tiene referencia `r[0]` (α) y la hoja `r` (`A<α>`) es recursiva e incompatible (D-REF): E1303 en `r`; su testigo parcial α se iguala con `q` (hoja externa compatible): α=`A³`, y la E1303 queda con esperado `A³` y encontrado `A<α>`=`A⁴`. Es la salida de main y del oráculo; b1 la sustituye por E0103 espurios.

## 4. Hallazgo adicional (no causado por la enmienda, pero bloquea B1): ALT-2 también regresiona programas que main acierta
En el mismo barrido hay 15 programas (lit y b1 iguales entre sí) donde main = oráculo y la letra del plan ya falla, con la unión anidada en primera posición y hoja externa pendiente:
- `(if p then (if p then r else 1) else q);q=[[ar]];` → oráculo y main: solo E1303 en `1` `[115,116)`; lit y b1: E0103 `[76,78)` y E1303 en la hoja `q` `[123,124)`. Derivación: la unión interior tiene referencia `r` (`A<α>`) y `1` es incompatible (E1303 en `1`, al cierre `A³`/`I64`); `q` es compatible con el testigo parcial, así que no hay E0103 ni denuncia sobre `q`.
- `(if p then (if p then [q,r] else ar) else q)`, sin asignaciones → oráculo y main: E0103 en `[]` de `q` y de `r`; lit y b1: una E1303 sobre `ar`.
Son familia del plan (hojas `q`, `r`, `1`, `ar`, `[q,r]`, anidado en primera posición) y activan la PARADA de §9 («diferencial deja una DEFECTO»). Hay que arreglarlo antes de D, y no es seguro que una corrección local de `second_leaf_anchor` baste: los dos hallazgos comparten la unión anidada en primera posición, cuyo ancla exterior es el testigo parcial de la interior.

## 5. Texto enmendado de §3.4
La regla es única y clara (`found = second.or(Deferred(v))` solo si la clave está registrada; con clave ausente `found = second`; la guarda `anchor_error.is_none()` y el `Pending` ignorado solo con `Absent`), y el código lo implementa así (`second_leaf_anchor`, l.14013–14034 del parche). Pero no es correcta en todos los casos: la unión externa de una anidada en primera posición también está «registrada» (su origen es el span del `if` anidado) y su ancla es un testigo parcial; contrastar ahí la hoja pendiente produce las regresiones del §3. La enmienda debe delimitar con un criterio verificable cuándo se contrasta la hoja pendiente (p. ej. solo si el ancla procede de una continuación con tipo propio y no de un testigo parcial de una unión fallida) y probarlo con N1 a N4 y con los programas del §4. Hasta entonces el texto no es «correcto, único y sin decisión abierta».
- Ciclo o segunda denuncia: no observé ciclos ni dobles denuncias en los 512 programas de la forma de la enmienda (b1 los acierta casi todos); la terminación y la independencia de orden que argumenta A (una `unify` por hoja, `registered` monótono, occurs-check) son razonables; las regresiones son de resultado, no de terminación.
- El aviso de `if_finish_join` (prefiere la variable) frente a `first_for_anchor` (prefiere el tipo): los programas con asignación previa que fijan la variable antes de la unión (hoja con tipo y variable a la vez) están en el barrido (variantes `q=[[ar]]`, `r=[ar];q=[[ar]]` antes de la unión) y no producen una regresión de b1 distinta de la clase del §3; sin otro contraejemplo, D debe seguir informando de lo que encuentre.

## 6. Pila
El parche no añade campo a `IfProgress` ni código a `drive`/`resume_non_match`; `second_leaf_anchor` ya era `#[inline(never)]`. No medí la pila (no compilé): INCONCLUSO por medición propia; se mantiene la condición del plan §4 en cada commit.

## 7. Alternativas descartadas y condiciones nuevas
- Contrastar siempre, también con clave ausente (rompe la identidad con main en `match`): descartada con argumento, no con evidencia ejecutable (por CLI `match` no se acepta); se registra en `Pending.md`. Aceptable.
- Reconstruir `Deferred` en `BranchAction::Done` y no suprimir el `join` local: razonadas; la segunda está respaldada por X_join_local. Aceptable.
- Condiciones nuevas (R4, P15, P16 con positivos P15pos, P16pos y R4pos; B1-M9 y B1-M10; `Pending.md`): bien formuladas. Añadir N1 a N4 y los programas del §4 como negativos o guardas (los de §4 son guardas de main que ALT-2 pierde) y un mutante que ejerza «clave registrada pero ancla de testigo parcial».
- `generated1681`, P4, P4pos y R5: A los remite correctamente a D. Aviso: la pérdida de la E1303 en P4, P4pos y R5 es del tipo de los hallazgos del §4 (ALT-2 con anidado en primera posición), de modo que conviene derivarlas antes de D, no después.

## Veredicto: FAIL
1. La desviación propuesta (`found = second.or(Deferred(v))` con la unión registrada) corrige R4, P15 y P16 y no cambia ninguna otra fila de la matriz, pero produce 12 regresiones frente a la letra del plan, a main y al oráculo en uniones anidadas en primera posición con `r[0]`/`r` y hoja externa pendiente (N1–N4, reproducibles con `lit-release.exe` y `b1-release.exe`).
2. ALT-2 (letra y desviación) regresiona además 15 programas de la familia que main acierta (§4: `(if p then (if p then r else 1) else q);q=[[ar]];` y `(if p then (if p then [q,r] else ar) else q)`): activa la PARADA de §9 y exige volver a A/B con la causa común (unión anidada en primera posición con ancla de testigo parcial) antes de que D reciba B1.
3. Delimitar en el texto de §3.4 el criterio de contraste de la hoja pendiente y probarlo con N1–N4 y §4.

Condiciones para el siguiente intento: C/A rederivan con el oráculo la familia completa de anidados en primera posición, el diferencial no debe producir ninguna fila donde main = oráculo y B1 no, y R4, P15 y P16 deben seguir verdes.
