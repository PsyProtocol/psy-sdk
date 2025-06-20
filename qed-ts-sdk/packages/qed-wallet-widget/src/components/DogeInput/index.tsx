import React from "react";
import { NumberInput, NumberInputProps } from "@mantine/core";
import { decimalToSats, satsToDecimal } from "../../utils/sats";

interface IDogeInputProps extends Omit<NumberInputProps, "value" | "onChange"> {
    value: number;
    onChange: (value: number) => void;
    useSats?: boolean;
}

const DogeInput: React.FC<IDogeInputProps> = ({ useSats, value, onChange, min, ...props }) => {
    return (
        <NumberInput
            value={useSats ? satsToDecimal(value) : value}
            min={typeof min === "number" ? min : 0}
            decimalScale={8}
            rightSection={
                <span style={{ fontSize: "10px", marginRight: "20px", userSelect: "none", cursor: "default" }}>
                    DOGE
                </span>
            }
            onChange={(v) => {
                const num = typeof v === "number" ? v : parseFloat(v);
                onChange(useSats ? decimalToSats(num) : num);
            }}
            {...props}
        />
    );
};

export { DogeInput };
