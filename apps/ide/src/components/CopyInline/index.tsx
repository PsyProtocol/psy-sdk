import React from "react";
import { ActionIcon, CopyButton, Tooltip, rem } from "@mantine/core";
import { IconCheck, IconCopy } from "@tabler/icons-react";
import { TOOLTIP_COLOR } from "../../constants/style";
import styles from './CopyInline.module.scss';
import classNames from "classnames";

interface ICopyInlineProps {
  label: React.ReactNode;
  children: React.ReactNode;
  value: string;
  className?:string;
  timeout?: number;
}
const CopyInline: React.FC<ICopyInlineProps> = ({value, label, children, className, timeout}) => {
  return (
    <CopyButton value={value} timeout={typeof timeout === 'number' ? timeout : 2000} >
      {({ copied, copy }) => (
        <Tooltip color={TOOLTIP_COLOR}
        events={{hover: true, focus: true, touch: false}}
        label={copied ? (
          <span className={styles.copyInlineCopied}><IconCheck color="#1f1" size="1em"/><span className={styles.copied}> Copied</span></span>
        ) : label} withArrow>
          <span className={classNames(styles.copyInline, className)} onClick={copy}>{children}</span>
        </Tooltip>
      )}
    </CopyButton>
  );
}

export {
  CopyInline,
}