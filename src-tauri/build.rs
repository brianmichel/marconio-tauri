fn main() {
    #[cfg(target_os = "macos")]
    {
        cc::Build::new()
            .file("src/shazam_bridge.m")
            .flag("-fobjc-arc")
            .compile("marconio_shazam_bridge");

        println!("cargo:rerun-if-changed=src/shazam_bridge.h");
        println!("cargo:rerun-if-changed=src/shazam_bridge.m");
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=ShazamKit");
    }

    tauri_build::build()
}
