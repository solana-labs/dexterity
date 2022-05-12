# Dexterity
## What is Dexterity
At a high level, Dexterity is a smart contract (or collection of smart contracts) that allow for the creation of a decentralized exchange on the Solana blockchain. The modular design of these contracts allows operators to create generic instruments with customizable risk and fee models.

## To Deploy
In order to deploy Dexterity, you will need to modify the mock keys in `master_program_config.json` to match the target program IDs. The build script will automatically fill in the keys into the program files.

If you would like to use the `deploy_all.sh` script. Paste the upgrade authority keypair for each of these contracts into the file `~/.config/solana/dexterity_shared.json`. Be sure to not commit this file.

## Build and Run Tests 
**Requirements:**
- rust nightly
- solana clis
- [Optional] solana-test-validator

First, install poetry for the python client (only needed the first time around):

```bash
cd client
poetry install
```

Then build and test the protocol

```bash
git submodule init
git submodule update
./build.sh
./test.sh
```

Note that if you use the `--replace` option in the `build.sh` script, you will need to explicitly call `cargo build-bpf` in the root directory to seed the target folder with program keypair files.
