use super::*;

#[derive(Default)]
pub(super) struct LoweredItems {
    pub(super) aliases: Vec<PublicAlias>,
    pub(super) effects: Vec<EffectDecl>,
    pub(super) handlers: Vec<HandlerDecl>,
    pub(super) types: Vec<TypeDecl>,
    pub(super) schemas: Vec<SchemaDecl>,
    pub(super) module_values: Vec<ModuleValueDecl>,
    pub(super) functions: Vec<Function>,
}

impl AstBuilder {
    pub(super) fn lower_items(
        &mut self,
        syntax_items: &[SyntaxItem],
        module_name: &Option<String>,
        package_name: &Option<String>,
    ) -> LoweredItems {
        let mut items = LoweredItems::default();
        for item in syntax_items {
            self.lower_item(item, module_name, package_name, &mut items);
        }
        items
    }

    fn lower_item(
        &mut self,
        item: &SyntaxItem,
        module_name: &Option<String>,
        package_name: &Option<String>,
        lowered: &mut LoweredItems,
    ) {
        match item {
            SyntaxItem::ModuleValue(value) => {
                lowered.module_values.push(self.lower_module_value(
                    value,
                    module_name.clone(),
                    package_name.clone(),
                ));
            }
            SyntaxItem::Function(function) => {
                self.lower_function_item(function, module_name, package_name, lowered);
            }
            SyntaxItem::Effect(effect) => {
                lowered.effects.push(self.lower_effect_decl(
                    effect,
                    module_name.clone(),
                    package_name.clone(),
                ));
            }
            SyntaxItem::Handler(handler) => {
                self.lower_handler_item(handler, module_name, package_name, lowered);
            }
            SyntaxItem::Type(type_decl) => {
                lowered.types.push(self.lower_type_decl(
                    type_decl,
                    module_name.clone(),
                    package_name.clone(),
                ));
            }
            SyntaxItem::Schema(schema) => {
                lowered.schemas.push(self.lower_schema_decl(
                    schema,
                    module_name.clone(),
                    package_name.clone(),
                ));
            }
            SyntaxItem::PublicAlias(alias) => {
                lowered.aliases.push(self.lower_public_alias(
                    alias,
                    module_name.clone(),
                    package_name.clone(),
                ));
            }
        }
    }

    fn lower_function_item(
        &mut self,
        function: &SyntaxFunction,
        module_name: &Option<String>,
        package_name: &Option<String>,
        lowered: &mut LoweredItems,
    ) {
        lowered.functions.push(self.lower_function(
            function,
            module_name.clone(),
            package_name.clone(),
        ));
    }

    fn lower_handler_item(
        &mut self,
        handler: &SyntaxHandlerDecl,
        module_name: &Option<String>,
        package_name: &Option<String>,
        lowered: &mut LoweredItems,
    ) {
        lowered.handlers.push(self.lower_handler_decl(
            handler,
            module_name.clone(),
            package_name.clone(),
        ));
    }
}
