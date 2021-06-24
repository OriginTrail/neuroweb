// const PrivateKeyProvider = require('./private-provider');
var PrivateKeyProvider = require("truffle-privatekey-provider");
// Standalone Development Node Private Key
const privateKeyDev =
   'e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a';

module.exports = {
   networks: {
      // Standalode Network
      dev: {
         provider: () => {
            if (!privateKeyDev.trim()) {
               throw new Error('Please enter a private key with funds, you can use the default one');
            }
            return new PrivateKeyProvider(privateKeyDev, 'http://localhost:9933/', 42)
         },
         network_id: 42,
         skipDryRun: true,
      },
   },
   compilers: {
      solc: {
         version: "^0.4.24"
      }
   },
};
