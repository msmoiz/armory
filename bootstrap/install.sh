#!/bin/sh

set -e

info() {
    echo "install: $@"
}

# Download the binary
armory_bin="armory_bootstrap"

info "downloading armory"
curl https://armory.msmoiz.com/download/armory \
    --output "${armory_bin}" \
    --silent
info "downloaded armory"

# Make it executable
chmod +x "${armory_bin}"

# Create the armory home and binary dirs
armory_home="${HOME}/.armory"
armory_home_bin="${armory_home}/bin"
if [ ! -d "${armory_home_bin}" ]; then
    mkdir -p "${armory_home_bin}"
fi

# Install the binary to the correct location
install_path="${armory_home_bin}/armory"
mv "${armory_bin}" "${install_path}"

info "installed armory to ${install_path}"
info "add ${armory_home_bin} to path to complete installation"