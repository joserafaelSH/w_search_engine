use redb::{Database, Error};
use std::slice;

use crate::db::{TABLE_MAP_FILE_ID, TABLE_MAP_FILE_NAME};
use crate::model::Node;

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

pub fn build_index(db: &Database) -> Result<(), Error> {
    let write_txn = db.begin_write()?;
    {
        let mut table_id = write_txn.open_table(TABLE_MAP_FILE_ID)?;
        let mut table_name = write_txn.open_table(TABLE_MAP_FILE_NAME)?;

        unsafe {
            let drives_mask = GetLogicalDrives();

            for i in 0..26 {
                if (drives_mask >> i) & 1 == 1 {
                    let drive_letter = (b'A' + i) as char;

                    let handle = open_volume(drive_letter.to_string());

                    let mut enum_data = MFT_ENUM_DATA {
                        start_file_reference_number: 0,
                        low_usn: 0,
                        high_usn: i64::MAX,
                    };

                    let mut buffer = vec![0u8; 1024 * 1024];
                    let mut bytes_returned = 0u32;

                    loop {
                        let result = DeviceIoControl(
                            handle,
                            FSCTL_ENUM_USN_DATA,
                            Some(&mut enum_data as *mut _ as *mut _),
                            std::mem::size_of::<MFT_ENUM_DATA>() as u32,
                            Some(buffer.as_mut_ptr() as *mut _),
                            buffer.len() as u32,
                            Some(&mut bytes_returned),
                            None,
                        );

                        if result.is_err() || bytes_returned == 0 {
                            break;
                        }

                        parse_buffer(
                            &buffer,
                            bytes_returned,
                            drive_letter,
                            &mut table_id,
                            &mut table_name,
                        )?;

                        let next_usn = *(buffer.as_ptr() as *const u64);
                        if next_usn == 0 {
                            break;
                        }

                        enum_data.start_file_reference_number = next_usn;
                    }

                    let _ = CloseHandle(handle);
                }
            }
        }
    }
    write_txn.commit()?;

    Ok(())
}

fn parse_buffer(
    buffer: &[u8],
    bytes_returned: u32,
    drive: char,
    table_map_file_id: &mut redb::Table<u64, Node>,
    table_map_file_name: &mut redb::Table<(String, u64), ()>,
) -> Result<(), Error> {
    let mut offset = 8;

    while offset < bytes_returned as usize {
        unsafe {
            if offset + std::mem::size_of::<UsnRecordHeader>() > bytes_returned as usize {
                break;
            }

            let ptr = buffer.as_ptr().add(offset);
            let header = &*(ptr as *const UsnRecordHeader);
            let record_len = header.record_length as usize;

            if record_len == 0 || offset + record_len > bytes_returned as usize {
                break;
            }

            let name_ptr = ptr.add(header.file_name_offset as usize) as *const u16;
            let name_len = (header.file_name_length / 2) as usize;
            let name_slice = slice::from_raw_parts(name_ptr, name_len);
            let name = String::from_utf16_lossy(name_slice).to_lowercase();

            let is_directory = (header.file_attributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;

            let node = Node {
                name: name.clone(),
                parent_id: header.parent_file_reference_number,
                drive_letter: drive,
                is_directory,
            };

            table_map_file_id.insert(header.file_reference_number, node)?;
            table_map_file_name.insert((name, header.file_reference_number), ())?;

            offset += record_len;
        }
    }

    Ok(())
}
