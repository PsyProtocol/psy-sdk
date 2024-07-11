import * as React from "react"
import { TSVGIconProps } from "./types"

const ASPECT_RATIO = 29/41;
const TrezorIcon: React.FC<TSVGIconProps> = (props) => (
  <svg
    fill="none"
    viewBox="0 0 29 41"
    width={props.size?(props.size*ASPECT_RATIO):undefined}
    height={props.size}
    {...props}
  >
    <path
      fill={props.color||"currentColor"}
      d="M24.306 9.461C24.306 4.29 19.761 0 14.228 0 8.694 0 4.148 4.292 4.148 9.46v3.025H0v21.75l14.225 6.536 14.233-6.534V12.581H24.31l-.003-3.121Zm-15.02 0c0-2.438 2.175-4.389 4.942-4.389 2.767 0 4.94 1.951 4.94 4.389v3.024H9.287V9.461Zm13.44 21.264-8.502 3.904-8.499-3.901V17.655h17v13.07z"
    />
    <path
      fill={props.color||"currentColor"}
      d="M24.306 9.461C24.306 4.29 19.761 0 14.228 0 8.694 0 4.148 4.292 4.148 9.46v3.025H0v21.75l14.225 6.536 14.233-6.534V12.581H24.31l-.003-3.121-.001.001Zm-15.02 0c0-2.438 2.175-4.389 4.942-4.389 2.767 0 4.94 1.951 4.94 4.389v3.024H9.287l-.001-3.024Zm13.44 21.264-8.502 3.904-8.499-3.901V17.655h17l.001 13.07Z"
    />
  </svg>
);
export default TrezorIcon;
