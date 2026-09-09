#!/system/bin/sh
# Magisk Module Installer Script

ui_print "****************************************"
ui_print "*      Komodo Android Periphery        *"
ui_print "*   Native Bionic / Magisk Agent       *"
ui_print "****************************************"

ui_print "- Installing binary to $MODPATH..."
set_perm "$MODPATH/komodo-android-periphery" 0 0 0755
set_perm "$MODPATH/service.sh" 0 0 0755

KOMODO_DIR="/data/adb/komodo"
mkdir -p "$KOMODO_DIR"
mkdir -p "$KOMODO_DIR/keys"
set_perm "$KOMODO_DIR" 0 0 0700
set_perm "$KOMODO_DIR/keys" 0 0 0700

if [ ! -f "$KOMODO_DIR/config.toml" ]; then
    ui_print "- Initializing default configuration at $KOMODO_DIR/config.toml..."
    cp "$MODPATH/config.toml.example" "$KOMODO_DIR/config.toml"
    set_perm "$KOMODO_DIR/config.toml" 0 0 0600
else
    ui_print "- Existing configuration found at $KOMODO_DIR/config.toml (preserved)."
fi

ui_print "- Installation complete. Configure $KOMODO_DIR/config.toml before starting daemon."
