use qed_ast::{Position, Program, TextPosition, TextRange};
use qed_interpreter::Interpreter;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::vec;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, InitializeParams, InitializeResult, Range,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, Url,
};
use tower_lsp::{
    lsp_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Hover,
        HoverContents, MarkupContent, MarkupKind,
    },
    Client, LanguageServer,
};

use tower_lsp::jsonrpc::{Error as TError, Result as TResult};

use crate::error::{QLspError, QLspResult};
use crate::utils::span_to_range;
use dargo::{resolve_entries, EntryManager};
use qed_common::FileId;
use qed_dargo_toml::files::{find_file_manifest_root, get_package_manifest};
use qed_dargo_toml::resolve_workspace_from_toml;
use qed_sema::{offset_from_position, TypeCheckError, TypeCheckerVisitorContext};
use qedlang_core::dpn::ops::{exec_context::QExecContext, sym_felt::SymFeltRef};
use tower_lsp::lsp_types::{
    CompletionParams, CompletionResponse, DidChangeConfigurationParams,
    DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams, DidSaveTextDocumentParams,
    DocumentFormattingParams, GotoDefinitionParams, GotoDefinitionResponse, HoverParams,
    InitializedParams, Location, OneOf, ReferenceParams, TextEdit,
};

pub struct QLspSimple {
    client: Client,
    ctx: Arc<RwLock<TypeCheckerVisitorContext<SymFeltRef, QExecContext>>>,
    root_path: Arc<RwLock<PathBuf>>,
    entry_manager_cache: Arc<RwLock<HashMap<PathBuf, EntryManager>>>,
    last_diagnostic_uri: RwLock<Option<Url>>,
}

impl QLspSimple {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            ctx: Arc::new(RwLock::new(TypeCheckerVisitorContext::new(Program::new()))),
            root_path: Arc::new(RwLock::new(PathBuf::new())),
            entry_manager_cache: Arc::new(RwLock::new(HashMap::new())),
            last_diagnostic_uri: RwLock::new(None),
        }
    }

    pub fn get_ctx_read(
        &self,
    ) -> RwLockReadGuard<'_, TypeCheckerVisitorContext<SymFeltRef, QExecContext>> {
        self.ctx.read().expect("ctx read lock poisoned")
    }

    pub fn get_ctx_write(
        &self,
    ) -> RwLockWriteGuard<'_, TypeCheckerVisitorContext<SymFeltRef, QExecContext>> {
        self.ctx.write().expect("ctx write lock poisoned")
    }

    pub fn get_root_path(&self) -> PathBuf {
        self.root_path
            .read()
            .expect("root_path read lock poisoned")
            .clone()
    }
    pub fn root_uri(&self) -> Option<Url> {
        let path = self.get_root_path();
        Url::from_file_path(path).ok()
    }
    pub fn client(&self) -> &Client {
        &self.client
    }
    pub fn set_root_path(&self, path: &PathBuf) -> QLspResult<()> {
        let mut locked = self.root_path.write().map_err(|e| {
            QLspError::Internal(format!("Failed to lock root_path (poisoned): {}", e))
        })?;
        *locked = path.clone();
        Ok(())
    }
    pub fn set_ctx(
        &self,
        new_ctx: TypeCheckerVisitorContext<SymFeltRef, QExecContext>,
    ) -> QLspResult<()> {
        let mut ctx = self.get_ctx_write();
        *ctx = new_ctx;
        Ok(())
    }
    pub fn set_last_diagnostic_uri(&self, uri: Url) {
        let mut guard = self.last_diagnostic_uri.write().expect("lock poisoned");
        *guard = Some(uri);
    }
    pub fn get_last_diagnostic_uri(&self) -> Option<Url> {
        self.last_diagnostic_uri
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    pub fn get_cached_entry_manager(&self, path: &PathBuf) -> Option<EntryManager> {
        self.entry_manager_cache
            .read()
            .ok()
            .and_then(|map| map.get(path).cloned())
    }
    pub fn set_entry_manager_cache(&self, path: PathBuf, manager: EntryManager) {
        let mut map = self.entry_manager_cache.write().expect("lock poisoned");
        map.insert(path, manager);
    }
    pub fn clear_all_entry_manager_cache(&self) {
        let mut map = self.entry_manager_cache.write().expect("lock poisoned");
        map.clear();
    }
    pub fn remove_entry_manager_cache(&self, path: &PathBuf) {
        let mut map = self.entry_manager_cache.write().expect("lock poisoned");
        map.remove(path);
    }

    pub fn is_ready(&self) -> bool {
        let ctx = self.get_ctx_read();
        !ctx.symbols.is_empty()
    }

    pub fn resolve_file_id(&self, path: &PathBuf) -> QLspResult<FileId> {
        let ctx_guard = self.get_ctx_read();
        ctx_guard
            .program
            .file_resolver
            .resolve_id(path)
            .cloned()
            .ok_or_else(|| QLspError::FileIdNotFound(path.clone()))
    }

    pub async fn init_and_publish_diagnostics(&self, root_path: &PathBuf) -> TResult<()> {
        match self.collect_diagnostics_sync(root_path) {
            Ok((Some(uri), diagnostics)) => {
                eprintln!("[diagnostics] Start publishing diagnostics");
                eprintln!("[diagnostics] URI: {:?}", uri);
                eprintln!("[diagnostics] Payload: {:?}", diagnostics);

                self.set_last_diagnostic_uri(uri.clone());
                self.client
                    .publish_diagnostics(uri.clone(), diagnostics, None)
                    .await;

                eprintln!("[diagnostics] Publish finished");
            }
            Ok((None, _)) => {
                eprintln!("[diagnostics] No URI returned, skipping publish");
                if let Some(uri) = self.get_last_diagnostic_uri() {
                    self.client.publish_diagnostics(uri, vec![], None).await;
                }
                // self.client.publish_diagnostics(uri.clone(), vec![], None).await;
            }
            Err(err) => {
                eprintln!("[diagnostics] Failed to collect diagnostics: {:?}", err);
                let _ = self
                    .client
                    .log_message(
                        tower_lsp::lsp_types::MessageType::ERROR,
                        format!("Context error: {}", err),
                    )
                    .await;
            }
        }

        Ok(())
    }

    pub fn collect_diagnostics_sync(
        &self,
        root_path: &PathBuf,
    ) -> QLspResult<(Option<Url>, Vec<Diagnostic>)> {
        self.set_root_path(root_path)?;

        //use cached entry manager if available
        let entry_manager = if let Some(cached) = self.get_cached_entry_manager(root_path) {
            dbg!("Using cached entry manager");
            cached
        } else {
            dbg!("Cannot find cached entry manager, creating a new one");

            let package_dir = find_file_manifest_root(&root_path)
                .map_err(|e| QLspError::Internal(format!("Failed to find manifest root: {}", e)))?;

            let toml_path = get_package_manifest(&package_dir)
                .map_err(|e| QLspError::Internal(format!("Failed to get manifest file: {}", e)))?;

            // Resolve the workspace from the toml file. It will download dependencies as well.
            let workspace = resolve_workspace_from_toml(&toml_path)
                .map_err(|e| QLspError::Internal(format!("Failed to resolve workspace: {}", e)))?;

            let new_manager = resolve_entries(&workspace, None)
                .map_err(|e| QLspError::Internal(format!("Error resolving entries: {}", e)))?;

            self.set_entry_manager_cache(root_path.clone(), new_manager.clone());
            new_manager
        };

        let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());

        let result = interpreter.typecheck_lsp(
            entry_manager.entry,
            entry_manager.dependencies_entries.into_iter().collect(),
        );

        match result {
            Ok((_typechecker, ctx)) => {
                eprintln!("typecheck_lsp success");

                if let Err(e) = self.set_ctx(ctx) {
                    eprintln!("[init warning] Failed to set ctx: {}", e);
                    let _ = self.client.log_message(
                        tower_lsp::lsp_types::MessageType::ERROR,
                        format!("cannot set context: {}", e),
                    );
                }
                Ok((None, vec![]))
            }
            Err(err) => {
                let (uri_opt, diagnostic) = match err {
                    TypeCheckError::Parse(desc) | TypeCheckError::TypeCheck(desc) => {
                        let uri_opt = desc
                            .file
                            .as_ref()
                            .and_then(|path| Url::from_file_path(path).ok());

                        let diagnostic = Diagnostic {
                            range: desc
                                .text_range
                                .map(to_lsp_range)
                                .unwrap_or_else(dummy_range),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: desc.message,
                            source: Some("qed-lsp".into()),
                            ..Default::default()
                        };

                        let ret = (uri_opt, diagnostic);
                        eprintln!("typecheck_lsp failed ret = {:?}", ret);
                        ret
                    }
                    TypeCheckError::Cycle(msg) | TypeCheckError::StoragePreprocess(msg) => {
                        let uri_opt = self.root_uri();
                        let diagnostic = Diagnostic {
                            range: dummy_range(),
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: msg,
                            source: Some("qed-lsp".into()),
                            ..Default::default()
                        };
                        (uri_opt, diagnostic)
                    }
                };
                Ok((uri_opt, vec![diagnostic]))
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for QLspSimple {
    async fn initialize(&self, params: InitializeParams) -> TResult<InitializeResult> {
        let root_uri = params.root_uri.ok_or_else(|| {
            let msg = "Missing root_uri in InitializeParams".to_string();
            eprintln!("{msg}");
            TError::invalid_params(msg)
        })?;

        let root_path = root_uri.to_file_path().map_err(|_| {
            let msg = format!("Failed to convert root_uri {:?} to file path", root_uri);
            eprintln!("{msg}");
            TError::invalid_params(msg)
        })?;

        let _ = self.init_and_publish_diagnostics(&root_path).await;

        Ok(InitializeResult {
            capabilities: tower_lsp::lsp_types::ServerCapabilities {
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL), // Or FULL
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true).into()),
                    },
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }
    async fn initialized(&self, _: InitializedParams) {
        dbg!("initialized!");
    }

    async fn shutdown(&self) -> TResult<()> {
        dbg!("shutdown!");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        // dbg!(format!("did_open: {:?}", params));
        // let uri = &params.text_document.uri;
        //
        // if let Ok(path) = uri.to_file_path() {
        //     let _ = self.init_and_publish_diagnostics(&path).await;
        // }
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        dbg!(format!("did_change: {:?}", params));
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        dbg!(format!("did_save: {:?}", params));
        let uri = &params.text_document.uri;

        let path = match try_uri_to_path(uri) {
            Some(p) => p,
            None => {
                eprintln!("{:?} to_file_path error", uri);
                return;
            }
        };

        // Check if the file is already in the file_resolver
        if self.is_ready() {
            let ctx_guard = self.get_ctx_read();
            if ctx_guard.program.file_resolver.resolve_id(&path).is_none() {
                eprintln!("{:?} not found in file_resolver", uri);
                return;
            }
        }

        let root_path = self.get_root_path();
        dbg!("did_save prepare init: {:?}", &root_path);
        let _ = self.init_and_publish_diagnostics(&root_path).await;

        dbg!("did_save init Success");

        dbg!(format!("Saved file: {}", uri));
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        dbg!(format!("did_close: {:?}", params));
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> TResult<Option<GotoDefinitionResponse>> {
        if !self.is_ready() {
            return Ok(None);
        }

        let uri = &params.text_document_position_params.text_document.uri;

        let path = uri
            .to_file_path()
            .map_err(|_| QLspError::UriToPathError(uri.to_string()))?;

        let file_id = self.resolve_file_id(&path)?;

        let position = Position {
            file_id,
            line: params.text_document_position_params.position.line as usize,
            column: params.text_document_position_params.position.character as usize,
        };
        dbg!(format!("goto position: {:?}", position));

        let ctx = self.get_ctx_read();
        let location = ctx
            .position_to_location(position)
            .ok_or_else(|| QLspError::Internal("Failed to map position to location".into()))?;

        dbg!(format!("goto: source loc {:?}", location));

        let target_location = ctx
            .goto_definition(location)
            .ok_or_else(|| QLspError::Internal("goto definition: target not found".into()))?;

        dbg!(format!("goto: target location {:?}", target_location));

        let source_text = ctx
            .program
            .file_resolver
            .resolve_content(&location.file_id)
            .ok_or_else(|| {
                QLspError::Internal(format!(
                    "Cannot resolve file content for file_id: {:?}",
                    location.file_id
                ))
            })?;

        let range = span_to_range(&target_location, source_text);

        let target_path = ctx
            .program
            .file_resolver
            .resolve_path(&target_location.file_id)
            .ok_or_else(|| QLspError::Internal("Cannot resolve path for target file_id".into()))?;

        let target_uri = if target_path == &path {
            uri.clone()
        } else {
            Url::from_file_path(target_path)
                .map_err(|_| QLspError::InvalidUri("Invalid target file path".to_string()))?
        };
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: target_uri,
            range,
        })))
    }
    async fn references(&self, params: ReferenceParams) -> TResult<Option<Vec<Location>>> {
        if !self.is_ready() {
            return Ok(None);
        }
        let uri = &params.text_document_position.text_document.uri;

        let path = uri
            .to_file_path()
            .map_err(|_| QLspError::UriToPathError(uri.to_string()))?;

        let file_id = self.resolve_file_id(&path)?;
        let position = Position {
            file_id,
            line: params.text_document_position.position.line as usize,
            column: params.text_document_position.position.character as usize,
        };

        let ctx = self.get_ctx_read();

        let location = ctx
            .position_to_location(position)
            .ok_or_else(|| QLspError::Internal("Cannot find location for position".to_string()))?;

        let Some(locations) = ctx.find_all_references(location, true, false) else {
            return Ok(None);
        };

        let resolved_locations = locations
            .iter()
            .filter_map(|loc| {
                ctx.program
                    .file_resolver
                    .resolve_content(&loc.file_id)
                    .map(|source| {
                        let range = span_to_range(loc, source);
                        Location {
                            uri: uri.clone(),
                            range,
                        }
                    })
            })
            .collect::<Vec<_>>();
        Ok(Some(resolved_locations))
    }

    async fn hover(&self, params: HoverParams) -> TResult<Option<Hover>> {
        if !self.is_ready() {
            return Ok(None);
        }
        let uri = &params.text_document_position_params.text_document.uri;
        let path = uri
            .to_file_path()
            .map_err(|_| QLspError::UriToPathError(uri.to_string()))?;
        let file_id = self.resolve_file_id(&path)?;

        let position = Position {
            file_id,
            line: params.text_document_position_params.position.line as usize,
            column: params.text_document_position_params.position.character as usize,
        };

        let ctx = self.get_ctx_read();
        let location = ctx
            .position_to_location(position)
            .ok_or_else(|| QLspError::Internal("Failed to convert position to location".into()))?;

        let hover_text = ctx.hover(location);

        let hover = hover_text.map(|text| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**Type**: `{}`", text),
            }),
            range: None,
        });

        Ok(hover)
    }

    async fn completion(&self, params: CompletionParams) -> TResult<Option<CompletionResponse>> {
        dbg!(&params);
        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> TResult<Option<Vec<TextEdit>>> {
        if !self.is_ready() {
            return Ok(None);
        }
        let DocumentFormattingParams { text_document, .. } = params;

        let uri = &text_document.uri;
        let path = uri
            .to_file_path()
            .map_err(|_| QLspError::UriToPathError(uri.to_string()))?;
        let mut ctx = self.get_ctx_write();

        let formatted = ctx
            .format_file(&path)
            .map_err(|e| QLspError::Internal(format!("format_file failed: {}", e)))?;

        let text_edit = TextEdit {
            range: Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: tower_lsp::lsp_types::Position {
                    //todo!: replace with the actual end position
                    line: 100000,
                    character: 1000,
                },
            },
            new_text: formatted,
        };

        Ok(Some(vec![text_edit]))
    }
    //rename
    async fn rename(
        &self,
        params: tower_lsp::lsp_types::RenameParams,
    ) -> TResult<Option<tower_lsp::lsp_types::WorkspaceEdit>> {
        dbg!(&params);
        Ok(None)
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        dbg!("configuration changed!");
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        dbg!("workspace folders changed!");
    }
    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        dbg!("watched files have changed!");
    }
}

pub fn try_uri_to_path(uri: &Url) -> Option<PathBuf> {
    match uri.to_file_path() {
        Ok(p) => Some(p),
        Err(_) => {
            dbg!(format!("{:?} to_file_path error", uri));
            None
        }
    }
}

/// `TextPosition` → LSP `Position`
fn to_lsp_position(pos: TextPosition) -> tower_lsp::lsp_types::Position {
    tower_lsp::lsp_types::Position {
        line: pos.line,
        character: pos.character,
    }
}

/// `TextRange` → LSP `Range`
fn to_lsp_range(range: TextRange) -> Range {
    Range {
        start: to_lsp_position(range.start),
        end: to_lsp_position(range.end),
    }
}
/// Dummy range: typically used for unknown or fallback diagnostic positions.
pub fn dummy_range() -> Range {
    Range {
        start: tower_lsp::lsp_types::Position {
            line: 0,
            character: 0,
        },
        end: tower_lsp::lsp_types::Position {
            line: 0,
            character: 1,
        },
    }
}
