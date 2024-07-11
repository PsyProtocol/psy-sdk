import React from "react";
import styles from "./TransactionStatus.module.scss";
import { TSVGIconProps } from "../icons/types";

const TransactionStatusIcon: React.FC<TSVGIconProps & { loading: boolean }> = ({
  loading,
  className,
  size,
  ...props
}) => (
  <svg
    {...props}
    version="1.1"
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 100 100"
    width={size}
    height={size}
    className={
      (loading ? styles.progress : styles.ready) +
      (className ? " " + className : "")
    }
  >
    <circle
      className={styles.circle}
      cx="50"
      cy="50"
      r="46"
      fill="transparent"
    />
    <polyline
      className={styles.tick}
      points="25,55 45,70 75,33"
      fill="transparent"
    />
  </svg>
);
export default TransactionStatusIcon;
