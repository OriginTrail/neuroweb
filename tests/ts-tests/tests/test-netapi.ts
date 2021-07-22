import { expect } from "chai";
import { step } from "mocha-steps";

import { describeWithOTParachain, customRequest } from "./util";

describeWithOTParachain("OriginTrail Parachain RPC (Net)", (context) => {
    step("should return `net_version` 2160", async function () {
        expect(await context.web3.eth.net.getId()).to.equal(2160);
    });
    step("should return `peer_count` in hex directly using the provider", async function () {
        expect((await customRequest(context.web3, "net_peerCount", [])).result).to.be.eq(0x0);
    });
    step("should format `peer_count` as decimal using `web3.net`", async function () {
        expect(await context.web3.eth.net.getPeerCount()).to.equal("0xb");
    });
});