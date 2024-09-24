$ErrorActionPreference = "Stop"

function Main {
    # Detect platform
    $Arch = Architecture
    $Os = "windows"
    Info "detected platform | arch: ${Arch} | os: ${Os}"

    # Create the armory home and binary dirs
    $ArmoryHome = "${HOME}\.armory"
    $ArmoryHomeBin = "${ArmoryHome}\bin"
    if (!(Test-Path -PathType Container $ArmoryHomeBin)) {
        New-Item -ItemType Directory -Force -Path $ArmoryHomeBin | Out-Null
    }

    # Download the binary
    $InstallPath = "${ArmoryHomeBin}\armory.exe"
    Info "downloading armory"
    curl "https://armory.msmoiz.com/download/armory-${Arch}-${Os}" `
        --output "${InstallPath}" `
        --fail `
        --silent
    Info "downloaded armory"
    Info "installed armory to ${InstallPath}"
    Info "add ${ArmoryHomeBin} to path to complete installation"

}

function Info {
    param ($String)

    Write-Output $String
}

function Architecture {
    $Arch = (Get-CimInstance Win32_Processor).Architecture
    if ($Arch -eq 9) {
        return "x86_64"
    }
    elseif ($Arch -eq 12) {
        return "aarch64"
    }
    else {
        return "Unknown"
    }
}

Main