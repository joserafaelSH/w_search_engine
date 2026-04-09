

use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Ioctl::*;

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

#[derive(Debug)]
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

fn parse_buffer_to_nodes(buffer: &[u8], drive: char) -> Vec<IndexedEntry> {
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

            if name.is_empty()
                || name == "."
                || name == ".."
                || name.contains('~')
                || name.contains("\\.")
                || name.ends_with(".tmp")
            {
                offset += record_len;
                continue;
            }

            let is_directory =
                (record.file_attributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;

            results.push(IndexedEntry {
                id: record.file_reference_number,
                parent_id: record.parent_file_reference_number,
                name,
                drive,
                is_directory,
            });

            offset += record_len;
        }
    }

    results
}