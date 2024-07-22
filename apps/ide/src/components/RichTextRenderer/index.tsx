
import React, {useState, useEffect, useMemo} from "react";
import styles from './RichTextRenderer.module.scss';
import { IRichTextAnnotation, IRichTextElem, RichTextElemType, TRichTextContent } from "./types";
import { ICityL1Deposit, ICityRPCProvider, ICityUserState } from "@qstudio/city-sdk";
import { IAddressStatsResponse, IDogeLinkElectrsRPC, IScriptHashStatsResponse } from "doge-sdk";
import { Tooltip } from "@mantine/core";
import { CityJSON } from "packages/city-sdk/src/utils/json";
import classNames from "classnames";
interface IRichTextRendererProps {
  content: TRichTextContent;
  className?: string;
  rpc: ICityRPCProvider;
  dogeRPC: IDogeLinkElectrsRPC;
  checkpointId: number;
  blockExplorerUrl: string;
}



type TRichTextRendererMap= {[K in RichTextElemType]: RichTextElemRenderer<K>};
type RichTextElemRenderer<T extends RichTextElemType> = React.FC<{elem: IRichTextElem & {type: T}, 
checkpointId: number,
blockExplorerUrl: string,
rpc: ICityRPCProvider,
dogeRPC: IDogeLinkElectrsRPC,}>;
const RichTextElemRenderers: TRichTextRendererMap = {
  [RichTextElemType.Annotation]: ({elem}: {elem: IRichTextAnnotation})=>{
    return <Tooltip multiline={Array.isArray(elem.annotation)} inline label={Array.isArray(elem.annotation)?elem.annotation.join("\n"):elem.annotation}>{elem.text}</Tooltip>
  },
  [RichTextElemType.User]: ({elem, rpc, checkpointId})=>{
    const [user, setUser] = useState<ICityUserState>();
    useEffect(()=>{
      rpc.getUserById(checkpointId, Number(elem.userId))
      .then((u)=>{
        setUser(u);
      })
      .catch(console.error)
    },[elem.userId]);
    return  <Tooltip inline multiline label={user?CityJSON.stringify(user, 2):"Loading..."}><span className={styles.userElem}>{elem.text}</span></Tooltip>;
  },
  [RichTextElemType.Deposit]: ({elem, rpc, checkpointId})=>{
    const [deposit, setDeposit] = useState<ICityL1Deposit>();
    useEffect(()=>{
      rpc.getDepositById(checkpointId, Number(elem.depositId))
      .then((d)=>{
        setDeposit(d);
      })
      .catch(console.error)
    },[elem.depositId]);
    return  <Tooltip inline multiline label={deposit?CityJSON.stringify(deposit, 2):"Loading..."}><span className={styles.depositElem}>{elem.text}</span></Tooltip>;
  },
  [RichTextElemType.Hash]: ({elem})=>{
    return <span className={styles.hashElem}>{elem.text}</span>
  },
  [RichTextElemType.L1Address]: ({elem, dogeRPC, blockExplorerUrl})=>{
    const [stats, setStats] = useState<IAddressStatsResponse|IScriptHashStatsResponse>();
    useEffect(()=>{
      dogeRPC.getStatsFor(elem.address)
      .then((info)=>{
        setStats(info);
      })
      .catch(console.error)
    },[elem.address]);
    return <Tooltip inline label={stats?(
      [
        `${elem.address}`,
        `${stats.chain_stats.funded_txo_sum - stats.chain_stats.spent_txo_sum}`,

      ].join("\n")
    ):elem.address}><a href={blockExplorerUrl+"/address/"+encodeURIComponent(elem.address)} className={styles.l1AddressElem} target="_blank" referrerPolicy="no-referrer">{elem.text}</a></Tooltip>;
  },
  [RichTextElemType.TransactionId]: ({elem, rpc, blockExplorerUrl})=>{
    return <a href={blockExplorerUrl+"/tx/"+encodeURIComponent(elem.txid)} className={styles.txidElem} target="_blank" referrerPolicy="no-referrer">{elem.text}</a>
  },  
};


const RichTextComponent: React.FC<IRichTextRendererProps> = ({content, className, rpc, dogeRPC, checkpointId, blockExplorerUrl})=>{

  const children = useMemo<React.ReactNode>(()=>{
    return typeof content === 'string'?content:content.map((elem, i)=>{
      if(typeof elem === "string"){
        return elem;
      }else if(Object.hasOwnProperty.call(RichTextElemRenderers, elem.type)){
        const Renderer = RichTextElemRenderers[elem.type];
        //@ts-ignore
        return <Renderer key={i} elem={elem as any} rpc={rpc} dogeRPC={dogeRPC} checkpointId={checkpointId} blockExplorerUrl={blockExplorerUrl} />;
      }else{
        return "";
      }
  })
},[content]);

  return <span className={classNames(styles.richTextCon)}>{children}</span>;


};

export {
  RichTextComponent,
}