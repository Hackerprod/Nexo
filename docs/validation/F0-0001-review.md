# Revisión independiente de F0-0001

Fecha: 2026-10-20 UTC. Revisor: subagente distinto del implementador.

## Veredicto

**PASS limitado: aceptable como primer corte exploratorio descrito en el ADR.**
Los tres defectos reproducidos durante esta revisión fueron corregidos por el
implementador y revalidados independientemente. No quedan fallos conocidos abiertos
que bloqueen ese alcance concreto. El revisor no modificó la implementación ni
rebajó los contratos para obtener este resultado; solo escribió este informe y
los dos arneses de validación que lo acompañan.

**F0 completo: NOT EVALUATED / NO APROBADO.** Tampoco se aprueban F1 ni las familias
T01/T02/T07/T08/T25 completas. La evidencia se limita a sus casos representados en
este subconjunto. Un único revisor-agente no satisface automáticamente la puerta
documental de dos revisores ni constituye una auditoría humana externa.

## Contrato revisado

Se leyeron `docs/F0_ACUERDOS.md`, `docs/adr/F0-0001-first-slice.md` y las partes
pertinentes de la base 0.2: P02/P03, D01/D02/D04/D07, F0 y T01/T02/T07/T08/T25.
Se inspeccionaron parser, checker, runtime, API de compilación, CLI, corpus y tests.

El programa ejecutable es un AST encapsulado y comprobado, no una IR propia bajada.
La copia es física profunda; los recursos son simulados. `inspect` no equivale a
préstamos generales. Los errores recuperables del intérprete no implementan
`Result`, `?`, ni la política completa de `panic`. Estos límites están declarados,
no se cuentan como prestaciones implementadas.

## Hallazgos: FAIL original → PASS tras corrección

### V01 — Alta: desbordamiento de pila por profundidad compuesta

La versión inicial limitaba recursión sintáctica y cadenas por separado, pero no
la altura total del AST. Este generador produce una fuente de 28.400 bytes:

```python
expr = '1'
for _ in range(110):
    expr = '(' + expr + '+' + '+'.join(['0'] * 128) + ')'
source = 'fn main(){print(' + expr + ');}'
```

Antes de corregir: `check` y `run` debug abortaban por stack overflow, código de
proceso -6; release aceptaba e imprimía `1`. Una entrada muy inferior a 1 MiB
eludía el límite y cambiaba de resultado según el perfil.

Corrección: límite composicional de 128 comprobado al construir cada expresión.
Revalidación: ambos comandos/perfiles rechazan el repro mediante E0003 sin aborto.
Regresión: `composed_ast_depth_is_bounded_before_recursive_passes`.

Además se revisó la protección añadida para tipos inferidos por declaraciones
sucesivas `let a0=0; let a1=[a0]; ...`: profundidad 128 se acepta y 129 se rechaza
con E0003. El repro de 4.000 declaraciones también se rechaza. Este endurecimiento
lo identificó asimismo el implementador; no se afirma haber reproducido un crash
original de profundidad de tipos.

### V02 — Media: comprobación negativa adelantada al RHS

```nexo
fn main(){let r=mock_open(1);var a=[0];a[-1]=9223372036854775807+1;}
```

Antes: R0102 por convertir prematuramente el índice a `usize`. Con índice positivo
fuera de rango se obtenía R0101. El ADR exige evaluar todos los índices, después el
RHS y finalmente comprobar límites: debía prevalecer R0101 en ambos casos.
También fallaba `a[-1][9223372036854775807+1]=0`.

Corrección: conservar índices firmados hasta la fase de comprobación de límites.
Revalidación: R0101 en ambos perfiles y orden exacto open/close conservado.
Regresión: `assignment_checks_negative_bounds_after_rhs`.

### V03 — Baja: fuente UTF-8 válida mal diagnosticada tras lectura limitada

Durante el endurecimiento de la CLI se introdujo este caso:

```python
source = b'fn main(){}//'
source += b' ' * (1_048_576 - len(source))
source += '🙂'.encode()
```

La lectura de 1.048.577 bytes como texto cortaba el emoji: una fuente válida de
1.048.580 bytes recibía E0004/exit 2 por UTF-8 en vez de E0003/exit 1 por tamaño.

Corrección: leer bytes acotados, comprobar tamaño y luego decodificar UTF-8.
Revalidación: E0003/exit 1 en debug y release. Se verificaron además el límite
exacto, exceso ASCII y UTF-8 realmente inválido dentro del límite.
Regresión: `cli_size_limit_precedes_utf8_decoding_of_truncated_input`.

## Evidencia final: PASS

Entorno ejecutado: Linux 6.18.55 x86_64, Rust 1.99.1
(`58cf8f9c3`, 2026-10-13). Sin dependencias externas del crate.

- `scripts/validate.sh`: formato, clippy con warnings denegados y tests offline,
  locked, debug/release. 22 tests por perfil: 19 semánticos/corpus y 3 CLI.
- Corpus registrado: 5 archivos positivos y 10 negativos con códigos esperados.
  Los casos de propiedad se rechazan por sus reglas, no por errores de parser.
- Arnés adversarial independiente: 26 casos dirigidos × 2 perfiles = 52 resultados
  exactos de salida, diagnóstico y secuencia de eventos. Incluye copia, sombreado,
  transferencia, doble consumo, tipos, inmutabilidad, orden de errores y limpieza.
- 1.846 truncamientos/mutaciones deterministas, semilla 19022026: ningún aborto ni
  terminación anómala en `check` debug. Esto no es fuzzing exhaustivo.
- 16 comprobaciones independientes de bordes de tamaño, UTF-8, altura del AST y
  profundidad inferida, repartidas entre debug y release.
- 400 programas generados, semilla 20261020, con resultados modelados en Python
  independientemente del intérprete: 200 de copias/mutaciones/sombreado de arrays
  anidados y 200 de suma i64 comprobada/precedencia de errores. 800 ejecuciones
  debug/release con igualdad exacta de stdout, código y eventos.
- 3 recorridos adicionales × 2 perfiles verifican orden exacto de transferencia,
  retorno que omite adquisiciones y limpieza externa/interna al agotar presupuesto.
- Los hashes de ambos documentos base coinciden con los registrados en acuerdos.

La contabilidad de trabajo se revisó conforme al ADR: sentencias, expresiones de
datos y celdas copiadas. Indexar consume el temporal propietario sin una segunda
copia. La limpieza no se corta al agotar presupuesto. No es una cota de tiempo,
memoria ni de todas las operaciones del anfitrión.

## Reproducción

Desde la raíz del repositorio, con la cadena fijada y Python 3 disponibles:

```sh
./scripts/validate.sh
cargo build --locked --offline
cargo build --locked --offline --release
python3 docs/validation/F0-0001-adversarial.py
python3 docs/validation/F0-0001-differential.py
```

Los arneses usan solo la biblioteca estándar de Python y archivos temporales.
No ejecutarlos con `python -O`: sus verificaciones usan `assert`. Recompilar los
binarios antes de validar cambios; su existencia en `target/` no acredita frescura.

## Fuera de esta aprobación: NOT EVALUATED

IR bajada, efectos, préstamos/exclusividad, traits/coherencia, D09, D11–D13,
closures, tareas, generadores, `Result`, biblioteca real, EX01/EX04 completos,
backend nativo y diferencias intérprete/backend, otras plataformas, ABI/FFI,
seguridad formal, protocolos de rendimiento/adopción y todas las puertas F0.
No se comprobó publicación remota ni CI remoto en esta revisión; corresponde
verificar el commit publicado separadamente. Un nuevo cambio semántico reabre
las comprobaciones afectadas.

## Identidad del candidato validado

SHA-256 del código y tests al terminar la revisión, antes del commit de respaldo:

```text
cfe9525108ae8b62c9ef5cedd4b9e64001b0c34137e5728675ce6aff85eac28e  src/check.rs
704f3a6fa96fe4c12b236a382b3604b6979eec86b8c2f4dbfbc53a898e999c52  src/lib.rs
27676ae6db9ec3f59072979ae7bdf30b9a26988af4c33d18e90483cffaaf5487  src/main.rs
4405a9df124dfb331f5852c9d81bcbaba3c44c5111f5508996142560ab3b5af8  src/runtime.rs
bba15efb250d2f6dfa8febb047989821d74223b4c14c3b12ba8cdad4a6d8ad82  src/syntax.rs
3764c0c81d0122ba9c8b141fab896175d7b1209702ecbfa24b09e2283b7d824d  tests/cli.rs
dc4856e00f06188d457dd93a41194719f0221f21d9db4aa21f3cabb6a4f1f6dc  tests/corpus.rs
afe6cfb23e66fec143247e30d9ab753eb7866dc5ba0bfbd718bc8cba5deb92a5  Cargo.toml
c1d2d5fb16604f00c6d3b0d4c0a79e90e85c4dd85c7f7c9663b60ff48efb60ad  Cargo.lock
6b01b513c7c84859b8a03800ca00e4e593381a4756bc42b6b4a9118fbaf59e5c  scripts/validate.sh
9dd4dfe320e6d0b375d3573b82f20a58e0dc4336388f3ef593e5be4b11ea53d7  docs/validation/F0-0001-adversarial.py
0ef8a15193eb182b8c96abbaae4240f0437d9b5a61546b9d6ccfdae4f8afff0b  docs/validation/F0-0001-differential.py
```
