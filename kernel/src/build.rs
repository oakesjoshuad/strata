use std::process::Command;

pub fn git_build_id() -> String {
    let commit = match Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if value.is_empty() {
                "unknown".to_string()
            } else {
                value
            }
        }
        _ => "unknown".to_string(),
    };
    let dirty = match Command::new("git").args(["status", "--porcelain"]).output() {
        Ok(output) => output.status.success() && !output.stdout.is_empty(),
        Err(_) => false,
    };
    if dirty {
        format!("{commit}-dirty")
    } else {
        commit
    }
}
