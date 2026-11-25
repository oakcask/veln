"""Executable acceptance oracle; expected answers come from proposal Q1-Q9."""

from itertools import permutations
import unittest

from model import (INT, STRING, UNIT, STATE, READY, CLOSED, Type, Scheme,
                   Rejected, Constraints, Pending, alias, bounded, function,
                   instantiate, named, record, subtype)
from surface import Parser, check, emit_java, parse_program


T, U = named("$identity::0"), named("$apply::1")
IDENTITY = Scheme("identity", (T,), function([T], T))
EMPTY = Scheme("empty", (T,), function([], named("Vec", T)))
APPLY = Scheme("apply", (T, U), function([function([T], U, {"@E"}), T], U, {"@E"}), True)


def result(scheme, args, **context):
    return instantiate(scheme, args, **context)[0].args[-1]


class ConstraintTests(unittest.TestCase):
    def test_independent_instances_and_aliases_in_both_orders(self):
        # Import/re-export aliases preserve the immutable owning scheme object.
        imported = {"public::id": IDENTITY}
        f = alias(imported["public::id"])
        g = alias(f)
        for order in permutations((INT, STRING)):
            self.assertIs(g, IDENTITY)
            self.assertEqual([result(g, [t]) for t in order], list(order))
        self.assertEqual(IDENTITY.variables, (T,))
        self.assertEqual(IDENTITY.signature, function([T], T))

    def test_annotation_selects_monotype_and_no_expression_generalization(self):
        annotation = function([INT], INT)
        self.assertEqual(alias(IDENTITY, annotation=annotation), annotation)
        self.assertFalse(subtype(function([STRING], STRING), annotation))
        self.assertEqual(alias(annotation), annotation)
        with self.assertRaisesRegex(Rejected, "context"):
            alias(IDENTITY, transparent=False)
        self.assertEqual(alias(IDENTITY, transparent=False, annotation=annotation), annotation)

    def test_contextual_widening_and_explicit_arguments(self):
        self.assertEqual(result(IDENTITY, [READY]), READY)
        self.assertEqual(result(IDENTITY, [READY], expected=STATE), STATE)
        self.assertEqual(result(IDENTITY, [READY], explicit=(STATE,)), STATE)
        for kwargs in ({"explicit": ()}, {"explicit": (INT, STRING)},
                       {"explicit": (CLOSED,)}, {"explicit": (READY,), "expected": CLOSED}):
            with self.assertRaises(Rejected):
                result(IDENTITY, [READY], **kwargs)
        self.assertEqual(result(IDENTITY, [STRING]), STRING)

    def test_lower_upper_and_equality_constraints(self):
        pair = Scheme("pair", (T,), function([T, T], T))
        for order in permutations((READY, CLOSED)):
            self.assertEqual(result(pair, order), STATE)
        consume = Scheme("consume", (T,), function([T, function([T], UNIT)], T))
        self.assertEqual(result(consume, [READY, function([STATE], UNIT)]), READY)
        with self.assertRaises(Rejected):
            result(consume, [STATE, function([READY], UNIT)])
        upper_only = Scheme("upper", (T,), function([function([T], UNIT)], T))
        self.assertEqual(result(upper_only, [function([READY], UNIT)]), READY)
        invariant = Scheme("invariant", (T,), function([named("Cell", T), named("Cell", T)], T))
        with self.assertRaises(Rejected):
            result(invariant, [named("Cell", READY), named("Cell", CLOSED)])

    def test_record_width_and_complete_set_order(self):
        r1, r2 = record(x=INT), record(x=INT, y=STRING)
        for order in permutations((r1, r2)):
            self.assertEqual(bounded(order), r1)
            self.assertEqual(bounded(order, False), r2)
        for order in permutations((READY, CLOSED, STATE)):
            self.assertEqual(bounded(order), STATE)
        with self.assertRaises(Rejected):
            bounded([INT, STRING])
        # The first two contributions have no meet. The third supplies the
        # unique contributed candidate; a lossy pairwise fold would fail.
        left, right, both = record(x=INT), record(y=INT), record(x=INT, y=INT)
        contributions = [function([r], INT) for r in (left, right, both)]
        with self.assertRaises(Rejected):
            bounded(contributions[:2])
        for order in permutations(contributions):
            self.assertEqual(bounded(order), function([both], INT))

    def test_nested_polarity_and_nominal_shape(self):
        twice = Scheme("twice", (T,), function([function([function([T], UNIT)], UNIT)], T))
        self.assertEqual(result(twice, [function([function([INT], UNIT)], UNIT)]), INT)
        wrong = Scheme("vec", (T,), function([named("Vec", T)], T))
        with self.assertRaises(Rejected):
            result(wrong, [named("Option", INT)])
        invariant = Scheme("cell", (T,), function([named("Cell", named("Vec", T))], T))
        self.assertEqual(result(invariant, [named("Cell", named("Vec", INT))]), INT)
        with self.assertRaises(Rejected):
            result(invariant, [named("Cell", named("Vec", READY))], explicit=(STATE,))

    def test_irrelevant_occurrences_do_not_default(self):
        phantom = Scheme("phantom", (T,), function([named("Phantom", T)], UNIT))
        with self.assertRaisesRegex(Rejected, "unresolved"):
            result(phantom, [named("Phantom", INT)])
        self.assertEqual(result(phantom, [named("Phantom", INT)], explicit=(STRING,)), UNIT)

    def test_context_projection_is_consistent(self):
        repeated = Scheme("repeated", (T,), function([], record(a=T, b=T)))
        self.assertEqual(result(repeated, [], expected=record(a=INT, b=INT)), record(a=INT, b=INT))
        with self.assertRaisesRegex(Rejected, "projections"):
            result(repeated, [], expected=record(a=INT, b=STRING))

    def test_pending_local_is_completed_once_and_rejection_preserves_state(self):
        local = Pending(EMPTY, ())
        with self.assertRaises(Rejected):
            local.complete(INT)
        self.assertIsNone(local.instance)
        self.assertEqual(local.complete(named("Vec", INT)).args[-1], named("Vec", INT))
        original = local.instance
        with self.assertRaises(Rejected):
            local.complete(named("Vec", STRING))
        self.assertIs(local.instance, original)
        self.assertEqual(local.complete(named("Vec", INT)), original)
        with self.assertRaisesRegex(Rejected, "unresolved"):
            result(EMPTY, [])

    def test_pending_local_retains_argument_and_row_bounds(self):
        factory = Scheme("factory", (T,), function([function([T], UNIT, {"@E"})], named("Vec", T), {"@E"}), True)
        local = Pending(factory, (function([INT], UNIT, {"stdio"}),), frozenset())
        for expected in (named("Vec", STRING), named("Vec", INT)):
            with self.assertRaises(Rejected):
                local.complete(expected)
            self.assertIsNone(local.instance)
        # A failed type completion also leaves retained callback bounds usable.
        local = Pending(factory, (function([INT], UNIT, {"stdio"}),), frozenset({"stdio"}))
        with self.assertRaises(Rejected):
            local.complete(named("Vec", STRING))
        self.assertIsNone(local.instance)
        self.assertEqual(local.complete(named("Vec", INT)).effects, frozenset({"stdio"}))

    def test_joint_type_and_row_substitution(self):
        for effects in (frozenset(), frozenset({"stdio"}), frozenset({"$outer::E"})):
            instance, mapping, row = instantiate(APPLY, [function([INT], STRING, effects), INT])
            self.assertEqual(instance.args[-1], STRING)
            self.assertEqual(mapping, {T: INT, U: STRING})
            self.assertEqual(row, effects)
            self.assertEqual(instance.effects, effects)
        snapshot = APPLY.signature
        for arguments, upper in (([function([INT], STRING), STRING], None),
                                 ([function([INT], STRING, {"stdio"}), INT], frozenset())):
            with self.assertRaises(Rejected):
                instantiate(APPLY, arguments, row_upper=upper)
            self.assertEqual(APPLY.signature, snapshot)
            self.assertEqual(result(APPLY, [function([INT], STRING), INT]), STRING)

    def test_row_only_and_result_only_rows(self):
        scheme = Scheme("rowOnly", (), function([], function([], INT, {"@E"}), {"@E"}), True)
        instance, mapping, row = instantiate(scheme, [])
        self.assertEqual(mapping, {})
        self.assertEqual(row, frozenset())
        self.assertEqual(instance.args[-1].effects, frozenset())

    def test_binder_identity_is_not_spelling(self):
        other_t = named("$other::0")
        self.assertFalse(subtype(T, other_t))
        self.assertEqual(result(IDENTITY, [other_t]), other_t)
        self.assertFalse(subtype(INT, T))

    def test_fixed_rigid_binder_rejects_caller_repair(self):
        # A generic definition expecting its own rigid T cannot be repaired
        # by a concrete callback or an Int-only operation in its body.
        with self.assertRaises(Rejected):
            instantiate(IDENTITY, [INT], explicit=(T,))
        self.assertEqual(result(IDENTITY, [T], explicit=(T,)), T)


class SurfaceTests(unittest.TestCase):
    def test_binder_decisions_and_canonical_spelling(self):
        for text, names, row in (("<T>", ("T",), False),
                                 ("<T, U, effect E,>", ("T", "U"), "E"),
                                 ("<effect E>", (), "E")):
            parser = Parser(text)
            self.assertEqual(parser.binders(), (names, row))
            parser.finish()
            printed = "<" + ", ".join((*names, *(("effect " + row,) if row else ()))) + ">"
            again = Parser(printed)
            self.assertEqual(again.binders(), (names, row))
            again.finish()
        for text in ("<>", "<effect E, T>", "<T, T>", "<T, effect T>", "<T: Trait>"):
            with self.assertRaises(Rejected, msg=text):
                Parser(text).binders()

    def test_typed_call_lookahead_and_comparisons(self):
        for text in ("module::identity<Vec<Int>>([1])", "f<Int,>(1)", "f<T>(x)"):
            parser = Parser(text)
            expr = parser.expr()
            parser.finish()
            self.assertEqual(expr[0], "call")
            self.assertIsNotNone(expr[2])
        for text in ("x < y", "(f < t) > x"):
            parser = Parser(text)
            self.assertEqual(parser.expr()[0], "binary")
            parser.finish()
        for text in ("identity<Int>", "factory()<Int>(1)", "f<_, Int>(x)", "f<>(x)"):
            with self.assertRaises(Rejected, msg=text):
                parser = Parser(text)
                parser.expr()
                parser.finish()

    def test_rigid_body_check_before_emission(self):
        for body in ("0", "x + 1"):
            with self.assertRaises(Rejected):
                emit_java(parse_program(f"fn bad<T>(x: T) -> T {body} end"))
        # The checker has no caller-fact input; an Int use cannot repair bad<T>.
        with self.assertRaises(Rejected):
            emit_java(parse_program("fn bad<T>(x: T) -> T 0 end fn caller(x: Int) -> Int bad(x) end"))
        self.assertTrue(check(parse_program("fn id<A>(x: A) -> A x end")))

    def test_recursion_boundary(self):
        good = "fn recur<T>(x: T, n: Int) -> T if n == 0 x else recur<T>(x, n - 1) end end"
        self.assertTrue(check(parse_program(good)))
        for body in ("recur<Vec<T>>([x])", "recur([x])"):
            with self.assertRaises(Rejected):
                check(parse_program(f"fn recur<T>(x: T) -> T {body} end"))

    def test_checked_source_drives_shared_erased_methods(self):
        from pathlib import Path
        declarations = parse_program(Path(__file__).with_name("functions.veln").read_text())
        java = emit_java(declarations)
        self.assertIn("Object identity(Object value)", java)
        self.assertNotIn("<T>", java)
        self.assertIn("wrap(java.util.List.of(value))", java)
        self.assertEqual(len(check(declarations)), 5)


if __name__ == "__main__":
    unittest.main()
