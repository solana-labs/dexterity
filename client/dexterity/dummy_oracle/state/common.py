from podite import U8, Enum, pod


@pod
class AccountTag(Enum[U8]):
    UNINITIALIZED = None
    ORACLE_PRICE = None
