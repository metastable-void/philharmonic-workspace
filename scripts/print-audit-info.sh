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

if [ "${1:-}" = "--anonymize" ] ; then
    v4=hidden/ZZ
    v6=hidden/ZZ
elif command -v curl > /dev/null 2>&1 ; then
    tmp_v4=$(mktemp /tmp/tmp_v4.XXXXXX)
    tmp_v6=$(mktemp /tmp/tmp_v6.XXXXXX)
    curl -s https://1.1.1.1/cdn-cgi/trace > "$tmp_v4" 2>/dev/null || :
    curl -s https://[2606:4700:4700::1111]/cdn-cgi/trace > "$tmp_v6" 2>/dev/null || :

    v4="$(awk -F= '/^ip=/{print $2}' < "$tmp_v4")/$(awk -F= '/^loc=/{print $2}' < "$tmp_v4")"
    v6="$(awk -F= '/^ip=/{print $2}' < "$tmp_v6")/$(awk -F= '/^loc=/{print $2}' < "$tmp_v6")"

    rm -f "$tmp_v4" "$tmp_v6"
elif command -v wget > /dev/null 2>&1 ; then
    tmp_v4=$(mktemp /tmp/tmp_v4.XXXXXX)
    tmp_v6=$(mktemp /tmp/tmp_v6.XXXXXX)
    wget -q -O "$tmp_v4" https://1.1.1.1/cdn-cgi/trace >/dev/null 2>&1 || :
    wget -q -O "$tmp_v6" https://[2606:4700:4700::1111]/cdn-cgi/trace >/dev/null 2>&1 || :

    v4="$(awk -F= '/^ip=/{print $2}' < "$tmp_v4")/$(awk -F= '/^loc=/{print $2}' < "$tmp_v4")"
    v6="$(awk -F= '/^ip=/{print $2}' < "$tmp_v6")/$(awk -F= '/^loc=/{print $2}' < "$tmp_v6")"

    rm -f "$tmp_v4" "$tmp_v6"
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

echo t="${t}" h="${host}" u="${user}/${uid}" g="${grp}/${gid}" 4="${v4}" 6="${v6}" k="${kern}/${krel}" a="${arch}" o="${os}"
