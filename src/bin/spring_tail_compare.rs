use std::path::PathBuf;

use clap::Parser;
use thyllore_anim_core::spring_bone::{compare_tail_trajectories, parse_tail_csv};

#[derive(Parser)]
struct Args {
    reference: PathBuf,
    candidate: PathBuf,
    #[arg(long)]
    max_rms: Option<f32>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let ref_text = std::fs::read_to_string(&args.reference)?;
    let cand_text = std::fs::read_to_string(&args.candidate)?;
    let reference = parse_tail_csv(&ref_text)?;
    let candidate = parse_tail_csv(&cand_text)?;
    let errors = compare_tail_trajectories(&reference, &candidate);
    if errors.is_empty() {
        eprintln!("no matching chain entries found between reference and candidate");
        std::process::exit(1);
    }
    println!("chain rms max max_frame samples");
    for e in &errors {
        println!(
            "{} {:.6} {:.6} {} {}",
            e.chain, e.rms, e.max, e.max_frame, e.sample_count
        );
    }
    if let Some(threshold) = args.max_rms {
        for e in &errors {
            if e.rms > threshold {
                eprintln!("{} rms {:.6} exceeds {:.6}", e.chain, e.rms, threshold);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}
