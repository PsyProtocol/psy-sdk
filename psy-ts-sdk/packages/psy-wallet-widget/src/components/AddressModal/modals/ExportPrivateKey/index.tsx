import React, { useEffect, useState } from "react";
import { useWalletState } from "../../../../hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import { FaucetFromWalletForm, FormControls, InputCon } from "./ExportPrivateKey.styles";
import { Box, Button, TextInput } from "@mantine/core";

import { WWCopyButton } from "../../../WWCopyButton";

interface IExportPrivateKeyFormProps {
    onComplete: () => void;
    className?: string;
    privateKey: string;
}

const ExportPrivateKeyForm: React.FC<IExportPrivateKeyFormProps> = ({ onComplete, className, privateKey }) => {
    return (
        <FaucetFromWalletForm className={className}>
            <Box pos="relative">
                <div>
                    <InputCon>
                        <TextInput
                            label="Private Key"
                            spellCheck={false}
                            value={privateKey}
                            disabled={true}
                            style={{ flexGrow: 1 }}
                        />
                        <WWCopyButton value={privateKey} />
                    </InputCon>
                </div>
                <FormControls>
                    <Button
                        onClick={() => {
                            onComplete();
                        }}
                    >
                        Close
                    </Button>
                </FormControls>
            </Box>
        </FaucetFromWalletForm>
    );
};
const ExportPrivateKeyModal: TAddressModalComponent = ({ onComplete }) => {
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
