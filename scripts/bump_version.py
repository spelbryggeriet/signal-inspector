#!/usr/bin/env python3

from get_version import get_version as get_current_version
from util import error
import datetime
import os
import subprocess
import sys


SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))
REPO_DIR = os.path.realpath(os.path.join(SCRIPT_DIR, ".."))


def run(*cmd):
    output = subprocess.run(cmd, capture_output=True)
    return output.stdout.decode("utf-8").strip(), output.stderr.decode("utf-8").strip()


def get_next_version(bump_comp_idx, current_version):
    components = current_version.split(".")

    all_digits = lambda c: all(map(str.isdigit, c))
    is_empty = lambda c: len(c) == 0
    is_int = lambda c: not is_empty(c) and all_digits(c)

    has_three_components = len(components) == 3
    all_components_are_ints = all(map(is_int, components))

    if not (has_three_components and all_components_are_ints):
        error(f'"{current_version}" version number invalid')

    components[bump_comp_idx] = int(components[bump_comp_idx]) + 1
    [major, minor, patch] = [*components[:bump_comp_idx+1], 0, 0, 0][:3]

    return f"{major}.{minor}.{patch}"


def update_version_file(next_version, path):
    version_path = os.path.join(REPO_DIR, path)
    with open(version_path, "w") as f:
        f.write(next_version)

    return next_version


def update_hocfile(next_version, path, current_version):
    hocfile_path = os.path.join(REPO_DIR, path)
    with open(hocfile_path, "r") as f:
        content = f.read()

    content = content.replace(f'version: {current_version}', f'version: {next_version}', 1)

    with open(hocfile_path, "w") as f:
        f.write(content)


def update_manifest(next_version, path, current_version):
    manifest_path = os.path.join(REPO_DIR, path)
    with open(manifest_path, "r") as f:
        content = f.read()

    content = content.replace(f'version = "{current_version}"', f'version = "{next_version}"', 1)

    with open(manifest_path, "w") as f:
        f.write(content)


def update_changelog(next_version, path, repository_owner):
    HEADER_KEY = "## [Unreleased]"

    changelog_path = os.path.join(REPO_DIR, path)
    last_version, _ = run("git", "-C", REPO_DIR, "rev-list", "--date-order", "--tags", "--max-count=1")

    if len(last_version) > 0:
        stdout, stderr = run("git", "-C", REPO_DIR, "diff", last_version, "HEAD", "--", path)

        if len(stderr) > 0:
            error(stderr.strip())
        if len(stdout) == 0:
            error("no changes have been made to the changelog")

    with open(changelog_path, "r") as f:
        content = f.read()

    current_date = datetime.datetime.now(datetime.timezone.utc)
    formatted_date = current_date.strftime("%Y-%m-%d")
    new_header = f"## [{next_version}] - {formatted_date}"
    new_content = content.replace(HEADER_KEY, new_header, 1)

    if content == new_content:
        error("no unreleased version section found")
    if new_content != new_content.replace(HEADER_KEY, new_header):
        error("multiple unreleased version sections found")

    new_content = new_content.replace(new_header, f"{HEADER_KEY}\n\n{new_header}\n\nImage tag: ghcr.io/{repository_owner}/signal-inspector-backend:{next_version}")

    with open(changelog_path, "w") as f:
        f.write(new_content)


def bump_version(bump_comp_idx, repository_owner):
    current_version = get_current_version()
    next_version = get_next_version(bump_comp_idx, current_version)

    update_version_file(next_version, "VERSION")
    update_hocfile(next_version, "hocfile.yaml", current_version)
    update_manifest(next_version, "frontend/Cargo.toml", current_version)
    update_manifest(next_version, "backend/Cargo.toml", current_version)
    update_changelog(next_version, "CHANGELOG.md", repository_owner)

    return next_version


if __name__ == "__main__":
    if len(sys.argv) < 2:
        error("component argument missing")
    if len(sys.argv) < 3:
        error("repository owner argument missing")

    component = sys.argv[1]
    if component == "major":
        bump_comp_idx = 0
    elif component == "minor":
        bump_comp_idx = 1
    elif component == "patch":
        bump_comp_idx = 2
    else:
        error(f'"major", "minor" or "patch" expected')

    print(bump_version(bump_comp_idx, sys.argv[2]))
