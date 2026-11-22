"""Restore the archived 6394b67 source after an interrupted mutant run."""
import hashlib
from pathlib import Path
import subprocess

repo = Path('C:/projects/Nexo-i07c')
source = Path('C:/nexo-work/val/hunt-s4/tree/src/check/types.rs')
expected = 'd5046aedf42b6c14e8b4b4c0273402472b7a479fa942bb27cec2a25b34682ca6'
base = subprocess.check_output(
    ['git', 'show', '6394b67727334c479b700546d9d4b223da3c4943:src/check/types.rs'],
    cwd=repo,
)
assert hashlib.sha256(base).hexdigest() == expected
source.write_bytes(base)
assert hashlib.sha256(source.read_bytes()).hexdigest() == expected
print('Restored exact base blob', expected)
