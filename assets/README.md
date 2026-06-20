# StreamDeck Core icon (placeholder)

The icon is a 256×256 PNG. Replace `icon.png` with a real branded icon before
shipping a release. The same path is referenced by `packaging.toml`:

- Linux:    `assets/icon.png`        (referenced by `linux.icon_file`)
- macOS:    `assets/icon.icns`       (referenced by `macos.icon`)
- Windows:  `assets/icon.ico`        (used by the NSIS installer; not yet wired)

You can generate an `.icns` from a `.png` with:

```sh
mkdir icon.iconset
sips -z 16 16     icon.png --out icon.iconset/icon_16x16.png
sips -z 32 32     icon.png --out icon.iconset/icon_16x16@2x.png
sips -z 32 32     icon.png --out icon.iconset/icon_32x32.png
sips -z 64 64     icon.png --out icon.iconset/icon_32x32@2x.png
sips -z 128 128   icon.png --out icon.iconset/icon_128x128.png
sips -z 256 256   icon.png --out icon.iconset/icon_128x128@2x.png
sips -z 256 256   icon.png --out icon.iconset/icon_256x256.png
sips -z 512 512   icon.png --out icon.iconset/icon_256x256@2x.png
sips -z 512 512   icon.png --out icon.iconset/icon_512x512.png
sips -z 1024 1024 icon.png --out icon.iconset/icon_512x512@2x.png
iconutil -c icns icon.iconset -o icon.icns
```

And an `.ico` (Windows) with ImageMagick:

```sh
magick convert icon.png -define icon:auto-resize=256,128,96,64,48,32,16 icon.ico
```
