#!/bin/bash
# Controls for the long-BABILong 64k drop: budget + halving sketch alone, and budget + per-entity cap alone
cd /home/claude/evolvehd; B=runs/exp36/sit-41j
BIN=$B TAG=_bud17sd EXTRA="--sit-lt-budget 131072 --sit-sketch-decay" bash runs/exp38/bl_split.sh bl_long > /dev/null &
BIN=$B TAG=_bud17ef EXTRA="--sit-lt-budget 131072 --sit-entity-fair" bash runs/exp38/bl_split.sh bl_long > /dev/null; wait
for t in bud17sd bud17ef; do echo "== $t"; cat runs/exp38/R_bl_long_$t.score; grep -E "words,|cost per" runs/exp38/R_bl_long_$t.log | cut -c1-120; done
