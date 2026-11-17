# I07c S1 — hallazgo de sonda; retorno a A/B

Estado: **S1 no implementada**. Base `c45e935` (`e6e5f82` en código). La sonda se hizo en la copia desechable `D:\nexo-work\i07c-c-probe`, con `CARGO_TARGET_DIR=D:/nexo-work/target-i07c-c`, `TEMP/TMP=D:\nexo-work\tmp` y compilación `--release --locked --offline`. Ninguna modificación de producción de esa copia se trasladó al árbol de trabajo.

## Contradicción de familia/oráculo en b2d

El plan §2.1 agrupa b2d con las pérdidas de resultado nominal F1 y exige un único `NX-E0102/type_mismatch` sobre el `r` de `[a0,r]`, `[162,163)`. Fuente exacta:

```nexo
fn Main(){var r=(if true then {return;} else {return;});let a0=(if true then {return;} else {return;})+((if true then {return;} else {return;})-Len(r));let b=[a0,r];}
```

La sonda instrumentó `finish_deferred_binary_right`, `pending_nominal_available`, el cierre de `GuardedComparison` y `bind`. Al conservar el resultado nominal I64 de `Len(r)`, de `-` y de `+` en la copia, el checker ejecuta `bind id=0 ty=I64 origin=[159,161) found=[162,163)` al comprobar `[a0,r]`. El `r` de la homogeneidad queda fijado a I64; el predicado `domain:array` pendiente sobre el `r` de `Len(r)` queda refutado. Salida obtenida de `check --profile f0-0003-b-v1 --format json`: `NX-E0102/not_indexable`, primario y `found_type` `[148,149)` (`r` en `Len(r)`), `data={"expected_type":"domain:array","found_type":"I64"}`. `run` no se utilizó como oráculo para esta sonda.

Control sin envoltorios `Never`, ejecutado con el binario release **sin instrumentar** de `e6e5f82`:

```nexo
fn Main(){var r=(if true then {return;} else {return;});let a0=Len(r);let b=[a0,r];}
```

La base ya rechaza con `NX-E0102/not_indexable` sobre el `r` de `Len(r)` `[67,68)`, `expected_type=domain:array`, `found_type=I64`. Añadir `r=[1];` después conserva ese C y añade otro `NX-E0102/type_mismatch` en el RHS posterior `[85,88)`. La fuente b2d original, en la base sin cambios, se acepta indebidamente (`checked`), como documenta el ciclo 0.

Según ADR F0-0002 §§4.1.1–4.1.2.3, `Len` tiene resultado nominal I64, su dominio Array no fija ni introduce un componente de `r`, y la homogeneidad de `[a0,r]` añade igualdad. La igualdad compatible fija `r=I64`; §4.1.2.3 sitúa el diagnóstico de un dominio diferido refutado en su operando original. Los `DV` de b2d no añaden restricciones concretas. Así, recuperar el nominal expone un **conflicto de dominio posterior a una igualdad compatible**, que no es el conflicto de tipos en el último `r` exigido por el oráculo b2d del plan. Obtener ese último C requeriría rechazar la igualdad antes de fijar `r`, alterar la prioridad causal del dominio o definir otra regla; ninguna está especificada en S1. No elijo una de esas opciones ni cambio el oráculo.

## Alcance de la sonda y estado del árbol

Las trazas de b2a/b2b/b2c confirmaron la pérdida de contexto del consumidor: b2a pasa `expected=Bool` a `+` sin registrar su comparación; b2b/b2c crean `GuardedComparison` con `consumer_expected=I64`, pero el cierre exige ambos lados resueltos y omite ese consumidor cuando el izquierdo es `Never`. b2g/q4w reproducen `pending_nominal_available=false` y `finish_statement_at_site tainted=true`, como F1 del plan. B1/B3 siguen reservados a S2. Esos hallazgos **no** corrigen la contradicción específica de b2d.

El prototipo local en la copia rechazó b2a/b2b/b2c/b2e/b2g/q4w con los primarios del plan y pasó 60/60 pruebas release de `tests/i07c_causal_obligations.rs` tras ajustar guardas; también mostró el C diferente de b2d. Un cambio previo que ofrecía el nominal sin guardas falló dos de esas 60 pruebas, evidencia de que no procede trasplantar un parche local. La sonda no es una implementación ni una puerta mecánica.

El árbol de trabajo conserva `src/` y `tests/` idénticos a HEAD. Solo este informe pertenece a C; los documentos no seguidos de la laguna §8.3 proceden de otro trabajo concurrente y se dejaron intactos. **A/B deben resolver el oráculo y la familia de b2d antes de reanudar S1.** Después siguen pendientes la implementación del grafo, todas las regresiones y mutaciones inversas, y la puerta mecánica completa.
