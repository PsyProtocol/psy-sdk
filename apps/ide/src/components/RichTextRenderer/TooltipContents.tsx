import { cityFelt, cityFeltSatsToDoge, ICityL1Deposit, ICityUserState } from "@qstudio/city-sdk";
import styles from './TooltipContents.module.scss';
const UserTooltipContent: React.FC<{ user: ICityUserState }> = ({ user }) => {
  return (
    <div className={styles.ttContent}>
      <div className={styles.ttField}>
        <div className={styles.ttFieldLabel}>User ID</div>
        <div className={styles.ttFieldValue}>{user.user_id}</div>
      </div>
      <div className={styles.ttField}>
        <div className={styles.ttFieldLabel}>Balance</div>
        <div className={styles.ttFieldValue}>{cityFeltSatsToDoge(user.balance)} DOGE</div>
      </div>
      <div className={styles.ttField}>
        <div className={styles.ttFieldLabel}>Nonce</div>
        <div className={styles.ttFieldValue}>{cityFelt(user.nonce) + ""}</div>
      </div>
      <div className={styles.ttField}>
        <div className={styles.ttFieldLabel}>Public Key</div>
        <div className={styles.ttFieldValue}>{user.public_key}</div>
      </div>
    </div>
  );
}

const DepositTooltipContent: React.FC<{ deposit: ICityL1Deposit }> = ({ deposit }) => {
  return (
    <div className={styles.ttContent}>
      <div className={styles.ttField}>
        <div className={styles.ttFieldLabel}>Checkpoint ID</div>
        <div className={styles.ttFieldValue}>{deposit.checkpoint_id}</div>
      </div>
      <div className={styles.ttField}>
        <div className={styles.ttFieldLabel}>Deposit ID</div>
        <div className={styles.ttFieldValue}>{deposit.deposit_id}</div>
      </div>
      <div className={styles.ttField}>
        <div className={styles.ttFieldLabel}>Amount</div>
        <div className={styles.ttFieldValue}>{cityFeltSatsToDoge(deposit.value)} DOGE</div>
      </div>
      <div className={styles.ttField}>
        <div className={styles.ttFieldLabel}>Public Key</div>
        <div className={styles.ttFieldValue}>{deposit.public_key}</div>
      </div>
      <div className={styles.ttField}>
        <div className={styles.ttFieldLabel}>L1 TXID</div>
        <div className={styles.ttFieldValue}>{deposit.txid}</div>
      </div>
    </div>
  );
}

export {
  UserTooltipContent,
  DepositTooltipContent,
}