use rusqlite::{Connection, Result, params};
use std::sync::mpsc;
use std::thread;

use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::*;

#[repr(C)]
struct MFT_ENUM_DATA {
    start_file_reference_number: u64,
    low_usn: i64,
    high_usn: i64,
}


#[repr(C)]
#[derive(Debug)]
struct USN_RECORD_V2 {
    record_length: u32,
    major_version: u16,
    minor_version: u16,
    file_reference_number: u64,
    parent_file_reference_number: u64,
    usn: i64,
    timestamp: i64,
    reason: u32,
    source_info: u32,
    security_id: u32,
    file_attributes: u32,
    file_name_length: u16,
    file_name_offset: u16,
}

#[derive(Debug)]
pub struct IndexedEntry {
    pub id: u64,
    pub parent_id: u64,
    pub name: String,
    pub drive: char,
    pub is_directory: bool,
}

fn find_drivers() -> Vec<char> {
    let mut drives = Vec::new();
    let bitmask = unsafe { GetLogicalDrives() };

    for i in 0..26 {
        if (bitmask & (1 << i)) != 0 {
            drives.push((b'A' + i) as char);
        }
    }

    drives
}

fn open_volume(letter: char) -> HANDLE {
    let path = format!("\\\\.\\{}:", letter);
    unsafe {
        CreateFileW(
            &windows_core::HSTRING::from(path),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .expect("Failed to open volume")
    }
}

fn flush_batch(conn: &mut Connection, batch: &mut Vec<IndexedEntry>) -> Result<()> {
    let tx = conn.transaction()?;

    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO files (id, parent_id, name, drive_letter, is_directory)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;

        for e in batch.drain(..) {
            if e.name.is_empty() || e.name.contains('~') {
                continue;
            }

            stmt.execute(params![
                e.id as i64,
                e.parent_id as i64,
                e.name,
                e.drive.to_string(),
                e.is_directory as i32
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn build_index(conn: &mut Connection) -> Result<()> {
    let (tx, rx) = mpsc::channel();

    let drives = find_drivers();

    for drive in drives {
        let tx_clone = tx.clone();

        thread::spawn(move || {
            let handle = open_volume(drive);

            let mut enum_data = MFT_ENUM_DATA {
                start_file_reference_number: 0,
                low_usn: 0,
                high_usn: i64::MAX,
            };

            let mut buffer = vec![0u8; 1024 * 1024];

            loop {
                let mut bytes = 0u32;
                
                let ok = unsafe {
                    DeviceIoControl(
                        handle,
                        FSCTL_ENUM_USN_DATA,
                        Some(&mut enum_data as *mut _ as *mut _),
                        std::mem::size_of::<MFT_ENUM_DATA>() as u32,
                        Some(buffer.as_mut_ptr() as *mut _),
                        buffer.len() as u32,
                        Some(&mut bytes),
                        None,
                    )
                };

                if !ok.is_ok() || bytes == 0 {
                    break;
                }

                let chunk = &buffer[..bytes as usize];

                let entries = parse_buffer(chunk, drive);
                println!(
                    "Parsed {} entries from drive {} entries {:?}",
                    entries.len(),
                    drive,
                    entries
                );
                let _ = tx_clone.send(entries);

                unsafe {
                    enum_data.start_file_reference_number = *(chunk.as_ptr() as *const u64);
                }
            }
        });
    }

    drop(tx);

    let mut batch = Vec::with_capacity(10_000);

    for entries in rx {
        for e in entries {
            batch.push(e);

            if batch.len() >= 10_000 {
                flush_batch(conn, &mut batch)?;
            }
        }
    }

    if !batch.is_empty() {
        flush_batch(conn, &mut batch)?;
    }

    println!("✅ Index built");
    Ok(())
}

fn parse_buffer(buf: &[u8], drive: char) -> Vec<IndexedEntry> {
    let mut out = Vec::new();
    let mut offset = 8;

    while offset < buf.len() {
        unsafe {
            let ptr = buf.as_ptr().add(offset);

            let record: USN_RECORD_V2 = std::ptr::read_unaligned(ptr as *const _);

            let len = record.record_length as usize;

            // safety checks
            if len == 0 || offset + len > buf.len() {
                break;
            }

            if record.file_name_length == 0 {
                offset += len;
                continue;
            }

            // 🚀 FIX: base pointer must be the record start
            let base = ptr as *const u8;

            let name_ptr = base.add(record.file_name_offset as usize) as *const u16;

            let name_len = (record.file_name_length / 2) as usize;

            let name_slice = std::slice::from_raw_parts(name_ptr, name_len);

            let name = String::from_utf16_lossy(name_slice);

            let is_dir = (record.file_attributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;

            // filter garbage
            if !is_valid_name(&name) {
                offset += len;
                continue;
            }

            out.push(IndexedEntry {
                id: record.file_reference_number,
                parent_id: record.parent_file_reference_number,
                name,
                drive,
                is_directory: is_dir,
            });

            offset += len;
        }
    }

    out
}

fn is_valid_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    if name.contains("dubug") {
        return false;
    }

    // ignore current/parent
    if name == "." || name == ".." {
        return false;
    }

    // 🔥 system NTFS files
    if name.starts_with('$') {
        return false;
    }

    // 🔥 temp / noisy
    if name.starts_with('~') || name.ends_with(".tmp") {
        return false;
    }

    // 🔥 skip weird control characters
    if name.chars().any(|c| c.is_control()) {
        return false;
    }

    // 🔥 skip garbage decoding (common in broken USN reads)
    if name.contains('\u{FFFD}') {
        return false;
    }

    true
}
