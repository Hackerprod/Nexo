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

## Corte ejecutable actual (experimental)

Rust 1.99.1, sin dependencias externas. Se necesita `rustfmt` y `clippy` para
validación. La cadena está fijada en `rust-toolchain.toml`.

```sh
cargo run --locked --offline -- check corpus/pass/nested_copy.n
cargo run --locked --offline -- run corpus/pass/nested_copy.n
sh scripts/validate.sh
```

Salida del segundo comando: `[[1, 9], [7, 8]]`. El programa comprueba también
que cambiar una copia anidada no modifica su original.

La sintaxis oficial es B, descrita en [F0-D15](docs/adr/F0-D15-sintaxis-oficial.md).
`check` analiza una biblioteca sin exigir `Main` y verifica la IR sin ejecutar
efectos. `run` exige `Main`, compila la IR verificada y ejecuta el subconjunto
disponible. Los recursos son simulados y su rastro se escribe a stderr; no abre
archivos Nexo ni sockets. Los códigos de salida son 0 éxito, 1 diagnóstico y 2
error de invocación, lectura o fallo interno controlado. Las opciones de
`check` y `run` todavía no están disponibles. Result, contratos de efectos y
la interfaz mock integrada reciben `NX-E0006/implementation_limit` con el
ámbito pendiente correspondiente. No hay compilador de producción.

- [Contrato y límites actuales](docs/adr/F0-0002-second-slice.md)
- [Sintaxis oficial y límites de la migración](docs/adr/F0-D15-sintaxis-oficial.md)

El intérprete sigue siendo evidencia parcial de F0. Ningún resultado suyo es
una medida competitiva de Nexo ni aprueba una fase.
