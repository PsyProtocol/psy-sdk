import React, { useEffect, useState } from "react";
import { useWalletState } from "../../../../hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import styles from "./Transfer.module.scss";
import { Alert, Box, Button, Combobox, Input, InputBase, LoadingOverlay, TextInput, useCombobox } from "@mantine/core";
import { DogeInput } from "../../../DogeInput";
import type {IQWidgetWallet} from '../../../../types';
import { DogeNetworkId, ICreateP2PKHParams, createP2PKHTransaction, decodeAddressFull, u8ArrayToHex } from "doge-sdk";
import { IconInfoCircle } from "@tabler/icons-react";
import { BlokiesIcon } from "@qstudio/blokies-react";
import { WalletWidgetRPC } from "../../../../utils/rpc/walletRPC";
import { coinSelectP2PKH } from "packages/wallet-widget/src/utils/txPlanner";

interface ITransferFormProps {
  onSubmit: (params: ICreateP2PKHParams) => Promise<string>;
  onComplete: (txid: string) => void;
  className?: string;
  networkId: DogeNetworkId;
  wallet: IQWidgetWallet;
  rpc: WalletWidgetRPC;
}
function isValidAddress(networkId: DogeNetworkId, address: string): boolean {
  try {
    const parsed = decodeAddressFull(address);
    return parsed.networkId === networkId;
  } catch (e) {
    return false;
  }
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
        console.log("feeMap",feeMap )
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
  networkId,
  onSubmit,
  onComplete,
  className,
  rpc,
  wallet,
}) => {
  const [address, setAddress] = useState("");
  const [amount, setAmount] = useState(0);
  const [feeRate, setFeeRate] = useState(0);
  const [addressError, setAddressError] = useState<string>();
  const [amountError, setAmountError] = useState<string>();
  const [loadingState, setLoadingState] = useState<
    "idle" | "loading" | "success" | "error"
  >("idle");
  const [loadingError, setLoadingError] = useState<string>();
  const [estimateFeesResult, setEstimateFeesResult] = useState<ICreateP2PKHParams & {fee: number}>();

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
              label="Transfer Amount"
              placeholder="Transfer Amount..."
              error={addressError}
              useSats={true}
              onChange={(v) => {
                setAmount(v);
                if (amountError) {
                  setAmountError(undefined);
                }
                if(estimateFeesResult){
                  setEstimateFeesResult(undefined);
                }
              }}
              value={amount}
            />
          </div>
          <div className={styles.inputCon}>
            <FeeRateSelector
              onFeeRateChange={(v) => {
                setFeeRate(v);
                if(estimateFeesResult){
                  setEstimateFeesResult(undefined);
                }
              }}
              rpc={rpc}
            />
          </div>
          {estimateFeesResult?<div className={styles.inputCon}>
            <div>Estimated Fees: {(estimateFeesResult.fee/100_000_000).toFixed(3)} DOGE</div>
          </div>:null}
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
                if(!feeRate){
                  setLoadingError("Fee Rate is required");
                }
                if(estimateFeesResult){
                setLoadingState("loading");
                onSubmit(estimateFeesResult)
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
                }else{
                  try {
                    const result = coinSelectP2PKH(wallet.address, feeRate, wallet.utxos, [{address, value: amount}]);
                    setEstimateFeesResult(result);
                  }catch(err){
                    setLoadingError(err+"");
                    setLoadingState("error");
                  }
                }
              }
            }}
            disabled={!amount || address.length < 33 || !feeRate}
          >
            {estimateFeesResult?"Transfer":"Estimate Fees"}
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
  const [rpc, currentWallet, refreshCurrentWalletUTXOs] = useWalletState(
    (state) => [state.rpc, state.currentWallet, state.refreshCurrentWalletUTXOs]
  );
  if (!currentWallet) {
    return <div>You must select a wallet to perform a transfer.</div>;
  }
  const networkId = currentWallet.networkId;
  return (
    <TransferForm
      rpc={rpc}
      wallet={currentWallet}
      networkId={networkId}
      onSubmit={async (params) => {
        const ftx = createP2PKHTransaction(currentWallet.signer, params);
        const signed = await ftx.finalizeAndSign();
        const hexTx = u8ArrayToHex(signed.toBuffer());
        const txid = await rpc.sendRawTransaction(hexTx);
        
        //await rpc.mineBlocks(10);
        await rpc.waitForTx(txid);
        //await rpc.mineBlocks(10);
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

export { TransferModal };

export type { ITransferFormProps };
