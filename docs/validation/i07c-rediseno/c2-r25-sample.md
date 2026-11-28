# I07c C2 — muestra dirigida de 30/102, B r25 + D9

Cada fila fue ejecutada en `main` y candidato con el perfil congelado y tiene 
decisiones JSONL de igualdad. SHA de fuente, firmas completas y traza están en 
`c2-r25-ledger.json` y `c2-r25-trace-120.json.gz`. Los nueve dictámenes 
iniciales de conformidad proceden de B r25. Los otros 17 conformes usan 
el límite normativo de una denuncia por unión sin contexto externo, 
comprobado contra el `owner_ordinal` y la salida; el oráculo parcial 
no decide el veredicto.

| Fila | Estado | `main` / candidato (códigos y primarios) | Decisiones rechazadas en traza |
|---|---|---|---:|
| `b-projections-2026112401:280` | PASA según B r25 | NX-E1303@159, NX-E1303@185, NX-E1303@231, NX-E0102@245 / NX-E1303@159, NX-E1303@185, NX-E1303@231, NX-E0102@245 | 3 |
| `b-projections-2026112401:550` | PASA según B r25 | NX-E0102@224, NX-E0102@232, NX-E0102@244 / NX-E0102@224, NX-E0102@232, NX-E0102@244 | 12 |
| `b-projections-2026112401:1183` | PASA según B r25 | NX-E1303@151, NX-E1303@183, NX-E0102@192 / NX-E1303@151, NX-E1303@183, NX-E0102@192 | 0 |
| `b-projections-t5-2026112402:1266` | PASA según B r25 | NX-E0102@87, NX-E0102@106, NX-E0102@113, NX-E0102@123, NX-E0102@148 / NX-E0102@87, NX-E0102@106, NX-E0102@113, NX-E0102@123, NX-E0102@148 | 13 |
| `c2:2858` | PASA según B r25 | NX-E1303@123, NX-E1303@131 / NX-E1303@123 | 0 |
| `c2:8742` | PASA según B r25 | NX-E1303@138, NX-E1303@145 / NX-E1303@138 | 1 |
| `c2:15473` | PASA según B r25 | NX-E1303@132, NX-E1303@151, NX-E1303@161 / NX-E1303@132, NX-E1303@151 | 0 |
| `c2@f0-0003-b-v1:7609` | PASA según B r25 | NX-E0103@78, NX-E1303@188, NX-E1303@195 / NX-E0103@78, NX-E1303@188 | 3 |
| `s7021993:3267` | PASA según B r25 | NX-E1303@182, NX-E1303@192 / NX-E1303@182 | 3 |
| `b-projections-2026112401:1390` | FALLA: ruta de testigo | NX-E0103@70, NX-E0102@131, NX-E0102@192, NX-E0102@200 / NX-E0103@70, NX-E0102@131, NX-E0102@192, NX-E0102@200 | 6 |
| `c2:9591` | D9: conservar main | NX-E0103@67, NX-E1303@144, NX-E1303@166, NX-E1303@223 / NX-E0103@67, NX-E1303@144, NX-E1303@223 | 6 |
| `c2@f0-0003-b-v1:9591` | D9: conservar main | NX-E0103@67, NX-E1303@144, NX-E1303@166, NX-E1303@223 / NX-E0103@67, NX-E1303@144, NX-E1303@223 | 6 |
| `b-projections-2026112401:1196` | PASA: una por unión | NX-E1303@165, NX-E1303@187, NX-E1303@197, NX-E1303@257 / NX-E1303@165, NX-E1303@187, NX-E1303@257 | 4 |
| `b-projections-2026112401:2977` | PASA: una por unión | NX-E1303@124, NX-E1303@144, NX-E1303@151, NX-E1303@221, NX-E1303@266, NX-E0102@282, NX-E0102@292 / NX-E1303@124, NX-E1303@144, NX-E1303@221, NX-E1303@266, NX-E0102@282, NX-E0102@292 | 12 |
| `b-projections-t5-2026112402:820` | FALLA: falta E0103 | NX-E0103@70, NX-E0102@147, NX-E0102@169, NX-E1303@177 / NX-E0102@147, NX-E0102@169, NX-E1303@177 | 7 |
| `b-projections-t5-2026112402:838` | PASA: una por unión | NX-E0103@70, NX-E0102@77, NX-E1303@102, NX-E1303@109, NX-E0102@124, NX-E1303@161, NX-E0102@173 / NX-E0103@70, NX-E0102@77, NX-E1303@102, NX-E0102@124, NX-E1303@161, NX-E0102@173 | 7 |
| `c1-nueva:631` | PASA: una por unión | NX-E0103@76, NX-E1303@114, NX-E1303@144, NX-E1303@154, NX-E1303@176 / NX-E0103@76, NX-E1303@114, NX-E1303@144, NX-E1303@176 | 3 |
| `b-projections-2026112403:1916` | PASA: una por unión | NX-E1303@143, NX-E1303@153, NX-E0102@194 / NX-E1303@143, NX-E0102@194 | 6 |
| `c1:921` | PASA: una por unión | NX-E0102@90, NX-E1303@141, NX-E1303@148, NX-E1303@168, NX-E1303@190 / NX-E0102@90, NX-E1303@141, NX-E1303@168, NX-E1303@190 | 9 |
| `c1:11189` | PASA: una por unión | NX-E1303@135, NX-E1303@145, NX-E0102@191 / NX-E1303@135, NX-E0102@191 | 4 |
| `c1:13247` | PASA: una por unión | NX-E1303@133, NX-E1303@140, NX-E1303@166 / NX-E1303@133, NX-E1303@166 | 6 |
| `c1:15115` | PASA: una por unión | NX-E1303@130, NX-E1303@137, NX-E1303@159 / NX-E1303@130, NX-E1303@159 | 3 |
| `c1:15304` | PASA: una por unión | NX-E1303@142, NX-E1303@164, NX-E1303@174 / NX-E1303@142, NX-E1303@164 | 0 |
| `c1:21363` | PASA: una por unión | NX-E1303@131, NX-E1303@138, NX-E1303@146 / NX-E1303@131, NX-E1303@146 | 3 |
| `c2:176` | PASA: una por unión | NX-E0103@67, NX-E0103@85, NX-E1303@140, NX-E1303@150 / NX-E0103@67, NX-E0103@85, NX-E1303@140 | 0 |
| `c2@f0-0003-b-v1:176` | PASA: una por unión | NX-E0103@67, NX-E0103@85, NX-E1303@140, NX-E1303@150 / NX-E0103@67, NX-E0103@85, NX-E1303@140 | 0 |
| `b:3042` | PASA: una por unión | NX-E1303@153, NX-E1303@164, NX-E1303@191 / NX-E1303@153, NX-E1303@191 | 6 |
| `b:3122` | PASA: una por unión | NX-E0103@85, NX-E1303@139, NX-E1303@149, NX-E1303@181 / NX-E0103@85, NX-E1303@139, NX-E1303@181 | 6 |
| `s7021993:278` | PASA: una por unión | NX-E1303@182, NX-E1303@192, NX-E0102@228, NX-E0102@235 / NX-E1303@182, NX-E0102@228, NX-E0102@235 | 9 |
| `s7021993:2391` | PASA: una por unión | NX-E1303@146, NX-E1303@153 / NX-E1303@146 | 3 |

26 conformes, 2 defectos propios del candidato (1390 y 820), 2 filas de 
laguna con conservación D9 (9591 ×2). Son **4/30 = 13,3 %** que exigen 
corrección en esta muestra dirigida; 2/30 = 6,7 % son defectos intrínsecos 
ya derivados. La selección deliberada por familias no permite extrapolar 
esas tasas a las 102. El rango 10–25 % de B r25 sigue como hipótesis de 
trabajo. Restan 72 filas fuera de la muestra.
