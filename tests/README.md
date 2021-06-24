## Tests

This guide uses multiple nodejs versions so it is highly recommended to have the [nvm](https://github.com/nvm-sh/nvm) tool installed to easily switch between different versions during setup. You’ll also need node and npm installed, as well as some ot-node dependencies which are explained later in this guide.
Since the two repositories in this guide are dependent on one another, it is recommended to set up a folder and clone both repositories inside that folder. It will make setup and testing much easier, trust me :)

### Starfleet blockchain node setup

The first requirement for testing is running a custom substrate node, found at [https://github.com/OriginTrail/starfleet-node](https://github.com/OriginTrail/starfleet-node)

Clone the repo using the `feature/builders-demo` branch:
```sh
git clone -b feature/builders-demo https://github.com/OriginTrail/starfleet-node
```
Then run the following commands to start the node:
```sh
make run
```
This should start the Starfleet node in dev mode and will enable us to deploy smart contracts on it.

#### Smart contract deployment

Firstly, clone the OriginTrail OT-Node repository with the following command (remember to do it in the same folder you cloned the starfleet-node)
```
git clone -b feature/starfleet-integration https://github.com/OriginTrail/ot-node.git
```
This branch has the required edits to properly deploy the OriginTrail smart contracts to a starfleet node.

For now the smart contract deployment is only possible over web3. We would like to enable smart contract deployment with truffle, there are more details about that in a later section, this section focuses on how we were able to deploy the OriginTrail smart contracts to the substrate node.
Firstly, from the `starfleet-node` folder run the following commands to install the required dependencies

```
cd tests
nvm use 15.2.1
npm install
```

Now we can deploy the origintrail contracts using the following command:

```
node contract_deploy_web3.js
```

The migration file first deploys a Hub contract, which serves as an index of the addresses of all other OriginTrail contracts, then creates the other OriginTrail contracts using the Hub contract’s address as a constructor parameter, updating the Hub contract to contain the index for each contract. 

This test verifies the deployment of our contracts, enabling contract deployment with constructor parameters required a few iterations to succeed, but from this point we’re confident that we can deploy the entire suite of OriginTrail smart contracts. 


If the migration completes successfully you should see a log similar to the one below:
```
Hub contract address: 0x42e2EE7Ba8975c473157634Ac2AF4098190fc741
Approval contract address: 0xD96744C4376bBeF638d9608D02339d32209760Df
Token contract address: 0x3C841844cFe11d3999eD5c0B0d1714cC1eBB23dc
Profile contract address: 0x498f3CA5e0d179505dF4911F0c8FE3484A65c630
Holding contract address: 0x826b0e0B03f22C5E58557456bD8b8EDe318C2e0A
Litigation contract address: 0xB45104bE42cfBC6A6Cbf293EbDa97A0d35F2c987
Marketplace contract address: 0x29C1d359fb983003128DfFC84d5789A847C9B00c
Replacement contract address: 0x0b8413F1E57E3585eAe3F7Be1266e10b9E530DB3
ProfileStorage contract address: 0xaf810F20e36De6Dd64Eb8FA2e8FAc51D085c1De3
HoldingStorage contract address: 0x64c058dF7710CECC01481A474A03Da309cf2DF36
LitigationStorage contract address: 0x2681a2407F87303a738e77cFA17864D16612FF3a
MarketplaceStorage contract address: 0x653fDd8f2CF84c91997A462eF799c92c91269942
```
  

Copy the Hub contract address value somewhere, it might be required when setting up the nodes.

### OriginTrail node setup and startup

Firstly, make sure that you have the prerequisites for ot-node installed, you can find them on our [ReadTheDocs](http://docs.origintrail.io/en/latest/Running-a-Node/getting-started.html#manual-installation)

Switch to version 9 of nodejs and Install the dependencies for ot-node with the following commands:
```
nvm use 9.9.0
npm install
```
Because this is considered a development environment we need to add that as an environment variable so that the correct configuration parameters are used, which can be done by running the following command inside the ot-node folder.
```
echo “NODE_ENV=development” > .env
```
When the node modules are installed, we can start 4 nodes to test the network. For ease of use, we have pre-generated 4 nodes and their configurations in the starfleet node repo.

>NOTE: You might need to make one edit to the pre-generated configuration files. If your Hub contract address we mentioned earlier is different than `0x42e2EE7Ba8975c473157634Ac2AF4098190fc741` then you should open each .json file in the starfleet-node/ot-node-configs folder and edit the `hub_contract_address` to the value you received when deploying the smart contracts. Save the change and continue to the node startup :)

For each node open a new terminal window and run the following commands from inside the ot-node folder. Since Node 1 is also the bootstrap node, it is recommended to start that node first and once it has successfully finished startup move on to the other 3 nodes. You’ll see an `OT Node started` log line when the startup process is complete

  

Node 1: 
```
npm run setup -- --configDir=../starfleet-node/tests/ot-node-configs/DCG-config --config=../starfleet-node/tests/ot-node-configs/DCG.json
npm start -- --configDir=../starfleet-node/tests/ot-node-configs/DCG-config --config=../starfleet-node/tests/ot-node-configs/DCG.json
```  

Node 2:

 ```
npm run setup -- --configDir=../starfleet-node/tests/ot-node-configs/DHG1-config --config=../starfleet-node/tests/ot-node-configs/DHG1.json
npm start -- --configDir=../starfleet-node/tests/ot-node-configs/DHG1-config --config=../starfleet-node/tests/ot-node-configs/DHG1.json
```

Node 3:
```
npm run setup -- --configDir=../starfleet-node/tests/ot-node-configs/DHG2-config --config=../starfleet-node/tests/ot-node-configs/DHG2.json
npm start -- --configDir=../starfleet-node/tests/ot-node-configs/DHG2-config --config=../starfleet-node/tests/ot-node-configs/DHG2.json
```

Node 4:
  
```
npm run setup -- --configDir=../starfleet-node/tests/ot-node-configs/DHG3-config --config=../starfleet-node/tests/ot-node-configs/DHG3.json
npm start -- --configDir=../starfleet-node/tests/ot-node-configs/DHG3-config --config=../starfleet-node/tests/ot-node-configs/DHG3.json
```

### Testing dataset replication
With all 4 node started we can test replicating a dataset over the network. Import the dataset on the first node with the following command

  
```
curl -d '{"standard_id": "OT-JSON", "file": "{\"@graph\":[{\"identifiers\":[{\"@type\":\"id\",\"@value\":\"test_object_01\"}],\"properties\":{\"purpose\":\"Testingobjectnumber1\"},\"@id\":\"test_object_01\",\"@type\":\"otObject\",\"relations\":[]}]}"}' -H 'Content-Type: application/json' http://localhost:8900/api/latest/import
```
Once you see the `Import complete` log line you can start the replication process with the following command
```
curl -d '{"dataset_id": "0x8e43a6b345af96d8e1c23d8885b934f2d0eb3bf5e420a244abc9e2b81a40e481", "holding_time_in_minutes": 5}' -H 'Content-Type: application/json' http://localhost:8900/api/latest/replicate
```
After a few minutes, the replication process should be completed with the 3 DH nodes showing that they’ve been chosen for the offer.
