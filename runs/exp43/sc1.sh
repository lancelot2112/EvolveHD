#!/bin/bash
# The cheap fractal test (--sit-scales): no binding stops; exact-path bindings up to 8 back, band bindings 9-32 back
# between content words. No hand ban, no boundary judge. Small qa2, story set, pilot, then full qa2 with the cap.
cd /home/claude/evolvehd
B=runs/exp36/sit-43a; F=$(cat runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
S="--sit-scales"
(SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 --sit-entity-fair $S --sit-freeze-from 102303 --out runs/exp43/R_qa2s_sc 2> runs/exp43/R_qa2s_sc.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp43/R_qa2s_sc > runs/exp43/R_qa2s_sc.score) &
B=$B bash runs/exp36/rq30.sh "sc_story $F $X $S"; wait
BIN=$B TAG=_sc EXTRA="$S" bash runs/exp38/bl_split.sh bl_pilot > /dev/null
s() { grep -E "words,|learned boundaries|question boundaries|cost per" $1 | cut -c1-200; }
echo "== qa2s"; cat runs/exp43/R_qa2s_sc.score; s runs/exp43/R_qa2s_sc.log
echo "== story"; tail -1 runs/exp36/R_sc_story.score; s runs/exp36/R_sc_story.log
echo "== pilot"; cat runs/exp38/R_bl_pilot_sc.score; s runs/exp38/R_bl_pilot_sc.log
SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2.ids --vocab runs/exp40/bl_qa2.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 --sit-entity-fair $S --sit-freeze-from 220618 --out runs/exp43/R_qa2_sc 2> runs/exp43/R_qa2_sc.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2 runs/exp43/R_qa2_sc > runs/exp43/R_qa2_sc.score
echo "== full qa2"; cat runs/exp43/R_qa2_sc.score; s runs/exp43/R_qa2_sc.log; grep -E "^TIMING|^HEADS" runs/exp43/R_qa2_sc.log | tail -2 | cut -c1-200
