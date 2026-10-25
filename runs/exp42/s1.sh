#!/bin/bash
# Surprise proposals (--sit-bnd-surprise) disposed by the clause judge (--sit-bnd-unit --sit-bnd-unit-len), no hand
# ban: small qa2, story set, pilot (seed 0)
cd /home/claude/evolvehd
B=runs/exp36/sit-42a; F=$(cat runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
U="--sit-bnd-surprise --sit-bnd-unit --sit-bnd-unit-len"
(SIT_DEBUG_BND=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 $U --sit-freeze-from 102303 --out runs/exp42/R_qa2s_s1 2> runs/exp42/R_qa2s_s1.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp42/R_qa2s_s1 > runs/exp42/R_qa2s_s1.score) &
B=$B bash runs/exp36/rq30.sh "s1_story $F $X $U"; wait
BIN=$B TAG=_s1 EXTRA="$U" bash runs/exp38/bl_split.sh bl_pilot > /dev/null
s() { grep -E "words,|learned boundaries|binding stops|question boundaries" $1 | cut -c1-250; grep "boundary unit" $1 | cut -c1-400; grep -c "^UNIT" $1; }
echo "== qa2s"; cat runs/exp42/R_qa2s_s1.score; s runs/exp42/R_qa2s_s1.log
echo "== story"; tail -1 runs/exp36/R_s1_story.score; s runs/exp36/R_s1_story.log
echo "== pilot"; cat runs/exp38/R_bl_pilot_s1.score; s runs/exp38/R_bl_pilot_s1.log
