#!/bin/bash
# Exp 48, stage 4: report requests ("report NP" -> the reader says "NP is in the ANS ."), then the world says it.
cd /home/claude/evolvehd
B=${B:-runs/exp36/sit-49a}; F=$(sed "s/--sit-seed 0/--sit-seed ${SEED:-0}/" runs/exp37/F.flags); T=${T:-r}
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
G=${G:---sit-bnd-unit --sit-bnd-unit-len --sit-act --sit-seq-replay}; M=${M:-}
story() { $B assoc --sit --data runs/exp48/story_rep.ids --vocab runs/exp48/story_rep.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F $X $G $M --out runs/exp49/R_story_$T 2> runs/exp49/R_story_$T.log; python3 scripts/exp48/score_report.py runs/exp48/story_rep runs/exp49/R_story_$T 12329 > runs/exp49/R_story_$T.score; }
qs() { $B assoc --sit --data runs/exp48/qa2s_rep.ids --vocab runs/exp48/qa2s_rep.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 $G $M --sit-freeze-from 107303 --out runs/exp49/R_qa2s_$T 2> runs/exp49/R_qa2s_$T.log; python3 scripts/exp48/score_report.py runs/exp48/qa2s_rep runs/exp49/R_qa2s_$T 102303 > runs/exp49/R_qa2s_$T.score; }
for f in "${@:-story}"; do $f; done
