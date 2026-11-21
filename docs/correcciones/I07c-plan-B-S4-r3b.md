# Validador B - plan I07c S4, confirmacion paso 2.2 de la ampliacion r3

Veredicto: PASS.
- §1: SHA-256 = aad0c50fc49c219f4d08a22e40bb36513de203644725b4a14ea405700e1079e3, igual al de `git show HEAD:src/check/types.rs | sha256sum` (8aba06f). El valor erroneo fa39c369 ya no aparece.
- §11 anadido: recoge los tres no bloqueantes (constante contamina la visita, coherente con types.rs 10869-10872 y 11074; Checker.f5 es configuracion; SHA-256 de los nexo.exe de referencia 3100a29 en el informe de C). Sin contradicciones con §3/§5.
- No se comprobo nada fuera de ese delta.
