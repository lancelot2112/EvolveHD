#!/bin/bash
# G1 (forum eight, the recurrent seat): surprise chunks replace the learned boundary set in every job. Binding windows
# (level 0 within the open and previous chunk, level 1 chunk heads across 8 chunks), commits and drift at chunk close,
# the cue on the last 3 chunks, range texts on chunks, the write gate instead of the question drop, the contrast
# detector without L for answering (answer-only runs) and head credit. L is never computed. No hand ban.
cd /home/claude/evolvehd
B=${B:-runs/exp36/sit-44a}; F=$(cat runs/exp37/F.flags); T=${T:-g1}
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
G=${G:---sit-chunk --sit-chunk-gate --sit-q-contrast --sit-entity-fair}
qs() { SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 $G --sit-freeze-from 102303 --out runs/exp44/R_qa2s_$T 2> runs/exp44/R_qa2s_$T.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp44/R_qa2s_$T > runs/exp44/R_qa2s_$T.score; }
full() { SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2.ids --vocab runs/exp40/bl_qa2.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 $G --sit-freeze-from 220618 --out runs/exp44/R_qa2_$T 2> runs/exp44/R_qa2_$T.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2 runs/exp44/R_qa2_$T > runs/exp44/R_qa2_$T.score; }
story() { B=$B bash runs/exp36/rq30.sh "${T}_story $F $X $G"; }
pilot() { BIN=$B TAG=_$T EXTRA="$G" bash runs/exp38/bl_split.sh bl_pilot > /dev/null; }
long() { BIN=$B TAG=_$T EXTRA="--sit-lt-budget 131072 $G" bash runs/exp38/bl_split.sh bl_long > /dev/null; }
"${@:-qs}"
