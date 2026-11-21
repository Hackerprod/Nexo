# I07c — pendiente real de cierre (documento de recuperación)

Creado el 2026-11-15 por el coordinador (Claude). Existe para que este trabajo no se pierda entre contextos.
Estado de verdad: I07c **nunca se cerró como el ADR y el roadmap prometían**. Se cerró localmente, por agotamiento,
con un contrato acotado y deuda registrada. Este documento fija qué falta, qué se hará y cuándo.

## 1. Qué prometía Nexo

- ADR `docs/adr/F0-0002-second-slice.md` §17.2 fila I07c: inferencia coherente de tipos diferidos (§§4.1.1–4.1.2),
  TD01–TD07, DG07/DG08 y todos los casos RD de §14.10; «cierra ambas lagunas de tipado antes de I08». I08 depende de I07c
  «para no construir Result sobre tipos incoherentes».
- Roadmap rev 0.2 §4.1: ningún requisito obligatorio puede quedar pendiente al aprobar una fase; un defecto conocido que
  viola una garantía anunciada bloquea la promoción. F0 sale con «dos revisores sin discrepancias semánticas abiertas».
- Salida legítima del roadmap para inferencia inestable (F1): «exigir firmas públicas y acotar el solucionador». Eso es
  una **enmienda de ADR y decisión del propietario**, nunca un cierre implícito. La propuesta maestra
  (`docs/proposals/compilador-maestro-recarga-r3-1.md`) prohíbe cambiar sintaxis o semántica por conveniencia del compilador.

## 2. Cómo se cerró en realidad

- Contrato de cierre acotado: `docs/validation/F0-0002-I07c-contrato-de-cierre.md` (auditoría 2026-10-28): cuota reducida
  asignada a I10a; gemelos tipado/libre y campañas de cuota reducida declarados **informativos**; mutaciones sobre una
  población cerrada; hash de salidas no versionado.
- Una sesión Codex paralela corrió un bucle de robustez en `C:\projects\Nexo-robustez` (rama `i07c-robustez-loop`,
  commits `ce3f1da`..`a29f66c`, base `b506d7d`, **desfasada** respecto a main). Tras más de 20 ciclos seguían apareciendo
  bloqueantes de la misma familia (rondas 17, 18, 19): restricciones sobre tipos pendientes que no pasan por el
  comprobador único. Registro: `Nexo-robustez\.scratch\bucle-estado.md`; texto de arranque: `.scratch/codex-bucle-robustez.md`.
  A 2026-11-08 el bucle estaba atascado en el «par 5» (igualdad con dominio pendiente, wrapper `Eq14`), con 15 rojos de 59
  pruebas públicas en su rama.
- Después main siguió: I08–I11, F0-0007 (inferencia diferida de constructores Result; corrigió V06/B2 parcialmente), J01–J14.

## 3. Qué falta (a verificar en main antes de actuar; nada de esto está re-medido tras J14)

1. **B1**: `InvalidIr`/`internal_error` en una igualdad `(r==r)` sobre array pendiente que se resuelve después
   (22 de ~760.000 programas válidos en la ronda 19). Repros b1a/b1b/b1c en `Nexo-robustez\.scratch\insumos\val19-tipos-repros`.
2. **B2**: se aceptan programas que el ADR manda rechazar (§4.1.2.3, N(a)): resultado nominal perdido con `var q=DV` y
   operando `Never` delante de `Len(q)` o `(q==q)` (b2a–b2e); restricción perdida b2g (`var r=[DV]; … let c=a==r; r=aar;`
   debe dar NX-E0102).
3. **Campaña `twin` de tipos diferidos**: 19 casos señalados y 8 `other_differences` en release (10 y 3 en debug) sin adjudicar
   (`Pending.md`).
4. **Mutación**: 309 supervivientes en la población cerrada (305 informativos, 4 equivalencias); el criterio del bucle exigía
   0 supervivientes normativos y ≥1.000.000 de programas en release y debug con generador congelado.
5. **Revisión independiente**: nunca dio PASS (ronda 19: FAIL en tipos y en documentos).
6. **Lagunas del ADR**: `[DV][i]` frente a `DV[i]` (tipo de la proyección de `Never`); unión exterior con `[]` primero (§4.1.2).
7. **Deuda asociada**: `ck1` (`check` da NX-E0105 en escritura a elemento mientras `run` no), escala de proyecciones
   diferidas en `src/check/types.rs`, tres bloqueantes menores de documentos (etiqueta `ck1`, fixtures duplicados,
   texto sobre autocrlf), mutación sistemática de `explain`/`Print`/superficies I07c–I10b (NB-I11a-08).

## 4. Plan (Flujo B de AGENTS.md; reglas del propietario)

- **Cuándo.** Ciclo 0 (solo medición, sin tocar `types.rs`) puede correr ya. La implementación estructural arranca
  **tras cerrar J16** y **antes de F0-0005 y F0-0004**, porque esos cortes añaden tipos sobre este comprobador y
  J14 ya dejó el cargo diferido de profundidad dentro de `types.rs`.
- **Dónde.** Rama nueva desde main (no reutilizar `i07c-robustez-loop`), p. ej. `i07c-estructural`, worktree propio
  (`C:\projects\Nexo-i07c`, artefactos en `D:\nexo-work`). Sin push a main hasta PASS de D'.
- **Ciclo 0.** Medir en main: B1, B2, `twin`, mutantes M00017/M00073 y los repros de insumos; informe con qué sigue abierto.
- **Plan A→B** con dos vías: (a) estructural, con causa raíz probada (archivo:función:línea) y mecanismo único para
  todas las restricciones sobre tipos pendientes; (b) acotación, con texto exacto de la enmienda de ADR y ejemplos.
  Si hace falta (b), **detenerse y consultar al propietario**; no aplicarla por defecto.
- **Implementación y validación**: C, D, D' como en J14; una sola pasada de mutación al final; tope de 2 rondas de validación
  por corte y, si no converge, reducir alcance o escalar al propietario (no cerrar por agotamiento).
- **Criterio de parada objetivo** (heredado del bucle): bloquea solo pánico/InvalidIr/internal_error, aceptar lo que debe
  rechazarse o rechazar lo válido, primer error distinto del gemelo concreto en el corpus enumerado, y mutante normativo
  superviviente; generador y semillas congelados antes de la verificación final; ≥1.000.000 de programas en release y debug.
- **Riesgos**: conflicto en `types.rs` con otros carriles; regresión sobre J01–J14 (comprobar RD 152/152, ticks, T01–T14,
  diferencial contra main); coste de campañas grandes y cuota compartida de Codex; repetir el atasco de 20+ ciclos.

## 5. Fuentes

`docs/validation/F0-0002-I07c-contrato-de-cierre.md`, `docs/validation/F0-0002-I07c-estado.md`, `Pending.md`
(secciones I07c y NB-I11a-08), `docs/correcciones/` (estilo de planes), `Nexo-robustez\.scratch\bucle-estado.md` e
`insumos\` (val19-common.md, val19-tipos-informe.md, val19-docs-informe.md, val19-cuota-informe.md).

## 6. Hallazgo añadido el 2026-11-16 (descubierto en J16, no pertenece a J16)

`fn Main(){match Some([]){Some(v) if false=>{let a:Array<I64>=v;},Some(w)=>{let b:Array<I64>=w;},None=>{}}}` da `driver_error`/`internal_error` en `check` tanto en main `99c8491` como tras J16 T0 (no es regresión de J16). Es del dominio de I07c (array pendiente `Some([])` fijado por dos ramas de `match`, una con guarda): un error interno ante un programa que el ADR manda decidir. Incluirlo como repro B3 en el ciclo 0 de I07c.

## 7. Requisito añadido el 2026-11-17 (viene de J16 Bloque 1)

El solver de inferencia rediseñado NO debe recurrir por profundidad de tipo: `Checker::bind`, `unify`, `selection_type`, `resolved_syntax_type` y `concrete_with_depth` son recursivos hoy y desbordan a 204 KiB en debug con `Some(`×84..126, `Option<…>`×126 con `match`, y Array inválido/`None[[…]]` (193/257/449 KiB). Las filas `J16-S1C-SOME-TYPES`, `J16-S1C-ARRAY-HELPER-MARGEN` y `J16-S1C-MATCH-TYPE-DEPTH` de `Pending.md` son el criterio de aceptación (pila ≤ 204 KiB en debug, contrato F0-0003 §13.2 de 256 KiB), con los repros de `.scratch/c-j16b1-desvios4.md`, `c-j16b1-desvios8.md` y `c-j16b1-cierre-integral.md`. Mientras tanto J16 aplaza S1b y S1c.

## 8. Estado tras integrar el Bloque II (2026-11-20, coordinador)

- **Bloque II (S1 grafo único de obligaciones, S2 patrón constructor sobre raíz abierta con subcausa `constructor_shape`, S3 solver iterativo sin recursión por profundidad de tipo) INTEGRADO en `main`** por avance rápido a `0324a72` (árbol `3edcabc2`), con PASS de D (D1) y de D' sobre `0324a72`. Un primer D' sobre `6c59a15` dio FAIL acotado por el mutante M00436 (con programa distinguidor); se cerró con la regresión del lote 6 (10 mutantes antes «sin programa» ya con prueba). La producción de S3 no cambia desde `11a70ec`.
- Desbloquea los bloques de tipado de F0-0005 (B1b/B2b/B3b) y, después, F0-0004.
- Mutación acotada: 739 clasificados = 387 detectados + 135 por regresiones nuevas + 10 por no terminación medida + 97 no compilan + 15 equivalentes con argumento + 95 sin programa tras la caza (no se afirma equivalencia). D' confirmó por muestreo (25/25 regresiones, 8 «sin programa» sin hallazgo, 4 argumentos de equivalencia), no el 100 %.
- Abierto (registrado como `I07C-*` en `Pending.md`): campaña extendida `I07C-MUT-EXT` (pausada; la reanuda la sesión I07c en segundo plano), reducción de alcance del «solver único» por cursores `I07C-Q-CURSORS`, 95 `I07C-MUT-SIN-PROGRAMA` (si una caza posterior halla programa, es un defecto con regresión), y la limpieza no bloqueante de `RESULTADO.md` (SHA de `clasificacion.json`) y de los SHA de blob de los lotes 1-3.
- Los cuelgues espurios de `nexo.exe` en la salida del proceso (destructor del `thread_local` `PARSE_WORKER`) quedaron corregidos en `main` (`9be9f76`).

## 9. Decisión del propietario sobre I07C-Q-CURSORS (2026-11-20)

- El propietario decidió que el criterio es **lo más robusto posible, sin atajos**: se vuelve a lo que prometía el roadmap. El estado transitorio `tainted` y `numeric_domain_owner` (hoy cursores de visita `VisitCursor` en `src/check/types.rs`, ~131 apariciones) **vuelve al grafo único** de obligaciones como hecho con vida útil explícita por ocurrencia/obligación, sin cursores ni campos permanentes equivalentes en `Checker`. Se conserva la propiedad que daban los cursores: ningún dato transitorio puede filtrarse de una visita a otra.
- Motivos registrados: (1) en cinco sitios (`require_union`, `union_inner`) no existe programa fuente que distinga heredar el cursor de crearlo vacío (I07C-CURSOR-SIN-CLI): corrección no verificable desde fuera, que F0-0004/F0-0005 podrían volver observable; (2) garantía por construcción y no por disciplina; (3) es el momento más barato, antes de que F0-0005 B2b/B3b y F0-0004 añadan más enganches a `types.rs`.
- Se tramita como **I07c-S4** por Flujo B (plan A → validación B → implementación C → D → D'), con puertas: cero cursores y cero campos equivalentes (grep), RD71/TD/DG y fixtures protegidos sin cambios, RD 152/152, diferencial contra `main` sin diferencias, medición de pila 204/256 KiB de las ocho filas `J16-S1C-*` y de S3 sin empeorar, mutación de las regiones tocadas con población nueva congelada por SHA. Orden: F0-0005 B1b (L01b/L02b publicados, L03b en curso) se valida e integra primero; S4 va a continuación y ANTES de B2b/B3b (un solo escritor en `types.rs`). El plan de S4 se escribe en paralelo (solo documentación).
- Cuando S4 integre, `I07C-Q-CURSORS` se cierra (la reducción de alcance deja de existir) y I07c puede declararse cerrado sin esa reserva; siguen abiertos aparte los 95 mutantes sin programa, la campaña extendida, la campaña `twin` y las lagunas del ADR.
