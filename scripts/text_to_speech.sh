#!/usr/bin/env bash
#
# Turns a text into speech through the ElevenLabs API and saves the audio to a file.
#
#   scripts/text_to_speech.sh 121                      # page 121, saved as <book>/audio/121.mp3
#   scripts/text_to_speech.sh -t "Bonjour, aventurier." -o bonjour.mp3
#   scripts/text_to_speech.sh -f page.txt -o page.mp3
#   echo "Bonjour" | scripts/text_to_speech.sh -o bonjour.mp3
#   scripts/text_to_speech.sh -l
#
# The key is ELEVENLABS_API_KEY, from the environment or from the repository's .env. Nothing in
# the application reads it: this is a standalone tool, run by hand.

set -euo pipefail

readonly API_URL="https://api.elevenlabs.io/v1"
readonly REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly DEFAULT_VOICE_ID="JBFqnCBsd6RMkjVDRZzb"
readonly DEFAULT_MODEL_ID="eleven_multilingual_v2"
readonly DEFAULT_FORMAT="mp3_44100_128"
readonly DEFAULT_BOOK="$REPO_ROOT/books/pretre_jean/forteresse_alamuth"
# Where a page's audio is kept inside its book, next to pages.json, so it ships and can be served
# with the book: <book>/audio/<page>.<extension>.
readonly AUDIO_DIR="audio"
# The longest text eleven_multilingual_v2 accepts in one request.
readonly MAX_CHARACTERS=10000

usage() {
    cat <<EOF
Usage: $(basename "$0") [options] [PAGE]

Turns a text into speech with ElevenLabs and saves the audio.

  PAGE, -p, --page N     read page N of the book (its paragraphs, from pages.json)
  -b, --book DIR         the book the page is taken from (default: $DEFAULT_BOOK)
  -t, --text TEXT        the text to read
  -f, --file PATH        a file holding the text to read (stdin when no page, -t or -f is given)
  -o, --output PATH      the audio file to write (default: <book>/$AUDIO_DIR/N.mp3 for a page,
                         else speech.mp3)
  -v, --voice ID         the voice id (default: \$ELEVENLABS_VOICE_ID, else $DEFAULT_VOICE_ID)
  -m, --model ID         the model id (default: $DEFAULT_MODEL_ID)
  -F, --format FORMAT    the ElevenLabs output format (default: $DEFAULT_FORMAT)
  -l, --list-voices      list the voices the account can use, then exit
  -h, --help             show this help

The API key is ELEVENLABS_API_KEY, from the environment or from $REPO_ROOT/.env.
EOF
}

fail() {
    echo "error: $*" >&2
    exit 1
}

# Reads one KEY=value line out of .env without sourcing it, so the file runs nothing.
read_dotenv() {
    local key="$1" env_file="$REPO_ROOT/.env"
    [[ -f "$env_file" ]] || return 0
    grep -E "^${key}=" "$env_file" | tail -n 1 | cut -d= -f2- | sed -e 's/^"//' -e 's/"$//' || true
}

api_key() {
    local key="${ELEVENLABS_API_KEY-}"
    [[ -n "$key" ]] || key="$(read_dotenv ELEVENLABS_API_KEY)"
    [[ -n "$key" ]] || fail "ELEVENLABS_API_KEY is not set (environment or $REPO_ROOT/.env)"
    printf '%s' "$key"
}

# Prints the paragraphs of one page of a book's pages.json, separated by blank lines.
page_text() {
    local book="$1" page="$2" pages_file="$1/pages.json" text
    [[ -f "$pages_file" ]] || fail "no pages.json in $book"
    text="$(jq -r --arg page "$page" \
        'map(select(.page == $page)) | first | if . == null then empty else .text | join("\n\n") end' \
        "$pages_file")" || fail "$pages_file is not valid JSON"
    [[ -n "$text" ]] || fail "page $page is not in $pages_file"
    printf '%s' "$text"
}

# Prints where a page's audio belongs in its book; the extension follows the format (mp3_44100_128
# gives mp3, opus_48000_64 gives opus).
page_audio_path() {
    local book="$1" page="$2" format="$3"
    printf '%s/%s/%s.%s' "$book" "$AUDIO_DIR" "$page" "${format%%_*}"
}

list_voices() {
    local key
    key="$(api_key)"
    curl --silent --show-error --fail-with-body \
        --header "xi-api-key: $key" \
        "$API_URL/voices" \
        | jq -r '.voices[] | "\(.voice_id)\t\(.name)"'
}

# Posts the text and moves the audio to the output path, or prints the API's error and fails.
synthesize() {
    local text="$1" output="$2" voice="$3" model="$4" format="$5"
    local key body tmp status
    key="$(api_key)"
    body="$(jq -n --arg text "$text" --arg model_id "$model" '{text: $text, model_id: $model_id}')"

    tmp="$(mktemp "${TMPDIR:-/tmp}/text_to_speech.XXXXXX")"
    trap 'rm -f "$tmp"' EXIT

    status="$(curl --silent --show-error \
        --output "$tmp" --write-out '%{http_code}' \
        --header "xi-api-key: $key" \
        --header "Content-Type: application/json" \
        --data "$body" \
        "$API_URL/text-to-speech/$voice?output_format=$format")"

    if [[ "$status" != "200" ]]; then
        echo "error: ElevenLabs answered HTTP $status" >&2
        cat "$tmp" >&2
        echo >&2
        exit 1
    fi

    mv "$tmp" "$output"
    echo "$output ($(wc -c <"$output" | tr -d ' ') bytes)"
}

main() {
    local text="" file="" output="" format="$DEFAULT_FORMAT" page="" book="$DEFAULT_BOOK"
    local voice="${ELEVENLABS_VOICE_ID-}" model="$DEFAULT_MODEL_ID" has_text=false
    [[ -n "$voice" ]] || voice="$(read_dotenv ELEVENLABS_VOICE_ID)"
    [[ -n "$voice" ]] || voice="$DEFAULT_VOICE_ID"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            -p|--page)        [[ $# -ge 2 ]] || fail "$1 needs a value"; page="$2"; shift 2 ;;
            -b|--book)        [[ $# -ge 2 ]] || fail "$1 needs a value"; book="$2"; shift 2 ;;
            -t|--text)        [[ $# -ge 2 ]] || fail "$1 needs a value"; text="$2"; has_text=true; shift 2 ;;
            -f|--file)        [[ $# -ge 2 ]] || fail "$1 needs a value"; file="$2"; shift 2 ;;
            -o|--output)      [[ $# -ge 2 ]] || fail "$1 needs a value"; output="$2"; shift 2 ;;
            -v|--voice)       [[ $# -ge 2 ]] || fail "$1 needs a value"; voice="$2"; shift 2 ;;
            -m|--model)       [[ $# -ge 2 ]] || fail "$1 needs a value"; model="$2"; shift 2 ;;
            -F|--format)      [[ $# -ge 2 ]] || fail "$1 needs a value"; format="$2"; shift 2 ;;
            -l|--list-voices) list_voices; exit 0 ;;
            -h|--help)        usage; exit 0 ;;
            -*)               usage >&2; fail "unknown option: $1" ;;
            *)                [[ -z "$page" ]] || fail "one page at a time: $page, then $1"
                              page="$1"; shift ;;
        esac
    done

    local sources=0
    [[ "$has_text" == false ]] || sources=$((sources + 1))
    [[ -z "$file" ]] || sources=$((sources + 1))
    [[ -z "$page" ]] || sources=$((sources + 1))
    (( sources <= 1 )) || fail "give the text with one of a page, -t or -f"

    if [[ -n "$page" ]]; then
        [[ "$page" =~ ^[0-9]+$ ]] || fail "not a page number: $page"
        text="$(page_text "$book" "$page")"
        if [[ -z "$output" ]]; then
            output="$(page_audio_path "$book" "$page" "$format")"
            mkdir -p "$(dirname "$output")"
        fi
    elif [[ -n "$file" ]]; then
        [[ -f "$file" ]] || fail "no such file: $file"
        text="$(cat "$file")"
    elif [[ "$has_text" == false ]]; then
        [[ ! -t 0 ]] || fail "no text given (use a page number, -t, -f or stdin; -h for help)"
        text="$(cat)"
    fi

    [[ -n "$output" ]] || output="speech.mp3"
    [[ -n "${text//[[:space:]]/}" ]] || fail "the text is empty"
    (( ${#text} <= MAX_CHARACTERS )) \
        || fail "the text is ${#text} characters long, more than the $MAX_CHARACTERS one request takes"

    synthesize "$text" "$output" "$voice" "$model" "$format"
}

main "$@"
