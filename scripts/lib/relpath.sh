# Compute relative path from $1 to $2, both absolute and normalized.
relpath() {
    # $1 = from (e.g. submodule working dir)
    # $2 = to   (e.g. hooks directory absolute path)
    awk -v from="$1" -v to="$2" '
    BEGIN {
        # Split paths into components
        fn = split(from, f, "/")
        tn = split(to,   t, "/")

        # Find common prefix length
        common = 0
        for (i = 1; i <= fn && i <= tn; i++) {
            if (f[i] == t[i]) common = i
            else break
        }

        # Build result: (fn - common) ".." segments, then remaining t components
        result = ""
        for (i = common + 1; i <= fn; i++) {
            result = result "../"
        }
        for (i = common + 1; i <= tn; i++) {
            result = result t[i] (i < tn ? "/" : "")
        }

        if (result == "") result = "."
        # Strip trailing slash if we built one
        sub(/\/$/, "", result)
        print result
    }'
}
