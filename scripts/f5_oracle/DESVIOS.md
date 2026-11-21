# Desvíos normativos del oráculo F5

El desvío F5-S08 quedó resuelto por decisión revertible del coordinador:
`check` acredita la biblioteca reparada; `run` espera
`NX-E0112/missing_main`. Detalle y citas en `LAGUNAS.md`. No hay desvío
normativo abierto en esta pieza.

## Observación de producto en G1b (3100a29)

`fn F()->I64{while true{return 1;}}fn Main(){}` en `check` y `run`
con perfil `f0-0005-b-v1` obtiene NX-E1317/ownership/flow.return en
debug y release. F5 §2.2 dice que ese bucle satisface `->I64`, y §8.2
marca L04 pendiente para el `while true` literal en función no unitaria
sin `return` final; el esperado en G1b es NX-E0006,
`implementation_limit`/L04, ownership/ownership.check. Este caso queda
fuera de la familia de contraste G1b hasta que se corrija el producto.
`while true{break;}` sí tiene caída normal y recibe NX-E1317 según §2.2
y F5-C02 (§9.2.1); no hay contradicción entre esas reglas del ADR.
