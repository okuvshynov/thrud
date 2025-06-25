use std::process::Command;

fn main() {
    // Only compile Swift bridge on macOS
    if cfg!(target_os = "macos") {
        println!("cargo:rerun-if-changed=src/collectors/gpu/apple_silicon_bridge.swift");
        println!("cargo:rerun-if-changed=src/collectors/cpu/apple_silicon_bridge.swift");
        
        // Compile GPU Swift bridge to object file
        let gpu_output = Command::new("swiftc")
            .args(&[
                "-c",
                "-emit-object",
                "-o", "target/gpu_bridge.o",
                "src/collectors/gpu/apple_silicon_bridge.swift",
            ])
            .output()
            .expect("Failed to compile GPU Swift bridge");

        if !gpu_output.status.success() {
            panic!("GPU Swift compilation failed: {}", String::from_utf8_lossy(&gpu_output.stderr));
        }

        // Compile CPU Swift bridge to object file
        let cpu_output = Command::new("swiftc")
            .args(&[
                "-c",
                "-emit-object",
                "-o", "target/cpu_bridge.o",
                "src/collectors/cpu/apple_silicon_bridge.swift",
            ])
            .output()
            .expect("Failed to compile CPU Swift bridge");

        if !cpu_output.status.success() {
            panic!("CPU Swift compilation failed: {}", String::from_utf8_lossy(&cpu_output.stderr));
        }

        // Create combined static library
        let ar_output = Command::new("ar")
            .args(&[
                "rcs",
                "target/libbridge.a",
                "target/gpu_bridge.o",
                "target/cpu_bridge.o",
            ])
            .output()
            .expect("Failed to create static library");

        if !ar_output.status.success() {
            panic!("Archive creation failed: {}", String::from_utf8_lossy(&ar_output.stderr));
        }

        println!("cargo:rustc-link-search=native=target");
        println!("cargo:rustc-link-lib=static=bridge");
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
}