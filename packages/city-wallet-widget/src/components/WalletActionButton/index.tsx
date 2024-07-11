import React from "react";
import { Tooltip, UnstyledButton } from "@mantine/core";
import styles from "./WalletActionButton.module.scss";
interface IWalletActionButtonProps {
  icon: React.ReactNode;
  label: string;
  disabledText?: string;
  onClick: () => void;
}

const WalletActionButton: React.FC<IWalletActionButtonProps> = ({
  disabledText,
  icon,
  label,
  onClick,
}) => {
  return disabledText ? (
    <Tooltip label={disabledText} position="top">
    <UnstyledButton
      onClick={() => 0}
      disabled={true}
      style={{ width: "100%" }}
      className={styles.walletActionButton}
    >
      <div className={styles.icon}>{icon}</div>
      <div className={styles.label}>{label}</div>
    </UnstyledButton>
  </Tooltip>
  ) : (
    
    <UnstyledButton
      onClick={onClick}
      style={{ width: "100%" }}
      className={styles.walletActionButton}
    >
      <div className={styles.icon}>{icon}</div>
      <div className={styles.label}>{label}</div>
    </UnstyledButton>
  );
};

export { WalletActionButton };
