"""Bounded rank-one feasibility model, deliberately independent of veln-sema.

This is a proposal experiment, not a second language implementation. Types are
immutable; solving produces a candidate without modifying its scheme or caller.
"""

from dataclasses import dataclass, field


class Rejected(ValueError):
    pass


@dataclass(frozen=True)
class Type:
    name: str
    args: tuple = ()
    fields: tuple = ()
    effects: frozenset = frozenset()


def named(name, *args):
    return Type(name, tuple(args))


def record(**fields):
    return Type("record", fields=tuple(sorted(fields.items())))


def function(params, result, effects=()):
    return Type("fn", (*params, result), effects=frozenset(effects))


INT, STRING, UNIT = named("Int"), named("String"), named("()")
STATE, READY, CLOSED = map(named, ("State", "State::Ready", "State::Closed"))
VARIANCE = {"Vec": (1,), "Option": (1,), "Cell": (0,), "Sink": (-1,), "Phantom": (None,)}


def subtype(a, b):
    if a == b:
        return True
    if a in (READY, CLOSED) and b == STATE:
        return True
    if a.name == b.name == "record":
        actual = dict(a.fields)
        return all(k in actual and subtype(actual[k], v) for k, v in b.fields)
    if a.name == b.name == "fn" and len(a.args) == len(b.args):
        return (all(subtype(y, x) for x, y in zip(a.args[:-1], b.args[:-1]))
                and subtype(a.args[-1], b.args[-1]) and a.effects <= b.effects)
    if a.name == b.name and len(a.args) == len(b.args) and a.name in VARIANCE:
        return all(v is None or (subtype(x, y) if v == 1 else
                                 subtype(y, x) if v == -1 else x == y)
                   for x, y, v in zip(a.args, b.args, VARIANCE[a.name]))
    return False


def bounded(types, join=True):
    """Structural bounds, then the contributed-candidate rule; never a top."""
    types = tuple(dict.fromkeys(types))
    if not types:
        raise Rejected("no bounds")
    if len(types) == 1:
        return types[0]
    if join and set(types) <= {READY, CLOSED, STATE}:
        return STATE
    first = types[0]
    candidate = None
    if all(t.name == first.name and len(t.args) == len(first.args) for t in types):
        if first.name == "record" and all(tuple(k for k, _ in t.fields) ==
                                           tuple(k for k, _ in first.fields) for t in types):
            candidate = record(**{k: bounded([dict(t.fields)[k] for t in types], join)
                                  for k, _ in first.fields})
        elif first.name == "fn":
            rows = [t.effects for t in types]
            if any(any(e.startswith("$") for e in row) for row in rows):
                if any(row != rows[0] for row in rows):
                    raise Rejected("different rigid effect tails")
                effects = rows[0]
            else:
                effects = (frozenset.union(*rows) if join else frozenset.intersection(*rows))
            candidate = function([bounded([t.args[i] for t in types], not join)
                                  for i in range(len(first.args) - 1)],
                                 bounded([t.args[-1] for t in types], join), effects)
        elif first.name in VARIANCE:
            args = []
            for i, variance in enumerate(VARIANCE[first.name]):
                values = [t.args[i] for t in types]
                if variance in (0, None):
                    if len(set(values)) != 1:
                        raise Rejected("inexact invariant/irrelevant arguments")
                    args.append(values[0])
                else:
                    args.append(bounded(values, join if variance == 1 else not join))
            candidate = named(first.name, *args)
    relation = subtype if join else lambda a, b: subtype(b, a)
    if candidate is not None and all(relation(t, candidate) for t in types):
        return candidate
    valid = [c for c in types if all(relation(t, c) for t in types)]
    best = [c for c in valid if all(relation(c, other) for other in valid)]
    if len(best) != 1:
        raise Rejected("no unique bounded result")
    return best[0]


def replace(ty, mapping, row=None):
    if ty in mapping:
        return mapping[ty]
    effects = ty.effects
    if row is not None and "@E" in effects:
        effects = (effects - {"@E"}) | row
    return Type(ty.name, tuple(replace(t, mapping, row) for t in ty.args),
                tuple((k, replace(t, mapping, row)) for k, t in ty.fields), effects)


@dataclass(frozen=True)
class Scheme:
    owner: str
    variables: tuple
    signature: Type
    has_row: bool = False


@dataclass
class Constraints:
    variables: tuple
    lower: dict = field(default_factory=dict)
    upper: dict = field(default_factory=dict)
    equal: dict = field(default_factory=dict)
    row_lower: set = field(default_factory=set)
    row_upper: list = field(default_factory=list)

    def add(self, bucket, variable, ty):
        bucket.setdefault(variable, []).append(ty)

    def equality(self, left, right):
        if right in self.variables:
            self.add(self.equal, right, left)
        elif left in self.variables:
            self.add(self.equal, left, right)
        elif (left.name != right.name or len(left.args) != len(right.args)
              or tuple(k for k, _ in left.fields) != tuple(k for k, _ in right.fields)
              or left.effects != right.effects):
            raise Rejected("incompatible invariant shape")
        else:
            for a, b in zip(left.args, right.args):
                self.equality(a, b)
            for (_, a), (_, b) in zip(left.fields, right.fields):
                self.equality(a, b)

    def relation(self, left, right):
        if right in self.variables:
            self.add(self.lower, right, left)
        elif left in self.variables:
            self.add(self.upper, left, right)
        elif left.name == right.name == "record":
            fields = dict(left.fields)
            for key, ty in right.fields:
                if key not in fields:
                    raise Rejected("missing field")
                self.relation(fields[key], ty)
        elif left.name == right.name == "fn" and len(left.args) == len(right.args):
            for actual, expected in zip(left.args[:-1], right.args[:-1]):
                self.relation(expected, actual)
            self.relation(left.args[-1], right.args[-1])
            if "@E" in right.effects:
                self.row_lower.update(left.effects - (right.effects - {"@E"}))
            elif "@E" in left.effects:
                self.row_upper.append(right.effects - (left.effects - {"@E"}))
            elif not left.effects <= right.effects:
                raise Rejected("effect inclusion")
        elif left.name == right.name and len(left.args) == len(right.args) and left.name in VARIANCE:
            for a, b, variance in zip(left.args, right.args, VARIANCE[left.name]):
                if variance == 1:
                    self.relation(a, b)
                elif variance == -1:
                    self.relation(b, a)
                elif variance == 0:
                    self.equality(a, b)
        elif not subtype(left, right):
            raise Rejected("incompatible outer shape or rigid type")

    def solve(self, fixed):
        mapping = dict(fixed)
        for var in self.variables:
            lo, hi, eq = (b.get(var, []) for b in (self.lower, self.upper, self.equal))
            if var in mapping:
                selected = mapping[var]
            elif eq:
                selected = eq[0]
            elif lo:
                selected = bounded(lo)
            elif hi:
                selected = bounded(hi, False)
            else:
                raise Rejected("unresolved ordinary parameter")
            if (any(t != selected for t in eq) or any(not subtype(t, selected) for t in lo)
                    or any(not subtype(selected, t) for t in hi)):
                raise Rejected("conflicting bounds")
            mapping[var] = selected
        row = frozenset(self.row_lower)
        if any(not row <= upper for upper in self.row_upper):
            raise Rejected("row upper bound")
        return mapping, row


def project(template, expected, variables, fixed):
    if template in variables:
        if template in fixed and fixed[template] != expected:
            raise Rejected("inconsistent contextual projections")
        fixed[template] = expected
    elif template.name == expected.name == "record":
        context = dict(expected.fields)
        for key, ty in template.fields:
            if key in context:
                project(ty, context[key], variables, fixed)
    elif template.name == expected.name and template.name in VARIANCE:
        for a, b, variance in zip(template.args, expected.args, VARIANCE[template.name]):
            if variance is not None:
                project(a, b, variables, fixed)


def instantiate(scheme, arguments, expected=None, explicit=None, row_upper=None):
    if len(arguments) != len(scheme.signature.args) - 1:
        raise Rejected("value arity")
    if explicit is not None and len(explicit) != len(scheme.variables):
        raise Rejected("type arity")
    fixed = dict(zip(scheme.variables, explicit)) if explicit is not None else {}
    bounds = Constraints(scheme.variables)
    for actual, template in zip(arguments, scheme.signature.args[:-1]):
        bounds.relation(actual, template)
    if expected is not None:
        if explicit is None:
            project(scheme.signature.args[-1], expected, scheme.variables, fixed)
        bounds.relation(scheme.signature.args[-1], expected)
    if row_upper is not None:
        bounds.row_upper.append(frozenset(row_upper))
    mapping, row = bounds.solve(fixed)
    instance = replace(scheme.signature, mapping, row)
    if any(not subtype(a, b) for a, b in zip(arguments, instance.args[:-1])):
        raise Rejected("complete argument compatibility")
    if expected is not None and not subtype(instance.args[-1], expected):
        raise Rejected("complete result compatibility")
    return instance, mapping, row


@dataclass
class Pending:
    scheme: Scheme
    arguments: tuple
    row_upper: frozenset | None = None
    instance: Type | None = None

    def complete(self, expected):
        if self.instance is not None:
            if not subtype(self.instance.args[-1], expected):
                raise Rejected("already completed monotype")
            return self.instance
        instance, _, _ = instantiate(self.scheme, self.arguments, expected, row_upper=self.row_upper)
        self.instance = instance
        return instance


def alias(value, transparent=True, annotation=None):
    if isinstance(value, Scheme):
        if annotation is not None:
            instance, _, _ = instantiate(value, annotation.args[:-1], annotation.args[-1])
            if not subtype(instance, annotation):
                raise Rejected("function value context")
            return annotation
        if not transparent:
            raise Rejected("monotype context required")
    return value
