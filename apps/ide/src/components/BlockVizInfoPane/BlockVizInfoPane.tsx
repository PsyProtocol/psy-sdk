import React from "react";
import { BlockVizDataStore } from "../../utils/blockviz/BlockVizDataStore";
import styles from "./BlockVizInfoPane.module.scss";
import { useSelectedJobIdAndVizScenario } from "../../hooks/useSelectedJobId";
import { BlockVizJobId } from "./BlockVizJobId";
import { Button } from "@mantine/core";

interface IBlockVizInfoPaneProps {
  blockVizDataStore: BlockVizDataStore;
}
const BlockVizInfoPane: React.FC<IBlockVizInfoPaneProps> = ({blockVizDataStore}) => {
  const {scenario, selectedJobId, psBlock} = useSelectedJobIdAndVizScenario(blockVizDataStore);
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
        {psBlock && <div className={styles.bviBodyBlock}>

            <Button onClick={async ()=>{
              const res = await psBlock.loadJobWitness(selectedJobId);
              console.log("Loaded Witness", res);

            }}>Load Witness</Button>
          </div>}
      </div>
      <div className={styles.bviBottom}>
      </div>
    </div>
  );
};
export {
  BlockVizInfoPane,
}
