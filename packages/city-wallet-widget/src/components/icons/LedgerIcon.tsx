import * as React from "react"
import { TSVGIconProps } from "./types"

const ASPECT_RATIO = 148/128;
const LedgerIcon: React.FC<TSVGIconProps> = (props) => (
  <svg
    fill="none"
    viewBox="0 0 148 128"
    width={props.size}
    height={props.size?(props.size/ASPECT_RATIO):undefined}
    {...props}
  >
  <path
    fill={props.color||"currentColor"}
    d="M0 91.655V128h55.308v-8.06H8.058V91.655H0Zm138.98 0v28.285H91.731v8.058h55.308V91.655h-8.059Zm-83.592-55.31v55.308h36.343v-7.269H63.446V36.345h-8.058ZM0 0v36.345h8.058V8.058h47.25V0H0Zm91.731 0v8.058h47.249v28.287h8.059V0H91.731Z"
  />
  </svg>
)
export default LedgerIcon;