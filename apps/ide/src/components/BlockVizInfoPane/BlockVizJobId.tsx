import { deserializeJobId, getCircuitNameForJobId, IQProvingJobDataID, ProvingJobCircuitType, ProvingJobDataType, QJobTopic } from '@qstudio/city-block';
import styles from './BlockVizInfoPane.module.scss';
import { useMemo, useState } from 'react';
import classNames from 'classnames';
interface IBlockVizJobIdProps {
  jobId: string;
}


interface IBlockVizJobInfoFieldProps {
  label: string;
  value: string | number;
  property: string
}

interface IBlockVizInfoDisplayProp {
  property: keyof IQProvingJobDataID;
  label: string;
  resolver?: (value: any)=>any;
}
const DisplayProps: IBlockVizInfoDisplayProp[] = [
  {
    property: "goal_id",
    label: "Checkpoint",
  },
  {
    property: "circuit_type",
    label: "Circuit",
    resolver: (value: any)=>(ProvingJobCircuitType[value]||"")
  },
  {
    property: "group_id",
    label: "Group",
  },
  {
    property: "sub_group_id",
    label: "Sub Group",
  },
  {
    property: "task_index",
    label: "Task Index",
  },
]
/*


interface IQProvingJobDataID {
  topic: QJobTopic,
  goal_id: number, // goal_id is u64, but block number should not exceed 2^53-1 (Number.MAX_SAFE_INTEGER)
  circuit_type: ProvingJobCircuitType,
  group_id: number,
  sub_group_id: number,
  task_index: number,
  data_type: ProvingJobDataType,
  data_index: number,
}*/
const BlockVizJobInfoField: React.FC<IBlockVizJobInfoFieldProps> = ({label, value, property})=>{
  return(
    <div className={classNames(styles.jobIdInfoField, styles["jifKey_"+property])}>
      <span className={styles.jobIdInfoFieldLabel}>{label}</span>
      <span className={styles.jobIdInfoFieldValue}>{value+""}</span>
    </div>
  )
}
const defaultJob: IQProvingJobDataID = {
  topic: QJobTopic.AggregateJobs,
  goal_id: 0,
  group_id: 0,
  sub_group_id: 0,
  circuit_type: ProvingJobCircuitType.Unknown,
  task_index: 0,
  data_type: ProvingJobDataType.InputWitness,
  data_index: 0
}
const BlockVizJobId: React.FC<IBlockVizJobIdProps> = ({jobId})=>{
  const [opened, setOpened] = useState(false);
  const decodedJobId = useMemo(()=>jobId?deserializeJobId(jobId):defaultJob,[jobId]);
  console.log(decodedJobId);
  if(!jobId){
    return (
      <div className={styles.jobIdCon}>
        <div className={styles.noJobSelectedMsg}>Select a widget to view info...</div>
      </div>
    )
  }

  const name = getCircuitNameForJobId(decodedJobId);

  return (
    <div className={styles.jobIdCon} onClick={(e)=>{
      e.preventDefault();
      e.stopPropagation();
      setOpened(!opened);
    }}>
    <div className={styles.jobIdTop}>
    <div className={styles.jobIdTopName}>{name}</div>
      <div className={styles.jobIdTopLabel}>Job ID</div>
      <div className={styles.jobIdHex}>{jobId}</div>
    </div>
      {opened && <div className={styles.jobIdInfoCon}>
        {DisplayProps.map(x=>(
          <BlockVizJobInfoField key={x.property} label={x.label} value={typeof x.resolver === 'function'?x.resolver(decodedJobId[x.property]):decodedJobId[x.property]} property={x.property} />
        ))}
      </div>}
    </div>
  )
};

export {
  BlockVizJobId,
}