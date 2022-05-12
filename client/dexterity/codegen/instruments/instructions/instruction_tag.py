# LOCK-BEGIN[imports]: DON'T MODIFY
from podite import (
    Enum,
    U64,
    pod,
)
from solmate.anchor import InstructionDiscriminant

# LOCK-END


# LOCK-BEGIN[instruction_tag]: DON'T MODIFY
@pod
class InstructionTag(Enum[U64]):
    INITIALIZE_DERIVATIVE = InstructionDiscriminant()
    SETTLE_DERIVATIVE = InstructionDiscriminant()
    CLOSE_DERIVATIVE_ACCOUNT = InstructionDiscriminant()
    # LOCK-END
