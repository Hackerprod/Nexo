. /c/nexo-work/val/mut-regiones/env.sh
cd /c/projects/Nexo-i07c-f
D=docs/validation/i07c-mut-regiones
n() { [ -f $D/results-$1.jsonl ] && wc -l < $D/results-$1.jsonl || echo 0; }
while true; do
  [ -e /c/nexo-work/val/mut-regiones/STOP ] && break
  a=$(n s6cor); b=$(n s5b1)
  [ "$a" -ge 400 ] && [ "$b" -ge 400 ] && break
  if [ "$a" -le "$b" ] && [ "$a" -lt 400 ]; then c=s6cor; else c=s5b1; fi
  [ "$b" -ge 400 ] && c=s6cor
  t0=$(date +%s)
  python -B scripts/heavy-lock.py run --session i07c-mut-regiones-c -- python -B $D/run_mut.py /c/nexo-work/val/mut-regiones/tree $D/population-$c.json $D/results-$c.jsonl --max-seconds 400 >> /c/nexo-work/val/mut-regiones/loop-$c.log 2>&1
  t1=$(date +%s)
  if [ $((t1-t0)) -lt 8 ]; then sleep 30; else sleep 75; fi
done
echo FINISHED > /c/nexo-work/val/mut-regiones/loop.done
