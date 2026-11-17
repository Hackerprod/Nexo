# I07c — validación B del plan corregido (paso 2.2, ronda 2)

Plan: docs/correcciones/I07c-plan.md (con «Registro de correcciones r2»). Evidencia propia: lectura de types.rs, ejecución del clon instrumentado D:\nexo-work\i07c-a-probe (binario target-i07c-a) y del binario base, recálculo de offsets con Python.

## Veredicto global: FAIL (dos bloqueantes menores, ambos corregibles en el .md)

## Bloqueantes de r1: estado

1. B1/oráculo: CORREGIDO en lo esencial. b1a=ir1 enlazado a §8.3; b1b/b1c con oráculo propio. Ver bloqueante nuevo N2 sobre ir1c/ir1d.
2. Causas INCONCLUSAS: CORREGIDO y verificado. Reproduje las sondas: B3 `pattern local at=30 selected=Array(Deferred(4))`, `export binding at=30 variable=None resolved=Unresolved` mientras la expresión `[]` exporta Concrete(Array(I64)); q4w `bottom_only=true` frente a q4v `bottom_only=false`, ambos `available=false`; b1a `(22,34)` Tainted con `conditional=Some(Bool)`. F1/F2/F3 separadas con coherencia; B3 ya no se atribuye a F1.
3. Estado lateral: PARCIAL, ver N1. Las ocho variantes de PendingConstraint (types.rs:568–578) y los campos citados son correctos y las lecturas 1422–1503 también.
4. Vía (b): CORREGIDO. §9 coincide con F0-CODIGOS.md:61 y ADR §4.1 (NX-E0113 con subcause=null, Array irresuelto NX-E0103 con subcause=null, `cannot_infer_type` inexistente); sin texto de enmienda, impacto RD87/TD/DG listado; sigue NO por defecto.
5. adr_gap_*: CORREGIDO (pausa, Flujo A, sustitución al tocar el corte).
6. Oráculos B2: CORREGIDO. Recalculé los primarios con la expansión literal de DV: b2a [67,113), b2b [66,112), b2c [66,113), b2d [162,163), b2e [139,141), b2g [144,147), q4w [140,141); todos coinciden. Derivación y condiciones de §4.1.2.7 declaradas, `Never` aclarado.
7. Contrato §3.3–§3.7 y base b506d7d: CORREGIDO. 8. Presupuesto por corte, fuzz y mutación ubicados: CORREGIDO. 9. 256 KiB normativos / 204 KiB operativos: CORREGIDO y coherente entre §1, S3 y Parada. 10. F0-0007 (`tests/f0007.rs` existe): CORREGIDO.

No bloqueantes de r1: incorporados (8474, seis funciones de barrido F3, rebase ce84ba8/eb0a323, script de referencias, offsets TD). Verificados.

## Bloqueantes nuevos

N1. El inventario nominal no es exhaustivo. Comparé por script los campos de Checker (types.rs:1422–1503) con el plan: quedan sin nombrar ni clasificar, y no los cubre la frase «mapas de planificación de formas/lecturas/asignación/proyección», los de estado de uniones y ramas: `deferred_join_roots`, `recursive_join_roots`, `branch_join_sites`, `preseeded_children`, `active_if_span`, `eligible_shapes`, `nested_assignment_sites`, además de `closure_local_sites`, `hidden_for`, `generator_slots`, `saved_closures`, `analysis_exhausted`, `quota_error`. Los de uniones (deferred_join_roots se usa en 1974, 3655, 3839) pueden decidir aceptación y son justo el tipo de estado lateral que F1 debe absorber. Añadir una línea por campo restante (autoridad retirada / índice derivado / estructura no semántica) para que la búsqueda estática de D sea completa.
N2. ir1c/ir1d se declaran independientes de §8.3 sin argumento. Medido en la base: `var r=[];r=[r==r];` checked, `var r=[];r=[(r==r)==true];` NX-E0103 [16,18), y con `r=[true]` posterior b1a es internal_error. Ambas tienen la misma autorreferencia hipotética con solución finita Array<Bool>; el plan fija ir1c/ir1d como «Aceptar» pero b1a como indecidida sin explicar la frontera. Hasta decidir §8.3, ir1c/ir1d solo deben afirmar N(f) (como b1a), o el plan debe justificar por qué `==true` cambia la regla.

## Sobre la dependencia de §8.3

Bien planteada: b1b, b1c, B2, B3 y F3 no dependen de ella, S1/S2 pueden avanzar y el cierre de S2 exige la decisión. Salvedad registrable: Flujo A de §8.3 debería lanzarse ya en paralelo, porque el Bloque I y su presupuesto de dos rondas no terminan sin ese oráculo.

## No bloqueante

- Añadir `r=[false]` (b1a con false, también internal_error en la base) al conjunto N(f) de B1.
- Hay más tests adr_gap_* (`adr_gap_entry_gate_is_reserved_for_execution`, `adr_gap_resolved_variable_within_element_keeps_current_union_site`); la instrucción de §8 los cubre por el patrón, pero conviene nombrarlos.
