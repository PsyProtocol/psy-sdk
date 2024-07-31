import React, { useEffect, useState } from "react";
import { BlockVizDataStore } from "../../utils/blockviz/BlockVizDataStore";
import styles from "./BlockVizInfoPane.module.scss";
import { useSelectedJobIdAndVizScenario } from "../../hooks/useSelectedJobId";
import { BlockVizJobId } from "./BlockVizJobId";
import { Button, Textarea } from "@mantine/core";
import { BlockVizJobSummary } from "./BlockVizJobSummary";

interface IBlockVizInfoPaneProps {
  blockVizDataStore: BlockVizDataStore;
}
const BlockVizInfoPane: React.FC<IBlockVizInfoPaneProps> = ({blockVizDataStore}) => {
  const {scenario, selectedJobId, psBlock} = useSelectedJobIdAndVizScenario(blockVizDataStore);

  /*const [witness, setWitness] = useState<any>();

  useEffect(()=>{
    setWitness(undefined);
  },[selectedJobId]);*/
  console.log("ips",psBlock,scenario);
  if(!selectedJobId){ 
    return (
      <div className={styles.blockVizInfoPaneNoSel}>
        <div className={styles.noSelMessage}>Please select a job in the viz panel...</div>
      </div>
    )
  }
  return(
    <div className={styles.blockVizInfoPane}>
      <div className={styles.bviTop}>
        <BlockVizJobId jobId={selectedJobId} />
      </div>
      <div className={styles.bviBody}>
        {psBlock&&<BlockVizJobSummary psBlock={psBlock} jobId={selectedJobId} />}
      </div>
      <div className={styles.bviBottom}>
      </div>
    </div>
  );
};
export {
  BlockVizInfoPane,
}
