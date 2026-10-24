#!/bin/bash
# BABILong qa2-qa5, frozen test read in full (predicting every word, so answers count even when the learned question
# detector does not mark "?" as a question)
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096 --sit-path-forget 2"
run() { t=$1; T=$2; SIT_TIMING=1 runs/exp36/sit-40e assoc --sit --data runs/exp40/bl_$t.ids --vocab runs/exp40/bl_$t.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-freeze-from $T --out runs/exp40/R_${t}_full 2> runs/exp40/R_${t}_full.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_$t runs/exp40/R_${t}_full > runs/exp40/R_${t}_full.score; }
run qa2 220618 & run qa4 138161; wait
while ps -eo args | grep -q "[R]_qa[35] "; do sleep 20; done
run qa3 619634 & run qa5 412405; wait
for t in qa2 qa3 qa4 qa5; do echo "== $t"; cat runs/exp40/R_${t}_full.score; grep -E "words,|question boundaries" runs/exp40/R_${t}_full.log; done
