use qed_sema::CheckedPathNode;
use tower_lsp::lsp_types::Range;

//when user hover in a symbol, we should show the symbol's type and definition
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub range: Range,
    pub path: CheckedPathNode,
    pub type_name: String,
    pub definition: String,
    pub documentation: String,
}

pub type SymbolMap = Vec<SymbolInfo>; //maybe we should use a hashmap or btree_map
