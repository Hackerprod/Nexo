# Nexo — Visión y principios

Este documento orienta y enlaza; no es una especificación. Si resume mal una regla, manda la fuente
normativa. Orden de autoridad, de mayor a menor: decisiones explícitas vigentes del propietario
(`docs/F0_ACUERDOS.md`), la base 0.2 y sus enmiendas registradas (`docs/baseline/`), los ADR aplicables
(`docs/adr/`) y, como resumen de todo ello, esta página. `Pending.md`, `HANDOFF.md` y `RECOVERY.md`
informan de lo que está implementado y validado; no cambian las reglas del lenguaje.

## Qué es Nexo

Un lenguaje de propósito general, de tipado estático con inferencia, que busca facilitar el desarrollo
y permitir control progresivo de la máquina (memoria, representación, concurrencia) sin desechar la
lógica del programa. Debe tener valor por sí mismo, sin modelos de IA ni servicios remotos. Sus
herramientas deben resultar útiles tanto a personas como a agentes; los diagnósticos estructurados y
estables sirven a ambos, pero no sustituyen a la finalidad del lenguaje: ante cada decisión se pregunta
también si la regla se entiende, si facilita escribir y modificar una aplicación, si evita trabajo
repetitivo y si permite rendimiento sin rehacer el programa.

## Invariantes del núcleo

- Los datos se copian con semántica de valores independientes, los recursos se transfieren y el estado
  compartido se declara. Un valor que contiene estado compartido no es una copia independiente por estar
  dentro de otra estructura.
- Propiedad y limpieza exactas de recursos, préstamos locales, enteros comprobados, `Result` y `?`,
  `return` explícito y efectos transitivos (`no_alloc`, `no_block`, `no_suspend`, `no_io`), que permiten
  controlar una parte del programa sin obligar a escribir toda la aplicación a bajo nivel.
- Una sola semántica rige la interpretación y la ejecución nativa. Las optimizaciones preservan los
  resultados definidos, los errores, los efectos y las modificaciones autorizadas. No existe un modo
  cómodo inseguro ni uno rápido que elimine garantías en silencio.
- La capa alta aporta colecciones, texto y servicios útiles; la capa baja ofrece control sobre la misma
  semántica (`docs/F0_ACUERDOS.md` §9.4).
- La productividad, el rendimiento, la portabilidad y los costes se comprueban con programas reales.

## Sintaxis

La sintaxis B (`docs/adr/F0-D15-sintaxis-oficial.md`) es la única oficial; la extensión de los archivos
es `.n`. No hay competencia entre candidatas, modo de compatibilidad ni segunda sintaxis. Ningún lenguaje
debe dominar la superficie; se admiten influencias familiares cuando aportan por mérito.

## Dirección de largo plazo (intención del propietario, no capacidades implementadas)

Autoalojamiento y arranque sin Rust en la cadena final; bibliotecas escritas en Nexo sobre dos capas;
backend propio tras la validación de extremo a extremo; recarga en caliente por niveles; una biblioteca
de interfaz basada en AetherUI con HTML y CSS vanilla. La secuencia del backend es: arranque en Rust,
núcleo e IR propios, LLVM como generación nativa inicial y transición a un backend propio con criterios
medibles (`docs/adr/F0-D17-autoalojamiento-y-backend.md`). Ninguna de estas metas sustituye a los
compromisos del núcleo ni se convierte por sí sola en el siguiente hito: la interfaz de usuario llega
después de validar sintaxis, motor y concurrencia.

## Cómo se decide

- Una sola implementación en el árbol final: sin adaptadores permanentes, sin parches por patrón de
  fuente y sin flujos duplicados. Un defecto local se corrige localmente; si varios fallos revelan una
  causa estructural, se rediseña la causa.
- Las observaciones de una revisión se clasifican según el contrato del corte, no según el número de
  rondas: un incumplimiento obligatorio bloquea; una mejora fuera del compromiso se registra sin
  bloquear.
- Cada corte debe acercar una capacidad visible y comprobable. La demostración con entrada y salida
  simuladas y la aplicación utilizable con entrada y salida reales son hitos distintos.
- Diseño, implementación y validación las hacen agentes distintos, y un PASS vale para el corte, no para
  la fase. La aprobación de las fases F0 a F7 es del propietario.
