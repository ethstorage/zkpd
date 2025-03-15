
# zkpd, zkp delegate

`zkpd` can be used to distribute zkp proving to multiple workers without leaking any privacy.

# Test

```bash

################ single process testing ################

########################################################
# test for polynomial evaluation in a single process
########################################################
cargo run
########################################################
# test for polynomial multiplication in a single process
########################################################
cargo run --bin poly

################ multiple process testing ##############

########################################################
# test for polynomial evaluation in multiple processes #
########################################################
# start 3 workers in 3 terminals
cargo run --bin p2p_scalar_worker  -- --index 1 --port 1080
cargo run --bin p2p_scalar_worker  -- --index 2 --port 1081
cargo run --bin p2p_scalar_worker  -- --index 3 --port 1082
# start delegator and kick off the work
cargo run --bin p2p_scalar_delegator  -- --workers "1@ws://127.0.0.1:1080" "2@ws://127.0.0.1:1081" "3@ws://127.0.0.1:1082"


```