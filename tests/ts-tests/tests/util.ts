import Web3 from "web3";
import { JsonRpcResponse } from "web3-core-helpers";
import { spawn, ChildProcess } from "child_process";
import { Subscription as Web3Subscription } from "web3-core-subscriptions";
import { BlockHeader } from "web3-eth";
import { Log } from "web3-core";

import { Contract } from "web3-eth-contract";
import { getCompiled } from "./contracts";
import * as RLP from "rlp";
const debug = require("debug")("test:transaction");

export const PORT = 19931;
export const RPC_PORT = 19932;
export const WS_PORT = 19933;

export const DISPLAY_LOG = process.env.FRONTIER_LOG || false;
export const OTParachain_LOG = process.env.FRONTIER_LOG || "info";

export const BINARY_PATH = `../../target/release/origintrail-parachain`;
export const SPAWNING_TIME = 30000;

import { BlockHash } from "@polkadot/types/interfaces/chain";
import { ethers } from "ethers";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { HttpProvider } from "web3-core"
import { typesBundle } from "../ot-parachain-types-bundle";

export type EnhancedWeb3 = Web3 & {
	customRequest: (method: string, params: any[]) => Promise<JsonRpcResponse>;
  };

export interface BlockCreation {
	parentHash?: BlockHash;
	finalize?: boolean;
	transactions?: string[];
  }

export interface DevTestContext {
  
	createBlock: (options?: BlockCreation) => Promise<{
	  txResults: JsonRpcResponse[];
	  block: {
		duration: number;
		hash: BlockHash;
	  };
	}>;
  
	// We also provided singleton providers for simplicity
	web3;
	ethers;
	polkadotApi;
  }


export interface TransactionOptions {
	from?: string;
	to?: string;
	privateKey?: string;
	nonce?: number;
	gas?: string | number;
	gasPrice?: string | number;
	value?: string | number | BigInt;
	data?: string;
  }

export const GENESIS_ACCOUNT = "0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b";
export const GENESIS_ACCOUNT_PRIVATE_KEY =
  "0x99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342";

const GENESIS_TRANSACTION: TransactionOptions = {
	from: GENESIS_ACCOUNT,
	privateKey: GENESIS_ACCOUNT_PRIVATE_KEY,
	nonce: null,
	gas: 12_000_000,
	gasPrice: 1_000_000_000,
	value: "0x00",
  };
  
  export const createTransaction = async (
	web3: Web3,
	options: TransactionOptions
  ): Promise<string> => {
	const gas = options.gas || 12_000_000;
	const gasPrice = options.gasPrice !== undefined ? options.gasPrice : 1_000_000_000;
	const value = options.value !== undefined ? options.value : "0x00";
	const from = options.from || GENESIS_ACCOUNT;
	const privateKey =
	  options.privateKey !== undefined ? options.privateKey : GENESIS_ACCOUNT_PRIVATE_KEY;
  
	const data = {
	  from,
	  to: options.to,
	  value: value && value.toString(),
	  gasPrice,
	  gas,
	  nonce: options.nonce,
	  data: options.data,
	};
	debug(
	  `Tx [${/:([0-9]+)$/.exec((web3.currentProvider as any).host)[1]}] ` +
		`from: ${data.from.substr(0, 5) + "..." + data.from.substr(data.from.length - 3)}, ` +
		(data.to
		  ? `to: ${data.to.substr(0, 5) + "..." + data.to.substr(data.to.length - 3)}, `
		  : "") +
		(data.value ? `value: ${data.value.toString()}, ` : "") +
		`gasPrice: ${data.gasPrice.toString()}, ` +
		(data.gas ? `gas: ${data.gas.toString()}, ` : "") +
		(data.nonce ? `nonce: ${data.nonce.toString()}, ` : "") +
		(!data.data
		  ? ""
		  : `data: ${
			  data.data.length < 50
				? data.data
				: data.data.substr(0, 5) + "..." + data.data.substr(data.data.length - 3)
			}`)
	);
	const tx = await web3.eth.accounts.signTransaction(data, privateKey);
	return tx.rawTransaction;
  };
  
  export const createTransfer = async (
	web3: Web3,
	to: string,
	value: number | string | BigInt,
	options: TransactionOptions = GENESIS_TRANSACTION
  ): Promise<string> => {
	return await createTransaction(web3, { ...options, value, to });
  };
  

// Will create the transaction to deploy a contract.
// This requires to compute the nonce. It can't be used multiple times in the same block from the
// same from
export async function createContract(
	web3: Web3,
	contractName: string,
	options: TransactionOptions = GENESIS_TRANSACTION,
	contractArguments: any[] = []
  ): Promise<{ rawTx: string; contract: Contract; contractAddress: string }> {
	const contractCompiled = await getCompiled(contractName);
	const from = options.from !== undefined ? options.from : GENESIS_ACCOUNT;
	const nonce = options.nonce || (await web3.eth.getTransactionCount(from));
	const contractAddress =
	  "0x" +
	  web3.utils
		.sha3(RLP.encode([from, nonce]) as any)
		.slice(12)
		.substring(14);
  
	const contract = new web3.eth.Contract(contractCompiled.contract.abi, contractAddress);
	const data = contract
	  .deploy({
		data: contractCompiled.byteCode,
		arguments: contractArguments,
	  })
	  .encodeABI();
  
	const rawTx = await createTransaction(web3, { ...options, from, nonce, data });
  
	return {
	  rawTx,
	  contract,
	  contractAddress,
	};
  }

// Extra type because web3 is not well typed
export interface Subscription<T> extends Web3Subscription<T> {
	once: (type: "data" | "connected", handler: (data: T) => void) => Subscription<T>;
  }

// Little helper to hack web3 that are not complete.
export function web3Subscribe(web3: Web3, type: "newBlockHeaders"): Subscription<BlockHeader>;
export function web3Subscribe(web3: Web3, type: "pendingTransactions"): Subscription<string>;
export function web3Subscribe(web3: Web3, type: "logs", params: {}): Subscription<Log>;
export function web3Subscribe(
  web3: Web3,
  type: "newBlockHeaders" | "pendingTransactions" | "logs",
  params?: any
) {
  return (web3.eth as any).subscribe(...[].slice.call(arguments, 1));
}

export async function customRequest(web3: Web3, method: string, params: any[]) {
	return new Promise<JsonRpcResponse>((resolve, reject) => {
		(web3.currentProvider as any).send(
			{
				jsonrpc: "2.0",
				id: 1,
				method,
				params,
			},
			(error: Error | null, result?: JsonRpcResponse) => {
				if (error) {
					reject(
						`Failed to send custom request (${method} (${params.join(",")})): ${
							error.message || error.toString()
						}`
					);
				}
				resolve(result);
			}
		);
	});
}

// Create a block and finalize it.
// It will include all previously executed transactions since the last finalized block.
export async function createAndFinalizeBlock(web3: Web3) {
	const response = await customRequest(web3, "engine_createBlock", [true, true, null]);
	if (!response.result) {
		throw new Error(`Unexpected result: ${JSON.stringify(response)}`);
	}
}

let nodeStarted = false;

export async function startOTParachainNode(provider?: string): Promise<{ web3: Web3; binary: ChildProcess }> {

	while (nodeStarted) {
		// Wait 100ms to see if the node is free
		await new Promise((resolve) => {
			setTimeout(resolve, 100);
		});
	}
	nodeStarted = true;

	var web3;
	if (!provider || provider == 'http') {
		web3 = new Web3(`http://localhost:${RPC_PORT}`);
	}

	const cmd = BINARY_PATH;
	const args = [
		`--execution=Native`, // Faster execution using native
		`--no-telemetry`,
		`--no-prometheus`,
		`--dev`,
		`--sealing=manual`,
		`-l${OTParachain_LOG}`,
		`--port=${PORT}`,
		`--rpc-port=${RPC_PORT}`,
		`--ws-port=${WS_PORT}`,
		`--tmp`,
	];


	const onProcessExit = function() {
		binary && binary.kill();
	}

	const onProcessInterrupt = function() {
		process.exit(2);
	}

	let binary: ChildProcess = null;
	process.once("exit", onProcessExit);
	process.once("SIGINT", onProcessInterrupt);
	binary = spawn(cmd, args);

	binary.once("exit", () => {
		process.removeListener("exit", onProcessExit);
		process.removeListener("SIGINT", onProcessInterrupt);
		nodeStarted = false;
	});

	binary.on("error", (err) => {
		if ((err as any).errno == "ENOENT") {
			console.error(
				`\x1b[31mMissing OriginTrail Parachain binary (${BINARY_PATH}).\nPlease compile the OriginTrail Parachain project:\ncargo build\x1b[0m`
			);
		} else {
			console.error(err);
		}
		process.exit(1);
	});

	const binaryLogs = [];
	await new Promise((resolve) => {
		const timer = setTimeout(() => {
			console.error(`\x1b[31m Failed to start OriginTrail Parachain Node.\x1b[0m`);
			console.error(`Command: ${cmd} ${args.join(" ")}`);
			console.error(`Logs:`);
			console.error(binaryLogs.map((chunk) => chunk.toString()).join("\n"));
			throw new Error("Failed to launch node");
		}, SPAWNING_TIME - 2000);

		const onData = async (chunk) => {
			if (DISPLAY_LOG) {
				console.log(chunk.toString());
			}
			binaryLogs.push(chunk);
			if (chunk.toString().match(/Development Service Ready/)) {
				clearTimeout(timer);
				if (!DISPLAY_LOG) {
					binary.stderr.off("data", onData);
					binary.stdout.off("data", onData);
				}
				// console.log(`\x1b[31m Starting RPC\x1b[0m`);
				resolve();
			}
		};
		binary.stderr.on("data", onData);
		binary.stdout.on("data", onData);
	});

	if (provider == 'ws') {
		web3 = new Web3(`ws://localhost:${WS_PORT}`);
	}

	return { web3, binary };
}

async function createPolkadotApi() {
	const apiPromise = await ApiPromise.create({
		initWasm: false,
		provider: new WsProvider(`ws://127.0.0.1:${WS_PORT}`),
		types: {
        AccountId: 'EthereumAccountId',
        AccountId32: 'H256',
        AccountInfo: 'AccountInfoWithTripleRefCount',
        Address: 'AccountId',
        AuthorId: 'AccountId32',
        Balance: 'u128',
        LookupSource: 'AccountId',
        Account: {
          nonce: 'U256',
          balance: 'u128'
        },
        ExtrinsicSignature: 'EthereumSignature',
        RoundIndex: 'u32',
        Candidate: {
          id: 'AccountId',
          fee: 'Perbill',
          bond: 'Balance',
          nominators: 'Vec<Bond>',
          total: 'Balance',
          state: 'CollatorStatus'
        },
        Nominator: {
          nominations: 'Vec<Bond>',
          total: 'Balance'
        },
		NominatorAdded: {
			_enum: [],
		  },
        Bond: {
          owner: 'AccountId',
          amount: 'Balance'
        },
        TxPoolResultContent: {
          pending: 'HashMap<H160, HashMap<U256, PoolTransaction>>',
          queued: 'HashMap<H160, HashMap<U256, PoolTransaction>>'
        },
        TxPoolResultInspect: {
          pending: 'HashMap<H160, HashMap<U256, Summary>>',
          queued: 'HashMap<H160, HashMap<U256, Summary>>'
        },
        TxPoolResultStatus: {
          pending: 'U256',
          queued: 'U256'
        },
        Summary: 'Bytes',
        PoolTransaction: {
          hash: 'H256',
          nonce: 'U256',
          block_hash: 'Option<H256>',
          block_number: 'Option<U256>',
          from: 'H160',
          to: 'Option<H160>',
          value: 'U256',
          gas_price: 'U256',
          gas: 'U256',
          input: 'Bytes'
        },
        // Staking inflation
        Range: 'RangeBalance',
        RangeBalance: {
          min: 'Balance',
          ideal: 'Balance',
          max: 'Balance'
        },
        RangePerbill: {
          min: 'Perbill',
          ideal: 'Perbill',
          max: 'Perbill'
        },
        InflationInfo: {
          expect: 'RangeBalance',
          annual: 'RangePerbill',
          round: 'RangePerbill'
        },
        OrderedSet: 'Vec<Bond>',
        Collator: {
          id: 'AccountId',
          bond: 'Balance',
          nominators: 'Vec<Bond>',
          total: 'Balance',
          state: 'CollatorStatus'
        },
		Collator2: {
			id: "AccountId",
			bond: "Balance",
			nominators: "Vec<AccountId>",
			top_nominators: "Vec<Bond>",
			bottom_nominators: "Vec<Bond>",
			total_counted: "Balance",
			total_backing: "Balance",
			state: "CollatorStatus",
		  },
        CollatorSnapshot: {
          bond: 'Balance',
          nominators: 'Vec<Bond>',
          total: 'Balance'
        },
        SystemInherentData: {
          validation_data: 'PersistedValidationData',
          relay_chain_state: 'StorageProof',
          downward_messages: 'Vec<InboundDownwardMessage>',
          horizontal_messages: 'BTreeMap<ParaId, Vec<InboundHrmpMessage>>'
        },
        RelayChainAccountId: 'AccountId32',
        RoundInfo: {
          current: 'RoundIndex',
          first: 'BlockNumber',
          length: 'u32'
        },
        RewardInfo: {
          total_reward: 'Balance',
          claimed_reward: 'Balance'
        },
        RegistrationInfo: {
          account: 'AccountId',
          deposit: 'Balance'
        },
		ParachainBondConfig: {
			account: "AccountId",
			percent: "Percent",
		  },
      }
	  });
	// We keep track of the polkadotApis to close them at the end of the test
	await apiPromise.isReady;
	// Necessary hack to allow polkadotApi to finish its internal metadata loading
	// apiPromise.isReady unfortunately doesn't wait for those properly
	await new Promise((resolve) => {
	  setTimeout(resolve, 100);
	});

	return apiPromise;
}

export function describeWithOTParachain(title: string, cb: (context: DevTestContext) => void, provider?: string) {
	describe(title, () => {
		let context: DevTestContext = {} as DevTestContext;
		let binary: ChildProcess;
		// Making sure the OriginTrail Parachain node has started
		before("Starting OTParachain", async function () {
			this.timeout(SPAWNING_TIME);
			const init = await startOTParachainNode(provider);
			context.web3 = init.web3;
			binary = init.binary;
			context.polkadotApi = await createPolkadotApi();
		});

		after(async function () {
			await context.polkadotApi.disconnect();
			//console.log(`\x1b[31m Killing RPC\x1b[0m`);
			binary.kill();
		});

		cb(context);
	});
}
