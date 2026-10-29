#!/bin/bash
# Exp 63 step 3: the description expert inside the stack, on experiment 60's progressive formats stream, with and
# without it (both with the position sense, stride act off, binary sit-63c).
cd /home/claude/evolvehd
export B=runs/exp36/sit-63c
fm() { t=$1; shift; EVOLVEHD_PROGRESS=100000 $B stack --data runs/exp60/fmt2.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act --file-starts runs/exp60/fmt2.starts "$@" --out-dir runs/exp63/fmt_$t > runs/exp63/fmt_$t.out 2>&1; }
fm base & fm desc --desc-expert & wait
