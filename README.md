# Nexo

Un lenguaje de programación de propósito general que busca combinar ergonomía,
control de recursos y costes explícitos. Su identidad no depende de IA.

## Estado real

**F0 en exploración. Ninguna fase aprobada; no hay compilador de producción.**
La revisión 0.2 es la línea base documental. Sus ejemplos y cifras son objetivos,
no resultados ejecutados. No se prepara una revisión 0.3.

- [Roadmap 0.2 original](docs/baseline/Nexo_ROADMAP_rev0.2.md)
- [Cambios 0.2 originales](docs/baseline/Nexo_CAMBIOS_rev0.2.md)
- [Acuerdos posteriores y peticiones de revisión](docs/F0_ACUERDOS.md)
- [Entrega para el siguiente agente](HANDOFF.md)
- [Trabajo pendiente y estado de recuperación](Pending.md)
- [Cómo continuar después de un reinicio](RECOVERY.md)

Los originales se conservan byte por byte. Los acuerdos posteriores se registran
por separado para no hacer pasar cambios nuevos por contenido de la línea base.
El índice anterior es el punto de entrada para recuperar el contexto.

## Responsabilidad y validación

El implementador escribe el código. Un subagente separado revisa decisiones,
implementación y pruebas, puede rechazar entregas y exige regresiones para cada
fallo corregido. No se permiten aprobaciones basadas únicamente en demos o en el
recuento de tests. Esta revisión por agentes no se presentará como auditoría humana
externa ni como prueba universal de seguridad.

En cada paso terminado se guarda un commit en este repositorio y se verifica el
estado remoto. No se publica información privada, credenciales ni registros ajenos
al proyecto. Los hitos parciales deben declararse parciales.

## Primer corte ejecutable (experimental)

Rust 1.99.1, sin dependencias externas. Se necesita `rustfmt` y `clippy` para
validación. La cadena está fijada en `rust-toolchain.toml`.

```sh
cargo run --locked --offline -- check corpus/pass/nested_copy.nx
cargo run --locked --offline -- run corpus/pass/nested_copy.nx
./scripts/validate.sh
```

Salida del segundo comando: `[[1, 9], [7, 8]]`. El programa comprueba también
que cambiar una copia anidada no modifica su original.

El corte incluye parser, tipos/propiedad secuenciales, intérprete y CLI. Los
recursos son simulados y su rastro se escribe a stderr; no abre archivos Nexo ni
sockets. Los códigos de salida son 0 éxito, 1 diagnóstico y 2 error de invocación
(o lectura). `check` no ejecuta efectos. No hay backend nativo, IR bajada, préstamos,
efectos, closures, generadores ni biblioteca real. Los puntos y coma son obligatorios
en este corte; las características no implementadas se rechazan.

- [Contrato y límites del corte](docs/adr/F0-0001-first-slice.md)
- [Revisión independiente](docs/validation/F0-0001-review.md)

Este resultado aporta evidencia parcial a T01/T02/T07/T08/T25, no su aprobación
completa. Ningún resultado de este intérprete es una medida competitiva de Nexo.
