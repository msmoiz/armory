#!/bin/sh

set -e

main() {
    arch=$(architecture)
    os=$(operating_system)

    info "detected platform | arch: ${arch} | os: ${os}"

    # Download the binary
    armory_bin="armory_bootstrap"

    info "downloading armory"
    curl "https://armory.msmoiz.com/download/armory-${arch}-${os}" \
        --output "${armory_bin}" \
        --fail \
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
}

info() {
    echo "install: $@"
}

architecture() {
    case $(uname -m) in
    arm64)
        echo "aarch64"
        ;;
    *)
        uname -m
        ;;
    esac
}

operating_system() {
    uname | tr '[:upper:]' '[:lower:]'
}

main
