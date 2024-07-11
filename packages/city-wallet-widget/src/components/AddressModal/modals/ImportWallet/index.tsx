
import React from 'react';
import { useWalletState } from "packages/wallet-widget/src/hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import { TWalletAbility } from 'doge-sdk';
import { ImportWIFForm } from '../../forms/ImportWIF';
type ImportAbility = "add-wallet-bip39" | "add-wallet-bip44" | "add-wallet-bip178";
import styles from './ImportWallet.module.scss';
interface IImportWalletFormProps {
  onImport: (data: any) => void;
  className?: string;
}
const TodoForm = ({ onImport, className }: IImportWalletFormProps) => {
  return (
    <div className={styles.importForm + (className ? " " + className : "")}>
      <h3>Import Wallet</h3>
      <div className={styles.formBody}>
        <div>TODO</div>
      </div>
    </div>
  );
}
type TImportWalletForm = React.FC<IImportWalletFormProps>;
const ImportForms: Record<string, TImportWalletForm> = {
  'add-wallet-bip39': TodoForm,
  'add-wallet-bip44': TodoForm,
  'add-wallet-bip178': ImportWIFForm,
};
const ImportWalletModal: TAddressModalComponent = ({ onCancel, onComplete }) => {
  const abilities = useWalletState((state)=>state.abilities);
  const supportedAbilities = Object.keys(ImportForms).filter((ability)=>abilities.includes(ability as TWalletAbility)) as ImportAbility[];

  if(supportedAbilities.length === 0){
    return <div>Importing wallets not supported for this provider.</div>;
  }else if(supportedAbilities.length === 1){
    const Form = ImportForms[supportedAbilities[0]];
    return <Form onImport={(data)=>onComplete({data})} />;
  }else{
    return(
      <div className={styles.importWalletModal}>
        <div className={styles.modalTitle}>Import Wallet</div>
        <div>
          {supportedAbilities.map((ability)=>{
            const Form = ImportForms[ability];
            return <Form key={ability} onImport={(wif)=>onComplete({wif})} />;
          })}
        </div>
      </div>
    )
  }
};

export {
  ImportWalletModal,
};

export type {
  IImportWalletFormProps,
  TImportWalletForm,
  ImportAbility,
}