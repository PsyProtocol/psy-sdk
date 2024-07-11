import React, { useEffect, useRef, useState } from "react";
import styles from "./REPLInput.module.scss";
import classNames from "classnames";
import { ReplCommandInputProcessor } from "../../cmd/ui";
import { ICityREPLCommandDef } from "../../cmd/defs";
import { CityRPCCommandRequest } from "@qstudio/city-sdk";
interface IREPLInputProps {
  className?: string;
  onSubmit: (command: string, args: string[], request: CityRPCCommandRequest) => Promise<any>;
  onCmdError: (error: string) => void;
  onCmdInfo: (info: ICityREPLCommandDef | null) => void;
  onReplCommandInputProcessor: (processor: ReplCommandInputProcessor) => void;

}

const REPLInput: React.FC<IREPLInputProps> = ({ onSubmit, onCmdError, onCmdInfo, className, onReplCommandInputProcessor }) => {
  const inputRef = useRef<HTMLInputElement>(null);
  const autoCompleteRef = useRef<HTMLInputElement>(null);
  const [processor, setProcessor] = useState<ReplCommandInputProcessor>();

  useEffect(()=>{
    if(inputRef.current && autoCompleteRef.current&&!processor){
      console.log("creating new processor");
      const newProcessor = new ReplCommandInputProcessor({
        input: inputRef.current,
        autoComplete: autoCompleteRef.current,
        onSetCommandInfo: onCmdInfo,
        onSubmitHandler: onSubmit,
        setCommandError:onCmdError,
      });
      onReplCommandInputProcessor(newProcessor);
      setProcessor(newProcessor);
    }

    return ()=>{
      if(processor){
        processor.dispose();
        setProcessor(undefined);
      }
    };
  },[inputRef, autoCompleteRef, onCmdError, onCmdInfo, onSubmit])
  

  return (
    <div className={classNames(styles.replInputCon, className)}>
      <input
        ref={autoCompleteRef}
        className={styles.replInputAutoComplete}
        spellCheck={false}
        autoComplete="off"
      />
      <input
        ref={inputRef}
        className={styles.replInput}
        spellCheck={false}
        autoComplete="off"
      />
    </div>
  );
}

export {
  REPLInput,
}