#!/bin/bash
cd /home/claude/evolvehd
jobs=(); for s in 0 1 2; do for S in qs st; do for P in base names verb question longer all; do jobs+=("$S $P $s"); done; done; done
printf '%s\n' "${jobs[@]}" | xargs -P 2 -L 1 bash runs/exp55/p.sh
