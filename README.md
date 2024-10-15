# Armory

Armory is a personal package manager. It has two main components: a registry
that hosts packages and a CLI that can be used to publish and install packages.

```shell
> armory install path
info: installed package to /home/msmoiz/.armory/registry/path-1.0.0
info: installed binary to /home/msmoiz/.armory/bin/path
```

## Installation

To install the CLI, run the following command:

```shell
curl https://armory.msmoiz.com/install.sh | sh # on Unix-like systems
Invoke-RestMethod https://armory.msmoiz.com/install.ps1 | Invoke-Expression # on Windows
```

This will install the appropriate binary for the target platform. Armory data
and configuration lives in the `${HOME}/.armory` directory. To complete
installation:

1. Add the Armory binary directory (_${HOME}/.armory/bin_) to your PATH.
2. Log in to the Armory registry using `armory login`. Registry credentials can
   be found on the server that hosts the registry.

## Supported platforms

Armory is supported on Windows, MacOS, and Linux. It supports both x86_64 and
aarch64 architectures for these operating systems.

## Registry

The registry is hosted at <https://armory.msmoiz.com>.

## Publishing packages

A package represents a single binary or executable. It does not include manual
pages, autocompletions, libraries, config files, or other peripheral artifacts
related to the application.

To publish a package, use the `armory publish` command. This command reads an
_armory.toml_ file in the current directory to determine the name and other
metadata needed to describe the package. It should contain the the following
fields.

### `[package]`

General package information.

#### `name`

The name of the package.

#### `version`

The version of the package.

### `[[targets]]`

Information about a specific target. There should be one `targets` section for
each platform that your tool supports.

#### `triple`

The target triple that this target corresponds to.

#### `path`

### Example

```toml
[package]
name = "armory"
version = "0.2.2"

[[targets]]
triple = "x86_64_linux"
path = "target/x86_64-unknown-linux-musl/release/armory"
```
