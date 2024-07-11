import React from "react";
import { SimpleGrid } from "@mantine/core";
import { WalletActionButton } from "../WalletActionButton";
import { IconBookUpload, IconCertificate, IconFileExport, IconSignLeft, IconTransfer } from "@tabler/icons-react";
import styles from './WalletActions.module.scss';
import { AddressModalType, useAddressModal } from "../../hooks/useAddressModal";
import { useWalletState } from "../../hooks/useWalletState";
import FaucetIcon from "../icons/FaucetIcon";


const WalletActions: React.FC = () => {
  const openModal = useAddressModal((state)=>state.openModal);
  const [rpc, abilities, signer] = useWalletState(state=>[state.rpc, state.abilities, state.currentWallet?.signer]);
  if(!signer){
    return null;
  }
  const canSignHash = signer.canSignHash();
  const canFaucet = rpc.canSendFromWallet();
  const canExportWallet = abilities.includes("export-private-key-wif");



  const cols = (~~canSignHash) + (~~canFaucet) + (~~canExportWallet) + 1;


  return (
    <div className={styles.walletActionsContainer}>
    <div className={styles.walletActionsInner}>
    <SimpleGrid
      type="container"
      cols={{ base: cols, '100px': cols, '400px': cols }}
      spacing={{ base: 4, '300px': 'xl' }}
    >
      {canSignHash?<div>
        <WalletActionButton disabledText={canSignHash?undefined:"This wallet does not support signing arbitrary messages"} label="Sign Message" icon={<IconCertificate />} onClick={() => {
          openModal(AddressModalType.SignMessage);
        }} />
      </div>:null}

      <div>
        <WalletActionButton label="Transfer" icon={<IconTransfer />} onClick={() => {
          openModal(AddressModalType.Transfer);
        }} />
      </div>
      {canExportWallet?<div>
        <WalletActionButton label="Export Wallet" icon={<IconBookUpload />} onClick={() => {
          console.log("Export Wallet");
        }} />
      </div>:null}
      {canFaucet?<div>
        <WalletActionButton label="Faucet" icon={<FaucetIcon size={24} />} onClick={() => {
          openModal(AddressModalType.Faucet);
        }} />
      </div>:null}
    </SimpleGrid>
    </div></div>
  );
}

export {
  WalletActions,
}