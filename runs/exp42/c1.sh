#!/bin/bash
# TEM clean-slot ledger as the clause judge (--sit-bnd-clean): A disposes today's k90 proposals (start as stops);
# B disposes forum five's surprise-peak proposals from an all-crossed start (the bootstrapping test). No hand ban.
cd /home/claude/evolvehd
B=runs/exp36/sit-42f; F=$(cat runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
A="--sit-bnd-unit --sit-bnd-clean"; P="--sit-bnd-surprise --sit-bnd-peak --sit-bnd-unit --sit-bnd-clean"
qs() { tag=$1; shift; SIT_DEBUG_BND=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 "$@" --sit-freeze-from 102303 --out runs/exp42/R_qa2s_$tag 2> runs/exp42/R_qa2s_$tag.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp42/R_qa2s_$tag > runs/exp42/R_qa2s_$tag.score; }
qs cA $A & qs cB $P; wait
B=$B bash runs/exp36/rq30.sh "cA_story $F $X $A" "cB_story $F $X $P"
BIN=$B TAG=_cA EXTRA="$A" bash runs/exp38/bl_split.sh bl_pilot > /dev/null &
BIN=$B TAG=_cB EXTRA="$P" bash runs/exp38/bl_split.sh bl_pilot > /dev/null; wait
s() { grep -E "words,|learned boundaries|binding stops|question boundaries" $1 | cut -c1-250; grep "boundary unit" $1 | cut -c1-350; grep "^UNIT" $1 | head -10 | tr '\n' ';'; echo; }
for t in cA cB; do echo "== qa2s $t"; cat runs/exp42/R_qa2s_$t.score; s runs/exp42/R_qa2s_$t.log; echo "== story $t"; tail -1 runs/exp36/R_${t}_story.score; s runs/exp36/R_${t}_story.log; echo "== pilot $t"; cat runs/exp38/R_bl_pilot_$t.score; s runs/exp38/R_bl_pilot_$t.log; done
