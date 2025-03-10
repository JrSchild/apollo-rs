// schema
mod schema;

// leaf nodes
mod enum_;
mod scalar;
mod union_;

// composite nodes
mod directive;
mod input_object;
mod interface;
mod object;

// executable definitions
mod operation;

mod unused_variable;

use std::collections::HashMap;
use std::sync::Arc;
use apollo_parser::SyntaxNode;

use crate::{
    database::db::Upcast, ApolloDiagnostic, AstDatabase, DocumentDatabase, HirDatabase,
    diagnostics::UniqueArgument,
    hir,
    InputDatabase,
};

#[salsa::query_group(ValidationStorage)]
pub trait ValidationDatabase:
    Upcast<dyn DocumentDatabase> + InputDatabase + AstDatabase + HirDatabase
{
    fn validate(&self) -> Vec<ApolloDiagnostic>;
    fn validate_schema(&self) -> Vec<ApolloDiagnostic>;
    fn validate_scalar(&self) -> Vec<ApolloDiagnostic>;
    fn validate_enum(&self) -> Vec<ApolloDiagnostic>;
    fn validate_union(&self) -> Vec<ApolloDiagnostic>;
    fn validate_interface(&self) -> Vec<ApolloDiagnostic>;
    fn validate_directive(&self) -> Vec<ApolloDiagnostic>;
    fn validate_input_object(&self) -> Vec<ApolloDiagnostic>;
    fn validate_object(&self) -> Vec<ApolloDiagnostic>;
    fn validate_operation(&self) -> Vec<ApolloDiagnostic>;
    fn validate_unused_variable(&self) -> Vec<ApolloDiagnostic>;

    fn check_directive_definition(&self, directive: hir::DirectiveDefinition) -> Vec<ApolloDiagnostic>;
    fn check_object_type_definition(&self, object_type: hir::ObjectTypeDefinition) -> Vec<ApolloDiagnostic>;
    fn check_interface_type_definition(&self, object_type: hir::InterfaceTypeDefinition) -> Vec<ApolloDiagnostic>;
    fn check_union_type_definition(&self, object_type: hir::UnionTypeDefinition) -> Vec<ApolloDiagnostic>;
    fn check_enum_type_definition(&self, object_type: hir::EnumTypeDefinition) -> Vec<ApolloDiagnostic>;
    fn check_input_object_type_definition(&self, object_type: hir::InputObjectTypeDefinition) -> Vec<ApolloDiagnostic>;
    fn check_schema_definition(&self, object_type: hir::SchemaDefinition) -> Vec<ApolloDiagnostic>;
    fn check_selection_set(&self, selection_set: hir::SelectionSet) -> Vec<ApolloDiagnostic>;
    fn check_arguments_definition(&self, arguments_def: hir::ArgumentsDefinition) -> Vec<ApolloDiagnostic>;
    fn check_field_definition(&self, field: hir::FieldDefinition) -> Vec<ApolloDiagnostic>;
    fn check_input_values(&self, input_values: Arc<Vec<hir::InputValueDefinition>>) -> Vec<ApolloDiagnostic>;
    fn check_db_definitions(&self, definitions: Arc<Vec<hir::Definition>>) -> Vec<ApolloDiagnostic>;
    fn check_directive(&self, schema: hir::Directive) -> Vec<ApolloDiagnostic>;
    fn check_arguments(&self, schema: Vec<hir::Argument>) -> Vec<ApolloDiagnostic>;
    fn check_field(&self, field: Arc<hir::Field>) -> Vec<ApolloDiagnostic>;
}

pub fn check_directive_definition(db: &dyn ValidationDatabase, directive: hir::DirectiveDefinition) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(db.check_arguments_definition(directive.arguments));

    diagnostics
}

pub fn check_object_type_definition(db: &dyn ValidationDatabase, object_type: hir::ObjectTypeDefinition) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    for field in object_type.fields_definition() {
        diagnostics.extend(db.check_field_definition(field.clone()));
    }

    diagnostics
}

pub fn check_interface_type_definition(db: &dyn ValidationDatabase, interface_type: hir::InterfaceTypeDefinition) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    for field in interface_type.fields_definition() {
        diagnostics.extend(db.check_field_definition(field.clone()));
    }

    diagnostics
}

pub fn check_union_type_definition(_db: &dyn ValidationDatabase, _union_type: hir::UnionTypeDefinition) -> Vec<ApolloDiagnostic> {
    vec![]
}

pub fn check_enum_type_definition(_db: &dyn ValidationDatabase, _enum_type: hir::EnumTypeDefinition) -> Vec<ApolloDiagnostic> {
    vec![]
}

pub fn check_input_object_type_definition(_db: &dyn ValidationDatabase, _input_object_type: hir::InputObjectTypeDefinition) -> Vec<ApolloDiagnostic> {
    // Not checking the `input_values` here as those are checked as fields elsewhere.
    vec![]
}

pub fn check_schema_definition(db: &dyn ValidationDatabase, schema_def: hir::SchemaDefinition) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    for directive in schema_def.directives() {
        diagnostics.extend(db.check_directive(directive.clone()));
    }

    diagnostics
}

pub fn check_selection_set(db: &dyn ValidationDatabase, selection_set: hir::SelectionSet) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    for selection in selection_set.selection.iter() {
        match selection {
            hir::Selection::Field(field) => {
                diagnostics.extend(db.check_field(Arc::clone(field)));
            },
            hir::Selection::FragmentSpread(_) | hir::Selection::InlineFragment(_) => {
                // no diagnostics yet
            },
        }
    }

    diagnostics
}

pub fn check_arguments_definition(db: &dyn ValidationDatabase, arguments_def: hir::ArgumentsDefinition) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(db.check_input_values(arguments_def.input_values));

    diagnostics
}

pub fn check_field_definition(db: &dyn ValidationDatabase, field: hir::FieldDefinition) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    for directive in field.directives() {
        diagnostics.extend(db.check_directive(directive.clone()));
    }

    diagnostics.extend(db.check_arguments_definition(field.arguments));

    diagnostics
}

pub fn check_input_values(db: &dyn ValidationDatabase, input_values: Arc<Vec<hir::InputValueDefinition>>) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: HashMap<&str, &hir::InputValueDefinition> = HashMap::new();

    for input_value in input_values.iter() {
        let name = input_value.name();
        if let Some(prev_arg) = seen.get(name) {
            let prev_offset: usize = prev_arg.ast_node(db.upcast()).unwrap().text_range().start().into();
            let prev_node_len: usize = prev_arg.ast_node(db.upcast()).unwrap().text_range().len().into();

            let current_offset: usize = input_value.ast_node(db.upcast()).unwrap().text_range().start().into();
            let current_node_len: usize = input_value.ast_node(db.upcast()).unwrap().text_range().len().into();

            diagnostics.push(ApolloDiagnostic::UniqueArgument(UniqueArgument {
                name: name.into(),
                src: db.input(),
                original_definition: (prev_offset, prev_node_len).into(),
                redefined_definition: (current_offset, current_node_len).into(),
                help: Some(format!("`{name}` argument must only be defined once.")),
            }));
        } else {
            seen.insert(name, input_value);
        }
    }

    diagnostics
}

pub fn check_db_definitions(db: &dyn ValidationDatabase, definitions: Arc<Vec<hir::Definition>>) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    for definition in definitions.iter() {
        for directive in definition.directives() {
            diagnostics.extend(db.check_directive(directive.clone()));
        }

        use hir::Definition::*;
        match definition {
            OperationDefinition(def) => {
                diagnostics.extend(db.check_selection_set(def.selection_set().clone()));
            },
            FragmentDefinition(def) => {
                diagnostics.extend(db.check_selection_set(def.selection_set().clone()));
            },
            DirectiveDefinition(def) => {
                diagnostics.extend(db.check_directive_definition(def.clone()));
            },
            ScalarTypeDefinition(_def) => {
            },
            ObjectTypeDefinition(def) => {
                diagnostics.extend(db.check_object_type_definition(def.clone()));
            },
            InterfaceTypeDefinition(def) => {
                diagnostics.extend(db.check_interface_type_definition(def.clone()));
            },
            UnionTypeDefinition(def) => {
                diagnostics.extend(db.check_union_type_definition(def.clone()));
            },
            EnumTypeDefinition(def) => {
                diagnostics.extend(db.check_enum_type_definition(def.clone()));
            },
            InputObjectTypeDefinition(def) => {
                diagnostics.extend(db.check_input_object_type_definition(def.clone()));
            },
            SchemaDefinition(def) => {
                diagnostics.extend(db.check_schema_definition(def.clone()));
            },
        }
    }

    diagnostics
}

pub fn check_field(db: &dyn ValidationDatabase, field: Arc<hir::Field>) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    for directive in field.directives.iter() {
        diagnostics.extend(db.check_directive(directive.clone()));
    }
    diagnostics.extend(db.check_arguments(field.arguments().to_vec()));

    diagnostics
}

pub fn check_directive(db: &dyn ValidationDatabase, directive: hir::Directive) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(db.check_arguments(directive.arguments().to_vec()));

    diagnostics
}

pub fn check_arguments(db: &dyn ValidationDatabase, arguments: Vec<hir::Argument>) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: HashMap<&str, &hir::Argument> = HashMap::new();

    for argument in &arguments {
        let name = argument.name();
        if let Some(prev_arg) = seen.get(name) {
            let prev_offset: usize = prev_arg.ast_node(db.upcast()).text_range().start().into();
            let prev_node_len: usize = prev_arg.ast_node(db.upcast()).text_range().len().into();

            let current_offset: usize = argument.ast_node(db.upcast()).text_range().start().into();
            let current_node_len: usize = argument.ast_node(db.upcast()).text_range().len().into();

            diagnostics.push(ApolloDiagnostic::UniqueArgument(UniqueArgument {
                name: name.into(),
                src: db.input(),
                original_definition: (prev_offset, prev_node_len).into(),
                redefined_definition: (current_offset, current_node_len).into(),
                help: Some(format!("`{name}` argument must only be provided once.")),
            }));
        } else {
            seen.insert(name, argument);
        }
    }

    diagnostics
}

pub fn validate(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(db.syntax_errors());

    diagnostics.extend(db.validate_schema());

    diagnostics.extend(db.validate_scalar());
    diagnostics.extend(db.validate_enum());
    diagnostics.extend(db.validate_union());

    diagnostics.extend(db.validate_interface());
    diagnostics.extend(db.validate_directive());
    diagnostics.extend(db.validate_input_object());
    diagnostics.extend(db.validate_object());
    diagnostics.extend(db.validate_operation());

    diagnostics.extend(db.validate_unused_variable());

    diagnostics.extend(db.check_db_definitions(db.db_definitions()));

    diagnostics
}

pub fn validate_schema(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    schema::check(db)
}

pub fn validate_scalar(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    scalar::check(db)
}

pub fn validate_enum(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    enum_::check(db)
}

pub fn validate_union(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    union_::check(db)
}

pub fn validate_interface(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    interface::check(db)
}

pub fn validate_directive(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    directive::check(db)
}

pub fn validate_input_object(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    input_object::check(db)
}

pub fn validate_object(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    object::check(db)
}

pub fn validate_operation(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    operation::check(db)
}

pub fn validate_unused_variable(db: &dyn ValidationDatabase) -> Vec<ApolloDiagnostic> {
    unused_variable::check(db)
}

// #[salsa::query_group(ValidationStorage)]
// pub trait Validation: Document + Inputs + DocumentParser + Definitions {
//     fn validate(&self) -> Arc<Vec<ApolloDiagnostic>>;
// }
//
// pub fn validate(db: &dyn Validation) -> Arc<Vec<ApolloDiagnostic>> {
//     let mut diagnostics = Vec::new();
//     diagnostics.extend(schema::check(db));
//
//     Arc::new(diagnostics)
// }

#[derive(Debug, Eq)]
struct ValidationSet {
    name: String,
    node: SyntaxNode,
}

impl std::hash::Hash for ValidationSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for ValidationSet {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
