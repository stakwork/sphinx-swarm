use anyhow::Result;
use fs::File;
use rocket::tokio;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tokio::fs;
use walkdir::WalkDir;

// ls -la

#[tokio::main]
async fn main() -> Result<()> {
    let proj = std::env::args().nth(1).expect("no proj given");
    for entry in WalkDir::new(format!("vol/{}", proj)) {
        if let Ok(entry) = entry {
            println!("=> {}", entry.path().display());
            set_perms(entry.path(), 0o766).await?;
        }
    }
    Ok(())
}

pub async fn test_rw_perms() -> Result<()> {
    let path = Path::new("somefile");

    set_perms(path, 0o746).await?;
    let meta = get_perms(path).await;
    println!("{:?}", meta);

    set_perms(path, 0o700).await?;
    let meta = get_perms(path).await;
    println!("{:?}", meta);

    set_perms(path, 0o766).await?;
    let meta = get_perms(path).await;
    println!("{:?}", meta);

    Ok(())
}

async fn get_file(path: &Path) -> Result<File> {
    Ok(if path.exists() {
        File::open(path).await
    } else {
        File::create(path).await
    }?)
}
async fn set_perms(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, PermissionsExt::from_mode(mode)).await?;
    Ok(())
}
async fn get_perms(path: &Path) -> Result<std::fs::Permissions> {
    Ok(get_file(path).await?.metadata().await?.permissions())
}
