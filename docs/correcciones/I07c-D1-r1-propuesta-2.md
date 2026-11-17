# D1 - propuesta r1 corregida y enmienda r7 (HEAD 18aead2)

Veredicto: PASS.

1. Archivos: confirmado por grep de constructor_type/constructor_arity en src, tests, scripts y docs/adr. Conjunto completo: src/diagnostic.rs (subcause_description 2160, published_source 2480, CATALOG 2739, mas validate_data 1042), F0-CODIGOS.md, tests/i01_diagnostic.rs, j01_profile.rs, j04_match_semantics.rs; j03_match.rs sin conjunto cerrado; F0-0004 solo cita tipos de usuario futuros. Sin otro catalogo.
2. Criterio de cursores: comprobable (cero campos, cero booleanos transportados entre ocurrencias, lecturas permitidas enumeradas, grep con alias). Cierra la escapatoria.
3. Esperados: recalcule los 16 offsets de la tabla nueva sobre la fuente expandida; todos coinciden. None-primero (NX-E1304 [75,76), context [66,70)) concuerda con F0-0003 8.2 y con la salida actual de la base. `let k=x;` aceptado es derivacion, no literal del ADR; la propuesta limita a N(f) lo no decidido.
4. Diferencial: incluye mi fuzz semillas 1-5 y 369 fuentes contra 9b8e03e. Congelar ademas el SHA-256 de h/fuzz.py (hoy: ver abajo) en el repositorio; no bloquea.
5. r7 coherente: S2 lleva catalogo y emisor en un solo commit; S1/S2 separados; excepcion documental a "no tocar ADR" declarada y acotada a F0-CODIGOS. Pendiente de A/B antes de editar codigo.

Nota: durante el paso anterior hice un checkout temporal en C:\projects\Nexo-i07c para leer archivos y volvi a la rama i07c-estructural; arbol limpio.
