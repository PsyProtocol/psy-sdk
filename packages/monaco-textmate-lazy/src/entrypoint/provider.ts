import * as monaco from "monaco-editor";
import type {
  IGrammar,
  IRawGrammar,
  IRawTheme,
  IOnigLib,
  StateStack,
} from "vscode-textmate";

import { INITIAL, Registry, parseRawGrammar } from "vscode-textmate";

// @ts-ignore
import { Color } from "monaco-editor/esm/vs/base/common/color.js";
import { IMonacoGlobalSetupConfig } from "../types";
import { LanguageInfo } from "./loadMonacoTextmate";

/** String identifier for a "scope name" such as 'source.cpp' or 'source.java'. */
export type ScopeName = string;

export type TextMateGrammar = {
  type: "json" | "plist";
  grammar: string;
};

export type SimpleLanguageInfoProviderConfig = {
  // Key is a ScopeName.
  grammars: { [scopeName: string]: ScopeNameInfo };

  fetchGrammar: (scopeName: ScopeName) => Promise<TextMateGrammar>;

  // This must be available synchronously to the SimpleLanguageInfoProvider
  // constructor, so the user is responsible for fetching the theme data rather
  // than SimpleLanguageInfoProvider.
  theme?: IRawTheme;
  colorMap?: string[];
  setupConfig: IMonacoGlobalSetupConfig;

  onigLib: Promise<IOnigLib>;
};

export interface ScopeNameInfo {
  /**
   * If set, this is the id of an ILanguageExtensionPoint. This establishes the
   * mapping from a MonacoLanguage to a TextMate grammar.
   */
  language?: string;

  /**
   * Scopes that are injected *into* this scope. For example, the
   * `text.html.markdown` scope likely has a number of injections to support
   * fenced code blocks.
   */
  injections?: string[];
}

export function generateTokensCSSForColorMap(colorMap: any[]) {
  let rules = [];
  for (let i = 1, len = colorMap.length; i < len; i++) {
    let color = colorMap[i];
    rules[i] = `.mtk${i} { color: ${color}; }`;
  }
  rules.push(".mtki { font-style: italic; }");
  rules.push(".mtkb { font-weight: bold; }");
  rules.push(
    ".mtku { text-decoration: underline; text-underline-position: under; }"
  );
  return rules.join("\n");
}

/**
 * Basic provider to implement the fetchLanguageInfo() function needed to
 * power registerLanguages(). It is designed to fetch all resources
 * asynchronously based on a simple layout of static resources on the server.
 */
export class SimpleLanguageInfoProvider {
  private registry: Registry;
  private tokensProviderCache: TokensProviderCache;
  private isInjectedCss: HTMLElement | null = null;

  constructor(private config: SimpleLanguageInfoProviderConfig) {
    const { grammars, fetchGrammar, theme, onigLib } = config;

    this.registry = new Registry({
      onigLib,

      async loadGrammar(scopeName: ScopeName): Promise<IRawGrammar | null> {
        const scopeNameInfo = grammars[scopeName];
        if (scopeNameInfo == null) {
          console.log(`No grammar found for scope ${scopeName}`);
          return null;
        }

        const { type, grammar } = await fetchGrammar(scopeName);
        // If this is a JSON grammar, filePath must be specified with a `.json`
        // file extension or else parseRawGrammar() will assume it is a PLIST
        // grammar.
        const iRawGrammar = parseRawGrammar(grammar, `example.${type}`);
        return iRawGrammar;
      },
      getInjections(scopeName: ScopeName): string[] | undefined {
        const grammar = grammars[scopeName];
        return grammar ? grammar.injections : undefined;
      },
      theme,
    });

    this.tokensProviderCache = new TokensProviderCache(this.registry);
  }

  /**
   * Be sure this is done after Monaco injects its default styles so that the
   * injected CSS overrides the defaults.
   */
  injectCSS() {
    if (!this.isInjectedCss) {
      const cssColors = this.registry.getColorMap();
      const colorMap = cssColors.map(Color.Format.CSS.parseHex);

      const css = generateTokensCSSForColorMap(colorMap);
      const style = createStyleElementForColorsCSS(css);
      this.isInjectedCss = style;
    }
  }

  removeCSS() {
    if (this.isInjectedCss) {
      document.head.removeChild(this.isInjectedCss);
      this.isInjectedCss = null;
    }
  }

  switchTheme(rawTheme: IRawTheme) {
    this.registry.setTheme(rawTheme);
    this.removeCSS();
    this.injectCSS();
  }

  async fetchLanguageInfo(language: string): Promise<LanguageInfo> {
    const tokensProvider = await this.getTokensProviderForLanguage(language);
    return { tokensProvider, configuration: this.config.setupConfig.languageConfiguration[language] };
  }

  private getTokensProviderForLanguage(
    language: string
  ): Promise<monaco.languages.EncodedTokensProvider | null> {
    const scopeName = this.getScopeNameForLanguage(language);
    if (scopeName == null) {
      return Promise.resolve(null);
    }
    const encodedLanguageId = monaco.languages.getEncodedLanguageId(language);
    // Ensure the result of createEncodedTokensProvider() is resolved before
    // setting the language configuration.
    return this.tokensProviderCache.createEncodedTokensProvider(
      scopeName,
      encodedLanguageId
    );
  }

  private getScopeNameForLanguage(language: string): string | null {
    for (const [scopeName, grammar] of Object.entries(this.config.grammars)) {
      if (grammar.language === language) {
        return scopeName;
      }
    }
    return null;
  }
}

class TokensProviderCache {
  private scopeNameToGrammar: Map<string, Promise<IGrammar>> = new Map();

  constructor(private registry: Registry) {}

  async createEncodedTokensProvider(
    scopeName: string,
    encodedLanguageId: number
  ): Promise<monaco.languages.EncodedTokensProvider> {
    const grammar = await this.getGrammar(scopeName, encodedLanguageId);

    return {
      getInitialState() {
        return INITIAL;
      },

      tokenizeEncoded(
        line: string,
        state: monaco.languages.IState
      ): monaco.languages.IEncodedLineTokens {
        const tokenizeLineResult2 = grammar.tokenizeLine2(
          line,
          state as StateStack
        );
        const { tokens, ruleStack } = tokenizeLineResult2;

        return { tokens, endState: ruleStack };
      },
    };
  }

  getGrammar(scopeName: string, encodedLanguageId: number): Promise<IGrammar> {
    const grammar = this.scopeNameToGrammar.get(scopeName);
    if (grammar != null) {
      return grammar;
    }

    // This is defined in vscode-textmate and has optional embeddedLanguages
    // and tokenTypes fields that might be useful/necessary to take advantage of
    // at some point.
    const grammarConfiguration = {};
    // We use loadGrammarWithConfiguration() rather than loadGrammar() because
    // we discovered that if the numeric LanguageId is not specified, then it
    // does not get encoded in the TokenMetadata.
    //
    // Failure to do so means that the LanguageId cannot be read back later,
    // which can cause other Monaco features, such as "Toggle Line Comment",
    // to fail.

    const promise = this.registry
      .loadGrammarWithConfiguration(
        scopeName,
        encodedLanguageId,
        grammarConfiguration
      )
      .then((grammar: IGrammar | null) => {
        if (grammar) {
          return grammar;
        } else {
          throw Error(`failed to load grammar for ${scopeName}`);
        }
      });
    this.scopeNameToGrammar.set(scopeName, promise);
    return promise;
  }
}

function createStyleElementForColorsCSS(css: string): HTMLStyleElement {
  // We want to ensure that our <style> element appears after Monaco's so that
  // we can override some styles it inserted for the default theme.
  const style = document.createElement("style");

  // We expect the styles we need to override to be in an element with the class
  // name 'monaco-colors' based on:
  // https://github.com/microsoft/vscode/blob/f78d84606cd16d75549c82c68888de91d8bdec9f/src/vs/editor/standalone/browser/standaloneThemeServiceImpl.ts#L206-L214
  const monacoColors = document.getElementsByClassName("monaco-colors")[0];
  if (monacoColors) {
    monacoColors.parentElement?.insertBefore(style, monacoColors.nextSibling);
  } else {
    // Though if we cannot find it, just append to <head>.
    let { head } = document;
    if (head == null) {
      head = document.getElementsByTagName("head")[0];
    }
    head?.appendChild(style);
  }

  style.innerHTML = css;

  const itv = setInterval(() => {
    if (!style.parentElement) {
      createStyleElementForColorsCSS(css);
      clearInterval(itv);
    }
  }, 1000);

  return style;
}
