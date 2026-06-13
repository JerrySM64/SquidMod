VERSION    := $(shell awk -F '"' '/^version =/ {print $$2; exit}' Cargo.toml)
APP_NAME   := SquidMod
BUILD_DIR  := build
REL_DIR    := $(BUILD_DIR)/release
DBG_DIR    := $(BUILD_DIR)/debug

.PHONY: all linux linux-release linux-debug \
        windows windows-release windows-debug \
        macos macos-release macos-debug clean

all: linux-release windows-release macos-release

linux:   linux-release
windows: windows-release
macos:   macos-release

$(REL_DIR):
	mkdir -p $(REL_DIR)

$(DBG_DIR):
	mkdir -p $(DBG_DIR)

DEBIAN_IMG := docker.io/library/debian:13

BASE_DEPS := build-essential ca-certificates curl file git pkg-config \
             libssl-dev libgtk-4-dev libadwaita-1-dev

INSTALL_RUST = curl https://sh.rustup.rs -sSf | sh -s -- -y && \
               . \$$HOME/.cargo/env

linux-debug: $(DBG_DIR)
	podman run --rm -v $(PWD):/project:Z -w /project $(DEBIAN_IMG) bash -c "\
		apt-get update && \
		apt-get install -y --no-install-recommends $(BASE_DEPS) && \
		$(INSTALL_RUST) && \
		cargo build && \
		cp target/debug/$(APP_NAME) $(DBG_DIR)/$(APP_NAME)"

linux-release: $(REL_DIR)
	podman run --rm -v $(PWD):/project:Z -w /project $(DEBIAN_IMG) bash -c "\
		apt-get update && \
		apt-get install -y --no-install-recommends $(BASE_DEPS) libfuse2 upx-ucl && \
		$(INSTALL_RUST) && \
		cargo build --release && \
		upx --best --lzma target/release/$(APP_NAME) && \
		APPDIR=AppDir && \
		mkdir -p \"\$$APPDIR/usr/bin\" \"\$$APPDIR/usr/share/applications\" \
			\"\$$APPDIR/usr/share/icons/hicolor/256x256/apps\" && \
		cp target/release/$(APP_NAME) \"\$$APPDIR/usr/bin/squidmod\" && \
		printf '[Desktop Entry]\nType=Application\nName=$(APP_NAME)\nExec=squidmod\nIcon=squidmod\nCategories=Utility;\nTerminal=false\n' > \"\$$APPDIR/usr/share/applications/SquidMod.desktop\" && \
		cp \"\$$APPDIR/usr/share/applications/SquidMod.desktop\" \"\$$APPDIR/SquidMod.desktop\" && \
		printf '#!/bin/sh\nexec \"\$$(dirname \"\$$0\")/usr/bin/squidmod\" \"\$$@\"\n' > \"\$$APPDIR/AppRun\" && \
		chmod +x \"\$$APPDIR/AppRun\" && \
		cp assets/hicolor/256x256/apps/dev.jerrysm64.squidmod.png \"\$$APPDIR/usr/share/icons/hicolor/256x256/apps/squidmod.png\" && \
		cp \"\$$APPDIR/usr/share/icons/hicolor/256x256/apps/squidmod.png\" \"\$$APPDIR/squidmod.png\" && \
		ln -s squidmod.png \"\$$APPDIR/.DirIcon\" && \
		curl -L -o appimagetool.AppImage https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage && \
		chmod +x appimagetool.AppImage && \
		./appimagetool.AppImage --appimage-extract && \
		ARCH=x86_64 ./squashfs-root/AppRun \"\$$APPDIR\" $(APP_NAME)-v$(VERSION)-x86_64.AppImage && \
		mv $(APP_NAME)-v$(VERSION)-x86_64.AppImage $(REL_DIR)/ && \
		rm -rf AppDir squashfs-root appimagetool.AppImage"

windows-debug: $(DBG_DIR)
	cargo build
	mkdir -p pkg
	cp target/debug/$(APP_NAME).exe pkg/
	ldd pkg/$(APP_NAME).exe | grep '\/mingw64\/bin\/.*\.dll' -o | xargs -I{} cp "{}" pkg/
	mkdir -p pkg/share/glib-2.0/schemas pkg/share/icons pkg/share/themes
	cp -r /mingw64/share/glib-2.0/schemas/* pkg/share/glib-2.0/schemas/
	glib-compile-schemas pkg/share/glib-2.0/schemas
	cp -r /mingw64/share/icons/hicolor pkg/share/icons/
	cp -r /mingw64/share/icons/Adwaita pkg/share/icons/ || true
	cd pkg && 7z a ../$(DBG_DIR)/$(APP_NAME)-Windows.zip *
	rm -rf pkg

windows-release: $(REL_DIR)
	cargo build --release
	upx --best --lzma target/release/$(APP_NAME).exe
	mkdir -p pkg
	cp target/release/$(APP_NAME).exe pkg/
	ldd pkg/$(APP_NAME).exe | grep '\/mingw64\/bin\/.*\.dll' -o | xargs -I{} cp "{}" pkg/
	mkdir -p pkg/share/glib-2.0/schemas pkg/share/icons pkg/share/themes
	cp -r /mingw64/share/glib-2.0/schemas/* pkg/share/glib-2.0/schemas/
	glib-compile-schemas pkg/share/glib-2.0/schemas
	cp -r /mingw64/share/icons/hicolor pkg/share/icons/
	cp -r /mingw64/share/icons/Adwaita pkg/share/icons/ || true
	cd pkg && 7z a ../$(REL_DIR)/$(APP_NAME)-Windows.zip *
	rm -rf pkg

ICON_SRC  := assets/hicolor/256x256/apps/dev.jerrysm64.squidmod.png
ICNS_NAME := squidmod.icns

define BUILD_ICNS
	rm -rf /tmp/squidmod.iconset
	mkdir /tmp/squidmod.iconset
	sips -z 16   16   $(ICON_SRC) --out /tmp/squidmod.iconset/icon_16x16.png
	sips -z 32   32   $(ICON_SRC) --out /tmp/squidmod.iconset/icon_16x16@2x.png
	sips -z 32   32   $(ICON_SRC) --out /tmp/squidmod.iconset/icon_32x32.png
	sips -z 64   64   $(ICON_SRC) --out /tmp/squidmod.iconset/icon_32x32@2x.png
	sips -z 128  128  $(ICON_SRC) --out /tmp/squidmod.iconset/icon_128x128.png
	sips -z 256  256  $(ICON_SRC) --out /tmp/squidmod.iconset/icon_128x128@2x.png
	sips -z 256  256  $(ICON_SRC) --out /tmp/squidmod.iconset/icon_256x256.png
	sips -z 512  512  $(ICON_SRC) --out /tmp/squidmod.iconset/icon_256x256@2x.png
	sips -z 512  512  $(ICON_SRC) --out /tmp/squidmod.iconset/icon_512x512.png
	sips -z 1024 1024 $(ICON_SRC) --out /tmp/squidmod.iconset/icon_512x512@2x.png
	iconutil -c icns /tmp/squidmod.iconset -o $(1)
	rm -rf /tmp/squidmod.iconset
endef

define INFO_PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>$(APP_NAME)</string>
  <key>CFBundleDisplayName</key><string>$(APP_NAME)</string>
  <key>CFBundleExecutable</key><string>$(APP_NAME)</string>
  <key>CFBundleIdentifier</key><string>dev.jerrysm64.squidmod</string>
  <key>CFBundleVersion</key><string>$(VERSION)</string>
  <key>CFBundleShortVersionString</key><string>$(VERSION)</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>squidmod</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSRequiresAquaSystemAppearance</key><false/>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
</dict></plist>
endef
export INFO_PLIST

DEV_ID := $(shell security find-identity -v -p codesigning 2>/dev/null | \
             grep -o 'Developer ID Application: [^"]*' | head -1)
ifeq ($(DEV_ID),)
  SIGN_ID := -
else
  SIGN_ID := $(DEV_ID)
endif

ifeq ($(SIGN_ID),-)
  CODESIGN_OPTS := --force --deep --entitlements entitlements.plist -s "-"
else
  CODESIGN_OPTS := --force --deep --options runtime \
                   --entitlements entitlements.plist -s "$(SIGN_ID)"
endif

macos-debug: $(DBG_DIR)
	cargo build
	$(eval BUNDLE := $(DBG_DIR)/$(APP_NAME).app/Contents)
	mkdir -p "$(BUNDLE)/MacOS" "$(BUNDLE)/Resources"
	cp target/debug/$(APP_NAME) "$(BUNDLE)/MacOS/$(APP_NAME)"
	$(call BUILD_ICNS,$(BUNDLE)/Resources/$(ICNS_NAME))
	printf '%s' "$$INFO_PLIST" > "$(BUNDLE)/Info.plist"

macos-release: $(REL_DIR)
	cargo build --release
	$(eval BUNDLE := $(REL_DIR)/$(APP_NAME).app/Contents)
	mkdir -p "$(BUNDLE)/MacOS" "$(BUNDLE)/Resources"

	cp target/release/$(APP_NAME) "$(BUNDLE)/MacOS/$(APP_NAME).bin"

	mkdir -p "$(BUNDLE)/Frameworks"
	@if ! command -v dylibbundler >/dev/null 2>&1; then \
	  echo "ERROR: dylibbundler not found. Please run: brew install dylibbundler"; \
	  exit 1; \
	fi
	dylibbundler -od -b \
	  -x "$(BUNDLE)/MacOS/$(APP_NAME).bin" \
	  -d "$(BUNDLE)/Frameworks/" \
	  -p @executable_path/../Frameworks/

	printf '%s\n' \
	  '#!/bin/sh' \
	  'BUNDLE="$$(cd "$$(dirname "$$0")/.." && pwd)"' \
	  'export XDG_DATA_DIRS="$$BUNDLE/Resources/share:/opt/homebrew/share:/usr/local/share:/usr/share"' \
	  'export GSETTINGS_SCHEMA_DIR="$$BUNDLE/Resources/share/glib-2.0/schemas"' \
	  'export GTK_DATA_PREFIX="$$BUNDLE/Resources"' \
	  'exec "$$BUNDLE/MacOS/$(APP_NAME).bin" "$$@"' \
	> "$(BUNDLE)/MacOS/$(APP_NAME)"
	chmod +x "$(BUNDLE)/MacOS/$(APP_NAME)"

	$(call BUILD_ICNS,$(BUNDLE)/Resources/$(ICNS_NAME))

	printf '%s' "$$INFO_PLIST" > "$(BUNDLE)/Info.plist"

	mkdir -p "$(BUNDLE)/Resources/share/icons"

	cp -r assets/hicolor "$(BUNDLE)/Resources/share/icons/"

	for prefix in /opt/homebrew /usr/local; do \
	  if [ -d "$$prefix/share/icons/hicolor" ]; then \
	    cp -rn "$$prefix/share/icons/hicolor/." \
	           "$(BUNDLE)/Resources/share/icons/hicolor/"; \
	    break; \
	  fi; \
	done

	for prefix in /opt/homebrew /usr/local; do \
	  if [ -d "$$prefix/share/icons/Adwaita" ]; then \
	    cp -rn "$$prefix/share/icons/Adwaita" \
	           "$(BUNDLE)/Resources/share/icons/"; \
	    break; \
	  fi; \
	done
	@if [ -d "$(BUNDLE)/Resources/share/icons/Adwaita" ]; then \
	  echo "Caching bundled Adwaita icon theme..."; \
	  gtk4-update-icon-cache -f -t "$(BUNDLE)/Resources/share/icons/Adwaita"; \
	else \
	  echo "WARNING: Adwaita icon theme not found – run: brew install adwaita-icon-theme"; \
	fi

	gtk4-update-icon-cache -f -t "$(BUNDLE)/Resources/share/icons/hicolor"

	mkdir -p "$(BUNDLE)/Resources/share/glib-2.0/schemas"
	for prefix in /opt/homebrew /usr/local; do \
	  if [ -d "$$prefix/share/glib-2.0/schemas" ]; then \
	    cp "$$prefix/share/glib-2.0/schemas/"*.gschema.xml \
	       "$(BUNDLE)/Resources/share/glib-2.0/schemas/" 2>/dev/null || true; \
	    break; \
	  fi; \
	done
	glib-compile-schemas "$(BUNDLE)/Resources/share/glib-2.0/schemas"

	@echo "Code-signing with identity: $(SIGN_ID)"
	codesign $(CODESIGN_OPTS) "$(REL_DIR)/$(APP_NAME).app"


clean:
	rm -rf target $(BUILD_DIR)
