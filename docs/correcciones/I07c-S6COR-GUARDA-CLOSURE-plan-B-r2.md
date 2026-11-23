# Revalidación B (contexto conservado) del plan I07c-S6COR-GUARDA-CLOSURE en origin/i07c-s6cor-guarda-closure @ 7e6dd7f

## Veredicto: PASS (los 4 motivos de la ronda 1 están corregidos con evidencia reproducida)

Candidato de B reconstruido: git archive e7b1813 + S1+S2 de §5 aplicados por B (copia desechable, borrada); binarios debug y release de base y candidato.

1. Semillas S5c: CORREGIDO. §7.3 enuncia comando (python gen2.py SEMILLA 1500 out.json), N=1500, blob de gen2.py y hashes. Recalculado: blob git de origin/i07c-estructural-r2:docs/validation/i07c-s5c/b-d2/gen2.py = 11ac8537bda405ae60b97a444dea112d2e41a5dc; SHA-256 del contenido = 11f10ff1cc3dab3bcbc7f8964951de23634b39549bdff1f8b9f23f477b357b90 (ambos iguales al plan). Las tres salidas (6112023, 1357911, 424242) dan exactamente los SHA-256 del plan (48f6e809..., 9e4c4d9f..., f9952957...). Base vs candidato, check y run, f0-0003-b-v1: 0 diferencias y 0 anomalías en las tres (4.500 fuentes); release igual en una semilla. Criterio del plan (0 cambios fuera de clases con efecto) cumplido.
2. Fuzz Apéndice C: CORREGIDO. El bloque se extrae del plan y se ejecuta sin cambios: python fuzzgen.py 3000 fuzz.json da 3000 fuentes (2573 distintas) y SHA-256 a797ef463114cde39105012fd6f85e210ae7ab9b027ac5211ee87be092c90e01, igual al plan. Ejecutado en check y run, f0-0003-b-v1 y f0-0005-b-v1 (12.000 ejecuciones por binario): base vs candidato debug y base vs candidato release: 0 anomalías (exit 0/1, stderr vacío), 0 rechazos de la base que pasen a aceptados, 0 discrepancias check/run en aceptación; cambian 6.136 de 12.000 salidas (el generador sí ejercita la regla); candidato debug==release y base debug==release en las 12.000. 
3. L1/L2/L3 firmes: CORREGIDO. Búsqueda de "abierta", "condicionad", "si L1/2/3", "lectura B", "pendiente de decisión": 0 coincidencias. §0 y §10 recogen L1=A, L2=1, L3=adoptar texto; M3 firme y matriz §7.4/§11.9 coherentes (M3 muere por la fila de Entity function:<offset de fn>).
4. §7.2: CORREGIDO. Exige DOS diagnósticos en el control de las 8 filas name y literal (las 4 if_* no tienen control), con los primarios [82,83), [40,42), [92,93), [45,47), ayudante propio que exige exactamente dos y no reutiliza one_diagnostic, y check==run. Primarios y doble diagnóstico verificados en ronda 1 con el candidato. Nota de corrección de B: en el informe 1 escribí 6 filas if_*; son 4 (name 4 + literal 4 + if 4 = 12); el plan es correcto.
   Notas: from_ast(ast).0 en l.4405 corregida; auditor por línea explícito (sin función+expresión, vecindad l.264, S1 no desplaza la l.1223); nota de escalado preexistente con cifras de B r1 y criterio de 60 s acotado; sin cambios en C0/S1/S2/D1 (tabla de commits) ni en M1-M5 salvo M3.

## Regresión de lo ya validado
- Corpus de 840 (Apéndice A): SHA-256 b82fa323f5d4c8ab0f0101017e02206b08857066aeeff1d06310e3a5ea97b860 (igual). Base vs candidato: cambian exactamente las clases *_eff de closure en guarda y guarda en closure; contract_*, fn_guard_* y las *_pure sin cambio; 0 anomalías; debug==release.
- Tabla de casos y suite (2205 tests, 2 fallos previstos) medidas en ronda 1 con el mismo mecanismo; el plan no cambió §2, §5, §6, §8 ni §7.4 salvo M3.

## Texto de reemplazo de la fila «guarda de match»
Sigue válido con el candidato (reverificado: 90 bytes inválido -> NX-E0415 [41,43) con contract_origin [15,20) y pasos [41,43)/intrinsic.call [53,58), más NX-E1305/guard_effect [41,43), io, known_effect, mismos pasos; 81 bytes válido con return true; check checked y run unit; 79 bytes sin with y con Print: solo guard_effect [30,32)). Comparado con la fila de §10 del plan: idéntica salvo que el plan pone `check`/`run` entre comillas inversas. Se puede pasar tal cual al coordinador (basta copiar la fila de §10 del plan).

## No bloqueante
Sin cambios respecto a la ronda 1 (escalado preexistente, auditor por línea, L3 edita un addendum revisado por decisión del coordinador, mutación solo ejecutable con el código de C).
