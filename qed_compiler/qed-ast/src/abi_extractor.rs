use crate::{
    AstVisitor, ContractAbi, DefaultVisitorContext, DefId, DefinitionNode, EnumAbi, EnumNode,
    EnumVariant, ExprId, ExprNode, FieldAbi, FunctionAbi, FunctionNode, ParameterAbi, Program,
    StmtId, StmtNode, StructAbi, StructNode, TraitAbi, TraitNode, TypeAbi, VariantAbi,
    VariantTypeAbi, Visibility, VisitorContext, AssociatedTypeAbi,
    // New spec-compliant ABI types
    SpecCompliantAbi, StructAbiSpec, FieldAbiSpec, FunctionAbiSpec, ParamAbiSpec, TypeAbiSpec
};

pub struct AbiExtractor {
    pub contract_abi: ContractAbi,
}

impl AbiExtractor {
    pub fn new(contract_name: String) -> Self {
        Self {
            contract_abi: ContractAbi::new(contract_name),
        }
    }

    pub fn extract_from_program<F: Clone + From<u32> + 'static>(
        mut self,
        program: &'static mut Program<F>,
    ) -> Result<ContractAbi, qed_common::Error> {
        let mut ctx = DefaultVisitorContext::<F, ()>::new(program);
        self.visit_program(&mut ctx).map_err(|_| qed_common::Error::CycleGraph)?;
        Ok(self.contract_abi)
    }

    pub fn extract_spec_compliant_abi<F: Clone + From<u32> + 'static>(
        self,
        program: &'static mut Program<F>,
    ) -> Result<SpecCompliantAbi, qed_common::Error> {
        let ctx = DefaultVisitorContext::<F, ()>::new(program);
        let mut spec_abi = SpecCompliantAbi::new("1.0.0".to_string());

        // First pass: collect all struct information
        let mut struct_map = std::collections::HashMap::new();
        
        // Collect all structs by iterating through all definitions in the arena
        for i in 0..ctx.program().defs.len() {
            let def_id = DefId::from(i);
            if let Some(struct_node) = ctx.definition(def_id).as_struct() {
                let struct_name = ctx.ident(struct_node.name).0.to_string();
                
                // Skip internal types
                if self.is_internal_type(&struct_name) {
                    continue;
                }
                
                let is_contract = self.has_contract_attr(struct_node, &ctx);
                let is_public = Self::is_public(&struct_node.visibility);
                
                // Include all structs (both public and private)
                // We'll let the user decide what should be in ABI
                if true {
                    let fields = struct_node
                        .fields
                        .iter()
                        .filter(|(_, field)| Self::is_public(&field.visibility))
                        .map(|(name, field)| FieldAbiSpec {
                            name: ctx.ident(*name).0.to_string(),
                            field_type: TypeAbiSpec::from_unchecked_type(&field.ty, &ctx),
                        })
                        .collect();

                    let mut struct_spec = StructAbiSpec {
                        name: struct_name.clone(),
                        is_contract,
                        fields,
                        functions: None,
                    };

                    // Find associated functions (only for contracts or when functions are explicitly associated)
                    let functions = self.find_impl_functions(&struct_name, &ctx);
                    if !functions.is_empty() {
                        struct_spec.functions = Some(functions);
                    }

                    struct_map.insert(struct_name, struct_spec);
                }
            }
        }

        // Add all structs to the ABI
        for struct_spec in struct_map.into_values() {
            spec_abi.add_struct(struct_spec);
        }

        Ok(spec_abi)
    }

    fn is_public(visibility: &Visibility) -> bool {
        matches!(visibility, Visibility::Public)
    }

    fn is_internal_type(&self, type_name: &str) -> bool {
        // Filter out internal types that shouldn't appear in the ABI
        // 1. Known internal types
        if matches!(type_name, "ContractMetadata" | "StorageRef") {
            return true;
        }
        
        // 2. Generated Ref types (e.g., ContractRef, OtherUserInfoRef)
        if type_name.ends_with("Ref") {
            return true;
        }
        
        false
    }

    fn is_internal_function(&self, function_name: &str) -> bool {
        // Filter out internal functions that shouldn't appear in the ABI
        matches!(function_name, "new" | "get" | "set")
    }

    fn has_contract_attr<F: Clone + From<u32>>(
        &self, 
        struct_node: &StructNode, 
        ctx: &DefaultVisitorContext<F, ()>
    ) -> bool {
        struct_node.attrs.iter().any(|attr| {
            let attr_name = ctx.ident(attr.name).0.as_str();
            attr_name == "contract" || attr_name == "storage"
        })
    }

    fn find_impl_functions<F: Clone + From<u32>>(
        &self,
        struct_name: &str,
        ctx: &DefaultVisitorContext<F, ()>
    ) -> Vec<FunctionAbiSpec> {
        let mut functions = Vec::new();
        
        // Look for impl blocks that implement this struct
        for i in 0..ctx.program().defs.len() {
            let def_id = DefId::from(i);
            if let Some(impl_node) = ctx.definition(def_id).as_impl() {
                // Check if this impl is for our target struct or its Ref version
                let impl_type_name = self.extract_type_name(&impl_node.ty, ctx);
                if impl_type_name == struct_name || 
                   impl_type_name == format!("{}Ref", struct_name) {
                    
                    // Extract functions from this impl block
                    for &function_def_id in &impl_node.body {
                        if let Some(function) = ctx.definition(function_def_id).as_function() {
                            let function_name = ctx.ident(function.name).0.to_string();
                            
                            // Skip internal functions and only include public functions
                            if Self::is_public(&function.visibility) && !self.is_internal_function(&function_name) {
                                let function_spec = self.extract_function_abi_spec(function, ctx);
                                functions.push(function_spec);
                            }
                        }
                    }
                }
            }
        }
        
        functions
    }

    fn extract_type_name<F: Clone + From<u32>>(
        &self,
        unchecked_type: &crate::UncheckedType,
        ctx: &DefaultVisitorContext<F, ()>
    ) -> String {
        match unchecked_type {
            crate::UncheckedType::Basic(identifier) => {
                ctx.ident(*identifier).0.to_string()
            },
            crate::UncheckedType::Path(path) => {
                self.extract_type_name(&path.target, ctx)
            },
            _ => "unknown".to_string(),
        }
    }

    fn extract_function_abi_spec<F: Clone + From<u32>>(
        &self, 
        function: &FunctionNode, 
        ctx: &DefaultVisitorContext<F, ()>
    ) -> FunctionAbiSpec {
        let params = function
            .parameters
            .iter()
            .map(|param| ParamAbiSpec {
                name: ctx.ident(param.name).0.to_string(),
                param_type: TypeAbiSpec::from_unchecked_type(&param.ty, ctx),
            })
            .collect();

        FunctionAbiSpec {
            name: ctx.ident(function.name).0.to_string(),
            params,
            return_type: vec![], // As per spec, always empty for now
        }
    }

    fn extract_function_abi<F: Clone + From<u32>>(&self, function: &FunctionNode, ctx: &DefaultVisitorContext<F, ()>) -> FunctionAbi {
        let parameters = function
            .parameters
            .iter()
            .map(|param| ParameterAbi {
                name: ctx.ident(param.name).0.to_string(),
                param_type: TypeAbi::from_unchecked_type(&param.ty, ctx),
                qualifier: format!("{:?}", param.qualifier),
            })
            .collect();

        let generic_parameters = function
            .generic_parameters
            .iter()
            .map(|generic| ctx.ident(generic.name).0.to_string())
            .collect();

        FunctionAbi {
            name: ctx.ident(function.name).0.to_string(),
            parameters,
            return_type: function.return_type.as_ref().map(|rt| TypeAbi::from_unchecked_type(rt, ctx)),
            generic_parameters,
            visibility: function.visibility.to_string(),
            is_const: function.qualifier.is_const,
        }
    }

    fn extract_struct_abi<F: Clone + From<u32>>(&self, struct_node: &StructNode, ctx: &DefaultVisitorContext<F, ()>) -> StructAbi {
        let fields = struct_node
            .fields
            .iter()
            .filter(|(_, field)| Self::is_public(&field.visibility))
            .map(|(name, field)| {
                (
                    ctx.ident(*name).0.to_string(),
                    FieldAbi {
                        name: ctx.ident(*name).0.to_string(),
                        field_type: TypeAbi::from_unchecked_type(&field.ty, ctx),
                        visibility: field.visibility.to_string(),
                    },
                )
            })
            .collect();

        let generic_parameters = struct_node
            .generic_parameters
            .iter()
            .map(|generic| ctx.ident(generic.name).0.to_string())
            .collect();

        StructAbi {
            name: ctx.ident(struct_node.name).0.to_string(),
            fields,
            generic_parameters,
            visibility: struct_node.visibility.to_string(),
        }
    }

    fn extract_enum_abi<F: Clone + From<u32>>(&self, enum_node: &EnumNode, ctx: &DefaultVisitorContext<F, ()>) -> EnumAbi {
        let variants = enum_node
            .variants
            .iter()
            .map(|variant| self.extract_variant_abi(variant, ctx))
            .collect();

        let generic_parameters = enum_node
            .generic_parameters
            .iter()
            .map(|generic| ctx.ident(generic.name).0.to_string())
            .collect();

        EnumAbi {
            name: ctx.ident(enum_node.name).0.to_string(),
            variants,
            generic_parameters,
            visibility: enum_node.visibility.to_string(),
        }
    }

    fn extract_variant_abi<F: Clone + From<u32>>(&self, variant: &EnumVariant, ctx: &DefaultVisitorContext<F, ()>) -> VariantAbi {
        match variant {
            EnumVariant::Basic(name) => VariantAbi {
                name: ctx.ident(*name).0.to_string(),
                variant_type: VariantTypeAbi::Basic,
            },
            EnumVariant::Tuple(name, types) => VariantAbi {
                name: ctx.ident(*name).0.to_string(),
                variant_type: VariantTypeAbi::Tuple(
                    types.iter().map(|t| TypeAbi::from_unchecked_type(t, ctx)).collect(),
                ),
            },
            EnumVariant::Struct(name, fields) => {
                let struct_fields = fields
                    .iter()
                    .filter(|(_, field)| Self::is_public(&field.visibility))
                    .map(|(field_name, field)| {
                        (
                            ctx.ident(*field_name).0.to_string(),
                            TypeAbi::from_unchecked_type(&field.ty, ctx),
                        )
                    })
                    .collect();

                VariantAbi {
                    name: ctx.ident(*name).0.to_string(),
                    variant_type: VariantTypeAbi::Struct(struct_fields),
                }
            }
        }
    }

    fn extract_trait_abi<F: Clone + From<u32>>(&self, trait_node: &TraitNode, ctx: &mut DefaultVisitorContext<F, ()>) -> Result<TraitAbi, qed_common::Error> {
        let mut methods = Vec::new();

        // Extract trait methods
        for &def_id in &trait_node.body {
            let definition = ctx.definition(def_id);
            if let Some(function) = definition.as_function() {
                if Self::is_public(&function.visibility) {
                    methods.push(self.extract_function_abi(function, ctx));
                }
            }
        }

        let associated_types = trait_node
            .associated_types
            .iter()
            .filter(|(_, assoc_type)| Self::is_public(&assoc_type.visibility))
            .map(|(name, assoc_type)| {
                (
                    ctx.ident(*name).0.to_string(),
                    AssociatedTypeAbi {
                        name: ctx.ident(*name).0.to_string(),
                        constraints: assoc_type
                            .constraints
                            .iter()
                            .map(|c| TypeAbi::from_unchecked_type(c, ctx))
                            .collect(),
                        visibility: assoc_type.visibility.to_string(),
                    },
                )
            })
            .collect();

        let generic_parameters = trait_node
            .generic_parameters
            .iter()
            .map(|generic| ctx.ident(generic.name).0.to_string())
            .collect();

        Ok(TraitAbi {
            name: ctx.ident(trait_node.name).0.to_string(),
            methods,
            associated_types,
            generic_parameters,
            visibility: trait_node.visibility.to_string(),
        })
    }
}

impl<F: Clone + From<u32> + 'static> AstVisitor<F, ()> for AbiExtractor {
    type Expr = ExprNode<F>;
    type Stmt = StmtNode;
    type Definition = DefinitionNode;
    type ExprResult = ();
    type StmtResult = ();
    type DefinitionResult = ();
    type Context = DefaultVisitorContext<'static, F, ()>;
    type Error = qed_common::Error;

    fn visit_function(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let function = ctx.definition(def_id).as_function().unwrap();
        
        if Self::is_public(&function.visibility) {
            let function_abi = self.extract_function_abi(function, ctx);
            self.contract_abi.add_function(function_abi);
        }
        
        Ok(())
    }

    fn visit_struct(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let struct_node = ctx.definition(def_id).as_struct().unwrap();
        
        if Self::is_public(&struct_node.visibility) {
            let struct_abi = self.extract_struct_abi(struct_node, ctx);
            self.contract_abi.add_struct(struct_abi);
        }
        
        Ok(())
    }

    fn visit_enum(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        let enum_node = ctx.definition(def_id).as_enum().unwrap();
        
        if Self::is_public(&enum_node.visibility) {
            let enum_abi = self.extract_enum_abi(enum_node, ctx);
            self.contract_abi.add_enum(enum_abi);
        }
        
        Ok(())
    }

    fn visit_trait(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        if Self::is_public(&ctx.definition(def_id).as_trait().unwrap().visibility) {
            let trait_node = ctx.definition(def_id).as_trait().unwrap().clone();
            let trait_abi = self.extract_trait_abi(&trait_node, ctx)?;
            self.contract_abi.add_trait(trait_abi);
        }
        
        Ok(())
    }

    // Default implementations for other visitor methods (they just return empty results)
    fn visit_use(&mut self, _def_id: DefId, _ctx: &mut Self::Context) -> Result<Self::DefinitionResult, Self::Error> {
        Ok(())
    }

    fn visit_impl(&mut self, _def_id: DefId, _ctx: &mut Self::Context) -> Result<Self::DefinitionResult, Self::Error> {
        Ok(())
    }

    fn visit_trait_impl(&mut self, _def_id: DefId, _ctx: &mut Self::Context) -> Result<Self::DefinitionResult, Self::Error> {
        Ok(())
    }

    fn visit_type_alias(&mut self, _def_id: DefId, _ctx: &mut Self::Context) -> Result<Self::DefinitionResult, Self::Error> {
        Ok(())
    }

    fn visit_const(&mut self, _def_id: DefId, _ctx: &mut Self::Context) -> Result<Self::DefinitionResult, Self::Error> {
        Ok(())
    }

    fn visit_path(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_value(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_binary(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_unary(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_call(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_member_call(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_cast(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_index_access(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_member_access(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_intrinsic_expr(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_lambda_function(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_block_expr(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_if_expr(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_tuple(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_tuple_access(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_match(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_parentheses(&mut self, _expr_id: ExprId, _ctx: &mut Self::Context) -> Result<Self::ExprResult, Self::Error> {
        Ok(())
    }

    fn visit_while(&mut self, _stmt_id: StmtId, _ctx: &mut Self::Context) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_for(&mut self, _stmt_id: StmtId, _ctx: &mut Self::Context) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_assignment(&mut self, _stmt_id: StmtId, _ctx: &mut Self::Context) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_variable(&mut self, _stmt_id: StmtId, _ctx: &mut Self::Context) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_return(&mut self, _stmt_id: StmtId, _ctx: &mut Self::Context) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }

    fn visit_intrinsic_stmt(&mut self, _stmt_id: StmtId, _ctx: &mut Self::Context) -> Result<Self::StmtResult, Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_abi_creation() {
        let contract_abi = ContractAbi::new("TestContract".to_string());
        assert_eq!(contract_abi.name, "TestContract");
        assert!(contract_abi.functions.is_empty());
        assert!(contract_abi.structs.is_empty());
        assert!(contract_abi.enums.is_empty());
        assert!(contract_abi.traits.is_empty());
    }

    #[test]
    fn test_contract_abi_json_serialization() {
        let mut contract_abi = ContractAbi::new("TestContract".to_string());
        
        // Add a simple function
        let function_abi = FunctionAbi {
            name: "test_function".to_string(),
            parameters: vec![],
            return_type: None,
            generic_parameters: vec![],
            visibility: "pub".to_string(),
            is_const: false,
        };
        contract_abi.add_function(function_abi);

        // Test JSON serialization
        let json = contract_abi.to_json().expect("Failed to serialize to JSON");
        assert!(json.contains("TestContract"));
        assert!(json.contains("test_function"));

        // Test JSON deserialization
        let deserialized = ContractAbi::from_json(&json).expect("Failed to deserialize from JSON");
        assert_eq!(deserialized.name, "TestContract");
        assert_eq!(deserialized.functions.len(), 1);
        assert!(deserialized.functions.contains_key("test_function"));
    }

    #[test]
    fn test_abi_extractor_creation() {
        let extractor = AbiExtractor::new("TestContract".to_string());
        assert_eq!(extractor.contract_abi.name, "TestContract");
    }
}