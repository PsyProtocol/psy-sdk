use std::path::PathBuf;

use qed_ast::{DefaultVisitorContext, ModuleId};

use qedlang_core::dpn::ops::context_trait::{ContextFelt, DPNContext};

use crate::TypeCheckerVisitorContext;

use crate::error::{Error, Result};
use qed_fmt::Formatter;

use qed_ast::AstVisitor;

impl<F: ContextFelt + From<u32> + 'static, C: DPNContext<F>> TypeCheckerVisitorContext<F, C> {
    pub fn format_file(&mut self, file_path: &PathBuf) -> Result<String> {
        let file_path = file_path
            .canonicalize()
            .map_err(|err| Error::AnyhowError(anyhow::anyhow!("{}", err)))?;
        let file_id =
            self.program
                .file_resolver
                .resolve_id(&file_path)
                .ok_or(Error::AnyhowError(anyhow::anyhow!(
                    "cant resolve file `{}`",
                    file_path.display().to_string()
                )))?;
        let module_id = self
            .program
            .file_resolver
            .resolve_module_id(file_id)
            .ok_or(Error::AnyhowError(anyhow::anyhow!(
                "cant resolve module file `{}`",
                file_path.display().to_string()
            )))?;
        self.format_module(ModuleId(module_id))
    }

    pub fn format_module(&mut self, module_id: ModuleId) -> Result<String> {
        let mut default_visitor_context: DefaultVisitorContext<'_, F, C> =
            DefaultVisitorContext::new(&mut self.program);
        let mut formatter = Formatter::new();
        formatter
            .visit_module(module_id, &mut default_visitor_context)
            .map_err(|err| Error::AnyhowError(anyhow::anyhow!("{}", err)))?;

        Ok(formatter.get_output().to_owned())
    }
}
