use qed_ast::Program;
use qed_dargo::workspace::Workspace;
use qed_interpreter::Interpreter;
use std::path::PathBuf;
use std::sync::Mutex;
use std::vec;
use tower_lsp::jsonrpc::Error;
use tower_lsp::lsp_types::{
    Range, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, Url,
};
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Hover,
        HoverContents, MarkupContent, MarkupKind,
    },
    Client, LanguageServer,
};

use log::debug;
use tower_lsp::lsp_types::{
    CompletionParams, CompletionResponse, DidChangeConfigurationParams,
    DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams, DidSaveTextDocumentParams,
    DocumentFormattingParams, GotoDefinitionParams, GotoDefinitionResponse, HoverParams,
    InitializedParams, Location, MessageType, OneOf, ReferenceParams, TextEdit,
};

use qed_sema::TypeCheckerVisitorContext;
use qedlang_core::dpn::ops::{exec_context::QExecContext, sym_felt::SymFeltRef};

use crate::utils::span_to_range;

pub struct QLspSimple {
    client: Client,
    ctx: Mutex<TypeCheckerVisitorContext<SymFeltRef, QExecContext>>,
    root_path: Mutex<PathBuf>,
}

impl QLspSimple {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            ctx: Mutex::new(TypeCheckerVisitorContext::new(Program::new())),
            root_path: Mutex::new(PathBuf::new()),
        }
    }

    pub fn init(&self, root_path: &PathBuf) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let package_dir = qed_dargo_toml::files::find_file_manifest_root(&root_path)?;
        let toml_path = qed_dargo_toml::files::get_package_manifest(&package_dir)?;
        // Resolve the workspace from the toml file. It will download dependencies as well.
        let workspace = qed_dargo_toml::resolve_workspace_from_toml(&toml_path)?;

        let mut self_root_path = self.root_path.lock().map_err(|err| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to lock mutex: {}", err),
            ))
        })?;
        *self_root_path = root_path.clone();

        let entry_manager = match qed_dargo_cli::resolve_entries(&workspace, None) {
            Ok(entry_manager) => entry_manager,
            Err(err) => {
                eprintln!("Error resolving entries: {}", err);
                return Err(Box::new(err));
            }
        };
        let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());
        let (_typechecker, ctx) = interpreter.typecheck(
            entry_manager.entry,
            entry_manager.dependencies_entries.into_iter().collect(),
        )?;

        let mut self_ctx = self.ctx.lock().map_err(|err| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to lock mutex: {}", err),
            ))
        })?;
        *self_ctx = ctx;
        Ok(())
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for QLspSimple {
    async fn initialize(
        &self,
        params: tower_lsp::lsp_types::InitializeParams,
    ) -> Result<tower_lsp::lsp_types::InitializeResult> {
        let tower_lsp::lsp_types::InitializeParams { root_uri, .. } = params;

        let uri = root_uri.clone().unwrap();
        let root_path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                dbg!(format!("{:?} to_file_path error", uri.to_string()));
                return Err(Error::invalid_request());
            }
        };

        self.init(&root_path).map_err(|err| {
            dbg!(format!("{:?} init error", err));
            Error::invalid_request()
        });

        Ok(tower_lsp::lsp_types::InitializeResult {
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
        debug!("initialized!");
    }

    async fn shutdown(&self) -> Result<()> {
        dbg!("shutdown!");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        dbg!(format!("did_open: {:?}", params));
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        dbg!(format!("did_change: {:?}", params));
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        dbg!(format!("did_save: {:?}", params));
        let uri = params.text_document.uri.clone();

        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                dbg!(format!("{:?} to_file_path error", uri.to_string()));
                return;
            }
        };

        let self_ctx = self.ctx.lock().unwrap();
        if self_ctx.program.file_resolver.resolve_id(&path).is_none() {
            eprintln!("{:?} not found in file_resolver", uri);
            return;
        }

        let root_path = { self.root_path.lock().unwrap().clone() };

        if let Err(e) = self.init(&root_path) {
            eprintln!("init failed: {:?}", e);
        }

        dbg!(format!("Saved file: {}", uri));
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        dbg!(format!("did_close: {:?}", params));
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let GotoDefinitionParams {
            text_document_position_params,
            ..
        } = params;

        let uri = &text_document_position_params.text_document.uri;
        let path = uri.to_file_path().unwrap();

        let ctx = self.ctx.lock().unwrap();
        let file_id = ctx.program.file_resolver.resolve_id(&path).unwrap().clone();

        let position = qed_ast::Position {
            file_id,
            line: text_document_position_params.position.line as usize,
            column: text_document_position_params.position.character as usize,
        };

        let location = match ctx.position_to_location(position) {
            Some(loc) => loc,
            None => {
                eprintln!("goto definition: cannot find location for position");
                return Ok(None);
            }
        };
        dbg!("goto definition: location: {:?}", location);
        match ctx.goto_definition(location) {
            Some(target_location) => {
                dbg!("goto definition: {:?}", location);
                let source_text = ctx
                    .program
                    .file_resolver
                    .resolve_content(&location.file_id)
                    .unwrap_or_default();

                let range = span_to_range(&location, source_text);
                //let path = ctx.program.file_resolver.resolve_path(&location.file_id);
                let target_path = ctx
                    .program
                    .file_resolver
                    .resolve_path(&target_location.file_id);

                let target_uri = match target_path {
                    Some(path_buf) => {
                        if path_buf == &path {
                            uri.clone()
                        } else {
                            match Url::from_file_path(path_buf) {
                                Ok(url) => url,
                                Err(_) => {
                                    return Err(Error::invalid_params("Invalid target file path"));
                                }
                            }
                        }
                    }
                    None => {
                        return Err(Error::invalid_params(
                            "Cannot resolve path for target file_id",
                        ));
                    }
                };

                Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: target_uri,
                    range,
                })))
            }
            None => {
                eprintln!("goto definition: cannot find go to position");
                Ok(None)
            }
        }
    }
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let ReferenceParams {
            text_document_position,
            ..
        } = params;

        let uri = &text_document_position.text_document.uri;
        let path = uri.to_file_path().unwrap();

        let ctx = self.ctx.lock().unwrap();
        let file_id = ctx.program.file_resolver.resolve_id(&path).unwrap().clone();

        let position = qed_ast::Position {
            file_id: file_id,
            line: text_document_position.position.line as usize,
            column: text_document_position.position.character as usize,
        };

        let location = ctx.position_to_location(position).unwrap();

        match ctx.find_all_references(location, true, false) {
            Some(locations) => Ok(Some(
                locations
                    .iter()
                    .map(|loc| {
                        let range = span_to_range(
                            loc,
                            ctx.program
                                .file_resolver
                                .resolve_content(&loc.file_id)
                                .unwrap(),
                        );
                        Location {
                            uri: uri.clone(),
                            range: range,
                        }
                    })
                    .collect::<Vec<_>>(),
            )),
            None => Ok(None),
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let HoverParams {
            text_document_position_params,
            ..
        } = params;

        let uri = &text_document_position_params.text_document.uri;
        let path = uri.to_file_path().unwrap();
        let ctx = self.ctx.lock().unwrap();

        let position = qed_ast::Position {
            file_id: *ctx.program.file_resolver.resolve_id(&path).unwrap(),
            line: text_document_position_params.position.line as usize,
            column: text_document_position_params.position.character as usize,
        };

        let location = match ctx.position_to_location(position) {
            Some(l) => l,
            None => {
                dbg!("position_to_location error");
                return Err(Error::internal_error());
            }
        };

        let hover = ctx.hover(location).map(|hover_str| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**Type**: `{}`\n\n", hover_str),
            }),
            range: None,
        });
        Ok(hover)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let DocumentFormattingParams { text_document, .. } = params;

        let uri = &text_document.uri;
        let path = uri.to_file_path().unwrap();
        let mut ctx = self.ctx.lock().unwrap();

        let formatted_content = ctx.format_file(&path).map_err(|err| {
            dbg!(&err);
            Error::parse_error()
        })?;

        let text_edit = TextEdit {
            range: Range {
                start: tower_lsp::lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: tower_lsp::lsp_types::Position {
                    line: 1000,
                    character: 1000,
                },
            },
            new_text: formatted_content,
        };

        Ok(Some(vec![text_edit]))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        dbg!(&params);
        Ok(None)
    }
    //rename
    async fn rename(
        &self,
        params: tower_lsp::lsp_types::RenameParams,
    ) -> Result<Option<tower_lsp::lsp_types::WorkspaceEdit>> {
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
}
