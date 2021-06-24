const Web3 = require('web3');
const web3 = new Web3('http://localhost:9933');

const EthereumTx = require('ethereumjs-tx').Transaction;
try {
    var lightwallet = require('eth-lightwallet');
} catch (err) {
    delete global._bitcore;
    var lightwallet = require('eth-lightwallet');
}
const { txutils } = lightwallet;
const PATH_TO_CONTRACTS = '../truffle-tests/build/contracts';

const testingUtilitiesContractAbi = require(`${PATH_TO_CONTRACTS}/TestingUtilities`).abi;
const testingUtilitiesContractBytecode = require(`${PATH_TO_CONTRACTS}/TestingUtilities`).bytecode;

const wallet = '0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b';
const privateKey = '99b3c12287537e38c90a9219d4cb074a89a16e9cdb20bf85728ebd97c343e342';

const deployTestingContract = async () => {
    let createTransaction = await web3.eth.accounts.signTransaction({
        from: wallet,
        data: testingUtilitiesContractBytecode,
        value: "0x00",
        gasPrice: "01",
        gas: "2000000",
    }, privateKey);
    let createReceipt = await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);
    console.log("Testing contract deployed at address: ", createReceipt.contractAddress);
    const testingUtilitiesContractAddress = createReceipt.contractAddress;

    const util =
        new web3.eth.Contract(testingUtilitiesContractAbi, testingUtilitiesContractAddress);

    return util;
};

const sendTransactionUsingWeb3 = async (testingContract) => {
    const data = testingContract.methods.moveTheBlock().encodeABI();

    const createTransaction = await web3.eth.accounts.signTransaction({
        from: wallet,
        to: testingContract.options.address,
        data,
        value: "0x00",
        gasPrice: "01",
        gas: "2000000",
    }, privateKey);
    const createReceipt = await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);
};

const main = async () => {
    const testingContract = await deployTestingContract();

    // Get Initial state of test variable
    let state = await testingContract.methods.getInternalData().call();
    console.log(`Current Utitlites contract test variable state: ${state}`);

    // Update the variable using web3
    console.log(`Updating the Utitlites contract test variable state using web3`);
    await sendTransactionUsingWeb3(testingContract);
    state = await testingContract.methods.getInternalData().call();
    console.log(`New Utitlites contract test variable state: ${state}`);

    // Attempt to update the value using the libraries used on ot-node
    try {
        const buildContractTransaction = async (wallet, contractAddress, contractAbi, functionName, args) => {
            const nonce = await web3.eth.getTransactionCount(wallet);
            const txOptions = {
                gasLimit: web3.utils.toHex(800000),
                gasPrice: web3.utils.toHex(123),
                to: contractAddress,
                nonce: nonce
            };
            return txutils.functionTx(
                contractAbi,
                functionName,
                args,
                txOptions
            );
        };

        const signAndSendTransaction = async function (wallet, privateKey, tx) {
            try {
                const transaction = new EthereumTx(tx);
                transaction.sign(Buffer.from(privateKey, 'hex'));
                const serializedTx = transaction.serialize().toString('hex');
                return web3.eth.sendSignedTransaction(`0x${serializedTx}`);
            } catch (err) {
                console.log(err);
            }
        };

        const txApproval = await buildContractTransaction(
            wallet,
            testingContract.options.address,
            testingUtilitiesContractAbi,
            'moveTheBlock',
            [],
        );
        await signAndSendTransaction(wallet, privateKey, txApproval);
    } catch (e) {
        console.log('\n');
        console.log('Calling transaction using ethereumjs-tx and eth-lightwallet failed');
        console.log(e);
        console.log('\n');
    }

    state = await testingContract.methods.getInternalData().call();
    console.log(`Final Utilities contract state: ${state}`);
};

main();