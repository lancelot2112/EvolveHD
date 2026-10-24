#!/bin/bash
# E2 with the normalized judge (--sit-bnd-use-norm)
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-41j
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
E="--sit-bnd-use --sit-bnd-use-norm --sit-sketch-decay"
qs() { tag=$1; shift; SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_qa2s.ids --vocab runs/exp40/bl_qa2s.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 "$@" --sit-freeze-from 102303 --out runs/exp41/R_qa2s_$tag 2> runs/exp41/R_qa2s_$tag.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_qa2s runs/exp41/R_qa2s_$tag > runs/exp41/R_qa2s_$tag.score; }
qs e2n $E &
B=$B bash runs/exp36/rq30.sh "e2n_story $F $X $E"; wait
BIN=$B TAG=_e2n EXTRA="$E" bash runs/exp38/bl_split.sh bl_pilot > /dev/null &
wait
s() { grep -E "words,|binding stops|boundary use|question boundaries" $1 | cut -c1-220; }
echo "== qa2s e2n"; cat runs/exp41/R_qa2s_e2n.score; s runs/exp41/R_qa2s_e2n.log
for n in e2n_story; do echo "== $n"; tail -1 runs/exp36/R_$n.score; s runs/exp36/R_$n.log; done
for n in e2n; do echo "== pilot $n"; cat runs/exp38/R_bl_pilot_$n.score; s runs/exp38/R_bl_pilot_$n.log; done
