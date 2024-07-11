import React, { FC, createRef, useEffect } from "react";
import { IBlokiesInputOpts, renderBlokiesIcon } from "./blokies";
interface IBlockiesIconProps extends IBlokiesInputOpts {
  className?: string;
}
export const BlokiesIcon: FC<IBlockiesIconProps> = (props: IBlockiesIconProps) => {
  const ref = createRef<HTMLCanvasElement>();
  useEffect(()=>{
    if (ref.current) {
      renderBlokiesIcon(props, ref.current);
    }
  },[props, ref.current]);
  return (
    <canvas ref={ref} className={props.className} />
  );
};

export type {
  IBlockiesIconProps,
}