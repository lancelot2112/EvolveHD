#!/bin/bash
# Exp 60: the byte brain (floor, guard, two maps, copy act) on the made-up formats stream. usage: T=tag BEXTRA= run.sh STREAM
cd /home/claude/evolvehd
S=${1:-runs/exp60/fmt}
EVOLVEHD_PROGRESS=100000 ${B:-runs/exp36/sit-60a} stack --data $S.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act $BEXTRA --out-dir runs/exp60/brain_$T > runs/exp60/brain_$T.out 2>&1
