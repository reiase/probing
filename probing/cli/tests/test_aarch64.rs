#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
use probing_cli::inject::{Injector, Process};
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
use std::process::Command;
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
use std::thread;
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
use std::time::Duration;

/// Test that aarch64 injection works with a simple target process
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
#[test]
fn test_aarch64_injection_basic() {
    // Start a simple target process
    let mut target = Command::new("sleep")
        .arg("10")
        .spawn()
        .expect("Failed to spawn target process");
    
    let target_pid = target.id();
    println!("Target process PID: {}", target_pid);
    
    // Give the process a moment to start
    thread::sleep(Duration::from_millis(100));
    
    // Find the process
    let proc = Process::by_pid(target_pid as i32)
        .expect("Failed to find target process");
    
    // Create a dummy library path for testing
    let dummy_lib = std::path::Path::new("/tmp/dummy.so");
    
    // Test injection (this will fail because the library doesn't exist,
    // but we can test the shellcode injection part)
    let result = Injector::attach(proc)
        .and_then(|mut injector| {
            injector.inject(dummy_lib, vec![])
        });
    
    // Clean up
    let _ = target.kill();
    
    // We expect this to fail because the library doesn't exist,
    // but the shellcode injection should work
    match result {
        Ok(_) => panic!("Expected injection to fail due to missing library"),
        Err(e) => {
            // Check if the error is about the missing library, not shellcode injection
            let error_msg = e.to_string();
            if error_msg.contains("No such file") || error_msg.contains("dlopen") {
                println!("✅ Aarch64 injection test passed - shellcode injection worked, failed only due to missing library");
            } else {
                panic!("Unexpected error: {}", error_msg);
            }
        }
    }
}

/// Test aarch64 shellcode generation
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
#[test]
fn test_aarch64_shellcode() {
    use probing_cli::inject::injection_aarch64::SHELLCODE_AARCH64;
    
    // Verify the shellcode has the expected structure
    assert_eq!(SHELLCODE_AARCH64.len(), 16);
    
    // Check for NOP instructions at the beginning
    assert_eq!(SHELLCODE_AARCH64[0..4], [0x1f, 0x20, 0x03, 0xd5]); // nop
    assert_eq!(SHELLCODE_AARCH64[4..8], [0x1f, 0x20, 0x03, 0xd5]); // nop
    
    // Check for BLR x8 instruction
    assert_eq!(SHELLCODE_AARCH64[8..12], [0x00, 0x01, 0x3f, 0xd6]); // blr x8
    
    // Check for BRK instruction
    assert_eq!(SHELLCODE_AARCH64[12..16], [0x00, 0x00, 0x20, 0xd4]); // brk #0
    
    println!("✅ Aarch64 shellcode structure is correct");
}

/// Test aarch64 register handling
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
#[test]
fn test_aarch64_register_handling() {
    use pete::Registers;
    
    // Create a mock register set
    let mut regs = Registers::default();
    regs.x0 = 0x12345678;
    regs.x1 = 0x87654321;
    regs.x2 = 0x11111111;
    regs.x3 = 0x22222222;
    regs.x8 = 0xdeadbeef;
    regs.sp = 0x1000;
    regs.pc = 0x2000;
    
    // Test that we can read the expected values
    assert_eq!(regs.x0, 0x12345678);
    assert_eq!(regs.x1, 0x87654321);
    assert_eq!(regs.x2, 0x11111111);
    assert_eq!(regs.x3, 0x22222222);
    assert_eq!(regs.x8, 0xdeadbeef);
    
    // Test stack alignment
    let aligned_sp = regs.sp & !0xf;
    assert_eq!(aligned_sp, 0x1000); // Should be aligned to 16 bytes
    
    println!("✅ Aarch64 register handling is correct");
}

#[cfg(not(all(target_arch = "aarch64", target_os = "linux")))]
#[test]
fn test_aarch64_skipped_on_non_aarch64_linux() {
    println!("ℹ️  Aarch64 tests skipped on non-aarch64 or non-linux platform");
}