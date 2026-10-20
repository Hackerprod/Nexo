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
