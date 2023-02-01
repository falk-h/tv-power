use std::{env, fs, path::Path};

use dbus_codegen::{ConnectionType, GenOpts};

const XML_FILE: &str = "gnome-session/gnome-session/org.gnome.SessionManager.Presence.xml";

fn main() {
    let xml = fs::read_to_string(XML_FILE)
        .unwrap_or_else(|e| panic!("Failed to read DBUS interface metadata at {XML_FILE:?}: {e}"));
    let options = GenOpts {
        // Make the names of generated types a bit shorter.
        skipprefix: Some("org.gnome".into()),

        // This gives us only client implementations.
        methodtype: None,

        connectiontype: ConnectionType::Blocking,

        ..Default::default()
    };
    let code = dbus_codegen::generate(&xml, &options)
        .unwrap_or_else(|e| panic!("Failed to generate code for DBUS interface: {e}"));

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_file = Path::new(&out_dir).join("generated.rs");
    fs::write(&out_file, code)
        .unwrap_or_else(|e| panic!("Failed to write generated code to file {out_file:?}: {e}"));

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={XML_FILE}");
}
