pragma solidity 0.5.16;

contract DkgSizeOracle {
    function calculateDkgSize(uint vertices, uint edges) public pure returns(uint d) {
        return vertices * 5000 + edges * 3000;
    }
}
