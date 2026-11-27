# C1+C2: contradicción normativa del programa de 78 bytes (continuación 28)

Estado: **bloqueo ADR reproducible; sin PASS y sin v13**. Esta nota adjudica el
programa que detuvo la continuación 27. No modifica `docs/baseline/`, el ADR,
el oráculo ni las baterías congeladas.

## Fuente y derivación independiente

Fuente ASCII sin LF final, 78 bytes, SHA-256
`cf0279fa3fc51b9498120c6c2c4d2f2f1f23050b5d5a1089ff7e3fc8a5a0d69f`:

```nexo
fn Main(){var v=(if true then {return;} else {return;});var a=[];a=[v];v=[a];}
```

La misma fuente sin `v=[a];` mide 72 bytes y tiene SHA-256
`a6ef62d3f65e88940050c84ebc16e4f7624822d858a0334bacd3a2a0e843c016`.
Su salida en `main` y en la referencia es exactamente dos `NX-E0103`, en
`[62,64)` y `[67,70)`. El oráculo independiente `i07c_rd_oracle.derive`
deriva esos mismos dos C tanto para el prefijo como para la fuente completa.

Derivación manual según F0-0002 §§4.1.1, 4.1.2.1–2, 4.1.2.5 y 13.9:

1. Las dos ramas de `if` retornan. `v` recibe una variable monomórfica `β`,
   sin tipo fijado por ese inicializador divergente.
2. `[]` establece `a: Array<α>` y crea su deber propio de resolver el
   elemento en `[62,64)`. `[v]` tiene tipo `Array<β>` y crea otro deber propio
   en `[67,70)`. La igualdad compatible `a=[v]` publica `α=β`; ninguno de los
   dos constructores tiene aún un elemento finito determinado.
3. `v=[a]` intenta `β=Array<Array<β>>`. No tiene solución finita. La igualdad
   compuesta se rechaza atómicamente en su ocurrencia y, por §4.1.2.2, **no
   desconecta ni sustituye** la igualdad previa `α=β`.
4. Al cierre de la función `α` y `β` siguen indeterminadas. §4.1.2.2 exige
   un E0103 por *constructor con obligación propia*, aunque compartan una
   incógnita por unificación. Por tanto los C normativos siguen siendo los
   dos del prefijo: `[62,64)` y `[67,70)`. El constructor final `[a]` consume
   el componente afectado por la igualdad rechazada; no reemplaza esos C
   anteriores por un C único del ciclo (§§4.1.2.5 y 13.9).

`main` `31d91db` emite en `check` **y** `run` solo `NX-E0103 [73,76)`.
La referencia experimental después del arreglo del gate también lo emite.
Así, ambas salidas omiten los dos C obligatorios. La propiedad de igualdad
no puede «absorber» deberes anteriores con sitios independientes: un
`EqualityId` que rechazó `v=[a]` no es dueño de los constructores `[]` y
`[v]` ya publicados. Emitir una vez por ciclo contradice expresamente
«un diagnóstico por constructor con obligación propia» de §4.1.2.2.

## Imposibilidad dentro de C1+C2

Sea `D` la salida diagnóstica de un mecanismo de C1+C2. Reproducir `main`
exige `D = {E0103 [73,76)}`. Cumplir F0-0002 §4.1.2 N(b,d,h), la cláusula
2 y §13.9 exige que `D` contenga al menos `E0103 [62,64)` y
`E0103 [67,70)`. Son conjuntos incompatibles. Ninguna elección de orden,
journal, `EqualityId`, replay, `ReadContext` ni deduplicación cambia las
restricciones fuente o elimina esos deberes tras rechazar la igualdad final.
El bloqueo requiere decidir por Flujo A si se corrige el esperado de `main`
y la puerta cruzada, o se enmienda explícitamente el ADR con un ejemplo válido
e inválido. No se publica v13 sobre una regla que contradice el ADR vigente.

La puerta de 20 corpus no incluye este literal como fila modelada; que un
binario dé A=0/D=0/A-unmodeled=0 en esas 151.188 filas no resuelve esta
contradicción externa al corpus. `owned_rows_gate.py --commit C1` comprueba
3/74, 0/45, 0/13 y F06, no este literal.

## Iteraciones ejecutadas, sin inferir cierre de la igualdad

Cada barrido siguiente ejecutó los 20 corpus completos (151.188 filas) con
`candidate_gate.py`, bajo `heavy-lock` de la sesión. Las cifras son diferencias
contra `main`, no un juicio de conformidad con el ADR. `H nuevas` fue cero en
todos; se conservaron dos H históricas.

| Iteración | Cambio de mecanismo probado | A | D | A-unmodeled | SHA-256 del ejecutable release | Resultado |
|---|---|---:|---:|---:|---|---|
| 1 | Elegir el último constructor con testigo del camino recursivo | no medido→0 | no medido→8 | no medido→8 | `bdbfb9b12a9a1c8a24f80fdd8c2983721808aa01d33f459b96812174f6a4e0d6` | 78 B coincide con `main`; gate rojo |
| 2 | Conservar `found` en el `Join` activo bajo `if` | 0→0 | 8→0 | 8→3 | `4881db70a5560d5e782027f77b0dfea573f251c0f87c363b56e804ef0734aae6` | `dirigido5:2069,4425,5287` |
| 3 | Usar origen previo solo con un constructor testigo del camino | 0→0 | 0→0 | 3→0 | `3009e11f92d74811acbba7e49242467b28be87301935ae6d108e38e02535424c` | gate de corpus verde; `validate.sh` rojo |
| 4 | Excluir del recuento los constructores de la igualdad actual | 0→0 | 0→0 | 0→0 | `2c4612ffaa8daa1f10a365ec1d6ca27ac15bdc5d884aa322730447e6cc1a9831` | gate de corpus verde; `validate.sh` rojo |

Las iteraciones 1 y 2 eran árboles de trabajo sin commit. La única tabla
`commit → gate completo → filas propias` respaldada por ejecución es:

| Commit experimental | Gate completo | `owned_rows_gate.py --commit C1` | Puerta mecánica |
|---|---|---|---|
| `4688cdbe275a15f1640f714260d434ae85a6956c` (iteración 3) | 20/20, 151.188, A/D/A-unmodeled/H nuevas 0/0/0/0; informe SHA-256 `020eade008b5b8015b95c632eee5ef8cffbe0fd56c17f96df6f9077e28dea88e` | 3/74, 0/45, 0/13; F06=`main` | `validate.sh` falló: `deferred_join_has_one_diagnostic_per_source_site`, esperado `[104,107)`, obtenido `[110,113)` |
| `1e4457fec7c02dc426afa74b25ae4660521c73a8` (iteración 4) | 20/20, 151.188, A/D/A-unmodeled/H nuevas 0/0/0/0; informe SHA-256 `c29d9c7a20cd129cf903b63c886fe10297954a0bb4569767d8b1c02fe07bdbe6` | 3/74, 0/45, 0/13; F06=`main` | `validate.sh` falló: pánico en `check_inferred_depth` |

En la segunda puerta limpia, la prueba
`i07c_normative_survivors::deferred_projection_and_constructor_programs_finish_with_causal_results`
reprodujo `src/check/types.rs:7503:37: no entry found for key` con
`fn Main(){var q1=(if true then {return;} else {return;});q1[0]=MockOpen(1);q1=[];}`
(LF final, 83 bytes, SHA-256
`64e10e24c456edc281d5b376ce14343fc912d0e0680864babf5816f3e6046883`).
El journal revierte nodos nuevos de `type_nodes` y conserva monotónico
`next_variable`; `check_inferred_depth` indexa `type_nodes[&id]` para todos
los IDs anteriores al contador. El pánico es un defecto estructural adicional
de la referencia, ajeno a la contradicción normativa de los 78 B. La puerta
se detuvo en debug; release y pila 5/5 no se ejecutaron para esos commits.

Se retiró la referencia experimental por el commit reversible `ac962629d6a9b8b21ad8e7c021e61adafc84dc85`:
su árbol de código coincide con C0 `96d564496b1072d09a4a169c3be801d69aa3af96`.
Los informes de gate siguen identificados arriba; ningún resultado de la
referencia retirada acredita C1. `r9-commit-gates.json`, `owned_rows_gate.py`,
el contrato v12 y el ADR permanecen sin v13. El verificador v12
`verify_r9_contract.py` terminó sin errores, lo que comprueba su estructura,
no resuelve la contradicción.

La retirada `ac96262` se verificó después en clon limpio con
`sh scripts/validate.sh`, bajo `heavy-lock` `i07c-rediseno`,
`CARGO_BUILD_JOBS=4` y target aislado en C:. Formato, Clippy con
`-D warnings`, selección nextest, 2.218/2.218 pruebas debug,
2.220/2.220 release y las sondas TYPES `option_match126` y
`result_match126` a 65 KiB (5/5 cada una) terminaron sin error.
Esto acredita únicamente el regreso al árbol C0; no es una validación de
C1+C2 ni una puerta de v13.
