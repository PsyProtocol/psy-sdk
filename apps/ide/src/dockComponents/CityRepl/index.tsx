import { useEffect, useMemo, useRef, useState } from 'react';
import styles from './CityRepl.module.scss';
import { IDEContext } from '../../utils/ideContext';
import { Fieldset, NumberInput, Textarea, Button, Group } from '@mantine/core';
import { ICitySynthBlockConfig, ICitySynthBlockResult, synthPlanner } from '@qstudio/city-block';
import { genCodeForSVG } from '@qstudio/qsvg';
import copy from 'copy-to-clipboard';
import {CityREPL, REPLInput} from "@qstudio/city-repl"
import { CityRPCProvider, CityRPCCommandProcessor } from '@qstudio/city-sdk';
interface ICityReplDockComponentProps {
  ctx: IDEContext;
}
const CityReplDockComponent: React.FC<ICityReplDockComponentProps> = ({ ctx }) => {
  const rpcUrl = "http://localhost:3000";
  const rpcProvider = useMemo(()=>{
    return new CityRPCProvider(rpcUrl);
  },[rpcUrl]);
  const cmdProcessor = useMemo(()=>{
    return new CityRPCCommandProcessor(rpcProvider);
  },[rpcProvider]);
  return (
    <div className={styles.cityReplDockPage}>
      <CityREPL cmdProcessor={cmdProcessor}  />
    </div>
  )
};

export default CityReplDockComponent;