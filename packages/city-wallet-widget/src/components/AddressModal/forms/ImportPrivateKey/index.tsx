import { Button, Group, Input, TextInput } from "@mantine/core";
import { decodePrivateKeyAndNetworkFromWIF } from "doge-sdk";
import React, { useState } from "react";
import { IImportWalletFormProps, TImportWalletForm } from "../../modals/ImportWallet";
import styles from '../../modals/ImportWallet/ImportWallet.module.scss';
import { useWalletState } from "../../../../hooks/useWalletState";

function validatePrivateKeyHex(privateKey: string): boolean {
  if(!privateKey || privateKey.length !== 64){
    return false;
  }else{
    return /^[0-9a-fA-F]+$/.test(privateKey);
  }
}
const ImportPrivateKeyForm: TImportWalletForm = ({ onImport, className }) => {
  const [privateKey, setPrivateKey] = useState("");
  const [error, setError] = useState<string>();
  const [addWalletFromPrivateKey] = useWalletState((state)=>[state.addWalletFromPrivateKey]);
  return (
    <div className={styles.importForm+" "+styles.importWIFForm + (className?(" "+className):"")}>
      <h3>Import Wallet from Private Key</h3>
      <div className={styles.formBody}>
    <TextInput
      label="Private Key (Hex)"
      description="Import a wallet from a private key"
      placeholder="Private Key..."
      error={error}
      onChange={(e)=>{
        setPrivateKey(e.currentTarget.value.replace(/\s/g, ''));
        if(error){
          setError(undefined);
        }
      }}
    />
    </div>
    <div className={styles.formControls}>
      <Button onClick={() => {
        if(!privateKey.length){
          setError("Private Key is required");
          return;
        }else{
          if(validatePrivateKeyHex(privateKey)){
            addWalletFromPrivateKey(privateKey, true).then(()=>{
              onImport(privateKey);
            }).catch(err=>console.error("error importing private key", err));
          }else{
            setError("Invalid Private Key");
          }
        }
      }} disabled={privateKey.length!==64}>Import</Button>
      </div>
    </div>
  );
}

export {
  ImportPrivateKeyForm,
}