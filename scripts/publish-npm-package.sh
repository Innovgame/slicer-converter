#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
package_dir="$root/packages/slicer-wasm"
name="$(node -p "require('$package_dir/package.json').name")"
version="$(node -p "require('$package_dir/package.json').version")"

publish_or_skip_existing() {
    local registry="$1"
    local label="$2"
    shift 2

    if npm publish --ignore-scripts "$@" --registry "$registry"; then
        echo "Published $name@$version to $label."
        return 0
    fi

    if npm view "$name@$version" version --registry "$registry" >/dev/null 2>&1; then
        echo "$name@$version is already on $label."
        return 0
    fi

    echo "Failed to publish $name@$version to $label." >&2
    return 1
}

cd "$package_dir"

github_status=0
if [ -n "${GITHUB_TOKEN:-}" ]; then
    printf '%s\n' "@innovgame:registry=https://npm.pkg.github.com" "//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}" >"$HOME/.npmrc"
    publish_or_skip_existing "https://npm.pkg.github.com" "GitHub Packages" || github_status=$?
else
    echo "GITHUB_TOKEN is not set. Skipping GitHub Packages."
fi

npm_status=0
if [ -n "${NPM_TOKEN:-}" ]; then
    printf '%s\n' "//registry.npmjs.org/:_authToken=${NPM_TOKEN}" >"$HOME/.npmrc"
    publish_or_skip_existing "https://registry.npmjs.org" "npm" --access public || npm_status=$?
else
    echo "NPM_TOKEN is not set. Skipping npm."
fi

if [ "$github_status" -ne 0 ] || [ "$npm_status" -ne 0 ]; then
    exit 1
fi
