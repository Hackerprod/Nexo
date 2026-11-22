#!/bin/sh
# uso: verdict.sh ID
. /c/nexo-work/val/s6mut/env.sh
cd /c/nexo-work/val/s6mut/tree
python -B scripts/heavy-lock.py run --session i07c-s6-mut -- python -B scripts/mutant-verdict.py --tree C:/nexo-work/val/s6mut/tree --patch D:/nexo-work/mut-s3/s6mut/$1.patch.json --id $1 --output D:/nexo-work/mut-s3/s6mut/$1.verdict.json --timeout 1800
