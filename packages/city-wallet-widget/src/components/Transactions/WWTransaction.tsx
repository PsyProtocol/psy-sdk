import React, { useState } from "react";
import { IGetTXResponse } from "doge-sdk";
import {
  ActionIcon,
  Button,
  Card,
  Group,
  Menu,
  Text,
  UnstyledButton,
  rem,
} from "@mantine/core";
import { IconDots, IconExternalLink } from "@tabler/icons-react";
import styles from "./Transactions.module.scss";
import TransactionStatusIcon from "../TransactionStatus";
import classNames from 'classnames';
const TransactionValue = ({ value }: { value: number }) => {
  return (
    <div className={styles.txValue}>
      <span className={classNames(styles.value, {
        [styles.valueNegative]: value < 0,
        [styles.valuePositive]: value > 0
      })}>
        {value > 0 ? "+" : ""}{(value/100_000_000).toLocaleString()}
      </span>
      <span> DOGE</span>
    </div>
  );
}
interface IWalletTransactionProps extends IGetTXResponse {
  selfAddress: string;
  url: string;
}
function calculateTransactionAmount(selfAddress: string, resp: IGetTXResponse) {
  const spentValue = resp.vin.filter(x => x.prevout.scriptpubkey_address === selfAddress).reduce((acc, x) => acc + x.prevout.value, 0);
  const receivedValue = resp.vout.filter(x => x.scriptpubkey_address === selfAddress).reduce((acc, x) => acc + x.value, 0);

  return receivedValue - spentValue;
}
const WWTransaction: React.FC<IWalletTransactionProps> = (props: IWalletTransactionProps) => {
  const [expanded, setExpanded] = useState(false);
  const {
    url,
    selfAddress,
    txid,
    fee,
    weight,
    vin,
    vout,
    status,
  } = props;

  return (
    <UnstyledButton
      onClick={() => {
        setExpanded(!expanded);
      }}
      className={styles.walletTransactionContainer}
    >
      <Card
        withBorder
        shadow="sm"
        radius="md"
        className={styles.walletTransaction}
      >
        <Card.Section withBorder inheritPadding py="xs">
          <div className={styles.cardTop}>
            <Text fw={500} className={styles.txid}>
              {txid}
            </Text>
            <ActionIcon
              variant="subtle"
              color="gray"
              className={styles.actionIcon}
              component="a"
              href={url}
              target="_blank"
            >
              <IconExternalLink style={{ width: rem(16), height: rem(16) }} />
            </ActionIcon>
          </div>
        </Card.Section>

        <div className={styles.txCoreSummary}>            <TransactionStatusIcon loading={!status.confirmed} size={28} />

          <TransactionValue value={calculateTransactionAmount(selfAddress, props)} />
        </div>

        
      </Card>
    </UnstyledButton>
  );
};

export { WWTransaction };
