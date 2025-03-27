use qed_ast::Program;
use qed_common::FileId;
use qed_interpreter::Interpreter;
use tower_lsp::jsonrpc::Error;
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
    GotoDefinitionParams, GotoDefinitionResponse, HoverParams, InitializedParams, Location,
    MessageType, ReferenceParams,
};

use qed_sema::TypeCheckerVisitorContext;
use qedlang_core::dpn::ops::{exec_context::QExecContext, sym_felt::SymFeltRef};

use crate::store::span_to_range;

pub struct QLspSimple {
    client: Client,
    ctx: TypeCheckerVisitorContext<SymFeltRef, QExecContext>,
}

impl QLspSimple {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            ctx: TypeCheckerVisitorContext::new(Program::new()),
        }
    }

    async fn did_open(&mut self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(err) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("{:?} to_file_path error", uri.to_string()),
                    )
                    .await;
                return;
            }
        };

        let mut interpreter = Interpreter::new(QExecContext::new());

        let (_typechecker, ctx) = match interpreter.typecheck(path.clone()) {
            Ok(t) => t,
            Err(err) => {
                self.client
                    .log_message(MessageType::ERROR, format!("Reload error: {err:?}"))
                    .await;
                return;
            }
        };

        self.ctx = ctx;

        let paths = unsafe { &*self.ctx.program.file_resolver.file_paths.get() };
        for p in paths.iter() {
            dbg!(p.to_string_lossy());
        }

        self.client
            .log_message(
                tower_lsp::lsp_types::MessageType::INFO,
                format!("Opened file: {}", uri),
            )
            .await;
    }

    /*
       When the user closes the file, immediately clean up the document and program cache.
       Maybe the cleanup should be delayed, but we handle it simply here.
    */
    async fn did_close(&mut self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        self.client
            .log_message(
                tower_lsp::lsp_types::MessageType::INFO,
                format!("Closed file and removed all related caches: {}", uri),
            )
            .await;
    }

    async fn did_change(&mut self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(err) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("{:?} to_file_path error", uri.to_string()),
                    )
                    .await;
                return;
            }
        };

        let mut interpreter = Interpreter::new(QExecContext::new());

        let (_typechecker, ctx) = match interpreter.typecheck(path.clone()) {
            Ok(t) => t,
            Err(err) => {
                self.client
                    .log_message(MessageType::ERROR, format!("Reload error: {err:?}"))
                    .await;
                return;
            }
        };

        self.ctx = ctx;

        self.client
            .log_message(MessageType::INFO, format!("Changed file: {}", uri))
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for QLspSimple {
    async fn initialize(
        &self,
        _: tower_lsp::lsp_types::InitializeParams,
    ) -> Result<tower_lsp::lsp_types::InitializeResult> {
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
        let this = self as *const _ as *mut QLspSimple;
        // Safety: LSP ensures exclusive access to this function
        // todo!: remove unsafe
        unsafe { &*this }.did_open(params).await;
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let this = self as *const _ as *mut QLspSimple;
        unsafe { &*this }.did_change(params).await;
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
        let GotoDefinitionParams {
            text_document_position_params,
            ..
        } = params;

        let uri = &text_document_position_params.text_document.uri;
        let mut path = uri.to_file_path().unwrap();

        let file_id = self.ctx.program.file_resolver.resolve_id(&path).unwrap();

        let position = qed_ast::Position {
            file_id: *file_id,
            line: text_document_position_params.position.line as usize,
            column: text_document_position_params.position.character as usize,
        };

        let location = self.ctx.position_to_location(position).unwrap();
        match self.ctx.goto_definition(location) {
            Some(location) => {
                let range = span_to_range(
                    &location,
                    self.ctx
                        .program
                        .file_resolver
                        .resolve_content(file_id)
                        .unwrap(),
                );
                Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: uri.clone(),
                    range: range,
                })))
            }
            None => Ok(None),
        }
    }
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        dbg!(&params);
        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        dbg!(&params);
        let HoverParams {
            text_document_position_params,
            ..
        } = params;

        //get user hover file
        let uri = &text_document_position_params.text_document.uri;

        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(err) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("{:?} to_file_path error", uri.to_string()),
                    )
                    .await;
                dbg!("to_file_path error");
                return Err(Error::internal_error());
            }
        };

        let mut interpreter = Interpreter::new(QExecContext::new());

        let (_typechecker, ctx) = match interpreter.typecheck(path.clone()) {
            Ok(t) => t,
            Err(err) => {
                dbg!(err);
                return Err(Error::internal_error());
            }
        };

        let paths = unsafe { &*ctx.program.file_resolver.file_paths.get() };
        for p in paths.iter() {
            dbg!(p.to_string_lossy());
        }

        let file_id = ctx
            .program
            .file_resolver
            .resolve_id(&path)
            .unwrap_or(&FileId(0));

        let position = qed_ast::Position {
            file_id: *file_id,
            line: text_document_position_params.position.line as usize,
            column: text_document_position_params.position.character as usize,
        };

        let location = match ctx.position_to_location(position){
            Some(l) => l,
            None => {
                dbg!("position_to_location error");
                return Err(Error::internal_error());
            }
        };

        let hover = self.ctx.hover(location).map(|hover_str| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**Type**: `{}`\n\n", hover_str),
            }),
            range: None,
        });
        Ok(hover)
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
    // async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
    //     debug!("command executed!");
    //     Ok(None)
    // }
}
