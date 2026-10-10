import unittest
from oracle import Oracle

class ExtendedOracleDecimal(unittest.TestCase):
    def test_exact_digits_signs_and_empty_arrays(self):
        huge='9'*5000
        self.assertEqual(Oracle.extended_decimal_atoms('1 _0 '+huge,3),['1','0',huge])
        self.assertEqual(Oracle.extended_decimal_atoms('_9007199254740993',1),['-9007199254740993'])
        self.assertEqual(Oracle.extended_decimal_atoms('',0),[])
    def test_malformed_or_incomplete_bridge_output_is_not_a_match(self):
        for text,count in [('1x',1),('_.',1),('_',1),('1',2),('1 2',1),('١',1)]:
            with self.assertRaises(RuntimeError):
                Oracle.extended_decimal_atoms(text,count)
