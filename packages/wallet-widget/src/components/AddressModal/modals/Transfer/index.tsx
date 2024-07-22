import React, { useEffect, useState } from "react";
import { useWalletState } from "../../../../hooks/useWalletState";
import { TAddressModalComponent } from "../../index";
import styles from "./Transfer.module.scss";
import { Alert, Box, Button, Checkbox, Combobox, Input, InputBase, LoadingOverlay, Switch, TextInput, useCombobox } from "@mantine/core";
import { DogeInput } from "../../../DogeInput";
import type {IQWidgetWallet} from '../../../../types';
import { DogeNetworkId, ICreateP2PKHParams, createP2PKHTransaction, decodeAddressFull, isP2SHAddress, u8ArrayToHex } from "doge-sdk";
import { IconInfoCircle } from "@tabler/icons-react";
import { BlokiesIcon } from "@qstudio/blokies-react";
import { WalletWidgetRPC } from "../../../../utils/rpc/walletRPC";
import { coinSelectP2PKH, getStandardP2PKHTxSize } from "../../../../utils/txPlanner";
import { waitMs } from "packages/wallet-widget/src/utils/wait";

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
    if(parsed.networkId === networkId){
      return true;
    }else if(networkId === "dogeRegtest" && parsed.networkId === "dogeTestnet" && parsed.version === 0xc4){
      return true;
    }else{
      return false;
    }
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

async function transferSinglet(wallet: IQWidgetWallet, amount: number, feeRate: number, destination: string, rpc: WalletWidgetRPC, onStatusUpdate: (status: string)=>any = ()=>0){
  const isP2SH = isP2SHAddress(destination);
  const tx2Size = getStandardP2PKHTxSize(1, isP2SH?0:1, isP2SH?1:0);
  const tx2Cost = Math.ceil(tx2Size * feeRate);
  const totalCost2 = amount + tx2Cost;
  const result = coinSelectP2PKH(wallet.address, feeRate, wallet.utxos, [{address: wallet.address, value: totalCost2}])
  const tx1 = createP2PKHTransaction(wallet.signer, result);
  onStatusUpdate("Signing Self Transfer...");
  const signed = await tx1.finalizeAndSign();
  const hexTx = u8ArrayToHex(signed.toBuffer());
  const txid = await rpc.sendRawTransaction(hexTx);
  if(wallet.networkId === "dogeRegtest"){
    await rpc.mineBlocks(10);
    await waitMs(3000);
  }
  onStatusUpdate("Waiting for Self Transfer to be Confirmed...");
  
  //await rpc.mineBlocks(10);
  const resp = await rpc.waitForTx(txid);
  onStatusUpdate("Self Transfer Complete");
  let ind = -1;
  for(let i=0; i<resp.vout.length; i++){
    if(resp.vout[i].value === totalCost2){
      ind = i;
      break;
    }
  }
  
  return {
    address: wallet.address,
    inputs: [{
      value: totalCost2,
      txid: txid,
      vout: ind,
    }],
    outputs: [{
      address: destination,
      value: amount,
    }]
  };








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
  const [isSinglet, setIsSinglet] = useState(false);
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
              error={amountError}
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
          <div className={styles.singletInputCon}>
    <Checkbox
      checked={isSinglet}
      onChange={(e)=>{
        setIsSinglet(e.target.checked);
        if(estimateFeesResult){
          setEstimateFeesResult(undefined);
        }
      }}
      labelPosition="left"
      label="Singlet Transaction"
      description="Send the transaction with a single P2PKH input"
      size="xs"
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
                const base = isSinglet?transferSinglet(wallet, amount, feeRate, address, rpc, (s)=>console.log(s)):Promise.resolve(estimateFeesResult);
                base.then((r)=>onSubmit(r))
                  .then((txid) => {
                    onComplete(txid);
                  })
                  .catch((e) => {
                    setLoadingState("error");
                    setLoadingError(e.message);
                  });
                }else{
                  try {
                    const result = coinSelectP2PKH(wallet.address, feeRate, wallet.utxos, [{address, value: amount}]);
                    const isDestP2SH = isP2SHAddress(address); 
                    const extraFees = isSinglet?(getStandardP2PKHTxSize(1, isDestP2SH?0:1, isDestP2SH?1:0)*feeRate):0;
                    setEstimateFeesResult({...result, fee: result.fee+extraFees});
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
        if(networkId === "dogeRegtest"){
          await rpc.mineBlocks(10);
        }
        
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
