import { DogeLinkRPC, DogeMemoryWalletProvider, FullDogeWalletProvider, IDogeTransactionSigner, IDogeWalletProvider, ISignatureResult, SIGHASH_ALL, Transaction, compressPublicKey, createP2PKHTransaction, createP2SHTransaction, derEncodeSignature, getP2SHAddress, hashBuffer, hashHex, hexToU8Array, u8ArrayToHex } from "doge-sdk";
import LedgerBitcoinApp from '@ledgerhq/hw-app-btc';
import { connectSpeculos } from "../../providers/ledger/speculos";
import { LedgerHardwareWalletProvider } from "./ledgerSigner";

async function signSendLedgerTx(){
  // networkId can be doge, dogeTestnet, or dogeRegtest
  const networkId = "dogeRegtest";

  // your dogecoin rpc node url, with an added query equal to doge, dogeTestnet, or dogeRegtest
  const RPC_API_URL = "http://devnet:devnet@localhost:1337/bitcoin-rpc/?network="+networkId;

  const rpc = new DogeLinkRPC(RPC_API_URL);

  const transport = await connectSpeculos();
  const btc = new LedgerBitcoinApp({transport: transport});
  const ledgerInstance = btc;
  const ledgerHWWallet = new LedgerHardwareWalletProvider(rpc, btc, 1);
  const provider = new FullDogeWalletProvider(ledgerHWWallet);
  const addresses = await provider.getP2PKHAddresses("dogeRegtest");
  const signers = await provider.getSigners();
  const ledgerSigner = signers[0];
  const ledgerAddress = addresses[0].address;
  console.log("ledgerAddress",addresses[0]);
  const walletPath = ledgerHWWallet.signers[0].walletPath;
  console.log("walletPath",walletPath);
  const datPub = await btc.getWalletPublicKey(walletPath, { format: "legacy" });
  console.log("datPub",datPub)

  // wallet provider, in this case an in-memory wallet provider
  const walletProvider = new DogeMemoryWalletProvider();

  // create a random dogecoin P2PKH wallet
  const wallet1 = walletProvider.addRandomWallet(networkId);
  console.log("wallet 1 address: ", wallet1.address);
  const wallet2 = walletProvider.addRandomWallet(networkId);
  console.log("wallet 2 address: ", wallet2.address);

  // in dogeRegtest, we can faucet tokens to any address we like after mining some blocks
  await rpc.mineBlocks(200);
  // faucet 10 DOGE to the wallet
  const faucetTxid = await rpc.sendFromWallet(wallet1.address, 10);

  // send 9.5 DOGE from wallet1 to wallet2
  // get the funding transaction
  const faucetFundingTx = await rpc.getTransaction(faucetTxid);
  // get the unspent transaction output for wallet 1
  const faucetUTXO = faucetFundingTx.getUTXOsForAddress(wallet1.address)[0];
  // create a transaction which sends 9.5 DOGE from wallet1 to wallet2
  const txBuilder0 = createP2PKHTransaction(wallet1, {
    inputs: [faucetUTXO],
    outputs: [{address: ledgerAddress, value: 900_500_000}],
    address: wallet1.address,
  });

  // sign the transaction
  const finalizedTx0 = await txBuilder0.finalizeAndSign();
  const txid0 = await rpc.sendRawTransaction(finalizedTx0.toHex());

  // in dogeRegtest, we can faucet tokens to any address we like after mining some blocks
  await rpc.mineBlocks(200);

  const srcTx = await rpc.getTransaction(txid0);
  const srcUTXO = srcTx.getUTXOsForAddress(ledgerAddress)[0];
  const txBuilder1 = createP2PKHTransaction(ledgerSigner, {
    inputs: [srcUTXO],
    outputs: [{address: wallet2.address, value: 800_500_000}],
    address: ledgerAddress,
  });
  const signed = await txBuilder1.finalizeAndSign();
  const finalTxId = await rpc.sendRawTransaction(signed.toHex());
  console.log("Final tx id: ", finalTxId);

  /*
  const preimage = txBuilder1.getSigHashPreimage(0, SIGHASH_ALL);
  
  const keyset = walletPath.split("/").slice(-3).join("/");
  const txPreimage = ledgerInstance.splitTransaction(u8ArrayToHex(preimage));
  const txInput = ledgerInstance.splitTransaction(srcTx.toHex());
  const outputScriptHex = ledgerInstance.serializeTransactionOutputs(txPreimage).toString('hex');
  const ledgerResponse = await ledgerInstance.createPaymentTransaction({
    inputs: [[txInput, 0, undefined, undefined]],
    outputScriptHex,
    associatedKeysets: [walletPath],
    additionals: [],
    segwit: false,
  });
  console.log(ledgerResponse);
  const finalTxId = await rpc.sendRawTransaction(ledgerResponse);
  console.log("Final tx id: ", finalTxId);

  */


  







  
}



async function exampleComplexP2SHv2(){
  /*
   * Wallet 1 -> sign(P2SH Puzzle, Wallet 2) -> Wallet 3
   */
  // networkId can be doge, dogeTestnet, or dogeRegtest
  const networkId = "dogeRegtest";

  // note: if you don't have an RPC node, you can start one up with docker:
  // docker run -p 1337:1337 -it --rm qedprotocol/bitide-doge:latest

  // your dogecoin rpc node url, with an added query equal to doge, dogeTestnet, or dogeRegtest
  const RPC_API_URL = "http://devnet:devnet@localhost:1337/bitcoin-rpc/?network="+networkId;

  const rpc = new DogeLinkRPC(RPC_API_URL);

  const transport = await connectSpeculos();
  const btc = new LedgerBitcoinApp({transport: transport});
  const ledgerInstance = btc;
  const ledgerHWWallet = new LedgerHardwareWalletProvider(rpc, btc, 1);
  const provider = new FullDogeWalletProvider(ledgerHWWallet);
  // create some wallets, wallet 1 will sign the tx and wallet 2 will receive the funds
  const walletProvider = new DogeMemoryWalletProvider();
  const wallet1 = walletProvider.addRandomWallet(networkId);
  console.log("wallet 1 address: ", wallet1.address);
  const wallet2 = walletProvider.addRandomWallet(networkId);
  console.log("wallet 2 address: ", wallet1.address);
  const wallet3 = walletProvider.addRandomWallet(networkId);
  console.log("wallet 3 address: ", wallet1.address);
  const addresses = await provider.getP2PKHAddresses("dogeRegtest");
  const signers = await provider.getSigners();
  const ledgerSigner = signers[0];
  const ledgerAddress = addresses[0].address;
  const ledgerPublicKey = addresses[0].publicKey;

  // create a secret string and hash it
  const secretString = "hello world";
  const secretStringHashHex = hashBuffer("hash160", new TextEncoder().encode(secretString), "hex");

  
  // hash the public key of wallet2
  const pubKeyHashHex = hashHex("hash160", ledgerPublicKey, "hex")


  // a more complex puzzle utxo that can be unlocked by providing the secret string "hello world" and a signature from wallet2
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
  // our unlock script will contain the secret string and a signature from wallet2
  // (the signature will be added automatically when we run finalizeAndSign)
  const UNLOCK_SCRIPT = `
    <"${secretString}">
  `;

  // compute the pay-to-script-hash address for our puzzle
  const p2shAddress = getP2SHAddress(REDEEM_SCRIPT, networkId);
  console.log("pay-to-script-hash address: ", p2shAddress);


  // in dogeRegtest, we can faucet tokens to any address we like after mining some blocks
  await rpc.mineBlocks(200);
  // faucet 10 DOGE to wallet1
  
  const faucetTxid = await rpc.sendFromWallet(wallet1.address, 10);
  await rpc.mineBlocks(1);

  // -- STEP 1: send 9.9 DOGE from wallet1 to the puzzle script --
  // get the funding transaction
  const faucetFundingTx = await rpc.getTransaction(faucetTxid);
  // get the unspent transaction output for wallet 1
  const faucetUTXO = faucetFundingTx.getUTXOsForAddress(wallet1.address)[0];
  // create a transaction which sends 9.9 DOGE from wallet1 to wallet2
  const txBuilder1 = createP2PKHTransaction(wallet1, {
    inputs: [faucetUTXO],
    outputs: [{address: p2shAddress, value: 990_000_000}],
    address: wallet1.address,
  });
  // finalize the transaction
  const finalizedTx1 = await txBuilder1.finalizeAndSign();

  // broadcast the transaction
  const txid1 = await rpc.sendRawTransaction(finalizedTx1.toHex());
  console.log("(send from wallet 1 to p2sh script) transaction id: ", txid1);

  await rpc.mineBlocks(1);







  // -- unlock the puzzle and spend 9.8 DOGE from the puzzle to wallet3 --
  // get the p2sh funding transaction
  const p2shFundingTx = await rpc.getTransaction(txid1);
  // get the unspent transaction output for the puzzle p2sh script
  const p2shUTXO = p2shFundingTx.getUTXOsForAddress(p2shAddress)[0];
  console.log("p2shUTXO",faucetUTXO);

  // create a transaction which sends 9.8 DOGE from the puzzle script to wallet1
  const p2shTxBuilder = createP2SHTransaction({
    redeemScriptBASM: REDEEM_SCRIPT,
    unlockScriptBASM: UNLOCK_SCRIPT,
    inputs: [p2shUTXO],
    outputs: [{address: wallet3.address, value: 980_000_000}],
    // wallet2 will sign the transaction
    signers: [ledgerSigner],
  });

  // finalize the transaction
  const finalizedTx = await p2shTxBuilder.finalizeAndSign();

  // broadcast the transaction
  const txid = await rpc.sendRawTransaction(finalizedTx.toHex());
  console.log("(send from the p2sh script to wallet 3) transaction id: ", txid);
}
export {
  signSendLedgerTx,
  exampleComplexP2SHv2,
}