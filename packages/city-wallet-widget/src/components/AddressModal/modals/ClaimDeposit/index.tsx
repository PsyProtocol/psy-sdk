import React, { useEffect, useState } from "react";
import { useWalletState } from "../../../../hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import styles from "./ClaimDeposit.module.scss";
import { Alert, Box, Button, LoadingOverlay, TextInput } from "@mantine/core";
interface IDepositRequestItem {
  hash: string;
  deposit: ICityL1Deposit;
}
interface IClaimDepositFormProps {
  onSubmit: (params: {txid: string, signature: string}) => Promise<any>;
  onComplete: (txid: string) => void;
  className?: string;
  wallet: IQCityWidgetWallet
}
import { IconInfoCircle } from "@tabler/icons-react";
import { ICityL1Deposit } from "@qstudio/city-sdk";
import { formatBalance } from "../../../../utils/balance";
import { IQCityWidgetWallet } from "../../../../types";
import { verifyNormalizeSecp256K1Signature } from "packages/city-wallet-widget/src/utils/signature";
import { WWCopyButton } from "../../../WWCopyButton";
const DepositInfo: React.FC<IDepositRequestItem> = ({deposit, hash}) => {
  return (
    <div className={styles.depositInfo}>
     <div className={styles.depositHeader}>Deposit Info</div>
     <div className={styles.depositRow}>
       <div className={styles.depositLabel}>Deposit Amount</div>
       <div className={styles.depositValue}>{formatBalance(deposit.value, "DOGE")}</div>
      </div>
     <div className={styles.depositRow}>
       <div className={styles.depositLabel}>Deposit Public Key</div>
       <div className={styles.depositValue}>{deposit.public_key}</div>
      </div>
      <div className={styles.depositRow}>
        <div className={styles.depositLabel}>Deposit Message Hash</div>
        <div className={styles.depositValueHash}>{hash}

          <WWCopyButton value={hash} className={styles.copyBtn} />
        </div>
      </div>
    </div>
  );
}
const ClaimDepositForm: React.FC<IClaimDepositFormProps> = ({
  onSubmit,
  onComplete,
  className,
  wallet,
}) => {
  const [txid, setTxid] = useState("");
  const [signature, setSignature] = useState("");
  const [deposit, setDeposit] = useState<IDepositRequestItem>();
  const [txidError, setTxidError] = useState<string>();
  const [signatureError, setSignatureError] = useState<string>();
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
              label="Transaction ID"
              placeholder="Transaction ID of Deposit..."
              error={txidError}
              spellCheck={false}
              value={txid}
              onChange={(e) => {
                setTxid(e.currentTarget.value.replace(/[^a-f|0-9|A-F]+/g, ""));
                if (txidError) {
                  setTxidError(undefined);
                }
                if(deposit){
                  setDeposit(undefined);
                  setSignature("");
                }
              }}
            />
          </div>
          {deposit?<div className={styles.depositInfoCon}>
            <DepositInfo {...deposit} />
          </div>:null}
          {deposit?<div className={styles.inputCon}>
            <TextInput
              label="Signature"
              placeholder="Signature of Deposit..."
              error={signatureError}
              spellCheck={false}
              value={signature}
              onChange={(e) => {
                setSignature(e.currentTarget.value.replace(/[^a-f|0-9|A-F]+/g, ""));
                if (signatureError) {
                  setSignatureError(undefined);
                }
              }}
            />
          </div>:null}
        </div>
        <div className={styles.formControls}>
          <Button
            onClick={() => {
              setLoadingError(undefined);

              if(!txid.length){
                setTxidError("Transaction ID is required");
                return;
              }
              if(deposit){
                if(!signature.length){
                  setSignatureError("Signature is required");
                  return;
                }
                let normalizedSignature = "";
                try{
                  normalizedSignature = verifyNormalizeSecp256K1Signature(signature, deposit.hash, deposit.deposit.public_key);
                }catch(e){
                  setSignatureError(e+"");
                  return;
                }
                setLoadingState("loading");
                onSubmit({txid, signature: normalizedSignature}).then(()=>{
                  setLoadingState("success");
                  onComplete(txid);
                }).catch((e)=>{
                  setLoadingState("error");
                  setLoadingError(e.message);
                });
              }else{
                setLoadingState("loading");
                wallet.wallet.getClaimDepositMessageHash(txid).then((res)=>{
                  setDeposit(res);
                  setLoadingState("idle");
                }).catch((e)=>{
                  setLoadingState("error");
                  setLoadingError(e.message);
                });
              }
            }}
            disabled={txid.length !== 64 || (deposit && signature.length < 64)}
          >
            {deposit?"Claim Deposit":"Get Deposit Info"}
          </Button>
        </div>
      </Box>
    </div>
  );
};
const ClaimDepositModal: TAddressModalComponent = ({
  onCancel,
  onComplete,
}) => {
  const [provider, currentWallet, refreshCurrentWallet] = useWalletState(
    (state) => [state.provider, state.currentWallet, state.refreshCurrentWallet]
  );
  if ( !currentWallet) {
    return <div>Select a wallet before claiming a deposit.</div>;
  }
  const networkId = currentWallet.networkId;
  return (
    <ClaimDepositForm
      wallet={currentWallet}
      onSubmit={async ({txid, signature}) => {
        await currentWallet.wallet.claimDeposit(txid, signature, provider.prover);
        await refreshCurrentWallet();
        return txid;
      }}
      onComplete={(txid) => {
        refreshCurrentWallet().then(() => {
          onComplete({ txid });
        });
      }}
    />
  );
};

export { ClaimDepositModal };

export type { IClaimDepositFormProps };
