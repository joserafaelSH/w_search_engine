use redb::{Database, Error};
use std::collections::HashMap;
use std::slice;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use crate::db::{TABLE_MAP_FILE_ID, TABLE_MAP_FILE_NAME};
use crate::model::Node;

use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::*;

use std::sync::mpsc;

#[repr(C)]
struct MFT_ENUM_DATA {
    start_file_reference_number: u64,
    low_usn: i64,
    high_usn: i64,
}

#[repr(C)]
struct UsnRecordHeader {
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

pub struct IndexedEntry {
    pub id: u64,
    pub parent_id: u64,
    pub name: String,
    pub drive: char,
    pub is_directory: bool,
}

fn open_volume(letter: String) -> HANDLE {
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

fn flush_batch(
    batch: &mut Vec<IndexedEntry>,
    table_id: &mut redb::Table<u64, Node>,
    table_name: &mut redb::Table<(String, u64), bool>,
) -> Result<(), Error> {
    for entry in batch.drain(..) {
        let normalized = entry.name.to_ascii_lowercase();

        let node = Node {
            name: entry.name.clone(),
            parent_id: entry.parent_id,
            drive_letter: entry.drive,
            is_directory: entry.is_directory,
        };

        table_id.insert(entry.id, node)?;
        table_name.insert((normalized, entry.id), entry.is_directory)?;
    }

    Ok(())
}

fn build_full_path(
    id: u64,
    cache: &mut HashMap<u64, String>,
    table: &redb::ReadOnlyTable<u64, Node>, // ✅ FIX HERE
) -> Result<String, Error> {
    if let Some(path) = cache.get(&id) {
        return Ok(path.clone());
    }

    let node_guard = table.get(&id)?;
    let node = match node_guard {
        Some(n) => n.value(),
        None => return Ok(String::new()),
    };

    let path = if node.parent_id == 0 || node.parent_id == id {
        format!("{}:\\{}", node.drive_letter, node.name)
    } else {
        let parent_path = build_full_path(node.parent_id, cache, table)?;

        if parent_path.ends_with('\\') {
            format!("{}{}", parent_path, node.name)
        } else {
            format!("{}\\{}", parent_path, node.name)
        }
    };

    cache.insert(id, path.clone());
    Ok(path)
}

pub fn parse_buffer_to_nodes(buffer: &[u8], drive: char) -> Vec<IndexedEntry> {
    let mut results = Vec::new();

    if buffer.len() < 8 {
        return results;
    }

    let mut offset = 8;

    while offset < buffer.len() {
        unsafe {
            if offset + std::mem::size_of::<UsnRecordHeader>() > buffer.len() {
                break;
            }

            let ptr = buffer.as_ptr().add(offset);
            let record = &*(ptr as *const UsnRecordHeader);

            let record_len = record.record_length as usize;

            // ✅ critical safety checks
            if record_len < std::mem::size_of::<UsnRecordHeader>() {
                break;
            }

            if offset + record_len > buffer.len() {
                break;
            }

            if record.file_name_length == 0 {
                offset += record_len;
                continue;
            }

            // skip deletes
            if record.reason & USN_REASON_FILE_DELETE != 0 {
                offset += record_len;
                continue;
            }

            let name_offset = record.file_name_offset as usize;
            let name_len_bytes = record.file_name_length as usize;

            if name_offset + name_len_bytes > record_len {
                offset += record_len;
                continue;
            }

            let name_ptr = ptr.add(name_offset) as *const u16;
            let name_len = name_len_bytes / 2;

            let name_slice = std::slice::from_raw_parts(name_ptr, name_len);
            let name = String::from_utf16_lossy(name_slice);

            if !name.is_empty() {
                let is_directory = (record.file_attributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;

                results.push(IndexedEntry {
                    id: record.file_reference_number,
                    parent_id: record.parent_file_reference_number,
                    name,
                    drive,
                    is_directory,
                });
            }

            offset += record_len;
        }
    }

    results
}

pub fn build_index(db: Arc<Database>) -> Result<(), Error> {
    let (tx, rx) = mpsc::channel();

    let drives = vec!['C']; // 👈 extend later if needed

    // 🔥 Spawn reader threads (one per drive)
    for drive in drives {
        let tx_clone = tx.clone();

        thread::spawn(move || {
            let handle = open_volume(drive.to_string());

            let mut enum_data = MFT_ENUM_DATA {
                start_file_reference_number: 0,
                low_usn: 0,
                high_usn: i64::MAX,
            };

            let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer

            loop {
                let mut bytes_returned = 0u32;

                let success = unsafe {
                    DeviceIoControl(
                        handle,
                        FSCTL_ENUM_USN_DATA,
                        Some(&mut enum_data as *mut _ as *mut _),
                        std::mem::size_of::<MFT_ENUM_DATA>() as u32,
                        Some(buffer.as_mut_ptr() as *mut _),
                        buffer.len() as u32,
                        Some(&mut bytes_returned),
                        None,
                    )
                };

                if !success.is_ok() || bytes_returned == 0 {
                    break;
                }

                let chunk = &buffer[..bytes_returned as usize];

                let entries = parse_buffer_to_nodes(chunk, drive);

                if tx_clone.send(entries).is_err() {
                    break;
                }

                // move forward in MFT
                unsafe {
                    enum_data.start_file_reference_number = *(chunk.as_ptr() as *const u64);
                }
            }
        });
    }

    drop(tx); // close sender when threads finish

    // 🔥 Writer thread (single writer for redb)
    let write_txn = db.begin_write()?;
    {
        let mut table_id = write_txn.open_table(TABLE_MAP_FILE_ID)?;
        let mut table_name = write_txn.open_table(TABLE_MAP_FILE_NAME)?;

        let mut batch = Vec::with_capacity(10_000);

        for entries in rx {
            for entry in entries {
                batch.push(entry);

                if batch.len() >= 10_000 {
                    flush_batch(&mut batch, &mut table_id, &mut table_name)?;
                }
            }
        }

        // flush remaining
        if !batch.is_empty() {
            flush_batch(&mut batch, &mut table_id, &mut table_name)?;
        }
    }

    write_txn.commit()?;

    println!("✅ Index built using USN Journal");

    Ok(())
}
