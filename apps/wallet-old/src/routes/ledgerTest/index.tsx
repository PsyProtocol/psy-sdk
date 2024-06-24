import SpeculosTransport from "@ledgerhq/hw-transport-node-speculos-http";
import AppBtc from '@ledgerhq/hw-app-btc';

import { Button } from '@mantine/core';
import styles from './LedgerTest.module.scss';
import { connectSpeculos } from "../../providers/ledger/speculos";
import {DogeLinkElectrsRPC, DogeLinkRPC, DogeMemoryWalletProvider, FullDogeWalletProvider, compressPublicKey, createP2PKHTransaction, createP2SHTransaction, decodeAddress, decodeBase58WithChecksum, encodeAddress, encodeBase58WithChecksum, getDogeNetworkById, getP2SHAddress, hashBuffer, hashHex, hexToU8Array, u8ArrayToHex} from 'doge-sdk';
import { exampleP2PKH, exampleComplexP2SH, exampleP2SH } from "./test";
import { signSendLedgerTx, exampleComplexP2SHv2 } from "./ledgerEx3";
import { exampleP2PKHLedger } from "./ledgerEx5";
import { LedgerHardwareWalletProvider } from "./ledgerSigner";

async function runConnectTest() {
  const transport = await connectSpeculos();
  console.log("hiii");
  const btc = new AppBtc({transport: transport});
  const result = await btc.getWalletPublicKey("44'/0'/0'/0/0", { format: "legacy" });
  console.log("result", result);
  const uncomp = hexToU8Array(result.publicKey);
  console.log("uncomp", u8ArrayToHex(uncomp));
  const comp = compressPublicKey(uncomp);
  console.log("comp", u8ArrayToHex(comp));
  const hash = hashBuffer("hash160",comp);
  console.log("hash", u8ArrayToHex(hash));
  const resultAddress = encodeAddress(hash,0);
  console.log("resultAddress", resultAddress);

  







}

async function runLedgerSignTest() {
  const transport = await connectSpeculos();
  console.log("hiii");
  // networkId can be doge, dogeTestnet, or dogeRegtest
  const networkId = "dogeRegtest";

  // your dogecoin rpc node url, with an added query equal to doge, dogeTestnet, or dogeRegtest
  const RPC_API_URL = "http://devnet:devnet@localhost:1337/bitcoin-rpc/?network="+networkId;

  const rpc = new DogeLinkRPC(RPC_API_URL);
  const btc = new AppBtc({transport: transport});
  const provider = new FullDogeWalletProvider(new LedgerHardwareWalletProvider(rpc, btc));
  const addresses = await provider.getP2PKHAddresses("dogeRegtest");
  console.log("addresses", addresses);
  const address1 = addresses[0].address;
  const signer1 = await provider.getSignerForAddress(address1);



  // wallet provider, in this case an in-memory wallet provider
  const walletProvider = new DogeMemoryWalletProvider();

  // create a random dogecoin P2PKH wallet
  const wallet1 = walletProvider.addRandomWallet(networkId);
  console.log("wallet 1 address: ", wallet1.address);

  // in dogeRegtest, we can faucet tokens to any address we like after mining some blocks
  await rpc.mineBlocks(200);
  // faucet 10 DOGE to the wallet
  const faucetTxid = await rpc.sendFromWallet(address1, 10);

  // send 9.5 DOGE from wallet1 to wallet2
  // get the funding transaction
  const faucetFundingTx = await rpc.getTransaction(faucetTxid);
  // get the unspent transaction output for wallet 1
  const faucetUTXO = faucetFundingTx.getUTXOsForAddress(address1)[0];
  // create a transaction which sends 9.5 DOGE from wallet1 to wallet2
  const txBuilder = createP2PKHTransaction(signer1, {
    inputs: [faucetUTXO],
    outputs: [{address: wallet1.address, value: 900_500_000}],
    address: address1,
  });

  // sign the transaction
  const finalizedTx = await txBuilder.finalizeAndSign();

  // broadcast the transaction
  const txid = await rpc.sendRawTransaction(finalizedTx.toHex());
  console.log("transaction id: ", txid);

  

}
function waitMs(duration: number){
  return new Promise((resolve)=>{
    setTimeout(resolve, duration);
  });
}
async function runP2PKHTest() {

  const rpc = new DogeLinkElectrsRPC("http://devnet:devnet@localhost:1337/bitcoin-rpc/?network=dogeRegtest", "http://localhost:1337/api");
  const walletProvider = new DogeMemoryWalletProvider();

  const wallet1 = walletProvider.addRandomWallet("dogeRegtest", "wallet1");

  console.log("wallet1.address", wallet1.address);
  console.log("wallet1.publicKey", wallet1.compressedPublicKeyHex);
  console.log("wallet1.hashPublicKey", hashHex("hash160", wallet1.compressedPublicKeyHex, "hex"));
  const decodedAddress = decodeAddress(wallet1.address);
  console.log("wallet1.decodedAddress", decodedAddress);
  console.log("wallet1.decodedAddress.hex", u8ArrayToHex(decodedAddress.hash));
  

  const testReEncode = encodeAddress(decodedAddress.hash, decodedAddress.version);
  console.log("wallet1.reEncode", testReEncode);
  const wallet1Pub = wallet1.compressedPublicKeyHex;

  await rpc.mineBlocks(200);
  const txid = await rpc.sendFromWallet(wallet1.address, 5);
  console.log("faucet txid: ", txid);
  await rpc.mineBlocks(1);
  await waitMs(1000);
  await rpc.mineBlocks(1);
  await waitMs(1000);
  await rpc.mineBlocks(1);
  await waitMs(5000);
  const utxos = await rpc.getUTXOs(wallet1.address);
  const utxo = utxos[0];

  const wallet2 = walletProvider.addRandomWallet("dogeRegtest", "wallet2");


  const ftx = createP2PKHTransaction(wallet1,{
    inputs: [utxo],
    outputs: [{address: wallet2.address, value: 100_000_000}],
    address: wallet1.address,
  });
  const signed = await ftx.finalizeAndSign();
  console.log("signed", signed);
  const hexTx = u8ArrayToHex(signed.toBuffer());
  console.log("hexTx", hexTx);
  const txid2 = await rpc.sendRawTransaction(hexTx);
  console.log("txid2", txid2);
  await rpc.mineBlocks(1);
  console.log("final: ",ftx);
}
async function runP2SHTest() {

  const rpc = new DogeLinkElectrsRPC("http://devnet:devnet@localhost:1337/bitcoin-rpc/?network=dogeRegtest", "http://localhost:1337/api");
  const walletProvider = new DogeMemoryWalletProvider();

  const wallet1 = walletProvider.addRandomWallet("dogeRegtest", "wallet1");
  console.log("wallet1.address", wallet1.address);
  console.log("wallet1.publicKey", wallet1.compressedPublicKeyHex);
  console.log("wallet1.hashPublicKey", hashHex("hash160", wallet1.compressedPublicKeyHex, "hex"));
  const decodedAddress = decodeAddress(wallet1.address);
  console.log("wallet1.decodedAddress", decodedAddress);
  console.log("wallet1.decodedAddress.hex", u8ArrayToHex(decodedAddress.hash));
  

  const testReEncode = encodeAddress(decodedAddress.hash, decodedAddress.version);
  console.log("wallet1.reEncode", testReEncode);
  const wallet1Pub = wallet1.compressedPublicKeyHex

  await rpc.mineBlocks(200);
  const txid = await rpc.sendFromWallet(wallet1.address, 5);
  console.log("faucet txid: ", txid);
  await rpc.mineBlocks(1);
  await waitMs(1000);
  const utxos = (await rpc.getTransaction(txid)).getUTXOsForAddress(wallet1.address);
  const utxo = utxos[0];

  const wallet2 = walletProvider.addRandomWallet("dogeRegtest", "wallet2");

  //hashBuffer("hash160", new TextEncoder().encode("hello world"));



  const REDEEM_SCRIPT = `
    <5>
    OP_ADD
    <7>
    OP_EQUAL
  `;
  const UNLOCK_SCRIPT = `
    <2>
  `;
  const p2shAddress = getP2SHAddress(REDEEM_SCRIPT, "dogeRegtest");
  console.log("p2shAddress", p2shAddress);
  
  const ftx = createP2PKHTransaction(wallet1,{
    inputs: [utxo],
    outputs: [{address: p2shAddress, value: 250_000_000}],
    address: wallet1.address,
  });
  const signed = await ftx.finalizeAndSign();
  console.log("signed", signed);
  const hexTx = u8ArrayToHex(signed.toBuffer());
  console.log("hexTx", hexTx);
  const txid2 = await rpc.sendRawTransaction(hexTx);
  console.log("txid2", txid2);
  await rpc.mineBlocks(1);
  console.log("final: ",ftx);
  await waitMs(1000);
  const utxos2 = (await rpc.getTransaction(txid2)).getUTXOsForAddress(p2shAddress);

  const utxo2 = utxos2[0];
  const ftx2 = createP2SHTransaction({
    redeemScriptBASM: REDEEM_SCRIPT,
    unlockScriptBASM: UNLOCK_SCRIPT,
    inputs: [utxo2],
    outputs: [{address: wallet2.address, value: 100_000_000}],
  });
  const signed2 = await ftx2.finalizeAndSign();
  console.log("signed2", signed2);
  const hexTx2 = u8ArrayToHex(signed2.toBuffer());
  console.log("hexTx2", hexTx2);
  const txid3 = await rpc.sendRawTransaction(hexTx2);
  console.log("txid3", txid3);

}


async function runP2SHTest2() {

  const rpc = new DogeLinkElectrsRPC("http://devnet:devnet@localhost:1337/bitcoin-rpc/?network=dogeRegtest", "http://localhost:1337/api");
  const walletProvider = new DogeMemoryWalletProvider();

  const wallet1 = walletProvider.addRandomWallet("dogeRegtest", "wallet1");
  console.log("wallet1.address", wallet1.address);
  console.log("wallet1.publicKey", wallet1.compressedPublicKeyHex);
  console.log("wallet1.hashPublicKey", hashHex("hash160", wallet1.compressedPublicKeyHex, "hex"));
  const decodedAddress = decodeAddress(wallet1.address);
  console.log("wallet1.decodedAddress", decodedAddress);
  console.log("wallet1.decodedAddress.hex", u8ArrayToHex(decodedAddress.hash));
  

  const testReEncode = encodeAddress(decodedAddress.hash, decodedAddress.version);
  console.log("wallet1.reEncode", testReEncode);
  const wallet1Pub = wallet1.getCompressedPublicKey();

  await rpc.mineBlocks(200);
  const txid = await rpc.sendFromWallet(wallet1.address, 5);
  console.log("faucet txid: ", txid);
  await rpc.mineBlocks(1);
  await waitMs(1000);
  const utxos = (await rpc.getTransaction(txid)).getUTXOsForAddress(wallet1.address);
  const utxo = utxos[0];

  const wallet2 = walletProvider.addRandomWallet("dogeRegtest", "wallet2");


  const secretString = "hello world";
  const secretStringHashHex = hashBuffer("hash160", new TextEncoder().encode(secretString), "hex");

  const wallet3 = walletProvider.addWalletFromWIF("cN1CE8kQ3QADHeumGSVvMBNaqMZUyNnKmURqEryYzNDorB7xRRab", "dogeRegtest", "wallet3");
  const pubKeyHashHex = hashBuffer("hash160", wallet3.compressedPublicKey, "hex")



  const REDEEM_SCRIPT = `
OP_HASH160
<0x${secretStringHashHex}>
OP_EQUALVERIFY 

OP_DUP
OP_HASH160
<0x${pubKeyHashHex}>
OP_EQUALVERIFY
OP_CHECKSIG
  `;
  console.log("REDEEM_SCRIPT", REDEEM_SCRIPT);
  const UNLOCK_SCRIPT = `
    <"hello world">
  `;
  const p2shAddress = getP2SHAddress(REDEEM_SCRIPT, "dogeRegtest");
  console.log("p2shAddress", p2shAddress);
  
  const ftx = createP2PKHTransaction(wallet1,{
    inputs: [utxo],
    outputs: [{address: p2shAddress, value: 250_000_000}],
    address: wallet1.address,
  });
  const signed = await ftx.finalizeAndSign();
  console.log("signed", signed);
  const hexTx = u8ArrayToHex(signed.toBuffer());
  console.log("hexTx", hexTx);
  const txid2 = await rpc.sendRawTransaction(hexTx);
  console.log("txid2", txid2);
  await rpc.mineBlocks(1);
  console.log("final: ",ftx);
  await waitMs(1000);
  const utxos2 = (await rpc.getTransaction(txid2)).getUTXOsForAddress(p2shAddress);

  const utxo2 = utxos2[0];
  const ftx2 = createP2SHTransaction({
    redeemScriptBASM: REDEEM_SCRIPT,
    unlockScriptBASM: UNLOCK_SCRIPT,
    inputs: [utxo2],
    outputs: [{address: wallet2.address, value: 100_000_000}],
    signers: [wallet3],
  });
  const signed2 = await ftx2.finalizeAndSign();
  console.log("signed2", signed2);
  const hexTx2 = u8ArrayToHex(signed2.toBuffer());
  console.log("hexTx2", hexTx2);
  const txid3 = await rpc.sendRawTransaction(hexTx2);
  console.log("txid3", txid3);

}
const LedgerTest: React.FC = () => {
  return (
    <div className={styles.ledgerTestPage}>
      <h1>Ledger Test</h1>
      <div>
        <Button onClick={()=>{
          exampleP2PKHLedger().catch(console.error);
          //exampleP2PKH()/*.then(()=>exampleP2SH()).then(()=>exampleComplexP2SH())*/.catch(console.error);
        }}>Connect</Button>
      </div>
    </div>
  );
};

export default LedgerTest;