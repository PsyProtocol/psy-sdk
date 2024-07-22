import React, { useState } from "react";
import { useWalletState } from "../../../../hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import styles from "./Withdraw.module.scss";
import { Alert, Box, Button,  LoadingOverlay, TextInput, } from "@mantine/core";
import { DogeInput } from "../../../DogeInput";
import type {IQCityWidgetWallet} from '../../../../types';
import { DogeNetworkId, decodeAddressFull } from "doge-sdk";
import { IconInfoCircle } from "@tabler/icons-react";
import { BlokiesIcon } from "@qstudio/blokies-react";

interface IWithdrawFormProps {
  onSubmit: (params: {address: string, amount: string, feeAmount: number}) => Promise<any>;
  onComplete: () => void;
  className?: string;
  networkId: DogeNetworkId;
  wallet: IQCityWidgetWallet;
}
function isValidAddress(networkId: DogeNetworkId, address: string): boolean {
  try {
    const parsed = decodeAddressFull(address);
    return parsed.networkId === networkId;
  } catch (e) {
    return false;
  }
}

const WithdrawForm: React.FC<IWithdrawFormProps> = ({
  networkId,
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
  const feeAmount = 100_000;

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
              title="Withdraw Error"
              icon={<IconInfoCircle />}
            >
              {loadingError}
            </Alert>
          ) : null}
          {(address&&address.length>=33)?<div className={styles.blokiesCon}>
            <BlokiesIcon seed={address} size={8} scale={8} className={styles.blokiesIcon} />
          </div>:null}

          <div className={styles.inputCon}>
            <TextInput
              label="Recipient Address"
              placeholder="Address to send tokens to..."
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
              label="Withdraw Amount"
              placeholder="Withdraw Amount..."
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
          <div className={styles.inputCon}>
            <DogeInput
              label="Fee"
              placeholder="Fee Amount..."
              error={addressError}
              useSats={true}
              value={feeAmount}
              disabled={true}
              onChange={()=>0}
            />
          </div>
        </div>
        <div className={styles.formControls}>
          <Button
            onClick={() => {
              setLoadingError(undefined);
              if (!address.length) {
                setAddressError("Address is required");
                return;
              } else if (!isValidAddress(networkId, address)) {
                setAddressError("Invalid Address");
              } else if (!amount) {
                setAmountError("Amount must be greater than 0");
              } else {
                setLoadingState("loading");
                onSubmit({feeAmount, address, amount: amount+""})
                  .then(() => {
                    onComplete();
                  })
                  .catch((e) => {
                    setLoadingState("error");
                    setLoadingError(e.message);
                  });
                
              }
            }}
            disabled={!amount || address.length < 33}
          >
            Withdraw
          </Button>
        </div>
      </Box>
    </div>
  );
};
const WithdrawModal: TAddressModalComponent = ({
  onCancel,
  onComplete,
}) => {
  const [currentWallet, refreshCurrentWallet] = useWalletState(
    (state) => [state.currentWallet, state.refreshCurrentWallet]
  );
  if (!currentWallet) {
    return <div>You must select a wallet to perform a withdrawal.</div>;
  }
  const networkId = currentWallet.networkId;
  return (
    <WithdrawForm
      wallet={currentWallet}
      networkId={networkId}
      onSubmit={async (params) => {
        await currentWallet.wallet.withdraw(params.address, BigInt(params.amount));
      }}
      onComplete={() => {
        refreshCurrentWallet().then(() => {
          onComplete({});
        }).catch(err=>console.error("error refreshing wallet", err))
      }}
    />
  );
};

export { WithdrawModal };

export type { IWithdrawFormProps };
