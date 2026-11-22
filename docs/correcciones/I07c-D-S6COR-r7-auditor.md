# Informe de D: verificacion dirigida del auditor, commit ddf551e4f47338df6a494f50badf797b6466978b

## VEREDICTO: PASS

Mi unico motivo de FAIL (el auditor no detectaba un .span() crudo nuevo en find_witness) esta corregido y
tiene regresion. `git diff f8c148f ddf551e -- src` esta vacio (160 lineas solo en scripts y el informe de C).

(1) Repro original: `fn rogue(e: &Expr) -> Span { e.span().clone() }` antes de `fn find_witness(` en copia
    de ddf551e: exit 1 (unreviewed raw span). Batería de evasion propia (28 inserciones en copias desechables):
    - exit 1 (correcto): antes de find_witness; al final de effects.rs y de names.rs; antes de mod tests en
      names.rs; en una funcion producto con llaves en cadena, comentario, char (`'}'`, `'{'`, `'"'`, `'\''`) y raw
      string r#"}"#; cfg(test) fn con llaves en cadena y char seguido de funcion producto con .span(); raw
      string con hashes r##..##; lifetimes `<'a, 'b>`; cfg(test) en `use`, en const con `[u8; 4]` y en un `let`
      dentro de una fn; `#[cfg(all(test, unix))] fn` y `#[test] fn` fuera de modulo (conservador: se tratan como
      producto); `.span()` encadenado en varias lineas y `.span().start`; `.span()` dentro del cuerpo de una fn
      producto; types.rs en una fn producto.
    - exit 0 (correcto): `.span()` dentro de witness_differential_tests (modulo de test real); en `mod tests` de
      names.rs; en `#[cfg(test)] fn` (con y sin atributo `#[allow]` intercalado).
    Huecos reproducibles, ninguno bloquea (todos contrived o preexistentes y ajenos al motivo):
    H1. `#[cfg(test)]` sobre una variante de enum o un campo de struct (`enum Ev { #[cfg(test)] A, B }`): el
        atributo queda pendiente y la siguiente `{` (la de la funcion producto siguiente) se marca como zona de
        test; un `.span()` crudo en esa funcion da exit 0. Hoy ni names.rs ni effects.rs usan esa forma.
    H2. types.rs conserva la logica antigua (todo lo posterior a `mod quota_tests {`, linea 16019, es test): un
        `.span()` crudo en una fn producto anadida al final del fichero da exit 0. Preexistente, no tocado por esta
        correccion; types.rs no tiene produccion tras quota_tests hoy.
    H3. El patron es el texto `.span()`: `Expr::span(e)` (UFCS) no se detecta y `.span ()` con espacio tampoco
        (rustfmt lo normaliza; el UFCS no). Heuristica heredada de S6, no regresion.
(2) Arbol limpio: auditor exit 0, `reviewed: 20/20` (types 14, names 5, effects 1), sin pendiente_decision;
    `--self-test` PASS con 13 rechazos.
(3) Informe de C sobre S2e/Q7: HONESTO. Retira el argumento falso de inalcanzabilidad, cita mi ejemplo
    (`(f())()` que devuelve closure llega a efectos y comparte sitio), declara Q7 "no observable en los programas
    probados" y registra Q4 y Q7 como supervivientes sin programa. Coincide con mis resultados.
(4) Puerta: `sh scripts/gate.sh ddf551e4f47338df6a494f50badf797b6466978b --verify`: GREEN (arbol
    ce9081ecf704e38536e39f5583e266234c125ada); log 2168/2168 debug y 2170/2170 release (coinciden con el JSON),
    f5-stack65 5/5 + 5/5; SHA-256 de scripts/validate.sh = origin/main:scripts/validate.sh =
    7a1c7a199e4bcb09356bc6d56df8bb80556a5403f59a57cf502343deded130df.

Resto de requisitos de la ronda anterior (producto, barridos, pila, mutacion): sin cambios, el codigo es el ya
validado en f8c148f.

Limpieza: copias y temporales propios borrados; sin procesos propios vivos.
