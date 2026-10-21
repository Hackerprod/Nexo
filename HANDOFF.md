# Continuación de Nexo

Estado: I03 validada tras revisión independiente y publicada; I02b está publicada en `84d5f53`.
El propietario decide el inicio de cada fase.

## Leer primero

1. README.md y Pending.md: estado real, ejecución y pendientes.
2. docs/baseline/Nexo_ROADMAP_rev0.2.md y Nexo_CAMBIOS_rev0.2.md completos.
3. docs/F0_ACUERDOS.md: D11–D13, propuesta H-PY y proceso de validación.
4. docs/adr/F0-0001-first-slice.md: semántica exacta del prototipo reducido.
5. docs/validation/F0-0001-review.md: revisión independiente, incidencias y límites.

La base 0.2 permanece intacta. No crear una revisión 0.3 ni dar por aprobada F0.
El implementador y el validador deben ser distintos; el validador no implementa ni
rebaja los requisitos para aprobar. Cada avance terminado debe respaldarse en
GitHub y verificarse contra el estado remoto, sin force push ni secretos.

## Qué hay implementado

Un crate Rust 1.99.1 sin dependencias: lexer/parser, checker de tipos y propiedad,
AST comprobado opaco, intérprete y CLI `check`/`run`. Arrays anidados se copian
físicamente. Los recursos mock solo se transfieren y cierran; no representan
handles reales. Hay 5 archivos positivos, 10 negativos por código esperado y
22 tests de integración (19 de semántica/corpus, 3 de CLI).

La validación independiente añade arneses reproducibles de casos adversos y un
modelo Python para comparación diferencial. Consultar su informe para resultados
exactos; su aprobación se limita al corte, no al lenguaje completo.

## Cómo verificar tras clonar

Instalar o usar Rust 1.99.1 con rustfmt y clippy. Python 3 se usa solo por los arneses
independientes. Desde la raíz:

```sh
./scripts/validate.sh
cargo build --locked --offline
cargo build --locked --offline --release
python3 docs/validation/F0-0001-adversarial.py
python3 docs/validation/F0-0001-differential.py
```

No confiar en binarios de un workspace anterior. El script principal ejecuta fmt,
clippy estricto y tests debug/release. Las pruebas no requieren red. No se ha
configurado ni verificado CI remoto; la evidencia actual es local Linux x86-64.

## Tres defectos encontrados y corregidos

- Altura compuesta del AST: combinar cadenas y paréntesis eludía los límites
  locales y causaba stack overflow en debug. Se limita altura al construir nodos.
- Índices negativos de asignación: se comprobaban antes del RHS, contradiciendo
  el orden declarado. Se evalúan todos los índices y el RHS antes de los límites.
- Lectura CLI truncada: cortar un emoji en el límite confundía tamaño con UTF-8
  inválido. Se limita lectura de bytes, comprueba tamaño y después decodifica.

Además se acotó profundidad inferida de arrays entre bindings, se protegió el
renderizado de diagnósticos UTF-8 y se probó limpieza al agotar presupuesto.

## Próximo trabajo concreto

1. Implementar I04 (IR y intérprete escalar) con identidades de función estables y tabla de funciones sustituible.
2. Continuar I04–I11 según §17 del ADR F0-0002 cuando se autorice cada corte.
3. Extender los tres flujos reducidos exigidos por F0: herramienta, actualización
   de datos y petición simulada. EX01/EX04 completos no están implementados.
4. Completar los modelos y protocolos de F0, incluidos estados D09, plataformas,
   IR/backend, invariantes, H-* y T01–T28. No reducir F0 a una lista de demos.

El `move` del prototipo es sintaxis provisional del corte; la base utiliza
`consume` en sus interfaces. Al ampliar la gramática, resolver y registrar esa
correspondencia. No afirmar que los terminadores opcionales, Result, efectos,
closures o generadores ya existen: se rechazan en el frontend actual.

## I01

I01 implementa el esquema de diagnósticos, su presentación JSON/texto, `nexo explain` y lectura acotada; fue validada tras revisión independiente y publicada en `d4836cf`. I02a fue validada y publicada en `f073275`; I02b está validada tras revisión independiente y publicada en `84d5f53`; I03 validada tras revisión independiente y publicada; I04–I11 no iniciadas. El addendum F0-CODIGOS-ADDENDUM-0002 queda pendiente de fusionar en F0-CODIGOS.md. D11–D13 y F0-0002 tienen PASS de diseño, no aprobación de fase. Las reglas de emisión y las inserciones vacías en el mismo punto no se validan por el esquema; se comprobarán en los emisores. Las lagunas conocidas de I01 constan en docs/validation/F0-0002-I01-implementation-notes.md. F0 sigue pendiente de dos revisores sin discrepancias, T01–T28, plataformas y las demás puertas de Pending.md.

## I02a

I02a implementa un `src/frontend/lex.rs` separado con tokens y spans de bytes para el perfil léxico F0-0002, usando la API de diagnósticos estructurados de I01 para NX-E0001. No sustituye el lexer actual de `syntax.rs`; `check` y `run` conservan su ruta anterior. LX01–LX05 tienen negativo y positivo pareado con `+`. El alcance, la reproducción y las ambigüedades del ADR constan en `docs/validation/F0-0002-I02a-implementation-notes.md`. I02a está validada tras revisión independiente y publicada en `f073275`.

## I02b

I02b está validada tras revisión independiente y publicada en `84d5f53`. `frontend::parse` y `frontend::print` modelan sintaxis, diagnósticos y texto canónico; `compile`, `check` y `run` siguen la ruta anterior. Las regresiones locales incluyen `OperationList` vacío ante tokens erróneos, reparación tras trivia, prefijos de sentencias tras saltos y entradas anidadas desde pilas pequeñas. El parser usa un hilo ancho reutilizable y devuelve un diagnóstico si no puede crearlo. La cobertura, los límites y la reproducción constan en `docs/validation/F0-0002-I02b-implementation-notes.md`.

## I03

I03 está validada tras revisión independiente y publicada. `check::check_proposed` usa el AST de I02b, registra firmas y resuelve nombres por ámbito antes de asociar argumentos y comprobar tipos básicos y retornos escritos. Devuelve diagnósticos ordenados de sitios independientes y ejecuta las pasadas semánticas en un hilo acotado de 16 MiB. `src/check/names.rs` y `src/check/types.rs` siguen aislados de `compile`, `nexo check` y `nexo run`. Las notas de alcance, diagnósticos, pruebas y límites están en `docs/validation/F0-0002-I03-implementation-notes.md`. El cierre completo de las fixtures con ejecución, préstamos, control o Result sigue en los pasos que asigna F0-0002 §17.2. I04–I11 no están iniciadas; F0 conserva sus puertas pendientes.

La prueba de escalado de I03 usa 16 familias por defecto y 24 con `NEXO_FULL_SCALE`, de 256 KiB a 1 MiB. El tiempo de la suite completa y las razones medidas se consignan en las notas de I03; la prueba de escalado preexistente del parser domina la duración. Los límites y las lagunas observadas están en esas notas.
