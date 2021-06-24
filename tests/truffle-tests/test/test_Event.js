const fs = require('fs');
const bn = require('bn.js');
const Web3 = require('web3');
const web3 = new Web3('http://localhost:9933');

// Example test script - Uses Mocha and Ganache
const Eventer = artifacts.require("Eventer");
const Eventest = artifacts.require("Eventest");

const EventerData = require('../build/contracts/Eventer.json');
const EventestData = require('../build/contracts/Eventest.json');

contract('Event testing', accounts => {
    let eventer;
    let eventest;

    beforeEach(async () => {
        // Deploy event contracts
        eventer = await Eventer.new({ from: accounts[0] });
        eventest = await Eventest.new({ from: accounts[0] });
    });

    it("Test emitting and fetching events", async () => {
        const number = "1000000000000000000";
        const bytes = '0x5cad6896887d99d70db8ce035d331ba2ade1a5e1161f38ff7fda76cf7c308cde';
        const address = accounts[0];

        console.log(`Eventer address ${eventer.address}`);
        console.log(`Eventest address ${eventest.address}`);

        const startBlockNumber = await web3.eth.getBlockNumber();

        console.log('Running test transactions, please wait a few minutes');

        await eventer.emitEventOne(number, false);
        await eventer.emitEventOne(number, true);
        await eventer.emitEventTwo(number, bytes);

        await eventest.emitEventThree(number, address);
        await eventest.emitEventFour(number, number, number);
        await eventest.emitEventFour(number, number, number);

        const localEventer = new web3.eth.Contract(EventerData.abi, eventer.address);
        const localEventest = new web3.eth.Contract(EventestData.abi, eventest.address);

        const blockNumber = await web3.eth.getBlockNumber();
        console.log(`Fecthed block number ${blockNumber}: ${typeof blockNumber}`);

        // let events = await localEventest.getPastEvents('allEvents');
        let events = await localEventer.getPastEvents('allEvents', {
            fromBlock: startBlockNumber,
            toBlock: blockNumber,
        });

        console.log('******************    Eventer   *********************');
        console.log(`Events fetched from eventer:\n ${events.length}`);
        // console.log(JSON.stringify(events, null, 4));

        let contractAddresses = {};
        for (const event of events) {
            if (!contractAddresses[event.address]) {
                contractAddresses[event.address] = 1;
            } else {
                contractAddresses[event.address] += 1;
            }
        }
        console.log(`Contracts received from eventer: \n${JSON.stringify(contractAddresses, null, 2)}`);

        events = await localEventest.getPastEvents('allEvents', {
            fromBlock: startBlockNumber,
            toBlock: blockNumber,
        });

        console.log('******************    Eventest   *********************');
        console.log(`Events fetched from eventest:\n ${events.length}`);
        // console.log(JSON.stringify(events, null, 4));

        contractAddresses = {};
        for (const event of events) {
            if (!contractAddresses[event.address]) {
                contractAddresses[event.address] = 1;
            } else {
                contractAddresses[event.address] += 1;
            }
        }
        console.log(`Contracts received from eventest: \n${JSON.stringify(contractAddresses, null, 2)}`);
    })
});
