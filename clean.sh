#~/bin/bash

# Open
find -D exec ./open-division -type d -name '.git' -exec rm -rf {} +
find -D exec ./open-division -type d -name '.github' -exec rm -rf {} +
find -D exec ./open-division -type d -name '.circleci' -exec rm -rf {} +
find -D exec ./open-division -type d -name '.vscode' -exec rm -rf {} +

find -D exec ./open-division -name '.gitignore' -exec rm -rf {} +
find -D exec ./open-division -name '.gitmodules' -exec rm -rf {} +

# Team
find -D exec ./team-division -type d -name '.git' -exec rm -rf {} +
find -D exec ./team-division -type d -name '.github' -exec rm -rf {} +
find -D exec ./team-division -type d -name '.circleci' -exec rm -rf {} +
find -D exec ./team-division -type d -name '.vscode' -exec rm -rf {} +

find -D exec ./team-division -name '.gitignore' -exec rm -rf {} +
find -D exec ./team-division -name '.gitmodules' -exec rm -rf {} +

# Visual check for external deps in Cargo
echo 'External dependencies found in Cargo.toml files:'
grep -rnw './' -e 'git = '
