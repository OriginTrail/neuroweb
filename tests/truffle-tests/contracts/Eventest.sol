pragma solidity ^0.4.24;


contract Eventest {
    event EventThree(uint256 firstValue, address secondValue);
    event EventFour(uint256 firstValue, uint256 indexed secondValue, uint256 thirdValue);

    function emitEventThree(uint256 _one, address _two) public {
        emit EventThree(_one, _two);
    }

    function emitEventFour(uint256 _one, uint256 _two, uint256 _three) public {
        emit EventFour(_one, _two, _three);
    }
}
