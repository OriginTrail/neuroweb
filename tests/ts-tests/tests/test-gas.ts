import { expect } from "chai";

import Test from "../build/contracts/Test.json"
import { describeWithOTParachain, createAndFinalizeBlock } from "./util";
import { AbiItem } from "web3-utils";

describeWithOTParachain("OriginTrail Parachain RPC (Gas)", (context) => {
	const GENESIS_ACCOUNT = "0x6be02d1d3665660d22ff9624b7be0551ee1ac91b";

	const TEST_CONTRACT_BYTECODE = Test.bytecode;
	const TEST_CONTRACT_ABI = Test.abi as AbiItem[];
	const FIRST_CONTRACT_ADDRESS = "0xc2bf5f29a4384b1ab0c063e1c666f02121b6084a"; // Those test are ordered. In general this should be avoided, but due to the time it takes	// to spin up a OriginTrail Parachain node, it saves a lot of time.

	it("eth_estimateGas for contract creation", async function () {
		let estimation = await context.web3.eth.estimateGas({
			from: GENESIS_ACCOUNT,
			data: Test.bytecode,
		});

		expect(estimation).to.equal(186067);
	});

	it("block gas limit over 5M", async function () {
		expect((await context.web3.eth.getBlock("latest")).gasLimit).to.be.above(5000000);
	});

	it("eth_estimateGas for contract call", async function () {
		const contract = new context.web3.eth.Contract(TEST_CONTRACT_ABI, FIRST_CONTRACT_ADDRESS, {
			from: GENESIS_ACCOUNT,
			gasPrice: "0x01",
		});

		expect(await contract.methods.multiply(3).estimateGas()).to.equal(21204);
	});

	it("eth_estimateGas without gas_limit should pass", async function () {
		const contract = new context.web3.eth.Contract(TEST_CONTRACT_ABI, FIRST_CONTRACT_ADDRESS, {
			from: GENESIS_ACCOUNT
		});

		expect(await contract.methods.multiply(3).estimateGas()).to.equal(21204);
	});

});
