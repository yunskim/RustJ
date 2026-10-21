"""Conv1D algebraic-view contract checks; standalone, standard-library only.

Run: python3 tests/conv1d_algebraic_view_contract.py
This tests the design's mathematical identities, not RustJ IR integration.
"""
import math
import random


def conv(x, w):
    return [sum(x[p + k] * w[k] for k in range(len(w)))
            for p in range(len(x) - len(w) + 1)]


def adj_input(dy, w, input_length):
    out = [0.0] * input_length
    for p, v in enumerate(dy):
        for k, weight in enumerate(w):
            out[p + k] += v * weight
    return out


def adj_weight(x, dy, kernel_length):
    return [sum(x[p + k] * dy[p] for p in range(len(dy)))
            for k in range(kernel_length)]


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def close_seq(a, b):
    return len(a) == len(b) and all(
        math.isclose(x, y, rel_tol=1e-11, abs_tol=1e-11)
        for x, y in zip(a, b)
    )


def test_contract():
    x, w, dy = [1., 2., 3., 4.], [2., 3.], [1., 2., 3.]
    assert conv(x, w) == [8., 13., 18.]
    assert adj_input(dy, w, len(x)) == [2., 7., 12., 9.]
    assert adj_weight(x, dy, len(w)) == [14., 20.]
    assert dot(conv(x, w), dy) == dot(x, adj_input(dy, w, len(x))) == dot(w, adj_weight(x, dy, len(w))) == 88.

    rng = random.Random(31)
    for _ in range(1000):
        n = rng.randint(3, 19)
        k = rng.randint(1, n)
        x = [rng.uniform(-1, 1) for _ in range(n)]
        w = [rng.uniform(-1, 1) for _ in range(k)]
        w2 = [rng.uniform(-1, 1) for _ in range(k)]
        dy = [rng.uniform(-1, 1) for _ in range(n - k + 1)]
        lhs = dot(conv(x, w), dy)
        assert math.isclose(lhs, dot(x, adj_input(dy, w, n)), rel_tol=1e-11, abs_tol=1e-11)
        assert math.isclose(lhs, dot(w, adj_weight(x, dy, k)), rel_tol=1e-11, abs_tol=1e-11)
        assert close_seq([a + b for a, b in zip(conv(x, w), conv(x, w2))],
                         conv(x, [a + b for a, b in zip(w, w2)]))


if __name__ == "__main__":
    test_contract()
    print("PASS: deterministic contract + 1000 randomized cases")
