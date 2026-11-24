M=/c/nexo-work/val/mut-regiones
cd /c/projects/Nexo-i07c-f
until [ -e $M/dbgall2.done ]; do
  python -B scripts/heavy-lock.py run --session i07c-mut-regiones-c -- sh $M/dbgall2.sh >> $M/dbgall2.out 2>&1
  sleep 20
done
