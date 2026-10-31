# Installs Statac on Windows from a release's package, in PowerShell:
#
#     irm https://get.statac.dev/install.ps1 | iex
#     $env:STATAC_TO = 'C:\Tools'; $env:STATAC_UPDATES = 'on'; irm https://get.statac.dev/install.ps1 | iex
#
# The options are environment variables, STATAC_TO, STATAC_UPDATES and
# STATAC_VERSION, standing for install.sh's --to, --updates and
# --version: `irm | iex` runs the script as text, which no parameter can
# reach. In a console it asks where to put Statac, a folder under
# %LOCALAPPDATA% unless told otherwise, and whether Statac keeps itself up
# to date. A variable answers its question, so the two run it unattended.
# With no console to ask in, or its input taken from elsewhere, it takes
# the answer in brackets and says so.
#
# It downloads the Windows package and checks it against the release's
# SHA256SUMS, and SHA256SUMS against its signature where the release has
# one. Ed25519 is written out below, since the .NET that Windows
# PowerShell runs on has none. It adds the folder to the user's PATH,
# never the machine's, where neither holds it already, and the licences
# that travel with the program go to %LOCALAPPDATA%\Statac. Beside the
# program, .statac-install records the answer, where the licences are and
# the entry it added to PATH, for `statac upgrade` and `statac uninstall`,
# which removes what this puts in place. It asks for no administrator.
#
# Everything is inside one block, so a download cut short runs nothing,
# and nothing it names is left behind in the session.
& {
    $ProgressPreference = 'SilentlyContinue'

    # Each release is in v<version>/ there, and latest/ names the newest.
    $releases = 'https://get.statac.dev'
    if ($env:STATAC_RELEASES) { $releases = $env:STATAC_RELEASES }

    # The public half of the key a release's SHA256SUMS is signed with, from
    # 0.8.0 on: Statac's own, as src/upgrade.rs holds it.
    $key = '9ba90e0b6ec11a149baad9a54d6cf33d15a872e75c08d915f7361225d32febde'

    $target = 'x86_64-pc-windows-msvc'
    $licences = 'LICENSE-MIT', 'LICENSE-APACHE', 'THIRD-PARTY.md', 'THIRD-PARTY-GRAMMARS.md'

    # Thrown once the reason has been said.
    $stopped = 'Statac install stopped'

    function Stop-Install([string] $line, [string] $then) {
        $Host.UI.WriteErrorLine($line)
        if ($then) { $Host.UI.WriteErrorLine("  $then") }
        throw $stopped
    }

    function Save-Download([string] $address, [string] $file) {
        try {
            Invoke-WebRequest -UseBasicParsing -Uri $address -OutFile $file -ErrorAction Stop
            $true
        } catch {
            $false
        }
    }

    function ConvertFrom-Hex([string] $hex) {
        $bytes = New-Object byte[] ($hex.Length / 2)
        for ($at = 0; $at -lt $bytes.Length; $at++) {
            $bytes[$at] = [Convert]::ToByte($hex.Substring($at * 2, 2), 16)
        }
        , $bytes
    }

    # Ed25519 as RFC 8032 has it, checking only. A point is an array of its
    # extended coordinates, X, Y, Z and T, each kept below $q or above -$q.
    # PowerShell's variable names ignore case, so these are all lower case.
    if (-not ('System.Numerics.BigInteger' -as [type])) { Add-Type -AssemblyName System.Numerics }
    $big = 'System.Numerics.BigInteger' -as [type]
    $q = $big::Pow(2, 255) - 19
    $order = $big::Pow(2, 252) + $big::Parse('27742317777372353535851937790883648493')
    $curve = ($q - 121665) * $big::ModPow(121666, $q - 2, $q) % $q
    $curve2 = $curve * 2 % $q
    $rootPower = $big::Divide($q - 5, 8)
    $rootOfMinusOne = $big::ModPow(2, $big::Divide($q - 1, 4), $q)
    $one = $big::One
    $zero = $big::Zero

    # The point 32 bytes encode, or $null where they encode none.
    function ConvertTo-Point([byte[]] $bytes) {
        $odd = ($bytes[31] -shr 7) -band 1
        $little = New-Object byte[] 33
        [Array]::Copy($bytes, $little, 32)
        $little[31] = $little[31] -band 0x7f
        $y = $big::new($little)
        if ($y -ge $q) { return $null }
        $yy = $y * $y % $q
        $u = ($yy - 1 + $q) % $q
        $v = ($curve * $yy + 1) % $q
        $v3 = $v * $v % $q * $v % $q
        $v7 = $v3 * $v3 % $q * $v % $q
        $x = $u * $v3 % $q * $big::ModPow($u * $v7 % $q, $rootPower, $q) % $q
        $check = $v * $x % $q * $x % $q
        if ($check -ne $u) {
            if ($check -ne ($q - $u) % $q) { return $null }
            $x = $x * $rootOfMinusOne % $q
        }
        if ($x.IsZero -and $odd -eq 1) { return $null }
        if ([int](-not $x.IsEven) -ne $odd) { $x = $q - $x }
        , @($x, $y, $one, ($x * $y % $q))
    }

    # The sum of two points, which serves for doubling too.
    function Add-Point($first, $second) {
        $a = ($first[1] - $first[0]) * ($second[1] - $second[0]) % $q
        $b = ($first[1] + $first[0]) * ($second[1] + $second[0]) % $q
        $c = $first[3] * $curve2 % $q * $second[3] % $q
        $d = $first[2] * $second[2] % $q * 2 % $q
        $e = $b - $a
        $f = $d - $c
        $g = $d + $c
        $h = $b + $a
        , @(($e * $f % $q), ($g * $h % $q), ($f * $g % $q), ($e * $h % $q))
    }

    function Get-Multiple($point, $scalar) {
        $bits = $scalar.ToByteArray()
        $sum = @($zero, $one, $one, $zero)
        for ($at = $bits.Length * 8 - 1; $at -ge 0; $at--) {
            $sum = Add-Point $sum $sum
            if (($bits[$at -shr 3] -shr ($at -band 7)) -band 1) { $sum = Add-Point $sum $point }
        }
        , $sum
    }

    $base = ConvertTo-Point (ConvertFrom-Hex '5866666666666666666666666666666666666666666666666666666666666666')

    # Whether $signature is the Ed25519 signature of $message by the key
    # $public: S below the group's order, and [S]B - [k]A encoding as R,
    # which is how the library Statac's upgrade uses checks it.
    function Test-Signature([byte[]] $message, [byte[]] $signature, [byte[]] $public) {
        if ($signature.Length -ne 64 -or $public.Length -ne 32) { return $false }
        $point = ConvertTo-Point $public
        if ($null -eq $point) { return $false }
        $little = New-Object byte[] 33
        [Array]::Copy($signature, 32, $little, 0, 32)
        $s = $big::new($little)
        if ($s -ge $order) { return $false }
        $hashed = New-Object System.IO.MemoryStream
        $hashed.Write($signature, 0, 32)
        $hashed.Write($public, 0, 32)
        $hashed.Write($message, 0, $message.Length)
        $digest = [System.Security.Cryptography.SHA512]::Create().ComputeHash($hashed.ToArray())
        $little = New-Object byte[] 65
        [Array]::Copy($digest, $little, 64)
        $k = $big::new($little) % $order
        $negated = @((($q - $point[0]) % $q), $point[1], $point[2], (($q - $point[3]) % $q))
        $r = Add-Point (Get-Multiple $base $s) (Get-Multiple $negated $k)
        $inverse = $big::ModPow(($r[2] % $q + $q) % $q, $q - 2, $q)
        $x = ($r[0] * $inverse % $q + $q) % $q
        $y = ($r[1] * $inverse % $q + $q) % $q
        $encoded = New-Object byte[] 32
        $written = $y.ToByteArray()
        [Array]::Copy($written, $encoded, [Math]::Min(32, $written.Length))
        if (-not $x.IsEven) { $encoded[31] = $encoded[31] -bor 0x80 }
        for ($at = 0; $at -lt 32; $at++) {
            if ($encoded[$at] -ne $signature[$at]) { return $false }
        }
        $true
    }

    # The options PowerShell was started with, without its own path.
    function Get-Started {
        @([Environment]::GetCommandLineArgs() | Select-Object -Skip 1)
    }

    # Whether this PowerShell ends once the command it was started with has
    # run, as `powershell -Command` does, so the script can end it with a
    # code without closing a window someone is typing in. Each option may
    # be shortened.
    function Test-Ending {
        $ending = $false
        foreach ($option in Get-Started) {
            if ($option -match '^[-/]noe') { return $false }
            if ($option -match '^[-/](c|com|comm|comma|comman|command|cwa|commandwithargs|f|fi|fil|file|e|ec|en|enc|enco|encod|encode|encoded[a-z]*)$') {
                $ending = $true
            }
        }
        $ending
    }

    # Whether someone can answer a question.
    function Test-Console {
        if (-not [Environment]::UserInteractive) { return $false }
        foreach ($option in Get-Started) {
            if ($option -match '^[-/]noni') { return $false }
        }
        try { -not [Console]::IsInputRedirected } catch { $false }
    }

    # The whole path of the folder written $written, as typed or pasted:
    # with quotes round it taken off, a ~ at its start read as the home
    # folder, %NAME% as the variable, and from PowerShell's current folder
    # where it isn't whole. Nothing where it can't be a path.
    function Expand-Folder([string] $written) {
        $written = [Environment]::ExpandEnvironmentVariables($written.Trim().Trim('"'))
        if ($written -eq '~') {
            $written = $HOME
        } elseif ($written -match '^~[\\/]') {
            $written = Join-Path $HOME $written.Substring(2)
        }
        try {
            $whole = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($written)
            $whole = [IO.Path]::GetFullPath($whole)
        } catch {
            return ''
        }
        if ($whole.Length -gt 3) { $whole = $whole.TrimEnd('\') }
        $whole
    }

    # Why Statac can't be put in the folder $folder, or nothing where it
    # can: the nearest folder on the way that is there must be one that
    # can be written in.
    function Get-Unusable([string] $folder) {
        $at = $folder
        while (-not (Test-Path -LiteralPath $at)) {
            $up = Split-Path -Parent $at
            if (-not $up) { return ('There''s no `{0}` on this machine.' -f $at) }
            $at = $up
        }
        if (-not (Test-Path -LiteralPath $at -PathType Container)) {
            return ('`{0}` is a file, not a folder.' -f $at)
        }
        # A name `statac upgrade` removes, were it ever left.
        $probe = Join-Path $at ".statac-new-$PID"
        try {
            [IO.File]::WriteAllBytes($probe, (New-Object byte[] 0))
            [IO.File]::Delete($probe)
        } catch {
            return ('You can''t write in `{0}`.' -f $at)
        }
        ''
    }

    # The value of the first line naming $name in the marker $file, or
    # nothing, read as Statac reads it.
    function Get-Setting([string] $file, [string] $name) {
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { return '' }
        foreach ($line in [IO.File]::ReadAllLines($file)) {
            $line = $line.Trim()
            $colon = $line.IndexOf(':')
            if ($line.StartsWith('#') -or $colon -lt 0) { continue }
            if ($line.Substring(0, $colon).Trim() -ceq $name) { return $line.Substring($colon + 1).Trim() }
        }
        ''
    }

    # Whether the entry $entry of a PATH names the folder $folder.
    function Test-SameFolder([string] $entry, [string] $folder) {
        $entry = [Environment]::ExpandEnvironmentVariables($entry).Trim().Trim('"')
        if (-not $entry -or -not $folder) { return $false }
        try { $entry = [IO.Path]::GetFullPath($entry) } catch { return $false }
        $entry.TrimEnd('\') -eq $folder.TrimEnd('\')
    }

    # Prints the prompt and reads an answer, the one in brackets where it
    # is left empty.
    function Read-Answer([string] $offered) {
        Write-Host ('  [{0}] > ' -f $offered) -NoNewline
        $answer = Read-Host
        if ($null -eq $answer) { Stop-Install 'No answer, so nothing was installed.' }
        if ($answer) { $answer } else { $offered }
    }

    function Install-Statac {
        $version = $env:STATAC_VERSION
        $to = $env:STATAC_TO
        $updates = $env:STATAC_UPDATES
        if ($version) { $version = $version -replace '^v', '' }
        if ($updates -and $updates -cne 'on' -and $updates -cne 'off') {
            Stop-Install ('Wrong kind of value: `STATAC_UPDATES` is `on` or `off`, not `{0}`' -f $updates)
        }
        if ($to) {
            $written = $to
            $to = Expand-Folder $written
            if (-not $to) { Stop-Install ('Can''t install Statac in {0}: it isn''t a path to a folder.' -f $written) }
        }
        if (-not $env:LOCALAPPDATA) {
            Stop-Install 'Can''t install Statac: `LOCALAPPDATA` isn''t set, so there''s nowhere for its licences.'
        }

        # Windows on Arm runs the x64 package from Windows 11 on.
        $processor = $env:PROCESSOR_ARCHITECTURE
        if ($env:PROCESSOR_ARCHITEW6432) { $processor = $env:PROCESSOR_ARCHITEW6432 }
        $runs = $processor -eq 'AMD64' -or ($processor -eq 'ARM64' -and [Environment]::OSVersion.Version.Build -ge 22000)
        if (-not $runs) {
            Stop-Install ('No package for this machine: Windows on {0}' -f $processor) 'Other ways to install: https://statac.dev/install/'
        }

        try {
            [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
        } catch {
        }
        $work = Join-Path ([IO.Path]::GetTempPath()) ('statac-install-' + [IO.Path]::GetRandomFileName())
        [IO.Directory]::CreateDirectory($work) | Out-Null
        try {
            Install-Into $work $version $to $updates
        } finally {
            Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    function Install-Into([string] $work, [string] $version, [string] $to, [string] $updates) {
        if ($version) {
            $sums = "$releases/v$version/SHA256SUMS"
            if (-not (Save-Download $sums "$work\SHA256SUMS")) {
                Stop-Install "Can't download $sums" "Is $version a release? They are listed in https://git.statac.dev/statac/statac/src/branch/main/CHANGELOG.md"
            }
        } else {
            $sums = "$releases/latest/SHA256SUMS"
            if (-not (Save-Download $sums "$work\SHA256SUMS")) { Stop-Install "Can't download $sums" }
        }

        # A line is the hash, a space, a space or an asterisk, and the name.
        $ending = '-' + [regex]::Escape($target) + '\.zip$'
        $line = @([IO.File]::ReadAllLines("$work\SHA256SUMS") | Where-Object { $_ -match $ending }) | Select-Object -First 1
        $package = ''
        if ($line) { $package = ($line -split '[ *]')[-1] }
        if ($package -notmatch '^statac-[^\\/:]+\.zip$') {
            Stop-Install ('No package for this machine in the release: nothing in {0} ends `{1}.zip`.' -f $sums, $target) 'Other ways to install: https://statac.dev/install/'
        }
        $expected = $line.Substring(0, $line.IndexOf(' '))
        $name = $package -replace '\.zip$', ''
        $version = ($name -replace '^statac-', '') -replace ('-' + [regex]::Escape($target) + '$'), ''
        # Keeping up to date, and the signature, came with 0.8.0.
        $keeps = $version -notmatch '^0\.[0-7]\.'
        if (-not $keeps -and $updates -eq 'on') {
            Stop-Install ('Statac {0} can''t keep itself up to date: that came in 0.8.0.' -f $version) 'Leave `STATAC_UPDATES` unset, or install a later release.'
        }

        # Fetched from v<version>/, which never changes, so a cache holding an
        # older latest/ can't pair it with a newer signature. A release is
        # signed once it is published, so a release tried before then has none.
        # SHA256SUMS.sig is the signature's 64 bytes, which Test-Signature
        # reads; SHA256SUMS.sshsig, the same in OpenSSH's form, is for
        # install.sh's ssh-keygen.
        $signed = $false
        if ($keeps -and (Save-Download "$releases/v$version/SHA256SUMS.sig" "$work\SHA256SUMS.sig")) {
            $signed = $true
            $message = [IO.File]::ReadAllBytes("$work\SHA256SUMS")
            $signature = [IO.File]::ReadAllBytes("$work\SHA256SUMS.sig")
            if (-not (Test-Signature $message $signature (ConvertFrom-Hex $key))) {
                Stop-Install "The release's SHA256SUMS isn't signed with Statac's key, so nothing was installed."
            }
        }

        Write-Host "Installing Statac $version ($target)."

        $asking = (-not $to -or ($keeps -and -not $updates)) -and (Test-Console)
        if ($asking) {
            Write-Host 'Enter takes the answer in brackets.'
            Write-Host ''
        }
        $usual = Join-Path $env:LOCALAPPDATA 'Programs\Statac'
        $taken = ''
        if (-not $to -and $asking) {
            Write-Host 'Where should Statac go?'
            Write-Host '  Statac adds the folder to your PATH.'
            while (-not $to) {
                $answer = Read-Answer $usual
                $to = Expand-Folder $answer
                if ($to) { $why = Get-Unusable $to } else { $why = ('`{0}` isn''t a path to a folder.' -f $answer) }
                if ($why) {
                    Write-Host "  $why"
                    $to = ''
                }
            }
        } elseif (-not $to) {
            $to = $usual
            $taken = "goes in $usual"
        }
        $why = Get-Unusable $to
        if ($why) { Stop-Install ('Can''t install Statac in {0}: {1}' -f $to, $why) }

        # Where this folder has an install already, its answer is the one in
        # brackets.
        $marker = Join-Path $to '.statac-install'
        $before = Get-Setting $marker 'updates'
        if ($before -cne 'on' -and $before -cne 'off') { $before = '' }
        $given = $updates
        if ($keeps -and -not $updates -and $asking) {
            Write-Host 'Should Statac keep itself up to date?'
            Write-Host '  If yes, `statac serve` checks for a fix each time it starts, installs it'
            Write-Host '  and tells you. Turn it off later with `statac upgrade --updates=off`.'
            $offered = 'no'
            if ($before -eq 'on') { $offered = 'yes' }
            while (-not $updates) {
                $answer = Read-Answer $offered
                if ($answer -ceq 'yes') {
                    $updates = 'on'
                } elseif ($answer -ceq 'no') {
                    $updates = 'off'
                } else {
                    Write-Host ('  `{0}` isn''t an answer here. Write `yes` or `no`.' -f $answer)
                }
            }
        } elseif ($keeps -and -not $updates) {
            $updates = 'off'
            if ($before) { $updates = $before }
            if ($updates -eq 'on') {
                $said = 'keeps itself up to date, as it did before'
            } elseif ($before -eq 'off') {
                $said = 'doesn''t keep itself up to date, as before'
            } else {
                $said = 'doesn''t keep itself up to date'
            }
            if ($taken) { $taken = "$taken, and $said" } else { $taken = $said }
        }
        if (-not $updates) { $updates = 'off' }
        if ($taken) {
            # The answer given, or the other one to the answer taken.
            $other = $given
            if (-not $given) {
                $other = 'on'
                if ($updates -eq 'on') { $other = 'off' }
            }
            $choose = '$env:STATAC_TO = ''<folder>''; '
            if ($keeps) { $choose += ('$env:STATAC_UPDATES = ''{0}''; ' -f $other) }
            Write-Host "No terminal to ask in, so Statac $taken."
            Write-Host ('  To choose: {0}irm https://get.statac.dev/install.ps1 | iex' -f $choose)
        }
        if ($asking) { Write-Host '' }

        Write-Host "Downloading $package"
        $address = "$releases/v$version/$package"
        $zip = Join-Path $work $package
        if (-not (Save-Download $address $zip)) { Stop-Install "Can't download $address" }

        $found = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($found -ne $expected.ToLowerInvariant()) {
            Stop-Install "The package doesn't match the release's SHA256SUMS, so nothing was installed." "Expected $expected, and the download is $found."
        }
        if ($signed) {
            Write-Host "Checked the package against the release's SHA256SUMS, and SHA256SUMS against"
            Write-Host '  its signature.'
        } elseif ($keeps) {
            Write-Host "Checked the package against the release's SHA256SUMS alone: its signature"
            Write-Host "  couldn't be downloaded."
        } else {
            Write-Host "Checked the package against the release's SHA256SUMS. Releases before 0.8.0 aren't signed."
        }

        if (-not ('System.IO.Compression.ZipFile' -as [type])) { Add-Type -AssemblyName System.IO.Compression.FileSystem }
        $zipFile = 'System.IO.Compression.ZipFile' -as [type]
        $unpacked = Join-Path $work 'unpacked'
        try {
            $zipFile::ExtractToDirectory($zip, $unpacked)
        } catch {
            Stop-Install "Can't unpack $package, so nothing was installed."
        }
        $program = Join-Path $unpacked "$name\statac.exe"
        if (-not (Test-Path -LiteralPath $program -PathType Leaf)) {
            Stop-Install ('The package holds no `{0}\statac.exe`, so nothing was installed.' -f $name)
        }

        try {
            [IO.Directory]::CreateDirectory($to) | Out-Null
        } catch {
            Stop-Install "Can't make the folder $to"
        }
        # Written beside its place and moved in. Windows won't replace a
        # program that is running but lets it be moved, so one in the way is
        # moved aside first and removed where nothing runs it. The names are
        # the ones `statac upgrade` and `statac uninstall` know.
        $placed = Join-Path $to 'statac.exe'
        $new = Join-Path $to ".statac-new-$PID"
        $old = Join-Path $to ".statac-old-$PID"
        try {
            [IO.File]::Copy($program, $new, $true)
            if ([IO.File]::Exists($placed)) {
                if ([IO.File]::Exists($old)) { [IO.File]::Delete($old) }
                [IO.File]::Move($placed, $old)
                try {
                    [IO.File]::Move($new, $placed)
                } catch {
                    [IO.File]::Move($old, $placed)
                    throw
                }
                try { [IO.File]::Delete($old) } catch { }
            } else {
                [IO.File]::Move($new, $placed)
            }
        } catch {
            try { [IO.File]::Delete($new) } catch { }
            Stop-Install "Can't write to $to"
        }

        $notices = Join-Path $env:LOCALAPPDATA 'Statac'
        try {
            [IO.Directory]::CreateDirectory($notices) | Out-Null
            foreach ($licence in $licences) {
                [IO.File]::Copy((Join-Path $unpacked "$name\$licence"), (Join-Path $notices $licence), $true)
            }
        } catch {
            Stop-Install "Can't write to $notices"
        }

        # The user's PATH as the registry holds it, %NAME% and all, written
        # back as the same kind of value. Where it already names the folder,
        # the entry is the script's only where an earlier run added it.
        $entry = ''
        $joined = $false
        $unjoined = ''
        try {
            $environment = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment')
            $path = $environment.GetValue('Path', '', 'DoNotExpandEnvironmentNames')
            $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
            if ($null -ne $environment.GetValue('Path')) { $kind = $environment.GetValueKind('Path') }
            $system = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('SYSTEM\CurrentControlSet\Control\Session Manager\Environment')
            $systemPath = ''
            if ($system) { $systemPath = $system.GetValue('Path', '', 'DoNotExpandEnvironmentNames') }
            $added = Get-Setting $marker 'path'
            if (@("$path;$systemPath" -split ';' | Where-Object { Test-SameFolder $_ $to }).Count -eq 0) {
                if (-not $path) {
                    $grown = $to
                } elseif ($path.EndsWith(';')) {
                    $grown = $path + $to
                } else {
                    $grown = "$path;$to"
                }
                $environment.SetValue('Path', $grown, $kind)
                # Deleting a variable the user doesn't have changes nothing,
                # and has .NET tell every window that the environment has
                # changed, which writing the registry alone doesn't.
                $nobodys = 'STATAC_INSTALL_PATH_CHANGED'
                if ($null -eq [Environment]::GetEnvironmentVariable($nobodys, 'User')) {
                    [Environment]::SetEnvironmentVariable($nobodys, $null, 'User')
                }
                $entry = $to
                $joined = $true
            } elseif ($added -and (Test-SameFolder $added $to) -and @($path -split ';' | Where-Object { $_ -eq $added }).Count -gt 0) {
                $entry = $added
            }
        } catch {
            $unjoined = $_.Exception.Message
        }
        if (@($env:Path -split ';' | Where-Object { Test-SameFolder $_ $to }).Count -eq 0) {
            $env:Path = $env:Path.TrimEnd(';') + ";$to"
        }

        $lines = @(
            "# Written by Statac's install script. statac upgrade and statac uninstall read it.",
            "updates: $updates",
            "licences: $notices"
        )
        if ($entry) { $lines += "path: $entry" }
        $fresh = Join-Path $to ".statac-install-$PID"
        try {
            [IO.File]::WriteAllText($fresh, ($lines -join "`n") + "`n", (New-Object System.Text.UTF8Encoding $false))
            if ([IO.File]::Exists($marker)) { [IO.File]::Delete($marker) }
            [IO.File]::Move($fresh, $marker)
        } catch {
            Stop-Install "Can't write to $to"
        }

        try {
            $said = (& $placed --version | Out-String).Trim()
            $ran = $LASTEXITCODE -eq 0
        } catch {
            $said = $_.Exception.Message
            $ran = $false
        }
        if (-not $ran) { Stop-Install "Installed $placed, and it doesn't run on this machine:" $said }
        Write-Host "Installed $said at $placed"
        Write-Host "Its licences are in $notices"
        if ($keeps) {
            if ($updates -eq 'on') {
                Write-Host 'Statac keeps itself up to date: `statac serve` installs each fix as it starts.'
                Write-Host 'Turn this off with `statac upgrade --updates=off`.'
            } else {
                Write-Host 'Statac doesn''t keep itself up to date.'
                Write-Host 'Turn this on with `statac upgrade --updates=on`.'
            }
        }

        if ($joined) {
            Write-Host ''
            Write-Host ('Added {0} to your PATH, so a terminal opened from now on finds `statac`.' -f $to)
        } elseif ($unjoined) {
            Write-Host ''
            Write-Host ('Can''t add {0} to your PATH: {1}' -f $to, $unjoined)
            Write-Host '  Add it in Settings, under "Edit environment variables for your account".'
        }
        $first = Get-Command statac.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($first -and -not (Test-SameFolder (Split-Path -Parent $first.Path) $to)) {
            Write-Host ''
            Write-Host ('Another `statac` comes first on your PATH: {0}' -f $first.Path)
            Write-Host '  Until it is removed, `statac` runs that one.'
        }
    }

    $failed = $false
    try {
        Install-Statac
    } catch {
        if ("$_" -ne $stopped) { $Host.UI.WriteErrorLine(('Can''t install Statac: {0}' -f $_.Exception.Message)) }
        $failed = $true
    }
    if ($failed -and (Test-Ending)) { exit 1 }
}
