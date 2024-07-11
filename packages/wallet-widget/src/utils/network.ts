import type { DogeNetworkId } from "doge-sdk";

function getNetworkNameById(id: DogeNetworkId){
  if(id === "doge"){
    return "Dogecoin";
  }else if(id === "dogeTestnet"){
    return "Dogecoin Testnet";
  }else if(id === "dogeRegtest"){
    return "Dogecoin Regtest";
  }else{
    return "Unknown";
  }
}

export {
  getNetworkNameById,
}