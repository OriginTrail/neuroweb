# NeuroWeb Node

<div align="center">
  <img src="https://140069760-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FqMnH71Mmd7Au6HK5UAji%2Fuploads%2FEmgmn8pgCqu9eJaQPNSO%2FNeuroWeb%20X%20visual%20(1).jpg?alt=media&token=320f5009-0932-4ecc-a5a6-3bbb7a56cf45">


[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Substrate version](https://img.shields.io/badge/Substrate-3.0.0-brightgreen?logo=Parity%20Substrate)](https://substrate.io)
[![Twitter URL](https://img.shields.io/twitter/follow/NeuroWebAI?style=social)](https://twitter.com/NeuroWebAI)
[![Telegram](https://img.shields.io/endpoint?color=neon&style=flat-square&url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Forigintrail)](https://t.me/origintrail)
[![Medium](https://badgen.net/badge/icon/medium?icon=medium&label)](https://medium.com/origintrail)
[![Discord](https://img.shields.io/badge/Discord-gray?logo=discord)](https://discord.gg/FCgYk2S)
  
</div>

NeuroWeb Network is a decentralized Artificial Intelligence blockchain designed to incentivise knowledge creation, connectivity and sharing through **Knowledge Mining**. It's utility token NEURO is designed to fuel the AI knowledge economy, rewarding relevant knowledge contributions to the **OriginTrail Decentralized Knowledge Graph**.

- NeuroWeb builds on the basis of its predecessor - the OriginTrail Parachain - which was transformed into NeuroWeb via a community Governance vote on OriginTrail Parachain in December 2023.
- NeuroWeb is a permissionless, EVM enabled blockchain secured by Polkadot validators.
- NeuroWeb node is built with [Substrate](https://substrate.dev) framework.

## Build & Run

Follow these steps to prepare a local Substrate development environment :hammer_and_wrench:

### Setup of Machine

If necessary, refer to the setup instructions at the
[Substrate Developer Hub](https://substrate.dev/docs/en/knowledgebase/getting-started/#manual-installation).

### Build

Once the development environment is set up, build the parachain node template. This command will
build the
[Wasm Runtime](https://substrate.dev/docs/en/knowledgebase/advanced/executor#wasm-execution) and
[native node](https://substrate.dev/docs/en/knowledgebase/advanced/executor#native-execution) code:

```bash
cargo build --release
```

### Run a network

To run a full network with multiple NeuroWeb nodes (collators and non-collators), first we need to start relay chain, and after that our parachain.

![](https://parachain.origintrail.io/storage/whitepaper-content/April2022/img-flow-chart@2x.png)

#### Run A Relay Chain

To start a relay chain we recommend reading and following instructions in [Cumulus Workshop](https://docs.substrate.io/tutorials/build-a-parachain/prepare-a-local-relay-chain/).
NeuroWeb is currently compatible with Polkadot v0.9.40 version.

#### Parachain Nodes (Collators)

From the OriginTrail parachain working directory:

```bash
# NOTE: this command assumes the chain spec is in a directory named `polkadot`
# that is at the same level of the template working directory. Change as needed.
./target/release/origintrail-parachain \
-d cumulus-parachain/alice\
--local \
--collator\
--alice\
--ws-port 9945\
--parachain-id 2000\
--\
--execution wasm\
--chain ../polkadot/rococo_local.json
```

Then you need to register on Local Relay Chain as it is presented in Cumulus Workshop. For registering we first need to export the Parachain Genesis and Runtime.

```bash
# Build the Chain spec
./target/release/origintrail-parachain build-spec\
--disable-default-bootnode > ./template-local-plain.json

# Build the raw file
./target/release/origintrail-parachain build-spec \
--chain=./resources/template-local-plain.json \
--raw --disable-default-bootnode > ./resources/template-local.json

# Export genesis state to `./resources files
./target/release/origintrail-parachain export-genesis-state --parachain-id 2000 > ./para-2000-genesis
# export runtime wasm
./target/release/origintrail-parachain export-genesis-wasm > ./para-2000-wasm
```

In order to produce blocks you will need to register the parachain as detailed in the
[Substrate Cumulus Workshop](https://docs.substrate.io/tutorials/build-a-parachain/connect-a-local-parachain/)
by going to:

`Developer -> sudo -> paraSudoWrapper -> sudoScheduleParaInitialize(id, genesis)`

Ensure you set the `ParaId to 2000` and the `parachain: Bool to Yes`.

### Containerize

#### Build
```shell
docker build -t origintrail-parachain .
```
#### Run
```shell
docker run -it -p 30333:30333 -p 9933:9933 -p 9944:9944 -p 9615:9615 -v /data:/data origintrail-parachain:latest\
  --base-path=/data --rpc-external --ws-external\
  --ws-max-connections=1000 --rpc-cors=all\
  --prometheus-external --rpc-methods=Unsafe\
  --chain=/config/origintrail-parachain-2043-raw.json\
  --no-mdns --execution=wasm --pruning=archive\
  -- --execution=wasm --wasm-execution=Compiled --chain=polkadot
```

## Learn More

- More about expanding the multi-chain OriginTrail with Polkadot and OriginTrail Parachain development roadmap you can find on 
  [parachain.origintrail.io](https://parachain.origintrail.io/)
- More about OriginTrail Parachain and Decentralized Knowledge Graph read in [whitepaper](https://origintrail.io/ecosystem/whitepaper)
