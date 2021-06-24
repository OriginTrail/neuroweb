cd ../../../ot-node/

nvm use 9.9.0

npm run setup -- --configDir=../starfleet-node/tests/ot-node-configs/DCG-config --config=../starfleet-node/tests/ot-node-configs/DCG.json
npm start -- --configDir=../starfleet-node/tests/ot-node-configs/DCG-config --config=../starfleet-node/tests/ot-node-configs/DCG.json

npm run setup -- --configDir=../starfleet-node/tests/ot-node-configs/DHG1-config --config=../starfleet-node/tests/ot-node-configs/DHG1.json
npm start -- --configDir=../starfleet-node/tests/ot-node-configs/DHG1-config --config=../starfleet-node/tests/ot-node-configs/DHG1.json

npm run setup -- --configDir=../starfleet-node/tests/ot-node-configs/DHG2-config --config=../starfleet-node/tests/ot-node-configs/DHG2.json
npm start -- --configDir=../starfleet-node/tests/ot-node-configs/DHG2-config --config=../starfleet-node/tests/ot-node-configs/DHG2.json

npm run setup -- --configDir=../starfleet-node/tests/ot-node-configs/DHG3-config --config=../starfleet-node/tests/ot-node-configs/DHG3.json
npm start -- --configDir=../starfleet-node/tests/ot-node-configs/DHG3-config --config=../starfleet-node/tests/ot-node-configs/DHG3.json

#Import dataset command
curl -d '{"standard_id": "OT-JSON", "file": "{\"@graph\":[{\"identifiers\":[{\"@type\":\"id\",\"@value\":\"test_object_01\"}],\"properties\":{\"purpose\":\"Testingobjectnumber1\"},\"@id\":\"test_object_01\",\"@type\":\"otObject\",\"relations\":[]}]}"}' -H 'Content-Type: application/json' http://localhost:8900/api/latest/import
#Replicate dataset command
curl -d '{"dataset_id": "0x8e43a6b345af96d8e1c23d8885b934f2d0eb3bf5e420a244abc9e2b81a40e481", "holding_time_in_minutes": 5}' -H 'Content-Type: application/json' http://localhost:8900/api/latest/replicate


