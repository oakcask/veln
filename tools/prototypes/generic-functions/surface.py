"""Small source-to-erased-Java prototype for the generic-function proposal.

Only the fixture subset is supported. This parser does not replace Veln's
executable grammar, formatter, type checker, or JVM backend.
"""

from dataclasses import dataclass
import re

from model import (Type, Scheme, Rejected, INT, UNIT, function, named, replace,
                   instantiate, subtype)


TOKEN = re.compile(r"\s+|::|->|\.\.\.|==|[A-Za-z_][A-Za-z_0-9]*|[0-9]+|[<>(){},:\[\]+\-]")


class Parser:
    def __init__(self, source):
        self.tokens = []
        offset = 0
        for match in TOKEN.finditer(source):
            if source[offset:match.start()]:
                raise Rejected("unsupported source token")
            if not match[0].isspace():
                self.tokens.append(match[0])
            offset = match.end()
        if source[offset:].strip():
            raise Rejected("unsupported trailing source")
        self.pos = 0

    def peek(self):
        return self.tokens[self.pos] if self.pos < len(self.tokens) else "<eof>"

    def eat(self, token):
        if self.peek() == token:
            self.pos += 1
            return True
        return False

    def take(self, token=None):
        value = self.peek()
        if value == "<eof>" or (token is not None and token != value):
            raise Rejected(f"expected {token}, found {value}")
        self.pos += 1
        return value

    def name(self):
        value = self.take()
        if not re.fullmatch(r"[A-Za-z][A-Za-z_0-9]*", value):
            raise Rejected("expected ordinary name")
        return value

    def path(self):
        value = self.name()
        while self.eat("::"):
            value += "::" + self.name()
        return value

    def sequence(self, closing, parse):
        result = []
        if self.eat(closing):
            return result
        while True:
            result.append(parse())
            if self.eat(closing):
                return result
            self.take(",")
            if self.eat(closing):
                return result

    def binders(self):
        if not self.eat("<"):
            return (), False
        names, row, seen = [], False, set()
        if self.peek() == ">":
            raise Rejected("empty binder")
        while True:
            effect = self.eat("effect")
            if row:
                raise Rejected("effect binder must be last")
            name = self.name()
            if name in seen:
                raise Rejected("duplicate binder")
            seen.add(name)
            if effect:
                row = name
            else:
                names.append(name)
            if self.eat(">"):
                return tuple(names), row
            self.take(",")
            if self.eat(">"):
                return tuple(names), row

    def ty(self):
        if self.eat("("):
            self.take(")")
            return UNIT
        if self.eat("fn"):
            self.take("(")
            params = self.sequence(")", self.ty)
            self.take("->")
            result = self.ty()
            return function(params, result, self.effects())
        if self.eat("{"):
            fields = self.sequence("}", self.parameter)
            return Type("record", fields=tuple(sorted(fields)))
        name = self.path()
        args = ()
        if self.eat("<"):
            args = tuple(self.sequence(">", self.ty))
            if not args:
                raise Rejected("empty type application")
        return named(name, *args)

    def effects(self):
        if not self.eat("effects"):
            return ()
        self.take("[")
        return self.sequence("]", lambda: ("..." if self.eat("...") else "") + self.path())

    def parameter(self):
        name = self.name()
        self.take(":")
        return name, self.ty()

    def typed_call_ahead(self):
        if self.peek() != "<":
            return False
        depth = 0
        for i in range(self.pos, len(self.tokens)):
            token = self.tokens[i]
            depth += (token == "<") - (token == ">")
            if depth == 0:
                return i + 1 < len(self.tokens) and self.tokens[i + 1] == "("
        return False

    def expr(self, precedence=0):
        if self.eat("if"):
            condition = self.expr()
            yes = self.expr()
            self.take("else")
            no = self.expr()
            self.take("end")
            left = ("if", condition, yes, no)
        elif self.eat("("):
            left = self.expr()
            self.take(")")
            left = ("paren", left)
        elif self.eat("["):
            left = ("vec", tuple(self.sequence("]", self.expr)))
        elif self.peek().isdigit():
            left = ("int", int(self.take()))
        else:
            left = ("name", self.path())
        while True:
            if left[0] != "name" and self.typed_call_ahead():
                raise Rejected("expression type application is outside the prototype subset")
            if left[0] == "name" and self.typed_call_ahead():
                self.take("<")
                arguments = tuple(self.sequence(">", self.ty))
                if not arguments:
                    raise Rejected("empty explicit arguments")
                self.take("(")
                left = ("call", left, arguments, tuple(self.sequence(")", self.expr)))
                continue
            if self.eat("("):
                left = ("call", left, None, tuple(self.sequence(")", self.expr)))
                continue
            powers = {"==": 1, "<": 2, ">": 2, "+": 3, "-": 3}
            power = powers.get(self.peek(), 0)
            if power <= precedence:
                break
            operator = self.take()
            left = ("binary", operator, left, self.expr(power))
        return left

    def declaration(self):
        self.eat("pub")
        self.take("fn")
        name = self.name()
        binders, row = self.binders()
        self.take("(")
        params = tuple(self.sequence(")", self.parameter))
        self.take("->")
        result = self.ty()
        effects = self.effects()
        body = self.expr()
        self.take("end")
        return Declaration(name, binders, row, params, result, tuple(effects), body)

    def finish(self):
        if self.peek() != "<eof>":
            raise Rejected("unexpected trailing source")


@dataclass(frozen=True)
class Declaration:
    name: str
    binders: tuple
    row: str | bool
    params: tuple
    result: Type
    effects: tuple
    body: tuple

    def scheme(self, owner="Provider"):
        if any(name in {"Int", "String", "Bool", "Vec", "Option"} for name in self.binders):
            raise Rejected("binder shadows visible type")
        variables = tuple(named(f"${owner}::{self.name}::{i}") for i, _ in enumerate(self.binders))
        mapping = dict(zip(map(named, self.binders), variables))
        def normalize(ty):
            effects = frozenset("@E" if self.row and e == "..." + self.row else e
                                for e in ty.effects)
            return Type(ty.name, tuple(normalize(t) for t in ty.args),
                        tuple((key, normalize(t)) for key, t in ty.fields), effects)
        signature = normalize(function([t for _, t in self.params], self.result, self.effects))
        return Scheme(f"{owner}::{self.name}", variables, replace(signature, mapping), bool(self.row))


def parse_program(source):
    parser = Parser(source)
    declarations = []
    while parser.peek() != "<eof>":
        declarations.append(parser.declaration())
    if len({d.name for d in declarations}) != len(declarations):
        raise Rejected("duplicate declaration")
    return declarations


def expression_type(expr, environment, schemes, current):
    kind = expr[0]
    if kind == "name":
        return environment[expr[1]]
    if kind == "paren":
        return expression_type(expr[1], environment, schemes, current)
    if kind == "int":
        return INT, frozenset()
    if kind == "binary":
        a, ae = expression_type(expr[2], environment, schemes, current)
        b, be = expression_type(expr[3], environment, schemes, current)
        if a != INT or b != INT:
            raise Rejected("integer operation on rigid parameter")
        return (named("Bool") if expr[1] in ("==", "<", ">") else INT), ae | be
    if kind == "if":
        condition, ce = expression_type(expr[1], environment, schemes, current)
        a, ae = expression_type(expr[2], environment, schemes, current)
        b, be = expression_type(expr[3], environment, schemes, current)
        if condition != named("Bool") or a != b:
            raise Rejected("conditional type mismatch")
        return a, ce | ae | be
    if kind == "call":
        if expr[1][0] != "name":
            raise Rejected("prototype supports named callees")
        name = expr[1][1]
        arguments = [expression_type(a, environment, schemes, current) for a in expr[3]]
        types = [t for t, _ in arguments]
        effects = frozenset().union(*(e for _, e in arguments))
        if name == "Some":
            if len(types) != 1:
                raise Rejected("constructor arity")
            return named("Option", types[0]), effects
        if name in environment:
            signature, _ = environment[name]
            if expr[2] is not None or signature.name != "fn" or len(signature.args) != len(types) + 1:
                raise Rejected("monomorphic callable")
            if any(not subtype(a, b) for a, b in zip(types, signature.args[:-1])):
                raise Rejected("callback argument")
            return signature.args[-1], effects | signature.effects
        scheme = schemes[name]
        explicit = expr[2]
        if explicit is not None:
            # Source T denotes the current rigid binder, never a new meta-variable.
            explicit = tuple(replace(t, current[1]) for t in explicit)
        if name == current[0]:
            instance = scheme.signature
            if explicit is not None and explicit != scheme.variables:
                raise Rejected("polymorphic recursion")
            if len(types) + 1 != len(instance.args) or any(
                    not subtype(a, b) for a, b in zip(types, instance.args[:-1])):
                raise Rejected("type-changing recursion")
        else:
            instance, _, _ = instantiate(scheme, types, explicit=explicit)
        return instance.args[-1], effects | instance.effects
    if kind == "vec":
        elements = [expression_type(a, environment, schemes, current) for a in expr[1]]
        if not elements or len({t for t, _ in elements}) != 1:
            raise Rejected("prototype requires nonempty homogeneous vector")
        return named("Vec", elements[0][0]), frozenset().union(*(e for _, e in elements))
    raise Rejected("unsupported expression")


def check(declarations):
    schemes = {d.name: d.scheme() for d in declarations}
    for declaration in declarations:
        scheme = schemes[declaration.name]
        environment = {name: (ty, frozenset()) for (name, _), ty in
                       zip(declaration.params, scheme.signature.args[:-1])}
        current = declaration.name, dict(zip(map(named, declaration.binders), scheme.variables))
        result, effects = expression_type(declaration.body, environment, schemes, current)
        if not subtype(result, scheme.signature.args[-1]):
            raise Rejected("body result is not valid for rigid binder")
        if not effects <= scheme.signature.effects:
            raise Rejected("body effects exceed declaration")
    return schemes


def java_expr(expr, parameters):
    kind = expr[0]
    if kind == "name":
        return expr[1]
    if kind == "int":
        return f"Long.valueOf({expr[1]}L)"
    if kind == "paren":
        return "(" + java_expr(expr[1], parameters) + ")"
    if kind == "binary":
        a, b = (java_expr(e, parameters) for e in expr[2:])
        operation = f"((Long){a}).longValue() {expr[1]} ((Long){b}).longValue()"
        return "(" + operation + ")" if expr[1] in ("==", "<", ">") else f"Long.valueOf({operation})"
    if kind == "if":
        return "(" + java_expr(expr[1], parameters) + " ? " + java_expr(expr[2], parameters) + " : " + java_expr(expr[3], parameters) + ")"
    if kind == "vec":
        return "java.util.List.of(" + ", ".join(java_expr(e, parameters) for e in expr[1]) + ")"
    if kind == "call":
        name = expr[1][1]
        args = ", ".join(java_expr(e, parameters) for e in expr[3])
        if name == "Some":
            return f"new Some({args})"
        if name in parameters:
            return f"((Callable){name}).call({args})"
        # Semantic type arguments are intentionally absent at this boundary.
        return f"{name}({args})"
    raise Rejected("unsupported lowering")


def emit_java(declarations):
    check(declarations)
    methods = []
    for d in declarations:
        params = ", ".join(f"Object {name}" for name, _ in d.params)
        body = java_expr(d.body, {name for name, _ in d.params})
        methods.append(f"public static Object {d.name}({params}) {{ return {body}; }}")
    return ("public final class Provider {\n"
            "public interface Callable { Object call(Object value); }\n"
            "public record Some(Object value) {}\n" + "\n".join(methods) + "\n}\n")
