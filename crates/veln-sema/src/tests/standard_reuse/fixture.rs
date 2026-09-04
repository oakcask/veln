use std::sync::{Mutex, MutexGuard};

use super::*;
use veln_ast::{UseDecl, lower_surface_ast_with_module_identity};
use veln_diagnostics::diagnostic_to_json;
use veln_source::TextRange;

static STANDARD_REUSE_TEST_LOCK: Mutex<()> = Mutex::new(());

pub(super) struct AppCase {
    pub(super) name: &'static str,
    pub(super) module: SurfaceModule,
}

impl AppCase {
    pub(super) fn with_module_name(mut self, module_name: &str) -> Self {
        set_module_name(&mut self.module, module_name);
        self
    }
}

pub(super) fn app_case(name: &'static str, path: &str, text: &str) -> AppCase {
    let mut module = module_with_identity(path, text, "app");
    let span = SourceFile::new(path, text).span(TextRange::new(0, 0));
    module
        .uses
        .push(UseDecl::implicit_standard_prelude("app".to_string(), span));
    AppCase { name, module }
}

pub(super) fn set_module_name(module: &mut SurfaceModule, module_name: &str) {
    let module_name = Some(module_name.to_string());
    for decl in &mut module.uses {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.aliases {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.effects {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.handlers {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.schemas {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.types {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.functions {
        decl.module_name = module_name.clone();
    }
}

pub(super) fn standard_module() -> SurfaceModule {
    module_with_identity(
        "prelude.veln",
        concat!(
            "pub effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn provide(offset: Int) -> Int\n",
            "  offset + 1\n",
            "end\n",
            "\n",
            "pub handler ask(offset: Int) handles Ask\n",
            "  value() => provide(offset)\n",
            "end\n",
            "\n",
            "type PayloadShape\n",
            "  PayloadShape(Int)\n",
            "end\n",
            "\n",
            "pub type SharedPayload = PayloadShape\n",
            "\n",
            "schema Packet\n",
            "  value: Int\n",
            "end\n",
            "\n",
            "pub schema PublicPacket = Packet\n",
            "\n",
            "fn identity(value: SharedPayload) -> SharedPayload\n",
            "  value\n",
            "end\n",
            "\n",
            "pub fn answer = identity\n",
        ),
        "std::prelude",
    )
}

pub(super) fn standard_modules_with_imports() -> SurfaceModule {
    merge_modules(vec![
        module_with_identity(
            "support.veln",
            concat!("type PayloadShape\n", "  PayloadShape(Int)\n", "end\n",),
            "std::support",
        ),
        module_with_identity(
            "prelude.veln",
            concat!(
                "use std::support\n",
                "\n",
                "pub type ImportedPayload = support::PayloadShape\n",
            ),
            "std::prelude",
        ),
    ])
}

pub(super) fn standard_modules_with_unused_module() -> SurfaceModule {
    merge_modules(vec![
        standard_module(),
        module_with_identity(
            "unused.veln",
            concat!(
                "pub fn unused(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
            ),
            "std::unused",
        ),
    ])
}

pub(super) fn standard_modules_with_extra_module() -> SurfaceModule {
    merge_modules(vec![standard_module(), extra_standard_module()])
}

pub(super) fn extra_standard_module() -> SurfaceModule {
    module_with_identity(
        "extra.veln",
        concat!(
            "pub fn extra_answer(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
        ),
        "std::extra",
    )
}

pub(super) fn unrelated_annotated_standard_module(function_count: usize) -> SurfaceModule {
    let mut text = String::new();
    for index in 0..function_count {
        text.push_str(&format!(
            "pub fn unrelated_{index}(value: Int) -> Int\n  value + {index}\nend\n\n"
        ));
    }
    module_with_identity("unrelated.veln", &text, "std::unrelated")
}

pub(super) fn module_with_identity(path: &str, text: &str, module_name: &str) -> SurfaceModule {
    let source = SourceFile::new(path, text);
    lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        module_name.to_string(),
        source.span(TextRange::new(0, 0)),
    )
}

pub(super) fn merge_modules(modules: Vec<SurfaceModule>) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for module in modules {
        merged.uses.extend(module.uses);
        merged.aliases.extend(module.aliases);
        merged.effects.extend(module.effects);
        merged.handlers.extend(module.handlers);
        merged.schemas.extend(module.schemas);
        merged.types.extend(module.types);
        merged.functions.extend(module.functions);
        merged.invalid_names.extend(module.invalid_names);
    }
    merged
}

pub(super) fn assert_same_analysis(
    name: &str,
    uncached: (Vec<Diagnostic>, LoweredSurfaceModule),
    cached: (Vec<Diagnostic>, LoweredSurfaceModule),
) {
    assert_eq!(
        diagnostic_json(&cached.0),
        diagnostic_json(&uncached.0),
        "{name}: semantic diagnostics differ"
    );
    assert_eq!(
        diagnostic_json(&cached.1.diagnostics),
        diagnostic_json(&uncached.1.diagnostics),
        "{name}: checked diagnostics differ"
    );
    assert_eq!(
        format!("{:?}", cached.1.core),
        format!("{:?}", uncached.1.core),
        "{name}: lowered core differs"
    );
    assert_eq!(
        format!("{:?}", cached.1.ir),
        format!("{:?}", uncached.1.ir),
        "{name}: lowered IR differs"
    );
}

pub(super) fn checked_json(
    module: &SurfaceModule,
    reusable: &ReusableStandardEnvironment,
) -> Vec<String> {
    let (_, checked) = check_project_surface_module_with_standard_environment(module, reusable);
    checked
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
        .collect()
}

pub(super) fn assert_core_function(core: &veln_core::CheckedProgram, name: &str) {
    assert!(
        core.functions.iter().any(|function| function.name == name),
        "missing core function {name}: {:#?}",
        core.functions
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>()
    );
}

pub(super) fn diagnostic_json(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
        .collect()
}

pub(super) fn standard_reuse_test_lock() -> MutexGuard<'static, ()> {
    STANDARD_REUSE_TEST_LOCK
        .lock()
        .expect("standard reuse tests should not poison their counter lock")
}
