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
