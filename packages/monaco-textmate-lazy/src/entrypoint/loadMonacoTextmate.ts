import {
  createOnigScanner,
  createOnigString,
  loadWASM,
} from "vscode-oniguruma";
import * as monaco from 'monaco-editor';
import { getCurrentRawTheme, setMonacoTheme } from "./theme";
import { IMonacoGlobalSetupConfig } from "../types";
import { SimpleLanguageInfoProvider, TextMateGrammar } from "./provider";

export type LanguageId = string;

export type LanguageInfo = {
  tokensProvider: monaco.languages.EncodedTokensProvider | null;
  configuration: monaco.languages.LanguageConfiguration | null;
};

export async function loadVSCodeOnigurumWASM(url: string) {
  // @ts-ignore
  const response = await fetch(url);
  const contentType = response.headers.get('content-type');
  if (contentType === 'application/wasm') {
    return response;
  }

  // Using the response directly only works if the server sets the MIME type 'application/wasm'.
  // Otherwise, a TypeError is thrown when using the streaming compiler.
  // We therefore use the non-streaming compiler :(.
  return await response.arrayBuffer();
}
async function registerLanguages(
  config: IMonacoGlobalSetupConfig,
  fetchLanguageInfo: (language: LanguageId) => Promise<LanguageInfo>
) {
  config.additionalLanguages?.forEach((language) => {
    monaco.languages.register({
      id: language.id,
      filenamePatterns: language.filenamePatterns,
    });
  });
  for (const extensionPoint of config.languages) {
    // Recall that the id is a short name like 'cpp' or 'java'.
    const { id: languageId } = extensionPoint;
    // monaco.languages.register(extensionPoint);

    // Lazy-load the tokens provider and configuration data.

    const { tokensProvider, configuration } = await fetchLanguageInfo(
      languageId
    );
    if (tokensProvider != null) {
      monaco.languages.setTokensProvider(languageId, tokensProvider);
    }

    if (configuration != null) {
      monaco.languages.setLanguageConfiguration(languageId, configuration);
    }
  }
}



const fetchGrammar = async (scopeName: string, url: string): Promise<TextMateGrammar> => {
  const response = await fetch(url);
  const grammar = await response.text();
  const type = url.endsWith(".json") ? "json" : "plist";
  return { type, grammar };
};
let provider: SimpleLanguageInfoProvider | null = null;

export default async function setupVSCodeTextmate(config: IMonacoGlobalSetupConfig) {
  const data: ArrayBuffer | Response = await loadVSCodeOnigurumWASM(config.onigurumaWasmUrl);
  loadWASM(data);
  const onigLib = Promise.resolve({
    createOnigScanner,
    createOnigString,
  });

  const theme = getCurrentRawTheme();

  const p = new SimpleLanguageInfoProvider({
    grammars: config.grammars,
    fetchGrammar: (scopeName: string) => {
      const url = config.grammars[scopeName]?.url;
      if(url){
        return fetchGrammar(scopeName, url);
      }else{
        throw new Error(`No grammar found for scopeName: ${scopeName}`);
      }
    },
    setupConfig: config,
    onigLib,
    theme,
    // colorMap,
  });
  await registerLanguages(config, (id) => p.fetchLanguageInfo(id));
  provider = p;
  return p;
}
export {
	provider,
}
