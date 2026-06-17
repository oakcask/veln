use veln_ast::{BinaryOp, ContractKind, NodeId, PrefixOp, SchemaMappingSelectorOp, Visibility};
use veln_core::{ContractObligationStatus, CoreType};
use veln_source::SourceSpan;

#[derive(Clone, Debug, PartialEq)]
pub struct TypedProgram {
    pub functions: Vec<IrFunction>,
    pub schema_decoders: Vec<IrSchemaDecodeSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaDecodeSpec {
    pub schema_name: String,
    pub function_name: String,
    pub fields: Vec<IrSchemaDecodeField>,
    pub validation: Option<String>,
    pub mapping: Vec<IrSchemaDecodeMappingField>,
    pub mapping_alternatives: Vec<IrSchemaDecodeMapping>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaDecodeField {
    pub name: String,
    pub width: u8,
    pub max_value: i64,
    pub little_endian: bool,
    pub flag_type: String,
    pub predicate: Option<String>,
    pub length_field: Option<String>,
    pub repeat: Option<IrSchemaRepeat>,
    pub dispatch: Option<IrSchemaDecodeDispatch>,
    pub reserved_bits: Option<IrSchemaReservedBits>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaRepeat {
    pub count_field: String,
    pub width: u8,
    pub max_value: i64,
    pub little_endian: bool,
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
    pub payload_schema: Option<Box<IrSchemaDecodeSpec>>,
    pub payload_schema_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaDecodeMappingField {
    pub target: String,
    pub source: String,
    pub expr: IrSchemaDecodeMappingExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaDecodeMapping {
    pub selector: Option<IrSchemaDecodeMappingSelector>,
    pub fields: Vec<IrSchemaDecodeMappingField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaDecodeMappingSelector {
    pub field: String,
    pub op: SchemaMappingSelectorOp,
    pub value: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrSchemaDecodeMappingExpr {
    Field(String),
    Literal(i64),
    FieldAccess {
        base: Box<IrSchemaDecodeMappingExpr>,
        field: String,
    },
    Record(Vec<IrSchemaDecodeMappingRecordField>),
    Constructor {
        name: Vec<String>,
        args: Vec<IrSchemaDecodeMappingExpr>,
    },
    Converter {
        function: String,
        arg: Box<IrSchemaDecodeMappingExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<IrSchemaDecodeMappingExpr>,
        right: Box<IrSchemaDecodeMappingExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrSchemaDecodeMappingRecordField {
    pub name: String,
    pub expr: IrSchemaDecodeMappingExpr,
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
    pub contracts: Vec<IrContract>,
    pub body: Vec<IrStmt>,
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
    BoolLiteral(bool),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    Unit,
    FunctionValue(String),
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
pub enum IrCallTarget {
    Function(String),
    CodecDecode { function: String, codec: String },
    SchemaDecode(String),
    SchemaDecodeStep(String),
    SchemaEncode(String),
    SchemaEncodeStep(String),
    SchemaValidate(String),
    StdioBuiltin(String),
    ConcurrencyBuiltin(String),
    StandardLibraryBuiltin(String),
    PreludeBuiltin(String),
    Value(String),
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
