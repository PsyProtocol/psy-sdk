import * as React from "react"
import { TSVGIconProps } from "./types"

const L1TransactionIcon: React.FC<Omit<TSVGIconProps, "size"> & { size: number | string }> = (props) => (
  <svg
    fill="none"
    viewBox="0 0 25 25"
    width={props.size}
    height={props.size}
    {...props}
  >

    <path
      fill={props.color || "currentColor"}
      fillRule="evenodd"
      d="m19.423.113 4.616 2.688-4.616 2.688V3.267H7.09c-1.582 0-2.704.335-3.503.809a4.162 4.162 0 0 0-1.617 1.73 5.233 5.233 0 0 0-.512 1.762 5.25 5.25 0 0 0-.029.774v.008l-.46.039-.459.038V8.42l-.002-.017A2.792 2.792 0 0 1 .5 8.148c0-.167.008-.403.04-.685a6.17 6.17 0 0 1 .606-2.08 5.09 5.09 0 0 1 1.972-2.11c.967-.574 2.258-.937 3.971-.937h12.334V.113Zm-13.846 24L.962 21.425l4.615-2.688v2.223H17.91c1.582 0 2.704-.335 3.503-.81a4.16 4.16 0 0 0 1.617-1.73 5.233 5.233 0 0 0 .512-1.761 5.246 5.246 0 0 0 .029-.774v-.008l.46-.039.459-.039v.008l.002.016.003.057a6.2 6.2 0 0 1-.035.884 6.169 6.169 0 0 1-.607 2.08 5.09 5.09 0 0 1-1.972 2.11c-.967.573-2.258.937-3.971.937H5.577v2.222Z"
      clipRule="evenodd"
    />
    <path
      fill={props.color || "currentColor"}
      d="M15.064 16.124v-5.675l.212.214-1.564.71v-1.078l2.082-.873h.333v6.702h-1.063Zm-1.456 0v-.969h3.71v.969h-3.71Zm-5.954 0V8.08h1.325v6.837h3.261v1.206H7.654Z"
    />
  </svg>
)
export default L1TransactionIcon;