#!/bin/bash
# Clean-slot judge, clauses include their opening stop (sit-42g): A2 disposes k90 proposals; B2 disposes surprise-peak
# proposals starting as stops (the judge removes); B3 the same starting crossed. No hand ban.
cd /home/claude/evolvehd
B=runs/exp36/sit-42g; F=$(cat runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
A="--sit-bnd-unit --sit-bnd-clean"; P="--sit-bnd-surprise --sit-bnd-peak --sit-bnd-unit --sit-bnd-clean"
qs() { tag=$1; shift; SIT_DEBUG_BND=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 "$@" --sit-freeze-from 102303 --out runs/exp42/R_qa2s_$tag 2> runs/exp42/R_qa2s_$tag.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp42/R_qa2s_$tag > runs/exp42/R_qa2s_$tag.score; }
qs cA2 $A & SIT_PROPOSE_STOP=1 qs cB2 $P; wait
B=$B bash runs/exp36/rq30.sh "cA2_story $F $X $A"
SIT_PROPOSE_STOP=1 B=$B bash runs/exp36/rq30.sh "cB2_story $F $X $P"
BIN=$B TAG=_cA2 EXTRA="$A" bash runs/exp38/bl_split.sh bl_pilot > /dev/null &
SIT_PROPOSE_STOP=1 BIN=$B TAG=_cB2 EXTRA="$P" bash runs/exp38/bl_split.sh bl_pilot > /dev/null; wait
s() { grep -E "words,|learned boundaries|binding stops|question boundaries" $1 | cut -c1-250; grep "boundary unit" $1 | cut -c1-350; grep "^UNIT" $1 | head -10 | tr '\n' ';'; echo; }
for t in cA2 cB2; do echo "== qa2s $t"; cat runs/exp42/R_qa2s_$t.score; s runs/exp42/R_qa2s_$t.log; echo "== story $t"; tail -1 runs/exp36/R_${t}_story.score; s runs/exp36/R_${t}_story.log; echo "== pilot $t"; cat runs/exp38/R_bl_pilot_$t.score; s runs/exp38/R_bl_pilot_$t.log; done
