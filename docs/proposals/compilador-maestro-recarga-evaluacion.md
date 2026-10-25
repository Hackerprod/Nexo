# Evaluación de la arquitectura maestra del compilador (revisión 3.1)

Documento evaluado: `docs/proposals/compilador-maestro-recarga-r3-1.md` (aportación externa, referencia
`1df641e`, anterior a la publicación de D16 y D17). Este registro conserva lo que se decide portar a Nexo y
las contradicciones que requieren decisión explícita. No es normativo: la autoridad la tienen los
acuerdos del propietario, la base 0.2 y los ADR (`docs/VISION.md`).

Criterio rector (decisión del propietario, `docs/F0_ACUERDOS.md` §9.8): las ideas se valoran por su
mérito para Nexo. Una idea que beneficia no se descarta por ser difícil ni por contradecir una regla
anterior; se porta mediante una enmienda explícita y razonada.

## Ideas que se portan

| Idea (sección del documento) | Destino | Nota |
|---|---|---|
| Transformación de bucles para recarga: elegibilidad sobre la IR, estado transportado, equivalencia sin actualizaciones, préstamos, recursos y salidas (§4) | ADR propio de recarga con bucle permanente (nivel H1b) | Habilita recargar el bucle de eventos de una aplicación sin reiniciar; enmienda la regla 3 de §9.8 |
| Compatibilidad frente a protección física, callbacks permanentes e identidad del estado (§5.1–5.3) | ADR de recarga y enmienda a D17 R44/R45 | Hoy D17 solo trata el rechazo |
| Contexto de ejecución materializado y reglas de resolución nativa (§6) | Enmienda a D17 R45 | Debe resolver cómo viaja el contexto por la ABI, callbacks en C incluidos |
| Admisión, publicación y retirada, ventana puntero→contador, ciclos de propiedad (§7.1, §7.3) | Enmienda a D17 R44/R45 | Requiere un protocolo de retirada (épocas o equivalente) |
| Efectos de la instrumentación: el controlador respeta `no_alloc`, `no_block`, `no_suspend`, `no_io` (§8) | Enmienda a D17 R45 | Debe resolver el choque entre el mutex de admisión y `no_block` |
| Invalidación incremental, incluidas las búsquedas fallidas (§13.4) | Enmienda a D17 R10 | D17 excluye dependencias no alcanzadas sin tratar la búsqueda fallida |
| Modelo de objetos, relocaciones tipadas, selección de impl independiente del orden de enlace, raíces para eliminar código, convergencia de la relajación de saltos (§16.2–16.3) | Insumo de las tareas N16–N20 de D17 | Complementa R36 |
| Experimentos de snapshots y paralelismo, caché y memoria, herramientas (VC03, VC04, VC11) | Ampliación de D17 §9 | Con identificadores propios que no colisionen |

## Ideas que se portan con adaptación

| Idea | Adaptación necesaria |
|---|---|
| Atomicidad sobre estado vivo con huella de compatibilidad (§5.7, §7.2) | La validación optimista no es implementable con la exclusividad `edit` sin sincronizar; hay que diseñar el mecanismo (punto de sincronización cooperativo acotado) o dejarla como requisito abierto |
| Conservar la continuación de generadores y tareas suspendidas (E98-05) | Se valora frente a la regla 4: si beneficia, se porta con migración de estado explícita y enmienda a la regla |
| Retirar la regla 7 de equivalencia (E98-08) | Se conserva el arnés diferencial como mínimo obligatorio; la regla se reformula, no se elimina |
| Carga de código mediante fragmentos y permisos de memoria (§16.4) | Se alinea con D17 R45 (biblioteca inmutable por generación, sin JIT anónimo ni libc); la sincronización de instrucciones y los metadatos de pila se estudian como mejora de H2 |

## Contradicciones con lo vigente que exigen decisión

| Tema | Documento externo | Vigente | Resolución |
|---|---|---|---|
| Recarga con bucle permanente | `Main` no tiene que terminar (E98-04) | Regla 3 de §9.8 y D17 R44, B26 | Decidida por el propietario: se enmienda la regla 3 para bucles transformables (H1b) |
| Carga y libc | Bibliotecas nativas necesarias sin lista (§12.4, §17.3) | Acuerdos §9.3(a) y D17 R40 | Prevalece D17 R40 (listas cerradas); la excepción de libSystem en macOS sigue como propuesta |
| Construcción diversa | Investigación opcional (§18.4) | D17 R26 obligatoria antes de retirar Rust | Prevalece D17 R26 |
| Android como objetivo general | Contemplado (§17.3) | Roadmap §4.2 y §11.2 | Fuera de alcance hasta decisión del propietario |
| Roadmap del compilador independiente | Propone diseñarlo (§21) | AGENTS.md prohíbe una revisión 0.3 | La tabla de tareas N de D17 cumple esa función |

## Correcciones obligatorias antes de integrar

1. Renombrar los identificadores N1–N12, E98-xx y VC01–VC14 para que no colisionen con N01–N24 y R01–R60.
2. Alinear los apartados de carga, libc y arranque con D17 R40, R45 y R20–R27.
3. Especificar la revalidación de la huella respetando la exclusividad `edit`, o degradarla a requisito abierto.
4. Resolver el choque del mutex de admisión con `no_block`.
5. Definir cómo viaja el contexto de ejecución por la ABI.
6. Actualizar la referencia a D16 y D17 (existen en el árbol de trabajo).
7. Marcar las cifras de latencia de §10.3 como de origen no localizado hasta medirlas.

## Orden previsto

Cerrar D17; ADR de recarga con bucle permanente y enmiendas a D17 con las ideas anteriores; implementación
después de N10 y N14.
