const Web3 = require('web3');

const web3 = new Web3('http://localhost:9933');

const testingUtilitiesContractAbi = require('./build/contracts/TestingUtilities').abi;
const testingUtilitiesContractBytecode = require('./build/contracts/TestingUtilities').bytecode;

const wallet = '0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b';
const privateKey = '99b3c12287537e38c90a9219d4cb074a89a16e9cdb20bf85728ebd97c343e342';
const val = web3.utils.toWei('0', 'ether');
const deploy = async () => {
    let createTransaction = await web3.eth.accounts.signTransaction({
        from: wallet,
        data: testingUtilitiesContractBytecode,
        value: val,
        gasPrice: 1,
        gas: 426552,
        chainId: 2160
    }, privateKey);

    // console.log(createTransaction);

    let createReceipt = await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);
    console.log("Contract deployed at address", createReceipt.contractAddress);
    const testingUtilitiesContractAddress = createReceipt.contractAddress;

    let util = new web3.eth.Contract(testingUtilitiesContractAbi, testingUtilitiesContractAddress);
    let state = await util.methods.getInternalData().call();
    // console.log(state);
    const data = util.methods.moveTheBlock().encodeABI();
    createTransaction = await web3.eth.accounts.signTransaction({
        from: wallet,
        to: testingUtilitiesContractAddress,
        data,
        value: val,
        gasPrice: 1,
        gas: 426552,
        chainId: 2160
    }, privateKey);
    console.log(createTransaction);
    createReceipt = await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);
    console.log(createReceipt);
    state = await util.methods.getInternalData().call();
    console.log(state);
};
deploy();