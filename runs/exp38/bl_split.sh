#!/bin/bash
# BABILong qa1 long test, split into two streams (same training prefix) so they run on two cores in parallel.
D=$1  # bl_l16 or bl_l64
F=$(cat runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096 --sit-path-forget 2"
SIT_TIMING=1 ${BIN:-runs/exp36/sit-38x} assoc --sit --data runs/exp38/$D.ids --vocab runs/exp38/$D.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-answer-only 156952 --sit-freeze-from 156952 --out runs/exp38/R_$D 2> runs/exp38/R_$D.log
python3 scripts/exp38/score_babilong.py runs/exp38/$D runs/exp38/R_$D > runs/exp38/R_$D.score
cat runs/exp38/R_$D.score
