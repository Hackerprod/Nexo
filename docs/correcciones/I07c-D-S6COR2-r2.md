# Verificacion dirigida D de S10 sobre 8e6f777d108249895967c4839d8885a6a5bca498

Veredicto: PASS.

1. Esperado por bytes: derivado por mi. En fn Main(){{}{}{}{}{}{}{}while true{let a=1;let y=move a;}} el prefijo move esta en [49,53) (10 + 14 + 11 + 8 + 6 = 49); F0-0002 secc. 11 (prefijo consume cobra 1 tick con origen en ese token) da R0001 con primario move. El test usa at(source,"move") (find en la fuente, no la salida del binario) y exige exit 1, R0001 y primario (token, token+4); pareja con bucle acotado aceptada. No tautologico.
2. Mutante M00029 (prepare_expr, == a != en la busqueda del token): SHA-256 inicial f8fcbfde7a21, mutado f4bccbc5, restaurado f8fcbfde7a21 verificado por mi. El test pasa con el candidato y falla con el mutante.
3. git diff c75e704 8e6f777 --stat: solo tests/i07c_group_primary.rs (+19) y docs/correcciones/I07c-C-S6COR2-informe.md; sin src/ ni scripts/.
4. gate.sh 8e6f777 --verify: GREEN (registro 8734e228...), 2194 debug y 2196 release pasan, RD 152/152; SHA-256 de validate.sh = blob = origin/main (7a1c7a19...).
5. Informe de C: coherente (M00029 pasa de superviviente a muerto por S10; quedan M00013, M00021, M00023, M00051, M00054 sin programa y M00047/M00048 por for, con fecha 2026-11-23).

No bloqueante: ninguno nuevo.
