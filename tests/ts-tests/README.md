# Functional testing for OriginTrail Parachain Node RPC

This folder contains a set of functional tests desgined to perform functional testing on the OriginTrail Parachain Eth RPC.

It is written in typescript, using Mocha/Chai as Test framework.

## Installation

```
npm install
```

## Run the tests

```
npm run test
```

You can also add the Frontier Node logs to the output using the `FRONTIER_LOG` env variable. Ex:

```
FRONTIER_LOG="warn,rpc=trace" npm run test
```

(The OriginTrail Parachain node be listening for RPC on port 19933, mostly to avoid conflict with already running substrate node)
