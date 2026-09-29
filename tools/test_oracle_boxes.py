"""Adapter tests with synthetic C descriptors; not a real J differential run."""
import ctypes as C
import re
import unittest
from oracle import Oracle


class DescriptorLibrary:
    def __init__(self, value):
        self.nouns = {'rustjresult': value}
        self.buffers = []

    def JGetM(self, jt, name, *outputs):
        noun = self.nouns[name.decode('ascii')]
        shape = (C.c_int64 * max(1, len(noun['shape'])))(*noun['shape'])
        elem = {1: C.c_uint8, 2: C.c_uint8, 4: C.c_int64, 8: C.c_double, 32: C.c_int64}[noun['type']]
        data = noun['data'] if noun['type'] != 32 else [0] * len(noun['data'])
        storage = (elem * max(1, len(data)))(*data)
        self.buffers.extend([shape, storage])
        for pointer, value in zip(outputs, [noun['type'], len(noun['shape']), C.addressof(shape), C.addressof(storage)]):
            C.cast(pointer, C.POINTER(C.c_int64))[0] = value
        return 0

    def JDo(self, jt, source):
        match = re.fullmatch(r'(\w+) =: > (\d+) \{ , (\w+)', source.decode('ascii'))
        if match is None:
            raise AssertionError(source)
        child, index, parent = match.groups()
        self.nouns[child] = self.nouns[parent]['data'][int(index)]
        return 0


class BoxReaderTests(unittest.TestCase):
    def test_nested_and_sibling_boxes_keep_parent_names_intact(self):
        number = {'type': 4, 'shape': [], 'data': [42]}
        text = {'type': 2, 'shape': [2], 'data': [97, 98]}
        empty = {'type': 32, 'shape': [0], 'data': []}
        nested = {'type': 32, 'shape': [], 'data': [text]}
        root = {'type': 32, 'shape': [2, 2], 'data': [nested, number, empty, nested]}
        oracle = Oracle.__new__(Oracle)
        oracle.jt = 1
        oracle.lib = DescriptorLibrary(root)
        self.assertEqual(oracle.read_noun('rustjresult'), root)
        self.assertEqual(oracle.read_noun('rustjresult'), root)

    def test_depth_guard_precedes_descriptor_read(self):
        oracle = Oracle.__new__(Oracle)
        with self.assertRaisesRegex(RuntimeError, 'nesting limit'):
            oracle.read_noun('unused', 129)
