# I07c-S4: diferencial de la familia

Scripts congelados de `I07c-S4-r1.md` §5.4 (Python 3.13, semilla 4071). SHA-256:

| Archivo | SHA-256 |
|---|---|
| `family_fuzz.py` | `e41a4da818f32f0067f4f0fe12f26c54001df5f71ad0814c5fb3973e58e65f25` |
| `family_fuzz_recheck.py` | `16f7124db624d97f8fe1a44079951cfc12fb0e3fe1a9386d13eea671c8f6b792` |
| `family_index.py` | `5d2f1a5fb1e6d709f8e0704ec91dbb45800d0d5a0c39a2f6aa88f75f857f906d` |

El primer corpus tiene 3.072 fuentes y SHA-256 `854578a62fe37895cd8a98c7062ac2bd5fcd910e88c6035745ce92b720171758`. La ampliación Index tiene 384 fuentes y SHA-256 `6c4071d6249a1c804d766bcd6bd0efdf18d60a95178d64ce21afc2c809d07433`. Ambos scripts comprueban que el corpus leído coincide con el generado.

Copie los tres scripts a una carpeta de trabajo junto a seis ejecutables: `base-nexo.exe`, `base-release-nexo.exe` (`be8c47c`), `3100a29-debug-nexo.exe`, `3100a29-release-nexo.exe`, `candidate-nexo.exe` y `candidate-release-nexo.exe`. Ejecute `python -B family_fuzz.py freeze` y `python -B family_index.py freeze`; compruebe los hashes del corpus **antes** de `python -B family_fuzz_recheck.py run` y `python -B family_index.py run`. El runner compara `check` y `run`, debug y release, código de salida y stdout/stderr completos. Registre los SHA-256 de los seis ejecutables antes de cada barrido.
