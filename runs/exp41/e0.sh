#!/bin/bash
# E0 (forum round six): split the boundary set's jobs (--sit-bind-indep) with the mood floor; no hand ban on "the"
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-41b
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
E="--sit-bind-indep --sit-mood-floor"
qs() { tag=$1; shift; SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 "$@" --sit-freeze-from 102303 --out runs/exp41/R_qa2s_$tag 2> runs/exp41/R_qa2s_$tag.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp41/R_qa2s_$tag > runs/exp41/R_qa2s_$tag.score; }
qs base & qs e0 $E; wait
B=$B bash runs/exp36/rq30.sh "e0_story $F $X $E" &
BIN=$B TAG=_e0 EXTRA="$E" bash runs/exp38/bl_split.sh bl_pilot > /dev/null; wait
for t in base e0; do echo "== qa2s $t"; cat runs/exp41/R_qa2s_$t.score; grep -E "learned boundaries|question boundaries" runs/exp41/R_qa2s_$t.log | cut -c1-120; done
echo "== story"; tail -1 runs/exp36/R_e0_story.score; grep -E "learned boundaries|question boundaries" runs/exp36/R_e0_story.log | cut -c1-120
echo "== pilot"; cat runs/exp38/R_bl_pilot_e0.score; grep -E "learned boundaries|question boundaries" runs/exp38/R_bl_pilot_e0.log | cut -c1-120
