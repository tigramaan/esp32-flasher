[CmdletBinding()]
param(
    [switch]$DryRun,
    [switch]$Verify
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$expectedChipId = [UInt16]0
$imageSize = 64

$targets = [ordered]@{
    'tests\fixtures\idf-build\nova.bin' = 'image'
    'tests\fixtures\direct\platformio\bootloader.bin' = 'image'
    'tests\fixtures\direct\platformio\partitions.bin' = 'partitions'
    'tests\fixtures\direct\platformio\firmware.bin' = 'image'
    'tests\fixtures\direct\update\firmware.bin' = 'image'
}

function New-ImageBytes {
    $bytes = [byte[]]::new($imageSize)
    $bytes[0] = 0xE9
    $chipBytes = [BitConverter]::GetBytes($expectedChipId)
    $bytes[12] = $chipBytes[0]
    $bytes[13] = $chipBytes[1]
    return $bytes
}

function Set-PartitionEntry {
    param(
        [byte[]]$Bytes,
        [int]$Index,
        [byte]$Type,
        [byte]$Subtype,
        [UInt32]$Offset,
        [UInt32]$Size,
        [string]$Label
    )
    $position = $Index * 32
    [BitConverter]::GetBytes([UInt16]0x50AA).CopyTo($Bytes, $position)
    $Bytes[$position + 2] = $Type
    $Bytes[$position + 3] = $Subtype
    [BitConverter]::GetBytes($Offset).CopyTo($Bytes, $position + 4)
    [BitConverter]::GetBytes($Size).CopyTo($Bytes, $position + 8)
    [Text.Encoding]::ASCII.GetBytes($Label).CopyTo($Bytes, $position + 12)
    [BitConverter]::GetBytes([UInt32]0).CopyTo($Bytes, $position + 28)
}

function New-PartitionBytes {
    $bytes = [byte[]]::new(0x1000)
    for ($index = 0; $index -lt $bytes.Length; $index++) {
        $bytes[$index] = 0xFF
    }
    Set-PartitionEntry $bytes 0 1 2 0x9000 0x4000 'nvs'
    Set-PartitionEntry $bytes 1 1 0 0xD000 0x2000 'otadata'
    Set-PartitionEntry $bytes 2 0 0x10 0x10000 0x100000 'ota_0'
    Set-PartitionEntry $bytes 3 0 0x11 0x110000 0x100000 'ota_1'

    $markerOffset = 4 * 32
    $marker = [byte[]](0xEB, 0xEB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF)
    $marker.CopyTo($bytes, $markerOffset)
    $hasher = [Security.Cryptography.MD5]::Create()
    try {
        $md5 = $hasher.ComputeHash($bytes[0..($markerOffset - 1)])
    }
    finally {
        $hasher.Dispose()
    }
    $md5.CopyTo($bytes, $markerOffset + 16)
    return $bytes
}

function Expected-Bytes([string]$Kind) {
    if ($Kind -eq 'image') {
        return New-ImageBytes
    }
    if ($Kind -eq 'partitions') {
        return New-PartitionBytes
    }
    throw "Unknown fixture kind: $Kind"
}

function Test-Fixture([string]$RelativePath, [string]$Kind) {
    $targetPath = Join-Path $repositoryRoot $RelativePath
    if (-not (Test-Path -LiteralPath $targetPath -PathType Leaf)) {
        throw "Fixture does not exist: $targetPath"
    }
    $actual = [IO.File]::ReadAllBytes($targetPath)
    $expected = Expected-Bytes $Kind
    if ([Convert]::ToBase64String($actual) -ne [Convert]::ToBase64String($expected)) {
        throw "Fixture content mismatch: $RelativePath"
    }
    Write-Output "OK: $RelativePath ($($actual.Length) bytes, $Kind)"
}

if ($DryRun) {
    foreach ($entry in $targets.GetEnumerator()) {
        $size = (Expected-Bytes $entry.Value).Length
        Write-Output "DRY-RUN: write $($entry.Key) ($size bytes, $($entry.Value))"
    }
    exit 0
}

if ($Verify) {
    foreach ($entry in $targets.GetEnumerator()) {
        Test-Fixture $entry.Key $entry.Value
    }
    exit 0
}

foreach ($entry in $targets.GetEnumerator()) {
    $targetPath = Join-Path $repositoryRoot $entry.Key
    [IO.Directory]::CreateDirectory((Split-Path -Parent $targetPath)) | Out-Null
    [IO.File]::WriteAllBytes($targetPath, (Expected-Bytes $entry.Value))
    Test-Fixture $entry.Key $entry.Value
}
