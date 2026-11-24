. /c/nexo-work/val/mut-regiones/env.sh
cd /c/projects/Nexo-i07c-f
D=docs/validation/i07c-mut-regiones
until grep -q '"M00275"' $D/results-s5b1.jsonl; do
  python -B scripts/heavy-lock.py run --session i07c-mut-regiones-c -- python -B $D/run_mut.py /c/nexo-work/val/mut-regiones/tree $D/population-s5b1.json $D/results-s5b1.jsonl --ids M00275 >> /c/nexo-work/val/mut-regiones/rerun.log 2>&1
  sleep 20
done
echo done > /c/nexo-work/val/mut-regiones/rerun.done
