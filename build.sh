#!/bin/bash
DEPLOY=0
INIT=0
REPLACE=0

if [[ $1 == "devnet" || $1 == "mainnet-beta" || $1 == "localhost" ]]
then
    NETWORK=$1
    shift
else
    NETWORK="localhost"
fi

while :; do
    case $1 in
        -i|--init)
            echo "Initialize IDL"
            INIT=1
        ;;
        -d|--deploy)
            echo "Deploy programs"
            DEPLOY=1
        ;;
        -r|--replace)
            echo "Replace program IDs"
            REPLACE=1
        ;;
        *)
            break 
    esac
    shift
done

ROOT=$(git rev-parse --show-toplevel)
cd $ROOT
# You should seed the program ids by calling `cargo build-bpf` in the root directory the first time
function fill_program_id() {
    if [[ "$OSTYPE" == "darwin"* ]]
    then 
        echo "Program ID for $2 is $1"
        sed -i '' 's/declare_id!(.*/declare_id!("'"$1"'");/g' $2
    else
        echo "Program ID for $2 is $1"
        sed -i 's/declare_id!(.*/declare_id!("'"$1"'");/g' $2
    fi
}
cd programs/agnostic-orderbook
git checkout e37a307
cd ../..
cargo fmt -p dex instruments dummy-oracle noop-risk-engine dex-macros constant-fees alpha-risk-engine
# Input keypair begins here

if [[ $REPLACE == 1 ]]
then 
    dex_pid=`solana-keygen pubkey target/deploy/dex-keypair.json`
    inst_pid=`solana-keygen pubkey target/deploy/instruments-keypair.json`
    nop_pid=`solana-keygen pubkey target/deploy/noop_risk_engine-keypair.json`
    alpha_pid=`solana-keygen pubkey target/deploy/alpha_risk_engine-keypair.json`
    dummy_pid=`solana-keygen pubkey target/deploy/dummy_oracle-keypair.json`
    fees_pid=`solana-keygen pubkey target/deploy/constant_fees-keypair.json`

    ## Dex
    fill_program_id $dex_pid programs/dex/src/lib.rs

    ## Instruments
    fill_program_id $inst_pid  programs/instruments/src/lib.rs

    ## Noop Risk Engine
    fill_program_id $nop_pid programs/risk/noop-risk-engine/src/lib.rs

    ## Alpha Risk Engine
    fill_program_id $alpha_pid programs/risk/alpha-risk-engine/src/lib.rs

    ## Dummy Oracle
    fill_program_id $dummy_pid programs/dummy-oracle/src/lib.rs

    ## Constant Fees
    fill_program_id $fees_pid programs/fees/constant-fees/src/lib.rs
else
    dex_pid=`cat master_program_config.json | jq .programs.dex | tr -d '"'`
    inst_pid=`cat master_program_config.json | jq .programs.instruments| tr -d '"'`
    nop_pid=`cat master_program_config.json | jq .programs.noop_risk_engine| tr -d '"'`
    alpha_pid=`cat master_program_config.json | jq .programs.alpha_risk_engine| tr -d '"'`
    dummy_pid=`cat master_program_config.json | jq .programs.dummy_oracle| tr -d '"'`
    fees_pid=`cat master_program_config.json | jq .programs.constant_fees| tr -d '"'`
    ## Dex
    fill_program_id $dex_pid programs/dex/src/lib.rs
    ## Instruments
    fill_program_id $inst_pid  programs/instruments/src/lib.rs
    ## Noop Risk Engine
    fill_program_id $nop_pid programs/risk/noop-risk-engine/src/lib.rs
    ## Alpha Risk Engine
    fill_program_id $alpha_pid programs/risk/alpha-risk-engine/src/lib.rs
    ## Dummy Oracle
    fill_program_id $dummy_pid programs/dummy-oracle/src/lib.rs
    ## Constant Fees
    fill_program_id $fees_pid programs/fees/constant-fees/src/lib.rs
fi

mkdir -p target/idl && mkdir -p target/types
cargo build-bpf
# Input keypair ends
anchor idl parse -f programs/dex/src/lib.rs -o target/idl/dex.json -t target/types/dex.ts
anchor idl parse -f programs/instruments/src/lib.rs -o target/idl/instruments.json -t target/types/instruments.ts
anchor idl parse -f programs/risk/noop-risk-engine/src/lib.rs -o target/idl/noop_risk_engine.json -t target/types/noop_risk_engine.ts
anchor idl parse -f programs/risk/alpha-risk-engine/src/lib.rs -o target/idl/alpha_risk_engine.json -t target/types/alpha_risk_engine.ts

# Update on chain IDL
if [[ $NETWORK == "devnet" || $NETWORK == "mainnet-beta" ]]
then
    if [[ $DEPLOY == 1 ]]
    then
        echo "Deploying to $1"
        $ROOT/deploy_all.sh $1
    fi
    if [[ $INIT == 1 ]]
    then
        echo "Initializing IDL on $1"
        anchor idl init $dex_pid -f $ROOT/target/idl/dex.json  --provider.cluster $1 --provider.wallet deploy_key.json
        anchor idl init $inst_pid -f $ROOT/target/idl/instruments.json --provider.cluster $1 --provider.wallet deploy_key.json
        anchor idl init $nop_pid -f $ROOT/target/idl/noop_risk_engine.json --provider.cluster $1 --provider.wallet deploy_key.json
        anchor idl init $alpha_pid -f $ROOT/target/idl/alpha_risk_engine.json --provider.cluster $1 --provider.wallet deploy_key.json
    else
        echo "Upgrading IDL on $1"
        anchor idl upgrade $dex_pid -f $ROOT/target/idl/dex.json  --provider.cluster $1 --provider.wallet deploy_key.json
        anchor idl upgrade $inst_pid -f $ROOT/target/idl/instruments.json --provider.cluster $1 --provider.wallet deploy_key.json
        anchor idl upgrade $nop_pid -f $ROOT/target/idl/noop_risk_engine.json --provider.cluster $1 --provider.wallet deploy_key.json
        anchor idl upgrade $alpha_pid -f $ROOT/target/idl/alpha_risk_engine.json --provider.cluster $1 --provider.wallet deploy_key.json
    fi
fi

# run `poetry install` within client folder first
cd client
poetry run generate-code
