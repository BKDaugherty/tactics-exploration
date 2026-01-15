#!/bin/bash

set -euox pipefail

# Build the project with le cache
docker run -v .:/usr/src/project -v ~/.cargo/registry:/usr/local/cargo/registry -v ~/.cargo/git:/usr/local/cargo/git bevy_steamos
