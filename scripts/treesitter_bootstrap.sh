#!/usr/bin/env sh

set -eu

dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

lang_pkg="tree-sitter-rust@0.23.1"
lang_dir="$dir/.$lang_pkg"

if [ ! -e "$lang_dir" ]; then
    mkdir "$lang_dir"
    curl -s "$(npm v "$lang_pkg" dist.tarball)" | tar -xz --strip-components=1 --directory "$lang_dir"
fi

# tree-sitter-stack-graphs index src -v -f --grammar .tree-sitter-rust@0.23.1

# tree-sitter-stack-graphs status .

# tree-sitter-stack-graphs query definition src/builder.rs:16:4

# sqlite3 "/Users/evanfeenstra/Library/Application Support/tree-sitter-stack-graphs.sqlite"
