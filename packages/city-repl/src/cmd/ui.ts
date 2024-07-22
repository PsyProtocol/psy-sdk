import { CityRPCCommandRequest } from "@qstudio/city-sdk";
import { ICityREPLCommandDef } from "./defs";
import { ICommandRef, autoCompleteCommand, commandArgs, commandIndicies, getCommandByName, getCommandRequest, parseReplLine } from "./processor";

interface IReplaceCommandInputProcessorConfig {
  input: HTMLInputElement;
  autoComplete: HTMLInputElement;
  onSubmitHandler: (command: string, args: string[], request: CityRPCCommandRequest) => Promise<any>;
  onSetCommandInfo: (cmdDef: ICityREPLCommandDef | null) => any;
  setCommandError: (error: string) => any;
}
class ReplCommandInputProcessor {
  args: string[] = [];
  command: string = "";
  cmdDef: ICityREPLCommandDef | null = null;
  lastSentCmdDef: ICityREPLCommandDef | null = null;
  value: string = "";
  cmdAutoCompleteRefs: ICommandRef[] = commandIndicies;
  autoCompleteCommandIndex: number = -1;
  lastSentError: string = "";
  rndId = Math.floor(Math.random()*0x10000).toString(16).padStart(4, "0");


  input: HTMLInputElement;
  autoComplete: HTMLInputElement;
  onSubmitHandler: (command: string, args: string[], request: CityRPCCommandRequest) => Promise<any>;
  onSetCommandInfo: (cmdDef: ICityREPLCommandDef | null) => any;
  setCommandError: (error: string) => any;

  constructor({ input, autoComplete, onSetCommandInfo, onSubmitHandler, setCommandError }: IReplaceCommandInputProcessorConfig) {
    this.input = input;
    this.autoComplete = autoComplete;
    this.onSubmitHandler = onSubmitHandler;
    this.onSetCommandInfo = onSetCommandInfo;
    this.setCommandError = setCommandError;

    this.onKeyDown = this.onKeyDown.bind(this);
    this.onChange = this.onChange.bind(this);
    this.onClick = this.onClick.bind(this);
    this.onInput = this.onInput.bind(this);
    this.input.addEventListener("keydown", this.onKeyDown);
    this.input.addEventListener("input", this.onInput);
    this.input.addEventListener("change", this.onChange);
    this.input.addEventListener("click", this.onClick);
    this.input.addEventListener("mousedown", this.onClick);
  }
  sendCommandError(error: string) {
      this.lastSentError = error;
      this.setCommandError(error);
  }
  setAutoCompleteValue(value: string) {
    this.autoComplete.value = value;
  }
  setCommandInfo(cmdDef: ICityREPLCommandDef | null) {
    if (this.lastSentCmdDef !== cmdDef) {
      this.lastSentCmdDef = cmdDef;
      //this.cmdDef = cmdDef;
      this.onSetCommandInfo(cmdDef);
    }
  }
  setCommandInfoByName(command: string) {
    this.setCommandInfo(getCommandByName(command));
  }
  updateAutoComplete(newValue: string) {
    console.log("this.command", this.command, newValue, "newValue");
    if (newValue.indexOf(" ") === -1 && this.command.length < newValue.length) {
      this.command = "";
      this.args = [];
      this.cmdDef = null;
      this.value = "";

    }
    if (!newValue.length) {
      this.command = "";
      this.args = [];
      this.cmdDef = null;
      this.value = "";
      this.cmdAutoCompleteRefs = commandIndicies;
      this.setAutoCompleteValue("");
      this.setCommandInfo(null);
      this.autoCompleteCommandIndex = -1;
    } else if (this.cmdDef && newValue.startsWith(this.command)) {
      const { args, command } = parseReplLine(newValue);
      this.command = command;
      this.args = args;
      const argText = this.cmdDef.arguments.slice(args.length).map(x => `<${(x.settings.tags || [])[0] || "?"}${x.settings.required ? "" : (" (optional)")}>`).join(" ");

      this.setAutoCompleteValue(newValue.trim() + " " + argText);
      if (newValue.startsWith(this.command + " ")) {
        this.autoCompleteCommandIndex = -1;
        this.cmdAutoCompleteRefs = [];
      } else {
        this.cmdAutoCompleteRefs = autoCompleteCommand(this.command);
        this.autoCompleteCommandIndex = this.cmdAutoCompleteRefs.length - 1;
        if (this.autoCompleteCommandIndex >= 0) {
          this.setCommandInfoByName(this.cmdAutoCompleteRefs[this.autoCompleteCommandIndex].command);
        }
      }
    } else {
      const { args, command } = parseReplLine(newValue);
      this.command = command;
      this.args = args;
      this.cmdDef = getCommandByName(this.command);
      this.setCommandInfo(this.cmdDef);
      if (!this.cmdDef || newValue.length === 1) {
        this.cmdAutoCompleteRefs = autoCompleteCommand(this.command);
        this.autoCompleteCommandIndex = this.cmdAutoCompleteRefs.length - 1;
      }
      if (!this.cmdDef) {

        if (this.autoCompleteCommandIndex >= 0) {
          this.setAutoCompleteValue(this.cmdAutoCompleteRefs[this.autoCompleteCommandIndex].command);
          this.setCommandInfoByName(this.cmdAutoCompleteRefs[this.autoCompleteCommandIndex].command);
        } else {
          this.setCommandInfo(null);
          this.setAutoCompleteValue("");
        }
      } else {
        /*
        this.autoCompleteCommandIndex = -1;
        this.cmdAutoCompleteRefs = [];
        */
        const argText = this.cmdDef.arguments.slice(args.length).map(x => `<${(x.settings.tags || [])[0] || "?"}${x.settings.required ? "" : (" (optional)")}>`).join(" ");
        this.setAutoCompleteValue(newValue.trim() + " " + argText);
        this.setCommandInfo(this.cmdDef);

      }
    }
  }
  onChange(event: Event) {
    const newValue = (event.target as HTMLInputElement).value;
    if (!newValue.length) {
      this.command = "";
      this.args = [];
      this.cmdDef = null;
      this.value = "";
      this.cmdAutoCompleteRefs = commandIndicies;
      this.setAutoCompleteValue("");
      this.sendCommandError("");
      this.autoCompleteCommandIndex = -1;

      this.setCommandInfo(null);
    } else {
      this.updateAutoComplete(newValue);
    }

    this.value = newValue;
  }
  submitCommand() {
    if (this.input.disabled) {
      return;
    }
    const val = this.input.value.trim();
    if (!val.length) {
      return;
    }
    this.sendCommandError("");
    const { args, command } = parseReplLine(val);
    this.command = command;
    this.args = args;
    this.cmdDef = getCommandByName(this.command);
    if (!this.cmdDef) {
      this.sendCommandError(`Unknown command: ${this.command}`);
      return;
    }
    this.input.disabled = true;
    try {
      const req = getCommandRequest(this.cmdDef, this.args);
      this.onSubmitHandler(this.command, this.args, req).then((result) => {
        console.log("Command Result: ", result);
        this.input.disabled = false;
        this.input.focus();
      }).catch((e) => {

        this.input.disabled = false;
        this.input.focus();
        this.sendCommandError(e + "");
      });
    } catch (e) {
      this.sendCommandError(e + "");
      this.input.disabled = false;
      this.input.focus();
      return;
    }
  }
  onKeyDown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      this.submitCommand();
    } else if (this.cmdAutoCompleteRefs.length) {
      const len = this.cmdAutoCompleteRefs.length;
      if (event.key === "ArrowUp") {
        event.preventDefault();
        event.stopPropagation();
        this.autoCompleteCommandIndex = (this.autoCompleteCommandIndex - 1 + len) % len;
        this.setAutoCompleteValue((this.cmdAutoCompleteRefs[this.autoCompleteCommandIndex].command + " " + commandArgs[this.cmdAutoCompleteRefs[this.autoCompleteCommandIndex].index].join(" ")).trim());
        this.setCommandInfoByName(this.cmdAutoCompleteRefs[this.autoCompleteCommandIndex].command);
      } else if (event.key === "ArrowDown") {
        event.preventDefault();
        event.stopPropagation();
        this.autoCompleteCommandIndex = (this.autoCompleteCommandIndex + 1) % len;
        this.setAutoCompleteValue((this.cmdAutoCompleteRefs[this.autoCompleteCommandIndex].command + " " + commandArgs[this.cmdAutoCompleteRefs[this.autoCompleteCommandIndex].index].join(" ")).trim());
        this.setCommandInfoByName(this.cmdAutoCompleteRefs[this.autoCompleteCommandIndex].command);
      } else if ((event.key === "Tab" || event.key === "ArrowRight")) {
        event.preventDefault();
        event.stopPropagation();
        if (!this.cmdDef || this.input.value.indexOf(" ") < 0) {
          const autoValue = this.autoComplete.value.split(" ")[0];
          if (autoValue === this.input.value && autoValue.length) {
            this.value = this.input.value = this.input.value.trim() + " ";
            this.cmdDef = getCommandByName(autoValue);
          } else {
            this.value = this.input.value = autoValue;
          }
        } else {
          this.value = this.input.value = this.input.value.trim() + " ";

        }
        this.updateAutoComplete(this.value);
      }
    }
  }
  onInput(event: Event) {
    this.onChange(event);
  }
  onClick(event: MouseEvent) {
    // get the caret position
    const caret = this.input.selectionStart;

  }
  dispose() {
    this.input.removeEventListener("keydown", this.onKeyDown);
    this.input.removeEventListener("input", this.onInput);
    this.input.removeEventListener("change", this.onChange);
    this.input.removeEventListener("click", this.onClick);
    this.input.removeEventListener("mousedown", this.onClick);
  }
}

export type {
  IReplaceCommandInputProcessorConfig,
}

export {
  ReplCommandInputProcessor,
}