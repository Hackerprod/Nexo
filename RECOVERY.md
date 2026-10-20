# Recuperación de Nexo

1. Leer README.md, Pending.md y docs/F0_ACUERDOS.md desde el estado remoto actual.
2. Leer íntegra la base 0.2 y comprobar los hashes indicados en F0_ACUERDOS.md.
3. Inspeccionar git status y el último commit remoto; no sobrescribir cambios de otro
   agente ni asumir que un archivo local llegó a GitHub.
4. Continuar únicamente el siguiente corte declarado. Toda fase sigue pendiente
   salvo evidencia y revisión explícitas; no confundir compilación con validación.
5. Implementador y validador deben estar separados. Guardar hallazgos y regresiones.
6. Respaldar cada paso completado en GitHub con commit descriptivo y verificar el
   nuevo HEAD. Nunca reescribir el historial para hacer desaparecer fallos.

Los documentos originales del usuario están en docs/baseline/. Las recomendaciones
posteriores y sus estados están en docs/F0_ACUERDOS.md. No reconstruir decisiones
basándose solo en resúmenes de chat.

Al primer respaldo documental no se publica ni se afirma validado el código parcial
que pueda existir en un workspace. Su publicación requiere alcance y pruebas propios.

## Reproducir el primer corte

Usar la cadena de `rust-toolchain.toml` con rustfmt/clippy. Ejecutar
`scripts/validate.sh` desde la raíz del repositorio; los tests no necesitan red
ni dependencias externas. Los artefactos de `target/` no se respaldan y no son
prueba de que el código actual haya pasado. Recompilar antes de repetir repros.
La revisión independiente y hashes están en `docs/validation/F0-0001-review.md`.
No ocultar los tres defectos originales encontrados durante la revisión.

## Recuperación del corte I01

Leer primero README.md, Pending.md y docs/F0_ACUERDOS.md. Comprobar los hashes de la base contra el blob de Git mediante `git show HEAD:<ruta> | sha256sum`; inspeccionar `git status` y el último commit remoto antes de continuar. Continuar solo el siguiente corte pendiente. Mantener separados implementador y validador, respaldar cada paso con commit descriptivo y verificar HEAD remoto sin force push. Los tres defectos originales del primer corte (altura compuesta del AST, orden de límites negativos y truncamiento UTF-8) permanecen documentados y no deben ocultarse.

I01 está implementada localmente y pendiente de revisión independiente; I02–I11 no iniciadas. El addendum de códigos sigue pendiente de fusionar. Las reglas de emisión y las inserciones vacías en el mismo punto no se validan por el esquema; se comprobarán en los emisores.
