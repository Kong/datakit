#!/usr/bin/env bash

# This is just a script I hacked together for quick feedback.
# It is not currently used in CI.

set -euo pipefail

DIR=$(dirname "$(realpath "$0")")

readonly META=$DIR/../../datakit.meta.json
readonly POSITIVE=$DIR/schema.json
readonly NEGATIVE=$DIR/negative.json
readonly OUTPUT=$DIR/out.txt

readonly DEPS=(
    jq
    yq
    jsonschema-cli
)

cleanup() {
    rm "$POSITIVE" "$NEGATIVE" "$OUTPUT" || true
}

fail() {
    echo "FAIL: $1"
    exit 1
}

check() {
    local -r fname=$1
    local -r schema=$2

    if jsonschema-cli \
        -i <(yq -o json < "$fname") \
        "$schema" \
        &>"$OUTPUT";
    then
        printf -- '+ %s (%s)\n' "${fname#"$DIR"/}" "${schema#"$DIR"/}"
        return 0
    fi

    printf -- '- %s (%s)\n' "${fname#"$DIR"/}" "${schema#"$DIR"/}"
    return 1
}

assert-pass() {
    local -r fname=$1
    local -r schema=$2

    if check "$fname" "$schema"; then
        return 0
    fi

    cat "$OUTPUT"
    fail "Expected ${fname#"$DIR"/} to pass ${schema#"$DIR"/}"
}

assert-not-pass() {
    local -r fname=$1
    local -r schema=$2

    if check "$fname" "$schema"; then
        cat "$OUTPUT"
        fail "Expected ${fname#"$DIR"/} NOT to pass ${schema#"$DIR"/}"
    fi
}

setup() {
    jq .config_schema \
        < "$META" \
        > "$DIR"/schema.json

    jq '{
        not: .config_schema,
        "$schema": .config_schema["$schema"],
        definitions: .config_schema.definitions
      }' \
        < "$META" \
        > "$DIR"/negative.json

    assert-pass "$POSITIVE" "$DIR"/draft-04.json
    assert-pass "$NEGATIVE" "$DIR"/draft-04.json
}

main() {
    for dep in "${DEPS[@]}"; do
        if ! command -v "$dep" &>/dev/null; then
            fail "missing dependency: $dep"
        fi
    done

    trap cleanup ERR EXIT
    setup

    for f in "$DIR"/invalid/*.yml; do
        assert-pass "$f" "$NEGATIVE"
        assert-not-pass "$f" "$POSITIVE"
    done

    for f in "$DIR"/valid/*.yml; do
        assert-pass "$f" "$POSITIVE"
        assert-not-pass "$f" "$NEGATIVE"
    done
}

main "$@"
