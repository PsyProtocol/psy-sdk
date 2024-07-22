import { ICityRPCProvider } from "@qstudio/city-sdk";
import classNames from "classnames";
import styles from './MineBlockButton.module.scss';
import{Button, rem} from '@mantine/core';
import { GiMiner } from "react-icons/gi";
import { useEffect, useState } from "react";
import {IDogeLinkElectrsRPC, IUTXO} from "doge-sdk";
function waitMs(ms: number) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}
interface IMineBlockButtonProps {
  rpc: ICityRPCProvider;
  dogeRPC: IDogeLinkElectrsRPC;
  className?: string;
}

async function waitForBlockSpend(dogeRPC: IDogeLinkElectrsRPC, address: string, interval = 1000, maxAttempts = 9999999){
  const stats = await dogeRPC.getStatsFor(address);
  const baseSpends = stats.chain_stats.spent_txo_count;
  for(let i = 0; i<maxAttempts; i++){
    const newStats = await dogeRPC.getStatsFor(address);
    console.log("oldStats",stats,"newStats",newStats);
    if(newStats.chain_stats.spent_txo_count>baseSpends){
      return;
    }
    await waitMs(interval);
  }
  throw new Error("Timeout waiting for block spend");
}
async function produceBlockGetNewAddress(rpc: ICityRPCProvider) {
  const currentBlock = await rpc.getLatestBlockState();
  const startAddress = await rpc.getCityBlockDepositAddressString(currentBlock.checkpoint_id+1);
  await rpc.produceBlock();
  /*
  await waitMs(1000);
  let nextBlock = await rpc.getLatestBlockState();
  while(nextBlock.checkpoint_id === currentBlock.checkpoint_id){
    nextBlock = await rpc.getLatestBlockState();
    if(nextBlock.checkpoint_id === currentBlock.checkpoint_id){
      await waitMs(1000);
    }
  }*/
  return startAddress;
}
async function mineForBlock(rpc: ICityRPCProvider, dogeRPC: IDogeLinkElectrsRPC){
  const address = await produceBlockGetNewAddress(rpc);
  await waitForBlockSpend(dogeRPC, address);
  const nextBlock = await rpc.getLatestBlockState();
  return nextBlock.checkpoint_id;
}
const MineBlockButton: React.FC<IMineBlockButtonProps> = ({rpc, dogeRPC, className}) => {
  const [checkpointId, setCheckpointId] = useState(-1);
  const [loading, setLoading] = useState(false);


  useEffect(()=>{
    rpc.getLatestBlockState().then((block)=>{
      if(block.checkpoint_id !== checkpointId){
        setCheckpointId(block.checkpoint_id);
      }
    }).catch(console.error);
    const interval = setInterval(()=>{
      rpc.getLatestBlockState().then((block)=>{
        if(block.checkpoint_id !== checkpointId){
          setCheckpointId(block.checkpoint_id);
        }
      }).catch(console.error);
    },10000);

    return ()=>{
      clearInterval(interval);
    }
  },[]);

  const leftText = checkpointId===-1?"...":checkpointId.toString();

  const fontSizeLeft = Math.max(8, 14/leftText.length);

  console.log(styles);

  return (
    <Button classNames={styles} loading={loading} onClick={() => {
      setLoading(true);
      mineForBlock(rpc, dogeRPC).then((newCheckpointId)=>{
        setCheckpointId(newCheckpointId);
        setLoading(false);
      }).catch((err)=>{
          console.error(err);
          setLoading(false);
      });
    }}
    size="xs"
    radius="md"
    variant="default"
    rightSection={<GiMiner size={"24px"} />}
    leftSection={<span style={{fontSize: fontSizeLeft}}>{leftText}</span>}
    >
      Mine Block
    </Button>
  );
}

export {
  MineBlockButton,
}