import { Network } from "bitcoinjs-lib";
import { DogeNetwork } from "../types/network";

const doge: Network = {
  messagePrefix: '\x19Dogecoin Signed Message:\n',
  bech32: 'dge', // doge doesn't have bech32
  bip32: {
    public: 0x02facafd,
    private: 0x02fac398,
  },
  pubKeyHash: 0x1e,
  scriptHash: 0x16,
  wif: 0x9e,
};
const dogeRegtest: Network = {
  messagePrefix: '\x19Dogecoin Signed Message:\n',
  bech32: 'dgr', // doge doesn't have bech32
  bip32: {
    public: 0x043587cf,
    private: 0x04358394,
  },
  pubKeyHash: 0x6f,
  scriptHash: 0xc4,
  wif: 0xef,
};
const dogeTestnet: Network = {
  messagePrefix: '\x19Dogecoin Signed Message:\n',
  bech32: 'dgt', // doge doesn't have bech32
  bip32: {
    public: 0x043587cf,
    private: 0x04358394,
  },
  pubKeyHash: 0x71,
  scriptHash: 0xc4,
  wif: 0xf1,
};



const dogeNetworks: Record<DogeNetwork, Network> = {
  doge,
  dogeRegtest,
  dogeTestnet,

};




function getNetworkById(name: string): Network {
  if(Object.hasOwnProperty.call(dogeNetworks, name)){
    return dogeNetworks[name as DogeNetwork];
  }
  if(name === "dogeTestnet"){
    return dogeNetworks.dogeTestnet;
  }else if(name === "dogeRegtest"){
    return dogeNetworks.dogeRegtest;
  }else if(name === "doge"){
    return dogeNetworks.doge;
  }else{
    throw new Error("unsupported network type '"+name+"'");
  }
}



export {
  getNetworkById,
  dogeNetworks,
}