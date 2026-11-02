# Draws the logo at every icon size and packs them into icon.ico.
# Two bars, like the capsule on the taskbar, and the pace mark above 32 pixels.
Add-Type -AssemblyName System.Drawing

function New-Rounded([single]$x, [single]$y, [single]$w, [single]$h, [single]$r) {
    $path = New-Object Drawing.Drawing2D.GraphicsPath
    $d = 2 * $r
    $path.AddArc($x, $y, $d, $d, 180, 90)
    $path.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $path.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $path.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    $path
}

function Add-Bar($graphics, [single]$scale, [single]$top, [single]$fraction, $from, $to) {
    $x = 40 * $scale; $width = 176 * $scale; $height = 44 * $scale; $y = $top * $scale
    $radius = $height / 2
    $track = New-Object Drawing.SolidBrush ([Drawing.Color]::FromArgb(46, 255, 255, 255))
    $graphics.FillPath($track, (New-Rounded $x $y $width $height $radius))
    $filled = [Math]::Max($height, $width * $fraction)
    $area = New-Object Drawing.RectangleF $x, $y, ($filled + 1), $height
    $brush = New-Object Drawing.Drawing2D.LinearGradientBrush $area, $from, $to, 0
    $graphics.FillPath($brush, (New-Rounded $x $y $filled $height $radius))
}

function New-Logo([int]$size) {
    $bitmap = New-Object Drawing.Bitmap $size, $size, ([Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = 'AntiAlias'
    $graphics.PixelOffsetMode = 'HighQuality'
    $graphics.Clear([Drawing.Color]::Transparent)
    $scale = [single]($size / 256.0)

    $margin = 8 * $scale
    $side = $size - 2 * $margin
    $tile = New-Rounded $margin $margin $side $side (56 * $scale)
    $bounds = New-Object Drawing.RectangleF 0, 0, $size, $size
    $from = [Drawing.Color]::FromArgb(255, 26, 34, 72)
    $to = [Drawing.Color]::FromArgb(255, 52, 36, 108)
    $graphics.FillPath((New-Object Drawing.Drawing2D.LinearGradientBrush $bounds, $from, $to, 45), $tile)

    Add-Bar $graphics $scale 72 0.72 ([Drawing.Color]::FromArgb(255, 255, 112, 67)) ([Drawing.Color]::FromArgb(255, 255, 183, 110))
    Add-Bar $graphics $scale 140 0.44 ([Drawing.Color]::FromArgb(255, 139, 118, 245)) ([Drawing.Color]::FromArgb(255, 205, 192, 255))

    if ($size -ge 32) {
        $mark = New-Rounded (150 * $scale) (56 * $scale) (10 * $scale) (76 * $scale) (5 * $scale)
        $graphics.FillPath([Drawing.Brushes]::White, $mark)
    }
    $graphics.Dispose()
    $bitmap
}

$sizes = 16, 20, 24, 32, 40, 48, 64, 256
$images = foreach ($size in $sizes) {
    $bitmap = New-Logo $size
    $stream = New-Object IO.MemoryStream
    $bitmap.Save($stream, [Drawing.Imaging.ImageFormat]::Png)
    if ($size -eq 256) { $bitmap.Save((Join-Path $PSScriptRoot 'icon.png'), [Drawing.Imaging.ImageFormat]::Png) }
    $bitmap.Dispose()
    , $stream.ToArray()
}

$out = New-Object IO.MemoryStream
$writer = New-Object IO.BinaryWriter $out
$writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $edge = if ($sizes[$i] -ge 256) { 0 } else { $sizes[$i] }
    $writer.Write([byte]$edge); $writer.Write([byte]$edge)
    $writer.Write([byte]0); $writer.Write([byte]0)
    $writer.Write([uint16]1); $writer.Write([uint16]32)
    $writer.Write([uint32]$images[$i].Length); $writer.Write([uint32]$offset)
    $offset += $images[$i].Length
}
foreach ($image in $images) { $writer.Write([byte[]]$image) }
$writer.Flush()
[IO.File]::WriteAllBytes((Join-Path $PSScriptRoot 'icon.ico'), $out.ToArray())
