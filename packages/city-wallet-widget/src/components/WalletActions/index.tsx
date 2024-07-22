import React from "react";
import { SimpleGrid } from "@mantine/core";
import { WalletActionButton } from "../WalletActionButton";
import { IconBookUpload, IconCertificate, IconFileExport, IconSignLeft, IconTransfer } from "@tabler/icons-react";
import styles from './WalletActions.module.scss';
import { AddressModalType, useAddressModal } from "../../hooks/useAddressModal";
import { useWalletState } from "../../hooks/useWalletState";
import FaucetIcon from "../icons/FaucetIcon";
import DepositIcon from "../icons/DepositIcon";
import WithdrawIcon from "../icons/WithdrawIcon";


const WalletActions: React.FC = () => {
  const openModal = useAddressModal((state)=>state.openModal);
  const [rpc, walletAbilities, providerAbilities, currentWallet] = useWalletState(state=>[state.rpc, state.walletAbilities, state.providerAbilities, state.currentWallet]);
  if(!currentWallet){
    return null;
  }
  const canExportWallet = walletAbilities.includes("export-private-key-hex");



  const cols = (~~canExportWallet) + 3;


  return (
    <div className={styles.walletActionsContainer}>
    <div className={styles.walletActionsInner}>
    <SimpleGrid
      type="container"
      cols={{ base: cols, '100px': cols, '400px': cols }}
      spacing={{ base: 4, }}
    >

      <div>
        <WalletActionButton label="Claim Deposit" icon={<DepositIcon size={24} />} onClick={() => {
          openModal(AddressModalType.ClaimDeposit);
        }} />
      </div>
      <div>
        <WalletActionButton label="Transfer" icon={<IconTransfer />} onClick={() => {
          openModal(AddressModalType.Transfer);
        }} />
      </div>
      <div>
        <WalletActionButton label="Withdraw to L1" icon={<WithdrawIcon size={24} />} onClick={() => {
          openModal(AddressModalType.Withdraw);
        }} />
      </div>
      {canExportWallet?<div>
        <WalletActionButton label="Export Wallet" icon={<IconBookUpload />} onClick={() => {
          openModal(AddressModalType.ExportPrivateKey);
        }} />
      </div>:null}
    </SimpleGrid>
    </div></div>
  );
}

export {
  WalletActions,
}