const Web3 = require('web3');
const web3 = new Web3('http://localhost:9933');

const PATH_TO_CONTRACTS = '../truffle-tests/build/contracts';

const wallet = '0x8097c3C354652CB1EEed3E5B65fBa2576470678A';
const privateKey = 'e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a';

const deployContract = async (contractName, inputTypes, inputValues) => {
  try {
      console.log(`Attempting to deploy ${contractName} contract`);

      const abi = require(`${PATH_TO_CONTRACTS}/${contractName}`).abi;
      const bytecode = require(`${PATH_TO_CONTRACTS}/${contractName}`).bytecode;

      const parameters = web3.eth.abi.encodeParameters(
          inputTypes,
          inputValues,
      ).slice(2);

      let createTransaction = await web3.eth.accounts.signTransaction({
          from: wallet,
          data: `${bytecode}${parameters}`,
          value: "0x00",
          gasPrice: "01",
          gas: "10000000",
      }, privateKey);

      let createReceipt = await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);
      console.log(`${contractName} contract deployed at address ${createReceipt.contractAddress}`);
      console.log(createReceipt);

      const contractInstance = new web3.eth.Contract(abi, createReceipt.contractAddress);

      return contractInstance;
  }  catch (error) {
      console.log('\n\n');
      console.log(error);
      console.log('\n\n');
  }
};

const setContractAddress = async (hubContract, contractName, contractAddress) => {
    console.log(`Attempting to set ${contractName} contract address into Hub contract`);

    const data = hubContract.methods.setContractAddress(contractName, contractAddress).encodeABI();

    const createTransaction = await web3.eth.accounts.signTransaction({
        from: wallet,
        to: hubContract.options.address,
        data,
        value: "0x00",
        gasPrice: "01",
        gas: "2000000",
    }, privateKey);
    const createReceipt = await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);
};

const main = async () => {
    const Hub = await deployContract('Hub', [], []);
    const hubAddress = Hub.options.address;

    const ProfileStorage = await deployContract('ProfileStorage', ['address'], [hubAddress]);
    await setContractAddress(Hub, 'ProfileStorage', ProfileStorage.options.address);

    const HoldingStorage = await deployContract('HoldingStorage', ['address'], [hubAddress]);
    await setContractAddress(Hub, 'HoldingStorage', HoldingStorage.options.address);

    const MarketplaceStorage = await deployContract('MarketplaceStorage', ['address'], [hubAddress]);
    await setContractAddress(Hub, 'MarketplaceStorage', MarketplaceStorage.options.address);

    const LitigationStorage = await deployContract('LitigationStorage', ['address'], [hubAddress]);
    await setContractAddress(Hub, 'LitigationStorage', LitigationStorage.options.address);

    const Approval = await deployContract('Approval', [], []);
    await setContractAddress(Hub, 'Approval', Approval.options.address);

    const TracToken = await deployContract(
        'TracToken',
        ['address','address','address'],
        [wallet,wallet,wallet],
        );
    await setContractAddress(Hub, 'Token', TracToken.options.address);

    const Profile = await deployContract('Profile', ['address'], [hubAddress]);
    await setContractAddress(Hub, 'Profile', Profile.options.address);

    const Holding = await deployContract('Holding', ['address'], [hubAddress]);
    await setContractAddress(Hub, 'Holding', Holding.options.address);

    const CreditorHandler = await deployContract('CreditorHandler', ['address'], [hubAddress]);
    await setContractAddress(Hub, 'CreditorHandler', CreditorHandler.options.address);

    const Litigation = await deployContract('Litigation', ['address'], [hubAddress]);
    await setContractAddress(Hub, 'Litigation', Litigation.options.address);

    const Marketplace = await deployContract('Marketplace', ['address'], [hubAddress]);
    await setContractAddress(Hub, 'Marketplace', Marketplace.options.address);

    const Replacement = await deployContract('Replacement', ['address'], [hubAddress]);
    await setContractAddress(Hub, 'Replacement', Replacement.options.address);


    const recepients = [
        '0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b',
        '0xd6879C0A03aDD8cFc43825A42a3F3CF44DB7D2b9',
        '0x2f2697b2a7BB4555687EF76f8fb4C3DFB3028E57',
        '0xBCc7F04c73214D160AA6C892FcA6DB881fb3E0F5',
        '0xE4745cE633c2a809CDE80019D864538ba95201E3',
        '0x193a22749001fA75497fb8fcCE11235947a19b3d',
        '0xFFEE9a020c3DdDE30D0391254E05c8Fe8DC4a680',
        '0xBBC863B0776f5F8F816dD71e85AaA81449A87D9A',
        '0x64B592e8e9AF51Eb0DBa5d4c18b817C01e8e75a8',
        '0xb664cf668534FDE04cF43173e2187b7a9196bfC3',
        '0xCE81B24feDb116FaC5A887ED7706B05A46060079',
        '0xcF9c758Ae7C21D8048Fc1C3cb6164Ff37A5b205e',
        '0xC8b866F2dD57d2889a7cccB989cfb42dB4579411',
        '0xD242D54ed86A64909d0f990bCe608b065ed07559',
        '0x3368d4DBeC10220D7Ba73c3FC65977ED215C62Fc',
        '0x9d2934024ccC3995056E7E067184B3B0bB8B66Ab',
        '0x73DF1C18F18AA54DB018b3273E44b1A4713c5fE2',
        '0xd2c714a04fEA61C599815ec57fAf25ba4F4d496B',
        '0xBA9d00b748217381674E277D2447fcaCB78bcAc7',
    ];
    const amounts = [];
    const amountToMint = '10000000000000000000000000';
    for (let i = 0; i < recepients.length; i += 1) {
        amounts.push(amountToMint);
    }

    let data = TracToken.methods.mintMany(recepients, amounts).encodeABI();

    let createTransaction = await web3.eth.accounts.signTransaction({
        from: wallet,
        to: TracToken.options.address,
        data,
        value: "0x00",
        gasPrice: "01",
        gas: "2000000",
    }, privateKey);
    let createReceipt = await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);


    data = TracToken.methods.finishMinting().encodeABI();

    createTransaction = await web3.eth.accounts.signTransaction({
        from: wallet,
        to: TracToken.options.address,
        data,
        value: "0x00",
        gasPrice: "01",
        gas: "2000000",
    }, privateKey);
    await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);

    console.log('Contracts have been deployed!');

    console.log('\n\n \t Contract adressess on starfleet:');
    console.log(`\t Hub contract address: \t\t\t\t\t${Hub.options.address}`);
    console.log(`\t Approval contract address: \t\t\t${Approval.options.address}`);
    console.log(`\t Token contract address: \t\t\t\t${TracToken.options.address}`);
    console.log(`\t Profile contract address: \t\t\t\t${Profile.options.address}`);
    console.log(`\t Holding contract address: \t\t\t\t${Holding.options.address}`);
    console.log(`\t Litigation contract address: \t\t\t${Litigation.options.address}`);
    console.log(`\t Marketplace contract address: \t\t\t${Marketplace.options.address}`);
    console.log(`\t Replacement contract address: \t\t\t${Replacement.options.address}`);

    console.log(`\t ProfileStorage contract address: \t\t${ProfileStorage.options.address}`);
    console.log(`\t HoldingStorage contract address: \t\t${HoldingStorage.options.address}`);
    console.log(`\t LitigationStorage contract address: \t${LitigationStorage.options.address}`);
    console.log(`\t MarketplaceStorage contract address: \t${MarketplaceStorage.options.address}`);
};
main();