#!/bin/bash
# F1 with length-weighted clause hits (--sit-bnd-unit-len): the full qa2 stream (where F1 put "the" back at word 170k,
# in book text) and the three small checks, seed 0
cd /home/claude/evolvehd
B=runs/exp36/sit-41q; F=$(cat runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
U="--sit-bnd-unit --sit-bnd-unit-len"
(SIT_DEBUG_BND=1 SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2.ids --vocab runs/exp40/bl_qa2.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 --sit-entity-fair $U --sit-freeze-from 220618 --out runs/exp41/R_qa2_f1lef 2> runs/exp41/R_qa2_f1lef.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2 runs/exp41/R_qa2_f1lef > runs/exp41/R_qa2_f1lef.score) &
SIT_DEBUG_BND=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 $U --sit-freeze-from 102303 --out runs/exp41/R_qa2s_f1l 2> runs/exp41/R_qa2s_f1l.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp41/R_qa2s_f1l > runs/exp41/R_qa2s_f1l.score
B=$B bash runs/exp36/rq30.sh "f1l_story $F $X $U"
BIN=$B TAG=_f1l EXTRA="$U" bash runs/exp38/bl_split.sh bl_pilot > /dev/null; wait
s() { grep -E "words,|binding stops|boundary unit" $1 | cut -c1-200; grep "^UNIT" $1 | head -8 | tr '\n' ';'; echo; }
echo "== qa2s"; cat runs/exp41/R_qa2s_f1l.score; s runs/exp41/R_qa2s_f1l.log
echo "== story"; tail -1 runs/exp36/R_f1l_story.score; s runs/exp36/R_f1l_story.log
echo "== pilot"; cat runs/exp38/R_bl_pilot_f1l.score; s runs/exp38/R_bl_pilot_f1l.log
echo "== full qa2"; cat runs/exp41/R_qa2_f1lef.score; s runs/exp41/R_qa2_f1lef.log
