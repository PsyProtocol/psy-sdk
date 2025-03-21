use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, TextDocumentIdentifier, TextDocumentPositionParams, DidOpenTextDocumentParams, DidCloseTextDocumentParams, DidChangeTextDocumentParams},
    Client, LanguageServer, LspService, Server
};
use tokio::sync::RwLock;
use std::sync::Arc;

use dashmap::DashMap;
use log::__private_api::Value;
use tower_lsp::lsp_types::{CompletionParams, CompletionResponse, DidChangeConfigurationParams, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams, DidSaveTextDocumentParams, ExecuteCommandParams, GotoDefinitionParams, GotoDefinitionResponse, HoverParams, InitializedParams, Location, MessageType, ReferenceParams, TextDocumentItem, Url};
use log::debug;
use lsp_types::{MarkedString, VersionedTextDocumentIdentifier, WorkspaceEdit};
use qed_lsp_server::builtins::get_builtin_description;
use qed_lsp_server::helpers::str_range;
use qed_lsp_server::store::{AnalysisCache, ProgramStore};

use qedlang_core::dpn::{
    ops::{exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::compile::QEDCompileResult,
};
pub struct QedFile {
    file_text: String,
}

use lsp_types::{Diagnostic, DiagnosticSeverity, PublishDiagnosticsParams};
use qedlang_core::dpn::ops::context_trait::DPNContext;
use qed_sema::CheckedProgram;

pub struct Diagnostics {
    pub inner: HashMap<Url, PublishDiagnosticsParams>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn insert(&mut self, url: Url, diagnostics: PublishDiagnosticsParams) {
        self.inner.insert(url, diagnostics);
    }
}

/*

 */
pub struct QedLsp <F: Clone + From<u32>, C: DPNContext<F> + Clone + 'static> {
    client: Client,
    files: DashMap<String, QedFile>,
    pub documents: Documents,
    pub programs: ProgramStore<SymFeltRef>,

    pub analysis: Arc<AnalysisCache<F, C>>,
    pub context_map: DashMap<Url, C>,

}

pub type Documents = HashMap<Url, TextDocumentItem>;

impl<F: Clone + From<u32>, C: DPNContext<F> + Clone + 'static> QedLsp<F, C> {
    pub fn new(client: Client, context: C) -> Self {
        Self {
            client,
            files: DashMap::new(),
            documents: HashMap::new(),
            programs: ProgramStore::new(),
            analysis: Arc::new(AnalysisCache::new()),
            context_map: DashMap::new(),
        }
    }

    async fn did_open(&mut self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        //todo: replace QExecContext::new() with C::new()?
        let context = QExecContext::new();
        self.context_map.insert(uri.clone(), context.clone());

        // use  analysis.reload 
        if let Err(err) = self.analysis.reload(&mut context.clone(), uri.clone(), &params.text_document.text) {
            self.client
                .log_message(MessageType::ERROR, format!("Reload error: {err:?}"))
                .await;
        } else {
            self.client
                .log_message(MessageType::INFO, format!("Reloaded: {}", uri))
                .await;
        }

        //insert the all content of the file into the documents
        self.documents.insert(uri.clone(), params.text_document);

        self.client.log_message(tower_lsp::lsp_types::MessageType::INFO, format!("Opened file: {}", uri)).await;
    }

    /*
        When the user closes the file, immediately clean up the document and program cache.
        Maybe the cleanup should be delayed, but we handle it simply here.
     */
    async fn did_close(&mut self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let uri_str = uri.to_string();

        self.files.remove(&uri_str);

        self.documents.remove(&uri);

        self.context_map.remove(&uri);

        self.analysis.checked_programs.remove(&uri);
        self.analysis.type_contexts.remove(&uri);
        self.analysis.symbol_ranges.remove(&uri);

        let path = std::path::PathBuf::from(uri.path());
        self.programs.map.remove(&path);

        self.client
            .log_message(
                tower_lsp::lsp_types::MessageType::INFO,
                format!("Closed file and removed all related caches: {}", uri),
            )
            .await;
    }

    async fn did_change(&mut self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let content = params
            .content_changes
            .last()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        self.files.insert(
            uri.to_string(),
            QedFile {
                file_text: content.clone(),
            },
        );

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            doc.text = content.clone();
        }

        self.client
            .log_message(MessageType::INFO, format!("Changed file: {}", uri))
            .await;

        self.reload(&uri, &content).await;
    }

    pub async fn reload(&self, uri: &Url, content: &str) {
        let ctx = self
            .context_map
            .entry(uri.clone())
            //todo: replace with QExecContext::new() with C::?
            .or_insert_with(|| QExecContext::new())
            .clone();

        if let Err(err) = self.analysis.reload(&mut ctx.clone(), uri.clone(), content) {
            self.client
                .log_message(MessageType::ERROR, format!("Reload error: {err:?}"))
                .await;
        } else {
            self.client
                .log_message(MessageType::INFO, format!("Reloaded: {}", uri))
                .await;
        }
    }
}


#[tower_lsp::async_trait]
impl<F, C> LanguageServer for QedLsp<F, C>
where
    F: Clone + From<u32> + Send + Sync + 'static,
    C: DPNContext<F> + Clone + Send + Sync + 'static,
{
    async fn initialize(&self, _: tower_lsp::lsp_types::InitializeParams) -> Result<tower_lsp::lsp_types::InitializeResult> {
        Ok(tower_lsp::lsp_types::InitializeResult {
            capabilities: tower_lsp::lsp_types::ServerCapabilities {
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }
    async fn initialized(&self, _: InitializedParams) {
        debug!("initialized!");
    }


    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {

        let mut this = self as *const _ as *mut QedLsp<F, C>;
        // Safety: LSP ensures exclusive access to this function
        // todo!: remove unsafe
        unsafe { &mut *this }.did_open(params).await;


    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let mut this = self as *const _ as *mut QedLsp<F, C>;
        unsafe { &mut *this }.did_change(params).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        dbg!(&params.text);
        todo!();
        debug!("file saved!");
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.did_close(params).await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        dbg!(&params.text_document_position_params);
        Ok(None)
    }
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        dbg!(&params);
        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let HoverParams {
            text_document_position_params,
            ..
        } = params;

        //get user hover file
        let uri = &text_document_position_params.text_document.uri;
        let position = text_document_position_params.position;

        // get the source text from documents
        let source_text = self
            .documents
            .get(uri)
            .map(|doc| doc.text.clone())
            .unwrap_or_default(); // if not found, use default

        // reload the file
        self.reload(&uri, &source_text).await;

        // get the symbol info
        if let Some(symbol) = self.analysis.get_symbol_info(uri, position) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        "**Type**: `{}`\n\n**Definition**: {}\n\n{}",
                        symbol.type_name, symbol.definition, symbol.documentation
                    ),
                }),
                range: Some(symbol.range),
            }));
        }

        // if not found, return the builtin description
        let hover_text = format!(
            "Hover {:?}:{:?}",
            text_document_position_params.text_document.uri.path(),
            text_document_position_params.position
        );


        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_text,
            }),
            range: None,
        }))
    }
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        dbg!(&params);
        Ok(None)
    }
    //rename
    async fn rename(&self, params: tower_lsp::lsp_types::RenameParams) -> Result<Option<tower_lsp::lsp_types::WorkspaceEdit>> {
        dbg!(&params);
        Ok(None)
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        debug!("configuration changed!");
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        debug!("workspace folders changed!");
    }
    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        debug!("watched files have changed!");
    }
    // async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
    //     debug!("command executed!");
    //     Ok(None)
    // }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let (service, socket) = LspService::build(QedLsp::new).finish();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    Server::new(stdin, stdout, socket).serve(service).await;
}