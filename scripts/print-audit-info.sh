#!/bin/sh

set -eu

uid=$(id -u)
user=$(id -un)
gid=$(id -g)
grp=$(id -gn)
t=$(date +%s)
kern=$(uname -s)
krel=$(uname -r)
arch=$(uname -m)
host=$(uname -n)

v4=
v6=

# Resolve this script's directory once so we can invoke sibling
# helpers without depending on CWD — print-audit-info.sh is
# called from inside `commit-all.sh` where CWD is the workspace
# root, and potentially from other callers later.
_here=$(cd -- "$(dirname -- "$0")" && pwd)

if [ "${1:-}" = "--anonymize" ] ; then
    v4=hidden/ZZ
    v6=hidden/ZZ
else
    # `web-fetch.sh` picks whichever of curl / wget / fetch /
    # ftp is available; we don't need to branch here. `|| :`
    # keeps the audit line produced even if the network is down
    # or the workspace convention's fetch tool is missing — an
    # empty `4=`/`6=` field is acceptable, a failed commit is not.
    tmp_v4=$("$_here/mktemp.sh" tmp_v4)
    tmp_v6=$("$_here/mktemp.sh" tmp_v6)
    trap 'rm -f "$tmp_v4" "$tmp_v6"' EXIT INT HUP TERM
    "$_here/web-fetch.sh" https://1.1.1.1/cdn-cgi/trace "$tmp_v4" || :
    "$_here/web-fetch.sh" 'https://[2606:4700:4700::1111]/cdn-cgi/trace' "$tmp_v6" || :

    v4="$(awk -F= '/^ip=/{print $2}' < "$tmp_v4")/$(awk -F= '/^loc=/{print $2}' < "$tmp_v4")"
    v6="$(awk -F= '/^ip=/{print $2}' < "$tmp_v6")/$(awk -F= '/^loc=/{print $2}' < "$tmp_v6")"
fi

os=

case "$kern" in
    Linux)
        os="$(grep '^ID=' /etc/os-release | sed 's/^ID=//' | tr -d '"')_$(grep '^VERSION_ID=' /etc/os-release | sed 's/^VERSION_ID=//' | tr -d '"')"
        ;;
    FreeBSD)
        os=$(uname -U)
        ;;
    Darwin)
        os=$(sw_vers -productVersion)
        ;;
esac

# Rust toolchain version from the committing environment. Empty
# if rustc isn't on PATH — commits without Rust on the box (rare,
# but possible for a docs-only machine) shouldn't fail here.
rust=
if command -v rustc > /dev/null 2>&1 ; then
    rust=$(rustc --version 2>/dev/null | awk '{print $2}')
fi

echo t="${t}" h="${host}" u="${user}/${uid}" g="${grp}/${gid}" 4="${v4}" 6="${v6}" k="${kern}/${krel}" a="${arch}" o="${os}" r="${rust}"
