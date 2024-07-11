import React, { useEffect, useState } from "react";
import { useWalletState } from "../../hooks/useWalletState";
import { IGetTXResponse } from "doge-sdk";
import styles from './Transactions.module.scss';
import { ActionIcon, Group, Text, rem } from "@mantine/core";
import { IconRefresh } from "@tabler/icons-react";
import { WalletTransaction } from "./WalletTransaction";
import { WWTransaction } from "./WWTransaction";

interface IWalletTransactionsProps {

}
const WalletTransactions = (props: IWalletTransactionsProps) => {
  const [currentWallet, rpc] = useWalletState((state)=>[state.currentWallet, state.rpc]);
  const [txs, setTxs] = useState<IGetTXResponse[]>([]);
  const [needsLoad, setNeedsLoad] = useState(true);
  const [loading, setLoading] = useState(false);

  useEffect(()=>{
    if(currentWallet&&needsLoad){
      rpc.getTransactionsFor(currentWallet.address).then((result)=>{
        setTxs(result);
      }).catch(err=>console.error(err));
    }
  },[needsLoad, currentWallet]);

  if(!needsLoad || !currentWallet){
    return <div></div>;
  }

  
  return (
    <div className={styles.walletTransactionsWidget}>
      <Group justify="space-between">
        <Text style={{fontSize:"24px"}}>Transactions</Text>
          <ActionIcon variant="subtle" color="gray" className={styles.actionIcon} 
          onClick={()=>{
            setLoading(true);

      rpc.getTransactionsFor(currentWallet.address).then((result)=>{
        
        setTxs(result);
        setLoading(false);
      }).catch(err=>{
        setLoading(false);
        console.error("error loading transactions",err);
      })
          }}
            loading={loading}
            >
            <IconRefresh style={{ width: rem(16), height: rem(16) }} />
          </ActionIcon>
      </Group>
      <div className={styles.walletTransactionsList}>
        {txs.map(tx=>(
          <WWTransaction
            selfAddress={currentWallet.address}
            key={tx.txid}
            url={rpc.getTXURL(tx.txid)}
            {...tx}
          />
        ))}
      </div>
    </div>
  );
}

export {
  WalletTransactions,
}