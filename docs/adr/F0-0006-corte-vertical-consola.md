# F0-0006 — Corte vertical de consola

Estado: **decisiones de planificación revisadas de forma independiente; sin implementación ni aprobación de fase**.

## 1. Mandato, autoridad y límite de la selección

Se aplica [F0_ACUERDOS §10](../F0_ACUERDOS.md), orden de trabajo, punto 4,
y [AGENTS.md](../../AGENTS.md), «Rigor proporcional y presupuesto de validación».
El objetivo es ejecutar sin cambios `total-registros.n` de F0-0004 §11 antes de
completar J, F5-L y K. Este documento solo selecciona y ordena entregas existentes;
no modifica semántica, aceptación, diagnósticos ni sintaxis de los ADR revisados o D15.

Referencias abreviadas: **A2** = [F0-0002](F0-0002-second-slice.md),
**J** = [F0-0003](F0-0003-third-slice.md),
**F5** = [F0-0005](F0-0005-operators-control-collections.md),
**K** = [F0-0004](F0-0004-text-and-user-types.md).
Las tablas de construcción están en J §12.2, **F5 §10** y K §12;
F5 §12 contiene enmiendas, no su tabla de subtareas.

**Hallazgo: el adelanto respecto de los tres cortes no es realizable conservando
el grafo exigido.** F5-L01 requiere «A2/J completos» y K01 requiere
«J01–J16 y F5-L01–F5-L12 completos». K10 exige transitivamente K01–K09.
Por tanto, cualquier selección cerrada bajo esas dependencias que incluya la
consola incluye todos los J y F5. No se puede llamar a esos prerrequisitos
«interfaz mínima» ni darlos por cumplidos con una fracción de su aceptación.

La selección admisible de §2 fija el primer hito ejecutable en **K10**, dejando
K11–K12 después. Esto ya es compatible con K §12; **no constituye el adelanto
solicitado respecto de J y F5**, ni se propone como cumplimiento completo del
mandato. Se registra el impedimento para validación independiente, sin suprimir
aristas ni inventar subtareas. La excepción del mandato para unidades inseparables
obliga aquí a conservar el orden original. El objetivo de anticipación queda abierto.

Entrada: I11b cerrado con la matriz V01–V13, suite migrada, reparaciones y contraste
debug/release de A2 §17; no se sustituye por la existencia de implementación I11a.
[HANDOFF.md](../../HANDOFF.md), «I11a val62 — cierre local de calidad», aún registra
revisión independiente e I11b pendientes. Los registros anteriores describen el
frontend B, inferencia, Result/`?`, efectos, cuotas y entorno mock existentes;
este ADR no certifica sus cierres ni presupone J, F5 o K implementados.

## 2. Selección mínima cerrada bajo las dependencias vigentes

La tabla es una ordenación topológica conservadora. La columna de dependencias
transcribe las filas de origen; I11b es además la puerta de entrada de §1.
«Indirecta» significa obligación del grafo, no uso de esa capacidad por la fuente.
La procedencia de la aceptación de cada J es J §12.2, de cada F5-L es F5 §10
y de cada K es K §12, incluidas sus asignaciones adicionales y pares.

**Entrega completa en todas las filas.** No se rebaja la aceptación a las
operaciones observadas en la aplicación. Para cada fila seleccionada rige el orden
original: no hay una parte mínima separable autorizada que permita omitir sus
prerrequisitos o publicar antes su capacidad. K03/K04/K08 sí tienen entregables
internos diseñados; se conservan todos y su orden, porque sus consumidores exigen
la K completa. Efectos, limpieza, IR y cuotas se entregan con cada capacidad.

| Orden / id | Dependencias de origen | Capacidad de K §11 que la exige y procedencia; alcance entregado |
|---|---|---|
| J01 | I07c,I08,I09,I10a,I10b | Base de Option/match para `siguiente`; contrato, barreras y perfil J §§2, 6–8 completos. |
| J02 | J01 | `Some`/`None` de ReadLine y selección consumidora; Option y propiedades transitivas J §3.1 completos. |
| J03 | J02 | Reconocer y comprobar match sobre `siguiente`; patrones/cobertura J §§3.2–3.3 completos. |
| J04 | J03 | `match move siguiente`, `Some(move texto)`; extracción y limpieza J §3 y §6.2 completas. |
| J05 | J04 | Indirecta: K07 exige J03–J05 para seleccionar `Lectura`; guardas J §3.3 completas, aunque la app no escribe guardas. |
| J06 | J02 | Indirecta: J13 soporta los `break`/`continue` de la app y depende de la cadena J06–J12; closures J §4.1 completas. |
| J07 | J06,J04 | Indirecta por J13; retención y escape de closures J §4.3 completos. |
| J08 | J07 | Indirecta por J13; invocación de closures J §4.2 completa. |
| J09 | J08,J05 | Indirecta por J13; resúmenes de efectos de closures J §4.2 completos. |
| J10 | J09 | Indirecta por J13; creación fría y estado propietario de generadores J §5.1 completos. |
| J11 | J10 | Indirecta por J13; Next/Yield/finalización J §5.2 completos. |
| J12 | J11 | Indirecta por J13; suspensión y limpieza J §§5.3–5.4 completas. |
| J13 | J12,J05 | `break` por EOF/fin y `continue` por línea vacía; control compartido J §5.4 completo, incluido for de generadores. |
| J14 | J13 | Indirecta: F5-L01/K01 requieren J terminado para los operadores/texto de la app; integración de costes y barreras J §§7, 12.2 completa. |
| J15 | J14,I11a | Indirecta por F5-L01/K01; EX04 y regresiones J §§11, 12.2 completos. La app K no llama las primitivas de EX04. |
| J16 | J15,I11b | Indirecta por F5-L01/K01; revisión integrada J §§12.2–13 completa. |
| F5-L01 | A2/J completos | Cálculo de importe/IVA/media, resto, comparaciones y `!=`; operadores F5 §2.1 completos. |
| F5-L02 | F5-L01 | `or` en dominio y `and` en descuento; lógica/cortocircuito F5 §2.2 completos, incluido `not`. |
| F5-L03 | F5-L01 | `self.total +=` y `self.cantidad +=`; cinco compuestas F5 §3.3 completas, ampliadas a campos por K04. |
| F5-L04 | J13, F5-L02 | Bordes de los bucles y retornos de la app; control/divergencia F5 §§2.2, 3.2 completos. |
| F5-L05 | F5-L01,F5-L04 | `for numero in 1..=maxLineas`; rangos F5 §3.1 completos. |
| F5-L06 | F5-L04 | `for r in registros`; iteración prestada y limpieza F5 §3.2 completas. Arnés IR hasta F5-L08, sin anticipar ejecución fuente. |
| F5-L07 | F5-L01, J02 | `[]` y `Push(edit registros, move r)`; Array dinámico/API F5 §4.1 completos, incluidos Pop/JoinArrays. |
| F5-L08 | F5-L01–F5-L07 | Habilitar en fuente los operadores y Array de la app; perfil/contabilidad F5 §7 completos y retirada de barrera conforme a F5 §10. |
| F5-L09 | F5-L01–F5-L05 | `const maxLineas` y `const iva`; resolución/pureza/tipos de constantes F5 §5 completos. |
| F5-L10 | F5-L08,F5-L09 | Evaluar los dos `comptime`; evaluación en IR, cuotas y caché F5 §§5–7 completas. |
| F5-L11 | F5-L01–F5-L10 | Indirecta: K01 exige F5 completo para texto/nominales; batería y conservación de B F5 §§10–11 completas. |
| F5-L12 | F5-L11 | Indirecta por K01; revisión adversarial F5 §10 completa. |
| K01 | J01–J16 y F5-L01–F5-L12 completos | Contrato de texto/nominales de toda la app; catálogo, descriptores y barreras K §§7–9 completos. |
| K02 | K01 | Literales `""`/`"fin"`, igualdad y copia propietaria de `original`; String base K §3 completo, sin interpolación parcial. |
| K03 | K02, F5-L01, F5-L06–F5-L08 | `parse.I64(texto)`; biblioteca y extensión común String/Array K §§3, 3.2 completas: K03.1–K03.3. |
| K04 | K03, F5-L03, F5-L06–F5-L08 | Registro/Lectura/Resumen, campos, `Array<Registro>` y for; datos nominales K §§3.2, 4 completos: K04.1–K04.3. |
| K05 | K04 | Préstamos de campos y limpieza de payloads; propiedad K §§4.2–4.4 completa. Recursos nominales no usados, pero K06/K07/K08 exigen K05 completo. |
| K06 | K05 | Activación de extensión y métodos Agregar/Texto; métodos K §5.1 completos, incluido receptor consumidor. |
| K07 | K05, J03–J05 | `match move lectura`, Vacia/Dato y extracción de Registro; selección nominal K §4.1 y §7 completa. |
| K08 | K05, F5-L01 | Ambos `@derive` de la fuente e interpolaciones de líneas/resumen; capacidades y BuildString K §§2.1, 3.1, 5.2 completos: K08.1–K08.3. |
| K09 | K06, K08 | `use std.console` y `use std.parse`; imports y visibilidad K §5.3 completos, incluidas exportaciones no usadas por la app. |
| K10 | K06, K07, K09 | ReadLine/Println y entorno de la app; consola K §§6–6.1, aplicación/variantes §§11, 11.2 y migración de fixtures §13 completas. |

Se conserva también el desglose interno de K §12, sin nuevos identificadores:

| Entregable interno, en orden dentro de su K | Dependencias de origen |
|---|---|
| K03.1 | K02 |
| K03.2 | K03.1, F5-L01 |
| K03.3 | K03.2, F5-L06–F5-L08 |
| K04.1 | K03 |
| K04.2 | K04.1, F5-L03 |
| K04.3 | K04.2, F5-L06–F5-L08 |
| K08.1 | K05 |
| K08.2 | K08.1, F5-L01 |
| K08.3 | K08.2 |

Sus unidades, evidencia y pending_scope siguen K §12. K03 no puede detenerse en
parseo para iniciar K04; K08.3 exige K08.2 aunque la app no compare nominales.
J13 no se divide en «solo break/continue»: J §12.2 exige J12/J05 y F5 §3.2
reutiliza ese control. K09 no se divide en «solo use»: K §12 conserva su unidad
de exportaciones/imports. K10 migra todo el entorno, sin lector compatible v1.

## 3. Exclusiones y hueco de planificación

| Subtarea excluida | Por qué no es necesaria para la primera ejecución de K §11 | Orden conservado después del hito |
|---|---|---|
| K11 | Robustez integrada y conservación del subconjunto anterior; no introduce una capacidad fuente de la app ni es prerrequisito de K10 (K §12). La robustez local sigue obligatoria. | Depende de K10. |
| K12 | Contraste integrado independiente de todo K; repite la app y amplía evidencia, sin aportar una operación para ejecutarla (K §§12–12.2). No sustituye la validación independiente del hito. | Depende de K11. |

**No hay J ni F5 excluible** bajo el grafo vigente. Closures, generadores, EX04 y
las baterías finales no son usos directos de la app, pero sus filas son
prerrequisitos obligatorios. Tampoco se omiten guardas, operaciones String,
Pop/JoinArrays, recursos nominales, igualdad derivada o exportaciones dentro de
las filas completas elegidas. Las subtareas excluidas conservan sus IDs y su
orden original; se cierran después del hito, sin renumeración.

No se identifica una capacidad de la fuente de K §11 sin propietario diseñado:
Result/`?` corresponde a I08, efectos a I09, y las demás se asignan en §2.
El hueco es **de planificación del adelanto**, localizado en F5 §§1.1, 10 y
K §§1.1, 12: no existe una interfaz de entrada parcial que satisfaga sus
dependencias «completos». Incluso omitir solo esas barreras globales dejaría la
cadena J13 → J12 → J11 → J10 → J09 → J08 → J07 → J06 para el control requerido.
Resolverlo exigiría revisar expresamente dependencias o diseñar separaciones;
este mandato prohíbe hacerlo. No se simula una solución mediante nuevos pasos,
aceptaciones parciales, barreras retiradas antes de tiempo o cambios a la app.

## 4. Casos de punta a punta

### 4.1. Aplicación y entorno, transcritos de K §11

`total-registros.n`: UTF-8 sin BOM, LF entre líneas y LF final. Fuente íntegra:

```nexo
use std.console;
use std.parse;
use extension package.Resumen;
const maxLineas: I64 = comptime 100;
const iva: I64 = comptime 21;

@derive(Format, PartialEq)
struct Registro {
    linea: I64;
    original: String;
    centimos: I64;
}

@derive(Format, PartialEq)
enum Lectura {
    Vacia,
    Dato(Registro)
}

struct Resumen {
    cantidad: I64;
    total: I64;
}
extend Resumen {
    fn Agregar(self: edit, r: view Registro) {
        self.total += r.centimos;
        self.cantidad += 1;
    }
    fn Texto(self: view) -> String {
        var media = 0;
        if self.cantidad != 0 {
            media = self.total / self.cantidad;
        }
        return "Registros: {self.cantidad}; total: {self.total / 100} EUR y {self.total % 100} cent; media: {media}";
    }
}

fn Convertir(linea: view I64, texto: view String) -> Result<Lectura, I64> {
    if texto == "" {
        return Ok(Lectura.Vacia);
    }
    let importe = parse.I64(texto)?;
    if importe < 0 or importe > 10000 {
        return Err(42);
    }
    var base = importe * 100;
    if importe >= 10 and importe <= 100 {
        base = base * 90 / 100;
    }
    let centimos = base + base * iva / 100;
    return Ok(Lectura.Dato(Registro(linea, texto, centimos)));
}

fn Main() -> Result<(), I64> {
    var registros: Array<Registro> = [];
    for numero in 1..=maxLineas {
        let siguiente = console.ReadLine()?;
        match move siguiente {
            Some(move texto) => {
                if texto == "fin" {
                    break;
                }
                let lectura = Convertir(numero, texto)?;
                match move lectura {
                    Lectura.Vacia => { continue; },
                    Lectura.Dato(move r) => {
                        Push(edit registros, move r);
                    }
                }
            },
            None => { break; }
        }
    }
    var resumen = Resumen(0, 0);
    for r in registros {
        edit resumen.Agregar(r);
        console.Println("L{r.linea}: {r.original} EUR => {r.centimos} cent")?;
    }
    console.Println(resumen.Texto())?;
    return Ok(());
}
```

Entorno completo, transcripción de K §11:

```json
{"schema_version":2,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"console_read_line":[{"ok":"12"},{"ok":""},{"ok":"8"},{"ok":"5"},{"ok":null}],"console_println":[{"ok":null},{"ok":null},{"ok":null},{"ok":null}]}
```

Stdout exacto, transcripción de K §11, UTF-8 con LF final:

```text
L1: 12 EUR => 1306 cent
L3: 8 EUR => 968 cent
L4: 5 EUR => 605 cent
Registros: 3; total: 28 EUR y 79 cent; media: 959
```

Resultado transcrito: `{"kind":"ok"}`, `diagnostics=[]`, `exit 0`,
`observation_bytes=320` (202 de eventos y 118 de stdout),
`reserved_observation_bytes=0`. Los recuentos de bytes son citas de K §11 y andamiaje: este ADR no los
fija ni los exige, solo reproduce el resultado de su fuente. Las observaciones, en orden, son:

| Tipo | Payload transcrito de K §11 |
|---|---|
| event | `read_line:ok:2` |
| event | `read_line:ok:0` |
| event | `read_line:ok:1` |
| event | `read_line:ok:1` |
| event | `read_line:eof` |
| event | `println:ok:23` |
| stdout | `L1: 12 EUR => 1306 cent` |
| event | `println:ok:21` |
| stdout | `L3: 8 EUR => 968 cent` |
| event | `println:ok:21` |
| stdout | `L4: 5 EUR => 605 cent` |
| event | `println:ok:49` |
| stdout | `Registros: 3; total: 28 EUR y 79 cent; media: 959` |

También se ejecutan las variantes literales de K §11 (entrada 18, EOF/fin,
parseo, dominio, errores de lectura/escritura y cola vacía) y el lote con
centinela de K §11.2, con sus entradas, salidas y trazas de origen, sin
recalcular esperados con el producto. Son obligaciones de K10 completo.

### 4.2. Negativo aislado y positivo de K §§9, 10.1

K4-X01, archivo `X01.n`, UTF-8 sin BOM y sin LF final:

```nexo
fn Main(){let s="{}";}
```

Esperado transcrito: exactamente un `NX-E1401/empty_interpolation`,
`stage=parse`, `operation=syntax.parse`, primario `[17,19)`, relacionado
`delimiter [16,17)`, `data={}`. Token inesperado: symbol `}` en `[18,19)`.
Rige el contrato común C de K §10.1: schema_version=1,
diagnostic_namespace=Nexo-proposed-v1, profile_id=f0-0004-b-v1, severity=error;
rule y explanation_id = `NX-E1401/empty_interpolation`; file=X01.n;
feature_id=null, expected_tokens=[], expected_tokens_kind=null,
parser_context=null, recognized_prefix=null, repair=[], cause_chain=[].
No se fija un mensaje distinto del contrato de origen.
El positivo sustituye únicamente `{}` por `{1}`; pasa check y run.
Ambos recorren la CLI; el negativo llega a su regla y no se ejecuta como programa
válido. Un Err de parse.I64 en la app es un resultado tipado, no este diagnóstico.

## 5. Clasificación, aceptación y cierre

La clasificación se aplica a **cada requisito de cada fila**, también a las
sub-K; una fila mixta no convierte toda su aceptación en andamiaje:

| Requisitos | Clase según AGENTS.md |
|---|---|
| Sintaxis B, nombres, tipos/inferencia, patrones, propiedad/préstamos, efectos, enteros, Result/`?`, valores y errores de ejecución de todas las filas de §2; código/subcausa/span primario de sus diagnósticos, incluido X01 | NÚCLEO |
| Resultado de la app y variantes, cálculo, transferencia/limpieza, orden de evaluación y de eventos de recursos; reglas de iteración, constantes, String, nominales, métodos y derivación | NÚCLEO |
| Perfiles, cifras de cuotas/trabajo/bytes y sus arneses en J01/J14, F5-L08/F5-L10, K01–K10 y comprobaciones locales de las demás filas | ANDAMIAJE |
| Entorno/JSON/migración, transporte y mensajes CLI, representación de eventos mock, formato stdout/JSON, bytes y exit de §4; scripts y arneses de J15/J16, F5-L11/F5-L12 y de todas las pruebas locales | ANDAMIAJE |
| Selección, orden documental y registro de evidencia/pendientes de este ADR | ANDAMIAJE de proceso; no altera reglas del lenguaje |

Los efectos y el resultado/error/orden semántico no se degradan a andamiaje por
usar consola simulada; las cifras, el protocolo y la presentación sí lo son.
Las cuotas no eximen de limpieza ni permiten pánicos, internal_error o cuelgues.

El **hito ejecutable hasta K10** cierra cuando la fuente íntegra de §4 pasa check
y run y produce el resultado, stdout y observaciones de K §11; las variantes y
el límite de K §11.2 cumplen sus esperados; y todas las filas seleccionadas
cumplen la aceptación de sus tablas en la parte entregada, aquí completa.
No se redefine su aceptación mediante esta demostración.

Rigen la evidencia y el presupuesto de AGENTS.md/F0_ACUERDOS §10: para núcleo,
pares aislados, esperados del ADR, fuzz y diferenciales debug/release y check/run;
mutación inversa por regla y una sola campaña final sobre núcleo. Para andamiaje,
ejemplos normativos y fuzz, sin campañas de mutación ni oráculos independientes.
Se conservan independencia del validador, evidencia por requisito con
PASA/FALLA/NO EVALUADO/INCONCLUSO, regresión de cada defecto de núcleo y máximo
de dos rondas completas antes de verificaciones dirigidas. Esta prevalencia de
proceso no elimina capacidades o resultados exigidos por las tablas de origen.

El cierre del hito **no cierra el objetivo de anticipar J/F5**, que sigue
impedido según §§1, 3. Tampoco cierra K11/K12 ni aprueba una fase. Al implementar,
se actualizan Pending.md, HANDOFF.md y RECOVERY.md conservando el historial útil;
el progreso se informa por programas objetivo ejecutados sobre el total.

## 6. Riesgos y diferidos

El principal riesgo es presentar como vertical una selección que conserva todo
J y F5: el grafo actual no aporta ese ahorro. Reducir su aceptación, aceptar
solo la fuente de referencia o retirar barreras antes de efectos/cuotas violaría
el encargo. Separar la consola de la migración completa produciría dos entornos
vivos, contrario a K §§6.1, 13. Copiar aquí los literales exige contrastarlos con
su fuente al revisar; no crea otro propietario de expectativas.

Quedan diferidos K11 → K12, la resolución arquitectónica del impedimento de
planificación, los límites de J §2.2/F5 §8.2/K §1.2 y §13, E/S real y las puertas
del roadmap: dos revisores sin discrepancias y aprobación de fase por el propietario.
No se declara PASS, implementación, ejecución medida ni aprobación de fase.

## 7. Decisión de planificación (coordinador, 2026-11-12)

La validación independiente confirmó el impedimento de §1: el grafo de prerrequisitos de F0-0003, F0-0005
y F0-0004 exige J01–J16 y F5-L01–F5-L12 completos antes de K01, de modo que la consola de K §11 ejecuta por
primera vez en K10. Se adopta el orden original con ese hito: `I11b → J01–J16 → F5-L01–F5-L12 → K01–K10`
(consola ejecutable) `→ K11–K12`. Se evaluó la reducción de prerrequisitos a J01–J13 y F5-L01–F5-L10, que
ahorraría cinco de las veintiocho filas J/F5 a cambio de una enmienda a los tres ADR y del riesgo de rehacer
K al integrar J14–J16 y F5-L11/F5-L12 después; no se adopta. El propietario puede revertir esta decisión o
pedir esa enmienda por el Flujo A. Mientras tanto, el avance se mide con los programas objetivo de
`corpus/objetivo/`, que muestran cada corte qué programas nuevos ejecutan.
