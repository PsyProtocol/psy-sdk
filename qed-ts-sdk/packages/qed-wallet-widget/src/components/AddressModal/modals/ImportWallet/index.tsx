import React from "react";
import { useWalletState } from "../../../../hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import { ImportPrivateKeyForm } from "../../forms/ImportPrivateKey";
type ImportAbility = "import-private-key";
import styles from "./ImportWallet.module.scss";
import { TQedTransactionSignerProviderAbility } from "@qed/qed-sdk";
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
            <div className={styles.importWalletModal}>
                <div className={styles.modalTitle}>Import Wallet</div>
                <div>
                    {supportedAbilities.map((ability) => {
                        const Form = ImportForms[ability];
                        return <Form key={ability} onImport={(privateKey) => onComplete({ privateKey })} />;
                    })}
                </div>
            </div>
        );
    }
};

export { ImportWalletModal };

export type { IImportWalletFormProps, TImportWalletForm, ImportAbility };
