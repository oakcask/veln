:- use_module(library(filesex)).
:- use_module(library(readutil)).

:- initialization(main, main).

main(Argv) :-
    (   argv_member("--grammar", Argv)
    ->  print_grammar
    ;   argv_member("--check", Argv)
    ->  check_fixtures,
        argv_after("--check", Argv, AcceptedPaths),
        validate_paths(accepted, AcceptedPaths)
    ;   usage,
        halt(2)
    ).

argv_member(Text, Argv) :-
    member(Value, Argv),
    same_text(Value, Text).

same_text(Value, Text) :-
    string(Value),
    !,
    Value = Text.
same_text(Value, Text) :-
    atom(Value),
    atom_string(Value, Text).

usage :-
    writeln("usage: swipl -q -s docs/specification/source-surface-executable.pl -- --check [accepted-source.veln ...]|--grammar").

check_fixtures :-
    validate_fixtures(accepted, accepted_fixture),
    validate_fixtures(rejected, rejected_fixture).

accepted_fixture(Path) :-
    fixture_path(accepted, Path).

rejected_fixture(Path) :-
    fixture_path(rejected, Path).

fixture_path(Outcome, Path) :-
    directory_file_path('docs/specification/source-surface-fixtures', Outcome, Dir),
    exists_directory(Dir),
    directory_files(Dir, Entries),
    include(veln_file, Entries, Files),
    member(Name, Files),
    directory_file_path(Dir, Name, Path).

validate_fixtures(Expected, FixturePredicate) :-
    findall(Path, call(FixturePredicate, Path), Paths),
    Paths \= [],
    sort(Paths, Sorted),
    validate_paths(Expected, Sorted).

validate_paths(Expected, Paths) :-
    findall(
        Path-Actual,
        (
            member(Path, Paths),
            fixture_outcome(Path, Actual),
            Actual \= Expected
        ),
        Failures
    ),
    report_fixture_failures(Expected, Failures).

veln_file(Name) :-
    file_name_extension(_, veln, Name).

fixture_outcome(Path, Actual) :-
    setup_call_cleanup(
        open(Path, read, Stream),
        read_string(Stream, _, Text),
        close(Stream)
    ),
    (   parse_source_text(Text)
    ->  Actual = accepted
    ;   Actual = rejected
    ).

report_fixture_failures(_, []).
report_fixture_failures(Expected, Failures) :-
    Failures \= [],
    forall(
        member(Path-Actual, Failures),
        format(user_error, "~w: expected ~w, got ~w~n", [Path, Expected, Actual])
    ),
    halt(1).

argv_after(Text, Argv, Values) :-
    append(_, [Value | Values], Argv),
    same_text(Value, Text),
    !.

parse_source_text(Text) :-
    string_chars(Text, Chars),
    phrase(tokens(Tokens), Chars),
    \+ member(t(invalid, _), Tokens),
    phrase(source_file, Tokens).

print_grammar :-
    forall(grammar_line(_, Line), writeln(Line)).

grammar_line(10, "Module        ::= UseDecl* Item*").
grammar_line(30, "UseDecl       ::= \"use\" ModulePath ImportSource? NL").
grammar_line(35, "ImportSource  ::= \"from\" PackageString").
grammar_line(40, "ModulePath    ::= Name (\"::\" Name)*").
grammar_line(45, "PackageString ::= String").
grammar_line(47, "IntLiteral    ::= DecimalLiteral | BinaryLiteral | HexadecimalLiteral").
grammar_line(47, "DecimalLiteral ::= ASCII decimal digit+").
grammar_line(47, "BinaryLiteral ::= \"0b\" (\"0\" | \"1\")+").
grammar_line(47, "HexadecimalLiteral ::= \"0x\" ASCII hexadecimal digit+").
grammar_line(50, "Item          ::= Function | TestDecl | EffectDecl | HandlerDecl | TypeDecl | SchemaDecl | PublicAlias").
grammar_line(60, "Function      ::= \"pub\"? \"fn\" Name EffectBinder? \"(\" ParamList? \")\" Return? Effects? NL").
grammar_line(70, "                  Contract* Body \"end\" NL?").
grammar_line(80, "TestDecl      ::= \"test\" Name \"(\" \")\" Return Effects? NL").
grammar_line(90, "                  Contract* Body \"end\" NL?").
grammar_line(100, "TypeDecl      ::= \"pub\"? \"type\" Name TypeParamList? NL TypeVariant+ \"end\" NL?").
grammar_line(101, "EffectDecl    ::= \"pub\"? \"effect\" Name NL EffectOperation+ \"end\" NL?").
grammar_line(101, "EffectOperation ::= Name \"(\" EffectParamList? \")\" \"->\" TypeText NL").
grammar_line(101, "EffectParamList ::= Name \":\" TypeText (\",\" Name \":\" TypeText)*").
grammar_line(101, "HandlerDecl   ::= \"pub\"? \"handler\" Name \"(\" ParamList? \")\" \"handles\" MemberPath Effects? NL HandlerOperationClause+ \"end\" NL?").
grammar_line(101, "HandlerOperationClause ::= Name \"(\" HandlerOperationParams? \")\" \"=>\" Expr NL").
grammar_line(101, "HandlerOperationParams ::= Name (\",\" Name)*").
grammar_line(102, "SchemaDecl    ::= \"pub\"? \"schema\" Name NL SchemaFormat? SchemaField+ SchemaValidation? \"end\" NL?").
grammar_line(103, "SchemaFormat  ::= \"format\" \"binary\" NL").
grammar_line(104, "SchemaField   ::= Name \":\" SchemaFieldType SchemaFieldWhere? NL").
grammar_line(105, "SchemaFieldType ::= TypeText | LowercaseSchemaPrimitive | LowercaseReservedBitsPrimitive | ReservedBitsPrimitive | RepeatPrimitive | CanonicalRepeatPrimitive").
grammar_line(106, "LowercaseSchemaPrimitive ::= \"uint\" IntLiteral (\"be\" | \"le\")?").
grammar_line(106, "LowercaseReservedBitsPrimitive ::= \"uint\" IntLiteral (\"be\" | \"le\")? \"reserves\" IntLiteral").
grammar_line(106, "ReservedBitsPrimitive ::= \"ReservedBits\" \"(\" IntLiteral \",\" IntLiteral \")\"").
grammar_line(106, "RepeatPrimitive ::= \"Repeat\" \"(\" CountExpr \",\" TypeText \")\"").
grammar_line(106, "CanonicalRepeatPrimitive ::= \"[\" SchemaFieldType \";\" CountExpr \"]\"").
grammar_line(106, "CountExpr ::= Name | Name (\"-\" | \"+\" | \"*\" | \"/\") Name").
grammar_line(107, "SchemaFieldWhere ::= \"where\" (ContractPredicate | ByteViewMultiplePredicate)").
grammar_line(107, "ByteViewMultiplePredicate ::= \"payload_count\" \"multiple\" \"of\" (Name | IntLiteral)").
grammar_line(107, "SchemaValidation ::= \"validate\" ContractPredicate NL").
grammar_line(108, "PublicAlias   ::= \"pub\" (\"fn\" | \"type\" | \"schema\") Name \"=\" MemberPath NL").
grammar_line(110, "TypeParamList ::= \"<\" Name (\",\" Name)* \",\"? \">\"").
grammar_line(112, "EffectBinder  ::= \"<\" \"effect\" Name \">\"").
grammar_line(120, "TypeVariant   ::= \"pub\"? UpperName TypeVariantFields? NL").
grammar_line(130, "TypeVariantFields ::= \"(\" TypeVariantField (\",\" TypeVariantField)* \",\"? \")\"").
grammar_line(140, "                  | \"{\" TypeVariantField (\",\" TypeVariantField)* \",\"? \"}\"").
grammar_line(145, "TypeVariantField ::= Name \":\" TypeText | TypeText").
grammar_line(150, "ParamList     ::= Param (\",\" Param)* \",\"?").
grammar_line(160, "Param         ::= Name (\":\" VariadicMarker? TypeText)?").
grammar_line(165, "VariadicMarker ::= \"...\"").
grammar_line(170, "Return        ::= \"->\" ResultBinding? TypeText").
grammar_line(180, "ResultBinding ::= Name \":\"").
grammar_line(190, "Effects       ::= \"effects\" \"[\" EffectList? \"]\"").
grammar_line(200, "EffectList    ::= EffectEntry (\",\" EffectEntry)* \",\"?").
grammar_line(205, "EffectEntry   ::= MemberPath | \"...\" Name").
grammar_line(210, "Contract      ::= (\"require\" | \"ensure\" | \"invariant\") ContractPredicate NL").
grammar_line(220, "Body          ::= (LetLine | ExprLine)*").
grammar_line(230, "LetLine       ::= \"let\" LetPattern (\":\" TypeText)? \"=\" Expr NL").
grammar_line(240, "LetPattern    ::= \"_\" | BindingName | ConstructorPattern | RecordPattern").
grammar_line(250, "ExprLine      ::= Expr NL").
grammar_line(260, "Expr          ::= PrefixExpr (BinaryOp PrefixExpr)*").
grammar_line(265, "BinaryOp      ::= \"|>\" | \"or\" | \"and\" | \"|\" | \"^\" | \"&\" | \"==\" | \"!=\"").
grammar_line(266, "                  | \"<\" | \"<=\" | \">\" | \">=\" | \"<<\" | \">>\" | \">>>\"").
grammar_line(267, "                  | \"+\" | \"-\" | \"*\" | \"/\"").
grammar_line(270, "PrefixExpr    ::= (\"not\" | \"-\" | \"~\") PrefixExpr | PostfixExpr").
grammar_line(280, "PostfixExpr   ::= PrimaryExpr (Call | TypeArgs | FieldAccess | \"?\")*").
grammar_line(290, "PrimaryExpr   ::= Hole | Literal | NamePath | Perform | Handle | SchemaDecode | SchemaEncode | \"(\" Expr \")\" | \"()\"").
grammar_line(300, "                  | Record | Dict | List | Match | If").
grammar_line(305, "SchemaDecode  ::= \"decode\" MemberPath \"from\" Expr \"at\" Expr").
grammar_line(307, "SchemaEncode  ::= \"encode\" MemberPath \"from\" Expr").
grammar_line(308, "Perform       ::= \"perform\" MemberPath \"::\" Name \"(\" ArgList? \")\"").
grammar_line(309, "Handle        ::= \"handle\" Expr \"with\" MemberPath \"(\" ArgList? \")\"").
grammar_line(310, "Call          ::= \"(\" ArgList? \")\"").
grammar_line(320, "ArgList       ::= Expr (\",\" Expr)* \",\"?").
grammar_line(330, "TypeArgs      ::= \"<\" TypeText (\",\" TypeText)* \",\"? \">\"").
grammar_line(340, "FieldAccess   ::= \".\" Name").
grammar_line(350, "Record        ::= \"{\" (Name \":\" Expr) (\",\" Name \":\" Expr)* \",\"? \"}\"").
grammar_line(360, "Dict          ::= \"{\" Expr \":\" Expr (\",\" Expr \":\" Expr)* \",\"? \"}\"").
grammar_line(370, "List          ::= \"[\" ArgList? \"]\"").
grammar_line(380, "Match         ::= \"match\" Expr NL MatchArm+ \"end\"").
grammar_line(390, "MatchArm      ::= Pattern \"=>\" Expr NL").
grammar_line(400, "If            ::= \"if\" Expr NL Expr NL ElseIf* \"else\" NL Expr NL \"end\"").
grammar_line(410, "ElseIf        ::= \"else\" \"if\" Expr NL Expr NL").
grammar_line(420, "Pattern       ::= \"_\" | BindingName | Literal | ConstructorPattern | RecordPattern").
grammar_line(430, "ConstructorPattern ::= ConstructorName \"(\" PatternList? \")\" | ConstructorName").
grammar_line(440, "ConstructorName ::= UpperName | Name \"::\" Name (\"::\" Name)*").
grammar_line(450, "RecordPattern ::= \"{\" PatternFieldList? \"}\"").
grammar_line(460, "PatternList   ::= Pattern (\",\" Pattern)* \",\"?").
grammar_line(470, "PatternFieldList ::= PatternField (\",\" PatternField)* \",\"?").
grammar_line(460, "PatternField  ::= Name \":\" Pattern").
grammar_line(470, "MemberPath    ::= Name (\"::\" Name)*").

tokens(Tokens) -->
    trivia,
    (   one_token(Token)
    ->  { Tokens = [Token | Rest] },
        tokens(Rest)
    ;   [],
        { Tokens = [] }
    ).

trivia --> trivia_char, !, trivia.
trivia --> comment, !, trivia.
trivia --> [].

trivia_char --> [Char], { memberchk(Char, [' ', '\t', '\r']) }.

comment --> ['#'], comment_tail.
comment_tail --> [Char], { Char \= '\n' }, !, comment_tail.
comment_tail --> [].

one_token(t(nl, "\n")) --> ['\n'].
one_token(Token) --> string_token(Token).
one_token(Token) --> number_token(Token).
one_token(Token) --> ident_token(Token).
one_token(Token) --> underscore_token(Token).
one_token(t(double_colon, "::")) --> [':', ':'].
one_token(t(arrow, "->")) --> ['-', '>'].
one_token(t(fat_arrow, "=>")) --> ['=', '>'].
one_token(t(equal_equal, "==")) --> ['=', '='].
one_token(t(bang_equal, "!=")) --> ['!', '='].
one_token(t(less_equal, "<=")) --> ['<', '='].
one_token(t(greater_equal, ">=")) --> ['>', '='].
one_token(t(pipe_greater, "|>")) --> ['|', '>'].
one_token(t(shift_left, "<<")) --> ['<', '<'].
one_token(t(lparen, "(")) --> ['('].
one_token(t(rparen, ")")) --> [')'].
one_token(t(lbracket, "[")) --> ['['].
one_token(t(rbracket, "]")) --> [']'].
one_token(t(lbrace, "{")) --> ['{'].
one_token(t(rbrace, "}")) --> ['}'].
one_token(t(comma, ",")) --> [','].
one_token(t(semicolon, ";")) --> [';'].
one_token(t(colon, ":")) --> [':'].
one_token(t(dot, ".")) --> ['.'].
one_token(t(question, "?")) --> ['?'].
one_token(t(equal, "=")) --> ['='].
one_token(t(less, "<")) --> ['<'].
one_token(t(greater, ">")) --> ['>'].
one_token(t(plus, "+")) --> ['+'].
one_token(t(minus, "-")) --> ['-'].
one_token(t(star, "*")) --> ['*'].
one_token(t(slash, "/")) --> ['/'].
one_token(t(pipe, "|")) --> ['|'].
one_token(t(caret, "^")) --> ['^'].
one_token(t(ampersand, "&")) --> ['&'].
one_token(t(tilde, "~")) --> ['~'].
one_token(t(invalid, Text)) --> [Char], { string_chars(Text, [Char]) }.

string_token(t(string, Text)) -->
    ['"'],
    string_tail(Chars),
    { string_chars(Text, ['"' | Chars]) }.

string_tail(['"']) --> ['"'], !.
string_tail(['\\', Char | Rest]) --> ['\\', Char], !, string_tail(Rest).
string_tail([Char | Rest]) --> [Char], { Char \= '\n' }, !, string_tail(Rest).
string_tail([]) --> [].

number_token(t(int, Text)) -->
    ['0', 'b'],
    binary_digit(First),
    binary_digits(Rest),
    { string_chars(Text, ['0', 'b', First | Rest]) }.
number_token(t(int, Text)) -->
    ['0', 'x'],
    hexadecimal_digit(First),
    hexadecimal_digits(Rest),
    { string_chars(Text, ['0', 'x', First | Rest]) }.
number_token(t(float, Text)) -->
    digit(First),
    digits(Rest),
    ['.'],
    digit(FloatFirst),
    digits(FloatRest),
    {
        append([[First | Rest], ['.'], [FloatFirst | FloatRest]], Chars),
        string_chars(Text, Chars)
    }.
number_token(t(int, Text)) -->
    digit(First),
    digits(Rest),
    { string_chars(Text, [First | Rest]) }.

digit(Char) --> [Char], { char_type(Char, digit) }.
digits([Char | Rest]) --> digit(Char), !, digits(Rest).
digits([]) --> [].

binary_digit(Char) --> [Char], { memberchk(Char, ['0', '1']) }.
binary_digits([Char | Rest]) --> binary_digit(Char), !, binary_digits(Rest).
binary_digits([]) --> [].

hexadecimal_digit(Char) -->
    [Char],
    { char_type(Char, digit) ; memberchk(Char, ['a', 'b', 'c', 'd', 'e', 'f', 'A', 'B', 'C', 'D', 'E', 'F']) }.
hexadecimal_digits([Char | Rest]) --> hexadecimal_digit(Char), !, hexadecimal_digits(Rest).
hexadecimal_digits([]) --> [].

ident_token(t(Kind, Text)) -->
    [First],
    { char_type(First, alpha) },
    ident_continue(Rest),
    {
        string_chars(Text, [First | Rest]),
        keyword_kind(Text, Kind)
    }.

ident_continue([Char | Rest]) -->
    [Char],
    { ident_continue_char(Char) },
    !,
    ident_continue(Rest).
ident_continue([]) --> [].

ident_continue_char(Char) :-
    char_type(Char, alnum) ; Char = '_'.

keyword_kind("pub", pub).
keyword_kind("fn", fn).
keyword_kind("type", type).
keyword_kind("schema", schema).
keyword_kind("codec", codec).
keyword_kind("for", for).
keyword_kind("decode", decode).
keyword_kind("encode", encode).
keyword_kind("derive", derive).
keyword_kind("with", with).
keyword_kind("format", format).
keyword_kind("where", where).
keyword_kind("test", test).
keyword_kind("effect", effect).
keyword_kind("effects", effects).
keyword_kind("perform", perform).
keyword_kind("handler", handler).
keyword_kind("handles", handles).
keyword_kind("handle", handle).
keyword_kind("let", let).
keyword_kind("end", end).
keyword_kind("require", require).
keyword_kind("ensure", ensure).
keyword_kind("invariant", invariant).
keyword_kind("mod", mod).
keyword_kind("use", use).
keyword_kind("from", from).
keyword_kind("at", at).
keyword_kind("match", match).
keyword_kind("if", if).
keyword_kind("else", else).
keyword_kind("or", or).
keyword_kind("and", and).
keyword_kind("not", not).
keyword_kind(_, ident).

underscore_token(t(hole, Text)) -->
    ['_'],
    [First],
    { ident_continue_char(First) },
    !,
    ident_continue(Rest),
    { string_chars(Text, ['_', First | Rest]) }.
underscore_token(t(underscore, "_")) --> ['_'].

source_file --> nls, use_decls, items, nls.

use_decls --> use_decl, !, nls, use_decls.
use_decls --> [].
use_decl --> tok(use), module_path, import_source, nl.

import_source --> tok(from), tok(string), !.
import_source --> [].

items --> item, !, items.
items --> [].
item --> nls, function_decl.
item --> nls, test_decl.
item --> nls, effect_decl.
item --> nls, handler_decl.
item --> nls, type_decl.
item --> nls, schema_decl.
item --> nls, public_alias.

function_decl -->
    visibility,
    tok(fn),
    ident,
    effect_binder_opt,
    tok(lparen),
    params_opt,
    tok(rparen),
    return_opt,
    effects_opt,
    nl,
    contracts,
    body,
    tok(end),
    newline_opt.

effect_binder_opt --> tok(less), tok(effect), ident, tok(greater), !.
effect_binder_opt --> [].

test_decl -->
    tok(test),
    ident,
    tok(lparen),
    tok(rparen),
    return_clause,
    effects_opt,
    nl,
    contracts,
    body,
    tok(end),
    newline_opt.

effect_decl -->
    visibility,
    tok(effect),
    ident,
    nl,
    effect_operations,
    tok(end),
    newline_opt.

effect_operations --> effect_operation, !, effect_operations_tail.
effect_operations_tail --> effect_operation, !, effect_operations_tail.
effect_operations_tail --> [].
effect_operation -->
    ident,
    tok(lparen),
    effect_params_opt,
    tok(rparen),
    tok(arrow),
    type_text_until([nl]),
    nl.

handler_decl -->
    visibility,
    tok(handler),
    ident,
    tok(lparen),
    params_opt,
    tok(rparen),
    tok(handles),
    member_path,
    effects_opt,
    nl,
    handler_operation_clauses,
    tok(end),
    newline_opt.

handler_operation_clauses --> handler_operation_clause, !, handler_operation_clauses_tail.
handler_operation_clauses_tail --> handler_operation_clause, !, handler_operation_clauses_tail.
handler_operation_clauses_tail --> [].
handler_operation_clause -->
    ident,
    tok(lparen),
    handler_operation_params_opt,
    tok(rparen),
    tok(fat_arrow),
    expr,
    nl.

handler_operation_params_opt --> ident, handler_operation_params_tail_no_trailing, !.
handler_operation_params_opt --> [].
handler_operation_params_tail_no_trailing --> tok(comma), ident, !, handler_operation_params_tail_no_trailing.
handler_operation_params_tail_no_trailing --> [].

effect_params_opt --> effect_param, effect_params_tail, trailing_comma_opt, !.
effect_params_opt --> [].
effect_params_tail --> tok(comma), effect_param, !, effect_params_tail.
effect_params_tail --> [].
effect_param --> ident, tok(colon), type_text_until([comma, rparen]).

type_decl -->
    visibility,
    tok(type),
    ident,
    type_params_opt,
    nl,
    type_variants,
    tok(end),
    newline_opt.

schema_decl -->
    visibility,
    tok(schema),
    ident,
    nl,
    schema_format_opt,
    nls,
    schema_fields,
    nls,
    schema_validation_opt,
    nls,
    tok(end),
    newline_opt.

schema_format -->
    tok(format),
    ident_text("binary"),
    nl.

schema_format_opt --> schema_format, !.
schema_format_opt --> [].

schema_fields --> schema_field, !, schema_fields_tail.
schema_fields_tail --> schema_field, !, schema_fields_tail.
schema_fields_tail --> [].

schema_field -->
    ident,
    tok(colon),
    type_text_until([where, nl]),
    schema_field_where_opt,
    nl.

schema_field_where_opt -->
    tok(where),
    line_tokens(Tokens),
    { Tokens \= [], valid_schema_field_where_tokens(Tokens) },
    !.
schema_field_where_opt --> [].

schema_validation_opt -->
    ident_text("validate"),
    line_tokens(Tokens),
    { Tokens \= [], valid_contract_tokens(Tokens) },
    nl,
    !.
schema_validation_opt --> [].

public_alias -->
    tok(pub),
    alias_kind,
    ident,
    tok(equal),
    member_path,
    nl.

alias_kind --> tok(fn).
alias_kind --> tok(type).
alias_kind --> tok(schema).

type_params_opt --> tok(less), ident_list_opt, tok(greater), !.
type_params_opt --> [].
ident_list_opt --> ident, ident_tail, trailing_comma_opt, !.
ident_list_opt --> [].
ident_tail --> tok(comma), ident, !, ident_tail.
ident_tail --> [].

type_variants --> type_variant, !, type_variants_tail.
type_variants_tail --> type_variant, !, type_variants_tail.
type_variants_tail --> [].
type_variant --> visibility, upper_name, type_variant_fields_opt, nl.
type_variant_fields_opt --> tok(lparen), type_variant_field_list, tok(rparen), !.
type_variant_fields_opt --> tok(lbrace), type_variant_record_field_list, tok(rbrace), !.
type_variant_fields_opt --> [].
type_variant_field_list --> type_variant_field, type_variant_field_tail, trailing_comma_opt, !.
type_variant_field_tail --> tok(comma), type_variant_field, !, type_variant_field_tail.
type_variant_field_tail --> [].
type_variant_field --> ident, tok(colon), type_text_until([comma, rparen]).
type_variant_field --> type_text_until([comma, rparen]).
type_variant_record_field_list --> type_variant_record_field, type_variant_record_field_tail, trailing_comma_opt, !.
type_variant_record_field_tail --> tok(comma), type_variant_record_field, !, type_variant_record_field_tail.
type_variant_record_field_tail --> [].
type_variant_record_field --> ident, tok(colon), type_text_until([comma, rbrace]).

visibility --> tok(pub), !.
visibility --> [].

params_opt --> param, params_tail, trailing_comma_opt, !.
params_opt --> [].
params_tail --> tok(comma), param, !, params_tail.
params_tail --> [].
trailing_comma_opt --> tok(comma), !.
trailing_comma_opt --> [].

param --> ident, annotation_opt.
annotation_opt --> tok(colon), type_text_until([comma, rparen]), !.
annotation_opt --> [].

return_opt --> return_clause, !.
return_opt --> [].
return_clause --> tok(arrow), result_binding_opt, type_text_until([effects, nl]).
result_binding_opt --> ident, tok(colon), !.
result_binding_opt --> [].

effects_opt --> effects_clause, !.
effects_opt --> [].
effects_clause --> tok(effects), tok(lbracket), effects_names_opt, tok(rbracket).
effects_names_opt --> effect_entry, effects_names_tail, trailing_comma_opt, !.
effects_names_opt --> [].
effects_names_tail --> tok(comma), effect_entry, !, effects_names_tail.
effects_names_tail --> [].
effect_entry --> row_tail, !.
effect_entry --> member_path.
row_tail --> tok(dot), tok(dot), tok(dot), ident.

contracts --> contract, !, contracts.
contracts --> [].
contract -->
    contract_keyword,
    line_tokens(Tokens),
    { Tokens \= [], valid_contract_tokens(Tokens) },
    nl.

contract_keyword --> tok(require).
contract_keyword --> tok(ensure).
contract_keyword --> tok(invariant).

valid_contract_tokens(Tokens) :-
    \+ member(t(hole, _), Tokens),
    \+ member(t(underscore, _), Tokens),
    \+ member(t(lbracket, _), Tokens),
    phrase(expr, Tokens).

valid_schema_field_where_tokens(Tokens) :-
    valid_contract_tokens(Tokens).
valid_schema_field_where_tokens(Tokens) :-
    phrase(byte_view_multiple_predicate, Tokens).

byte_view_multiple_predicate -->
    ident_text("payload_count"),
    ident_text("multiple"),
    ident_text("of"),
    byte_view_multiple_operand.

byte_view_multiple_operand --> ident.
byte_view_multiple_operand --> int_literal.

int_literal --> tok(int).

body(S0, S) :-
    nls(S0, S1),
    (   S1 = [t(end, _) | _]
    ->  S = S1
    ;   body_line(S1, S2),
        body(S2, S)
    ).

body_line --> let_line.
body_line --> expr_line.

let_line -->
    tok(let),
    pattern_text_until([colon, equal], PatternTokens),
    { PatternTokens \= [], phrase(let_pattern, PatternTokens) },
    let_annotation_opt,
    tok(equal),
    expr_line.

let_annotation_opt --> tok(colon), type_text_until([equal, nl]), !.
let_annotation_opt --> [].

expr_line -->
    expr_line_tokens(Tokens),
    { Tokens \= [], phrase(expr, Tokens) }.

expr_line_tokens(Tokens, S0, S) :-
    collect_expr_line(S0, S, 0, 0, none, [], Reversed),
    reverse(Reversed, Tokens).

collect_expr_line([t(nl, _) | Rest], Rest, 0, 0, _, Acc, Acc) :- !.
collect_expr_line([], [], 0, 0, _, Acc, Acc) :- !.
collect_expr_line([Token | Rest], S, Depth0, Block0, Previous, Acc0, Acc) :-
    Token = t(Kind, _),
    next_depth(Kind, Depth0, Depth),
    next_block_depth(Kind, Previous, Block0, Block),
    collect_expr_line(Rest, S, Depth, Block, Kind, [Token | Acc0], Acc).

line_tokens(Tokens, S0, S) :-
    collect_line(S0, S, [], Reversed),
    reverse(Reversed, Tokens).

collect_line([t(nl, _) | Rest], [t(nl, _) | Rest], Acc, Acc) :- !.
collect_line([], [], Acc, Acc) :- !.
collect_line([Token | Rest], S, Acc0, Acc) :-
    collect_line(Rest, S, [Token | Acc0], Acc).

pattern_text_until(Stop, Tokens, S0, S) :-
    collect_until_stop(Stop, S0, S, 0, [], Reversed),
    reverse(Reversed, Tokens).

type_text_until(Stop, S0, S) :-
    collect_type_until_stop(Stop, S0, S, 0, [], Reversed),
    Reversed \= [].

collect_until_stop(Stop, S, S, 0, Acc, Acc) :-
    S = [t(Kind, _) | _],
    memberchk(Kind, Stop),
    !.
collect_until_stop(_, [], [], 0, Acc, Acc) :- !.
collect_until_stop(Stop, [Token | Rest], S, Depth0, Acc0, Acc) :-
    Token = t(Kind, _),
    next_depth(Kind, Depth0, Depth),
    collect_until_stop(Stop, Rest, S, Depth, [Token | Acc0], Acc).

collect_type_until_stop(Stop, S, S, 0, Acc, Acc) :-
    S = [t(Kind, _) | _],
    memberchk(Kind, Stop),
    !.
collect_type_until_stop(_, [], [], 0, Acc, Acc) :- !.
collect_type_until_stop(Stop, [Token | Rest], S, Depth0, Acc0, Acc) :-
    Token = t(Kind, _),
    next_type_depth(Kind, Depth0, Depth),
    collect_type_until_stop(Stop, Rest, S, Depth, [Token | Acc0], Acc).

next_type_depth(less, Depth0, Depth) :- !, Depth is Depth0 + 1.
next_type_depth(greater, Depth0, Depth) :- !, Depth is max(0, Depth0 - 1).
next_type_depth(Kind, Depth0, Depth) :- next_depth(Kind, Depth0, Depth).

next_depth(lparen, Depth0, Depth) :- !, Depth is Depth0 + 1.
next_depth(lbracket, Depth0, Depth) :- !, Depth is Depth0 + 1.
next_depth(lbrace, Depth0, Depth) :- !, Depth is Depth0 + 1.
next_depth(rparen, Depth0, Depth) :- !, Depth is max(0, Depth0 - 1).
next_depth(rbracket, Depth0, Depth) :- !, Depth is max(0, Depth0 - 1).
next_depth(rbrace, Depth0, Depth) :- !, Depth is max(0, Depth0 - 1).
next_depth(_, Depth, Depth).

next_block_depth(match, _, Block0, Block) :- !, Block is Block0 + 1.
next_block_depth(if, Previous, Block0, Block) :- Previous \= else, !, Block is Block0 + 1.
next_block_depth(end, _, Block0, Block) :- Block0 > 0, !, Block is Block0 - 1.
next_block_depth(_, _, Block, Block).

expr --> prefix_expr, binary_tail.
binary_tail --> binary_op, prefix_expr, !, binary_tail.
binary_tail --> [].

prefix_expr --> tok(not), !, prefix_expr.
prefix_expr --> tok(minus), !, prefix_expr.
prefix_expr --> tok(tilde), !, prefix_expr.
prefix_expr --> postfix_expr.

postfix_expr --> primary_expr, postfix_tail.
postfix_tail --> call_suffix, !, postfix_tail.
postfix_tail --> type_args_suffix, !, postfix_tail.
postfix_tail --> field_suffix, !, postfix_tail.
postfix_tail --> tok(question), !, postfix_tail.
postfix_tail --> [].

primary_expr --> tok(hole), satisfy_opt.
primary_expr --> tok(underscore), satisfy_opt.
primary_expr --> literal.
primary_expr --> perform_expr.
primary_expr --> handle_expr.
primary_expr --> schema_decode_expr.
primary_expr --> schema_encode_expr.
primary_expr --> name_path.
primary_expr --> tok(lparen), nls, tok(rparen).
primary_expr --> tok(lparen), nls, expr, nls, tok(rparen).
primary_expr --> record_or_dict.
primary_expr --> list_expr.
primary_expr --> match_expr.
primary_expr --> if_expr.

satisfy_opt --> ident_text("satisfy"), ident, tok(fat_arrow), expr, !.
satisfy_opt --> [].

schema_decode_expr -->
    tok(decode),
    member_path,
    tok(from),
    expr,
    tok(at),
    expr.

schema_encode_expr -->
    tok(encode),
    member_path,
    tok(from),
    expr.

perform_expr -->
    tok(perform),
    effect_path_before_operation,
    tok(double_colon),
    ident,
    tok(lparen),
    nls,
    args_opt,
    nls,
    tok(rparen).

handle_expr -->
    tok(handle),
    expr,
    tok(with),
    member_path,
    tok(lparen),
    nls,
    args_opt,
    nls,
    tok(rparen).

effect_path_before_operation --> ident, effect_path_before_operation_tail.
effect_path_before_operation_tail -->
    [t(double_colon, _), t(ident, _)],
    peek_double_colon,
    !,
    effect_path_before_operation_tail.
effect_path_before_operation_tail --> [].

peek_double_colon([t(double_colon, _) | Rest], [t(double_colon, _) | Rest]).

call_suffix --> tok(lparen), nls, args_opt, nls, tok(rparen).
args_opt --> expr, nls, args_tail, trailing_comma_opt, !.
args_opt --> [].
args_tail --> tok(comma), nls, expr, nls, !, args_tail.
args_tail --> [].

type_args_suffix --> tok(less), type_arg, type_arg_tail, trailing_comma_opt, tok(greater).
type_arg_tail --> tok(comma), type_arg, !, type_arg_tail.
type_arg_tail --> [].
type_arg --> type_text_until([comma, greater]).

field_suffix --> tok(dot), ident.

record_or_dict --> tok(lbrace), nls, tok(rbrace).
record_or_dict --> tok(lbrace), nls, entry, nls, entries_tail, trailing_comma_opt, tok(rbrace).
entries_tail --> tok(comma), nls, entry, nls, !, entries_tail.
entries_tail --> [].
entry --> field_name, tok(colon), expr.
entry --> expr, tok(colon), expr.

field_name --> ident.
field_name --> tok(effect).

list_expr --> tok(lbracket), nls, args_opt, nls, tok(rbracket).

match_expr --> tok(match), expr, nls, match_arms, tok(end).
match_arms --> match_arm, !, match_arms_tail.
match_arms_tail --> match_arm, !, match_arms_tail.
match_arms_tail --> [].
match_arm -->
    pattern,
    tok(fat_arrow),
    expr_line_tokens(Tokens),
    { Tokens \= [], phrase(expr, Tokens) }.

if_expr --> tok(if), expr, nls, expr, nls, else_if_tail, tok(else), nls, expr, nls, tok(end).
else_if_tail --> tok(else), tok(if), expr, nls, expr, nls, !, else_if_tail.
else_if_tail --> [].

let_pattern --> tok(underscore).
let_pattern --> binding_name.
let_pattern --> record_pattern.
let_pattern --> constructor_pattern.

pattern --> tok(underscore).
pattern --> literal.
pattern --> record_pattern.
pattern --> constructor_pattern.
pattern --> binding_name.

constructor_pattern -->
    constructor_name,
    constructor_args_opt.

constructor_args_opt --> tok(lparen), pattern_list_opt, tok(rparen), !.
constructor_args_opt --> [].
pattern_list_opt --> pattern, pattern_tail, trailing_comma_opt, !.
pattern_list_opt --> [].
pattern_tail --> tok(comma), pattern, !, pattern_tail.
pattern_tail --> [].

record_pattern --> tok(lbrace), nls, pattern_fields_opt, nls, tok(rbrace).
pattern_fields_opt --> pattern_field, pattern_fields_tail, trailing_comma_opt, !.
pattern_fields_opt --> [].
pattern_fields_tail --> tok(comma), pattern_field, !, pattern_fields_tail.
pattern_fields_tail --> [].
pattern_field --> ident, tok(colon), pattern.

constructor_name --> upper_name.
constructor_name --> ident, tok(double_colon), ident, constructor_tail.
constructor_tail --> tok(double_colon), ident, !, constructor_tail.
constructor_tail --> [].

literal --> tok(string).
literal --> tok(int).
literal --> tok(float).
literal --> ident_text("true").
literal --> ident_text("false").

name_path --> ident, name_path_tail.
name_path_tail --> tok(double_colon), path_segment, !, name_path_tail.
name_path_tail --> [].

member_path --> path_segment, member_path_tail.
member_path_tail --> tok(double_colon), path_segment, !, member_path_tail.
member_path_tail --> [].

module_path --> path_segment, module_path_tail.
module_path_tail --> tok(double_colon), path_segment, !, module_path_tail.
module_path_tail --> [].

binary_op --> tok(pipe_greater).
binary_op --> tok(or).
binary_op --> tok(and).
binary_op --> tok(pipe).
binary_op --> tok(caret).
binary_op --> tok(ampersand).
binary_op --> tok(equal_equal).
binary_op --> tok(bang_equal).
binary_op --> tok(less).
binary_op --> tok(less_equal).
binary_op --> tok(greater).
binary_op --> tok(greater_equal).
binary_op --> tok(shift_left).
binary_op --> tok(greater), tok(greater), tok(greater), !.
binary_op --> tok(greater), tok(greater), !.
binary_op --> tok(plus).
binary_op --> tok(minus).
binary_op --> tok(star).
binary_op --> tok(slash).

binding_name --> identifier_text(Text), { string_chars(Text, [First | _]), \+ char_type(First, upper) }.
upper_name --> identifier_text(Text), { string_chars(Text, [First | _]), char_type(First, upper) }.
ident --> identifier_text(_).
identifier_text(Text) --> [t(ident, Text)].
identifier_text(Text) --> [t(handle, Text)].
identifier_text(Text) --> [t(handler, Text)].
identifier_text(Text) --> [t(handles, Text)].
path_segment --> ident.
path_segment --> tok(decode).
path_segment --> tok(encode).
ident_text(Text) --> [t(ident, Text)].
ident_text(Expected) --> [t(ident, Expected)].

nls --> nl, !, nls.
nls --> [].
nl --> tok(nl).
newline_opt --> nl, !.
newline_opt --> [].

tok(Kind) --> [t(Kind, _)].
