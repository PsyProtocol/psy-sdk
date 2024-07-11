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
import { IconDots } from "@tabler/icons-react";
import styles from "./Transactions.module.scss";

interface IWalletTransactionProps extends IGetTXResponse {
  selfAddress: string;
  url: string;
}

const WalletTransaction: React.FC<IWalletTransactionProps> = ({
  url,
  selfAddress,
  txid,
  fee,
  weight,
  vin,
  vout,
  status,
}: IWalletTransactionProps) => {
  const [expanded, setExpanded] = useState(false);
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
              <IconDots style={{ width: rem(16), height: rem(16) }} />
            </ActionIcon>
          </div>
        </Card.Section>

        <Text mt="sm" c="dimmed" size="sm">
          <Text span inherit c="var(--mantine-color-anchor)">
            200+ images s
          </Text>{" "}
          since last visit, review them to select which one should be added to
          your gallery
        </Text>

        {expanded ? <Card.Section mt="sm">Tx</Card.Section> : null}
      </Card>
    </UnstyledButton>
  );
};

export { WalletTransaction };
