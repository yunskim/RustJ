import unittest
from oracle import Oracle

class RationalOracleDecimal(unittest.TestCase):
    def test_finite_infinite_integral_and_empty_values_are_exact(self):
        self.assertEqual(Oracle.rational_decimal_atoms('1r2 _2r3 2 0 _ __',6),[
            {'numerator':'1','denominator':'2'}, {'numerator':'-2','denominator':'3'},
            {'numerator':'2','denominator':'1'}, {'numerator':'0','denominator':'1'},
            {'numerator':'1','denominator':'0'}, {'numerator':'-1','denominator':'0'}])
        self.assertEqual(Oracle.rational_decimal_atoms('',0),[])
    def test_long_fraction_does_not_use_float_or_python_integer_conversion(self):
        huge='9'*5000
        self.assertEqual(Oracle.rational_decimal_atoms(huge+'r2',1),[{'numerator':huge,'denominator':'2'}])
    def test_malformed_or_incomplete_output_is_not_a_match(self):
        for text,count in [('1r0',1),('1r_2',1),('1r',1),('1r2r3',1),('_.',1),('1 2',1),('1',2),('١r2',1)]:
            with self.assertRaises(RuntimeError):Oracle.rational_decimal_atoms(text,count)
