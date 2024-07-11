import { IFieldValidatorReal, validate } from "@qstudio/one-schema";
import { REPLParseError } from "./cmdError";
import { ICityREPLCommandDef, commandDefs } from "./defs";

interface ICommandRef {
  command: string;
  index: number;

}
const commandIndicies: ICommandRef[] = commandDefs.map((cmd, i) => ([{ command: cmd.name, index: i }]).concat(cmd.aliases.map((alias) => ({ command: alias, index: i }))).flat()).flat().sort((a,b)=>{
  return a.command > b.command ? 1 : -1;
});
const commandArgs = commandDefs.map(cmdDef=>cmdDef.arguments.map(x=>`<${(x.settings.tags||[])[0]||"?"}${x.settings.required?"":(" (optional)")}>`));

const commandMap: Record<string, number> = {};
commandIndicies.forEach((c) => {
  commandMap[c.command] = c.index;
});

function getCommandByName(command: string) {
  const index = commandMap[command];
  if (typeof index !== 'number') {
    return null;
  }
  return commandDefs[index];
}

function findCommandIndexCandidates(command: string) {
  const candidates = commandIndicies.filter((c) => c.command.startsWith(command));
  return candidates;
}


function resolveString(value: string) {
  if (value.startsWith('"') && value.endsWith('"')) {
    return value;
  } else {
    return ('"' + value + '"');
  }
}
function validateArgument(arg: IFieldValidatorReal, value: string | undefined) {
  if (arg.settings.type === "string") {
    value = typeof value === 'undefined' ? value : resolveString(value);
  }
  let realValue: any = null;

  try {
    realValue = typeof value === 'undefined' ? value : JSON.parse(value);
  } catch (e: any) {
    throw new REPLParseError(((arg.settings.tags || [])[0] || "unknown"), value, ((e||{}).message)||"unknown error");
  }

  const { success, error } = validate(realValue, arg);

  if (!success) {
    throw new REPLParseError(((arg.settings.tags || [])[0] || "unknown"), value, error || "unknown error");
  }
  if(typeof arg.settings.defaultValue !== 'undefined' && typeof realValue === 'undefined'){
    return arg.settings.defaultValue;
  }
  return realValue;
}
function getCommandRequest(cmdDef: ICityREPLCommandDef, strArgs: string[]) {
  const args = cmdDef.arguments.map((arg, i) => {
    return validateArgument(arg, strArgs[i]);
  });
  return cmdDef.processCommand(args);
}


interface ICityReplParsedCommand {
  command: string;
  args: string[];
}
function parseReplLine(line: string): ICityReplParsedCommand {
  const normalizedLine = line.trim().replace(/\s+/g, " ").replace(/\s*,\s*/g,",").replace(/\[\s*/g,"[").replace(/\s*\]/g,"]");
  const split = normalizedLine.split(" ");
  const command = split[0];
  const args = split.slice(1);
  return { command, args };
}

function autoCompleteCommand(line: string): ICommandRef[] {
  const cmd = line.trim().split(" ")[0];
  console.log("acc",line);
  if(!cmd.length){
    return commandIndicies;
  }else{
    return findCommandIndexCandidates(cmd);
  }
}

interface IAutoCompleteResultBase {
  isCommand: boolean;
}
interface IAutoCompleteCommand {
  isCommand: true;
  options: ICommandRef[];
}
interface IAutoCompleteArgument {
  isCommand: false;
  options: string[];
}
function autoCompleteReplLine(line: string){
  const parsed = parseReplLine(line);
  if(parsed.args.length){
    return [];
  }else{
    return autoCompleteCommand(line);
  }
}

export type {
  ICityReplParsedCommand,
  ICommandRef,
}

export {
  getCommandByName,
  parseReplLine,
  getCommandRequest,
  autoCompleteReplLine,
  autoCompleteCommand,
  commandIndicies,
  commandArgs,
}
