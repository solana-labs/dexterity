from hashlib import sha256
import sys


def main():
    input = sys.argv[1]
    print(sighash(input).hex())
    print(sighash_int(input))


def sighash(ix_name: str) -> bytes:
    """Not technically sighash, since we don't include the arguments.
    (Because Rust doesn't allow function overloading.)
    Args:
        ix_name: The instruction name.
    Returns:
        The sighash bytes.
    """
    formatted_str = f"global:{ix_name}"
    return sha256(formatted_str.encode()).digest()[:8]


def sighash_int(ix_name: str) -> int:
    return int.from_bytes(sighash(ix_name), byteorder="little")


if __name__ == "__main__":
    main()
