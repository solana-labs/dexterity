from podite import pod, U64, I64


@pod
class Clock:
    slot: U64
    epoch_start_timestamp: I64
    epoch: U64
    leader_schedule_epoch: U64
    unix_timestamp: I64
