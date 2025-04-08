# QED LSP Developer Tutorial

QED is a custom language with a dedicated Language Server Protocol (LSP) service, providing basic features such as `hover`, `goto definition`, `find references`, and `formatting`.


This article will introduce how to use QED's language server `qed-lsp-server` for development experience under **VSCode** and **Neovim**.

---

## 🛠️ Preparation

1. Clone repository:

```bash
  git clone https://github.com/QEDProtocol/qed-lang.git
  cd qed-lang
```

2. Compile `qed-lsp-server`：

```bash
  cd qed-lsp-server
  cargo build --release
```

## 💻 VSCode Usage Tutorial
Developer debugging mode (recommended for developers)
1. Start VSCode:
```bash
  cd qed-lsp-server/qed-lsp-vscode
  code .
```
2. Press F5 to enter plugin debugging mode. VSCode will start a new VSCode window and load the local plugin.
3. In the new window, open a QED project containing Dargo.toml to enable plugin features, such as:
   * Mouse hover → Show type information
   * Right click → Goto Definition / Find References / Format


💡 Note:
In the file `qed-lsp-vscode/src/extension.ts`, the path to the `qed-lsp-server` binary is currently hardcoded:

```typescript
const serverExecutable = path.join(
    // Warning: this path is hardcoded and may not be portable across systems.
    context.extensionPath, '..', '..', 'target', 'release', 'qed-lsp-server'
);
```
If you need to change the path (for example, to use a different build directory or binary location), please modify this line accordingly and then rebuild the extension by running:
```bash
  npm run build
```

## 🧑‍💻 Neovim usage tutorial (based on Lazy.nvim)
1. Make sure you are using the lazy.nvim plugin manager.
   Add to init.lua
```lua
require("lazy").setup({
   { "neovim/nvim-lspconfig" }, -- LSP support
   { "nvim-treesitter/nvim-treesitter", build = ":TSUpdate" }, -- syntax highlighting
   { "hrsh7th/nvim-cmp" },
   { "hrsh7th/cmp-nvim-lsp" },
})
```
2. Register QED LSP
```lua
local lspconfig = require("lspconfig")
local configs = require("lspconfig.configs")

-- Register custom LSP
if not configs.qed_lsp then
configs.qed_lsp = {
   default_config = {
      cmd = {"/full path/to/qed-lsp-server" }, -- ⚠️ Note to fill in the full path
      filetypes = {"qed"},
      root_dir = lspconfig.util.root_pattern("Dargo.toml"),
      settings = {},
   },
}
end

lspconfig.qed_lsp.setup({})

-- Let Neovim recognize the `.qed` file type
vim.filetype.add({
   extension = {
      qed = "qed",
   },
})
```

3. Enable Tree-sitter highlighting (using Rust's highlighting solution)
   If you have installed Rust's Tree-sitter parser, you can use the following method to make .qed files reuse Rust's highlighting:
```lua
   vim.treesitter.language.register("rust", "qed")
```