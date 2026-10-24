#!/bin/bash
# exactness check: the same stack on an 80k-word prefix with two binaries; the .top files must be byte-identical
B=$1; O=$2
F=$(cat runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096 --sit-path-forget 2"
SIT_TIMING=1 $B assoc --sit --data runs/exp38/bl_pre80k.ids --vocab runs/exp38/bl_pre80k.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --out runs/exp38/$O 2> runs/exp38/$O.log
