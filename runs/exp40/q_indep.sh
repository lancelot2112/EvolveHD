#!/bin/bash
# the independence veto on the standard checks, then qa2-qa5 with chains and the budget store
cd /home/claude/evolvehd; F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-40l
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
B=$B bash runs/exp36/rq30.sh "ind_story $F $X --sit-bnd-indep"; tail -1 runs/exp36/R_ind_story.score; grep "learned boundaries" runs/exp36/R_ind_story.log
BIN=$B TAG=_ind EXTRA="--sit-bnd-indep" bash runs/exp38/bl_split.sh bl_pilot; grep -E "learned boundaries|question boundaries|words," runs/exp38/R_bl_pilot_ind.log | cut -c1-100
run() { t=$1; T=$2; SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_$t.ids --vocab runs/exp40/bl_$t.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-bnd-indep --sit-lt-budget 131072 --sit-chain --sit-freeze-from $T --out runs/exp40/R_${t}_ic 2> runs/exp40/R_${t}_ic.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_$t runs/exp40/R_${t}_ic > runs/exp40/R_${t}_ic.score; }
run qa2 220618 & run qa4 138161; wait
run qa3 619634 & run qa5 412405; wait
for t in qa2 qa3 qa4 qa5; do echo "== $t"; cat runs/exp40/R_${t}_ic.score; grep -E "words,|learned boundaries" runs/exp40/R_${t}_ic.log | cut -c1-100; done
