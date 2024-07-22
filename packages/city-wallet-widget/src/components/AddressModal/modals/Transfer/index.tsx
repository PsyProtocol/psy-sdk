import React, { useEffect, useState } from "react";
import { useWalletState } from "../../../../hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import styles from "./Transfer.module.scss";
import { Alert, Box, Button, Combobox, Input, InputBase, LoadingOverlay, TextInput, useCombobox } from "@mantine/core";
import { DogeInput } from "../../../DogeInput";
import type {IQCityWidgetWallet} from '../../../../types';
import { IconInfoCircle } from "@tabler/icons-react";
import { BlokiesIcon } from "@qstudio/blokies-react";
import { WalletWidgetRPC } from "../../../../utils/rpc/walletRPC";
import { sha256Buffer } from "@qstudio/utils";

interface ITransferFormProps {
  onSubmit: (params: {recipient: number, amount: string}) => Promise<any>;
  onComplete: () => void;
  className?: string;
  wallet: IQCityWidgetWallet;
}

const FeeRateSelector = ({rpc, onFeeRateChange}: {rpc: WalletWidgetRPC, onFeeRateChange: (newFee: number)=>any}) => {
  const [feeEstimates, setFeeEstimates]= useState<{confirmations: number, feeRate: number}[]>([]);
  const [numConfirmations, setNumConfirmations] = useState<string | null>(null);

  const combobox = useCombobox({
    onDropdownClose: () => combobox.resetSelectedOption(),
  });

  useEffect(()=>{
    if(!feeEstimates.length){
      rpc.getFeeEstimateMap().then((feeMap)=>{
        const fe = Object.keys(feeMap).map((key)=>({confirmations: parseInt(key), feeRate: (feeMap as any)[key]})).sort((a,b)=>a.confirmations-b.confirmations);
        setFeeEstimates(fe);
      });
    }
  },[feeEstimates]);
  if(!feeEstimates.length){
    return <div>Loading...</div>;
  }

  const options = feeEstimates.map((item) => (
    <Combobox.Option value={item.confirmations+""} key={item.confirmations+""}>
      {item.confirmations+""} Confirmations - {item.feeRate} sats/vbyte
    </Combobox.Option>
  ));

  return (
    <Combobox
      store={combobox}
      onOptionSubmit={(val) => {
        setNumConfirmations(val);
        if(val){
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
          {numConfirmations?(`${numConfirmations} Confirmations - ${feeEstimates[parseInt(numConfirmations)].feeRate} sats/vbyte`): <Input.Placeholder>Number of Confirmations...</Input.Placeholder>}
        </InputBase>
      </Combobox.Target>

      <Combobox.Dropdown>
        <Combobox.Options mah={200} style={{ overflowY: 'auto' }}>{options}</Combobox.Options>
      </Combobox.Dropdown>
    </Combobox>
  );
}

const TransferForm: React.FC<ITransferFormProps> = ({
  onSubmit,
  onComplete,
  className,
  wallet,
}) => {
  const [address, setAddress] = useState("");
  const [amount, setAmount] = useState(0);
  const [addressError, setAddressError] = useState<string>();
  const [amountError, setAmountError] = useState<string>();
  const [loadingState, setLoadingState] = useState<
    "idle" | "loading" | "success" | "error"
  >("idle");
  const [loadingError, setLoadingError] = useState<string>();

  return (
    <div
      className={
        styles.faucetFromWalletForm + (className ? " " + className : "")
      }
    >
      <Box pos="relative">
        <LoadingOverlay
          visible={loadingState === "loading"}
          zIndex={1000}
          overlayProps={{ radius: "sm", blur: 2 }}
        />

        <div className={styles.formBody}>
          {loadingError ? (
            <Alert
              variant="light"
              color="red"
              title="Transfer Error"
              icon={<IconInfoCircle />}
            >
              {loadingError}
            </Alert>
          ) : null}
          {(address.length)?<div className={styles.blokiesCon}>
            <BlokiesIcon seed={sha256Buffer(new TextEncoder().encode("city-rollup:"+address), "hex")} size={8} scale={8} className={styles.blokiesIcon} />
          </div>:null}

          <div className={styles.inputCon}>
            <TextInput
              label="Recipient User ID"
              placeholder="User ID to send tokens to..."
              error={addressError}
              spellCheck={false}
              value={address}
              onChange={(e) => {
                setAddress(e.currentTarget.value.replace(/\s/g, ""));
                if (addressError) {
                  setAddressError(undefined);
                }
              }}
            />
          </div>
          <div className={styles.inputCon}>
            <DogeInput
              label="Transfer Amount"
              placeholder="Transfer Amount..."
              error={addressError}
              useSats={true}
              onChange={(v) => {
                setAmount(v);
                if (amountError) {
                  setAmountError(undefined);
                }
              }}
              value={amount}
            />
          </div>
        </div>
        <div className={styles.formControls}>
          <Button
            onClick={() => {
              setLoadingError(undefined);
              if (!address.length) {
                setAddressError("User ID is required");
                return;
              } else if ((parseInt(address) + "")!==address) {
                setAddressError("Invalid User ID");
              } else if (!amount) {
                setAmountError("Amount must be greater than 0");
              } else {
                setLoadingState("loading");



                onSubmit({recipient: parseInt(address), amount: amount+""})
                  .then(() => {
                    onComplete();
                  })
                  .catch((e) => {
                    setLoadingState("error");
                    setLoadingError(e.message);
                  });
                }
            }}
            disabled={!amount || address.length === 0}
          >
            Transfer
          </Button>
        </div>
      </Box>
    </div>
  );
};
const TransferModal: TAddressModalComponent = ({
  onCancel,
  onComplete,
}) => {
  const [rpc, currentWallet, refreshAllWallets] = useWalletState(
    (state) => [state.rpc, state.currentWallet, state.refreshAllWallets]
  );
  if (!currentWallet) {
    return <div>You must select a wallet to perform a transfer.</div>;
  }
  const networkId = currentWallet.networkId;
  return (
    <TransferForm
      wallet={currentWallet}
      onSubmit={async (params) => {
        await currentWallet.wallet.transfer(params.recipient, BigInt(params.amount));
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
