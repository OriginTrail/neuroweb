const Web3 = require('web3');
const web3 = new Web3('http://localhost:9933');

const addressFrom = '0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b';
const addressTo = '0x798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc';

const privateKey = '99b3c12287537e38c90a9219d4cb074a89a16e9cdb20bf85728ebd97c343e342';

const submit = async () => {
    let balance = await web3.eth.getBalance(addressFrom);
    console.log(`Balance of ${addressFrom} is ${balance}`);
    balance = await web3.eth.getBalance(addressTo);
    console.log(`Balance of ${addressTo} is ${balance}`);
    
    console.log(`Attempting to make transaction from ${addressFrom} to ${addressTo}`);
    const val = web3.utils.toWei('100000', 'ether');
    // console.log(val);
    const createTransaction = await web3.eth.accounts.signTransaction(
        {
            from: addressFrom,
            to: addressTo,
            value: val,
            gasPrice: 1,
            gas: 21000,
            chainId: 2160
        },
        privateKey
    );
    // Deploy transaction
    const createReceipt = await web3.eth.sendSignedTransaction(
        createTransaction.rawTransaction
    );
    console.log(
        `Transaction successful with hash: ${createReceipt.transactionHash}`
    );

};
submit();
