# LOCK-BEGIN[imports]: DON'T MODIFY
from podite import (
    I64,
    U64,
    pod,
)

# LOCK-END

import numpy as np
from typing import Tuple, Union
from decimal import *


MAX_PRECISION = 10

MAX_FRACTIONAL_M = 2 ** 63 - 1
MAX_FRACTIONAL_EXP = 0

MIN_FRACTIONAL_M = -(2 ** 63)
MIN_FRACTIONAL_EXP = 0


# LOCK-BEGIN[class(Fractional)]: DON'T MODIFY
@pod
class Fractional:
    m: I64
    exp: U64
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)

    @classmethod
    def into(cls, x: Union["Fractional", float, int], default_decimals: int):
        if isinstance(x, cls):
            return x
        elif isinstance(x, float):
            return cls(int(x * (10 ** default_decimals)), default_decimals)
        elif isinstance(x, int):
            return cls(x, 0)
        else:
            raise TypeError(f"Expected Fractional, float or int, got {type(x)}")

    @classmethod
    def to_decimal(cls, num):
        num_dec = Decimal(str(float(num)))
        exp = -1 * num_dec.as_tuple().exponent
        num_int = I64(num * (10 ** exp))

        return cls(num_int, exp)

    @property
    def value(self):
        exp = -1 * self.exp
        return self.m * (10 ** exp)

    def __radd__(self, other):
        return Fractional.to_decimal(other) + self

    def __add__(self, other):
        if not isinstance(other, Fractional):
            other = Fractional.to_decimal(other)

        exp = max(self.exp, other.exp)
        m = self.m * 10 ** (exp - self.exp) + other.m * 10 ** (exp - other.exp)
        return Fractional(m, exp).simplify()

    def __rmul__(self, other):
        return Fractional.to_decimal(other) * self

    def __mul__(self, other):
        if not isinstance(other, Fractional):
            other = Fractional.to_decimal(other)

        m = self.m * other.m
        exp = self.exp + other.exp
        return Fractional(m, exp).simplify()

    def __div__(self, other):
        if not isinstance(other, Fractional):
            other = Fractional.to_decimal(other)
        return Fractional.to_decimal(round(self.value / other.value, 6))

    def __rdiv__(self, other):
        if not isinstance(other, Fractional):
            other = Fractional.to_decimal(other)
        return Fractional.to_decimal(round(self.value / other.value, 6))

    def round_sf(self, digits):
        return Fractional.to_decimal(round(self.value, digits))

    def sqrt(self):
        return Fractional.to_decimal(round(np.sqrt(self.value), 2))

    def simplify(self):
        m = self.m
        exp = self.exp

        if m == 0:
            return Fractional(0, 0)

        while m % 10 == 0:
            m //= 10
            exp -= 1
        return Fractional(m, exp)

    def __repr__(self):
        if self.m == MAX_FRACTIONAL_M and self.exp == MAX_FRACTIONAL_EXP:
            return "Inf"
        if self.m == MIN_FRACTIONAL_M and self.exp == MIN_FRACTIONAL_EXP:
            return "-Inf"
        return str(round(self.value, MAX_PRECISION))