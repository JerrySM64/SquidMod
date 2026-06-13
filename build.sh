#!/bin/bash
if [ -z "$VERSION" ]; then
    echo "Error: VERSION environment variable is not set."
    exit 1
fi

APP_NAME="SquidMod"
echo "Packaging version: $VERSION"

mkdir pkg

if [ -f "target/release/SquidMod.exe" ]; then
    cp target/release/SquidMod.exe pkg/
else
    echo "Error: target/release/SquidMod.exe not found!"
    exit 1
fi

cp -r /mingw64/lib/gdk-pixbuf-2.0 pkg/lib/ 2>/dev/null || true
cp -r /mingw64/lib/gtk-4.0 pkg/lib/ 2>/dev/null || true

echo "Resolving DLL dependencies..."
for i in 1 2 3 4; do
    find pkg -type f \( -name "*.exe" -o -name "*.dll" \) | xargs ldd 2>/dev/null | grep -o "/mingw64/bin/[^ ]*\.dll" | sort -u | while read -r file; do
        if [ -f "$file" ]; then
            cp -u "$file" pkg/
        fi
    done
done

echo "Compiling GLib schemas..."
mkdir -p pkg/share/glib-2.0/schemas
cp -r /mingw64/share/glib-2.0/schemas/* pkg/share/glib-2.0/schemas/
glib-compile-schemas pkg/share/glib-2.0/schemas

mkdir -p pkg/share/icons
cp -r /mingw64/share/icons/hicolor pkg/share/icons/
cp -r /mingw64/share/icons/Adwaita pkg/share/icons/ || true

mkdir -p pkg/share/themes

ZIP_NAME="${APP_NAME}-v${VERSION}-x86_64.zip"
echo "Creating archive: $ZIP_NAME"

cd pkg
7z a "../$ZIP_NAME" *
cd ..

echo "$ZIP_NAME" > zip_name.txt
echo "Packaging complete: $ZIP_NAME"
