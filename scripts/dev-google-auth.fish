#!/usr/bin/env -S fish --private

# Paste into the prompt, never into this file or a command argument.
set --erase fish_trace
set --local --unexport GOOGLE_CLIENT_SECRET
if test (count $argv) -ne 0
    echo 'Usage: just dev-auth' >&2
    exit 1
end

set --local clipboard_backend
if set --query WSL_INTEROP; or set --query WSL_DISTRO_NAME
    if not command --query powershell.exe
        echo 'Windows clipboard access requires powershell.exe through WSL interop.' >&2
        exit 1
    end
    set clipboard_backend windows
else if set --query WAYLAND_DISPLAY; and command --query wl-copy
    set clipboard_backend wayland
else if set --query DISPLAY; and command --query xclip
    set clipboard_backend x11
else
    echo 'Clipboard clearing requires wl-copy (Wayland), xclip (X11), or Windows PowerShell (WSL).' >&2
    exit 1
end

function clear_dev_clipboard --inherit-variable clipboard_backend
    switch $clipboard_backend
        case windows
            env -u GOOGLE_CLIENT_SECRET powershell.exe -NoLogo -NoProfile -NonInteractive -STA -Command '$ErrorActionPreference = "Stop"; Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Clipboard]::Clear()' >/dev/null
        case wayland
            env -u GOOGLE_CLIENT_SECRET wl-copy --clear
        case x11
            env -u GOOGLE_CLIENT_SECRET xclip -selection clipboard -in </dev/null
    end
end

set --local --export APP_ENVIRONMENT development
set --local --export MAJ_APP_ORIGIN 'http://localhost:8080'
set --local --export MAJ_DATA_DIR "$MAJ_DATA_DIR"
if test -z "$MAJ_DATA_DIR"
    set MAJ_DATA_DIR "$HOME/.local/share/maj-admin-dev"
end
set --local --export GOOGLE_CLIENT_ID "$GOOGLE_CLIENT_ID"
if test -z "$GOOGLE_CLIENT_ID"
    set GOOGLE_CLIENT_ID '312770222166-qtkr1gn7e22jpofqtt31vhbq88bj3ppb.apps.googleusercontent.com'
end
if not string match --quiet --regex '^[A-Za-z0-9_-]+\.apps\.googleusercontent\.com$' -- "$GOOGLE_CLIENT_ID"
    echo 'Supply the development OAuth client ID from maj-webpage-admin-development.' >&2
    exit 1
end

if not string match --quiet '/*' -- "$MAJ_DATA_DIR"
    echo 'MAJ_DATA_DIR must be an absolute development directory.' >&2
    exit 1
end
if not test -d "$MAJ_DATA_DIR"
    install -d -m 700 "$MAJ_DATA_DIR"
    or exit 1
end

read --local --unexport --silent --line --prompt-str 'Paste development Google client secret: ' maj_google_secret
set --local read_status $status
clear_dev_clipboard
set --local clipboard_status $status
if test $clipboard_status -ne 0
    set --erase maj_google_secret
    echo 'Clipboard clearing failed; the development server was not started.' >&2
    exit 1
end
if test $read_status -ne 0; or test -z "$maj_google_secret"
    set --erase maj_google_secret
    echo 'No secret entered; the development server was not started.' >&2
    exit 1
end

set --local --export GOOGLE_CLIENT_SECRET "$maj_google_secret"
set --erase maj_google_secret
cd (path resolve (status dirname)/..)
or exit 1
just _watch
set --local server_status $status
set --erase GOOGLE_CLIENT_SECRET
exit $server_status
