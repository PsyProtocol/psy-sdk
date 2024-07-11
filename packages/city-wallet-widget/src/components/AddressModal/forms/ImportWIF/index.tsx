import { Button, Group, Input, TextInput } from "@mantine/core";
import { decodePrivateKeyAndNetworkFromWIF } from "doge-sdk";
import React, { useState } from "react";
import { IImportWalletFormProps, TImportWalletForm } from "../../modals/ImportWallet";
import styles from '../../modals/ImportWallet/ImportWallet.module.scss';
import { useWalletState } from "packages/wallet-widget/src/hooks/useWalletState";

function validatePrivateKeyWif(wif: string): boolean {
  try {
    const {networkId, privateKey} = decodePrivateKeyAndNetworkFromWIF(wif);
    return privateKey.length === 32;
  }catch(e){
    return false;
  }
}
const ImportWIFForm: TImportWalletForm = ({ onImport, className }) => {
  const [wif, setWif] = useState("");
  const [error, setError] = useState<string>();
  const [addWalletFromWIF] = useWalletState((state)=>[state.addWalletFromWIF]);
  return (
    <div className={styles.importForm+" "+styles.importWIFForm + (className?(" "+className):"")}>
      <h3>Import Wallet from WIF</h3>
      <div className={styles.formBody}>
    <TextInput
      label="WIF"
      description="Import a wallet using the standard Bitcoin Wallet Import Format (WIF)"
      placeholder="Input placeholder"
      error={error}
      onChange={(e)=>{
        setWif(e.currentTarget.value.replace(/\s/g, ''));
        if(error){
          setError(undefined);
        }
      }}
    />
    </div>
    <div className={styles.formControls}>
      <Button onClick={() => {
        if(!wif.length){
          setError("WIF is required");
          return;
        }else{
          if(validatePrivateKeyWif(wif)){
            addWalletFromWIF(wif, true).then(()=>{
              onImport(wif);
            }).catch(err=>console.error("error importing wif", err));
          }else{
            setError("Invalid WIF");
          }
        }
      }} disabled={wif.length<50}>Import</Button>
      </div>
    </div>
  );
}

export {
  ImportWIFForm,
}