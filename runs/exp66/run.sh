#!/bin/bash
# Exp 66: real files after real manuals. Four rotations (real0..3), alone and after exp 60's synthetic stream (prereal0..3).
# Arms: base (no expert), desc (the expert, guessing the role), role (the role gate taught against both the decoy and
# the background, exp 65's best), oracle (told the description segments). Exp 63-65's flags; binary sit-65d.
cd /home/claude/evolvehd
export B=runs/exp36/sit-65d
one() {
  st=$1; arm=$2; d=runs/exp66/${st}_$arm
  [ -s $d/report.txt ] && return
  case $arm in
    base) x="" ;;
    desc) x="--desc-expert" ;;
    role) x="--desc-expert --desc-role --desc-role-contrast --desc-role-null 2" ;;
    oracle) x="--desc-expert --desc-oracle runs/exp66/$st.descs" ;;
  esac
  ROLE_DUMP=1 $B stack --data runs/exp66/$st.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act --file-starts runs/exp66/$st.starts $x --out-dir $d > $d.out 2> $d.err
  grep "^ROLE" $d.err > $d.dump; grep -v "^ROLE" $d.err > $d.log; rm $d.err
}
export -f one
for p in "" pre; do for s in 0 1 2 3; do for a in base desc role oracle; do echo "${p}real$s $a"; done; done; done | xargs -P 2 -L 1 bash -c 'one $0 $1'
