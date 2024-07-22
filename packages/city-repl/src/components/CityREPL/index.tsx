import React, { useEffect, useState } from "react";
import classNames from "classnames";
import { ICityRPCCommandRequestProcessor } from "@qstudio/city-sdk";
import styles from "./CityREPL.module.scss";
import { REPLInput } from "../REPLInput";
import { ICityREPLCommandDef } from "../../cmd/defs";
import { ReplCommandInputProcessor } from "../../cmd/ui";
/*

.cityRepl {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  position: relative;
  top:0;
  left:0;
  margin:0;
  padding:0px;
  .cityReplBody {
    width: 100%;
    height: 100%;
    overflow: auto;
    padding-top: 0px;
    flex-grow: 1;
    .cityReplBodyInner {
      display: block;
      white-space: pre;
      font-family: monospace;
      font-size: 12px;
      padding: 8px;
      background:rgb(104, 25, 104);
      color:#0f0;
    }

  }
  .cityReplInputCon {
    height: 120px;
    width: 100%;
    display: block;

  }

}

*/

const InputInfo: React.FC<{ replCommandInputProcessor?: ReplCommandInputProcessor, errorMessage?: string }> = ({ replCommandInputProcessor, errorMessage }) => {
  const [currentCommand, setCurrentCommand] =
    useState<ICityREPLCommandDef | null>(null);
  useEffect(() => {
    if (replCommandInputProcessor) {
      let rnd = replCommandInputProcessor.rndId;
      replCommandInputProcessor.setCommandInfo = (info) => {
        setCurrentCommand(info);
      };
    }
  }, [replCommandInputProcessor]);
  return (
    <div className={classNames(styles.inputInfo, currentCommand ? styles.withCurCmd : styles.noCmd)}>
      {errorMessage ? (
        <div className={styles.cityReplErrorMessage}>{errorMessage}</div>
      ) : null}
      {currentCommand ? (
        <div className={styles.cityReplCommandInfo}>
          {currentCommand.description}
        </div>
      ) : null}
    </div>
  );
};
interface ICityREPLProps {
  className?: string;
  cmdProcessor: ICityRPCCommandRequestProcessor;
}
const CityREPL: React.FC<ICityREPLProps> = ({ className, cmdProcessor }) => {
  const outputRef = React.useRef<HTMLDivElement>(null);
  const [replCommandInputProcessor, setReplCommandInputProcessor] = useState<ReplCommandInputProcessor>();
  console.log("repl render", replCommandInputProcessor ? replCommandInputProcessor.rndId : "none");
  const [errorMessage, setErrorMessage] = useState<string>();
  return (
    <div className={classNames(styles.cityRepl, className)}>
      <div className={styles.cityReplBody}>
        <div className={styles.cityReplBodyInner}>
          <div className={styles.cityReplOutput} ref={outputRef}></div>
        </div>
        <div className={styles.controlsCon}>
          <button
            className={styles.clearButton}
            onClick={() => {
              if (outputRef.current) {
                outputRef.current.innerText = "";
              }
            }}
          >
            Clear
          </button>
        </div>
      </div>
      <div className={styles.cityReplFooter}>
        <div className={styles.cityReplInputCon}>
          <REPLInput
            onReplCommandInputProcessor={(processor) => setReplCommandInputProcessor(processor)}
            onSubmit={async (command, args, request) => {
              if (outputRef.current) {
                outputRef.current.innerText += `\n> ${command} ${args.join(
                  " "
                )}\n`;
                outputRef.current.parentElement?.scrollTo(0, outputRef.current.scrollHeight);
                try {
                  const result = await cmdProcessor.processRequest(
                    request as any
                  );
                  outputRef.current.innerText += JSON.stringify(
                    result,
                    null,
                    2
                  );
                  outputRef.current.parentElement?.scrollTo(0, outputRef.current.scrollHeight);
                } catch (e) {
                  outputRef.current.innerText += `Error processing request: ${e + ""
                    }`;
                  outputRef.current.parentElement?.scrollTo(0, outputRef.current.scrollHeight);
                  console.error("Error processing request: ", e);
                }
              }
            }}
            onCmdError={(error) => {
              if(error!==errorMessage){
                setErrorMessage(error);
              }
            }}
            onCmdInfo={(info) => {
              //setCurrentCommand(info);
            }}
          />
        </div>
        <InputInfo replCommandInputProcessor={replCommandInputProcessor} errorMessage={errorMessage} />
      </div>
    </div>
  );
};


export {
  CityREPL,
}