#!/bin/bash
# Exp 47 (act path, stage 3): echo trials. After every 3rd fact sentence S the stream has "echo", S as self slots (the
# reader says each word and hears its own; paid and credited against the teacher's word), then S again from the world.
# Modes: normal (hears itself as self), --echo-deaf (does not hear itself), --echo-as-world (hears itself as if the
# world had said it). Exp 42's best stack with the act path (--sit-act).
cd /home/claude/evolvehd
B=${B:-runs/exp36/sit-47a}; F=$(sed "s/--sit-seed 0/--sit-seed ${SEED:-0}/" runs/exp37/F.flags); T=${T:-e}
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
G=${G:---sit-bnd-unit --sit-bnd-unit-len --sit-act}; M=${M:-}
story() { $B assoc --sit --data runs/exp47/story_echo.ids --vocab runs/exp47/story_echo.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F $X $G $M --out runs/exp47/R_story_$T 2> runs/exp47/R_story_$T.log; python3 scripts/exp47/score_echo.py runs/exp47/story_echo runs/exp47/R_story_$T 12329 > runs/exp47/R_story_$T.score; }
qs() { $B assoc --sit --data runs/exp47/qa2s_echo.ids --vocab runs/exp47/qa2s_echo.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 $G $M --sit-freeze-from 171039 --out runs/exp47/R_qa2s_$T 2> runs/exp47/R_qa2s_$T.log; python3 scripts/exp47/score_echo.py runs/exp47/qa2s_echo runs/exp47/R_qa2s_$T 102303 > runs/exp47/R_qa2s_$T.score; }
for f in "${@:-story}"; do $f; done
