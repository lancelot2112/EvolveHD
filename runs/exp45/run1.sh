#!/bin/bash
cd /home/claude/evolvehd
B=runs/exp36/sit-45a
J="--sit-bnd-unit --sit-bnd-unit-len"
(T=x45nd G="$J --sit-no-qdrop" bash runs/exp45/v.sh story qs pilot; T=x45dv G="$J --sit-validate" bash runs/exp45/v.sh story qs pilot) &
T=x45ndv G="$J --sit-no-qdrop --sit-validate" bash runs/exp45/v.sh story qs pilot
T=x45g2q G="--sit-bnd-collide $J --sit-q-contrast" bash runs/exp45/v.sh story qs pilot
wait
