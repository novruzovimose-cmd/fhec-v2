use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use fhec_backend_tfhe::TfheBackend;
use fhec_backend_tfhe::profile::{V4ProfileLoad, load_v4_profile, save_v4_profile};

fn profile_path() -> PathBuf {
    PathBuf::from("target/fhec-profiles/tfhe-v4.json")
}

fn show(path: &Path) -> Result<(), Box<dyn Error>> {
    match load_v4_profile(path)? {
        V4ProfileLoad::Loaded(calibration) => {
            println!("FHEC Hardware Profile");
            println!("Status: VALID");
            println!("Path: {}", path.display());
            println!("Backend: tfhe-boolean-v4");
            println!("Worker threads: {}", calibration.baseline.rayon_threads);
            println!("Calibration width: {}", calibration.baseline.parallel_width);
            println!("Ciphertext size: {} bytes", calibration.ciphertext_bytes);
            println!(
                "AND latency: {:.3} ms",
                calibration.baseline.and_ns / 1_000_000.0
            );
            println!(
                "XOR latency: {:.3} ms",
                calibration.baseline.xor_ns / 1_000_000.0
            );
            println!(
                "MUX latency: {:.3} ms",
                calibration.baseline.mux_ns / 1_000_000.0
            );
            println!(
                "Calibration variability: {:.2}%",
                calibration.calibration_relative_mad * 100.0
            );
            Ok(())
        }
        V4ProfileLoad::Missing => Err("No saved profile. Run fhec profile calibrate.".into()),
        V4ProfileLoad::Invalid => {
            Err("Saved profile is invalid. Recalibrate to replace it.".into())
        }
        V4ProfileLoad::Incompatible => {
            Err("Saved profile is incompatible with this hardware.".into())
        }
    }
}

fn verify(path: &Path) -> Result<(), Box<dyn Error>> {
    match load_v4_profile(path)? {
        V4ProfileLoad::Loaded(calibration) => {
            let model = calibration.cost_model();

            if !model.is_calibrated() {
                return Err("The loaded cost model is not calibrated.".into());
            }

            println!("FHEC Profile Verification");
            println!("Status: PASS");
            println!("Path: {}", path.display());
            println!("Hardware compatibility: PASS");
            println!("Calibration model: VALID");
            Ok(())
        }
        V4ProfileLoad::Missing => Err("Profile verification failed: file is missing.".into()),
        V4ProfileLoad::Invalid => Err("Profile verification failed: invalid profile.".into()),
        V4ProfileLoad::Incompatible => {
            Err("Profile verification failed: incompatible hardware.".into())
        }
    }
}

fn calibrate(path: &Path) -> Result<(), Box<dyn Error>> {
    println!("FHEC TFHE Hardware Calibration");
    println!("Generating backend keys and measuring performance...");

    let backend = TfheBackend::new();

    let calibration = backend.calibrate_v4(2)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    save_v4_profile(path, &calibration)?;

    println!("Calibration saved: {}", path.display());
    println!("Worker threads: {}", calibration.baseline.rayon_threads);
    println!("Calibration width: {}", calibration.baseline.parallel_width);

    verify(path)?;

    println!("Hardware calibration completed successfully.");
    Ok(())
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let path = profile_path();

    match args {
        [command] if command == "show" => show(&path),
        [command] if command == "verify" => verify(&path),
        [command] if command == "calibrate" => calibrate(&path),
        _ => Err("Usage: fhec profile <show|verify|calibrate>".into()),
    }
}
