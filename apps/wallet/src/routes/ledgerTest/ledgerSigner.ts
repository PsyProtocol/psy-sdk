import {
  IDogeTransactionSigner,
  IDogeWalletProvider,
  ISignatureResult,
  Transaction,
  compressPublicKey,
  hexToU8Array,
  u8ArrayToHex,
  u8ArrayToHexReversed,
  disassembleScript,
  IDogeSignatureRequest,
  DogeNetworkId,
  TWalletAbility,
  IDogeLinkRPC,
} from "doge-sdk";
import LedgerBitcoinApp from "@ledgerhq/hw-app-btc";

class LedgerHardwareWalletSigner implements IDogeTransactionSigner {
  walletPath: string;
  ledgerInstance: LedgerBitcoinApp;
  cachedPublicKey: string = "";
  rpc: IDogeLinkRPC;
  constructor(
    walletPath: string,
    rpc: IDogeLinkRPC,
    ledgerInstance: LedgerBitcoinApp
  ) {
    this.walletPath = walletPath;
    this.ledgerInstance = ledgerInstance;
    this.rpc = rpc;
  }
  async getCompressedPublicKey(): Promise<string> {
    if (this.cachedPublicKey) {
      return this.cachedPublicKey;
    }
    const ledgerResponse = await this.ledgerInstance.getWalletPublicKey(
      this.walletPath,
      { format: "legacy" }
    );
    const compressedPublicKey = compressPublicKey(
      hexToU8Array(ledgerResponse.publicKey)
    );
    const compressedPublicKeyHex = u8ArrayToHex(compressedPublicKey);
    this.cachedPublicKey = compressedPublicKeyHex;
    return compressedPublicKeyHex;
  }
  canSignHash(): boolean {
    return false;
  }
  signHash(_hashHex: string): Promise<ISignatureResult> {
    // we don't have to implement this method since we can sign the transaction directly
    throw new Error("Method not implemented.");
  }
  async signP2PKHTransaction(
    signatureRequest: IDogeSignatureRequest
  ): Promise<ISignatureResult> {
    const tx = signatureRequest.transaction;
    const inputs: [any, number, undefined, undefined][] = await Promise.all(
      tx.inputs.map(async (input) => {
        const rawHex = await this.rpc.getRawTransaction(
          u8ArrayToHexReversed(input.hash)
        );
        return [
          this.ledgerInstance.splitTransaction(rawHex, false),
          input.index,
          undefined,
          undefined,
        ];
      })
    );

    const splitPreimage = this.ledgerInstance.splitTransaction(
      tx.toHex(),
      false
    );
    const ledgerResponse = await this.ledgerInstance.createPaymentTransaction({
      inputs: inputs,
      associatedKeysets: [this.walletPath],
      outputScriptHex: this.ledgerInstance
        .serializeTransactionOutputs(splitPreimage)
        .toString("hex"),
      additionals: [],
      segwit: false,
      sigHashType: signatureRequest.sigHashType,
      lockTime: tx.locktime,
    });
    const decodedLedgerTx = Transaction.fromHex(ledgerResponse);
    const disAsm = disassembleScript(
      decodedLedgerTx.inputs[signatureRequest.inputIndex].script
    );
    const [signatureBase, publicKey] = disAsm.split(" ").slice(0, 2);
    // remove sighash type from signature
    const signature = signatureBase.substring(0, signatureBase.length - 2);

    return {
      publicKey,
      signature,
    };
  }
  async signP2SHTransaction(
    signatureRequest: IDogeSignatureRequest
  ): Promise<ISignatureResult> {
    const tx = signatureRequest.transaction;
    const publicKey = await this.getCompressedPublicKey();

    const inputs: [
      any,
      number,
      string | null | undefined,
      number | null | undefined
    ][] = await Promise.all(
      tx.inputs.map(async (input) => {
        const rawHex = await this.rpc.getRawTransaction(
          u8ArrayToHexReversed(input.hash)
        );
        const script =
          input.script.length === 0 ? undefined : u8ArrayToHex(input.script);
        return [
          this.ledgerInstance.splitTransaction(rawHex, false),
          input.index,
          script,
          input.sequence,
        ];
      })
    );

    const splitPreimage = this.ledgerInstance.splitTransaction(
      tx.toHex(),
      false
    );

    const signatures = await this.ledgerInstance.signP2SHTransaction({
      inputs: inputs,
      associatedKeysets: [this.walletPath],
      outputScriptHex: this.ledgerInstance
        .serializeTransactionOutputs(splitPreimage)
        .toString("hex"),
      segwit: false,
      sigHashType: signatureRequest.sigHashType,
      lockTime: tx.locktime,
      transactionVersion: tx.version,
    });

    const signature = signatures[0];

    return {
      publicKey,
      signature,
    };
  }
  async signTransaction(
    signatureRequest: IDogeSignatureRequest
  ): Promise<ISignatureResult> {
    if (signatureRequest.transaction.getSigHashConfig().isP2PKH) {
      return this.signP2PKHTransaction(signatureRequest);
    } else {
      return this.signP2SHTransaction(signatureRequest);
    }
  }
}
class LedgerHardwareWalletProvider implements IDogeWalletProvider {
  numberOfWallets: number;
  ledgerInstance: LedgerBitcoinApp;
  signers: LedgerHardwareWalletSigner[] = [];
  rpc: IDogeLinkRPC;

  constructor(
    rpc: IDogeLinkRPC,
    ledgerInstance: LedgerBitcoinApp,
    numberOfWallets = 8
  ) {
    this.ledgerInstance = ledgerInstance;
    this.numberOfWallets = numberOfWallets;
    for (let i = 0; i < numberOfWallets; i++) {
      this.signers.push(
        new LedgerHardwareWalletSigner(
          "44'/0'/" + i + "'/0/0",
          rpc,
          ledgerInstance
        )
      );
    }
    this.rpc = rpc;
  }
  addWalletBIP44(networkId: DogeNetworkId, fullDerivationPath: string): Promise<IDogeTransactionSigner> {
    const existing = this.signers.find(x=>x.walletPath === fullDerivationPath);
    if(existing){
      return Promise.resolve(existing);
    }else{
      const newSigner = new LedgerHardwareWalletSigner(fullDerivationPath, this.rpc, this.ledgerInstance);
      this.signers.push(newSigner);
      return Promise.resolve(newSigner);
    }

  }
  getAbilities(): TWalletAbility[] {
    return ["sign-transaction"];
  }

  getSigners(): Promise<IDogeTransactionSigner[]> {
    return Promise.resolve(this.signers);
  }
}
export { LedgerHardwareWalletProvider, LedgerHardwareWalletSigner };
