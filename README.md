# OriginTrail Parachain Node

<div align="center">
  <img src="https://parachain.origintrail.io/images/navigation-logo.svg">


[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Substrate version](https://img.shields.io/badge/Substrate-3.0.0-brightgreen?logo=Parity%20Substrate)](https://substrate.io)
[![Twitter URL](https://img.shields.io/twitter/follow/OT_Parachain?style=social)](https://twitter.com/OT_Parachain)
[![Telegram](https://img.shields.io/endpoint?color=neon&style=flat-square&url=https%3A%2F%2Ftg.sumanjay.workers.dev%2Forigintrail)](https://t.me/origintrail)
[![Medium](https://badgen.net/badge/icon/medium?icon=medium&label)](https://medium.com/origintrail)
[![Discord](https://img.shields.io/badge/Discord-gray?logo=discord)](https://discord.gg/FCgYk2S)
  
</div>

The OriginTrail Parachain is the next-generation L1 blockchain designed to tightly integrate with the OriginTrail DKG. As an OriginTrail-tailored blockchain it is optimized for maximum performance and usability in the OriginTrail consensus layer. It leverages the strong trust model and inherent interoperability of Polkadot, enabling smooth integration with other Polkadot ecosystem projects.

![](https://parachain.origintrail.io/storage/whitepaper-content/April2022/img-otl-ayer-2@2x.jpg)

OriginTrail Parachain node is built with [Substrate](https://substrate.dev) framework.

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

To run a full network with multiple OriginTrail parachain nodes (collators and non-collators), first we need to start relay chain, and after that our parachain.

![](https://parachain.origintrail.io/storage/whitepaper-content/April2022/img-flow-chart@2x.png)

#### Run A Relay Chain

To start a relay chain we recommend reading and following instructions in [Cumulus Workshop](https://substrate.dev/cumulus-workshop/).
OriginTrail parachain is currently compatible with Polkadot v0.9.18 version.

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
[Substrate Cumulus Worship](https://substrate.dev/cumulus-workshop/#/en/3-parachains/2-register)
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
- More about OriginTrail Parachain and Decentralized Knowledge Graph read in [whitepaper](https://parachain.origintrail.io/whitepaper)
- More detailed instructions to use Cumulus parachains are found in the
[Cumulus Workshop](https://substrate.dev/cumulus-workshop/#/en/3-parachains/2-register)
- Refer to the upstream
[Substrate Developer Hub Node Template](https://github.com/substrate-developer-hub/substrate-node-template)
to learn more about the structure of this project, the capabilities it encapsulates and the way in
which those capabilities are implemented.
