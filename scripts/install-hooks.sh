#!/bin/sh
# Point git at the repo's tracked hooks. Run once after cloning.
set -e
git config core.hooksPath .githooks
echo "hooks installed: $(git config core.hooksPath)"
