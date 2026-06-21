/**
 * Built-in data types in DPN
 */
export declare enum DPNBuiltInDataType {
    Target = 0,
    Bool = 1,
    U32Target = 2,
    HashOut = 3,
    HashOut160 = 4,
    TargetArray = 5,
    BoolArray = 6,
    U32TargetArray = 7,
    Unknown = 63
}
/**
 * Operation types in DPN
 */
export declare enum DPNOpType {
    InputTarget = 0,
    Constant = 1,
    ConstantTrue = 2,
    ConstantFalse = 3,
    Add = 4,
    Sub = 5,
    Mul = 6,
    Div = 7,
    BoolNot = 8,
    BoolAnd = 9,
    BoolOr = 10,
    Xor = 11,
    Nor = 12,
    Eq = 13,
    Lte = 14,
    Gte = 15,
    Gt = 16,
    Lt = 17,
    SplitBits = 18,
    SumBits = 19,
    TargetAt = 20,
    HashNoPad = 21,
    HashPad = 22,
    Select = 23,
    Exp = 24,
    ExpConstantPower = 25,
    ExpConstantBase = 26,
    Mod = 27,
    ModConstantDividend = 28,
    ModConstantDivisor = 29,
    DivRem4 = 30,
    CastU32 = 31,
    U32And = 32,
    U32AndConstant = 33,
    U32Or = 34,
    U32OrConstant = 35,
    U32Xor = 36,
    U32XorConstant = 37,
    U32ShiftLeft = 38,
    U32ShiftLeftConstantBitDistance = 40,
    U32ShiftLeftConstantValue = 41,
    U32ShiftRight = 42,
    U32ShiftRightConstantBitDistance = 43,
    U32ShiftRightConstantValue = 44,
    CalculateMerkleRoot = 45,
    GetUserId = 46,
    GetContractId = 47,
    GetCheckpointId = 48,
    GetNonce = 49,
    GetUserPublicKeyHash = 50,
    GetStateQueryResult = 51,
    GetStateQueryResultSingle = 52,
    GetStateCommandResultHash = 53,
    GetStateCommandResultSingle = 54,
    GetStateCommandResultArray = 55,
    UnaryInverse = 64,
    UnaryNegative = 65,
    U32InputTarget = 66,
    ConstantU32 = 67,
    U32Add = 68,
    U32Sub = 69,
    U32Mul = 70,
    U32Div = 71,
    CastFelt = 72,
    CastBool = 73,
    BoolInputTarget = 74,
    U32Mod = 75,
    U32Exp = 76,
    Secp256k1Verify = 77,
    HashTwoToOne = 78,
    GetCallerContractId = 79
}
/**
 * State command types in DPN
 */
export declare enum DPNStateCommandType {
    SetContractStateSlotHash = 0,
    SetContractStateSlotSingle = 1,
    SetContractStateSlotRange = 2,
    InvokeExternalContractFunctionSync = 8,
    InvokeExternalContractFunctionDeferred = 9,
    GetSelfUserCurrentContractStateSlotHash = 16,
    GetSelfUserCurrentContractStateSlotSingle = 17,
    GetSelfUserCurrentContractStateSlotRange = 18,
    GetSelfUserExternalContractStateSlotHash = 24,
    GetSelfUserExternalContractStateSlotSingle = 25,
    GetSelfUserExternalContractStateSlotRange = 26,
    GetOtherUserContractStateSlotHash = 32,
    GetOtherUserContractStateSlotSingle = 33,
    GetOtherUserContractStateSlotRange = 34
}
/**
 * Definition of an indexed variable
 */
export interface DPNIndexedVarDef {
    data_type: DPNBuiltInDataType;
    index: number;
    op_type: DPNOpType;
    inputs: number[];
}
/**
 * Information about an assertion
 */
export interface DPNAssertEqInfoIndexed {
    left: number;
    right: number;
    message: string;
}
/**
 * Set contract state slot hash command
 */
export interface DPNStateCmdSetContractStateSlotHash {
    type: DPNStateCommandType.SetContractStateSlotHash;
    condition: number;
    slot_index: number;
    value: [number, number, number, number];
}
/**
 * Set contract state slot single command
 */
export interface DPNStateCmdSetContractStateSlotSingle {
    type: DPNStateCommandType.SetContractStateSlotSingle;
    condition: number;
    sub_slot_index: number;
    value: number;
}
/**
 * Set contract state slot range command
 */
interface DPNStateCmdSetContractStateSlotRange {
    type: DPNStateCommandType.SetContractStateSlotRange;
    condition: number;
    sub_slot_index: number;
    value: number[];
}
/**
 * Invoke external contract function synchronously
 */
export interface DPNStateCmdInvokeExternalContractFunctionSync {
    type: DPNStateCommandType.InvokeExternalContractFunctionSync;
    condition: number;
    contract_id: number;
    method_id: number;
    input_args: number[];
    num_outputs: number;
}
/**
 * Invoke external contract function with deferred execution
 */
export interface DPNStateCmdInvokeExternalContractFunctionDeferred {
    type: DPNStateCommandType.InvokeExternalContractFunctionDeferred;
    condition: number;
    contract_id: number;
    method_id: number;
    input_args: number[];
}
/**
 * Get self user current contract state slot hash
 */
export interface DPNStateCmdGetSelfUserCurrentContractStateSlotHash {
    type: DPNStateCommandType.GetSelfUserCurrentContractStateSlotHash;
    slot_index: number;
}
/**
 * Get self user current contract state slot single
 */
export interface DPNStateCmdGetSelfUserCurrentContractStateSlotSingle {
    type: DPNStateCommandType.GetSelfUserCurrentContractStateSlotSingle;
    sub_slot_index: number;
}
/**
 * Get self user current contract state slot range
 */
export interface DPNStateCmdGetSelfUserCurrentContractStateSlotRange {
    type: DPNStateCommandType.GetSelfUserCurrentContractStateSlotRange;
    sub_slot_index: number;
    length: number;
}
/**
 * Get self user external contract state slot hash
 */
export interface DPNStateCmdGetSelfUserExternalContractStateSlotHash {
    type: DPNStateCommandType.GetSelfUserExternalContractStateSlotHash;
    contract_id: number;
    slot_index: number;
    contract_state_tree_height: number;
}
/**
 * Get self user external contract state slot single
 */
export interface DPNStateCmdGetSelfUserExternalContractStateSlotSingle {
    type: DPNStateCommandType.GetSelfUserExternalContractStateSlotSingle;
    contract_id: number;
    sub_slot_index: number;
    contract_state_tree_height: number;
}
/**
 * Get self user external contract state slot range
 */
export interface DPNStateCmdGetSelfUserExternalContractStateSlotRange {
    type: DPNStateCommandType.GetSelfUserExternalContractStateSlotRange;
    contract_id: number;
    sub_slot_index: number;
    length: number;
    contract_state_tree_height: number;
}
/**
 * Get other user contract state slot hash
 */
export interface DPNStateCmdGetOtherUserContractStateSlotHash {
    type: DPNStateCommandType.GetOtherUserContractStateSlotHash;
    user_id: number;
    contract_id: number;
    slot_index: number;
    contract_state_tree_height: number;
}
/**
 * Get other user contract state slot single
 */
export interface DPNStateCmdGetOtherUserContractStateSlotSingle {
    type: DPNStateCommandType.GetOtherUserContractStateSlotSingle;
    user_id: number;
    contract_id: number;
    sub_slot_index: number;
    contract_state_tree_height: number;
}
/**
 * Get other user contract state slot range
 */
export interface DPNStateCmdGetOtherUserContractStateSlotRange {
    type: DPNStateCommandType.GetOtherUserContractStateSlotRange;
    user_id: number;
    contract_id: number;
    sub_slot_index: number;
    length: number;
    contract_state_tree_height: number;
}
/**
 * Union type for all state commands
 */
export type DPNStateCmd = DPNStateCmdSetContractStateSlotHash | DPNStateCmdSetContractStateSlotSingle | DPNStateCmdSetContractStateSlotRange | DPNStateCmdInvokeExternalContractFunctionSync | DPNStateCmdInvokeExternalContractFunctionDeferred | DPNStateCmdGetSelfUserCurrentContractStateSlotHash | DPNStateCmdGetSelfUserCurrentContractStateSlotSingle | DPNStateCmdGetSelfUserCurrentContractStateSlotRange | DPNStateCmdGetSelfUserExternalContractStateSlotHash | DPNStateCmdGetSelfUserExternalContractStateSlotSingle | DPNStateCmdGetSelfUserExternalContractStateSlotRange | DPNStateCmdGetOtherUserContractStateSlotHash | DPNStateCmdGetOtherUserContractStateSlotSingle | DPNStateCmdGetOtherUserContractStateSlotRange;
/**
 * Definition of a function circuit
 */
export interface DPNFunctionCircuitDefinition {
    name: string;
    method_id: number;
    circuit_inputs: number[];
    circuit_outputs: number[];
    state_commands: DPNStateCmd[];
    state_command_resolution_indices: number[];
    assertions: DPNAssertEqInfoIndexed[];
    definitions: DPNIndexedVarDef[];
}
export {};
//# sourceMappingURL=vmTypes.d.ts.map