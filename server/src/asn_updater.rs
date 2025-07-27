use crate::AppState;
use chrono::{Datelike, Weekday};
use clokwerk::TimeUnits;
use clokwerk::{AsyncScheduler, Job};
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

const DB_EDITION: &str = "GeoLite2-ASN";

/// Spawns a background task that periodically checks for and updates the MaxMind ASN database.
pub fn spawn_asn_updater_task(app_state: Arc<AppState>) {
    tokio::spawn(async move {
        info!("Spawning ASN database updater task.");
        let mut scheduler = AsyncScheduler::new();

        // Schedule the main update job to run once a day.
        scheduler.every(1.day()).at("4:00 am").run(move || {
            let state = app_state.clone();
            async move {
                let now = chrono::Utc::now();
                if now.weekday() != Weekday::Sat && now.weekday() != Weekday::Sun {
                    info!("It's a weekday! Running ASN Database Update Job.");
                    if let Err(e) = update_database(state).await {
                        error!(error = %e, "Failed to update ASN database");
                    }
                } else {
                    info!("It's the weekend, skipping ASN database update job.");
                }
            }
        });

        // The scheduler loop. It will run pending jobs and then sleep.
        loop {
            scheduler.run_pending().await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

/// Performs the full download, verification, and hot-swap of the ASN database.
async fn update_database(app_state: Arc<AppState>) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Download the checksum file.
    info!("Downloading database checksum...");
    let checksum_data = download_file(&app_state, "tar.gz.sha256").await?;
    let expected_checksum = std::str::from_utf8(&checksum_data)?
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("");

    if expected_checksum.is_empty() {
        return Err("Downloaded checksum was empty".into());
    }
    info!(expected_checksum, "Successfully downloaded checksum.");

    // 2. Download the database archive.
    info!("Downloading database archive...");
    let db_data = download_file(&app_state, "tar.gz").await?;
    info!("Database archive downloaded successfully.");

    // 3. Verify the checksum of the downloaded archive.
    info!("Verifying checksum of downloaded archive...");
    let calculated_checksum = sha256::digest(&*db_data);
    if calculated_checksum != expected_checksum {
        return Err(format!(
            "Checksum mismatch! Expected: {}, Calculated: {}",
            expected_checksum, calculated_checksum
        )
        .into());
    }
    info!("Checksum verified successfully.");

    // 4. Unpack the .mmdb file from the tarball in memory.
    info!("Unpacking .mmdb file from archive...");
    let tar_gz = std::io::Cursor::new(db_data);
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);

    let mut mmdb_data: Option<Vec<u8>> = None;

    for entry in archive.entries()? {
        let mut entry = entry?;
        if entry.path()?.extension().map_or(false, |ext| ext == "mmdb") {
            info!(path = ?entry.path()?.display(), "Found .mmdb file in archive.");
            let mut data = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut data)?;
            mmdb_data = Some(data);
            break; // Found it, no need to look further.
        }
    }

    let mmdb_data = mmdb_data.ok_or("Could not find .mmdb file in archive")?;
    info!("Successfully unpacked .mmdb data from archive.");

    // 5. Create a new reader from the in-memory data.
    let new_reader = maxminddb::Reader::from_source(mmdb_data)?;
    info!("New database loaded into a temporary in-memory reader.");

    // 6. The HOT-SWAP! Acquire a write lock and replace the reader.
    info!("Acquiring write lock to hot-swap the database reader...");
    let mut writer_guard = app_state.db_reader.write().await;
    *writer_guard = new_reader; // Replace the old reader with the new one.

    info!("Database hot-swap complete. New connections will use the updated database.");

    Ok(())
}

/// Generic async function to download a file from the MaxMind download service.
async fn download_file(
    app_state: &Arc<AppState>,
    suffix: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let url = format!(
        "https://download.maxmind.com/app/geoip_download?edition_id={}&license_key={}&suffix={}",
        DB_EDITION, &app_state.maxmind_license_key, suffix
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    } else {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read body".to_string());
        Err(format!(
            "Failed to download file with suffix '{}'. Status: {}. Body: {}",
            suffix, status, body
        )
        .into())
    }
}
