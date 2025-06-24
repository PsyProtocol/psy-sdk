use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{UncheckedType, VisitorContext};

// New spec-compliant ABI structures
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpecCompliantAbi {
    pub version: String,
    pub structs: Vec<StructAbiSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StructAbiSpec {
    pub name: String,
    pub is_contract: bool,
    pub fields: Vec<FieldAbiSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<FunctionAbiSpec>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FieldAbiSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: TypeAbiSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FunctionAbiSpec {
    pub name: String,
    pub params: Vec<ParamAbiSpec>,
    #[serde(rename = "return")]
    pub return_type: Vec<TypeAbiSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParamAbiSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: TypeAbiSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TypeAbiSpec {
    Basic(String),
    Array {
        #[serde(rename = "type")]
        type_name: String,
        inner_type: String,
        length: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractAbi {
    pub name: String,
    pub functions: IndexMap<String, FunctionAbi>,
    pub structs: IndexMap<String, StructAbi>,
    pub enums: IndexMap<String, EnumAbi>,
    pub traits: IndexMap<String, TraitAbi>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionAbi {
    pub name: String,
    pub parameters: Vec<ParameterAbi>,
    pub return_type: Option<TypeAbi>,
    pub generic_parameters: Vec<String>,
    pub visibility: String,
    pub is_const: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterAbi {
    pub name: String,
    pub param_type: TypeAbi,
    pub qualifier: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructAbi {
    pub name: String,
    pub fields: IndexMap<String, FieldAbi>,
    pub generic_parameters: Vec<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldAbi {
    pub name: String,
    pub field_type: TypeAbi,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumAbi {
    pub name: String,
    pub variants: Vec<VariantAbi>,
    pub generic_parameters: Vec<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantAbi {
    pub name: String,
    pub variant_type: VariantTypeAbi,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VariantTypeAbi {
    Basic,
    Tuple(Vec<TypeAbi>),
    Struct(IndexMap<String, TypeAbi>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitAbi {
    pub name: String,
    pub methods: Vec<FunctionAbi>,
    pub associated_types: IndexMap<String, AssociatedTypeAbi>,
    pub generic_parameters: Vec<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssociatedTypeAbi {
    pub name: String,
    pub constraints: Vec<TypeAbi>,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeAbi {
    pub type_name: String,
    pub generic_args: Vec<TypeAbi>,
    pub is_array: bool,
    pub array_size: Option<u32>,
    pub is_tuple: bool,
    pub tuple_elements: Vec<TypeAbi>,
}

impl ContractAbi {
    pub fn new(name: String) -> Self {
        Self {
            name,
            functions: IndexMap::new(),
            structs: IndexMap::new(),
            enums: IndexMap::new(),
            traits: IndexMap::new(),
        }
    }

    pub fn add_function(&mut self, function: FunctionAbi) {
        self.functions.insert(function.name.clone(), function);
    }

    pub fn add_struct(&mut self, struct_abi: StructAbi) {
        self.structs.insert(struct_abi.name.clone(), struct_abi);
    }

    pub fn add_enum(&mut self, enum_abi: EnumAbi) {
        self.enums.insert(enum_abi.name.clone(), enum_abi);
    }

    pub fn add_trait(&mut self, trait_abi: TraitAbi) {
        self.traits.insert(trait_abi.name.clone(), trait_abi);
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl TypeAbi {
    pub fn from_unchecked_type<F: Clone + From<u32>>(unchecked_type: &UncheckedType, ctx: &crate::DefaultVisitorContext<F, ()>) -> Self {
        match unchecked_type {
            UncheckedType::Basic(identifier) => Self {
                type_name: ctx.ident(*identifier).0.to_string(),
                generic_args: Vec::new(),
                is_array: false,
                array_size: None,
                is_tuple: false,
                tuple_elements: Vec::new(),
            },
            UncheckedType::Generic(identifier, generics, _) => Self {
                type_name: ctx.ident(*identifier).0.to_string(),
                generic_args: generics.iter().map(|g| Self::from_unchecked_type(g, ctx)).collect(),
                is_array: false,
                array_size: None,
                is_tuple: false,
                tuple_elements: Vec::new(),
            },
            UncheckedType::Array(element_type, size, _) => Self {
                type_name: "Array".to_string(),
                generic_args: vec![Self::from_unchecked_type(element_type, ctx)],
                is_array: true,
                array_size: Some(*size),
                is_tuple: false,
                tuple_elements: Vec::new(),
            },
            UncheckedType::Tuple(elements, _) => Self {
                type_name: "Tuple".to_string(),
                generic_args: Vec::new(),
                is_array: false,
                array_size: None,
                is_tuple: true,
                tuple_elements: elements.iter().map(|e| Self::from_unchecked_type(e, ctx)).collect(),
            },
            UncheckedType::Path(path) => {
                // For path types, recursively extract the target type
                Self::from_unchecked_type(&path.target, ctx)
            },
            UncheckedType::FunctionSignature(_, _) => Self {
                type_name: "Function".to_string(),
                generic_args: Vec::new(),
                is_array: false,
                array_size: None,
                is_tuple: false,
                tuple_elements: Vec::new(),
            },
            UncheckedType::TraitCast(base_type, trait_type, _) => {
                let mut base = Self::from_unchecked_type(base_type, ctx);
                base.type_name = format!("{} as {}", base.type_name, Self::from_unchecked_type(trait_type, ctx).type_name);
                base
            },
            UncheckedType::Const(value, _) => Self {
                type_name: format!("const {}", value),
                generic_args: Vec::new(),
                is_array: false,
                array_size: None,
                is_tuple: false,
                tuple_elements: Vec::new(),
            },
            UncheckedType::Unknown => Self {
                type_name: "unknown".to_string(),
                generic_args: Vec::new(),
                is_array: false,
                array_size: None,
                is_tuple: false,
                tuple_elements: Vec::new(),
            },
        }
    }
}

impl SpecCompliantAbi {
    pub fn new(version: String) -> Self {
        Self {
            version,
            structs: Vec::new(),
        }
    }

    pub fn add_struct(&mut self, struct_spec: StructAbiSpec) {
        self.structs.push(struct_spec);
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl TypeAbiSpec {
    pub fn from_unchecked_type<F: Clone + From<u32>>(
        unchecked_type: &UncheckedType, 
        ctx: &crate::DefaultVisitorContext<F, ()>
    ) -> Self {
        match unchecked_type {
            UncheckedType::Basic(identifier) => {
                TypeAbiSpec::Basic(ctx.ident(*identifier).0.to_string())
            },
            UncheckedType::Array(element_type, size, _) => {
                let inner_type = match Self::from_unchecked_type(element_type, ctx) {
                    TypeAbiSpec::Basic(name) => name,
                    TypeAbiSpec::Array { inner_type, .. } => inner_type,
                };
                TypeAbiSpec::Array {
                    type_name: "Array".to_string(),
                    inner_type,
                    length: *size,
                }
            },
            UncheckedType::Generic(identifier, _generics, _) => {
                TypeAbiSpec::Basic(ctx.ident(*identifier).0.to_string())
            },
            UncheckedType::Path(path) => {
                Self::from_unchecked_type(&path.target, ctx)
            },
            _ => TypeAbiSpec::Basic("unknown".to_string()),
        }
    }
}