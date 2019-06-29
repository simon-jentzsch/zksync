pragma solidity ^0.5.8;


contract Franklin {

    // Address which will excercise governance over the network
    // i.e. add tokens, change validator set, conduct upgrades
    address public networkGovernor;

    // Address of the account which is allowed to trigger exodus mode
    // (mass exits in the case that censorship resistance has failed)
    address public exitQueue;

    // Total number of ERC20 tokens registered in the network 
    // (excluding ETH, which is hardcoded as tokenId = 0)
    uint32 public totalTokens;

    // List of registered tokens by tokenId
    mapping (uint32 => address) public tokens;

    // List of permitted validators
    mapping (address => bool) public validators;

    // Root-chain balance: users can send funds from and to Franklin 
    // from the root-chain balances only (see docs)
    struct Balance {
        uint112 balance;

        // Locked amount is necessary for deposits, see docs
        uint112 amountLocked;

        // Locked amount becomes free at ETH blockNumber = lockedUntilBlock
        uint32  lockedUntilBlock;
    }

    // List of root-chain balances (per owner and tokenId)
    mapping (address => mapping (uint32 => Balance)) public balances;

    // Type of block processing operation holder
    enum HolderType {
        Deposit,
        Withdraw
    }

    // Holders keep balances for processing the committed data in blocks, see docs
    struct Holder {
        HolderType  opType;
        uint32      tokenId;
        address     owner;
        uint112     amount;
    }

    // Total number of registered holders
    uint totalHolders;

    // List of holders by index
    mapping (uint64 => Holder) holders;

    // Total number of verified blocks
    // i.e. blocks[totalBlocksVerified] points at the latest verified block (block 0 is genesis)
    uint256 public totalBlocksVerified;

    // Total number of committed blocks
    // i.e. blocks[totalBlocksCommitted] points at the latest committed block
    uint256 public totalBlocksCommitted;

    // Block data (once per block)
    struct Block {

        // Hash of committment to public data for the block circuit
        bytes32 dataCommitment;

        // New root hash
        bytes32 stateRoot;

        // ETH block number at which this block was committed
        uint32  committedAtBlock;

        // ETH block number at which this block was verified
        uint32  verifiedAtBlock;

        // Validator (aka block producer)
        address validator;

        // Index of the first operation to process for this block
        uint64  operationStartId;

        // Total number of operations to process for this block
        uint64  totalOperations;

        // Stored here for reverting expired blocks
        uint32 blockNumber;
    }

    // List of blocks by Franklin blockId
    mapping (uint32 => Block) public blocks;

    // Total number of registered blocks to revert (see docs)
    uint32 totalBlocksToRevert;

    // List of blocks by revertBlockId (see docs)
    mapping (uint32 => Block) public blocksToRevert;

    // Flag indicating that exodus (mass exit) mode is triggered
    // Once it was raised, it can not be cleared again, and all users must exit
    bool public exodusMode;

    // Flag indicating that a user has exited certain token balance (per owner and tokenId)
    mapping (address => mapping (uint32 => bool)) public exited;


    // Constructor

    constructor(bytes32 _genesisRoot, address _exitQueue, address _networkGovernor) public {
        blocks[0].stateRoot = _genesisRoot;
        exitQueue = _exitQueue;
        networkGovernor = _networkGovernor;

        // TODO: remove once governance is implemented
        validators[_networkGovernor] = true;
    }


    // Governance

    function changeGovernor(address _newGovernor) external {
        requireGovernor();
        networkGovernor = _newGovernor;
    }

    function addToken(address _token) external {
        requireGovernor();
        tokens[totalTokens + 1] = _token; // Adding one because tokenId = 0 is reserved for ETH
        totalTokens++;
    }

    function setValidator(address _validator, bool _active) external {
        requireGovernor();
        validators[_validator] = _active;
    }

    // TODO: implement migration by moving ETH and all token balances to a new contract, triggered by networkGovernor


    // Root-chain balances

    // Deposit ETH (simply by sending it to the contract)
    function() external {
        requireActive();
        revert("unimplemented");
    }

    function withdrawETH(uint112 _amount) external {
        requireActive();
        revert("unimplemented");
    }

    function depositERC20(address _token, uint112 _amount) external {
        requireActive();
        revert("unimplemented");
    }

    function withdrawERC20(address _token, uint112 _amount) external {
        requireActive();
        revert("unimplemented");
    }

    function registerDeposit(address token, uint112 _amount) internal {
        revert("unimplemented");
    }

    function registerWithdrawal(address token, uint112 _amount) internal {
        revert("unimplemented");
    }


    // Block committment

    function commitBlock(uint32 _blockNumber, bytes32 _newRoot, bytes calldata _dataCommitment) external {
        requireActive();
        require(validators[msg.sender], "only by validator");
        require(_blockNumber == totalBlocksCommitted + 1, "commit next block");

        // TODO: check that first committed has not expired yet
        // TODO: check that first committed is not more than 300 blocks away
        // TODO: enforce one commitment per eth block

        // TODO: check status at exit queue
        // TODO: store commitment
        // TODO: pre-process holders (up to max number of operation)
    }


    // Block verification

    function verifyBlock(uint32 _blockNumber, bytes calldata _proof) external {
        requireActive();
        require(validators[msg.sender], "only by validator");
        require(_blockNumber == totalBlocksVerified + 1, "verify next block");

        // TODO: check that committed has not expired yet

        // TODO: verify proof against commitment and increment totalBlocksVerified
        // TODO: post-process holders (up to max number of operation)
        // TODO: clear holders from the commitment
    }


    // Reverting committed blocks

    function revertExpiredBlocks() external {
        // TODO: check that committed expired
        // TODO: move blocks to the list of committed
    }

    function unprocessRevertedBlock(uint32 _revertedBlockId) external {
        // TODO: return deposits
    }


    // Exodus mode

    function triggerExodus() external {
        require(msg.sender == exitQueue, "only by exit queue");
        exodusMode = true;
    }

    function exit(uint32 _tokenId, address[] calldata _owners, uint112[] calldata _amounts, bytes calldata _proof) external {
        require(exodusMode, "must be in exodus mode");
        require(_owners.length == _amounts.length, "|owners| != |amounts|");

        for(uint256 i = 0; i < _owners.length; i++) {
            require(exited[_owners[i]][_tokenId] == false, "already exited");
        }

        // TODO: verify SNARK proof that all users have the specified amounts of this token in the latest state

        for(uint256 i = 0; i < _owners.length; i++) {
            balances [_owners[i]][_tokenId].balance += _amounts[i];
            exited   [_owners[i]][_tokenId] = true;
        }
    }


    // Helpers

    function requireGovernor() internal view {
        require(msg.sender == networkGovernor, "only by governor");
    }

    function requireActive() internal view {
        require(!exodusMode, "exodus mode");
    }

}