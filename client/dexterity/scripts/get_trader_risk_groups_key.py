import argparse
from base64 import b64decode as b64d
from base58 import b58encode as b58e
from solana.rpc.api import Client
from solana.rpc.types import MemcmpOpts


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dex_program_id")
    ap.add_argument("owner")
    ap.add_argument("--network", default="devnet")
    ap.add_argument("--market_product_group", default=None)
    args = ap.parse_args()

    urls = {
        "devnet": "https://api.devnet.solana.com",
        "dev": "https://api.devnet.solana.com",
        "localnet": "https://localhost:8899/",
        "local": "https://localhost:8899/",
        "mainnet": "https://api.mainnet-beta.solana.com/",
        "mainnet-beta": "https://api.mainnet-beta.solana.com/",
    }

    client = Client(urls[args.network])

    print(f"owner: {args.owner}")
    mem_cmp = [MemcmpOpts(8 + 33, args.owner)]
    if args.market_product_group != None:
        mem_cmp.append(MemcmpOpts(8 + 1, args.market_product_group))
        print(f"market product group: {args.market_product_group}")
    print("Searching...\n")
    resp = client.get_program_accounts(
        args.dex_program_id,
        "confirmed",
        encoding="base64",
        memcmp_opts=mem_cmp,
    )

    mpgs = {}
    for account in resp["result"]:
        data = b64d(account["account"]["data"][0])
        mpg = b58e(data[9:41]).decode('ascii')
        mpgs[mpg] = mpgs.get(mpg, []) + [account["pubkey"]]

    for k in mpgs:
        print(k)
        for trg in mpgs[k]:
            print(f"\t{trg}")


if __name__ == '__main__':
    main()