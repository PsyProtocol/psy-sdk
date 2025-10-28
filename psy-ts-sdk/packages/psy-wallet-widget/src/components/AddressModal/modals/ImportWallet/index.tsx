import React from "react";
import { useWalletState } from "../../../../hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import { ImportPrivateKeyForm } from "../../forms/ImportPrivateKey";
type ImportAbility = "import-private-key";
import { ImportWalletModal as StyledImportWalletModal, ModalTitle, ImportForm } from "./ImportWallet.styles";
import { TQedTransactionSignerProviderAbility } from "@qed/psy-sdk";
interface IImportWalletFormProps {
    onImport: (data: any) => void;
    className?: string;
}
const TodoForm = ({ onImport, className }: IImportWalletFormProps) => {
    return (
        <ImportForm className={className}>
            <h3>Import Wallet</h3>
            <div>
                <div>TODO</div>
            </div>
        </ImportForm>
    );
};
type TImportWalletForm = React.FC<IImportWalletFormProps>;
const ImportForms: Record<string, TImportWalletForm> = {
    "import-private-key": ImportPrivateKeyForm,
};
const ImportWalletModal: TAddressModalComponent = ({ onCancel, onComplete }) => {
    const providerAbilities = useWalletState((state) => state.providerAbilities);
    const supportedAbilities = Object.keys(ImportForms).filter((ability) =>
        providerAbilities.includes(ability as TQedTransactionSignerProviderAbility)
    ) as ImportAbility[];

    if (supportedAbilities.length === 0) {
        return <div>Importing wallets not supported for this provider.</div>;
    } else if (supportedAbilities.length === 1) {
        const Form = ImportForms[supportedAbilities[0]];
        return <Form onImport={(data) => onComplete({ data })} />;
    } else {
        return (
            <StyledImportWalletModal>
                <ModalTitle>Import Wallet</ModalTitle>
                <div>
                    {supportedAbilities.map((ability) => {
                        const Form = ImportForms[ability];
                        return <Form key={ability} onImport={(privateKey) => onComplete({ privateKey })} />;
                    })}
                </div>
            </StyledImportWalletModal>
        );
    }
};

export { ImportWalletModal };

export type { IImportWalletFormProps, TImportWalletForm, ImportAbility };
