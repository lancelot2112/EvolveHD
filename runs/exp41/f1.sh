#!/bin/bash
# F1 (forum round seven): boundaries disposed by the clause level (--sit-bnd-unit), no hand ban; alone, and with the
# question guards (contrast detector within L, containment drop with question isolation)
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-41p
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
U="--sit-bnd-unit"; G="--sit-bnd-unit --sit-q-contrast --sit-q-in-l --sit-q-contain"
qs() { tag=$1; shift; SIT_DEBUG_BND=1 SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 "$@" --sit-freeze-from 102303 --out runs/exp41/R_qa2s_$tag 2> runs/exp41/R_qa2s_$tag.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp41/R_qa2s_$tag > runs/exp41/R_qa2s_$tag.score; }
qs f1u $U & qs f1g $G; wait
B=$B bash runs/exp36/rq30.sh "f1u_story $F $X $U" "f1g_story $F $X $G"
BIN=$B TAG=_f1u EXTRA="$U" bash runs/exp38/bl_split.sh bl_pilot > /dev/null &
BIN=$B TAG=_f1g EXTRA="$G" bash runs/exp38/bl_split.sh bl_pilot > /dev/null; wait
s() { grep -E "words,|learned boundaries|binding stops|boundary unit|question boundaries" $1 | cut -c1-200; grep "^UNIT" $1 | head -8 | tr '\n' ';'; echo; }
for t in f1u f1g; do echo "== qa2s $t"; cat runs/exp41/R_qa2s_$t.score; s runs/exp41/R_qa2s_$t.log; echo "== story $t"; tail -1 runs/exp36/R_${t}_story.score; s runs/exp36/R_${t}_story.log; echo "== pilot $t"; cat runs/exp38/R_bl_pilot_$t.score; s runs/exp38/R_bl_pilot_$t.log; done
