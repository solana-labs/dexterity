from enum import IntEnum

from podite import U8, Enum, pod


@pod
class InstructionCode(Enum[U8]):
    INITIALIZE_CLOCK = None
    INITIALIZE_ORACLE = None
    UPDATE_CLOCK = None
    UPDATE_PRICE = None
