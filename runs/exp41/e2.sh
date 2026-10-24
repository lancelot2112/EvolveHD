#!/bin/bash
# E2 (forum round six, step 2): boundaries judged by use (--sit-bnd-use), on the halving sketch; no hand ban
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=${B:-runs/exp36/sit-41e}
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
E="--sit-bnd-use --sit-sketch-decay"
qs() { tag=$1; shift; SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 "$@" --sit-freeze-from 102303 --out runs/exp41/R_qa2s_$tag 2> runs/exp41/R_qa2s_$tag.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp41/R_qa2s_$tag > runs/exp41/R_qa2s_$tag.score; }
qs e2 $E &
B=$B bash runs/exp36/rq30.sh "e2_story $F $X $E"; wait
BIN=$B TAG=_e2 EXTRA="$E" bash runs/exp38/bl_split.sh bl_pilot > /dev/null
echo "== qa2s e2"; cat runs/exp41/R_qa2s_e2.score; grep -E "words,|learned boundaries|binding stops|boundary use|question boundaries" runs/exp41/R_qa2s_e2.log | cut -c1-200
echo "== story"; tail -1 runs/exp36/R_e2_story.score; grep -E "words,|learned boundaries|binding stops|boundary use|question boundaries" runs/exp36/R_e2_story.log | cut -c1-200
echo "== pilot"; cat runs/exp38/R_bl_pilot_e2.score; grep -E "words,|learned boundaries|binding stops|boundary use|question boundaries" runs/exp38/R_bl_pilot_e2.log | cut -c1-200
