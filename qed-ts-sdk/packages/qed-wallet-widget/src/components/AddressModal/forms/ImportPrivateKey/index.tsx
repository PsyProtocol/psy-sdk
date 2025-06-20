import { Button, TextInput } from "@mantine/core";
import React, { useState } from "react";
import { TImportWalletForm } from "../../modals/ImportWallet";
import { ImportForm, FormControls } from "../../modals/ImportWallet/ImportWallet.styles";
import { useWalletState } from "../../../../hooks/useWalletState";

function validatePrivateKeyHex(privateKey: string): boolean {
    if (!privateKey || privateKey.length !== 64) {
        return false;
    } else {
        return /^[0-9a-fA-F]+$/.test(privateKey);
    }
}
const ImportPrivateKeyForm: TImportWalletForm = ({ onImport, className }) => {
    const [privateKey, setPrivateKey] = useState("");
    const [error, setError] = useState<string>();
    const [addWalletFromPrivateKey] = useWalletState((state) => [state.addWalletFromPrivateKey]);
    return (
        <ImportForm className={className}>
            <h3>Import Wallet from Private Key</h3>
            <div>
                <TextInput
                    label="Private Key (Hex)"
                    description="Import a wallet from a private key"
                    placeholder="Private Key..."
                    error={error}
                    onChange={(e) => {
                        setPrivateKey(e.currentTarget.value.replace(/\s/g, ""));
                        if (error) {
                            setError(undefined);
                        }
                    }}
                />
            </div>
            <FormControls>
                <Button
                    onClick={() => {
                        if (!privateKey.length) {
                            setError("Private Key is required");
                            return;
                        } else {
                            if (validatePrivateKeyHex(privateKey)) {
                                addWalletFromPrivateKey(privateKey, true)
                                    .then((result) => {
                                        console.log("Import wallet result:", result);
                                        onImport(privateKey);
                                    })
                                    .catch((err) => {
                                        console.error("Error importing private key:", err);
                                        setError("Failed to import wallet: " + err.message);
                                    });
                            } else {
                                setError("Invalid Private Key");
                            }
                        }
                    }}
                    disabled={privateKey.length !== 64}
                >
                    Import
                </Button>
            </FormControls>
        </ImportForm>
    );
};

export { ImportPrivateKeyForm };
