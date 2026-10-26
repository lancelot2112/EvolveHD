#!/bin/bash
# Exp 51: why seeds differ (answer-source diagnostics).
cd /home/claude/evolvehd
B=${B:-runs/exp36/sit-51a}; F=$(sed "s/--sit-seed 0/--sit-seed ${SEED:-0}/" runs/exp37/F.flags); T=${T:-v}
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
G=${G:---sit-bnd-unit --sit-bnd-unit-len}
qs() { SIT_DEBUG_BND=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 $G --sit-freeze-from 102303 --out runs/exp51/R_qa2s_$T 2> runs/exp51/R_qa2s_$T.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp51/R_qa2s_$T > runs/exp51/R_qa2s_$T.score; }
full() { SIT_DEBUG_BND=1 SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2.ids --vocab runs/exp40/bl_qa2.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 --sit-entity-fair $G --sit-freeze-from ${FREEZE:-220618} --out runs/exp51/R_qa2_$T 2> runs/exp51/R_qa2_$T.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2 runs/exp51/R_qa2_$T > runs/exp51/R_qa2_$T.score; }
story() { SIT_DEBUG_BND=1 B=$B bash runs/exp36/rq30.sh "${T}_story $F $X $G"; }
pilot() { SIT_DEBUG_BND=1 BIN=$B TAG=_$T EXTRA="$G" bash runs/exp38/bl_split.sh bl_pilot > /dev/null; }
long() { SIT_DEBUG_BND=1 BIN=$B TAG=_$T EXTRA="--sit-lt-budget 131072 --sit-entity-fair $G" bash runs/exp38/bl_split.sh bl_long > /dev/null; }
for f in "${@:-qs}"; do $f; done
