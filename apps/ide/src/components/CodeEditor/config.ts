import {IMonacoGlobalSetupConfig, IndentAction} from '@qstudio/monaco-textmate-lazy';
const DEFAULT_MONACO_SETUP_CONFIG: IMonacoGlobalSetupConfig = {
  languages: [
    {
      id: "typescript",
      extensions: [".ts"],
      aliases: ["TypeScript", "ts"],
      filenames: ["tsconfig.json", "tsconfig.app.json", "tsconfig.spec.json"],
      firstLine: "^#!.*\\bnode",
    },
    {
      id: "javascript",
      extensions: [".js"],
      aliases: ["JavaScript", "js"],
      filenames: ["tsconfig.json", "tsconfig.app.json", "tsconfig.spec.json"],
    },
    {
      id: "python",
      extensions: [
        ".py",
        ".rpy",
        ".pyw",
        ".cpy",
        ".gyp",
        ".gypi",
        ".pyi",
        ".ipy",
        ".bzl",
        ".cconf",
        ".cinc",
        ".mcconf",
        ".sky",
        ".td",
        ".tw",
      ],
      aliases: ["Python", "py"],
      filenames: ["Snakefile", "BUILD", "BUCK", "TARGETS"],
      firstLine: "^#!\\s*/?.*\\bpython[0-9.-]*\\b",
    },
    {
      id: "dapen",
    },
    {
      id: "json",
    },
    {
      id: "bitide",
    },
    {
      id: "wgsl",
      extensions: [".wgsl"],
      aliases: ["WGSL"],
    }
  ],
  grammars: {
    "source.ts": {
      language: "typescript",
      url: "/grammars/typescript.json",
    },
    "source.wgsl": {
      language: "wgsl",
      url: "/grammars/wgsl.json",
    },
    "source.js": {
      language: "javascript",
      url: "/grammars/javascript.json",
    },
    "source.dapen": {
      language: "dapen",
      url: "/grammars/dapen.json",
    },
    "source.python": {
      language: "python",
      url: "/grammars/python.json",
    },
    "source.json": {
      language: "json",
      url: "/grammars/json.json",
    },
    "source.bitide": {
      language: "bitide",
      url: "/grammars/bitide.json",
    },
  },
  languageConfiguration: {
    bitide: {
      autoClosingPairs: [
        { open: '<', close: '>' },
        { open: '$(', close: ')' },
        { open: '"', close: '"', notIn: ['string'] },
        { open: "'", close: "'", notIn: ['string', 'comment'] },
        { open: '/**', close: ' */', notIn: ['string'] },
        { open: 'OP_IF', close: ' OP_ENDIF', notIn: ['string', 'comment'] },
        {
          open: 'OP_NOTIF',
          close: ' OP_ENDIF',
          notIn: ['string', 'comment'],
        },
      ],
      brackets: [
        ['<', '>'],
        ['$(', ')'],
      ],
  
      comments: {
        lineComment: '//',
        blockComment: ['/*', '*/'],
      },
      onEnterRules: [
        {
          // e.g. /** | */
          beforeText: /^\s*\/\*\*(?!\/)([^*]|\*(?!\/))*$/,
          afterText: /^\s*\*\/$/,
          action: {
            indentAction: IndentAction.IndentOutdent,
            appendText: ' * ',
          },
        },
        {
          // e.g. /** ...|
          beforeText: /^\s*\/\*\*(?!\/)([^*]|\*(?!\/))*$/,
          action: {
            indentAction: IndentAction.None,
            appendText: ' * ',
          },
        },
        {
          // e.g.  * ...|
          beforeText: /^(\t|[ ])*[ ]\*([ ]([^*]|\*(?!\/))*)?$/,
          afterText: /^(\s*(\/\*\*|\*)).*/,
          action: {
            indentAction: IndentAction.None,
            appendText: '* ',
          },
        },
        {
          // e.g.  */|
          beforeText: /^(\t|[ ])*[ ]\*\/\s*$/,
          action: {
            indentAction: IndentAction.None,
            removeText: 1,
          },
        },
        {
          // e.g.  *-----*/|
          beforeText: /^(\t|[ ])*[ ]\*[^/]*\*\/\s*$/,
          action: {
            indentAction: IndentAction.None,
            removeText: 1,
          },
        },
      ],
    },
    typescript: {
      wordPattern:
        /(-?\d*\.\d\w*)|([^\`\~\!\@\#\%\^\&\*\(\)\-\=\+\[\{\]\}\\\|\;\:\'\"\,\.\<\>\/\?\s]+)/g,
      comments: {
        lineComment: "//",
        blockComment: ["/*", "*/"],
      },
      brackets: [
        ["{", "}"],
        ["[", "]"],
        ["(", ")"],
        ["<", ">"],
      ],
      onEnterRules: [
        {
          beforeText: /^\s*\/\*\*(?!\/)([^\*]|\*(?!\/))*$/,
          afterText: /^\s*\*\/$/,
          action: {
            indentAction: IndentAction.IndentOutdent,
            appendText: " * ",
          },
        },
        {
          beforeText: /^\s*\/\*\*(?!\/)([^\*]|\*(?!\/))*$/,
          action: {
            indentAction: IndentAction.None,
            appendText: " * ",
          },
        },
        {
          beforeText: /^(\t|(\ \ ))*\ \*(\ ([^\*]|\*(?!\/))*)?$/,
          action: {
            indentAction: IndentAction.None,
            appendText: "* ",
          },
        },
        {
          beforeText: /^(\t|(\ \ ))*\ \*\/\s*$/,
          action: {
            indentAction: IndentAction.None,
            removeText: 1,
          },
        },
      ],
      autoClosingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: "(", close: ")" },
        { open: '"', close: '"', notIn: ["string"] },
        { open: "'", close: "'", notIn: ["string", "comment"] },
        { open: "`", close: "`", notIn: ["string", "comment"] },
        { open: "/**", close: " */", notIn: ["string"] },
      ],
      surroundingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: "(", close: ")" },
        { open: "<", close: ">" },
        { open: "'", close: "'" },
        { open: '"', close: '"' },
      ],
      folding: {
        markers: {
          start: new RegExp("^\\s*//\\s*#?region\\b"),
          end: new RegExp("^\\s*//\\s*#?endregion\\b"),
        },
      },
    },
    wgsl: {
      wordPattern:
        /(-?\d*\.\d\w*)|([^\`\~\!\@\#\%\^\&\*\(\)\-\=\+\[\{\]\}\\\|\;\:\'\"\,\.\<\>\/\?\s]+)/g,
      comments: {
        lineComment: "//",
        blockComment: ["/*", "*/"],
      },
      brackets: [
        ["{", "}"],
        ["[", "]"],
        ["(", ")"],
        ["<", ">"],
      ],
      onEnterRules: [
        {
          beforeText: /^\s*\/\*\*(?!\/)([^\*]|\*(?!\/))*$/,
          afterText: /^\s*\*\/$/,
          action: {
            indentAction: IndentAction.IndentOutdent,
            appendText: " * ",
          },
        },
        {
          beforeText: /^\s*\/\*\*(?!\/)([^\*]|\*(?!\/))*$/,
          action: {
            indentAction: IndentAction.None,
            appendText: " * ",
          },
        },
        {
          beforeText: /^(\t|(\ \ ))*\ \*(\ ([^\*]|\*(?!\/))*)?$/,
          action: {
            indentAction: IndentAction.None,
            appendText: "* ",
          },
        },
        {
          beforeText: /^(\t|(\ \ ))*\ \*\/\s*$/,
          action: {
            indentAction: IndentAction.None,
            removeText: 1,
          },
        },
      ],
      autoClosingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: "(", close: ")" },
        { open: '"', close: '"', notIn: ["string"] },
        { open: "'", close: "'", notIn: ["string", "comment"] },
        { open: "`", close: "`", notIn: ["string", "comment"] },
        { open: "/**", close: " */", notIn: ["string"] },
      ],
      surroundingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: "(", close: ")" },
        { open: "<", close: ">" },
        { open: "'", close: "'" },
        { open: '"', close: '"' },
      ],
      folding: {
        markers: {
          start: new RegExp("^\\s*//\\s*#?region\\b"),
          end: new RegExp("^\\s*//\\s*#?endregion\\b"),
        },
      },
    },
    javascript: {
      wordPattern:
        /(-?\d*\.\d\w*)|([^\`\~\!\@\#\%\^\&\*\(\)\-\=\+\[\{\]\}\\\|\;\:\'\"\,\.\<\>\/\?\s]+)/g,
      comments: {
        lineComment: "//",
        blockComment: ["/*", "*/"],
      },
      brackets: [
        ["{", "}"],
        ["[", "]"],
        ["(", ")"],
        ["<", ">"],
      ],
      onEnterRules: [
        {
          beforeText: /^\s*\/\*\*(?!\/)([^\*]|\*(?!\/))*$/,
          afterText: /^\s*\*\/$/,
          action: {
            indentAction: IndentAction.IndentOutdent,
            appendText: " * ",
          },
        },
        {
          beforeText: /^\s*\/\*\*(?!\/)([^\*]|\*(?!\/))*$/,
          action: {
            indentAction: IndentAction.None,
            appendText: " * ",
          },
        },
        {
          beforeText: /^(\t|(\ \ ))*\ \*(\ ([^\*]|\*(?!\/))*)?$/,
          action: {
            indentAction: IndentAction.None,
            appendText: "* ",
          },
        },
        {
          beforeText: /^(\t|(\ \ ))*\ \*\/\s*$/,
          action: {
            indentAction: IndentAction.None,
            removeText: 1,
          },
        },
      ],
      autoClosingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: "(", close: ")" },
        { open: '"', close: '"', notIn: ["string"] },
        { open: "'", close: "'", notIn: ["string", "comment"] },
        { open: "`", close: "`", notIn: ["string", "comment"] },
        { open: "/**", close: " */", notIn: ["string"] },
      ],
      surroundingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: "(", close: ")" },
        { open: "<", close: ">" },
        { open: "'", close: "'" },
        { open: '"', close: '"' },
      ],
      folding: {
        markers: {
          start: new RegExp("^\\s*//\\s*#?region\\b"),
          end: new RegExp("^\\s*//\\s*#?endregion\\b"),
        },
      },
    },
    python: {
      comments: {
        lineComment: "#",
        blockComment: ['"""', '"""'],
      },
      brackets: [
        ["{", "}"],
        ["[", "]"],
        ["(", ")"],
      ],
      autoClosingPairs: [
        {
          open: "{",
          close: "}",
        },
        {
          open: "[",
          close: "]",
        },
        {
          open: "(",
          close: ")",
        },
        {
          open: '"',
          close: '"',
          notIn: ["string"],
        },
        {
          open: 'r"',
          close: '"',
          notIn: ["string", "comment"],
        },
        {
          open: 'R"',
          close: '"',
          notIn: ["string", "comment"],
        },
        {
          open: 'u"',
          close: '"',
          notIn: ["string", "comment"],
        },
        {
          open: 'U"',
          close: '"',
          notIn: ["string", "comment"],
        },
        {
          open: 'f"',
          close: '"',
          notIn: ["string", "comment"],
        },
        {
          open: 'F"',
          close: '"',
          notIn: ["string", "comment"],
        },
        {
          open: 'b"',
          close: '"',
          notIn: ["string", "comment"],
        },
        {
          open: 'B"',
          close: '"',
          notIn: ["string", "comment"],
        },
        {
          open: "'",
          close: "'",
          notIn: ["string", "comment"],
        },
        {
          open: "r'",
          close: "'",
          notIn: ["string", "comment"],
        },
        {
          open: "R'",
          close: "'",
          notIn: ["string", "comment"],
        },
        {
          open: "u'",
          close: "'",
          notIn: ["string", "comment"],
        },
        {
          open: "U'",
          close: "'",
          notIn: ["string", "comment"],
        },
        {
          open: "f'",
          close: "'",
          notIn: ["string", "comment"],
        },
        {
          open: "F'",
          close: "'",
          notIn: ["string", "comment"],
        },
        {
          open: "b'",
          close: "'",
          notIn: ["string", "comment"],
        },
        {
          open: "B'",
          close: "'",
          notIn: ["string", "comment"],
        },
        {
          open: "`",
          close: "`",
          notIn: ["string"],
        },
      ],
      surroundingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: "(", close: ")" },
        { open: '"', close: '"' },
        { open: "'", close: "'" },
        { open: "`", close: "`" },
      ],
      folding: {
        offSide: true,
        markers: {
          start: new RegExp("^\\s*#\\s*region\\b"),
          end: new RegExp("^\\s*#\\s*endregion\\b"),
        },
      },
    },
    dapen: {
      comments: {
        // symbol used for single line comment. Remove this entry if your language does not support line comments
        lineComment: "//",
        // symbols used for start and end a block comment. Remove this entry if your language does not support block comments
        blockComment: ["/*", "*/"],
      },
      // symbols used as brackets
      brackets: [
        ["{", "}"],
        ["[", "]"],
        ["(", ")"],
      ],
      autoClosingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: "(", close: ")" },
        { open: "/**", close: " */", notIn: ["string"] },
      ],
      surroundingPairs: [
        { open: "{", close: "}" },
        { open: "[", close: "]" },
        { open: "(", close: ")" },
      ],
      // markers used to folding code regions
      folding: {
        markers: {
          start: new RegExp("^\\s*//\\s*#?region\\b"),
          end: new RegExp("^\\s*//\\s*#?endregion\\b"),
        },
      },
    },
  },
  themes: [
    {"name": "BitIDE", "url": "/themes/BitIDE.json"},

  ],
  defaultTheme: "BitIDE",
  onigurumaWasmUrl: "/onig.wasm",
  additionalLanguages: [
    {
      id: "dapen",
      filenamePatterns: ["*.dpn"],
    },
    {
      id: "wgsl",
      filenamePatterns: ["*.wgsl"],
    },
    {
      id: "bitide",
      filenamePatterns: ["*.bitide"],
    },
  ]
}

export {DEFAULT_MONACO_SETUP_CONFIG};