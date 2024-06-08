import marshal
import sys

def main() -> int:
    old_path = sys.argv[1]
    new_path = sys.argv[2]
    header_len = int(sys.argv[3])

    with open(old_path, "rb") as file:
        file.seek(header_len)
        old_marshal = marshal.load(file)

    with open(new_path, "rb") as file:
        file.seek(header_len)
        new_marshal = marshal.load(file)

    try:
        assert old_marshal == new_marshal
        return 0
    except AssertionError:
        return 1


if __name__ == "__main__":
    exit(main())
