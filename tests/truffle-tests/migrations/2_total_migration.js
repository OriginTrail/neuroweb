var BN = require('bn.js');

var TracToken = artifacts.require('TracToken'); // eslint-disable-line no-undef

var Hub = artifacts.require('Hub'); // eslint-disable-line no-undef
var OldHub = artifacts.require('OldHub'); // eslint-disable-line no-undef
var Profile = artifacts.require('Profile'); // eslint-disable-line no-undef
var Holding = artifacts.require('Holding'); // eslint-disable-line no-undef
var CreditorHandler = artifacts.require('CreditorHandler'); // eslint-disable-line no-undef
var Litigation = artifacts.require('Litigation'); // eslint-disable-line no-undef
var Marketplace = artifacts.require('Marketplace'); // eslint-disable-line no-undef
var Replacement = artifacts.require('Replacement'); // eslint-disable-line no-undef
var Approval = artifacts.require('Approval'); // eslint-disable-line no-undef

var ProfileStorage = artifacts.require('ProfileStorage'); // eslint-disable-line no-undef
var HoldingStorage = artifacts.require('HoldingStorage'); // eslint-disable-line no-undef
var LitigationStorage = artifacts.require('LitigationStorage'); // eslint-disable-line no-undef
var MarketplaceStorage = artifacts.require('MarketplaceStorage'); // eslint-disable-line no-undef

var MockHolding = artifacts.require('MockHolding'); // eslint-disable-line no-undef
var MockApproval = artifacts.require('MockApproval'); // eslint-disable-line no-undef
var TestingUtilities = artifacts.require('TestingUtilities'); // eslint-disable-line no-undef

var Identity = artifacts.require('Identity'); // eslint-disable-line no-undef

const amountToMint = (new BN(5)).mul((new BN(10)).pow(new BN(30)));

module.exports = async (deployer, network, accounts) => {
    let hub;
    let oldHub;
    let token;

    let profile;
    let holding;
    let creditorHandler;
    let litigation;
    let marketplace;
    let replacement;
    let approval;

    let profileStorage;
    let holdingStorage;
    let litigationStorage;
    let marketplaceStorage;

    var amounts = [];
    var recepients = [];

    var temp;
    var temp2;

    if (network === 'dev') {
        await deployer.deploy(Hub, {gas: 6000000, from: accounts[0]})
            .then((result) => {
                hub = result;
            });
        await hub.setContractAddress('Owner', accounts[0]);

        profileStorage = await deployer.deploy(
            ProfileStorage,
            hub.address, {gas: 6000000, from: accounts[0]},
        );
        await hub.setContractAddress('ProfileStorage', profileStorage.address);

        holdingStorage = await deployer.deploy(
            HoldingStorage,
            hub.address,
            {gas: 6000000, from: accounts[0]},
        );
        await hub.setContractAddress('HoldingStorage', holdingStorage.address);

        marketplaceStorage = await deployer.deploy(
            MarketplaceStorage,
            hub.address,
            {gas: 6000000, from: accounts[0]},
        );
        await hub.setContractAddress('MarketplaceStorage', marketplaceStorage.address);

        litigationStorage = await deployer.deploy(
            LitigationStorage,
            hub.address,
            {gas: 6000000, from: accounts[0]},
        );
        await hub.setContractAddress('LitigationStorage', litigationStorage.address);

        approval = await deployer.deploy(Approval);
        await hub.setContractAddress('Approval', approval.address);

        token = await deployer.deploy(TracToken, accounts[0], accounts[0], accounts[0]);
        await hub.setContractAddress('Token', token.address);

        profile = await deployer.deploy(Profile, hub.address, {gas: 10000000, from: accounts[0]});
        await hub.setContractAddress('Profile', profile.address);

        holding = await deployer.deploy(Holding, hub.address, {gas: 10000000, from: accounts[0]});
        await hub.setContractAddress('Holding', holding.address);

        creditorHandler = await deployer.deploy(
            CreditorHandler,
            hub.address,
            {gas: 10000000, from: accounts[0]},
        );
        await hub.setContractAddress('CreditorHandler', creditorHandler.address);

        litigation = await deployer.deploy(
            Litigation,
            hub.address,
            {gas: 6000000, from: accounts[0]},
        );
        await hub.setContractAddress('Litigation', litigation.address);

        marketplace = await deployer.deploy(
            Marketplace,
            hub.address,
            {gas: 7000000, from: accounts[0]},
        );
        await hub.setContractAddress('Marketplace', marketplace.address);

        replacement = await deployer.deploy(
            Replacement,
            hub.address,
            {gas: 7000000, from: accounts[0]},
        );
        await hub.setContractAddress('Replacement', replacement.address);

        recepients = [
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

        for (let i = 0; i < recepients.length; i += 1) {
            amounts.push(amountToMint);
        }
        await token.mintMany(recepients, amounts, {from: accounts[0]});
        await token.finishMinting({from: accounts[0]});

        console.log('\n\n \t Contract adressess on ganache:');
        console.log(`\t Hub contract address: \t\t\t${hub.address}`);
        console.log(`\t Approval contract address: \t\t${approval.address}`);
        console.log(`\t Token contract address: \t\t${token.address}`);
        console.log(`\t Profile contract address: \t\t${profile.address}`);
        console.log(`\t Holding contract address: \t\t${holding.address}`);
        console.log(`\t Litigation contract address: \t\t${litigation.address}`);
        console.log(`\t Marketplace contract address: \t\t${marketplace.address}`);
        console.log(`\t Replacement contract address: \t\t${replacement.address}`);

        console.log(`\t ProfileStorage contract address: \t${profileStorage.address}`);
        console.log(`\t HoldingStorage contract address: \t${holdingStorage.address}`);
        console.log(`\t LitigationStorage contract address: \t${litigationStorage.address}`);
        console.log(`\t MarketplaceStorage contract address: \t${marketplaceStorage.address}`);
    }
};
