import React, { useEffect, useState } from "react";
import { useWalletState } from "../../../../hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import { FaucetFromWalletForm, FormControls, InputCon, HelpText } from "./Transfer.styles";
import { Alert, Box, Button, Combobox, Input, InputBase, LoadingOverlay, TextInput, useCombobox } from "@mantine/core";
import type { IQedWidgetWallet } from "../../../../types";
import { IconInfoCircle } from "@tabler/icons-react";
import { WalletWidgetRPC } from "../../../../utils/rpc/walletRPC";

interface ITransferFormProps {
    onSubmit: (params: { contract_id: bigint; method_name: string; inputs: bigint[] }) => Promise<any>;
    onComplete: () => void;
    className?: string;
    wallet: IQedWidgetWallet;
}

const FeeRateSelector = ({
    rpc,
    onFeeRateChange,
}: {
    rpc: WalletWidgetRPC;
    onFeeRateChange: (newFee: number) => any;
}) => {
    const [feeEstimates, setFeeEstimates] = useState<{ confirmations: number; feeRate: number }[]>([]);
    const [numConfirmations, setNumConfirmations] = useState<string | null>(null);

    const combobox = useCombobox({
        onDropdownClose: () => combobox.resetSelectedOption(),
    });

    useEffect(() => {
        if (!feeEstimates.length) {
            rpc.getFeeEstimateMap().then((feeMap) => {
                const fe = Object.keys(feeMap)
                    .map((key) => ({ confirmations: parseInt(key), feeRate: (feeMap as any)[key] }))
                    .sort((a, b) => a.confirmations - b.confirmations);
                setFeeEstimates(fe);
            });
        }
    }, [feeEstimates]);
    if (!feeEstimates.length) {
        return <div>Loading...</div>;
    }

    const options = feeEstimates.map((item) => (
        <Combobox.Option value={item.confirmations + ""} key={item.confirmations + ""}>
            {item.confirmations + ""} Confirmations - {item.feeRate} sats/vbyte
        </Combobox.Option>
    ));

    return (
        <Combobox
            store={combobox}
            onOptionSubmit={(val) => {
                setNumConfirmations(val);
                if (val) {
                    onFeeRateChange(feeEstimates[parseInt(val)].feeRate);
                }
                combobox.closeDropdown();
            }}
        >
            <Combobox.Target>
                <InputBase
                    component="button"
                    type="button"
                    label="Fee Rate"
                    pointer
                    rightSection={<Combobox.Chevron />}
                    rightSectionPointerEvents="none"
                    onClick={() => combobox.toggleDropdown()}
                >
                    {numConfirmations ? (
                        `${numConfirmations} Confirmations - ${feeEstimates[parseInt(numConfirmations)].feeRate} sats/vbyte`
                    ) : (
                        <Input.Placeholder>Number of Confirmations...</Input.Placeholder>
                    )}
                </InputBase>
            </Combobox.Target>

            <Combobox.Dropdown>
                <Combobox.Options mah={200} style={{ overflowY: "auto" }}>
                    {options}
                </Combobox.Options>
            </Combobox.Dropdown>
        </Combobox>
    );
};

const TransferForm: React.FC<ITransferFormProps> = ({ onSubmit, onComplete, className, wallet }) => {
    const [contract_id, setContractID] = useState(0);
    const [method_name, setMethodName] = useState("");
    const [inputs, setInputs] = useState("");
    const [contractIDError, setContractIDError] = useState<string>();
    const [methodNameError, setMethodNameError] = useState<string>();
    const [inputsError, setInputsError] = useState<string>();
    // const [address, setAddress] = useState("");
    // const [amount, setAmount] = useState(0);
    // const [addressError, setAddressError] = useState<string>();
    // const [amountError, setAmountError] = useState<string>();
    const [loadingState, setLoadingState] = useState<"idle" | "loading" | "success" | "error">("idle");
    const [loadingError, setLoadingError] = useState<string>();

    return (
        <FaucetFromWalletForm className={className}>
            <Box pos="relative">
                <LoadingOverlay
                    visible={loadingState === "loading"}
                    zIndex={1000}
                    overlayProps={{ radius: "sm", blur: 2 }}
                />

                <div>
                    {loadingError ? (
                        <Alert variant="light" color="red" title="Transfer Error" icon={<IconInfoCircle />}>
                            {loadingError}
                        </Alert>
                    ) : null}

                    <InputCon>
                        <TextInput
                            label="Contract ID"
                            placeholder="Enter Contract ID..."
                            error={contractIDError}
                            spellCheck={false}
                            onChange={(v) => {
                                setContractID(v);
                                if (contractIDError) {
                                    setContractIDError(undefined);
                                }
                            }}
                            value={contract_id}
                        />
                    </InputCon>
                    <InputCon>
                        <TextInput
                            label="Method Name"
                            placeholder="Enter method name..."
                            error={methodNameError}
                            value={method_name}
                            onChange={(e) => {
                                setMethodName(e.currentTarget.value.replace(/\s/g, ""));
                                if (methodNameError) {
                                    setMethodNameError(undefined);
                                }
                            }}
                        />
                    </InputCon>
                    <InputCon>
                        <TextInput
                            label="Parameters (comma-separated)"
                            placeholder="Enter parameters separated by commas..."
                            error={inputsError}
                            value={inputs}
                            onChange={(e) => {
                                setInputs(e.currentTarget.value);
                                if (inputsError) {
                                    setInputsError(undefined);
                                }
                            }}
                        />
                        <HelpText>
                            Enter multiple values separated by commas (e.g., 100,200,300)
                        </HelpText>
                    </InputCon>
                </div>
                <FormControls>
                    <Button
                        onClick={() => {
                            setLoadingError(undefined);
                            const inputValues = inputs
                                .split(",")
                                .map((value) => value.trim())
                                .filter(Boolean);

                            if (inputValues.some((value) => isNaN(parseFloat(value)))) {
                                setInputsError("All parameters must be valid numbers");
                                return;
                            }

                            setLoadingState("loading");

                            onSubmit({
                                contract_id: BigInt(contract_id),
                                method_name,
                                inputs: inputValues.map((value) => BigInt(value)),
                            })
                                .then(() => {
                                    onComplete();
                                })
                                .catch((e) => {
                                    setLoadingState("error");
                                    setLoadingError(e.message);
                                });
                        }}
                    >
                        ContractCall
                    </Button>
                </FormControls>
            </Box>
        </FaucetFromWalletForm>
    );
};
const TransferModal: TAddressModalComponent = ({ onCancel, onComplete }) => {
    const [coordinator, realm, currentWallet, refreshAllWallets] = useWalletState((state) => [
        state.coordinatorEdgeRpcProvider,
        state.realmEdgeRpcProvider,
        state.currentWallet,
        state.refreshAllWallets,
    ]);
    if (!currentWallet) {
        return <div>You must select a wallet to perform a transfer.</div>;
    }
    const networkId = currentWallet.networkId;
    return (
        <TransferForm
            wallet={currentWallet}
            onSubmit={async (params) => {
                await currentWallet.wallet.execContractCall(currentWallet.publicKeyHex, {
                    contract_id: params.contract_id,
                    method_name: params.method_name,
                    inputs: params.inputs,
                });
                await refreshAllWallets();
            }}
            onComplete={() => {
                onComplete({});
            }}
        />
    );
};

export { TransferModal };

export type { ITransferFormProps };
