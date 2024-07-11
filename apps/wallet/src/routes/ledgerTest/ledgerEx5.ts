import {
  DogeLinkRPC,
  DogeMemoryWalletProvider,
  FullDogeWalletProvider,
  createP2PKHTransaction,
} from "doge-sdk";
import LedgerBitcoinApp from "@ledgerhq/hw-app-btc";
import { connectSpeculos } from "../../providers/ledger/speculos";
import { LedgerHardwareWalletProvider } from "./ledgerSigner";

async function exampleP2PKHLedger() {
  // initialize the ledger transport
  const transport = await connectSpeculos();

  // create a new instance of the LedgerBitcoinApp provided by @ledgerhq/hw-app-btc
  const ledgerBitcoinApp = new LedgerBitcoinApp({ transport: transport });
  console.log((window as any).ledgerBitcoinApp = ledgerBitcoinApp);
  // networkId can be doge, dogeTestnet, or dogeRegtest
  const networkId = "dogeRegtest";
  const RPC_API_URL =
    "http://devnet:devnet@localhost:1337/bitcoin-rpc/?network=" + networkId;

  // create an RPC instance to interact with the dogecoin network
  const rpc = new DogeLinkRPC(RPC_API_URL);

  // create a new instance of the LedgerHardwareWalletProvider, passing in the RPC instance and the LedgerBitcoinApp instance
  // we wrap the instance in a FullDogeWalletProvider to provide additional functionality
  const provider = new FullDogeWalletProvider(
    new LedgerHardwareWalletProvider(rpc, ledgerBitcoinApp, 8)
  );
  const addresses = await provider.getP2PKHAddresses(networkId);
  // our ledger's wallet address
  const ledgerAddress = addresses[0].address;
  // get the signer instance for the ledger address
  const ledgerSigner = await provider.getSignerForAddress(ledgerAddress);

  // generate a random recipient address for our transaction
  const recipientAddress = new DogeMemoryWalletProvider().addRandomWallet(
    networkId
  ).address;

  // in dogeRegtest, we can faucet tokens to any address we like after mining some blocks
  await rpc.mineBlocks(200);
  // faucet 10 DOGE to the wallet
  const faucetTxid = await rpc.sendFromWallet(ledgerAddress, 10);

  // send 9.5 DOGE from wallet1 to wallet2
  // get the funding transaction
  const faucetFundingTx = await rpc.getTransaction(faucetTxid);
  // get the unspent transaction output for wallet 1
  const faucetUTXO = faucetFundingTx.getUTXOsForAddress(ledgerAddress)[0];
  // create a transaction which sends 9.5 DOGE from our ledger wallet to recipientAddress
  const txBuilder = createP2PKHTransaction(ledgerSigner, {
    inputs: [faucetUTXO],
    outputs: [{ address: ledgerAddress, value: 900_500_000 }],
    address: recipientAddress,
  });

  // sign the transaction
  const finalizedTx = await txBuilder.finalizeAndSign();
  const txid = await rpc.sendRawTransaction(finalizedTx.toHex());
  console.log("Transaction id: ", txid);
}

export { exampleP2PKHLedger };
