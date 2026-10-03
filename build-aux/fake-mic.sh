#!/bin/sh
# Virtual microphone for testing dictation without speaking (docs/MANUAL_TESTS.md).
#   fake-mic.sh on                  make a silent virtual microphone the default input
#   fake-mic.sh play FILE [DELAY]   after DELAY seconds (default 3), start dictation and "speak" FILE
#   fake-mic.sh off                 restore the previous input, remove the microphone
set -eu
state="${XDG_RUNTIME_DIR:-/tmp}/diktu-fake-mic"
# Command that starts dictation before the clip; the development Flatpak by default.
DIKTU_TOGGLE="${DIKTU_TOGGLE:-flatpak run fr.gwenael_leger.Diktu.Devel --toggle}"

node_id() {
    wpctl inspect "$1" 2>/dev/null | sed -n '1s/^id \([0-9]*\),.*/\1/p'
}

case "${1:-}" in
on)
    [ -e "$state" ] && { echo "already on"; exit 0; }
    previous=$(node_id @DEFAULT_AUDIO_SOURCE@)
    # Whatever is played to the "fakemic" sink comes out of the "fakemic_src" source.
    setsid pw-loopback -m '[ FL FR ]' \
        --capture-props='media.class=Audio/Sink node.name=fakemic node.description=FakeMic' \
        --playback-props='media.class=Audio/Source node.name=fakemic_src node.description=FakeMicInput' \
        >/dev/null 2>&1 &
    pid=$!
    for _ in 1 2 3 4 5 6 7 8 9 10; do
        source=$(pw-dump 2>/dev/null | grep -c '"node.name": "fakemic_src"' || true)
        [ "$source" -gt 0 ] && break
        sleep 0.2
    done
    id=$(pw-cli ls Node | awk '/^\tid/ {id=$2} /node.name = "fakemic_src"/ {print id}' | tr -d ,)
    [ -n "$id" ] || { kill "$pid"; echo "could not create the fake microphone"; exit 1; }
    printf '%s\n%s\n' "${previous:-}" "$pid" > "$state"
    wpctl set-default "$id"
    echo "fake microphone on (previous input: node ${previous:-none})"
    ;;
play)
    file="${2:?usage: fake-mic.sh play FILE [DELAY]}"
    delay="${3:-3}"
    while [ "$delay" -gt 0 ]; do
        printf '\rplaying in %s s… (put the cursor in the target app) ' "$delay"
        sleep 1
        delay=$((delay - 1))
    done
    printf '\rstarting dictation, playing %s\n' "$file"
    # Starting dictation here keeps it in step with the clip: by hand, it was either too
    # early (the no-speech timeout fired) or too late (the start of the clip was lost).
    $DIKTU_TOGGLE
    sleep 0.5
    pw-play --target fakemic "$file"
    ;;
off)
    [ -e "$state" ] || { echo "already off"; exit 0; }
    { read -r previous; read -r pid; } < "$state"
    kill "$pid" 2>/dev/null || true
    [ -n "$previous" ] && wpctl set-default "$previous" || true
    rm -f "$state"
    echo "fake microphone off (input: node ${previous:-default})"
    ;;
*)
    sed -n '3,5s/^# //p' "$0"
    exit 1
    ;;
esac
