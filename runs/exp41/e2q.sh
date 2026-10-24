#!/bin/bash
# Boundaries judged from the level above (--sit-bnd-use-q): only answers (after a Q word, contrast detector) credit or
# blame the proposed words their route crossed. No hand ban on "the". Compare the qc2 runs (e3b.sh).
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-41o
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
E="--sit-q-contrast --sit-bnd-use --sit-bnd-use-q --sit-sketch-decay"
(SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 $E --sit-freeze-from 102303 --out runs/exp41/R_qa2s_e2q 2> runs/exp41/R_qa2s_e2q.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp41/R_qa2s_e2q > runs/exp41/R_qa2s_e2q.score) &
B=$B bash runs/exp36/rq30.sh "e2q_story $F $X $E"; wait
BIN=$B TAG=_e2q EXTRA="$E" bash runs/exp38/bl_split.sh bl_pilot > /dev/null
s() { grep -E "words,|binding stops|boundary use \(|question boundaries" $1 | cut -c1-200; }
echo "== qa2s e2q"; cat runs/exp41/R_qa2s_e2q.score; s runs/exp41/R_qa2s_e2q.log
echo "== story e2q"; tail -1 runs/exp36/R_e2q_story.score; s runs/exp36/R_e2q_story.log
echo "== pilot e2q"; cat runs/exp38/R_bl_pilot_e2q.score; s runs/exp38/R_bl_pilot_e2q.log
