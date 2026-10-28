#!/bin/bash
# Exp 58: the byte brain on plain text (three Austen novels, 2 MB), to check the floor, the guard and the map cap
# cost nothing where nothing is noise. usage: TAG=t BEXTRA="flags" text.sh
cd /home/claude/evolvehd
EVOLVEHD_PROGRESS=200000 ${B:-runs/exp36/sit-58b} stack --data runs/austen3.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions $BEXTRA --out-dir runs/exp57/text_$TAG > runs/exp57/text_$TAG.out 2>&1
