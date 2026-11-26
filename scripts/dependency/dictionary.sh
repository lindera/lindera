# The dictionary of a measurement script: sourced by the scripts that take
# `--dictionary <name>` and `--dict <URI or path>`.
#
# After `parse_dictionary_arguments "$@"`:
#   dictionary  The name: ipadic (the default), unidic, unidic-cwj or
#               unidic-csj.
#   dict        What `--dict` gave, or empty.
#   rest        The other arguments, an array.
# `resolve_dictionary <root> <data directory>` then sets `dict` when it is
# empty, and `dictionary_directory <data directory>` prints where the files
# made with the dictionary go.

dictionary=ipadic
dict=
rest=()

# Reads `--dictionary` and `--dict` from the arguments of a script.
parse_dictionary_arguments() {
    dictionary=ipadic
    dict=
    rest=()
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --dictionary | --dict)
                if [ "$#" -lt 2 ]; then
                    echo "$1 needs a value" >&2
                    exit 2
                fi
                if [ "$1" = --dictionary ]; then dictionary=$2; else dict=$2; fi
                shift 2
                ;;
            *)
                rest+=("$1")
                shift
                ;;
        esac
    done
    case "$dictionary" in
        ipadic | unidic | unidic-cwj | unidic-csj) ;;
        *)
            echo "there is no schema for the dictionary $dictionary: ipadic, unidic, unidic-cwj or unidic-csj" >&2
            exit 2
            ;;
    esac
}

# Sets `dict` when `--dict` did not: the embedded IPADIC, or the dictionary
# `lindera download` installs into <data directory>/dictionaries.
resolve_dictionary() {
    local root=$1 data=$2
    if [ -n "$dict" ]; then
        return
    fi
    if [ "$dictionary" = ipadic ]; then
        dict=embedded://ipadic
        return
    fi
    cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-cli
    mkdir -p "$data/dictionaries"
    # The path is the only line of the standard output.
    dict=$(env -u LINDERA_DATA_DIR LINDERA_DICTIONARIES_DIR="$data/dictionaries" \
        "$root/target/release/lindera" download "$dictionary")
    if [ ! -d "$dict" ]; then
        echo "lindera download $dictionary did not give a directory: $dict" >&2
        exit 1
    fi
}

# Prints the directory of the files made with the dictionary: the data
# directory itself for ipadic, and one of the name of the dictionary in it
# for another.
dictionary_directory() {
    local data=$1
    if [ "$dictionary" = ipadic ]; then
        printf '%s\n' "$data"
    else
        printf '%s\n' "$data/$dictionary"
    fi
}
