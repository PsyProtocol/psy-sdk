import React from "react";
import { Tooltip } from "@mantine/core";
import { StyledWalletActionButton, Icon, Label } from "./WalletActionButton.styles";
interface IWalletActionButtonProps {
    icon: React.ReactNode;
    label: string;
    disabledText?: string;
    onClick: () => void;
}

const WalletActionButton: React.FC<IWalletActionButtonProps> = ({ disabledText, icon, label, onClick }) => {
    return disabledText ? (
        <Tooltip label={disabledText} position="top">
            <StyledWalletActionButton
                onClick={() => 0}
                disabled={true}
            >
                <Icon>{icon}</Icon>
                <Label>{label}</Label>
            </StyledWalletActionButton>
        </Tooltip>
    ) : (
        <StyledWalletActionButton onClick={onClick}>
            <Icon>{icon}</Icon>
            <Label>{label}</Label>
        </StyledWalletActionButton>
    );
};

export { WalletActionButton };
