import { expect } from "chai";
import { step } from "mocha-steps";

import { describeWithOTParachain, customRequest } from "./util";

describeWithOTParachain("OriginTrail Parachain RPC (Web3Api)", (context) => {

	step("should get client version", async function () {
		const version = await context.web3.eth.getNodeInfo();
		expect(version).to.be.equal("origintrail-parachain/v102.1/fc-rpc-2.0.0-dev");
	});

	step("should remote sha3", async function () {
		const data = context.web3.utils.stringToHex("hello");
		const hash = await customRequest(context.web3, "web3_sha3", [data]);
		const local_hash = context.web3.utils.sha3("hello");
		expect(hash.result).to.be.equal(local_hash);
	});
});
