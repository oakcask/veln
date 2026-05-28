:- use_module(library(filesex)).
:- use_module(library(readutil)).

:- initialization(main, main).

main(Argv) :-
    (   argv_member("--grammar", Argv)
    ->  print_grammar
    ;   argv_member("--check", Argv)
    ->  check_fixtures
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
    writeln("usage: swipl -q -s docs/specification/source-surface-executable.pl -- --check|--grammar").

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
    maplist(validate_fixture(Expected), Sorted).

veln_file(Name) :-
    file_name_extension(_, veln, Name).

validate_fixture(Expected, Path) :-
    setup_call_cleanup(
        open(Path, read, Stream),
        read_string(Stream, _, Text),
        close(Stream)
    ),
    (   parse_source_text(Text)
    ->  Actual = accepted
    ;   Actual = rejected
    ),
    (   Actual = Expected
    ->  true
    ;   format(user_error, "~w: expected ~w, got ~w~n", [Path, Expected, Actual]),
        halt(1)
    ).

parse_source_text(Text) :-
    string_chars(Text, Chars),
    phrase(tokens(Tokens), Chars),
    \+ member(t(invalid, _), Tokens),
    phrase(source_file, Tokens).

print_grammar :-
    forall(grammar_line(_, Line), writeln(Line)).

grammar_line(10, "Module        ::= ModDecl? UseDecl* Item*").
grammar_line(20, "ModDecl       ::= \"mod\" ModuleName NL").
grammar_line(30, "UseDecl       ::= \"use\" ModuleName NL").
grammar_line(40, "ModuleName    ::= Name (\".\" Name)*").
grammar_line(50, "Item          ::= Function | TestDecl").
grammar_line(60, "Function      ::= \"pub\"? \"fn\" Name \"(\" ParamList? \")\" Return? Effects? NL").
grammar_line(70, "                  Contract* Body \"end\" NL?").
grammar_line(80, "TestDecl      ::= \"test\" Name \"(\" \")\" Return Effects NL").
grammar_line(90, "                  Contract* Body \"end\" NL?").
grammar_line(100, "ParamList     ::= Param (\",\" Param)* \",\"?").
grammar_line(110, "Param         ::= Name (\":\" TypeText)?").
grammar_line(120, "Return        ::= \"->\" ResultBinding? TypeText").
grammar_line(130, "ResultBinding ::= Name \":\"").
grammar_line(140, "Effects       ::= \"effects\" \"[\" EffectList? \"]\"").
grammar_line(150, "EffectList    ::= Name (\",\" Name)* \",\"?").
grammar_line(160, "Contract      ::= (\"require\" | \"ensure\" | \"invariant\") ContractPredicate NL").
grammar_line(170, "Body          ::= (LetLine | ExprLine)*").
grammar_line(180, "LetLine       ::= \"let\" LetPattern (\":\" TypeText)? \"=\" Expr NL").
grammar_line(190, "LetPattern    ::= \"_\" | BindingName | RecordPattern").
grammar_line(200, "ExprLine      ::= Expr NL").
grammar_line(210, "Expr          ::= PrefixExpr (BinaryOp PrefixExpr)*").
grammar_line(220, "PrefixExpr    ::= (\"not\" | \"-\") PrefixExpr | PostfixExpr").
grammar_line(230, "PostfixExpr   ::= PrimaryExpr (Call | TypeArgs | FieldAccess | \"?\")*").
grammar_line(240, "PrimaryExpr   ::= Hole | Literal | NamePath | \"(\" Expr \")\" | \"()\"").
grammar_line(250, "                  | Record | Dict | List | Match").
grammar_line(260, "Call          ::= \"(\" ArgList? \")\"").
grammar_line(270, "ArgList       ::= Expr (\",\" Expr)* \",\"?").
grammar_line(280, "TypeArgs      ::= \"[\" TypeText (\",\" TypeText)* \",\"? \"]\"").
grammar_line(290, "FieldAccess   ::= \".\" Name").
grammar_line(300, "Record        ::= \"{\" (Name \":\" Expr) (\",\" Name \":\" Expr)* \",\"? \"}\"").
grammar_line(310, "Dict          ::= \"{\" Expr \":\" Expr (\",\" Expr \":\" Expr)* \",\"? \"}\"").
grammar_line(320, "List          ::= \"[\" ArgList? \"]\"").
grammar_line(330, "Match         ::= \"match\" Expr NL MatchArm+ \"end\"").
grammar_line(340, "MatchArm      ::= Pattern \"=>\" Expr NL").
grammar_line(350, "Pattern       ::= \"_\" | BindingName | Literal | ConstructorPattern | RecordPattern").
grammar_line(360, "ConstructorPattern ::= ConstructorName \"(\" PatternList? \")\" | ConstructorName").
grammar_line(370, "ConstructorName ::= UpperName | Name \"::\" Name (\"::\" Name)*").
grammar_line(380, "RecordPattern ::= \"{\" PatternFieldList? \"}\"").
grammar_line(390, "PatternList   ::= Pattern (\",\" Pattern)* \",\"?").
grammar_line(400, "PatternFieldList ::= PatternField (\",\" PatternField)* \",\"?").
grammar_line(410, "PatternField  ::= Name \":\" Pattern").

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

comment --> ['/','/'], comment_tail.
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
one_token(t(lparen, "(")) --> ['('].
one_token(t(rparen, ")")) --> [')'].
one_token(t(lbracket, "[")) --> ['['].
one_token(t(rbracket, "]")) --> [']'].
one_token(t(lbrace, "{")) --> ['{'].
one_token(t(rbrace, "}")) --> ['}'].
one_token(t(comma, ",")) --> [','].
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
one_token(t(invalid, Text)) --> [Char], { string_chars(Text, [Char]) }.

string_token(t(string, Text)) -->
    ['"'],
    string_tail(Chars),
    { string_chars(Text, ['"' | Chars]) }.

string_tail(['"']) --> ['"'], !.
string_tail(['\\', Char | Rest]) --> ['\\', Char], !, string_tail(Rest).
string_tail([Char | Rest]) --> [Char], { Char \= '\n' }, !, string_tail(Rest).
string_tail([]) --> [].

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
keyword_kind("test", test).
keyword_kind("effects", effects).
keyword_kind("let", let).
keyword_kind("end", end).
keyword_kind("require", require).
keyword_kind("ensure", ensure).
keyword_kind("invariant", invariant).
keyword_kind("mod", mod).
keyword_kind("use", use).
keyword_kind("match", match).
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

source_file --> nls, optional_mod, use_decls, items, nls.

optional_mod --> mod_decl, !.
optional_mod --> [].

mod_decl --> tok(mod), module_name, nl.
use_decls --> use_decl, !, use_decls.
use_decls --> [].
use_decl --> tok(use), module_name, nl.

items --> item, !, items.
items --> [].
item --> nls, function_decl.
item --> nls, test_decl.

function_decl -->
    visibility,
    tok(fn),
    ident,
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

test_decl -->
    tok(test),
    ident,
    tok(lparen),
    tok(rparen),
    return_clause,
    effects_clause,
    nl,
    contracts,
    body,
    tok(end),
    newline_opt.

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
effects_names_opt --> ident, effects_names_tail, trailing_comma_opt, !.
effects_names_opt --> [].
effects_names_tail --> tok(comma), ident, !, effects_names_tail.
effects_names_tail --> [].

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
    collect_expr_line(S0, S, 0, 0, [], Reversed),
    reverse(Reversed, Tokens).

collect_expr_line([t(nl, _) | Rest], Rest, 0, 0, Acc, Acc) :- !.
collect_expr_line([], [], 0, 0, Acc, Acc) :- !.
collect_expr_line([Token | Rest], S, Depth0, Match0, Acc0, Acc) :-
    Token = t(Kind, _),
    next_depth(Kind, Depth0, Depth),
    next_match_depth(Kind, Match0, Match),
    collect_expr_line(Rest, S, Depth, Match, [Token | Acc0], Acc).

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
    collect_until_stop(Stop, S0, S, 0, [], Reversed),
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

next_depth(lparen, Depth0, Depth) :- !, Depth is Depth0 + 1.
next_depth(lbracket, Depth0, Depth) :- !, Depth is Depth0 + 1.
next_depth(lbrace, Depth0, Depth) :- !, Depth is Depth0 + 1.
next_depth(rparen, Depth0, Depth) :- !, Depth is max(0, Depth0 - 1).
next_depth(rbracket, Depth0, Depth) :- !, Depth is max(0, Depth0 - 1).
next_depth(rbrace, Depth0, Depth) :- !, Depth is max(0, Depth0 - 1).
next_depth(_, Depth, Depth).

next_match_depth(match, Match0, Match) :- !, Match is Match0 + 1.
next_match_depth(end, Match0, Match) :- Match0 > 0, !, Match is Match0 - 1.
next_match_depth(_, Match, Match).

expr --> prefix_expr, binary_tail.
binary_tail --> binary_op, prefix_expr, !, binary_tail.
binary_tail --> [].

prefix_expr --> tok(not), !, prefix_expr.
prefix_expr --> tok(minus), !, prefix_expr.
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
primary_expr --> name_path.
primary_expr --> tok(lparen), nls, tok(rparen).
primary_expr --> tok(lparen), nls, expr, nls, tok(rparen).
primary_expr --> record_or_dict.
primary_expr --> list_expr.
primary_expr --> match_expr.

satisfy_opt --> ident_text("satisfy"), ident, tok(fat_arrow), expr, !.
satisfy_opt --> [].

call_suffix --> tok(lparen), nls, args_opt, nls, tok(rparen).
args_opt --> expr, nls, args_tail, trailing_comma_opt, !.
args_opt --> [].
args_tail --> tok(comma), nls, expr, nls, !, args_tail.
args_tail --> [].

type_args_suffix --> tok(lbracket), type_arg, type_arg_tail, trailing_comma_opt, tok(rbracket).
type_arg_tail --> tok(comma), type_arg, !, type_arg_tail.
type_arg_tail --> [].
type_arg --> type_text_until([comma, rbracket]).

field_suffix --> tok(dot), ident.

record_or_dict --> tok(lbrace), nls, tok(rbrace).
record_or_dict --> tok(lbrace), nls, entry, nls, entries_tail, trailing_comma_opt, tok(rbrace).
entries_tail --> tok(comma), nls, entry, nls, !, entries_tail.
entries_tail --> [].
entry --> expr, tok(colon), expr.

list_expr --> tok(lbracket), nls, args_opt, nls, tok(rbracket).

match_expr --> tok(match), expr, nls, match_arms, tok(end).
match_arms --> match_arm, !, match_arms_tail.
match_arms_tail --> match_arm, !, match_arms_tail.
match_arms_tail --> [].
match_arm --> pattern, tok(fat_arrow), expr, nls.

let_pattern --> tok(underscore).
let_pattern --> binding_name.
let_pattern --> record_pattern.

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
name_path_tail --> tok(double_colon), ident, !, name_path_tail.
name_path_tail --> [].

module_name --> ident, module_name_tail.
module_name_tail --> tok(dot), ident, !, module_name_tail.
module_name_tail --> [].

binary_op --> tok(pipe_greater).
binary_op --> tok(or).
binary_op --> tok(and).
binary_op --> tok(equal_equal).
binary_op --> tok(bang_equal).
binary_op --> tok(less).
binary_op --> tok(less_equal).
binary_op --> tok(greater).
binary_op --> tok(greater_equal).
binary_op --> tok(plus).
binary_op --> tok(minus).
binary_op --> tok(star).
binary_op --> tok(slash).

binding_name --> ident_text(Text), { string_chars(Text, [First | _]), \+ char_type(First, upper) }.
upper_name --> ident_text(Text), { string_chars(Text, [First | _]), char_type(First, upper) }.
ident --> tok(ident).
ident_text(Text) --> [t(ident, Text)].
ident_text(Expected) --> [t(ident, Expected)].

nls --> nl, !, nls.
nls --> [].
nl --> tok(nl).
newline_opt --> nl, !.
newline_opt --> [].

tok(Kind) --> [t(Kind, _)].
