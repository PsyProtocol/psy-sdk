import React from 'react';
import styles from './AddressModal.module.scss';
import { AddressModalType, useAddressModal } from '../../hooks/useAddressModal';
import { Button, CloseButton, Group } from '@mantine/core';
import { ImportWalletModal } from './modals/ImportWallet';

const AddressModalTitle: Record<AddressModalType, string> = {
  [AddressModalType.Closed]: '',
  [AddressModalType.Import]: 'Import Wallet',
};
interface IAddressModalComponentProps<T>{
  activeModalData: T;
  onCancel: ()=>void;
  onComplete: (resultData: any)=>void;
}
type TAddressModalComponent = React.FC<IAddressModalComponentProps<any>>;
const AddressModalComponents: Record<AddressModalType, TAddressModalComponent> = {
  [AddressModalType.Closed]: () => null,
  [AddressModalType.Import]: ImportWalletModal,
};
const AddressModal: React.FC = () => {
  const [activeModalType, activeModalData, completeModal, cancelModal] = useAddressModal((state)=>[state.activeModalType, state.activeModalData, state.completeModal, state.cancelModal]);

  if(activeModalType === AddressModalType.Closed){
    return <div style={{display: "none"}}></div>;
  }
  const ActiveModal = AddressModalComponents[activeModalType];
  return (
    <div className={styles.addressModal}>
    <div className={styles.addressModalInner}>
      <div className={styles.modalTop}>
          <div>{AddressModalTitle[activeModalType]}</div>
          <CloseButton onClick={()=>cancelModal()} />
      </div>
      <div className={styles.modalContent}>
        <ActiveModal activeModalData={activeModalData} onCancel={cancelModal} onComplete={completeModal} />
      </div>
    </div>
    </div>
  );
}
export type {
  TAddressModalComponent,
  IAddressModalComponentProps,
}
export {
  AddressModal,
}