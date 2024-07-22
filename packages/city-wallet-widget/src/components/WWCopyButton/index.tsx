import React from "react";
import { ActionIcon, CopyButton, Tooltip, rem } from "@mantine/core";
import { IconCheck, IconCopy } from "@tabler/icons-react";
interface IWWCopyButtonProps {
  value: string;
  className?:string;
  timeout?: number;
}
const WWCopyButton: React.FC<IWWCopyButtonProps> = ({value, className, timeout}) => {
  return (
    <CopyButton value={value} timeout={typeof timeout === 'number' ? timeout : 2000} >
      {({ copied, copy }) => (
        <Tooltip label={copied ? 'Copied' : 'Copy'} withArrow position="right">
          <ActionIcon color={copied ? 'teal' : 'gray'} variant="subtle" onClick={copy} className={className}>
            {copied ? (
              <IconCheck style={{ width: rem(16) }} />
            ) : (
              <IconCopy style={{ width: rem(16) }} />
            )}
          </ActionIcon>
        </Tooltip>
      )}
    </CopyButton>
  );
}

export {
  WWCopyButton,
}