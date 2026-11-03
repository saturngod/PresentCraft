#!/usr/bin/env bash
# Sets the app version, commits it and tags the release.
#
#   packaging/release.sh 0.2.0          # an exact version
#   packaging/release.sh patch          # 0.1.0 -> 0.1.1
#   packaging/release.sh minor          # 0.1.0 -> 0.2.0
#   packaging/release.sh major          # 0.1.0 -> 1.0.0
#
# Options:
#   --push      also push the branch and the tag (starts the GitHub release build)
#   --dry-run   show what would happen without changing anything
#
# The version goes into Cargo.toml and Cargo.lock, so the app, the package
# file names, the macOS bundle and the Windows exe all carry it. The tag is
# "v<version>", which .github/workflows/release.yml builds.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

push=0
dry=0
want=""
for arg in "$@"; do
  case "$arg" in
    --push) push=1 ;;
    --dry-run) dry=1 ;;
    -h|--help) sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) echo "Unknown option $arg" >&2; exit 1 ;;
    *) [[ -z "$want" ]] || { echo "Pass one version." >&2; exit 1; }; want="$arg" ;;
  esac
done
[[ -n "$want" ]] || { echo "Usage: packaging/release.sh <version|patch|minor|major> [--push] [--dry-run]" >&2; exit 1; }

current="$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)"
semver='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'
[[ "$current" =~ $semver ]] || { echo "Cargo.toml version '$current' is not x.y.z." >&2; exit 1; }

IFS=. read -r major minor patch <<<"${current%%-*}"
case "$want" in
  patch) new="$major.$minor.$((patch + 1))" ;;
  minor) new="$major.$((minor + 1)).0" ;;
  major) new="$((major + 1)).0.0" ;;
  *) new="${want#v}" ;;
esac
[[ "$new" =~ $semver ]] || { echo "'$want' is not a version like 1.2.3 (or patch, minor, major)." >&2; exit 1; }
tag="v$new"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "The working tree has uncommitted changes. Commit or stash them first." >&2
  exit 1
fi
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  echo "Tag $tag already exists." >&2
  exit 1
fi
if [[ "$push" == 1 ]] && git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
  echo "Tag $tag already exists on origin." >&2
  exit 1
fi

branch="$(git rev-parse --abbrev-ref HEAD)"
echo "Version: $current -> $new"
echo "Tag:     $tag on $branch"
if [[ "$dry" == 1 ]]; then
  echo "(dry run: nothing changed)"
  exit 0
fi

if [[ "$new" != "$current" ]]; then
  # First "version =" line in Cargo.toml is the [package] one.
  awk -v v="$new" '!done && /^version *= *"/ { print "version = \"" v "\""; done=1; next } { print }' \
    Cargo.toml >Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml
  # The app's own entry in Cargo.lock.
  awk -v v="$new" '
    /^name = "presentcraft"$/ { print; mine=1; next }
    mine && /^version = "/ { print "version = \"" v "\""; mine=0; next }
    { print }' Cargo.lock >Cargo.lock.tmp && mv Cargo.lock.tmp Cargo.lock

  # Fails if Cargo.lock no longer matches Cargo.toml.
  cargo metadata --locked --offline --format-version 1 --no-deps >/dev/null

  git add Cargo.toml Cargo.lock
  git commit -q -m "Release $tag"
  echo "Committed: Release $tag"
fi

git tag -a "$tag" -m "PresentCraft $new"
echo "Tagged:    $tag"

if [[ "$push" == 1 ]]; then
  git push origin "$branch"
  git push origin "$tag"
  echo "Pushed. GitHub Actions is now building the release."
else
  echo "Push when ready:"
  echo "  git push origin $branch && git push origin $tag"
fi
