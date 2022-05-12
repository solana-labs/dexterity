# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.fractional import Fractional
from dexterity.codegen.instruments.types.instrument_type import InstrumentType
from dexterity.codegen.instruments.types.oracle_type import OracleType
from podite import pod
from solana.publickey import PublicKey
from solmate.dtypes import UnixTimestamp

# LOCK-END


# LOCK-BEGIN[class(InitializeDerivativeParams)]: DON'T MODIFY
@pod
class InitializeDerivativeParams:
    instrument_type: "InstrumentType"
    strike: Fractional
    full_funding_period: UnixTimestamp
    minimum_funding_period: UnixTimestamp
    initialization_time: UnixTimestamp
    close_authority: PublicKey
    oracle_type: "OracleType"
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
