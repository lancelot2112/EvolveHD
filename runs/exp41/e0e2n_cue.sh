#!/bin/bash
# E0 and the normalized E2 again, with the cue walk back on the learned boundary set (sit-41k); binding stops only
# change the binding walk
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-41k
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
qs() { tag=$1; shift; SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 "$@" --sit-freeze-from 102303 --out runs/exp41/R_qa2s_$tag 2> runs/exp41/R_qa2s_$tag.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp41/R_qa2s_$tag > runs/exp41/R_qa2s_$tag.score; }
E0="--sit-bind-indep --sit-mood-floor"; EN="--sit-bnd-use --sit-bnd-use-norm --sit-sketch-decay"
qs e0c $E0 & qs e2nc $EN; wait
B=$B bash runs/exp36/rq30.sh "e0c_story $F $X $E0" "e2nc_story $F $X $EN"
BIN=$B TAG=_e0c EXTRA="$E0" bash runs/exp38/bl_split.sh bl_pilot > /dev/null &
BIN=$B TAG=_e2nc EXTRA="$EN" bash runs/exp38/bl_split.sh bl_pilot > /dev/null; wait
s() { grep -E "binding stops|boundary use|question boundaries" $1 | cut -c1-200; }
for t in e0c e2nc; do echo "== qa2s $t"; cat runs/exp41/R_qa2s_$t.score; s runs/exp41/R_qa2s_$t.log; echo "== story $t"; tail -1 runs/exp36/R_${t}_story.score; s runs/exp36/R_${t}_story.log; echo "== pilot $t"; cat runs/exp38/R_bl_pilot_$t.score; s runs/exp38/R_bl_pilot_$t.log; done
