# Brief de RE-VALIDACIÓN con Codex de los cortes implementados por Sonnet (desvío por presupuesto)

Destinatario: Codex `gpt-6-sol` (nunca Astra), en el repositorio privado Nexo, en un worktree propio y limpio desde `origin/main`
(`git worktree add -b i07c-revalidacion <ruta> origin/main`). No modificar `docs/baseline/`, no push a `main`, sin secretos,
no tocar `C:\Projects\Nexo` ni `C:\projects\Nexo-robustez`, nunca parar procesos por nombre de imagen (solo PID propios),
trabajo pesado bajo `python -B scripts/heavy-lock.py run --session i07c-revalidacion -- ...`, `CARGO_TARGET_DIR` propio en `C:`
(≤ 10 GB, deja ≥ 10 GB libres), toolchain 1.99.1. Publicar solo ramas limpias.

## 1. Por qué existe esta re-validación
Entre 2026-11-22 y 2026-11-23, sin crédito de API ni de Codex, tres cortes se implementaron y validaron solo con agentes
Sonnet 5.5 (C implementa; D y D' validan), por **desvío por presupuesto** (nota en `docs/F0_ACUERDOS.md` §10). Entraron en `main`
con puerta completa GREEN y PASS de D y D'; la condición era **re-validarlos con Codex tras 2026-11-24 04:43** y que la
mutación de las regiones tocadas fuese obligatoria. Esta tarea cierra esa condición. No es un rediseño: es una auditoría
independiente con evidencia reproducible. Un defecto de núcleo reproducible con programa bloquea siempre (AGENTS.md,
«Rigor proporcional»); una duda sin programa se registra, no bloquea.

## 2. Cortes y diffs (todos ya en `main`)
| Corte | Qué hace | Diff de producción | Informes |
|---|---|---|---|
| **S6-COR-2** (S0..S10) | primarios, ticks y guardas bajo agrupaciones; lowering | `git diff eeb3c9c 9dbcc99^2 -- src/` (producción solo en `src/ir/lower.rs`, `src/ir/lower_match.rs`) | `I07c-S6COR2-plan.md`, `I07c-C-S6COR2-informe.md`, `I07c-D-S6COR2-r1/r2.md`, `I07c-Dprima-S6COR2-r1/r2.md` |
| **F5-L04b** | CFG de `while true` literal en `f0-0005-b-v1`, barrera pública NX-E0006/L08 | `git diff 784505b^ d676aef -- src/` | `docs/correcciones/F5-r1.md`, Pending fila `F5-WHILE-TRUE-RETURN` |
| **S6COR-GUARDA-CLOSURE** | closure con efecto llamada en guarda de `match`: NX-E0414/E0415 + NX-E1305 mismo primario | `git diff c76fa87 0b07fba -- src/` (`src/ir/effects.rs`) | `I07c-S6COR-GUARDA-CLOSURE-plan.md`, `I07c-C-…-informe.md`, `I07c-D-…-r1.md`, `I07c-Dprima-…-r1.md` |

Referencia normativa: `docs/adr/F0-0002-second-slice.md`, `F0-0002-lagunas.md` (fila de guarda de `match` con closure con efecto),
`F0-0003`, `F0-0005` (§2.2 l.116-123 para F5-L04b), `docs/validation/F0-0002-I07c-defectos-abiertos.md`.

## 3. Qué mirar (por corte)
1. **Diff de producción línea a línea** contra el plan validado y el ADR: ¿hay casos especiales por patrón de fuente, constantes
   hechas para pasar un test, rutas duplicadas o código muerto? Archivos fuera del plan.
2. **Diferenciales independientes** (no reutilizar las baterías de C/D): base = `git archive` del commit anterior al corte;
   candidato = `main`. `check` y `run`, debug y release, perfiles `f0-0002-b-v2`, `f0-0003-b-v1`, `f0-0005-b-v1`. Corpus propio
   generado con semilla y SHA-256 congelados antes de ejecutar (≥ 5.000 programas por corte, incluyendo: agrupaciones anidadas,
   ticks/`move`, guardas con closures con y sin efecto —Print, MockUnknown, suspend, `no_io`/`no_suspend`—, closures por nombre,
   literal e `if`, `while true` con/sin `break`/`return`, `for` sobre arrays). Cada diferencia se clasifica contra el ADR (mejora
   derivada / defecto), con orden estricto de diagnósticos (§13.9). `corpus/`, `tests/fixtures/` y los 152 RD sin cambios salvo lo
   que el corte declara.
3. **Mutación de las regiones tocadas** (obligatoria): población por `git blame` de cada corte; reglas: muerto = primer test que
   falla con `--profile default`; superviviente = suite completa debug y release verde; caza dirigida con programa antes de
   declararlo deuda; si se exhibe, regresión que falle con el mutante y pase sin él (SHA-256 inicial/mutado/restaurado).
4. **Mutantes con deuda ya registrada** (intentar exhibirlos con programa o matarlos con un test; si no, argumento razonado):
   - S6-COR-2: **M00013, M00021, M00023, M00051, M00054** (sin programa de D ni D'); **M00047, M00048** (`for` inalcanzable, NX-E0006 antes del lowering).
   - S6COR-GUARDA-CLOSURE: **R000, R006, R015, R016** (sin programa; ejecutados con suite completa en debug y release).
   - F5-L04b: **n10** (C aporta argumento de equivalencia; D2 lo juzgó sólido: re-juzgarlo).
   Los ids M000NN de S6-COR-2 y su programa/región figuran en la tabla de `docs/correcciones/I07c-C-S6COR2-informe.md` (la población se regenera por `git blame` de la región; los ids no son estables entre poblaciones: identificar cada mutante por archivo, línea y operador de la tabla); los R de `docs/correcciones/I07c-C-S6COR-GUARDA-CLOSURE-informe.md`.
5. **Campaña de regiones de S6-COR y S5b.1** (en curso por Sonnet, `docs/validation/i07c-mut-regiones/`): lo que quede sin ejecutar
   (800 mutantes congelados; semilla 20261123) se completa aquí si no está terminado; revisar la tabla de supervivientes sin programa.
6. **Pila F5:** `python -B scripts/f5-frame-sizes.py` y `scripts/f5-stack65.py`; línea base `drive` 27.536 B, pareja 60.616 B; margen ≥ +1.000 B sobre 66.560 B.
7. **Fuzz sin pánicos** (`fuzz 4000`, semilla 7391045882; más 3.000 fuentes del script `scripts/i07c_guarda_closure_diff.py --fuzz`) y
   `check`/`run` y debug/release coincidentes; auditor de spans (`scripts/i07c_span_audit.py`, clave de línea de `REVIEWED_EFFECTS`).
8. **Tests existentes cambiados por los cortes:** las 12 filas `head_*_guard` (reubicadas en `GUARD_CONTRACT_CASES`), la clave del
   auditor 1223→1232, las filas de S6-COR-2 del plan. Verificar que no se rebajó ninguna aserción.

## 4. Formato del informe (`docs/correcciones/I07c-Codex-revalidacion-informe.md`)
Por corte: veredicto **PASA / FALLA / INCONCLUSO** con evidencia; tabla de hallazgos numerados, cada uno con programa fuente,
perfil, comando, salida esperada (cita del ADR) y obtenida; diferencial (recuentos, semilla, SHA-256 del corpus, tabla de
diferencias clasificadas, 0 sin clasificar); mutación (población, SHA-256, muertos/no compilan/supervivientes, test que mata cada
muerto, programa o argumento de cada superviviente, regresiones nuevas con su mutante); pila; límites declarados (qué NO cubre).
Las regresiones nuevas van en commits aparte (un test por defecto, con el mutante/defecto que lo hace fallar).

## 5. Criterios y límites
- Solo cuentan defectos reproducibles; una opinión de diseño sin programa va a «observaciones».
- Si un corte falla, no se arregla aquí: se escribe el hallazgo y el coordinador decide (volver a Flujo B para ese corte).
- Lagunas del ADR: no inventar contenido; remitir al coordinador (Flujo A).
- Presupuesto: API compartida; informar del gasto acumulado al coordinador.
