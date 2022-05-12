#!/bin/bash

function deploy() {
    echo "Deploying $1..."
    solana program deploy target/deploy/$1.so -u $2 --upgrade-authority ~/.config/solana/dexterity_shared.json --keypair ~/.config/solana/dexterity_shared.json
}

deploy dex $1
deploy instruments $1
deploy dummy_oracle $1
deploy noop_risk_engine $1
deploy alpha_risk_engine $1
deploy constant_fees $1
deploy agnostic_orderbook $1
