from dexterity.scripts.extract_program_ids import get_program_to_id, get_root
import os
import subprocess


def main():
    programs = get_program_to_id()
    root = get_root()
    cmd = "solana-test-validator"
    args = ""
    for (name, pid) in programs.items():
        path = f"{root}/target/deploy/{name}.so"
        args += f" --bpf-program {pid} {path}"
    print(f"Running {cmd} {args}")
    print(os.system(cmd + " " + args))


if __name__ == "__main__":
    main()
