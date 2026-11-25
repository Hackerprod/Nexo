cd /c/projects/Nexo-i07c-f
M=/c/nexo-work/val/mut-regiones
while [ ! -e $M/hunt.done ]; do
  python -B scripts/heavy-lock.py run --session i07c-mut-regiones-c -- sh $M/hunt_chunk.sh >> $M/hunt-run.log 2>&1
  sleep 160
done
