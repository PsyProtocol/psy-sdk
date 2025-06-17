import React from "react";
import styles from "./AddressModal.module.scss";
import { AddressModalType, useAddressModal } from "../../hooks/useAddressModal";
import { Button, CloseButton, Group, Modal } from "@mantine/core";
import { ImportWalletModal } from "./modals/ImportWallet";
// import { ClaimDepositModal } from "./modals/ClaimDeposit";
import { TransferModal } from "./modals/Transfer";
// import { WithdrawModal } from "./modals/Withdraw";
import { ExportPrivateKeyModal } from "./modals/ExportPrivateKey";

interface IAddressModalComponentProps<T> {
    activeModalData: T;
    onCancel: () => void;
    onComplete: (resultData: any) => void;
}
type TAddressModalComponent = React.FC<IAddressModalComponentProps<any>>;
const AddressModalComponents: Record<AddressModalType, TAddressModalComponent> = {
    [AddressModalType.Closed]: () => null,
    [AddressModalType.Import]: ImportWalletModal,
    // [AddressModalType.ClaimDeposit]: ClaimDepositModal,
    [AddressModalType.Transfer]: TransferModal,
    // [AddressModalType.Withdraw]: WithdrawModal,
    [AddressModalType.ExportPrivateKey]: ExportPrivateKeyModal,
};
const AddressModalTitles: Record<AddressModalType, string> = {
    [AddressModalType.Closed]: "",
    [AddressModalType.Import]: "Import Wallet",
    // [AddressModalType.ClaimDeposit]: "Claim Deposit",
    [AddressModalType.Transfer]: "Transfer",
    // [AddressModalType.Withdraw]: "Withdraw",
    [AddressModalType.ExportPrivateKey]: "Export Private Key",
};

function getModalSize(type: AddressModalType) {
    return "md";
    // if (type === AddressModalType.ClaimDeposit) {
    //     return "lg";
    // } else {
    //     return "md";
    // }
}
const AddressModal: React.FC = () => {
    const [activeModalType, activeModalData, completeModal, cancelModal] = useAddressModal((state) => [
        state.activeModalType,
        state.activeModalData,
        state.completeModal,
        state.cancelModal,
    ]);
    /*
  if(activeModalType === AddressModalType.Closed){
    return <div style={{display: "none"}}></div>;
  }*/
    const ActiveModal = AddressModalComponents[activeModalType];
    return (
        <Modal
            size={getModalSize(activeModalType)}
            opened={activeModalType !== AddressModalType.Closed}
            onClose={cancelModal}
            title={<div className={styles.addressModalTitle}>{AddressModalTitles[activeModalType]}</div>}
        >
            <ActiveModal activeModalData={activeModalData} onCancel={cancelModal} onComplete={completeModal} />
        </Modal>
    );
};
export type { TAddressModalComponent, IAddressModalComponentProps };
export { AddressModal };
