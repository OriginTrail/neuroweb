pragma solidity ^0.4.24;


contract Eventer {
    event EventOne(uint256 firstValue, bool secondValue);
    event EventTwo(uint256 firstValue, bytes32 indexed secondValue);

    function emitEventOne(uint256 _one, bool _two) public {
        emit EventOne(_one, _two);
    }

    function emitEventTwo(uint256 _one, bytes32 _two) public {
        emit EventTwo(_one, _two);
    }
}
