# I07c C: mutaciones supervivientes, lote 1

Población congelada: `docs/validation/i07c-mutations-ccf/population.json`, SHA-256 `a6f21ab42c6bc0436f7b12f9053519824e819ae762f67f12f3bb8cb741a103f4` (739 entradas). Código de producción S3: `11a70ec`; `src/check/types.rs` tiene cambios solo dentro de `#[cfg(test)]`. La batería previa clasificó estos 31 IDs como supervivientes en release. Esta revisión añade tres regresiones: 28 IDs pasan a **detectada** y 3 son equivalentes por invariante.

## Ruta y criterio

- F1: `var q=(if true then {return;} else {return;}); let x=q[selector]; q=[1];` activa `clone_projection_selector`. En la copia instrumentada, siete selectores oficiales con asignación, `if/else`, `while`, `for`, `match` (patrón, guarda y brazos) y closure pasaron con S3; los 23 mutantes F1 fallan en ese mismo test.
- F2: la batería existente `i07c_pending_nominal_domain` alcanzó `attach_conditional_binding` (4 llamadas), `set_conditional_result` (8) y `refuted_result_sites` (373) en la copia instrumentada. Las pruebas internas fijan los resultados refutados por clase y el enlace de cada obligación con su sitio, usando estados que esas rutas alcanzan.
- F3: `clear_failed_constructors` solo se invoca al comenzar una función, después de `type_nodes.clear()` en `Checker::function`; por tanto M00079 y M00080 iteran cero nodos. M00082 solo invierte el valor local `was_new` devuelto por `mark`; todos sus llamadores descartan ese retorno y la escritura `state.flags |= bit` es idéntica. No hay programa fuente que distinga esos tres mutantes.

## Resultado por ID

El SHA-256 es el del blob `src/check/types.rs` sustituido exactamente según la población, calculado sobre la copia usada en las reejecuciones (antes del ajuste final de formato de una prueba interna).

| ID | Resultado | SHA-256 blob mutante |
|---|---|---|
| M00043 | matado por `pending_projection_clones_all_selector_statement_shapes` | `ccdb5085237a135cfe8ac962133aa3e4a0aa6f56e0fa47a30e4d991650593bc8` |
| M00044 | matado por `pending_projection_clones_all_selector_statement_shapes` | `fd67d824a2fd2eddb8b2757269d2979ff975bc4a1b7746295c5624e11844a3d0` |
| M00046 | matado por `pending_projection_clones_all_selector_statement_shapes` | `f9b3159a9cc3404ab1643d68b844ddb2a295ee0289b4f68f65ec8cbf4c7a9aa7` |
| M00047 | matado por `pending_projection_clones_all_selector_statement_shapes` | `0ed9c3ccc16578297beee748737d238fad0da0dac1b374521aec9025a413952b` |
| M00048 | matado por `pending_projection_clones_all_selector_statement_shapes` | `b35beb80f74da63adc28fda2a5086e944e803af1b90de2b788f732ce2797dec8` |
| M00049 | matado por `pending_projection_clones_all_selector_statement_shapes` | `9d89c1ad90b8b00e882b9c7efbe389cb4f158fcb7170503752abbe4e9845a56b` |
| M00050 | matado por `pending_projection_clones_all_selector_statement_shapes` | `35e5fdff588c66e6c897d4b9e1944811b016ded0aaa49afee075d3aa35c4e194` |
| M00051 | matado por `pending_projection_clones_all_selector_statement_shapes` | `5d991fe72a94c1b29438400c491a28a9c72a82c805d37640c5e1e6dc03412db7` |
| M00052 | matado por `pending_projection_clones_all_selector_statement_shapes` | `aa544d518f22fa79ebe83514eef55b1c901bb745feca00328880ce749e3ba939` |
| M00056 | matado por `pending_projection_clones_all_selector_statement_shapes` | `bead201644fe2ccfd91d67ed1d248e3b08f1da42d07ac3c6ee7faa02ece16703` |
| M00057 | matado por `pending_projection_clones_all_selector_statement_shapes` | `3e6fb3ca64769d79cc65a6557c66d9348cb9f558cb139162bcf685902081194d` |
| M00058 | matado por `pending_projection_clones_all_selector_statement_shapes` | `00aa7be933c40dcf1c72f64f56fcc58cc8c7c27ce8ff012466397deda85d9212` |
| M00059 | matado por `pending_projection_clones_all_selector_statement_shapes` | `eb0880ad3171017bd6de4c54c9b141e03470a2bbe3df779fc142e77e796a4a5b` |
| M00060 | matado por `pending_projection_clones_all_selector_statement_shapes` | `76c274875a106052462e1ea19f8d19a452271b984db8e36443aac5213aa5af91` |
| M00061 | matado por `pending_projection_clones_all_selector_statement_shapes` | `34a71a72752cfb108c39318df4cd86f2a597edd70d08908ea8c55047e34065d7` |
| M00062 | matado por `pending_projection_clones_all_selector_statement_shapes` | `804c3394ac7a6fda19696ef8a69a4ab38cb628424931f0fd1004ce30deb429fc` |
| M00063 | matado por `pending_projection_clones_all_selector_statement_shapes` | `dc73dc1c382452013b3fa79a497a7efc2bc5d665afefa124243e4da120ba44f5` |
| M00064 | matado por `pending_projection_clones_all_selector_statement_shapes` | `05871654a46a465f936ce17e47df248c211f40c3f5891553bf3ef2d1ad7b37b5` |
| M00065 | matado por `pending_projection_clones_all_selector_statement_shapes` | `8064a511bcb546a0357b88bf58b422cc3a4dc1c304c199837e3234d81185b13e` |
| M00066 | matado por `pending_projection_clones_all_selector_statement_shapes` | `6069d08821bca91a8943b650da7d384ac52acac77325ed48e27b05879d019233` |
| M00067 | matado por `pending_projection_clones_all_selector_statement_shapes` | `0edaee80be33cbb66696694e094ab8c57cafb65564aafdf28c34b4f84a2ee0e7` |
| M00068 | matado por `pending_projection_clones_all_selector_statement_shapes` | `c8ecd617a35e71f734492364a4d6280821851c072d934b8e395d7e64c38539d0` |
| M00069 | matado por `pending_projection_clones_all_selector_statement_shapes` | `e80bf2e6c253952230fd379e792429f61d65c2c3e19889860c2b2ef88ccc770b` |
| M00072 | matado por `i07c_survivor_refuted_binary_and_guarded_results` | `07a3ef52629d45d3bdbdacd7b7bb8eba46b88975c7f243585993f308184d04e2` |
| M00073 | matado por `i07c_survivor_conditional_bindings_follow_their_result_sites` | `bf66d0fa46a70afc173129188d756f0f5ecda0d70e4a686ceee98bb3e1630f6b` |
| M00074 | matado por `i07c_survivor_conditional_bindings_follow_their_result_sites` | `789033fcdb73838745ea0f346befb2fe9a51784d0324a1e65372d82077a86886` |
| M00075 | matado por `i07c_survivor_conditional_bindings_follow_their_result_sites` | `9544f830c4caff5bbd58c409f7799f5fac4d0d275552d986ebcd0e3d71d5e632` |
| M00076 | matado por `i07c_survivor_conditional_bindings_follow_their_result_sites` | `a7dedd14387d65a8544fa6b0a37d2c1193fcbea34b6e69f60846550973a5f1ad` |
| M00079 | equivalente: `type_nodes` ya está vacío al entrar en `clear_failed_constructors` | `b661ad06e3a2023c51957822fae1ef98560fc32aac06787a5690adb1d14c1fed` |
| M00080 | equivalente: `type_nodes` ya está vacío; asignar `true` no alcanza ningún nodo | `1c6c249577fbd4cb9c178e6b91defd5df9018f473a6892b202ed044fd2a4db3e` |
| M00082 | equivalente: todos los llamadores descartan el `bool` devuelto por `mark`; el bit se inserta igual | `10276ca12633d26aff01cb1665f9ecfb45429b0a5b964abfd85426a40f6e4af4` |

## Verificación

Entorno de las dos pasadas: `I07C_MUT_DIR=docs/validation/i07c-mutations-ccf`, `I07C_MUT_TIMEOUT_SCALE=2`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=1`, `TEMP=TMP=D:\nexo-work\tmp`. El script invoca Cargo con `--locked --offline`. Su comprobación exige que `--target-dir` quede bajo la raíz de la copia; se usó `D:\nexo-work\i07c-c-surv1\tree\target` y una segunda copia desechable en `tree\disposable\tree`. Una sola reejecución pesada estuvo activa a la vez.

```text
python scripts/i07c_mutaciones.py --base ccf6df1 --integration-test i07c_solver_depth --ids M00043,M00044,M00046,M00047,M00048,M00049,M00050,M00051,M00052,M00056,M00057,M00058,M00059,M00060,M00061,M00062,M00063,M00064,M00065,M00066,M00067,M00068,M00069 --reuse-tree D:/nexo-work/i07c-c-surv1/tree/disposable --target-dir D:/nexo-work/i07c-c-surv1/tree/target --results-copy D:/nexo-work/i07c-c-surv1/results.jsonl --mutation-profile release --profile release
Base i07c_solver_depth: 0.53 s
23/23 IDs F1: detectada (pending_projection_clones_all_selector_statement_shapes)

python scripts/i07c_mutaciones.py --base ccf6df1 --unit-prefix check::types::quota_tests::i07c_survivor --ids M00072,M00073,M00074,M00075,M00076 --reuse-tree D:/nexo-work/i07c-c-surv1/tree/disposable --target-dir D:/nexo-work/i07c-c-surv1/tree/target --results-copy D:/nexo-work/i07c-c-surv1/results.jsonl --mutation-profile release --profile release
Base check::types::quota_tests::i07c_survivor: 0.17 s
M00072: detectada (i07c_survivor_refuted_binary_and_guarded_results)
M00073,M00074,M00075,M00076: detectada (i07c_survivor_conditional_bindings_follow_their_result_sites)
```

El verificador afirmó la restauración SHA-256 después de cada sustitución. Hash de `types.rs` en la copia antes y después de las 28 reejecuciones: `499555f0d92669ec31596252594dfec5e4954cfc05bdf1e989f0e633c1ea1742`. El ajuste posterior de `cargo fmt --check` cambió solo la disposición de una llamada en `#[cfg(test)]`; el hash final sincronizado de las copias y del árbol de entrega es `539a6a570a97891956e49e599e605ffb060e69030081e8b03304c991c0807bd4`. El blob de `git show HEAD:src/check/types.rs` es `1c1f9edda28e8377b9f9fbd9dcb845b169f00163daeac161d7b8f5af86596b36`; las líneas de producción no difieren de HEAD.

`cargo fmt --all -- --check` pasó tras el ajuste. La primera repetición release reutilizó el binario de M00076 porque `Copy-Item` preservó una fecha anterior a la compilación mutante; falló la prueba de enlaces pese a que el archivo fuente estaba restaurado por SHA-256. Se actualizó solo la fecha de `types.rs` en la copia, sin alterar sus bytes, y Cargo recompiló la base correcta:

```text
cargo test --locked --offline --release --lib i07c_survivor -- --nocapture
test ...i07c_survivor_conditional_bindings_follow_their_result_sites ... ok
test ...i07c_survivor_refuted_binary_and_guarded_results ... ok
test result: ok. 2 passed; 0 failed

cargo test --locked --offline --release --test i07c_solver_depth pending_projection_clones_all_selector_statement_shapes -- --exact
test pending_projection_clones_all_selector_statement_shapes ... ok
test result: ok. 1 passed; 0 failed
```

## Puerta mecánica

En un clon nuevo con solo estos tests y este informe copiados, se ejecutó `sh scripts/validate.sh` con `CARGO_TARGET_DIR=D:/nexo-work/target-i07c-surv1`, `RUST_TEST_THREADS=4` y las variables de compilación indicadas arriba. Terminó con **código 0**: fmt, clippy `-D warnings`, tests debug y release. Salidas de los binarios pertinentes:

```text
debug:   lib 1138 passed, 0 failed; i07c_solver_depth 12 passed, 0 failed; i07c_rd_conformidad 1 passed, 0 failed
release: lib 1140 passed, 0 failed; i07c_solver_depth 12 passed, 0 failed; i07c_rd_conformidad 1 passed, 0 failed
148 resúmenes de test: ok; ninguno FAILED; doc-tests: 0 fallos
```

La primera ejecución de la puerta con el paralelismo predeterminado quedó detenida en un proceso hijo de `j14_semantic_boundaries` al invocar `nexo run` sobre `fn Main(){match 1{?=>{},_=>{}}}`; se terminó esa ejecución. La misma invocación aislada devolvió rechazo esperado (`status 1`) con `check` y `run` en menos de 15 s, y el test afectado pasó en la repetición completa con cuatro hilos. No se atribuye un defecto de lenguaje a ese bloqueo transitorio.
