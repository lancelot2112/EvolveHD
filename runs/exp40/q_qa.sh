#!/bin/bash
# BABILong qa2-qa5, each its own stream (train 0k, 1k or 2k; test 0k and 4k frozen, answer-only), today's stack
cd /home/claude/evolvehd
while ps -eo args | grep -q "[R]_bl_long_.*bud17"; do sleep 20; done
F=$(cat runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096 --sit-path-forget 2"
run() { t=$1; T=$2; SIT_TIMING=1 runs/exp36/sit-40e assoc --sit --data runs/exp40/bl_$t.ids --vocab runs/exp40/bl_$t.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-answer-only $T --sit-freeze-from $T --out runs/exp40/R_$t 2> runs/exp40/R_$t.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_$t runs/exp40/R_$t > runs/exp40/R_$t.score; }
run qa2 220618 & run qa4 138161; wait
run qa3 619634 & run qa5 412405; wait
for t in qa2 qa3 qa4 qa5; do echo "== $t"; cat runs/exp40/R_$t.score; grep "words," runs/exp40/R_$t.log; done
