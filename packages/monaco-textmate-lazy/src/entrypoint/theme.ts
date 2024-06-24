import * as monaco from "monaco-editor";
import { IRawTheme } from "vscode-textmate";
import camelcase from "camelcase";

const themes: Record<string, monaco.editor.IStandaloneThemeData> = {};

let currentTheme = "";

export function getCurrentTheme() {
  return {
    name: currentTheme,
    themeData: themes[currentTheme],
  };
}

let initDefaultColors = true;

export async function setMonacoTheme(name: string, url: string) {
  console.log("setting theme",name,url);
  let theme = themes[name];
  if (!theme) {
    theme = JSON.parse(await (await fetch(url)).text());
    themes[name] = theme;
    monaco.editor.defineTheme(name, theme);
  }
  currentTheme = name;

  const prefix = "--oas-";

  let style = document.getElementById("-injected-colors-");

  if (!style) {
    style = document.createElement("style");
    style.id = initDefaultColors
      ? "-default-injected-colors-"
      : "-injected-colors-";
    document.getElementsByTagName("head")[0].appendChild(style);
    initDefaultColors = false;
  }

  let res = "body > div {";

  Object.keys(theme.colors).forEach((v) => {
    res += `${prefix}${camelcase(v.split("."))}: ${
      theme.colors[v] || "rgba(0, 0, 0, 0)"
    };`;
  });

  res += "}";

  style.innerHTML = res;

  monaco.editor.setTheme(name);
}

export function getCurrentRawTheme() {
  const { name, themeData } = getCurrentTheme();
  if (themeData) {
    const rawTheme: IRawTheme = {
      name,
      settings: [],
    };
    themeData.rules.forEach((rule) => {
      const { token, ...restSetting } = rule;
      const setting: any = { settings: restSetting };
      if (rule.token) {
        setting.scope = rule.token;
      }
      rawTheme.settings.push(setting);
    });
    return rawTheme;
  } else {
    return undefined;
  }
}
