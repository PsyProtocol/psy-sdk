import { ICitySigAction } from "../action/types";

type TCityTransactionSignerAbility = 'sign-hash' | 'sign-sigaction' | 'export-private-key-hex';
type TCityTransactionSignerProviderAbility = 'import-private-key' | 'add-random-private-key';

interface ICityTransactionSigner {
  getPublicKeyHex(): Promise<string>;
  getPrivateKeyHex?(): Promise<string>;
  signHash?(hash: string): Promise<string>;
  signSigAction?(sigAction: ICitySigAction): Promise<string>;
  getAbilities(): TCityTransactionSignerAbility[];
}

interface ICityTransactionSignerProvider {
  getSigners(): Promise<ICityTransactionSigner[]>;
  getPublicKeysHex(): Promise<string[]>;
  getSignerByPublicKeyHex(publicKeyHex: string): Promise<ICityTransactionSigner>;
  getAbilities(): TCityTransactionSignerProviderAbility[];
  importPrivateKey?(privateKeyHex: string): Promise<ICityTransactionSigner>;
  addRandomPrivateKey?(): Promise<ICityTransactionSigner>;
}

export type {
  ICityTransactionSigner,
  ICityTransactionSignerProvider,
  TCityTransactionSignerAbility,
  TCityTransactionSignerProviderAbility,
}