#!/bin/sh

set -e

info() {
    echo "install: $@"
}

armory_bin="armory_bootstrap"

info "downloading armory"
curl https://armory.msmoiz.com/download/armory \
    --output "${armory_bin}" \
    --silent
info "downloaded armory"

chmod +x "${armory_bin}"

info "installing latest version"
./"${armory_bin}" install armory 0.1.0 2>&1 | sed 's/^/armory: /'

info "cleaning up"
rm ${armory_bin}

info "installed armory"

info "add ${HOME}/.armory/bin to path to complete installation"