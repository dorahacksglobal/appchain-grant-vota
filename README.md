# Appchain Voting Grant on Vota

Appchain Voting Grant on Vota

For previous EVM implementations refer to this [repo](https://github.com/dorahacksglobal/qf-grant-contract/tree/bsc-long-term).

## Quick Start

[Setup Rust](https://rustup.rs/)

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
```

Run tests

```sh
cargo test
```

## Scripts Entry

### initialize

Initialize the contract.

### batch_vote

Vote to a project which you like.

### end_round

Only owenr of round can end a round and withdraw fees.

## Publish

build

```sh
cargo wasm
```

optimize

```sh
cargo run-script optimize
```

check

```sh
cargo run-script check
```

deploy

```sh
archwayd tx wasm store ./artifacts/appchain-grant-vota.wasm --gas auto --gas-prices $(archwayd q rewards estimate-fees 1 --node 'https://rpc.constantine.archway.tech:443' --output json | jq -r '.gas_unit_price | (.amount + .denom)') --gas-adjustment 1.4 --from test-key --chain-id constantine-3 --node https://rpc.constantine.archway.tech:443 --broadcast-mode sync --output json -y
```

Generate Typescript SDK

```sh
cargo schema

cosmwasm-ts-codegen generate \
--plugin client \
--schema ./schema \
--out ./ts \
--name appchain-grant-vota \
--no-bundle
```

查询 IBC Token

```sh
dorad q bank balances xxxxxx --node https://vota-rpc.dorafactory.org:443
dorad q ibc-transfer denom-trace EF48E6B1A1A19F47ECAEA62F5670C37C0580E86A9E88498B7E393EB6F49F33C0 --node https://vota-rpc.dorafactory.org:443
```

```csv
uosmo, OSMO, ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518
ucore, CORE, 1C8A050BBE4C060A4392CC062B2D3B7C3BFD68E0FA6BB5A5D03DEFCBC0FA7C38
inj, INJ, 4DE84C92C714009D07AFEA7350AB3EC383536BB0FAAD7AF9C0F1A0BEA169304E
unls, NLS, C6824D6F41FC8F906D9656AC77B41FCF7CF136DD74A47E11385B021F6D578FE9
uatom, ATOM, EF48E6B1A1A19F47ECAEA62F5670C37C0580E86A9E88498B7E393EB6F49F33C0
```
