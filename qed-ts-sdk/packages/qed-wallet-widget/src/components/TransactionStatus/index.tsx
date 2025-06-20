import React from "react";
import { StatusSVG } from "./TransactionStatus.styles";
import { TSVGIconProps } from "../icons/types";

const TransactionStatusIcon: React.FC<TSVGIconProps & { loading: boolean }> = ({
    loading,
    className,
    size,
    ...props
}) => (
    <StatusSVG
        {...props}
        version="1.1"
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 100 100"
        width={size}
        height={size}
        loading={loading}
        className={className}
    >
        <circle className="circle" cx="50" cy="50" r="46" fill="transparent" />
        <polyline className="tick" points="25,55 45,70 75,33" fill="transparent" />
    </StatusSVG>
);
export default TransactionStatusIcon;
