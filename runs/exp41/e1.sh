#!/bin/bash
# E1 (forum round six): the halving path sketch on the full qa2 stream with chains (compare runs/exp40/R_qa2_chain),
# and with chains + the budget store
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-41a
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096 --sit-path-forget 2"
run() { t=$1; T=$2; tag=$3; shift 3; SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_$t.ids --vocab runs/exp40/bl_$t.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-chain "$@" --sit-freeze-from $T --out runs/exp41/R_${t}_$tag 2> runs/exp41/R_${t}_$tag.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_$t runs/exp41/R_${t}_$tag > runs/exp41/R_${t}_$tag.score; }
run qa2 220618 sd --sit-sketch-decay &
run qa2 220618 sdbud --sit-sketch-decay --sit-lt-budget 131072 &
wait
