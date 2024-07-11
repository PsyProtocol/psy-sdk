import React from 'react';
import styles from './AddressModal.module.scss';
import { AddressModalType, useAddressModal } from '../../hooks/useAddressModal';
import { Button, CloseButton, Group, Modal } from '@mantine/core';
import { ImportWalletModal } from './modals/ImportWallet';
import { FaucetFromWalletModal } from './modals/FaucetFromWallet';
import { TransferModal } from './modals/Transfer';
import { SignMessageModal } from './modals/SignMessage';

interface IAddressModalComponentProps<T>{
  activeModalData: T;
  onCancel: ()=>void;
  onComplete: (resultData: any)=>void;
}
type TAddressModalComponent = React.FC<IAddressModalComponentProps<any>>;
const AddressModalComponents: Record<AddressModalType, TAddressModalComponent> = {
  [AddressModalType.Closed]: () => null,
  [AddressModalType.Import]: ImportWalletModal,
  [AddressModalType.Faucet]: FaucetFromWalletModal,
  [AddressModalType.Transfer]: TransferModal,
  [AddressModalType.SignMessage]: SignMessageModal,
  
};
const AddressModalTitles: Record<AddressModalType, string> = {
  [AddressModalType.Closed]: '',
  [AddressModalType.Import]: 'Import Wallet',
  [AddressModalType.Faucet]: 'Faucet',
  [AddressModalType.Transfer]: 'Transfer',
  [AddressModalType.SignMessage]: 'Sign Message',
};

function getModalSize(type: AddressModalType) {
  if(type === AddressModalType.SignMessage){
    return "lg";
  }else{
    return "md";
  
  }

}
const AddressModal: React.FC = () => {
  const [activeModalType, activeModalData, completeModal, cancelModal] = useAddressModal((state)=>[state.activeModalType, state.activeModalData, state.completeModal, state.cancelModal]);
/*
  if(activeModalType === AddressModalType.Closed){
    return <div style={{display: "none"}}></div>;
  }*/
  const ActiveModal = AddressModalComponents[activeModalType];
  return (
    <Modal size={getModalSize(activeModalType)} opened={activeModalType !== AddressModalType.Closed} onClose={cancelModal} title={<div className={styles.addressModalTitle}>{AddressModalTitles[activeModalType]}</div>}>
        <ActiveModal activeModalData={activeModalData} onCancel={cancelModal} onComplete={completeModal} />
    </Modal>
  );
}
export type {
  TAddressModalComponent,
  IAddressModalComponentProps,
}
export {
  AddressModal,
}