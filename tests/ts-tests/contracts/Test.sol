pragma solidity 0.5.16;

contract Test {
    function multiply(uint a) public pure returns(uint d) {
        return a * 7;
    }
    function currentBlock() public view returns(uint) {
        return block.number;
    }
    function blockHash(uint number) public view returns(bytes32) {
        return blockhash(number);
    }
}
