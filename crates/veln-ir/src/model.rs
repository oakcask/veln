use veln_ast::{BinaryOp, ContractKind, NodeId, PredicateRemainder, PrefixOp, Visibility};
use veln_core::{ContractObligationStatus, CoreContractCall, CoreModuleValueIdentity, CoreType};
use veln_source::SourceSpan;

#[derive(Clone, Debug, PartialEq)]
pub struct TypedProgram {
    pub functions: Vec<IrFunction>,
    pub schema_decoders: Vec<IrSchemaDecodeSpec>,
    pub schema_refinement_validators: Vec<IrSchemaRefinementValidationSpec>,
    pub module_values: Vec<IrModuleValueDecl>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaRefinementValidationSpec {
    pub schema_name: String,
    pub declaration_name: String,
    pub plan: IrSchemaRefinementPlan,
}

impl IrSchemaRefinementPlan {
    pub fn root_node(&self) -> &IrSchemaRefinementPlanNode {
        &self.nodes[self.root]
    }

    pub fn node(&self, index: usize) -> &IrSchemaRefinementPlanNode {
        &self.nodes[index]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrModuleValueDecl {
    pub node_id: NodeId,
    pub identity: CoreModuleValueIdentity,
    pub annotation: String,
    pub ty: CoreType,
    pub visibility: Visibility,
    pub initializer: IrExpr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaDecodeSpec {
    pub schema_name: String,
    pub declaration_name: String,
    pub function_name: String,
    pub fields: Vec<IrSchemaDecodeField>,
    pub validation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaDecodeField {
    pub name: String,
    pub width: u8,
    pub max_value: i64,
    pub little_endian: bool,
    pub predicate: Option<String>,
    pub length_field: Option<String>,
    pub length_multiple: Option<String>,
    pub repeat: Option<IrSchemaRepeat>,
    pub payload_schema: Option<Box<IrSchemaDecodeSpec>>,
    pub dispatch: Option<IrSchemaDecodeDispatch>,
    pub reserved_bits: Option<IrSchemaReservedBits>,
    pub variant_refinement: Option<IrSchemaVariantRefinement>,
    pub refinement_plan: Option<Box<IrSchemaRefinementPlan>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IrSchemaVariantRefinement {
    pub expected_variants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaRefinementPlan {
    pub root: usize,
    pub nodes: Vec<IrSchemaRefinementPlanNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IrSchemaRefinementPlanNode {
    Variant(IrSchemaVariantRefinement),
    Adt {
        expected_variants: Vec<String>,
        variants: Vec<IrSchemaRefinementVariant>,
    },
    Record(Vec<IrSchemaRefinementField>),
    Option(usize),
    Result {
        ok: Option<usize>,
        err: Option<usize>,
    },
    List(usize),
    Vec(usize),
    Dict(usize),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IrSchemaRefinementVariant {
    pub name: String,
    pub payloads: Vec<IrSchemaRefinementPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IrSchemaRefinementPayload {
    pub index: usize,
    pub path_segment: String,
    pub plan: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IrSchemaRefinementField {
    pub name: String,
    pub plan: usize,
}

impl IrSchemaDecodeField {
    pub fn new(name: String) -> Self {
        Self {
            name,
            width: 0,
            max_value: 0,
            little_endian: false,
            predicate: None,
            length_field: None,
            length_multiple: None,
            repeat: None,
            payload_schema: None,
            dispatch: None,
            reserved_bits: None,
            variant_refinement: None,
            refinement_plan: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaRepeat {
    pub count_field: String,
    pub width: u8,
    pub max_value: i64,
    pub little_endian: bool,
    pub reserved_bits: Option<IrSchemaReservedBits>,
    pub byte_view_length_field: Option<String>,
    pub payload_schema: Option<Box<IrSchemaDecodeSpec>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaReservedBits {
    pub bit_width: u8,
    pub expected_value: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaDecodeDispatch {
    pub tag_field: String,
    pub length_field: Option<String>,
    pub preserves_unknown: bool,
    pub cases: Vec<IrSchemaDecodeDispatchCase>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaDecodeDispatchCase {
    pub tag: i64,
    pub width: u8,
    pub little_endian: bool,
    pub reserved_bits: Option<IrSchemaReservedBits>,
    pub payload_schema: Option<Box<IrSchemaDecodeSpec>>,
    pub payload_schema_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrFunction {
    pub node_id: NodeId,
    pub name: String,
    pub visibility: Visibility,
    pub params: Vec<IrParam>,
    pub return_binding: Option<String>,
    pub return_type: CoreType,
    pub effects: Vec<String>,
    pub callsite: bool,
    pub contracts: Vec<IrContract>,
    pub body: IrCleanupRegion,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrCleanupRegion {
    pub statements: Vec<IrStmt>,
}

impl IrCleanupRegion {
    pub fn new(statements: Vec<IrStmt>) -> Self {
        Self { statements }
    }
}

impl std::ops::Deref for IrCleanupRegion {
    type Target = [IrStmt];

    fn deref(&self) -> &Self::Target {
        &self.statements
    }
}

impl std::ops::DerefMut for IrCleanupRegion {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.statements
    }
}

impl<'a> IntoIterator for &'a IrCleanupRegion {
    type Item = &'a IrStmt;
    type IntoIter = std::slice::Iter<'a, IrStmt>;

    fn into_iter(self) -> Self::IntoIter {
        self.statements.iter()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrParam {
    pub node_id: NodeId,
    pub name: String,
    pub ty: CoreType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrContract {
    pub node_id: NodeId,
    pub kind: ContractKind,
    pub predicate: String,
    pub remainder_spans: Vec<PredicateRemainder>,
    pub callsite_calls: Vec<CoreContractCall>,
    pub obligation_status: ContractObligationStatus,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrStmt {
    pub node_id: NodeId,
    pub kind: IrStmtKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrStmtKind {
    Let {
        name: String,
        ty: CoreType,
        value: IrExpr,
    },
    Expr {
        value: IrExpr,
    },
    Return {
        value: IrExpr,
    },
    Defer(IrDeferredBlock),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrDeferredBlock {
    pub captures: Vec<IrDeferredCapture>,
    pub body: Vec<IrStmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrDeferredCapture {
    pub name: String,
    pub ty: CoreType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrExpr {
    pub node_id: NodeId,
    pub ty: CoreType,
    pub kind: IrExprKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrExprKind {
    Local(String),
    ModuleValue(CoreModuleValueIdentity),
    BoolLiteral(bool),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    Unit,
    FunctionValue {
        name: String,
        callsite: bool,
    },
    ResultOk(Box<IrExpr>),
    ResultErr(Box<IrExpr>),
    OptionSome(Box<IrExpr>),
    OptionNone,
    ListNil,
    ListCons {
        head: Box<IrExpr>,
        tail: Box<IrExpr>,
    },
    AdtVariant {
        name: Vec<String>,
        payloads: Vec<IrExpr>,
    },
    Call {
        target: IrCallTarget,
        args: Vec<IrExpr>,
    },
    Perform {
        effect: String,
        operation: String,
        args: Vec<IrExpr>,
    },
    Handle {
        effect: String,
        providers: Vec<IrHandlerProvider>,
        context_args: Vec<IrExpr>,
        body: Box<IrExpr>,
    },
    FieldAccess {
        base: Box<IrExpr>,
        field: String,
    },
    Try(Box<IrExpr>),
    Record(Vec<IrRecordField>),
    Dict(Vec<IrDictEntry>),
    List(Vec<IrExpr>),
    Match {
        scrutinee: Box<IrExpr>,
        arms: Vec<IrMatchArm>,
    },
    CleanupRegion {
        region: IrCleanupRegion,
    },
    Prefix {
        op: PrefixOp,
        expr: Box<IrExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<IrExpr>,
        right: Box<IrExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrHandlerProvider {
    pub operation: String,
    pub function: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrCallTarget {
    Function(String),
    SchemaDecode(String),
    SchemaDecodeStep(String),
    SchemaNeutralDecode(String),
    SchemaNeutralEncode(String),
    SchemaEncode(String),
    SchemaEncodeStep(String),
    SchemaValidate(String),
    SchemaRefinementValidate(String),
    StdioBuiltin(String),
    ConcurrencyBuiltin(String),
    StandardLibraryBuiltin(String),
    PreludeBuiltin(String),
    CallbackBoundary {
        target: IrCallbackTarget,
        callsite: Box<IrExpr>,
    },
    CallsiteValue {
        name: String,
        callsite: Box<IrExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrCallbackTarget {
    Function(String),
    ConcurrencyBuiltin(String),
    StandardLibraryBuiltin(String),
    PreludeBuiltin(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrRecordField {
    pub node_id: NodeId,
    pub name: String,
    pub value: IrExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrDictEntry {
    pub node_id: NodeId,
    pub key: IrExpr,
    pub value: IrExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrMatchArm {
    pub node_id: NodeId,
    pub pattern: IrPattern,
    pub value: IrExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrPattern {
    pub node_id: NodeId,
    pub kind: IrPatternKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrPatternKind {
    Wildcard,
    Binding(String),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    BoolLiteral(bool),
    Unit,
    Record(Vec<IrPatternField>),
    Constructor {
        name: Vec<String>,
        args: Vec<IrPattern>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrPatternField {
    pub node_id: NodeId,
    pub name: String,
    pub pattern: IrPattern,
}
