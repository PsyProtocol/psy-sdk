import React, { useEffect, useState } from "react";
import { useWalletState } from "packages/wallet-widget/src/hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import styles from "./FaucetFromWallet.module.scss";
import { Alert, Box, Button, LoadingOverlay, TextInput } from "@mantine/core";
import { DogeInput } from "../../../DogeInput";

interface IFaucetFromWalletFormProps {
  onSubmit: (address: string, amount: number) => Promise<string>;
  onComplete: (txid: string) => void;
  defaultAddress?: string;
  className?: string;
  networkId: DogeNetworkId;
}
import { DogeNetworkId, decodeAddressFull } from "doge-sdk";
import { waitMs } from "packages/wallet-widget/src/utils/wait";
import { IconInfoCircle } from "@tabler/icons-react";
function isValidAddress(networkId: DogeNetworkId, address: string): boolean {
  try {
    const parsed = decodeAddressFull(address);
    return parsed.networkId === networkId;
  } catch (e) {
    return false;
  }
}
const FaucetFromWalletForm: React.FC<IFaucetFromWalletFormProps> = ({
  defaultAddress,
  networkId,
  onSubmit,
  onComplete,
  className,
}) => {
  const [address, setAddress] = useState(defaultAddress ? defaultAddress : "");
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
              title="Faucet Error"
              icon={<IconInfoCircle />}
            >
              {loadingError}
            </Alert>
          ) : null}

          <div className={styles.inputCon}>
            <TextInput
              label="Address"
              placeholder="Address to faucet tokens to..."
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
              label="Faucet Amount"
              placeholder="Faucet Amount..."
              error={addressError}
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
                setAddressError("Address is required");
                return;
              } else if (!isValidAddress(networkId, address)) {
                setAddressError("Invalid Address");
              } else if (!amount) {
                setAmountError("Amount must be greater than 0");
              } else {
                setLoadingState("loading");
                onSubmit(address, amount)
                  .then((txid) => {
                    onComplete(txid);
                    /*
                setLoadingState("success");
                waitMs(2000).then(() => {
                  setLoadingState("idle");
                  setAddress("");
                  setAmount(0);
                  setLoadingError(undefined);
                  setAmountError(undefined);
                  setAddressError(undefined);
                  setLoadingState("idle");
                });*/
                  })
                  .catch((e) => {
                    setLoadingState("error");
                    setLoadingError(e.message);
                  });
              }
            }}
            disabled={!amount || address.length < 33}
          >
            Faucet
          </Button>
        </div>
      </Box>
    </div>
  );
};
const FaucetFromWalletModal: TAddressModalComponent = ({
  onCancel,
  onComplete,
}) => {
  const [rpc, currentWallet, refreshCurrentWalletUTXOs] = useWalletState(
    (state) => [state.rpc, state.currentWallet, state.refreshCurrentWalletUTXOs]
  );
  if (!rpc.canSendFromWallet() || !currentWallet) {
    return <div>Faucet not enabled for this rpc provider.</div>;
  }
  const networkId = currentWallet.networkId;
  return (
    <FaucetFromWalletForm
      defaultAddress={currentWallet.address}
      networkId={networkId}
      onSubmit={async (address, amount) => {
        await rpc.mineBlocks(100);
        const txid = await rpc.sendFromWallet(address, amount);
        await rpc.mineBlocks(10);
        await rpc.waitForTx(txid);
        await rpc.mineBlocks(10);
        await refreshCurrentWalletUTXOs();
        return txid;
      }}
      onComplete={(txid) => {
        refreshCurrentWalletUTXOs().then(() => {
          onComplete({ txid });
        });
      }}
    />
  );
};

export { FaucetFromWalletModal };

export type { IFaucetFromWalletFormProps };
