import { PSCityBlock } from "@qstudio/city-block";
import { IBlockVizJobInfo, IRealBlockVizJobInfo } from "../content/types";
import styles from './BlockVizJobSummary.module.scss';
import { useEffect, useState } from "react";
import { LoadingOverlay, Textarea } from "@mantine/core";
import { RichTextComponent } from "../../RichTextRenderer";
import { TRichTextContent } from "../../RichTextRenderer/types";
interface IBlockVizJobSummaryProps {
  psBlock: PSCityBlock<IRealBlockVizJobInfo>;
  jobId: string;
}

const PSRichText: React.FC<{content: TRichTextContent, psBlock: PSCityBlock<IRealBlockVizJobInfo>}> = ({content, psBlock}) => {
  return (
    <RichTextComponent content={content} rpc={psBlock.rpc} dogeRPC={psBlock.dogeRPC} blockExplorerUrl={psBlock.blockExplorerUrl} checkpointId={psBlock.checkpoint_id} />
  );
}
const BlockVizJobSummary: React.FC<IBlockVizJobSummaryProps> = ({ psBlock, jobId }) => {
  const [info, setInfo] = useState<IRealBlockVizJobInfo>();
  useEffect(()=>{
    setInfo(undefined);
    psBlock.loadJobInfo(jobId).then((res)=>setInfo(res)).catch(console.error);
  },[jobId,psBlock]);




  if(!info){
    return (
      <div className={styles.blockVizJobSummary}>
        <LoadingOverlay />
      </div>
    );
  }




  return (
    <div className={styles.blockVizJobSummary}>
      <div className={styles.blockVizJobSummaryTop}>
      <div className={styles.bvjsTitle}>{info.title}</div>
      <div className={styles.bvjsDescription}>{info.description}</div>
      <div className={styles.bvjsSummary}>
        <PSRichText psBlock={psBlock} content={info.summary} />
      </div>
      </div>
      <div className={styles.blockVizJobActions}>
        <div className={styles.actionsTitle}>Actions</div>
      <div className={styles.blockVizJobActionsInner}>
        {info.shortActions.map((action, idx)=>(
          <div className={styles.bvjaAction} key={idx}>
            <PSRichText psBlock={psBlock} content={action} />
          </div>
        ))}
      </div>
      </div>
      <div className={styles.blockVizJobWitness}>
        <Textarea className={styles.blockVizJobWitnessRawTA} label="Raw Witness" description="Raw JSON Witness of the Proof" value={JSON.stringify(info.witness, null, 2)} readOnly />
      </div>
    </div>
  );
}

export {
  BlockVizJobSummary,
}