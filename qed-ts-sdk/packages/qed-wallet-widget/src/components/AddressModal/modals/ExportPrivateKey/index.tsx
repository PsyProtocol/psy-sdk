import React, { useEffect, useState } from "react";
import { useWalletState } from "../../../../hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import styles from "./ExportPrivateKey.module.scss";
import { Box, Button, TextInput } from "@mantine/core";

import { WWCopyButton } from "../../../WWCopyButton";

interface IExportPrivateKeyFormProps {
    onComplete: () => void;
    className?: string;
    privateKey: string;
}

const ExportPrivateKeyForm: React.FC<IExportPrivateKeyFormProps> = ({ onComplete, className, privateKey }) => {
    return (
        <div className={styles.faucetFromWalletForm + (className ? " " + className : "")}>
            <Box pos="relative">
                <div className={styles.formBody}>
                    <div className={styles.inputCon}>
                        <TextInput
                            label="Private Key"
                            spellCheck={false}
                            value={privateKey}
                            disabled={true}
                            style={{ flexGrow: 1 }}
                        />
                        <WWCopyButton value={privateKey} />
                    </div>
                </div>
                <div className={styles.formControls}>
                    <Button
                        onClick={() => {
                            onComplete();
                        }}
                    >
                        Close
                    </Button>
                </div>
            </Box>
        </div>
    );
};
const ExportPrivateKeyModal: TAddressModalComponent = ({ onCancel, onComplete }) => {
    const currentWallet = useWalletState((state) => state.currentWallet);
    const [privateKey, setPrivateKey] = useState<string>("");

    useEffect(() => {
        if (
            currentWallet?.wallet.signer.getPrivateKeyHex &&
            currentWallet.wallet.signer.getAbilities().includes("export-private-key-hex")
        ) {
            (async () => {
                setPrivateKey(await currentWallet.wallet.signer.getPrivateKeyHex!());
            })();
        } else {
            setPrivateKey("");
        }
    }, [currentWallet]);
    if (!currentWallet?.wallet.signer.getPrivateKeyHex) {
        return <div>Does not support exporting private key.</div>;
    }
    return (
        <ExportPrivateKeyForm
            privateKey={privateKey}
            onComplete={() => {
                onComplete({});
            }}
        />
    );
};

export { ExportPrivateKeyModal };

export type { IExportPrivateKeyFormProps };
