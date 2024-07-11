import { IFieldValidatorReal, ons, validate } from '@qstudio/one-schema';
import { REPLParseError } from './cmdError';
import {CityRPCCommand, CityRPCCommandRequest} from '@qstudio/city-sdk';

const MAX_CHECKPOINT_ID = 4294967295;


interface ICityREPLCommandDef {
  commandType: CityRPCCommand,
  name: string;
  description: string;
  aliases: string[];
  arguments: IFieldValidatorReal[];
  processCommand: (args: any[]) => CityRPCCommandRequest;
}
function getCheckpointIdValidator(description: string){
  return ons().int32().min(0).tags(["checkpoint_id"]).description(description).defaultValue(MAX_CHECKPOINT_ID);
}
function getUserIdValidator(description: string){
  return ons().int32().min(0).tags(["user_id"]).required().description(description);
}
function getLeafIdValidator(description: string){
  return ons().int32().min(0).tags(["leaf_id"]).required().description(description);
}
function getHash256Validator(fieldName: string, description: string){
  return ons().string().max(64).min(64).validator("hex_string").tags([fieldName]).required().description(description);
}
function getPublicKeyValidator(fieldName: string, description: string){
  return ons().string().max(33).min(33).validator("hex_string").tags([fieldName]).required().description(description);
}
const commandDefs: ICityREPLCommandDef[] = [
  {
    commandType: CityRPCCommand.GetUserTreeRoot,
    name: "getUserTreeRoot",
    description: "Get the root of the user tree for a given checkpoint",
    aliases: ["gutr"],
    arguments: [
      getCheckpointIdValidator("The checkpoint id to get the user tree root for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetUserTreeRoot,
        params: {
          checkpoint_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetUserIdsForPublicKey,
    name: "getUserIdsForPublicKey",
    description: "Get the user ids for a given public key",
    aliases: ["guifpk","uidsforpk"],
    arguments: [
      getPublicKeyValidator("public_key", "The public key to get the user ids for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetUserIdsForPublicKey,
        params: {
          public_key: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetUserById,
    name: "getUserById",
    description: "Get a user by checkpoint id and user id",
    aliases: ["user", "u"],
    arguments: [
      getUserIdValidator("The user id to get"),
      getCheckpointIdValidator("The checkpoint id to get the user for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetUserById,
        params: {
          checkpoint_id: args[1],
          user_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetUserMerkleProofById,
    name: "getUserMerkleProofById",
    description: "Get the merkle proof for a user by checkpoint id and user id",
    aliases: ["gumpid"],
    arguments: [
      getUserIdValidator("The user id to get the merkle proof for"),
      getCheckpointIdValidator("The checkpoint id to get the user merkle proof for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetUserMerkleProofById,
        params: {
          checkpoint_id: args[0],
          user_id: args[1],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetUserTreeLeaf,
    name: "getUserTreeLeaf",
    description: "Get a user tree leaf by checkpoint id and leaf id",
    aliases: ["gutl"],
    arguments: [
      getLeafIdValidator("The leaf id to get"),
      getCheckpointIdValidator("The checkpoint id to get the user tree leaf for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetUserTreeLeaf,
        params: {
          checkpoint_id: args[1],
          leaf_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetUserTreeLeafMerkleProof,
    name: "getUserTreeLeafMerkleProof",
    description: "Get the merkle proof for a user tree leaf by checkpoint id and leaf id",
    aliases: ["gutlmp","utreeleaf"],
    arguments: [
      getLeafIdValidator("The leaf id to get the merkle proof for"),
      getCheckpointIdValidator("The checkpoint id to get the user tree leaf merkle proof for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetUserTreeLeafMerkleProof,
        params: {
          checkpoint_id: args[1],
          leaf_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetDepositTreeRoot,
    name: "getDepositTreeRoot",
    description: "Get the root of the deposit tree for a given checkpoint",
    aliases: ["gdtr"],
    arguments: [
      getCheckpointIdValidator("The checkpoint id to get the deposit tree root for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetDepositTreeRoot,
        params: {
          checkpoint_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetDepositById,
    name: "getDepositById",
    description: "Get a deposit by checkpoint id and deposit id",
    aliases: ["gdid","deposit","d"],
    arguments: [
      ons().int32().min(0).tags(["deposit_id"]).required().description("The deposit id to get"),
      getCheckpointIdValidator("The checkpoint id to get the deposit for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetDepositById,
        params: {
          checkpoint_id: args[1],
          deposit_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetDepositsById,
    name: "getDepositsById",
    description: "Get deposits by checkpoint id and deposit ids",
    aliases: ["gdbid"],
    arguments: [
      ons().array().arrayOf(ons().int32().min(0).tags(["deposit_id"]).required().description("The deposit id to get")).required().description("The deposit ids to get"),
      getCheckpointIdValidator("The checkpoint id to get the deposits for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetDepositsById,
        params: {
          checkpoint_id: args[1],
          deposit_ids: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetDepositByTxid,
    name: "getDepositByTxid",
    description: "Get a deposit by transaction id",
    aliases: ["gdbtid","depositbytxid","dtxid"],
    arguments: [
      getHash256Validator("transaction_id", "The transaction id to get the deposit for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetDepositByTxid,
        params: {
          transaction_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetDepositsByTxid,
    name: "getDepositsByTxid",
    description: "Get deposits by transaction ids",
    aliases: ["depositsbytxids","dtxids"],
    arguments: [
      ons().array().arrayOf(ons().string().tags(["transaction_id"]).required().description("The transaction id to get the deposit for")).required().description("The transaction ids to get the deposits for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetDepositsByTxid,
        params: {
          transaction_ids: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetDepositHash,
    name: "getDepositHash",
    description: "Get the hash of a deposit by checkpoint id and deposit id",
    aliases: ["gdh","dhash"],
    arguments: [
      ons().int32().min(0).tags(["deposit_id"]).required().description("The deposit id to get the hash for"),
      getCheckpointIdValidator("The checkpoint id to get the deposit hash for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetDepositHash,
        params: {
          checkpoint_id: args[1],
          deposit_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetDepositLeafMerkleProof,
    name: "getDepositLeafMerkleProof",
    description: "Get the merkle proof for a deposit leaf by checkpoint id and deposit id",
    aliases: ["gdlmp","dleaf"],
    arguments: [
      ons().int32().min(0).tags(["deposit_id"]).required().description("The deposit id to get the merkle proof for"),
      getCheckpointIdValidator("The checkpoint id to get the deposit leaf merkle proof for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetDepositLeafMerkleProof,
        params: {
          checkpoint_id: args[1],
          deposit_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetBlockState,
    name: "getBlockState",
    description: "Get the block state for a given checkpoint",
    aliases: ["gbs","bstate"],
    arguments: [
      getCheckpointIdValidator("The checkpoint id to get the block state for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetBlockState,
        params: {
          checkpoint_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetLatestBlockState,
    name: "getLatestBlockState",
    description: "Get the latest block state",
    aliases: ["glbs","lbstate","latestbstate"],
    arguments: [],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetLatestBlockState,
        params: undefined,
      };
    },
  },
  {
    commandType: CityRPCCommand.GetCityRoot,
    name: "getCityRoot",
    description: "Get the root of the city rollup state tree for a given checkpoint",
    aliases: ["gcr"],
    arguments: [
      getCheckpointIdValidator("The checkpoint id to get the city root for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetCityRoot,
        params: {
          checkpoint_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetCityBlockScript,
    name: "getCityBlockScript",
    description: "Get the P2SH script for a city rollup block",
    aliases: ["gcbs"],
    arguments: [
      getCheckpointIdValidator("The checkpoint id to get the city rollup block script for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetCityBlockScript,
        params: {
          checkpoint_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetCityBlockDepositAddress,
    name: "getCityBlockDepositAddress",
    description: "Get the P2SH deposit address for a city block",
    aliases: ["gcbda"],
    arguments: [
      getCheckpointIdValidator("The checkpoint id to get the city block deposit address for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetCityBlockDepositAddress,
        params: {
          checkpoint_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetCityBlockDepositAddressString,
    name: "getCityBlockDepositAddressString",
    description: "Get the deposit address string for a city block",
    aliases: ["gcbdads","daddr","depositaddress"],
    arguments: [
      getCheckpointIdValidator("The checkpoint id to get the city block deposit address string for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetCityBlockDepositAddressString,
        params: {
          checkpoint_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetWithdrawalTreeRoot,
    name: "getWithdrawalTreeRoot",
    description: "Get the root of the withdrawal tree for a given checkpoint",
    aliases: ["gwtr","wtr"],
    arguments: [
      getCheckpointIdValidator("The checkpoint id to get the withdrawal tree root for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetWithdrawalTreeRoot,
        params: {
          checkpoint_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetWithdrawalById,
    name: "getWithdrawalById",
    description: "Get a withdrawal by checkpoint id and withdrawal id",
    aliases: ["gwid","withdrawal","w"],
    arguments: [
      ons().int32().min(0).tags(["withdrawal_id"]).required().description("The withdrawal id to get"),
      getCheckpointIdValidator("The checkpoint id to get the withdrawal for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetWithdrawalById,
        params: {
          checkpoint_id: args[1],
          withdrawal_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetWithdrawalsById,
    name: "getWithdrawalsById",
    description: "Get withdrawals by checkpoint id and withdrawal ids",
    aliases: ["gwdbid","withdrawals","ws"],
    arguments: [
      ons().array().arrayOf(ons().int32().min(0).tags(["withdrawal_id"]).required().description("The withdrawal id to get")).required().description("The withdrawal ids to get"),
      getCheckpointIdValidator("The checkpoint id to get the withdrawals for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetWithdrawalsById,
        params: {
          checkpoint_id: args[1],
          withdrawal_ids: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetWithdrawalHash,
    name: "getWithdrawalHash",
    description: "Get the hash of a withdrawal by checkpoint id and withdrawal id",
    aliases: ["gwh","whash","wh"],
    arguments: [
      ons().int32().min(0).tags(["withdrawal_id"]).required().description("The withdrawal id to get the hash for"),
      getCheckpointIdValidator("The checkpoint id to get the withdrawal hash for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetWithdrawalHash,
        params: {
          checkpoint_id: args[1],
          withdrawal_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetWithdrawalLeafMerkleProof,
    name: "getWithdrawalLeafMerkleProof",
    description: "Get the merkle proof for a withdrawal leaf by checkpoint id and withdrawal id",
    aliases: ["gwlmh","wleaf"],
    arguments: [
      ons().int32().min(0).tags(["withdrawal_id"]).required().description("The withdrawal id to get the merkle proof for"),
      getCheckpointIdValidator("The checkpoint id to get the withdrawal leaf merkle proof for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetWithdrawalLeafMerkleProof,
        params: {
          checkpoint_id: args[1],
          withdrawal_id: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetProofStoreValue,
    name: "getProofStoreValue",
    description: "Get a value from the proof store by checkpoint id and key",
    aliases: ["gpsv","psvalue"],
    arguments: [
      ons().string().tags(["key"]).required().description("The key to get the value for"),
      getCheckpointIdValidator("The checkpoint id to get the proof store value for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetProofStoreValue,
        params: {
          checkpoint_id: args[1],
          key: args[0],
        }
      };
    },
  },
  {
    commandType: CityRPCCommand.GetProofStoreValues,
    name: "getProofStoreValues",
    description: "Get values from the proof store by checkpoint id and keys",
    aliases: ["gpsvs", "psvalues"],
    arguments: [
      ons().array().arrayOf(ons().string().tags(["key"]).required().description("The key to get the value for")).required().description("The keys to get the values for"),
      getCheckpointIdValidator("The checkpoint id to get the proof store values for"),
    ],
    processCommand: (args) => {
      return {
        commandType: CityRPCCommand.GetProofStoreValues,
        params: {
          checkpoint_id: args[1],
          keys: args[0],
        }
      };
    },
  },
];

export type {
  ICityREPLCommandDef,
}

export {
  commandDefs,
}