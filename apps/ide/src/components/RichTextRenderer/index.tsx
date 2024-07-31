
import React, { useState, useEffect, useMemo } from "react";
import styles from './RichTextRenderer.module.scss';
import { IRichTextAnnotation, IRichTextElem, RichTextElemType, TRichTextContent } from "./types";
import { ICityL1Deposit, ICityRPCProvider, ICityUserState } from "@qstudio/city-sdk";
import { IAddressStatsResponse, IDogeLinkElectrsRPC, IScriptHashStatsResponse } from "doge-sdk";
import { Tooltip } from "@mantine/core";
import { CityJSON } from "packages/city-sdk/src/utils/json";
import { TbUser } from "react-icons/tb";
import classNames from "classnames";
import { DepositTooltipContent, UserTooltipContent } from "./TooltipContents";
import { TOOLTIP_COLOR } from "../../constants/style";
import { InlineTooltip } from "../InlineTooltip";
import { CopyInline } from "../CopyInline";
import L1TransactionIcon from "../icons/L1Transaction";
import L1DepositIcon from "../icons/L1Deposit";
interface IRichTextRendererProps {
  content: TRichTextContent;
  className?: string;
  rpc: ICityRPCProvider;
  dogeRPC: IDogeLinkElectrsRPC;
  checkpointId: number;
  blockExplorerUrl: string;
}


type TRichTextRendererMap = { [K in RichTextElemType]: RichTextElemRenderer<K> };
type RichTextElemRenderer<T extends RichTextElemType> = React.FC<{
  elem: IRichTextElem & { type: T },
  checkpointId: number,
  blockExplorerUrl: string,
  rpc: ICityRPCProvider,
  dogeRPC: IDogeLinkElectrsRPC,
}>;
const RichTextElemRenderers: TRichTextRendererMap = {
  [RichTextElemType.Annotation]: ({ elem }: { elem: IRichTextAnnotation; }) => {
    return <Tooltip w={300} withArrow color={TOOLTIP_COLOR} multiline={Array.isArray(elem.annotation)} inline label={Array.isArray(elem.annotation) ? elem.annotation.join("\n") : elem.annotation}>{elem.text}</Tooltip>;
  },
  [RichTextElemType.User]: ({ elem, rpc, checkpointId }) => {
    const [user, setUser] = useState<ICityUserState>();
    useEffect(() => {
      rpc.getUserById(checkpointId, Number(elem.userId))
        .then((u) => {
          setUser(u);
        })
        .catch(console.error);
    }, [elem.userId]);
    return <InlineTooltip label={user ? (<UserTooltipContent user={user} />) : "Loading..."}><span className={styles.userElem}><TbUser className={styles.inlineIcon} />{elem.text}</span></InlineTooltip>;
  },
  [RichTextElemType.Deposit]: ({ elem, rpc, checkpointId }) => {
    const [deposit, setDeposit] = useState<ICityL1Deposit>();
    useEffect(() => {
      rpc.getDepositById(checkpointId, Number(elem.depositId))
        .then((d) => {
          setDeposit(d);
        })
        .catch(console.error);
    }, [elem.depositId]);
    return <Tooltip color={TOOLTIP_COLOR} inline multiline label={deposit ? <DepositTooltipContent deposit={deposit} /> : "Loading..."}><span className={styles.depositElem}>
              <L1DepositIcon className={styles.l1DepositIcon} size={"1em"} />

      {elem.text}</span></Tooltip>;
  },
  [RichTextElemType.Hash]: ({ elem }) => {
    if (elem.text === elem.hash) {
      return <CopyInline className={classNames(styles.hashElem, styles.hashElemRaw)} value={elem.hash} label={<span className={styles.hashToolTipLabel}>{elem.hash}</span>}>{elem.text.substring(0, 12)}<span className={styles.elipsis}>...</span></CopyInline>;
    } else {
      return <CopyInline className={styles.hashElem} value={elem.hash} label={<span className={styles.hashToolTipLabel}>{elem.hash}</span>}>{elem.text}</CopyInline>;
    }
  },
  [RichTextElemType.L1Address]: ({ elem, dogeRPC, blockExplorerUrl }) => {
    const [stats, setStats] = useState<IAddressStatsResponse | IScriptHashStatsResponse>();
    useEffect(() => {
      dogeRPC.getStatsFor(elem.address)
        .then((info) => {
          setStats(info);
        })
        .catch(console.error);
    }, [elem.address]);
    return <Tooltip color={TOOLTIP_COLOR} inline label={stats ? (
      [
        `${elem.address}`,
        `${stats.chain_stats.funded_txo_sum - stats.chain_stats.spent_txo_sum}`,
      ].join("\n")
    ) : elem.address}><a href={blockExplorerUrl + "/address/" + encodeURIComponent(elem.address)} className={styles.l1AddressElem} target="_blank" referrerPolicy="no-referrer">{elem.text}</a></Tooltip>;
  },
  [RichTextElemType.TransactionId]: ({ elem, rpc, blockExplorerUrl }) => {
    if (elem.text === elem.txid) {
      return <InlineTooltip label={<span className={styles.hashToolTipLabel}>TXID: {elem.txid}</span>}><a href={blockExplorerUrl + "/tx/" + encodeURIComponent(elem.txid)} className={classNames(styles.txidElem, styles.txidElemRaw)} target="_blank" referrerPolicy="no-referrer">
        <L1TransactionIcon className={styles.txidIcon} size={"1em"} />
        {elem.txid.substring(0, 12)}<span className={styles.elipsis}>...</span>
      </a></InlineTooltip>;

    } else {
      return <InlineTooltip label={<span className={styles.hashToolTipLabel}>TXID: {elem.txid}</span>}><a href={blockExplorerUrl + "/tx/" + encodeURIComponent(elem.txid)} className={styles.txidElem} target="_blank" referrerPolicy="no-referrer">
        <L1TransactionIcon className={styles.txidIcon} size={"1em"} />

        {elem.text}</a></InlineTooltip>;
    }
  },
  [RichTextElemType.LineBreak]: ({ elem }) => {
    return <br />;
  },
};


const RichTextComponent: React.FC<IRichTextRendererProps> = ({ content, className, rpc, dogeRPC, checkpointId, blockExplorerUrl }) => {

  const children = useMemo<React.ReactNode>(() => {
    return typeof content === 'string' ? content : (content as any[]).map((elem, i) => {
      if (typeof elem === "string") {
        return elem;
      } else if (Object.hasOwnProperty.call(RichTextElemRenderers, elem.type)) {
        const Renderer = RichTextElemRenderers[elem.type as RichTextElemType];
        //@ts-ignore
        return <Renderer key={i} elem={elem as any} rpc={rpc} dogeRPC={dogeRPC} checkpointId={checkpointId} blockExplorerUrl={blockExplorerUrl} />;
      } else {
        return "";
      }
    })
  }, [content]);

  return <span className={classNames(styles.richTextContent, className)}>{children}</span>;


};

export {
  RichTextComponent,
}