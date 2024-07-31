/*
import React, {useState, useEffect, useMemo} from "react";
import styles from './RichTextRenderer.module.scss';
import { IRichTextAnnotation, IRichTextElem, RichTextElemType, TRichTextContent } from "./types";
import { ICityL1Deposit, ICityRPCProvider, ICityUserState } from "@qstudio/city-sdk";
import { IAddressStatsResponse, IDogeLinkElectrsRPC, IScriptHashStatsResponse } from "doge-sdk";
import { Tooltip } from "@mantine/core";
import { CityJSON } from "packages/city-sdk/src/utils/json";
import { TbUser } from "react-icons/tb";
import classNames from "classnames";
import { UserTooltipContent } from "./TooltipContents";
import { TOOLTIP_COLOR } from "../../constants/style";
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
    return <Tooltip withArrow color={TOOLTIP_COLOR} multiline={Array.isArray(elem.annotation)} inline label={Array.isArray(elem.annotation)?elem.annotation.join("\n"):elem.annotation}>{elem.text}</Tooltip>
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
    return  <Tooltip withArrow={true} color={TOOLTIP_COLOR} inline multiline label={user?(<UserTooltipContent user={user} />):"Loading..."}><span className={styles.userElem}><TbUser className={styles.inlineIcon}/>{elem.text}</span></Tooltip>;
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
    return  <Tooltip color={TOOLTIP_COLOR} inline multiline label={deposit?CityJSON.stringify(deposit, 2):"Loading..."}><span className={styles.depositElem}>{elem.text}</span></Tooltip>;
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
    return <Tooltip color={TOOLTIP_COLOR} inline label={stats?(
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
    return typeof content === 'string'?content:(content as any[]).map((elem, i)=>{
      if(typeof elem === "string"){
        return elem;
      }else if(Object.hasOwnProperty.call(RichTextElemRenderers, elem.type)){
        const Renderer = RichTextElemRenderers[elem.type as RichTextElemType];
        //@ts-ignore
        return <Renderer key={i} elem={elem as any} rpc={rpc} dogeRPC={dogeRPC} checkpointId={checkpointId} blockExplorerUrl={blockExplorerUrl} />;
      }else{
        return "";
      }
  })
},[content]);

  return <span className={classNames(styles.richTextContent, className)}>{children}</span>;


};

export {
  RichTextComponent,
}*/