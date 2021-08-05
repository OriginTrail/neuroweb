import { expect } from "chai";

import { describeWithOTParachain, createAndFinalizeBlock } from "./util";

describeWithOTParachain("OriginTrail Parachain RPC (Fees)", (context) => {

  it("should match fee per byte", async function () {
    let transactionByteFee = await context.polkadotApi.consts.transactionPayment.transactionByteFee.toString();
    expect(transactionByteFee).to.eq("100000000000000");
  });

  it("should match weight to fee coefficients", async function () {
    let weightToFee = await context.polkadotApi.consts.transactionPayment.weightToFee.toHuman();
    expect(weightToFee[0].coeffInteger).to.eq("1.0000 aUnit");
    expect(weightToFee[0].degree).to.eq("1");
    expect(weightToFee[0].coeffFrac).to.eq("0.00%");
  });

  it("should match next fee multiplier", async function () {
    let nextFeeMultiplier = await context.polkadotApi.query.transactionPayment.nextFeeMultiplier();
    expect(nextFeeMultiplier.toHuman()).to.eq("1,000,000,000,000,000,000");
  });

  it("should match block weight", async function () {
    let blockWeights = await context.polkadotApi.consts.system.blockWeights.toHuman();
    expect(blockWeights.baseBlock).to.eq("5,000,000,000");
    expect(blockWeights.maxBlock).to.eq("500,000,000,000");
    expect(blockWeights.perClass.normal.baseExtrinsic).to.eq("125,000,000");
    expect(blockWeights.perClass.normal.maxExtrinsic).to.eq("83,168,000,000,001");
    expect(blockWeights.perClass.normal.reserved).to.eq("375,000,000,000");
    expect(blockWeights.perClass.operational.baseExtrinsic).to.eq("1");
    expect(blockWeights.perClass.operational.maxExtrinsic).to.eq("32,000,000,000");
    expect(blockWeights.perClass.operational.reserved).to.eq("449,875,000,000");
    expect(blockWeights.perClass.mandatory.baseExtrinsic).to.eq("128,000,000,000,001");
    expect(blockWeights.perClass.mandatory.maxExtrinsic).to.eq("8,192,000,000,000,256");
  });

  it("should match block length", async function () {
    let blockLength = await context.polkadotApi.consts.system.blockLength.toHuman();
    expect(blockLength.max.normal).to.eq("3,932,160");
    expect(blockLength.max.operational).to.eq("5,242,880");
    expect(blockLength.max.mandatory).to.eq("5,242,880");
  });
  
});
