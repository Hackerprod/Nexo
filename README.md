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
